//! Selector text, read at macro time (RFC 0001).
//!
//! [`parse_selector_list`] reads a selector list into [`Selector`]s. It
//! follows CSS Selectors Level 4 and CSS Nesting for the subset RFC 0001
//! supports. Anything outside that subset (pseudo-elements, namespaces,
//! unsupported pseudo-classes) is an error, never a selector that silently
//! matches nothing. Like the declaration block parser, this module exists
//! only with the internal `constructor-tokens` feature.

use std::ops::Range;

use crate::selector::{
    AttributeMatcher, AttributeSelector, Combinator, Compound, PseudoClass, RelativeSelector,
    Selector, SimpleSelector,
};

/// A selector the parser cannot accept, with its byte range in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectorError {
    pub message: String,
    pub source: Range<usize>,
}

/// Parse a selector list.
///
/// With `parent`, the list is the selector of a nested style rule: `&`
/// stands for `:is(<parent>)`, and a selector without `&` is relative to the
/// parent, as if it began with `& ` (or with `&` and its own leading
/// combinator). Without `parent`, `&` and leading combinators are errors.
pub fn parse_selector_list(
    text: &str,
    parent: Option<&[Selector]>,
) -> Result<Vec<Selector>, SelectorError> {
    let mut parser = Parser {
        text,
        offset: 0,
        position: 0,
        parent,
        in_has: false,
    };
    parser.selector_list()
}

#[derive(Clone, Copy)]
struct Parser<'a> {
    text: &'a str,
    /// Byte offset of `text` in the macro's source text, for errors.
    offset: usize,
    position: usize,
    parent: Option<&'a [Selector]>,
    in_has: bool,
}

/// A complex selector as written, left to right, before it is stored right
/// to left.
struct Written {
    leading: Option<Combinator>,
    compounds: Vec<Compound>,
    combinators: Vec<Combinator>,
    has_nesting: bool,
}

impl<'a> Parser<'a> {
    fn error<T>(
        &self,
        message: impl Into<String>,
        range: Range<usize>,
    ) -> Result<T, SelectorError> {
        Err(SelectorError {
            message: message.into(),
            source: self.offset + range.start..self.offset + range.end,
        })
    }

    fn peek(&self) -> Option<char> {
        self.text[self.position..].chars().next()
    }

    fn peek_at(&self, skip: usize) -> Option<char> {
        self.text[self.position..].chars().nth(skip)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.position += c.len_utf8();
        Some(c)
    }

    /// Skip whitespace and comments; whether anything was skipped.
    fn skip_whitespace(&mut self) -> Result<bool, SelectorError> {
        let start = self.position;
        loop {
            match self.peek() {
                Some(c) if is_whitespace(c) => {
                    self.bump();
                }
                Some('/') if self.peek_at(1) == Some('*') => {
                    let open = self.position;
                    match self.text[self.position + 2..].find("*/") {
                        Some(end) => self.position += 2 + end + 2,
                        None => return self.error("unterminated comment", open..self.text.len()),
                    }
                }
                _ => return Ok(self.position != start),
            }
        }
    }

    fn selector_list(&mut self) -> Result<Vec<Selector>, SelectorError> {
        let mut selectors = Vec::new();
        loop {
            let start = self.position;
            let written = self.complex()?;
            selectors.push(self.resolve(written, start)?);
            match self.peek() {
                None => return Ok(selectors),
                Some(',') => {
                    self.bump();
                }
                Some(c) => {
                    let at = self.position;
                    return self.error(
                        format!("unexpected `{c}` in selector"),
                        at..at + c.len_utf8(),
                    );
                }
            }
        }
    }

    /// Store a written complex selector right to left, applying nesting.
    fn resolve(&self, written: Written, start: usize) -> Result<Selector, SelectorError> {
        let range = start..self.position;
        let Written {
            leading,
            mut compounds,
            mut combinators,
            has_nesting,
        } = written;
        match self.parent {
            None => {
                if leading.is_some() {
                    return self.error("a selector cannot start with a combinator here", range);
                }
            }
            Some(parent) => {
                if leading.is_some() || !has_nesting {
                    // A relative nested selector: `> .a` is `& > .a`, `.a` is `& .a`.
                    compounds.insert(0, nesting_compound(parent));
                    combinators.insert(0, leading.unwrap_or(Combinator::Descendant));
                }
            }
        }
        let subject = compounds.pop().expect("a complex selector has a compound");
        let mut ancestors = Vec::with_capacity(compounds.len());
        while let Some(compound) = compounds.pop() {
            let combinator = combinators.pop().expect("one combinator between compounds");
            ancestors.push((combinator, compound));
        }
        Ok(Selector { subject, ancestors })
    }

