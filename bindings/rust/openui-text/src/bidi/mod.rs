//! Bidirectional text support — UAX#9 integration.
//!
//! Mirrors Blink's BiDi support from
//! `third_party/blink/renderer/platform/text/bidi_paragraph.h`.
//!
//! Uses the `unicode-bidi` crate for UAX#9 paragraph analysis and
//! provides run segmentation and visual reordering for inline layout.

mod paragraph;

pub use paragraph::{BidiParagraph, BidiRun};
