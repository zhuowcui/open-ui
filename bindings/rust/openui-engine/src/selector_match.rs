//! Matching typed selectors against elements (RFC 0001).
//!
//! The matcher follows Blink's `SelectorChecker`: a complex selector is
//! matched right to left from its subject, and descendant and subsequent
//! sibling combinators backtrack. The document root (`ElementTag::Viewport`)
//! plays Blink's document node, so `:root` is its element child, and only
//! authored elements take part: text, generated pseudo-elements and the
//! viewport do not.
//!
//! `:hover`, `:active`, `:focus`, `:focus-visible` and `:focus-within` read
//! the engine's interaction state. Pseudo-classes that depend on form or
//! dialog state are not matched yet; they return
//! [`SelectorMatchError::UnsupportedPseudoClass`] rather than a silent
//! non-match.

use openui_dom::ElementTag;
use openui_style::selector::{
    AttributeMatcher, AttributeSelector, Combinator, Compound, PseudoClass, RelativeSelector,
    Selector, SimpleSelector,
};

use crate::{Engine, EngineError, NodeHandle};

/// Why a selector could not be matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectorMatchError {
    Engine(EngineError),
    /// The pseudo-class depends on state the matcher does not read yet.
    UnsupportedPseudoClass(&'static str),
}

impl From<EngineError> for SelectorMatchError {
    fn from(error: EngineError) -> Self {
        Self::Engine(error)
    }
}

type MatchResult = Result<bool, SelectorMatchError>;

impl Engine {
    /// Whether `element` matches `selector`.
    pub fn matches_selector(&self, element: NodeHandle, selector: &Selector) -> MatchResult {
        if !self.is_selector_element(element)? {
            return Ok(false);
        }
        self.match_complex(element, selector)
    }

