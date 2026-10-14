//! Owned facts from the font operation that actually succeeded.
//!
//! These records do not verify a scaler/provider, execute a platform query,
//! or select a raster policy. In particular application registration remains
//! application registration even when its bytes are bundled test resources.

use std::sync::Arc;

use super::{FontDescription, FontFaceInfo};

/// The successful selection branch, independent of the intrinsic family name.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSelectionOrigin {
    ApplicationRegistration,
    InstalledFamilySelection,
    InstalledCharacterFallback,
    ExternalTypefaceUnspecified,
}

/// The constructor/matcher invoked by Open UI, not a verified font backend.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontProviderConstruction {
    SystemManagerDataLoader,
    SystemManagerFamilyMatcher,
    SystemManagerCharacterMatcher,
    ExternalTypefaceUnspecified,
}

/// Source-derived intent for a future platform rendering query.
///
/// The legacy selection path executes no such query. This is neither a query
/// result nor permission to route a font to a different provider.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontRenderQueryIntent {
    PlatformDefaultEmptyFamily,
    NamedSelectionFamily,
    NamedResolvedFallbackFamily,
    UnspecifiedLegacy,
}

/// Immutable application registration, owning the original validated bytes.
///
/// It retains no collection, manager, engine, or callbacks. Old views survive
/// removal and cannot manufacture an installed or verified provider identity.
#[derive(Clone)]
pub struct FontFaceSourceInfo {
    info: FontFaceInfo,
    bytes: Arc<[u8]>,
    registration_generation: u64,
}

impl FontFaceSourceInfo {
    pub(crate) fn registered(
        info: FontFaceInfo,
        bytes: Arc<[u8]>,
        registration_generation: u64,
    ) -> Self {
        Self {
            info,
            bytes,
            registration_generation,
        }
    }

    pub fn face_info(&self) -> &FontFaceInfo {
        &self.info
    }
    pub fn bytes(&self) -> &Arc<[u8]> {
        &self.bytes
    }
    pub fn registration_generation(&self) -> u64 {
        self.registration_generation
    }
    pub fn origin(&self) -> FontSelectionOrigin {
        FontSelectionOrigin::ApplicationRegistration
    }
    pub fn provider_construction(&self) -> FontProviderConstruction {
        FontProviderConstruction::SystemManagerDataLoader
    }
}

impl std::fmt::Debug for FontFaceSourceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontFaceSourceInfo")
            .field("info", &self.info)
            .field("registration_generation", &self.registration_generation)
            .finish_non_exhaustive()
    }
}

/// Actual arguments supplied to the existing typeface cloning operation.
///
/// Requested CSS values are separately available from `instance_description`.
/// Coordinates here have already passed the current axis filtering/clamping.
/// Arguments are not described as applied when the clone returned no typeface.
#[derive(Debug, Clone, Default)]
pub struct FontInstantiation {
    collection_index: Option<usize>,
    coordinates: Vec<([u8; 4], f32)>,
    palette_index: Option<i32>,
    palette_overrides: Vec<(u16, skia_safe::Color)>,
    clone_attempted: bool,
    clone_succeeded: bool,
}

impl FontInstantiation {
    pub(crate) fn attempted(
        collection_index: usize,
        coordinates: Vec<([u8; 4], f32)>,
        palette_index: i32,
        palette_overrides: Vec<(u16, skia_safe::Color)>,
        clone_succeeded: bool,
    ) -> Self {
        Self {
            collection_index: Some(collection_index),
            coordinates,
            palette_index: Some(palette_index),
            palette_overrides,
            clone_attempted: true,
            clone_succeeded,
        }
    }
    pub fn coordinates(&self) -> &[([u8; 4], f32)] {
        &self.coordinates
    }
    /// Actual clone argument, separate from the originally registered face index.
    pub fn collection_index(&self) -> Option<usize> {
        self.collection_index
    }
    pub fn palette_index(&self) -> Option<i32> {
        self.palette_index
    }
    pub fn palette_overrides(&self) -> &[(u16, skia_safe::Color)] {
        &self.palette_overrides
    }
    pub fn clone_attempted(&self) -> bool {
        self.clone_attempted
    }
    pub fn clone_succeeded(&self) -> bool {
        self.clone_succeeded
    }
}

/// Selection and construction facts retained by one resolved platform font.
#[derive(Debug, Clone)]
pub struct FontSelectionSource {
    origin: FontSelectionOrigin,
    provider_construction: FontProviderConstruction,
    application_face: Option<Arc<FontFaceSourceInfo>>,
    collection_id: Option<u64>,
    selection_generation: Option<u64>,
    selection_family: Option<String>,
    resolved_family: Option<String>,
    render_query_intent: FontRenderQueryIntent,
    selected_typeface_id: Option<u32>,
    instance_typeface_id: Option<u32>,
    instance_description: Option<FontDescription>,
    instantiation: FontInstantiation,
}

