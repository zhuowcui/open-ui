//! WPT test modules — auto-generated

use openui_dom::Document;

pub mod wpt_css2_floats;
pub mod wpt_css_backgrounds;
pub mod wpt_css_box;
pub mod wpt_css_break;
pub mod wpt_css_display;
pub mod wpt_css_flexbox;
pub mod wpt_css_multicol;
pub mod wpt_css_overflow;
pub mod wpt_css_position;
pub mod wpt_css_sizing;

pub fn all_wpt_registry() -> Vec<(&'static str, fn() -> Document)> {
    let mut all = Vec::new();
    all.extend(wpt_css2_floats::css2_floats_registry());
    all.extend(wpt_css_position::css_position_registry());
    all.extend(wpt_css_flexbox::css_flexbox_registry());
    all.extend(wpt_css_multicol::css_multicol_registry());
    all.extend(wpt_css_overflow::css_overflow_registry());
    all.extend(wpt_css_sizing::css_sizing_registry());
    all.extend(wpt_css_break::css_break_registry());
    all.extend(wpt_css_display::css_display_registry());
    all.extend(wpt_css_box::css_box_registry());
    all.extend(wpt_css_backgrounds::css_backgrounds_registry());
    all
}