    /// A complex selector, with an optional leading combinator.
    fn complex(&mut self) -> Result<Written, SelectorError> {
        self.skip_whitespace()?;
        let leading = self.combinator_char();
        if leading.is_some() {
            self.bump();
            self.skip_whitespace()?;
        }
        let mut written = Written {
            leading,
            compounds: Vec::new(),
            combinators: Vec::new(),
            has_nesting: false,
        };
        loop {
            let (compound, nesting) = self.compound()?;
            written.has_nesting |= nesting;
            written.compounds.push(compound);
            let had_whitespace = self.skip_whitespace()?;
            let combinator = match self.peek() {
                None | Some(',') => return Ok(written),
                Some(_) => match self.combinator_char() {
                    Some(combinator) => {
                        self.bump();
                        self.skip_whitespace()?;
                        combinator
                    }
                    None if had_whitespace => Combinator::Descendant,
                    None => {
                        let at = self.position;
                        let c = self.peek().unwrap_or(' ');
                        return self.error(
                            format!("unexpected `{c}` in selector"),
                            at..at + c.len_utf8(),
                        );
                    }
                },
            };
            written.combinators.push(combinator);
        }
    }

    fn combinator_char(&self) -> Option<Combinator> {
        match self.peek()? {
            '>' => Some(Combinator::Child),
            '+' => Some(Combinator::NextSibling),
            '~' => Some(Combinator::SubsequentSibling),
            _ => None,
        }
    }

    /// A compound selector and whether it contains `&`.
    fn compound(&mut self) -> Result<(Compound, bool), SelectorError> {
        let start = self.position;
        let mut simple = Vec::new();
        let mut nesting = false;
        loop {
            let at = self.position;
            match self.peek() {
                Some('*') => {
                    self.bump();
                    if self.peek() == Some('|') {
                        return self.error("namespaces are not supported", at..self.position + 1);
                    }
                    if !simple.is_empty() {
                        return self.error(
                            "`*` must come first in a compound selector",
                            at..self.position,
                        );
                    }
                    simple.push(SimpleSelector::Universal);
                }
                Some('|') => return self.error("namespaces are not supported", at..at + 1),
                Some('#') => {
                    self.bump();
                    if !self.starts_ident() {
                        return self.error("expected an ID after `#`", at..self.position);
                    }
                    simple.push(SimpleSelector::Id(self.ident()?));
                }
                Some('.') => {
                    self.bump();
                    if !self.starts_ident() {
                        return self.error("expected a class name after `.`", at..self.position);
                    }
                    simple.push(SimpleSelector::Class(self.ident()?));
                }
                Some('[') => simple.push(SimpleSelector::Attribute(self.attribute()?)),
                Some(':') => {
                    let (pseudo, inner_nesting) = self.pseudo()?;
                    nesting |= inner_nesting;
                    simple.push(pseudo);
                }
                Some('&') => {
                    self.bump();
                    let Some(parent) = self.parent else {
                        return self
                            .error("`&` is only allowed in a nested rule", at..self.position);
                    };
                    nesting = true;
                    simple.push(nesting_simple(parent));
                }
                Some(_) if self.starts_ident() => {
                    let name = self.ident()?;
                    if self.peek() == Some('|') {
                        return self.error("namespaces are not supported", at..self.position + 1);
                    }
                    if !simple.is_empty() {
                        return self.error(
                            format!(
                                "type selector `{name}` must come first in a compound selector"
                            ),
                            at..self.position,
                        );
                    }
                    simple.push(SimpleSelector::Tag(name.to_ascii_lowercase()));
                }
                _ => break,
            }
        }
        if simple.is_empty() {
            let end = self.position + self.peek().map_or(0, char::len_utf8);
            return self.error("expected a selector", start..end);
        }
        Ok((Compound { simple }, nesting))
    }

