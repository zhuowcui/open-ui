//! CSS declaration blocks, read at macro time (RFC 0001).
//!
//! `style!` hands its CSS text to [`parse_declaration_block`], which splits it
//! into declarations the way CSS Syntax Level 3 does and parses each value
//! with the same [`parse_literal`] the runtime uses. The set of accepted values
//! is therefore exactly what `parse_literal` accepts. This module exists only
//! with the internal `constructor-tokens` feature: the runtime never reads CSS
//! text.

use std::ops::Range;

use crate::{parse_literal, StyleProperty, StyleValue};

/// One declaration of a block, with its byte range in the source text.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockDeclaration {
    pub property: StyleProperty,
    pub value: StyleValue,
    pub important: bool,
    pub source: Range<usize>,
}

/// A declaration the block cannot accept, with its byte range in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockError {
    pub message: String,
    pub source: Range<usize>,
}

impl BlockError {
    fn new(message: impl Into<String>, source: Range<usize>) -> Self {
        Self {
            message: message.into(),
            source,
        }
    }
}

/// Parse `property: value; ...` into typed declarations.
///
/// Every error in the block is reported, not only the first. Unlike a
/// browser, which drops an invalid declaration and keeps the rest, a macro
/// rejects the block: a declaration that silently does nothing is a bug.
pub fn parse_declaration_block(text: &str) -> Result<Vec<BlockDeclaration>, Vec<BlockError>> {
    let cleaned = match strip_comments(text) {
        Ok(cleaned) => cleaned,
        Err(error) => return Err(vec![error]),
    };
    let mut declarations = Vec::new();
    let mut errors = Vec::new();
    for segment in split_top_level(&cleaned, b';') {
        let segment = match segment {
            Ok(segment) => segment,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        let source = trimmed_range(&cleaned, segment);
        if source.is_empty() {
            continue;
        }
        match parse_declaration(&cleaned, source.clone()) {
            Ok(declaration) => declarations.push(declaration),
            Err(error) => errors.push(error),
        }
    }
    if errors.is_empty() {
        Ok(declarations)
    } else {
        Err(errors)
    }
}

fn parse_declaration(text: &str, source: Range<usize>) -> Result<BlockDeclaration, BlockError> {
    let declaration = &text[source.clone()];
    let error = |message: String| BlockError::new(message, source.clone());
    if top_level_position(declaration, b'{').is_some() {
        return Err(error(format!(
            "`{declaration}`: a declaration block holds no nested rules"
        )));
    }
    let Some(colon) = top_level_position(declaration, b':') else {
        return Err(error(format!(
            "`{declaration}`: expected `property: value`"
        )));
    };
    let name = declaration[..colon].trim();
    if name.starts_with("--") {
        return Err(error(format!(
            "`{name}`: custom properties are not supported yet"
        )));
    }
    let lowercase = name.to_ascii_lowercase();
    let Some(property) = StyleProperty::from_css_name(&lowercase) else {
        return Err(error(format!("unknown Open UI style property `{name}`")));
    };
    let (value, important) = split_important(declaration[colon + 1..].trim());
    if value.is_empty() {
        return Err(error(format!("`{name}` has no value")));
    }
    let value_text = collapse_whitespace(value);
    let value = parse_literal(property, &value_text).map_err(|literal| error(literal.0))?;
    Ok(BlockDeclaration {
        property,
        value,
        important,
        source,
    })
}

/// The value without a trailing `!important` (any ASCII case, with optional
/// whitespace after the `!`), and whether it had one.
fn split_important(value: &str) -> (&str, bool) {
    let bytes = value.as_bytes();
    const IMPORTANT: &[u8] = b"important";
    if bytes.len() >= IMPORTANT.len()
        && bytes[bytes.len() - IMPORTANT.len()..].eq_ignore_ascii_case(IMPORTANT)
    {
        let rest = value[..value.len() - IMPORTANT.len()].trim_end();
        if let Some(rest) = rest.strip_suffix('!') {
            return (rest.trim_end(), true);
        }
    }
    (value, false)
}

/// The text with each comment's bytes replaced by spaces, so byte offsets
/// still point into the caller's text. A comment separates tokens, which a
/// space does too.
fn strip_comments(text: &str) -> Result<String, BlockError> {
    let mut bytes = text.as_bytes().to_vec();
    let mut index = 0;
    let mut quote = None;
    while index < bytes.len() {
        let byte = bytes[index];
        match quote {
            Some(open) => {
                if byte == b'\\' {
                    index += 1;
                } else if byte == open {
                    quote = None;
                }
            }
            None if byte == b'"' || byte == b'\'' => quote = Some(byte),
            None if byte == b'/' && bytes.get(index + 1) == Some(&b'*') => {
                let start = index;
                let Some(length) = text[index + 2..].find("*/") else {
                    return Err(BlockError::new("unterminated comment", start..text.len()));
                };
                let end = index + 2 + length + 2;
                bytes[start..end].fill(b' ');
                index = end;
                continue;
            }
            None => {}
        }
        index += 1;
    }
    if quote.is_some() {
        return Err(BlockError::new("unterminated string", 0..text.len()));
    }
    Ok(String::from_utf8(bytes).expect("only ASCII bytes were replaced"))
}

/// Byte ranges between top-level `separator` bytes, outside strings and
/// brackets. An unbalanced bracket is an error for its segment.
fn split_top_level(text: &str, separator: u8) -> Vec<Result<Range<usize>, BlockError>> {
    let bytes = text.as_bytes();
    let mut segments = Vec::new();
    let mut start = 0;
    let mut depth: Vec<u8> = Vec::new();
    let mut quote = None;
    let mut unbalanced = None;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(open) = quote {
            if byte == b'\\' {
                index += 1;
            } else if byte == open {
                quote = None;
            }
        } else {
            match byte {
                b'"' | b'\'' => quote = Some(byte),
                b'(' => depth.push(b')'),
                b'[' => depth.push(b']'),
                b'{' => depth.push(b'}'),
                b')' | b']' | b'}' => {
                    if depth.last() == Some(&byte) {
                        depth.pop();
                    } else {
                        unbalanced.get_or_insert(index);
                    }
                }
                _ if byte == separator && depth.is_empty() => {
                    segments.push(segment(start..index, unbalanced.take()));
                    start = index + 1;
                }
                _ => {}
            }
        }
        index += 1;
    }
    if !depth.is_empty() {
        unbalanced.get_or_insert(bytes.len());
    }
    segments.push(segment(start..bytes.len(), unbalanced));
    segments
}

