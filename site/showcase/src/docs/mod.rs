//! The docs examples. Each file `<name>.rs` here has `pub fn example()`,
//! and the site shows the whole file next to the image this renderer draws
//! from it (`docs-<name>-light.png`, see `site/src/components/Example.astro`).
//! So every example on the site compiles against the checked-out `openui` and
//! is drawn by it.
//!
//! An example is drawn at the width given here, in a white page with 20px of
//! padding and Geist as the font, and nothing else set; the image is cropped
//! to the example's height.
// Components are functions with capitalized names.
#![allow(non_snake_case)]

pub struct DocExample {
    pub name: &'static str,
    pub width: f64,
    pub build: fn() -> openui::prelude::ViewNode,
}

macro_rules! examples {
    ($($name:ident: $width:literal),* $(,)?) => {
        $(pub mod $name;)*
        pub const EXAMPLES: &[DocExample] = &[
            $(DocExample { name: stringify!($name), width: $width as f64, build: $name::example },)*
        ];
    };
}

examples! {
    cc_views: 480,
    cc_styling: 480,
    cc_flex: 480,
    cc_grid: 480,
    cc_fonts: 480,
    cc_events: 480,
    el_no_defaults: 480,
    el_div: 480,
    el_lists: 480,
    el_span: 480,
    el_br: 480,
    el_ruby: 480,
    el_table: 480,
    el_img: 480,
    el_button: 480,
    el_input_text: 480,
    el_input_choice: 480,
    el_input_other: 480,
    el_textarea: 480,
    el_select: 480,
    el_meter_progress: 480,
    el_details: 480,
    el_fieldset: 480,
    el_form: 480,
    el_embedded: 480,
}