    fn attribute(&mut self) -> Result<AttributeSelector, SelectorError> {
        let open = self.position;
        self.bump();
        self.skip_whitespace()?;
        if !self.starts_ident() {
            let at = self.position;
            return self.error("expected an attribute name", open..at.max(open + 1));
        }
        let name = self.ident()?.to_ascii_lowercase();
        if self.peek() == Some('|') && self.peek_at(1) != Some('=') {
            return self.error("namespaces are not supported", open..self.position + 1);
        }
        self.skip_whitespace()?;
        let operator = match (self.peek(), self.peek_at(1)) {
            (Some(']'), _) => {
                self.bump();
                return Ok(AttributeSelector {
                    name,
                    matcher: AttributeMatcher::Present,
                    case_insensitive: false,
                });
            }
            (Some('='), _) => {
                self.bump();
                '='
            }
            (Some(c @ ('~' | '|' | '^' | '$' | '*')), Some('=')) => {
                self.bump();
                self.bump();
                c
            }
            _ => {
                let at = self.position;
                return self.error("expected `]` or an attribute matcher", open..at + 1);
            }
        };
        self.skip_whitespace()?;
        let value = match self.peek() {
            Some(quote @ ('"' | '\'')) => self.string(quote)?,
            _ if self.starts_ident() => self.ident()?,
            _ => {
                let at = self.position;
                return self.error("expected an attribute value", open..at + 1);
            }
        };
        self.skip_whitespace()?;
        let case_insensitive = if self.starts_ident() {
            let flag_start = self.position;
            let flag = self.ident()?;
            let flag_end = self.position;
            self.skip_whitespace()?;
            if !flag.eq_ignore_ascii_case("i") {
                // Chromium 147 drops `[a=b s]`, so it is an error here too.
                return self.error(
                    format!("unsupported attribute selector flag `{flag}`"),
                    flag_start..flag_end,
                );
            }
            true
        } else {
            false
        };
        if self.peek() != Some(']') {
            let at = self.position;
            return self.error("expected `]`", open..at + 1);
        }
        self.bump();
        let matcher = match operator {
            '=' => AttributeMatcher::Exact(value),
            '~' => AttributeMatcher::Includes(value),
            '|' => AttributeMatcher::DashMatch(value),
            '^' => AttributeMatcher::Prefix(value),
            '$' => AttributeMatcher::Suffix(value),
            _ => AttributeMatcher::Substring(value),
        };
        Ok(AttributeSelector {
            name,
            matcher,
            case_insensitive,
        })
    }

    /// A pseudo-class and whether its arguments contain `&`.
    fn pseudo(&mut self) -> Result<(SimpleSelector, bool), SelectorError> {
        let start = self.position;
        self.bump();
        if self.peek() == Some(':') {
            self.bump();
            let name = if self.starts_ident() {
                self.ident()?
            } else {
                String::new()
            };
            return self.error(
                format!("pseudo-elements are not supported (`::{name}`)"),
                start..self.position,
            );
        }
        if !self.starts_ident() {
            return self.error(
                "expected a pseudo-class name after `:`",
                start..self.position,
            );
        }
        let name = self.ident()?.to_ascii_lowercase();
        if self.peek() != Some('(') {
            let pseudo = match name.as_str() {
                "hover" => PseudoClass::Hover,
                "active" => PseudoClass::Active,
                "focus" => PseudoClass::Focus,
                "focus-visible" => PseudoClass::FocusVisible,
                "focus-within" => PseudoClass::FocusWithin,
                "disabled" => PseudoClass::Disabled,
                "enabled" => PseudoClass::Enabled,
                "checked" => PseudoClass::Checked,
                "indeterminate" => PseudoClass::Indeterminate,
                "placeholder-shown" => PseudoClass::PlaceholderShown,
                "read-only" => PseudoClass::ReadOnly,
                "read-write" => PseudoClass::ReadWrite,
                "required" => PseudoClass::Required,
                "invalid" => PseudoClass::Invalid,
                "open" => PseudoClass::Open,
                "modal" => PseudoClass::Modal,
                "first-child" => PseudoClass::FirstChild,
                "last-child" => PseudoClass::LastChild,
                "only-child" => PseudoClass::OnlyChild,
                "empty" => PseudoClass::Empty,
                "root" => PseudoClass::Root,
                _ => {
                    return self.error(
                        format!("unsupported pseudo-class `:{name}`"),
                        start..self.position,
                    )
                }
            };
            return Ok((SimpleSelector::PseudoClass(pseudo), false));
        }
        let open = self.position;
        let close = self.matching_paren()?;
        let inner = Parser {
            text: &self.text[..close],
            offset: self.offset,
            position: open + 1,
            parent: self.parent,
            in_has: self.in_has,
        };
        self.position = close + 1;
        let range = start..self.position;
        let (pseudo, nesting) = match name.as_str() {
            "is" => {
                let (list, nesting) = inner.argument_list(&name, range.clone())?;
                (PseudoClass::Is(list), nesting)
            }
            "where" => {
                let (list, nesting) = inner.argument_list(&name, range.clone())?;
                (PseudoClass::Where(list), nesting)
            }
            "not" => {
                let (list, nesting) = inner.argument_list(&name, range.clone())?;
                (PseudoClass::Not(list), nesting)
            }
            "has" => {
                if self.in_has {
                    return self.error("`:has()` cannot contain `:has()`", range);
                }
                let (list, nesting) = inner.relative_list(range.clone())?;
                (PseudoClass::Has(list), nesting)
            }
            "nth-child" => inner.nth_child(range.clone())?,
            _ => return self.error(format!("unsupported pseudo-class `:{name}()`"), range),
        };
        Ok((SimpleSelector::PseudoClass(pseudo), nesting))
    }

