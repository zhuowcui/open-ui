//! Lossless author inputs and canonical longhand ownership.
//!
//! These helpers store source values, never computed snapshots. The Engine
//! supplies the permanent native source baseline and resolves each surviving
//! input in its current font/viewport/color context before projection.
use crate::{apply_to_computed, ComputedStyle, PropertyTypeError, StyleProperty, StyleValue};
use std::collections::TryReserveError;

include!("generated_authored_fields.rs");

/// Internal ownership set. Compatibility summaries are derived, not owners.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalStyleFields([u64; CANONICAL_STYLE_FIELD_WORDS]);

impl CanonicalStyleFields {
    pub const EMPTY: Self = Self([0; CANONICAL_STYLE_FIELD_WORDS]);

    pub fn for_property(property: StyleProperty) -> Self {
        let mut fields = Self::EMPTY;
        for field in property.canonical_field_groups() {
            let index = *field as usize;
            fields.0[index / 64] |= 1 << (index % 64);
        }
        fields
    }

    pub fn contains(self, field: CanonicalStyleField) -> bool {
        let index = field as usize;
        self.0[index / 64] & (1 << (index % 64)) != 0
    }

    pub fn is_empty(self) -> bool {
        self.0.iter().all(|word| *word == 0)
    }

    fn intersects(self, other: Self) -> bool {
        self.0
            .iter()
            .zip(other.0)
            .any(|(word, other)| word & other != 0)
    }

    fn subtract(&mut self, removed: Self) -> bool {
        let before = *self;
        for (word, remove) in self.0.iter_mut().zip(removed.0) {
            *word &= !remove;
        }
        *self != before
    }

    fn can_override(self, other: Self) -> bool {
        if self.intersects(other) {
            return true;
        }
        CANONICAL_STYLE_FIELDS.iter().copied().any(|field| {
            self.contains(field) && field.dependent_mutation_groups().intersects(other)
        })
    }
}

/// One original typed value and the parts that remain effective.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredStyleContribution {
    property: StyleProperty,
    value: StyleValue,
    fields: CanonicalStyleFields,
}

impl AuthoredStyleContribution {
    pub fn property(&self) -> StyleProperty {
        self.property
    }

    pub fn value(&self) -> &StyleValue {
        &self.value
    }

    pub fn fields(&self) -> CanonicalStyleFields {
        self.fields
    }

    /// Apply an already context-resolved value without reviving masked fields.
    /// Errors leave the supplied style unchanged. The caller retains the raw
    /// `value()` for the next recomposition and publishes effects afterwards.
    pub fn project_resolved(
        &self,
        style: &mut ComputedStyle,
        resolved: &StyleValue,
        viewport: (f32, f32),
    ) -> Result<(), PropertyTypeError> {
        let mut candidate = style.clone();
        apply_to_computed(&mut candidate, self.property, resolved, viewport)?;
        for field in CANONICAL_STYLE_FIELDS {
            if self.fields.contains(*field) {
                field.copy_from(style, &candidate);
            }
        }
        if [
            CanonicalStyleField::WhiteSpaceCollapse,
            CanonicalStyleField::TextWrapMode,
            CanonicalStyleField::TextWrapStyle,
        ]
        .into_iter()
        .any(|field| self.fields.contains(field))
        {
            style.fields.white_space = crate::property::normalized_white_space(
                style.fields.white_space_collapse,
                style.fields.text_wrap_mode,
            );
            style.fields.text_wrap = crate::property::normalized_text_wrap(
                style.fields.text_wrap_mode,
                style.fields.text_wrap_style,
            );
        }
        // Own color is published before dependent ordinary colors, as in the
        // existing Engine cascade. This does not resolve text-fill keywords.
        style.resolve_current_color_dependencies();
        Ok(())
    }
}

/// Ordered, currently live canonical contributions.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AuthoredStyleDeclarations {
    contributions: Vec<AuthoredStyleContribution>,
}

impl AuthoredStyleDeclarations {
    pub fn contributions(&self) -> &[AuthoredStyleContribution] {
        &self.contributions
    }

    /// Commit an owned input that the Engine has already type-validated.
    /// Reserve before changing ownership. A missing allocation leaves every
    /// older declaration intact; source values keep their original units.
    pub fn try_set_validated(
        &mut self,
        property: StyleProperty,
        value: StyleValue,
    ) -> Result<bool, TryReserveError> {
        let fields = CanonicalStyleFields::for_property(property);
        if let Some(index) = self.contributions.iter().position(|contribution| {
            contribution.property == property
                && contribution.value == value
                && contribution.fields == fields
        }) {
            if !self.contributions[index + 1..]
                .iter()
                .any(|contribution| contribution.fields.can_override(fields))
            {
                return Ok(false);
            }
        }
        self.contributions.try_reserve(1)?;
        for contribution in &mut self.contributions {
            contribution.fields.subtract(fields);
        }
        self.contributions
            .retain(|contribution| !contribution.fields.is_empty());
        self.contributions.push(AuthoredStyleContribution {
            property,
            value,
            fields,
        });
        Ok(true)
    }

    /// Remove the requested canonical set from every older declaration.
    /// A removed longhand never revives an earlier shorthand contribution.
    /// Native computed overrides retain their explicit, separate identities.
    pub fn remove(&mut self, property: StyleProperty) -> bool {
        let fields = CanonicalStyleFields::for_property(property);
        let mut changed = false;
        for contribution in &mut self.contributions {
            changed |= contribution.fields.subtract(fields);
        }
        if changed {
            self.contributions
                .retain(|contribution| !contribution.fields.is_empty());
        }
        changed
    }
}

#[cfg(test)]
#[path = "authored_style_tests.rs"]
mod tests;
