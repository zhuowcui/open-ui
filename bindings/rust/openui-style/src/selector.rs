//! Selectors of typed style rules (RFC 0001).
//!
//! A [`Selector`] is a complex selector stored right to left, starting at
//! the subject compound, as Blink stores it. Each compound keeps its simple
//! selectors in source order, so specificity can be computed exactly as
//! Blink's `CSSSelector::Specificity()` computes it, including repeated
//! simple selectors and its overflow clamping. Selectors are built at
//! compile time by the style macros; nothing parses selector text at run
//! time.

/// How a compound relates to the compound on its right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Combinator {
    /// `a b`
    Descendant,
    /// `a > b`
    Child,
    /// `a + b`
    NextSibling,
    /// `a ~ b`
    SubsequentSibling,
}

/// A complex selector, right to left.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    /// The compound that matches the styled element.
    pub subject: Compound,
    /// The compounds to the subject's left, nearest first. Each combinator
    /// relates its compound to the compound on its right.
    pub ancestors: Vec<(Combinator, Compound)>,
}

/// A compound selector: simple selectors in source order. Never empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compound {
    pub simple: Vec<SimpleSelector>,
}

/// One simple selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimpleSelector {
    /// `*`
    Universal,
    /// A type selector; the name is ASCII-lowercase.
    Tag(String),
    /// `#id`
    Id(String),
    /// `.class`
    Class(String),
    /// `[name]`, `[name=value]` and the other attribute matchers.
    Attribute(AttributeSelector),
    /// A supported pseudo-class.
    PseudoClass(PseudoClass),
}

/// An attribute selector. The name is ASCII-lowercase, as attribute names
/// in an HTML document are matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeSelector {
    pub name: String,
    pub matcher: AttributeMatcher,
    /// The `i` flag: compare ASCII case-insensitively. Chromium 147 drops
    /// a selector with the `s` flag, so there is none.
    pub case_insensitive: bool,
}

/// How an attribute selector compares the attribute's value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeMatcher {
    /// `[name]`
    Present,
    /// `[name=value]`
    Exact(String),
    /// `[name~=value]`: one of the whitespace-separated words.
    Includes(String),
    /// `[name|=value]`: the value, or the value followed by `-`.
    DashMatch(String),
    /// `[name^=value]`
    Prefix(String),
    /// `[name$=value]`
    Suffix(String),
    /// `[name*=value]`
    Substring(String),
}

/// The pseudo-classes RFC 0001 supports. Any other is a compile error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PseudoClass {
    Hover,
    Active,
    Focus,
    FocusVisible,
    FocusWithin,
    Disabled,
    Enabled,
    Checked,
    Indeterminate,
    PlaceholderShown,
    ReadOnly,
    ReadWrite,
    Required,
    Invalid,
    Open,
    Modal,
    FirstChild,
    LastChild,
    OnlyChild,
    Empty,
    Root,
    /// `:nth-child(An+B)` and `:nth-child(An+B of S)`.
    NthChild {
        step: i32,
        offset: i32,
        of: Option<Vec<Selector>>,
    },
    Is(Vec<Selector>),
    Where(Vec<Selector>),
    Not(Vec<Selector>),
    Has(Vec<RelativeSelector>),
}

/// An argument of `:has()`: a selector and the combinator that relates its
/// leftmost compound to the `:has()` anchor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelativeSelector {
    pub combinator: Combinator,
    pub selector: Selector,
}

/// A selector's specificity, packed as Blink packs it: ID-like in bits
/// 16..24, class-like in bits 8..16 and type-like in bits 0..8. Comparing
/// packed values orders specificities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Specificity(u32);

impl Specificity {
    const ID: u32 = 0x01_0000;
    const CLASS_LIKE: u32 = 0x00_0100;
    const TAG: u32 = 0x00_0001;
    const ID_MASK: u32 = 0xff_0000;
    const CLASS_MASK: u32 = 0x00_ff00;
    const ELEMENT_MASK: u32 = 0x00_00ff;

    /// The packed value.
    pub const fn packed(self) -> u32 {
        self.0
    }

    /// `(a, b, c)`: ID-like, class-like and type-like counts.
    pub const fn tuple(self) -> (u8, u8, u8) {
        (
            ((self.0 & Self::ID_MASK) >> 16) as u8,
            ((self.0 & Self::CLASS_MASK) >> 8) as u8,
            (self.0 & Self::ELEMENT_MASK) as u8,
        )
    }

    /// The largest specificity in a selector list, or zero for an empty
    /// one, as Blink's `MaximumSpecificity`.
    pub fn maximum(selectors: &[Selector]) -> Self {
        Self(
            selectors
                .iter()
                .map(|selector| selector.specificity().0)
                .max()
                .unwrap_or(0),
        )
    }
}

impl Selector {
    /// The specificity, as Blink's `CSSSelector::Specificity()`: the simple
    /// selectors' contributions are added from the subject compound
    /// leftwards, each compound in source order, and a component that would
    /// overflow is clamped to 255.
    pub fn specificity(&self) -> Specificity {
        let mut total: u32 = 0;
        let compounds = std::iter::once(&self.subject).chain(self.ancestors.iter().map(|(_, c)| c));
        for simple in compounds.flat_map(|compound| compound.simple.iter()) {
            let temp = total.wrapping_add(simple.specificity());
            if (temp & Specificity::ID_MASK) < (total & Specificity::ID_MASK) {
                total |= Specificity::ID_MASK;
            } else if (temp & Specificity::CLASS_MASK) < (total & Specificity::CLASS_MASK) {
                total |= Specificity::CLASS_MASK;
            } else if (temp & Specificity::ELEMENT_MASK) < (total & Specificity::ELEMENT_MASK) {
                total |= Specificity::ELEMENT_MASK;
            } else {
                total = temp;
            }
        }
        Specificity(total)
    }
}

impl SimpleSelector {
    /// One simple selector's contribution, as Blink's
    /// `CSSSelector::SpecificityForOneSelector()`.
    fn specificity(&self) -> u32 {
        match self {
            Self::Universal => 0,
            Self::Tag(_) => Specificity::TAG,
            Self::Id(_) => Specificity::ID,
            Self::Class(_) | Self::Attribute(_) => Specificity::CLASS_LIKE,
            Self::PseudoClass(pseudo) => match pseudo {
                PseudoClass::Where(_) => 0,
                PseudoClass::Is(list) | PseudoClass::Not(list) => Specificity::maximum(list).0,
                PseudoClass::Has(list) => list
                    .iter()
                    .map(|relative| relative.selector.specificity().0)
                    .max()
                    .unwrap_or(0),
                // Blink adds without clamping here; the caller clamps.
                PseudoClass::NthChild { of: Some(list), .. } => {
                    Specificity::CLASS_LIKE.wrapping_add(Specificity::maximum(list).0)
                }
                _ => Specificity::CLASS_LIKE,
            },
        }
    }
}