    /// The selector list argument of `:is()`, `:where()` or `:not()`, and
    /// whether it contains `&`.
    fn argument_list(
        mut self,
        name: &str,
        range: Range<usize>,
    ) -> Result<(Vec<Selector>, bool), SelectorError> {
        self.skip_whitespace()?;
        if self.peek().is_none() {
            return self.error(format!("`:{name}()` needs a selector"), range);
        }
        // Inside a function the arguments are not relative to the parent:
        // `&` still stands for it, but `.a` means `.a`.
        let mut selectors = Vec::new();
        let mut nesting = false;
        loop {
            let start = self.position;
            let written = self.complex()?;
            if written.leading.is_some() {
                return self.error(
                    "a selector cannot start with a combinator here",
                    start..self.position,
                );
            }
            nesting |= written.has_nesting;
            let unnested = Parser {
                parent: None,
                ..self
            };
            selectors.push(unnested.resolve(written, start)?);
            match self.peek() {
                None => return Ok((selectors, nesting)),
                Some(',') => {
                    self.bump();
                }
                Some(c) => {
                    let at = self.position;
                    return self.error(
                        format!("unexpected `{c}` in selector"),
                        at..at + c.len_utf8(),
                    );
                }
            }
        }
    }

    /// The relative selector list argument of `:has()`, and whether it
    /// contains `&`.
    fn relative_list(
        mut self,
        range: Range<usize>,
    ) -> Result<(Vec<RelativeSelector>, bool), SelectorError> {
        self.in_has = true;
        self.skip_whitespace()?;
        if self.peek().is_none() {
            return self.error("`:has()` needs a selector", range);
        }
        let mut selectors = Vec::new();
        let mut nesting = false;
        loop {
            let start = self.position;
            let written = self.complex()?;
            nesting |= written.has_nesting;
            let combinator = written.leading.unwrap_or(Combinator::Descendant);
            let unnested = Parser {
                parent: None,
                ..self
            };
            let selector = unnested.resolve(
                Written {
                    leading: None,
                    ..written
                },
                start,
            )?;
            selectors.push(RelativeSelector {
                combinator,
                selector,
            });
            match self.peek() {
                None => return Ok((selectors, nesting)),
                Some(',') => {
                    self.bump();
                }
                Some(c) => {
                    let at = self.position;
                    return self.error(
                        format!("unexpected `{c}` in selector"),
                        at..at + c.len_utf8(),
                    );
                }
            }
        }
    }

    /// `An+B` and `An+B of S`, and whether `S` contains `&`.
    fn nth_child(mut self, range: Range<usize>) -> Result<(PseudoClass, bool), SelectorError> {
        let rest = &self.text[self.position..];
        let (formula, of) = match find_of(rest) {
            Some((formula_end, of_start)) => (&rest[..formula_end], Some(self.position + of_start)),
            None => (rest, None),
        };
        let Some((step, offset)) = parse_an_plus_b(formula) else {
            return self.error(
                format!("invalid `:nth-child()` argument `{}`", formula.trim()),
                range,
            );
        };
        let (of, nesting) = match of {
            None => (None, false),
            Some(of_start) => {
                self.position = of_start;
                let (list, nesting) = self.argument_list("nth-child", range)?;
                (Some(list), nesting)
            }
        };
        Ok((PseudoClass::NthChild { step, offset, of }, nesting))
    }

