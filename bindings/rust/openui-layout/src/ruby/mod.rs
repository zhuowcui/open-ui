//! Ruby annotation layout — extracted from Blink's ruby layout algorithm.
//!
//! Implements the CSS Ruby Annotation Layout Module Level 1:
//! <https://drafts.csswg.org/css-ruby-1/>
//!
//! Ruby annotations are small runs of text rendered alongside base characters,
//! used primarily for East Asian typography:
//! - Japanese furigana (reading aids above kanji)
//! - Chinese pinyin / zhuyin (pronunciation guides)
//! - Korean hanja annotations
//!
//! Source files studied:
//! - `core/layout/layout_ruby_run.cc`
//! - `core/layout/layout_ruby_base.cc`
//! - `core/layout/layout_ruby_text.cc`
//! - `core/layout/ng/inline/ng_ruby_utils.cc`

mod layout;

pub use layout::{
    compute_ruby_layout, max_ruby_overhang, clamp_overhang, RubyInfo, RubyLayout,
};