    /// Whether `element` matches any selector of `list`.
    pub fn matches_selector_list(&self, element: NodeHandle, list: &[Selector]) -> MatchResult {
        for selector in list {
            if self.matches_selector(element, selector)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn is_selector_element(&self, handle: NodeHandle) -> Result<bool, EngineError> {
        Ok(self.is_authored_element(handle)? && self.element_tag(handle)? != ElementTag::Viewport)
    }

    fn match_complex(&self, element: NodeHandle, selector: &Selector) -> MatchResult {
        if !self.match_compound(element, &selector.subject)? {
            return Ok(false);
        }
        self.match_leftwards(element, &selector.ancestors)
    }

    /// Match the compounds left of the one `element` matched.
    fn match_leftwards(&self, element: NodeHandle, rest: &[(Combinator, Compound)]) -> MatchResult {
        let Some(((combinator, compound), further)) = rest.split_first() else {
            return Ok(true);
        };
        match combinator {
            Combinator::Child => match self.parent_element(element)? {
                Some(parent) => Ok(self.match_compound(parent, compound)?
                    && self.match_leftwards(parent, further)?),
                None => Ok(false),
            },
            Combinator::Descendant => {
                let mut current = self.parent_element(element)?;
                while let Some(ancestor) = current {
                    if self.match_compound(ancestor, compound)?
                        && self.match_leftwards(ancestor, further)?
                    {
                        return Ok(true);
                    }
                    current = self.parent_element(ancestor)?;
                }
                Ok(false)
            }
            Combinator::NextSibling => match self.previous_element_siblings(element)?.first() {
                Some(&sibling) => Ok(self.match_compound(sibling, compound)?
                    && self.match_leftwards(sibling, further)?),
                None => Ok(false),
            },
            Combinator::SubsequentSibling => {
                for sibling in self.previous_element_siblings(element)? {
                    if self.match_compound(sibling, compound)?
                        && self.match_leftwards(sibling, further)?
                    {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }

    fn match_compound(&self, element: NodeHandle, compound: &Compound) -> MatchResult {
        for simple in &compound.simple {
            if !self.match_simple(element, simple)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn match_simple(&self, element: NodeHandle, simple: &SimpleSelector) -> MatchResult {
        Ok(match simple {
            SimpleSelector::Universal => true,
            SimpleSelector::Tag(name) => {
                local_name(self.element_tag(element)?).is_some_and(|local| local == name)
            }
            SimpleSelector::Id(id) => self.attribute(element, "id")? == Some(id.as_str()),
            SimpleSelector::Class(class) => self
                .attribute(element, "class")?
                .is_some_and(|classes| classes.split(is_html_space).any(|token| token == class)),
            SimpleSelector::Attribute(attribute) => self.match_attribute(element, attribute)?,
            SimpleSelector::PseudoClass(pseudo) => return self.match_pseudo(element, pseudo),
        })
    }

    fn match_attribute(
        &self,
        element: NodeHandle,
        selector: &AttributeSelector,
    ) -> Result<bool, EngineError> {
        let Some(value) = self.attribute(element, &selector.name)? else {
            return Ok(false);
        };
        let insensitive =
            selector.case_insensitive || is_legacy_case_insensitive_attribute(&selector.name);
        let equal = |a: &str, b: &str| {
            if insensitive {
                a.eq_ignore_ascii_case(b)
            } else {
                a == b
            }
        };
        // As Blink's `AttributeValueMatches`.
        Ok(match &selector.matcher {
            AttributeMatcher::Present => true,
            AttributeMatcher::Exact(expected) => equal(value, expected),
            AttributeMatcher::Includes(word) => {
                !word.is_empty()
                    && !word.contains(is_html_space)
                    && value.split(is_html_space).any(|token| equal(token, word))
            }
            AttributeMatcher::DashMatch(prefix) => {
                value.len() >= prefix.len()
                    && value.is_char_boundary(prefix.len())
                    && equal(&value[..prefix.len()], prefix)
                    && (value.len() == prefix.len() || value.as_bytes()[prefix.len()] == b'-')
            }
            AttributeMatcher::Prefix(prefix) => {
                !prefix.is_empty()
                    && value.len() >= prefix.len()
                    && value.is_char_boundary(prefix.len())
                    && equal(&value[..prefix.len()], prefix)
            }
            AttributeMatcher::Suffix(suffix) => {
                !suffix.is_empty()
                    && value.len() >= suffix.len()
                    && value.is_char_boundary(value.len() - suffix.len())
                    && equal(&value[value.len() - suffix.len()..], suffix)
            }
            AttributeMatcher::Substring(part) => {
                !part.is_empty()
                    && if insensitive {
                        value
                            .to_ascii_lowercase()
                            .contains(&part.to_ascii_lowercase())
                    } else {
                        value.contains(part.as_str())
                    }
            }
        })
    }

    fn match_pseudo(&self, element: NodeHandle, pseudo: &PseudoClass) -> MatchResult {
        Ok(match pseudo {
            PseudoClass::Is(list) | PseudoClass::Where(list) => {
                self.match_any_complex(element, list)?
            }
            PseudoClass::Not(list) => !self.match_any_complex(element, list)?,
            PseudoClass::Has(list) => self.match_has(element, list)?,
            PseudoClass::FirstChild => self.previous_element_siblings(element)?.is_empty(),
            PseudoClass::LastChild => self.next_element_siblings(element)?.is_empty(),
            PseudoClass::OnlyChild => {
                self.previous_element_siblings(element)?.is_empty()
                    && self.next_element_siblings(element)?.is_empty()
            }
            PseudoClass::NthChild { step, offset, of } => {
                if let Some(list) = of {
                    if !self.match_any_complex(element, list)? {
                        return Ok(false);
                    }
                }
                let mut index: i64 = 1;
                for sibling in self.previous_element_siblings(element)? {
                    let counts = match of {
                        Some(list) => self.match_any_complex(sibling, list)?,
                        None => true,
                    };
                    if counts {
                        index += 1;
                    }
                }
                nth_matches(i64::from(*step), i64::from(*offset), index)
            }
            PseudoClass::Empty => self.is_empty_element(element)?,
            PseudoClass::Root => match self.parent(element)? {
                Some(parent) => self.element_tag(parent)? == ElementTag::Viewport,
                None => false,
            },
            PseudoClass::Hover => self.is_hovered(element)?,
            PseudoClass::Active => self.is_active(element)?,
            PseudoClass::Focus => self.focused() == Some(element),
            PseudoClass::FocusVisible => self.focused() == Some(element) && self.focus_visible(),
            // Blink sets the focus-within flag on the focused element and its
            // flat-tree ancestors.
            PseudoClass::FocusWithin => match self.focused() {
                Some(focused) => self.path_to_root(focused)?.contains(&element),
                None => false,
            },
            PseudoClass::Disabled => return unsupported("disabled"),
            PseudoClass::Enabled => return unsupported("enabled"),
            PseudoClass::Checked => return unsupported("checked"),
            PseudoClass::Indeterminate => return unsupported("indeterminate"),
            PseudoClass::PlaceholderShown => return unsupported("placeholder-shown"),
            PseudoClass::ReadOnly => return unsupported("read-only"),
            PseudoClass::ReadWrite => return unsupported("read-write"),
            PseudoClass::Required => return unsupported("required"),
            PseudoClass::Invalid => return unsupported("invalid"),
            PseudoClass::Open => return unsupported("open"),
            PseudoClass::Modal => return unsupported("modal"),
        })
    }

    fn match_any_complex(&self, element: NodeHandle, list: &[Selector]) -> MatchResult {
        for selector in list {
            if self.match_complex(element, selector)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// `:has()`: some element in the relation each relative selector's
    /// combinator names, relative to `anchor`, matches it.
    fn match_has(&self, anchor: NodeHandle, list: &[RelativeSelector]) -> MatchResult {
        for relative in list {
            let candidates = match relative.combinator {
                Combinator::Descendant => self.descendant_elements(anchor)?,
                Combinator::Child => self.child_elements(anchor)?,
                Combinator::NextSibling => self
                    .next_element_siblings(anchor)?
                    .into_iter()
                    .take(1)
                    .collect(),
                Combinator::SubsequentSibling => self.next_element_siblings(anchor)?,
            };
            for candidate in candidates {
                if self.match_relative(anchor, candidate, relative)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Whether `candidate` matches the relative selector with its leftmost
    /// compound in the right relation to `anchor`.
    fn match_relative(
        &self,
        anchor: NodeHandle,
        candidate: NodeHandle,
        relative: &RelativeSelector,
    ) -> MatchResult {
        let selector = &relative.selector;
        if !self.match_compound(candidate, &selector.subject)? {
            return Ok(false);
        }
        // Extend the selector with the anchor as its leftmost compound: the
        // anchor matches only itself.
        self.match_leftwards_to_anchor(candidate, &selector.ancestors, anchor, relative.combinator)
    }

    fn match_leftwards_to_anchor(
        &self,
        element: NodeHandle,
        rest: &[(Combinator, Compound)],
        anchor: NodeHandle,
        anchor_combinator: Combinator,
    ) -> MatchResult {
        let Some(((combinator, compound), further)) = rest.split_first() else {
            return self.related(anchor, element, anchor_combinator);
        };
        let candidates = match combinator {
            Combinator::Child => self.parent_element(element)?.into_iter().collect(),
            Combinator::Descendant => self.ancestor_elements(element)?,
            Combinator::NextSibling => self
                .previous_element_siblings(element)?
                .into_iter()
                .take(1)
                .collect(),
            Combinator::SubsequentSibling => self.previous_element_siblings(element)?,
        };
        for candidate in candidates {
            if self.match_compound(candidate, compound)?
                && self.match_leftwards_to_anchor(candidate, further, anchor, anchor_combinator)?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Whether `element` stands in `combinator`'s relation to `anchor`.
    fn related(
        &self,
        anchor: NodeHandle,
        element: NodeHandle,
        combinator: Combinator,
    ) -> MatchResult {
        Ok(match combinator {
            Combinator::Child => self.parent_element(element)? == Some(anchor),
            Combinator::Descendant => self.ancestor_elements(element)?.contains(&anchor),
            Combinator::NextSibling => {
                self.previous_element_siblings(element)?.first() == Some(&anchor)
            }
            Combinator::SubsequentSibling => {
                self.previous_element_siblings(element)?.contains(&anchor)
            }
        })
    }

    fn parent_element(&self, element: NodeHandle) -> Result<Option<NodeHandle>, EngineError> {
        match self.parent(element)? {
            Some(parent) if self.is_selector_element(parent)? => Ok(Some(parent)),
            _ => Ok(None),
        }
    }

    fn ancestor_elements(&self, element: NodeHandle) -> Result<Vec<NodeHandle>, EngineError> {
        let mut ancestors = Vec::new();
        let mut current = self.parent_element(element)?;
        while let Some(ancestor) = current {
            ancestors.push(ancestor);
            current = self.parent_element(ancestor)?;
        }
        Ok(ancestors)
    }

    fn child_elements(&self, element: NodeHandle) -> Result<Vec<NodeHandle>, EngineError> {
        let mut children = Vec::new();
        for child in self.children(element)? {
            if self.is_selector_element(child)? {
                children.push(child);
            }
        }
        Ok(children)
    }

    fn descendant_elements(&self, element: NodeHandle) -> Result<Vec<NodeHandle>, EngineError> {
        let mut descendants = Vec::new();
        let mut stack = self.child_elements(element)?;
        stack.reverse();
        while let Some(next) = stack.pop() {
            descendants.push(next);
            let mut children = self.child_elements(next)?;
            children.reverse();
            stack.extend(children);
        }
        Ok(descendants)
    }

    /// The element siblings before `element`, nearest first.
    fn previous_element_siblings(
        &self,
        element: NodeHandle,
    ) -> Result<Vec<NodeHandle>, EngineError> {
        let Some(parent) = self.parent(element)? else {
            return Ok(Vec::new());
        };
        let siblings = self.child_elements(parent)?;
        let mut before: Vec<NodeHandle> = siblings
            .into_iter()
            .take_while(|&sibling| sibling != element)
            .collect();
        before.reverse();
        Ok(before)
    }

    /// The element siblings after `element`, nearest first.
    fn next_element_siblings(&self, element: NodeHandle) -> Result<Vec<NodeHandle>, EngineError> {
        let Some(parent) = self.parent(element)? else {
            return Ok(Vec::new());
        };
        Ok(self
            .child_elements(parent)?
            .into_iter()
            .skip_while(|&sibling| sibling != element)
            .skip(1)
            .collect())
    }

    /// `:empty`: no element children and no text, as Blink, which counts a
    /// text node of any length, whitespace included, except an empty one.
    fn is_empty_element(&self, element: NodeHandle) -> Result<bool, EngineError> {
        for child in self.children(element)? {
            if !self.is_authored_element(child)? {
                if self.element_tag(child)? == ElementTag::Text
                    && self.text_data(child).is_ok_and(|text| !text.is_empty())
                {
                    return Ok(false);
                }
            } else if self.is_selector_element(child)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

fn unsupported(name: &'static str) -> MatchResult {
    Err(SelectorMatchError::UnsupportedPseudoClass(name))
}

/// Whether the 1-based `index` is `step × n + offset` for some n ≥ 0.
fn nth_matches(step: i64, offset: i64, index: i64) -> bool {
    if step == 0 {
        return index == offset;
    }
    let difference = index - offset;
    difference % step == 0 && difference / step >= 0
}

fn is_html_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0C')
}

/// The HTML local name of an element kind, for type selectors.
fn local_name(tag: ElementTag) -> Option<&'static str> {
    Some(match tag {
        ElementTag::Div => "div",
        ElementTag::Span => "span",
        ElementTag::Break => "br",
        ElementTag::WordBreak => "wbr",
        ElementTag::Ruby => "ruby",
        ElementTag::RubyText => "rt",
        ElementTag::Table => "table",
        ElementTag::TableCaption => "caption",
        ElementTag::TableColumnGroup => "colgroup",
        ElementTag::TableColumn => "col",
        ElementTag::TableHead => "thead",
        ElementTag::TableBody => "tbody",
        ElementTag::TableFoot => "tfoot",
        ElementTag::TableRow => "tr",
        ElementTag::TableCell => "td",
        ElementTag::TableHeaderCell => "th",
        ElementTag::Image => "img",
        ElementTag::Canvas => "canvas",
        ElementTag::Svg => "svg",
        ElementTag::IFrame => "iframe",
        ElementTag::Object => "object",
        ElementTag::Audio => "audio",
        ElementTag::Video => "video",
        ElementTag::Input => "input",
        ElementTag::Button => "button",
        ElementTag::Meter => "meter",
        ElementTag::Progress => "progress",
        ElementTag::Fieldset => "fieldset",
        ElementTag::Legend => "legend",
        ElementTag::Details => "details",
        ElementTag::Summary => "summary",
        ElementTag::TextArea => "textarea",
        ElementTag::Select => "select",
        ElementTag::Option => "option",
        ElementTag::OptGroup => "optgroup",
        ElementTag::Form => "form",
        ElementTag::Embed => "embed",
        ElementTag::Html => "html",
        ElementTag::Body => "body",
        ElementTag::Style => "style",
        _ => return None,
    })
}

/// HTML 4.01 attributes whose values Blink compares ASCII
/// case-insensitively in HTML documents (`HTMLDocument::
/// IsCaseSensitiveAttribute`).
fn is_legacy_case_insensitive_attribute(name: &str) -> bool {
    matches!(
        name,
        "accept-charset"
            | "accept"
            | "align"
            | "alink"
            | "axis"
            | "bgcolor"
            | "charset"
            | "checked"
            | "clear"
            | "codetype"
            | "color"
            | "compact"
            | "declare"
            | "defer"
            | "dir"
            | "direction"
            | "disabled"
            | "enctype"
            | "face"
            | "frame"
            | "hreflang"
            | "http-equiv"
            | "lang"
            | "language"
            | "link"
            | "media"
            | "method"
            | "multiple"
            | "nohref"
            | "noresize"
            | "noshade"
            | "nowrap"
            | "readonly"
            | "rel"
            | "rev"
            | "rules"
            | "scope"
            | "scrolling"
            | "selected"
            | "shape"
            | "target"
            | "text"
            | "type"
            | "valign"
            | "valuetype"
            | "vlink"
    )
}

#[cfg(test)]
mod tests {
    use super::nth_matches;

    #[test]
    fn nth_formula() {
        // 2n+1: 1, 3, 5
        assert!(nth_matches(2, 1, 1) && nth_matches(2, 1, 3) && !nth_matches(2, 1, 2));
        // -n+3: 1, 2, 3
        assert!(nth_matches(-1, 3, 3) && nth_matches(-1, 3, 1) && !nth_matches(-1, 3, 4));
        // 3: only 3
        assert!(nth_matches(0, 3, 3) && !nth_matches(0, 3, 6));
        // n: every index
        assert!(nth_matches(1, 0, 1) && nth_matches(1, 0, 7));
        // 2n-1 = 2n+1 over n >= 0 plus index -1, so 1, 3, ...
        assert!(nth_matches(2, -1, 1) && !nth_matches(2, -1, 2));
    }
}