    /// The byte index of the `)` that closes the `(` at the cursor.
    fn matching_paren(&self) -> Result<usize, SelectorError> {
        let open = self.position;
        let bytes = self.text.as_bytes();
        let mut depth = 0usize;
        let mut index = open;
        let mut quote: Option<u8> = None;
        while index < bytes.len() {
            let byte = bytes[index];
            match quote {
                Some(q) => {
                    if byte == b'\\' {
                        index += 1;
                    } else if byte == q {
                        quote = None;
                    }
                }
                None => match byte {
                    b'\\' => index += 1,
                    b'"' | b'\'' => quote = Some(byte),
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            return Ok(index);
                        }
                    }
                    _ => {}
                },
            }
            index += 1;
        }
        self.error("unclosed `(`", open..self.text.len())
    }

    /// Whether an identifier starts at the cursor (CSS Syntax 4.3.9).
    fn starts_ident(&self) -> bool {
        let rest = &self.text[self.position..];
        let mut chars = rest.chars();
        match chars.next() {
            Some('-') => match chars.next() {
                Some('-') => true,
                Some('\\') => !matches!(chars.next(), None | Some('\n')),
                Some(c) => is_ident_start(c),
                None => false,
            },
            Some('\\') => !matches!(chars.next(), None | Some('\n')),
            Some(c) => is_ident_start(c),
            None => false,
        }
    }

    /// An identifier, with escapes resolved (CSS Syntax 4.3.11).
    fn ident(&mut self) -> Result<String, SelectorError> {
        let mut name = String::new();
        loop {
            match self.peek() {
                Some(c) if is_ident_char(c) => {
                    name.push(c);
                    self.bump();
                }
                Some('\\') if !matches!(self.peek_at(1), None | Some('\n')) => {
                    self.bump();
                    name.push(self.escape());
                }
                _ => return Ok(name),
            }
        }
    }

    /// The character an escape stands for; the cursor is after the `\`.
    fn escape(&mut self) -> char {
        let mut hex = String::new();
        while hex.len() < 6 {
            match self.peek() {
                Some(c) if c.is_ascii_hexdigit() => {
                    hex.push(c);
                    self.bump();
                }
                _ => break,
            }
        }
        if hex.is_empty() {
            return self.bump().unwrap_or('\u{FFFD}');
        }
        if matches!(self.peek(), Some(c) if is_whitespace(c)) {
            self.bump();
        }
        let value = u32::from_str_radix(&hex, 16).unwrap_or(0);
        match char::from_u32(value) {
            Some(c) if value != 0 => c,
            _ => '\u{FFFD}',
        }
    }

    /// A quoted string, with escapes resolved; the cursor is at the quote.
    fn string(&mut self, quote: char) -> Result<String, SelectorError> {
        let open = self.position;
        self.bump();
        let mut value = String::new();
        loop {
            match self.bump() {
                Some(c) if c == quote => return Ok(value),
                Some('\n') | None => return self.error("unterminated string", open..self.position),
                Some('\\') => match self.peek() {
                    None => {}
                    Some('\n') => {
                        self.bump();
                    }
                    Some(_) => value.push(self.escape()),
                },
                Some(c) => value.push(c),
            }
        }
    }
}

/// `&` as a simple selector: `:is(<parent>)`, which has the parent list's
/// largest specificity, as Blink gives `&`.
fn nesting_simple(parent: &[Selector]) -> SimpleSelector {
    SimpleSelector::PseudoClass(PseudoClass::Is(parent.to_vec()))
}

fn nesting_compound(parent: &[Selector]) -> Compound {
    Compound {
        simple: vec![nesting_simple(parent)],
    }
}

fn is_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0C')
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || !c.is_ascii()
}

fn is_ident_char(c: char) -> bool {
    is_ident_start(c) || c.is_ascii_digit() || c == '-'
}