impl FontSelectionSource {
    pub(crate) fn external() -> Self {
        Self {
            origin: FontSelectionOrigin::ExternalTypefaceUnspecified,
            provider_construction: FontProviderConstruction::ExternalTypefaceUnspecified,
            application_face: None,
            collection_id: None,
            selection_generation: None,
            selection_family: None,
            resolved_family: None,
            render_query_intent: FontRenderQueryIntent::UnspecifiedLegacy,
            selected_typeface_id: None,
            instance_typeface_id: None,
            instance_description: None,
            instantiation: FontInstantiation::default(),
        }
    }

    pub(crate) fn application(
        face: Arc<FontFaceSourceInfo>,
        selection_family: &str,
        generation: u64,
        typeface: &skia_safe::Typeface,
    ) -> Self {
        Self {
            origin: FontSelectionOrigin::ApplicationRegistration,
            provider_construction: face.provider_construction(),
            collection_id: Some(face.face_info().handle.collection_id()),
            application_face: Some(face),
            selection_generation: Some(generation),
            selection_family: Some(selection_family.to_owned()),
            resolved_family: Some(typeface.family_name()),
            render_query_intent: FontRenderQueryIntent::PlatformDefaultEmptyFamily,
            selected_typeface_id: Some(typeface.unique_id()),
            ..Self::external()
        }
    }

    pub(crate) fn installed(
        collection_id: u64,
        generation: u64,
        selection_family: &str,
        typeface: &skia_safe::Typeface,
        character_fallback: bool,
    ) -> Self {
        Self {
            origin: if character_fallback {
                FontSelectionOrigin::InstalledCharacterFallback
            } else {
                FontSelectionOrigin::InstalledFamilySelection
            },
            provider_construction: if character_fallback {
                FontProviderConstruction::SystemManagerCharacterMatcher
            } else {
                FontProviderConstruction::SystemManagerFamilyMatcher
            },
            collection_id: Some(collection_id),
            selection_generation: Some(generation),
            selection_family: Some(selection_family.to_owned()),
            resolved_family: Some(typeface.family_name()),
            render_query_intent: if character_fallback {
                FontRenderQueryIntent::NamedResolvedFallbackFamily
            } else {
                FontRenderQueryIntent::NamedSelectionFamily
            },
            selected_typeface_id: Some(typeface.unique_id()),
            ..Self::external()
        }
    }

    pub(crate) fn with_instance(
        mut self,
        typeface: &skia_safe::Typeface,
        description: &FontDescription,
        instantiation: FontInstantiation,
    ) -> Self {
        self.instance_typeface_id = Some(typeface.unique_id());
        self.instance_description = Some(description.clone());
        self.instantiation = instantiation;
        self
    }

    pub(crate) fn with_external_typeface(mut self, typeface: &skia_safe::Typeface) -> Self {
        if self.origin == FontSelectionOrigin::ExternalTypefaceUnspecified {
            self.selected_typeface_id = Some(typeface.unique_id());
            self.instance_typeface_id = Some(typeface.unique_id());
            self.resolved_family = Some(typeface.family_name());
        }
        self
    }

    pub fn origin(&self) -> FontSelectionOrigin {
        self.origin
    }
    pub fn provider_construction(&self) -> FontProviderConstruction {
        self.provider_construction
    }
    pub fn application_face(&self) -> Option<&FontFaceSourceInfo> {
        self.application_face.as_deref()
    }
    pub fn collection_id(&self) -> Option<u64> {
        self.collection_id
    }
    pub fn selection_generation(&self) -> Option<u64> {
        self.selection_generation
    }
    pub fn selection_family(&self) -> Option<&str> {
        self.selection_family.as_deref()
    }
    pub fn resolved_family(&self) -> Option<&str> {
        self.resolved_family.as_deref()
    }
    pub fn render_query_intent(&self) -> FontRenderQueryIntent {
        self.render_query_intent
    }
    pub fn render_query_family_intent(&self) -> Option<&str> {
        match self.render_query_intent {
            FontRenderQueryIntent::PlatformDefaultEmptyFamily => Some(""),
            FontRenderQueryIntent::NamedSelectionFamily => self.selection_family(),
            FontRenderQueryIntent::NamedResolvedFallbackFamily => self.resolved_family(),
            FontRenderQueryIntent::UnspecifiedLegacy => None,
        }
    }
    /// Process-local Skia identity; it is not a font file or provider identity.
    pub fn selected_typeface_id(&self) -> Option<u32> {
        self.selected_typeface_id
    }
    /// Process-local identity of the actual returned instance, including failed clones.
    pub fn instance_typeface_id(&self) -> Option<u32> {
        self.instance_typeface_id
    }
    pub fn instance_description(&self) -> Option<&FontDescription> {
        self.instance_description.as_ref()
    }
    pub fn instantiation(&self) -> &FontInstantiation {
        &self.instantiation
    }
}