fn segment(range: Range<usize>, unbalanced: Option<usize>) -> Result<Range<usize>, BlockError> {
    match unbalanced {
        None => Ok(range),
        Some(_) => Err(BlockError::new("unbalanced bracket", range)),
    }
}

/// The byte position of the first top-level `target`, outside strings and
/// brackets.
fn top_level_position(text: &str, target: u8) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut quote = None;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(open) = quote {
            if byte == b'\\' {
                index += 1;
            } else if byte == open {
                quote = None;
            }
        } else if byte == target && depth == 0 {
            return Some(index);
        } else {
            match byte {
                b'"' | b'\'' => quote = Some(byte),
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        index += 1;
    }
    None
}

fn trimmed_range(text: &str, range: Range<usize>) -> Range<usize> {
    let slice = &text[range.clone()];
    let start = range.start + (slice.len() - slice.trim_start().len());
    let end = range.end - (slice.len() - slice.trim_end().len());
    start..end.max(start)
}

fn is_css_whitespace(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{c}')
}

/// Each run of CSS whitespace outside strings becomes one space, as the CSS
/// tokenizer makes it one whitespace token.
fn collapse_whitespace(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut quote = None;
    let mut escaped = false;
    let mut pending_space = false;
    for character in value.chars() {
        if let Some(open) = quote {
            out.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == open {
                quote = None;
            }
            continue;
        }
        if is_css_whitespace(character) {
            pending_space = true;
            continue;
        }
        if pending_space && !out.is_empty() {
            out.push(' ');
        }
        pending_space = false;
        if character == '"' || character == '\'' {
            quote = Some(character);
        }
        out.push(character);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Display, Edges, LengthValue};

    fn parse(text: &str) -> Vec<BlockDeclaration> {
        parse_declaration_block(text).unwrap_or_else(|errors| panic!("{text}: {errors:?}"))
    }

    fn messages(text: &str) -> Vec<String> {
        parse_declaration_block(text)
            .unwrap_err()
            .into_iter()
            .map(|error| error.message)
            .collect()
    }

    #[test]
    fn declarations_parse_with_the_runtime_literal_parser() {
        let block = "padding: 8px 16px; display: flex;\n opacity: 0.25";
        let declarations = parse(block);
        assert_eq!(declarations.len(), 3);
        assert_eq!(declarations[0].property, StyleProperty::Padding);
        assert_eq!(
            declarations[0].value,
            parse_literal(StyleProperty::Padding, "8px 16px").unwrap()
        );
        assert_eq!(
            declarations[0].value,
            StyleValue::from(Edges {
                top: LengthValue::px(8.0),
                right: LengthValue::px(16.0),
                bottom: LengthValue::px(8.0),
                left: LengthValue::px(16.0),
            })
        );
        assert_eq!(declarations[1].value, StyleValue::from(Display::Flex));
        assert_eq!(&block[declarations[2].source.clone()], "opacity: 0.25");
    }

    #[test]
    fn separators_inside_strings_brackets_and_comments_do_not_split() {
        let declarations = parse(
            "font-family: \"a;b\", 'c:d' /* ; */; \
             color: rgb(0 0 0 / 0.5); /* trailing: x; */",
        );
        assert_eq!(declarations.len(), 2);
        assert_eq!(
            declarations[0].value,
            parse_literal(StyleProperty::FontFamily, "\"a;b\", 'c:d'").unwrap()
        );
    }

    #[test]
    fn whitespace_runs_collapse_outside_strings() {
        assert_eq!(collapse_whitespace("8px \n\t 16px"), "8px 16px");
        assert_eq!(collapse_whitespace("\"a  b\"   c"), "\"a  b\" c");
        let declarations = parse("padding:\n    8px\n    16px");
        assert_eq!(
            declarations[0].value,
            parse_literal(StyleProperty::Padding, "8px 16px").unwrap()
        );
    }

    #[test]
    fn property_names_are_ascii_case_insensitive() {
        assert_eq!(parse("Display: flex")[0].property, StyleProperty::Display);
    }

    #[test]
    fn important_is_recognized_in_any_case_and_spacing() {
        for block in [
            "display: flex !important",
            "display: flex!IMPORTANT",
            "display: flex ! important",
            "display: flex /* x */ !important",
        ] {
            let declarations = parse(block);
            assert!(declarations[0].important, "{block}");
            assert_eq!(declarations[0].value, StyleValue::from(Display::Flex));
        }
        assert!(!parse("display: flex")[0].important);
    }

    #[test]
    fn empty_declarations_are_skipped() {
        assert!(parse("").is_empty());
        assert!(parse(" ; ;; /* only */ ").is_empty());
        assert_eq!(parse(";display: flex;;").len(), 1);
    }

    #[test]
    fn every_invalid_declaration_is_reported() {
        let errors = messages("colr: red; display: sideways; width; --x: 1; opacity: 1");
        assert_eq!(errors.len(), 4, "{errors:?}");
        assert!(errors[0].contains("unknown Open UI style property `colr`"));
        assert!(errors[1].contains("sideways"), "{}", errors[1]);
        assert!(errors[2].contains("expected `property: value`"));
        assert!(errors[3].contains("custom properties"));
    }

    #[test]
    fn malformed_text_is_an_error() {
        assert_eq!(messages("color: red /* open"), ["unterminated comment"]);
        assert_eq!(messages("font-family: \"open"), ["unterminated string"]);
        assert_eq!(messages("color: rgb(0 0 0"), ["unbalanced bracket"]);
        assert_eq!(messages("width: 1px)"), ["unbalanced bracket"]);
        assert!(messages("&:hover { color: red }")[0].contains("no nested rules"));
        assert!(messages("display:")[0].contains("has no value"));
    }

    #[test]
    fn error_ranges_point_at_the_declaration() {
        let block = "display: flex; colr: red";
        let errors = parse_declaration_block(block).unwrap_err();
        assert_eq!(&block[errors[0].source.clone()], "colr: red");
    }
}