/// The end of the `An+B` part and the start of the selector list in
/// `An+B of S`, when the argument has an `of`.
fn find_of(text: &str) -> Option<(usize, usize)> {
    let lower = text.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut search = 0;
    while let Some(found) = lower[search..].find("of") {
        let at = search + found;
        let before_ok = at > 0 && is_whitespace(bytes[at - 1] as char);
        let after_ok = bytes.get(at + 2).is_some_and(|b| is_whitespace(*b as char));
        if before_ok && after_ok {
            return Some((at, at + 2));
        }
        search = at + 2;
    }
    None
}

/// `odd`, `even`, `B`, `An`, `An+B` and `An-B`, with the whitespace CSS
/// Syntax's An+B grammar allows: around the sign before `B`, and nowhere
/// else.
fn parse_an_plus_b(text: &str) -> Option<(i32, i32)> {
    let text = text.trim_matches(is_whitespace);
    let lower = text.to_ascii_lowercase();
    match lower.as_str() {
        "odd" => return Some((2, 1)),
        "even" => return Some((2, 0)),
        _ => {}
    }
    let Some(n) = lower.find('n') else {
        return parse_integer(&lower, true).map(|b| (0, b));
    };
    let step = match &lower[..n] {
        "" | "+" => 1,
        "-" => -1,
        prefix => parse_integer(prefix, true)?,
    };
    let rest = lower[n + 1..].trim_start_matches(is_whitespace);
    if rest.is_empty() {
        return Some((step, 0));
    }
    let (sign, digits) = match rest.as_bytes()[0] {
        b'+' => (1, &rest[1..]),
        b'-' => (-1, &rest[1..]),
        _ => return None,
    };
    let digits = digits.trim_start_matches(is_whitespace);
    let offset = parse_integer(digits, false)?;
    Some((step, sign * offset))
}

fn parse_integer(text: &str, signed: bool) -> Option<i32> {
    let digits = if signed {
        text.strip_prefix('+')
            .or_else(|| text.strip_prefix('-').map(|_| text))
            .unwrap_or(text)
    } else {
        text
    };
    let unsigned = digits.strip_prefix('-').unwrap_or(digits);
    if unsigned.is_empty() || !unsigned.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if !signed && digits.starts_with('-') {
        return None;
    }
    digits
        .parse::<i64>()
        .ok()
        .map(|v| v.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selector::Specificity;

    fn one(text: &str) -> Selector {
        let mut list = parse_selector_list(text, None).unwrap_or_else(|e| panic!("{text}: {e:?}"));
        assert_eq!(list.len(), 1, "{text}");
        list.remove(0)
    }

    fn spec(text: &str) -> (u8, u8, u8) {
        Specificity::maximum(&parse_selector_list(text, None).unwrap()).tuple()
    }

    fn error(text: &str) -> String {
        parse_selector_list(text, None).unwrap_err().message
    }

    #[test]
    fn complex_selectors_are_stored_right_to_left() {
        let selector = one("ul > li.item + a:hover");
        assert_eq!(
            selector.subject.simple,
            vec![
                SimpleSelector::Tag("a".into()),
                SimpleSelector::PseudoClass(PseudoClass::Hover)
            ]
        );
        assert_eq!(selector.ancestors.len(), 2);
        assert_eq!(selector.ancestors[0].0, Combinator::NextSibling);
        assert_eq!(selector.ancestors[1].0, Combinator::Child);
        assert_eq!(
            selector.ancestors[1].1.simple,
            vec![SimpleSelector::Tag("ul".into())]
        );
    }

    #[test]
    fn escaped_class_names_resolve() {
        let selector = one(r".hover\:bg-muted.w-1\/2.data-\[state\=open\]\:p-2.\32xl");
        assert_eq!(
            selector.subject.simple,
            vec![
                SimpleSelector::Class("hover:bg-muted".into()),
                SimpleSelector::Class("w-1/2".into()),
                SimpleSelector::Class("data-[state=open]:p-2".into()),
                SimpleSelector::Class("2xl".into()),
            ]
        );
    }

    #[test]
    fn attribute_selectors() {
        let selector = one(r#"[data-state="open"][aria-x~=b i][DIR|='ltr' I][href]"#);
        assert_eq!(
            selector.subject.simple,
            vec![
                SimpleSelector::Attribute(AttributeSelector {
                    name: "data-state".into(),
                    matcher: AttributeMatcher::Exact("open".into()),
                    case_insensitive: false,
                }),
                SimpleSelector::Attribute(AttributeSelector {
                    name: "aria-x".into(),
                    matcher: AttributeMatcher::Includes("b".into()),
                    case_insensitive: true,
                }),
                SimpleSelector::Attribute(AttributeSelector {
                    name: "dir".into(),
                    matcher: AttributeMatcher::DashMatch("ltr".into()),
                    case_insensitive: true,
                }),
                SimpleSelector::Attribute(AttributeSelector {
                    name: "href".into(),
                    matcher: AttributeMatcher::Present,
                    case_insensitive: false,
                }),
            ]
        );
    }

    #[test]
    fn an_plus_b() {
        for (text, expected) in [
            ("odd", (2, 1)),
            ("EVEN", (2, 0)),
            ("3", (0, 3)),
            ("-3", (0, -3)),
            ("n", (1, 0)),
            ("-n+3", (-1, 3)),
            ("+n", (1, 0)),
            ("2n+1", (2, 1)),
            ("2n + 1", (2, 1)),
            ("2n- 1", (2, -1)),
            (" 3n -2 ", (3, -2)),
        ] {
            assert_eq!(parse_an_plus_b(text), Some(expected), "{text}");
        }
        for text in ["", "2 n", "n+", "2n+-1", "x", "2n1", "- n"] {
            assert_eq!(parse_an_plus_b(text), None, "{text}");
        }
    }

    #[test]
    fn specificity_follows_blink() {
        assert_eq!(spec("*"), (0, 0, 0));
        assert_eq!(spec("div"), (0, 0, 1));
        assert_eq!(spec(".a.a"), (0, 2, 0));
        assert_eq!(spec(":hover:hover"), (0, 2, 0));
        assert_eq!(spec("#x .b:hover"), (1, 2, 0));
        assert_eq!(spec(":where(#a) .b"), (0, 1, 0));
        assert_eq!(spec(":is(.a, #b) div"), (1, 0, 1));
        assert_eq!(spec("span:not(.x):nth-child(2n+1 of .y)"), (0, 3, 1));
        assert_eq!(spec(":has(> .q #r)"), (1, 1, 0));
        assert_eq!(spec(".a, #b"), (1, 0, 0));
        let many = ".c".repeat(300);
        assert_eq!(spec(&many), (0, 255, 0));
    }

    #[test]
    fn nesting() {
        let parent = parse_selector_list(".a, #b", None).unwrap();
        let nested = |text: &str| {
            Specificity::maximum(&parse_selector_list(text, Some(&parent)).unwrap()).tuple()
        };
        assert_eq!(nested(".c"), (1, 1, 0));
        assert_eq!(nested("&:hover"), (1, 1, 0));
        assert_eq!(nested("> .c"), (1, 1, 0));
        assert_eq!(nested(".c &"), (1, 1, 0));
        assert_eq!(nested(":is(.c)"), (1, 1, 0));
        // `&` inside `:is()` makes the selector absolute: no implicit `& `.
        assert_eq!(nested(":is(&) .c"), (1, 1, 0));
        let absolute = &parse_selector_list(":not(&) .c", Some(&parent)).unwrap()[0];
        assert_eq!(absolute.ancestors.len(), 1);
        let relative = &parse_selector_list("> .c", Some(&parent)).unwrap()[0];
        assert_eq!(relative.ancestors[0].0, Combinator::Child);
        let explicit = &parse_selector_list(".c &", Some(&parent)).unwrap()[0];
        assert_eq!(explicit.subject, nesting_compound(&parent));
    }

    #[test]
    fn unsupported_selectors_are_errors() {
        assert!(error("p::before").contains("pseudo-elements"));
        assert!(error("a:visited").contains("unsupported pseudo-class `:visited`"));
        assert!(error(":nth-of-type(2)").contains("unsupported"));
        assert!(error("svg|a").contains("namespaces"));
        assert!(error("&.a").contains("nested rule"));
        assert!(error("> a").contains("combinator"));
        assert!(error(":has(:has(a))").contains("cannot contain"));
        assert!(error(":is()").contains("needs a selector"));
        assert!(error("a,").contains("expected a selector"));
        assert!(error("a$").contains("unexpected"));
        assert!(error("[x=]").contains("attribute value"));
        assert!(error("[x=y s]").contains("flag `s`"));
        assert!(error(".1a").contains("class name"));
    }
}
