//! pixel_compare — Renders test patterns to PNG for comparison with Chromium.
//!
//! Usage:
//!   pixel_compare list                              # List all test IDs
//!   pixel_compare build-source-identity             # Source recorded by Cargo
//!   pixel_compare render <test_id> <output.png> [--viewport WxH] [--scale N]
//!       [--raster-config PROFILE] [--backend cpu-skia|ganesh-gl]
//!   pixel_compare render-all <output_dir> [--viewport WxH] [--scale N]
//!       [--raster-config PROFILE] [--backend cpu-skia|ganesh-gl]
//!
//! Each test ID corresponds to a specific CSS feature variant that has
//! a matching HTML file rendered by Chrome for pixel-by-pixel comparison.

use openui_compositor::SoftwareCompositor;
use openui_dom::{ElementTag, PseudoElementKind};
use openui_engine::{Engine, EngineError, EngineOptions, NodeHandle as NodeId, RendererNodeState};
use openui_geometry::{Length, RasterBackend, RasterConfiguration, ViewportMetrics};
use openui_style::*;
use std::sync::OnceLock;

// Generated WPT fixtures are kept byte-for-byte by the porter, not rustfmt.
#[rustfmt::skip]
mod wpt;

const LEGACY_WIDTH: f64 = 800.0;
const LEGACY_HEIGHT: f64 = 600.0;
const LEGACY_SCALE: f64 = 1.0;

static ACTIVE_VIEWPORT: OnceLock<ViewportMetrics> = OnceLock::new();
static ACTIVE_RASTER_CONFIGURATION: OnceLock<RasterConfiguration> = OnceLock::new();

/// Narrow fixture-construction facade. Every mutation is immediately routed
/// through a public `Engine` API; it intentionally exposes neither the native
/// document nor mutable computed styles.
pub struct FixtureEngine {
    inner: Engine,
    authored_scrollbar_widths: std::collections::HashMap<NodeId, ScrollbarWidth>,
}

impl FixtureEngine {
    fn new(viewport: ViewportMetrics) -> Result<Self, EngineError> {
        let mut inner = Engine::new_with_font_collection_and_options(
            viewport,
            openui_text::FontCollection::deterministic_test(),
            EngineOptions {
                raster_configuration: active_raster_configuration(),
            },
        )?;
        // The immutable capture styles hide webkit scrollbar pseudo-elements.
        // Standard non-auto width/color declarations take precedence over
        // those pseudo-elements in Chromium. Resolve that capture policy in
        // this adapter; native scrollbar-width:none always remains hidden.
        inner.set_renderer_style(
            inner.root(),
            RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
        )?;
        Ok(Self {
            inner,
            authored_scrollbar_widths: std::collections::HashMap::new(),
        })
    }

    fn into_engine(self) -> Engine {
        self.inner
    }

    fn root(&self) -> NodeId {
        self.inner.root()
    }

    fn create_node(&mut self, tag: ElementTag) -> NodeId {
        let node = self.inner.create_element(tag).expect("fixture element");
        self.inner
            .set_renderer_style(
                node,
                RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
            )
            .expect("fixture capture-harness scrollbar style");
        node
    }

    fn append_child(&mut self, parent: NodeId, child: NodeId) {
        self.inner
            .append_child(parent, child)
            .expect("fixture append")
    }

    fn set_attribute(&mut self, node: NodeId, name: impl Into<String>, value: impl Into<String>) {
        self.inner
            .set_attribute(node, name, value)
            .expect("fixture attribute")
    }

    fn set_style(&mut self, node: NodeId, value: RendererStyleValue) {
        let affects_scrollbars = matches!(
            &value,
            RendererStyleValue::ScrollbarWidth(_)
                | RendererStyleValue::ScrollbarThumbColor(_)
                | RendererStyleValue::ScrollbarTrackColor(_)
        );
        if let RendererStyleValue::ScrollbarWidth(width) = &value {
            self.authored_scrollbar_widths.insert(node, *width);
        }
        self.inner
            .set_renderer_style(node, value)
            .expect("schema-validated fixture style");
        if affects_scrollbars {
            let authored = self
                .authored_scrollbar_widths
                .get(&node)
                .copied()
                .unwrap_or(ScrollbarWidth::Auto);
            let computed = self.inner.computed_style(node).expect("fixture style read");
            let effective = if authored != ScrollbarWidth::Auto
                || computed.scrollbar_thumb_color.is_some()
                || computed.scrollbar_track_color.is_some()
            {
                authored
            } else {
                ScrollbarWidth::None
            };
            self.inner
                .set_renderer_style(node, RendererStyleValue::ScrollbarWidth(effective))
                .expect("capture scrollbar precedence");
        }
    }

    fn set_internal_style(&mut self, node: NodeId, value: RendererInternalStyleValue) {
        self.inner
            .set_internal_style(node, value)
            .expect("fixture internal style")
    }

    fn install_derived_style(&mut self, node: NodeId, value: ComputedStyle) {
        self.inner
            .install_derived_style(node, value)
            .expect("fixture derived style")
    }

    fn set_node_state(&mut self, node: NodeId, value: RendererNodeState) {
        self.inner
            .set_renderer_node_state(node, value)
            .expect("fixture node state")
    }

    fn computed_style(&self, node: NodeId) -> &ComputedStyle {
        self.inner.computed_style(node).expect("fixture style read")
    }

    fn insert_pseudo_element(&mut self, origin: NodeId, kind: PseudoElementKind) -> NodeId {
        self.inner
            .insert_renderer_pseudo(origin, kind)
            .expect("fixture pseudo")
    }

    fn materialize_generated_content(&mut self) {
        self.inner
            .materialize_generated_content()
            .expect("fixture generated content")
    }

    fn register_image_resource(
        &mut self,
        source: impl Into<String>,
        mime_type: impl Into<String>,
        sha256: impl Into<String>,
        bytes: impl Into<Vec<u8>>,
    ) -> ImageResourceId {
        self.inner
            .register_image_resource(source, mime_type, sha256, bytes)
    }

    fn font_collection(&self) -> &std::sync::Arc<openui_text::FontCollection> {
        self.inner.font_collection()
    }

    fn set_legacy_canvas_body(&mut self, body: NodeId) {
        self.inner
            .set_legacy_canvas_body(body)
            .expect("fixture legacy canvas body")
    }
}

fn legacy_viewport() -> ViewportMetrics {
    ViewportMetrics::from_logical_size(LEGACY_WIDTH, LEGACY_HEIGHT, LEGACY_SCALE)
        .expect("legacy qualification viewport must be valid")
}

fn active_viewport() -> ViewportMetrics {
    *ACTIVE_VIEWPORT.get_or_init(legacy_viewport)
}

fn active_raster_configuration() -> RasterConfiguration {
    *ACTIVE_RASTER_CONFIGURATION.get_or_init(RasterConfiguration::default)
}

/// Resolve a semantic viewport-relative declaration for a generated fixture.
/// R6 replaces the legacy fixed CLI profile with a profile-selected value;
/// keeping the resolution behind this boundary prevents the porter from ever
/// baking 800x600 constants into generated source again.
pub fn fixture_viewport_length(value: LengthValue) -> Length {
    Length::px(fixture_viewport_px(value))
}

/// Resolve a semantic viewport-relative value to CSS pixels for generated
/// fields, such as transform translations, that do not store a `Length`.
pub fn fixture_viewport_px(value: LengthValue) -> f32 {
    let viewport = active_viewport();
    value
        .resolve(
            (
                viewport.logical_width() as f32,
                viewport.logical_height() as f32,
            ),
            16.0,
            16.0,
        )
        .value()
}

/// Resolve `calc(<viewport-length> + <px-offset>)` without baking the legacy
/// 800x600 fixture viewport into generated source.
pub fn fixture_viewport_calc(value: LengthValue, px_offset: f32) -> Length {
    Length::px(fixture_viewport_px(value) + px_offset)
}

fn parse_viewport_options(
    options: &[String],
) -> Result<(ViewportMetrics, RasterConfiguration), String> {
    let mut logical_width = LEGACY_WIDTH;
    let mut logical_height = LEGACY_HEIGHT;
    let mut scale = LEGACY_SCALE;
    let mut raster_configuration = RasterConfiguration::default();
    let mut ganesh = false;
    let mut index = 0;
    while index < options.len() {
        match options[index].as_str() {
            "--viewport" => {
                let value = options
                    .get(index + 1)
                    .ok_or_else(|| "--viewport requires WIDTHxHEIGHT".to_string())?;
                (logical_width, logical_height) = parse_logical_size(value)?;
                index += 2;
            }
            "--scale" => {
                let value = options
                    .get(index + 1)
                    .ok_or_else(|| "--scale requires a number".to_string())?;
                scale = value
                    .parse::<f64>()
                    .map_err(|_| format!("invalid device scale: {value}"))?;
                index += 2;
            }
            "--raster-config" => {
                let value = options
                    .get(index + 1)
                    .ok_or_else(|| "--raster-config requires a profile".to_string())?;
                raster_configuration = match value.as_str() {
                    "default" => RasterConfiguration::default(),
                    "deterministic-alias" => RasterConfiguration::deterministic_aliased(false),
                    "deterministic-alias-subpixel" => {
                        RasterConfiguration::deterministic_aliased(true)
                    }
                    "chromium-linux-lcd" => RasterConfiguration::chromium_linux_lcd(),
                    "legacy-deterministic-alias" => {
                        RasterConfiguration::legacy_deterministic_aliased(false)
                    }
                    "legacy-deterministic-alias-subpixel" => {
                        RasterConfiguration::legacy_deterministic_aliased(true)
                    }
                    "legacy-chromium-linux-lcd" => RasterConfiguration::legacy_chromium_linux_lcd(),
                    _ => return Err(format!("unknown raster configuration: {value}")),
                };
                index += 2;
            }
            "--backend" => {
                let value = options
                    .get(index + 1)
                    .ok_or_else(|| "--backend requires cpu-skia or ganesh-gl".to_string())?;
                ganesh = match value.as_str() {
                    "cpu-skia" => false,
                    "ganesh-gl" => true,
                    _ => return Err(format!("unknown raster backend: {value}")),
                };
                index += 2;
            }
            option => return Err(format!("unknown render option: {option}")),
        }
    }
    Ok((
        ViewportMetrics::from_logical_size(logical_width, logical_height, scale)
            .map_err(|error| error.to_string())?,
        if ganesh {
            raster_configuration.with_backend(RasterBackend::GaneshGl)
        } else {
            raster_configuration
        },
    ))
}

fn parse_logical_size(value: &str) -> Result<(f64, f64), String> {
    let (width, height) = value
        .split_once(['x', 'X'])
        .ok_or_else(|| format!("invalid viewport {value:?}; expected WIDTHxHEIGHT"))?;
    let width = width
        .parse::<f64>()
        .map_err(|_| format!("invalid viewport width: {width}"))?;
    let height = height
        .parse::<f64>()
        .map_err(|_| format!("invalid viewport height: {height}"))?;
    Ok((width, height))
}

fn install_viewport(options: &[String]) -> ViewportMetrics {
    let (viewport, raster_configuration) =
        parse_viewport_options(options).unwrap_or_else(|error| {
            eprintln!("Invalid render profile: {error}");
            std::process::exit(2);
        });
    if let Err(existing) = ACTIVE_VIEWPORT.set(viewport) {
        if existing != active_viewport() {
            eprintln!("A process may render only one viewport profile");
            std::process::exit(2);
        }
    }
    if let Err(existing) = ACTIVE_RASTER_CONFIGURATION.set(raster_configuration) {
        if existing != active_raster_configuration() {
            eprintln!("A process may render only one raster configuration");
            std::process::exit(2);
        }
    }
    viewport
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("build-source-identity") => print!(
            "{}",
            include_str!(concat!(env!("OUT_DIR"), "/renderer-build-identity.json"))
        ),
        Some("list") => {
            for (id, _) in registry() {
                println!("{}", id);
            }
        }
        Some("raster-identity") => print_raster_identity(&args[2..]),
        Some("debug") => {
            let test_id = args.get(2).expect("Usage: pixel_compare debug <test_id>");
            let viewport = install_viewport(&args[3..]);
            let tests = registry();
            if let Some((_, builder)) = tests.iter().find(|(id, _)| *id == test_id) {
                let mut doc = builder(viewport).expect("fixture build");
                let root = doc.root();
                let children = doc.children(root).expect("fixture children");
                println!("Root node has {} children", children.len());
                for (i, &child) in children.iter().enumerate() {
                    let style = doc.computed_style(child).expect("fixture style");
                    println!(
                        "  child[{}]: display={:?} w={:?} h={:?} bg={:?}",
                        i, style.display, style.width, style.height, style.background_color
                    );
                    let grandchildren = doc.children(child).expect("fixture grandchildren");
                    println!("    {} grandchildren", grandchildren.len());
                    for (j, &gc) in grandchildren.iter().enumerate().take(3) {
                        let gs = doc.computed_style(gc).expect("fixture style");
                        println!(
                            "    gc[{}]: display={:?} float={:?} w={:?} h={:?} bg={:?}",
                            j, gs.display, gs.float, gs.width, gs.height, gs.background_color
                        );
                    }
                }
                let scene = doc.scene().expect("fixture scene");
                let fragment = scene.fragments();
                println!("Fragment size: {:?}", fragment.size);
                fn dump_frag(f: &openui_layout::Fragment, depth: usize) {
                    let indent = "  ".repeat(depth);
                    println!(
                        "{}frag: offset={:?} size={:?} margin={:?} overflow={:?} kind={:?} node={:?} baseline={} clip={} block_clip={} inline_clip={} skip_decoration={} decoration={:?} slice={:?} first={} last={} children={}",
                        indent,
                        f.offset,
                        f.size,
                        f.margin,
                        f.overflow_rect,
                        f.kind,
                        f.node_id,
                        f.baseline_offset,
                        f.has_overflow_clip,
                        f.block_axis_clip_only,
                        f.inline_axis_clip_only,
                        f.skip_box_decoration,
                        f.decoration_paint_block_size,
                        f.decoration_slice,
                        f.is_first_for_node,
                        f.is_last_for_node,
                        f.children.len()
                    );
                    if let Some(positioned) = &f.positioned_fragmentation {
                        println!(
                            "{}  positioned: static={:?} cb_offset={:?} cb_size={:?} inline_cb={:?} visual={:?} fi={:?}",
                            indent,
                            positioned.static_position,
                            positioned.containing_block_offset,
                            positioned.containing_block_size,
                            positioned.inline_containing_block_node,
                            positioned.visual_offset,
                            positioned.fragmentainer_index,
                        );
                    }
                    for child in &f.children {
                        dump_frag(child, depth + 1);
                    }
                }
                dump_frag(fragment, 0);
            } else {
                eprintln!("Unknown test ID: {}", test_id);
            }
        }
        Some("render") => {
            let test_id = args
                .get(2)
                .expect("Usage: pixel_compare render <test_id> <output.png>");
            let output = args
                .get(3)
                .expect("Usage: pixel_compare render <test_id> <output.png>");
            let viewport = install_viewport(&args[4..]);
            render_test(test_id, output, viewport);
        }
        Some("render-all") => {
            let dir = args
                .get(2)
                .expect("Usage: pixel_compare render-all <output_dir>");
            let viewport = install_viewport(&args[3..]);
            std::fs::create_dir_all(dir).unwrap();
            for (id, _) in registry() {
                let path = format!("{}/{}.png", dir, id);
                render_test(id, &path, viewport);
            }
            println!("Rendered {} tests to {}/", registry().len(), dir);
        }
        _ => {
            eprintln!(
                "Usage: pixel_compare <list|raster-identity|render|render-all> ... [--viewport WxH] [--scale N]"
            );
            std::process::exit(1);
        }
    }
}

fn render_test(test_id: &str, output: &str, viewport: ViewportMetrics) {
    let tests = registry();
    if let Some((_, builder)) = tests.iter().find(|(id, _)| *id == test_id) {
        let mut engine = builder(viewport).expect("fixture build failed");
        let scene = engine.scene().expect("scene construction failed");
        let png = if active_raster_configuration().backend == RasterBackend::GaneshGl {
            #[cfg(feature = "ganesh-gl")]
            {
                openui_compositor::GaneshGlCompositor::new()
                    .and_then(|mut compositor| compositor.render_png(&scene))
                    .expect("Ganesh render failed")
            }
            #[cfg(not(feature = "ganesh-gl"))]
            {
                panic!("ganesh-gl backend requested from a binary built without that feature")
            }
        } else {
            SoftwareCompositor::default()
                .render_png(&scene)
                .expect("render failed")
        };
        std::fs::write(output, png).expect("write failed");
        println!("OK: {} → {}", test_id, output);
    } else {
        eprintln!("Unknown test ID: {}", test_id);
        eprintln!("Use 'pixel_compare list' to see available tests.");
        std::process::exit(1);
    }
}

fn print_raster_identity(options: &[String]) {
    if options.len() != 2 || options[0] != "--backend" {
        eprintln!("Usage: pixel_compare raster-identity --backend cpu-skia|ganesh-gl");
        std::process::exit(2);
    }
    let identity = match options[1].as_str() {
        "cpu-skia" => openui_compositor::RasterBackendIdentity::cpu_skia(),
        "ganesh-gl" => {
            #[cfg(feature = "ganesh-gl")]
            {
                openui_compositor::GaneshGlCompositor::new()
                    .expect("Ganesh initialization failed")
                    .identity()
                    .clone()
            }
            #[cfg(not(feature = "ganesh-gl"))]
            {
                eprintln!("pixel_compare was built without the ganesh-gl feature");
                std::process::exit(2);
            }
        }
        value => {
            eprintln!("unknown raster backend: {value}");
            std::process::exit(2);
        }
    };
    let optional_json = |value: &Option<String>| {
        value
            .as_ref()
            .map_or_else(|| "null".to_string(), |value| format!("{value:?}"))
    };
    println!(
        "{{\"backend\":{:?},\"gl_renderer\":{},\"gl_version\":{},\"driver\":{},\"color_type\":{:?},\"sample_count\":{},\"surface_properties\":{:?}}}",
        identity.backend,
        optional_json(&identity.gl_renderer),
        optional_json(&identity.gl_version),
        optional_json(&identity.driver),
        identity.color_type,
        identity.sample_count,
        identity.surface_properties,
    );
}

type TestBuilder = fn(ViewportMetrics) -> Result<Engine, EngineError>;

/// Returns the full registry of test IDs → Document builders.
fn registry() -> Vec<(&'static str, TestBuilder)> {
    let mut tests = vec![
        // ── SP12 Display ─────────────────────────────────────────────
        (
            "sp12/display_outer_block",
            sp12_display_outer_block as TestBuilder,
        ),
        ("sp12/display_outer_inline", sp12_display_outer_inline),
        (
            "sp12/display_outer_inline_block",
            sp12_display_outer_inline_block,
        ),
        ("sp12/display_outer_none", sp12_display_outer_none),
        ("sp12/display_inner_flow_root", sp12_display_inner_flow_root),
        // ── SP12 Position ────────────────────────────────────────────
        ("sp12/position_static", sp12_position_static),
        ("sp12/position_relative", sp12_position_relative),
        ("sp12/position_absolute", sp12_position_absolute),
        ("sp12/position_fixed", sp12_position_fixed),
        // ── SP12 Float ───────────────────────────────────────────────
        ("sp12/float_left", sp12_float_left),
        ("sp12/float_right", sp12_float_right),
        ("sp12/float_none", sp12_float_none),
        ("sp12/clear_left", sp12_clear_left),
        ("sp12/clear_right", sp12_clear_right),
        ("sp12/clear_both", sp12_clear_both),
        // ── SP12 Box Model ───────────────────────────────────────────
        ("sp12/margin_positive", sp12_margin_positive),
        ("sp12/margin_negative", sp12_margin_negative),
        ("sp12/margin_auto", sp12_margin_auto),
        (
            "sp12/margin_collapsing_siblings",
            sp12_margin_collapsing_siblings,
        ),
        ("sp12/padding_basic", sp12_padding_basic),
        ("sp12/border_basic", sp12_border_basic),
        ("sp12/box_sizing_content_box", sp12_box_sizing_content_box),
        ("sp12/box_sizing_border_box", sp12_box_sizing_border_box),
        // ── SP12 Sizing ──────────────────────────────────────────────
        ("sp12/width_fixed_px", sp12_width_fixed_px),
        ("sp12/height_fixed_px", sp12_height_fixed_px),
        ("sp12/width_percent", sp12_width_percent),
        ("sp12/min_width", sp12_min_width),
        ("sp12/max_width", sp12_max_width),
        ("sp12/min_height", sp12_min_height),
        ("sp12/max_height", sp12_max_height),
        // ── SP12 Overflow ────────────────────────────────────────────
        ("sp12/overflow_visible", sp12_overflow_visible),
        ("sp12/overflow_hidden", sp12_overflow_hidden),
        // ── SP12 Flexbox ─────────────────────────────────────────────
        ("sp12/flex_direction_row", sp12_flex_direction_row),
        ("sp12/flex_direction_column", sp12_flex_direction_column),
        ("sp12/flex_justify_start", sp12_flex_justify_start),
        ("sp12/flex_justify_center", sp12_flex_justify_center),
        (
            "sp12/flex_justify_space_between",
            sp12_flex_justify_space_between,
        ),
        ("sp12/flex_align_center", sp12_flex_align_center),
        ("sp12/flex_align_stretch", sp12_flex_align_stretch),
        ("sp12/flex_wrap_basic", sp12_flex_wrap_basic),
        ("sp12/flex_grow_equal", sp12_flex_grow_equal),
        ("sp12/flex_gap", sp12_flex_gap),
        // ── SP12 Visual / Stacking ───────────────────────────────────
        ("sp12/z_index_stacking", sp12_z_index_stacking),
        ("sp12/opacity_basic", sp12_opacity_basic),
        ("sp12/border_radius", sp12_border_radius),
        ("sp12/visibility_hidden", sp12_visibility_hidden),
        ("sp12/nested_blocks", sp12_nested_blocks),
        // ── SP12 Sticky Positioning ──────────────────────────────────────
        ("sp12/position_sticky_top", sp12_position_sticky_top),
        ("sp12/position_sticky_bottom", sp12_position_sticky_bottom),
        // ── SP12 Multicol ────────────────────────────────────────────────
        ("sp12/multicol_2_columns", sp12_multicol_2_columns),
        ("sp12/multicol_column_width", sp12_multicol_column_width),
        ("sp12/multicol_column_gap", sp12_multicol_column_gap),
        // ── SP12 Flex Advanced ───────────────────────────────────────────
        (
            "sp12/flex_direction_row_reverse",
            sp12_flex_direction_row_reverse,
        ),
        (
            "sp12/flex_direction_column_reverse",
            sp12_flex_direction_column_reverse,
        ),
        ("sp12/flex_wrap_reverse", sp12_flex_wrap_reverse),
        (
            "sp12/flex_justify_space_around",
            sp12_flex_justify_space_around,
        ),
        (
            "sp12/flex_justify_space_evenly",
            sp12_flex_justify_space_evenly,
        ),
        // ── SP12 Margin Collapsing Advanced ──────────────────────────────
        (
            "sp12/margin_collapsing_parent_child",
            sp12_margin_collapsing_parent_child,
        ),
        (
            "sp12/margin_collapsing_through_empty",
            sp12_margin_collapsing_through_empty,
        ),
        // ── SP12 Overflow Axes ───────────────────────────────────────────
        ("sp12/overflow_scroll", sp12_overflow_scroll),
        ("sp12/overflow_auto", sp12_overflow_auto),
        // ── SP12 Aspect Ratio ────────────────────────────────────────────
        ("sp12/aspect_ratio_basic", sp12_aspect_ratio_basic),
        // ── SP11 Text Decoration ─────────────────────────────────────
        (
            "sp11/text_decoration_underline",
            sp11_text_decoration_underline,
        ),
        (
            "sp11/text_decoration_overline",
            sp11_text_decoration_overline,
        ),
        (
            "sp11/text_decoration_line_through",
            sp11_text_decoration_line_through,
        ),
        (
            "sp11/text_decoration_combined",
            sp11_text_decoration_combined,
        ),
        // ── SP11 Font Weight & Style ─────────────────────────────────
        ("sp11/font_weight_normal", sp11_font_weight_normal),
        ("sp11/font_weight_bold", sp11_font_weight_bold),
        ("sp11/font_style_normal", sp11_font_style_normal),
        ("sp11/font_style_italic", sp11_font_style_italic),
        // ── SP11 Font Size ───────────────────────────────────────────
        ("sp11/font_size_small", sp11_font_size_small),
        ("sp11/font_size_medium", sp11_font_size_medium),
        ("sp11/font_size_large", sp11_font_size_large),
        ("sp11/font_size_xlarge", sp11_font_size_xlarge),
        // ── SP11 Text Align ──────────────────────────────────────────
        ("sp11/text_align_left", sp11_text_align_left),
        ("sp11/text_align_center", sp11_text_align_center),
        ("sp11/text_align_right", sp11_text_align_right),
        ("sp11/text_align_justify", sp11_text_align_justify),
        // ── SP11 Text Transform ──────────────────────────────────────
        (
            "sp11/text_transform_uppercase",
            sp11_text_transform_uppercase,
        ),
        (
            "sp11/text_transform_lowercase",
            sp11_text_transform_lowercase,
        ),
        (
            "sp11/text_transform_capitalize",
            sp11_text_transform_capitalize,
        ),
        // ── SP11 Text Indent ─────────────────────────────────────────
        ("sp11/text_indent_positive", sp11_text_indent_positive),
        ("sp11/text_indent_negative", sp11_text_indent_negative),
        // ── SP11 Letter & Word Spacing ───────────────────────────────
        ("sp11/letter_spacing_positive", sp11_letter_spacing_positive),
        ("sp11/letter_spacing_negative", sp11_letter_spacing_negative),
        ("sp11/word_spacing_positive", sp11_word_spacing_positive),
        // ── SP11 Line Height ─────────────────────────────────────────
        ("sp11/line_height_normal", sp11_line_height_normal),
        ("sp11/line_height_number", sp11_line_height_number),
        ("sp11/line_height_length", sp11_line_height_length),
        // ── SP11 White Space ─────────────────────────────────────────
        ("sp11/white_space_normal", sp11_white_space_normal),
        ("sp11/white_space_nowrap", sp11_white_space_nowrap),
        ("sp11/white_space_pre", sp11_white_space_pre),
        ("sp11/white_space_pre_wrap", sp11_white_space_pre_wrap),
        ("sp11/white_space_pre_line", sp11_white_space_pre_line),
        // ── SP11 Color ───────────────────────────────────────────────
        ("sp11/color_red", sp11_color_red),
        ("sp11/color_blue", sp11_color_blue),
        ("sp11/color_green", sp11_color_green),
        ("sp11/color_custom", sp11_color_custom),
        // ── SP11 Text Shadow & Overflow ──────────────────────────────
        ("sp11/text_shadow_basic", sp11_text_shadow_basic),
        ("sp11/text_overflow_ellipsis", sp11_text_overflow_ellipsis),
        // ── SP11 Text Decoration Style ──────────────────────────────
        (
            "sp11/text_decoration_style_solid",
            sp11_text_decoration_style_solid,
        ),
        (
            "sp11/text_decoration_style_double",
            sp11_text_decoration_style_double,
        ),
        (
            "sp11/text_decoration_style_dotted",
            sp11_text_decoration_style_dotted,
        ),
        (
            "sp11/text_decoration_style_dashed",
            sp11_text_decoration_style_dashed,
        ),
        (
            "sp11/text_decoration_style_wavy",
            sp11_text_decoration_style_wavy,
        ),
        // ── SP11 Text Decoration Skip-Ink ───────────────────────────
        (
            "sp11/text_decoration_skip_ink_auto",
            sp11_text_decoration_skip_ink_auto,
        ),
        (
            "sp11/text_decoration_skip_ink_none",
            sp11_text_decoration_skip_ink_none,
        ),
        // ── SP11 Text Decoration Metrics ────────────────────────────
        (
            "sp11/text_decoration_color_red",
            sp11_text_decoration_color_red,
        ),
        (
            "sp11/text_decoration_thickness_3px",
            sp11_text_decoration_thickness_3px,
        ),
        (
            "sp11/text_underline_offset_5px",
            sp11_text_underline_offset_5px,
        ),
        // ── SP11 Font Family ────────────────────────────────────────
        ("sp11/font_family_serif", sp11_font_family_serif),
        ("sp11/font_family_monospace", sp11_font_family_monospace),
        // ── SP11 Word Spacing Negative ──────────────────────────────
        ("sp11/word_spacing_negative", sp11_word_spacing_negative),
        // ── SP11 Line Height Percentage ─────────────────────────────
        ("sp11/line_height_percentage", sp11_line_height_percentage),
        // ── SP11 Text Shadow Offset ─────────────────────────────────
        ("sp11/text_shadow_offset", sp11_text_shadow_offset),
        // ── SP13 Inline Basic ────────────────────────────────────────
        ("sp13/inline_single_span", sp13_inline_single_span),
        ("sp13/inline_multiple_spans", sp13_inline_multiple_spans),
        ("sp13/inline_nested_spans", sp13_inline_nested_spans),
        // ── SP13 Line Breaking ───────────────────────────────────────
        (
            "sp13/line_breaking_normal_wrap",
            sp13_line_breaking_normal_wrap,
        ),
        ("sp13/line_breaking_nowrap", sp13_line_breaking_nowrap),
        (
            "sp13/line_breaking_break_word",
            sp13_line_breaking_break_word,
        ),
        // ── SP13 Vertical Align ──────────────────────────────────────
        ("sp13/vertical_align_baseline", sp13_vertical_align_baseline),
        ("sp13/vertical_align_middle", sp13_vertical_align_middle),
        ("sp13/vertical_align_top", sp13_vertical_align_top),
        ("sp13/vertical_align_bottom", sp13_vertical_align_bottom),
        ("sp13/vertical_align_super", sp13_vertical_align_super),
        ("sp13/vertical_align_sub", sp13_vertical_align_sub),
        // ── SP13 Inline Block ────────────────────────────────────────
        ("sp13/inline_block_basic", sp13_inline_block_basic),
        (
            "sp13/inline_block_vertical_align",
            sp13_inline_block_vertical_align,
        ),
        // ── SP13 Mixed Content ───────────────────────────────────────
        ("sp13/mixed_block_inline", sp13_mixed_block_inline),
        // ── SP13 White Space Handling ────────────────────────────────
        ("sp13/white_space_collapsing", sp13_white_space_collapsing),
        ("sp13/white_space_preserving", sp13_white_space_preserving),
        // ── SP13 Inline Decoration ───────────────────────────────────
        ("sp13/inline_background_color", sp13_inline_background_color),
        ("sp13/inline_padding", sp13_inline_padding),
        ("sp13/inline_border", sp13_inline_border),
        // ── SP13 First-Letter / First-Line ─────────────────────────────
        ("sp13/first_letter_basic", sp13_first_letter_basic),
        ("sp13/first_line_basic", sp13_first_line_basic),
        // ── SP13 Word Break ────────────────────────────────────────────
        ("sp13/word_break_break_all", sp13_word_break_break_all),
        (
            "sp13/overflow_wrap_break_word",
            sp13_overflow_wrap_break_word,
        ),
        // ── SP13 Vertical Align Extended ───────────────────────────────
        ("sp13/vertical_align_text_top", sp13_vertical_align_text_top),
        (
            "sp13/vertical_align_text_bottom",
            sp13_vertical_align_text_bottom,
        ),
        // ── SP13 Line Breaking Extended ────────────────────────────────
        (
            "sp13/line_breaking_overflow_wrap",
            sp13_line_breaking_overflow_wrap,
        ),
        (
            "sp13/line_breaking_hyphens_auto",
            sp13_line_breaking_hyphens_auto,
        ),
        // ── SP13 Box Decoration Break ──────────────────────────────────
        ("sp13/inline_box_multiline", sp13_inline_box_multiline),
        // ── SP13 Float Interaction ─────────────────────────────────────
        ("sp13/inline_with_float", sp13_inline_with_float),
        // ── SP12 Sticky Positioning Extended ───────────────────────────────
        ("sp12/position_sticky_left", sp12_position_sticky_left),
        ("sp12/position_sticky_right", sp12_position_sticky_right),
        // ── SP12 Multicol Extended ──────────────────────────────────────────
        ("sp12/multicol_column_rule", sp12_multicol_column_rule),
        ("sp12/multicol_column_span", sp12_multicol_column_span),
        (
            "sp12/multicol_column_fill_auto",
            sp12_multicol_column_fill_auto,
        ),
        (
            "sp12/multicol_column_fill_balance",
            sp12_multicol_column_fill_balance,
        ),
        // ── SP12 Position Offsets ───────────────────────────────────────────
        (
            "sp12/position_absolute_top_left",
            sp12_position_absolute_top_left,
        ),
        (
            "sp12/position_absolute_bottom_right",
            sp12_position_absolute_bottom_right,
        ),
        (
            "sp12/position_relative_top_left",
            sp12_position_relative_top_left,
        ),
        (
            "sp12/position_absolute_percent",
            sp12_position_absolute_percent,
        ),
        // ── SP12 Overflow Axis ──────────────────────────────────────────────
        ("sp12/overflow_x_hidden", sp12_overflow_x_hidden),
        ("sp12/overflow_y_hidden", sp12_overflow_y_hidden),
        ("sp12/overflow_x_scroll", sp12_overflow_x_scroll),
        ("sp12/overflow_y_scroll", sp12_overflow_y_scroll),
        // ── SP12 Flex Alignment Extended ────────────────────────────────────
        ("sp12/flex_align_self_start", sp12_flex_align_self_start),
        ("sp12/flex_align_self_end", sp12_flex_align_self_end),
        ("sp12/flex_align_self_center", sp12_flex_align_self_center),
        (
            "sp12/flex_align_content_center",
            sp12_flex_align_content_center,
        ),
        (
            "sp12/flex_align_content_space_between",
            sp12_flex_align_content_space_between,
        ),
        ("sp12/flex_order", sp12_flex_order),
        ("sp12/flex_basis_100px", sp12_flex_basis_100px),
        ("sp12/flex_shrink_basic", sp12_flex_shrink_basic),
        // ── SP12 Border Variants ────────────────────────────────────────────
        ("sp12/border_style_dashed", sp12_border_style_dashed),
        ("sp12/border_style_dotted", sp12_border_style_dotted),
        ("sp12/border_color_per_side", sp12_border_color_per_side),
        // ── SP12 Fragmentation ──────────────────────────────────────────────
        (
            "sp12/fragmentation_break_before_column",
            sp12_fragmentation_break_before_column,
        ),
        (
            "sp12/fragmentation_break_inside_avoid",
            sp12_fragmentation_break_inside_avoid,
        ),
        // ── SP13 First-Letter / First-Line Extended ─────────────────────────
        ("sp13/first_letter_color", sp13_first_letter_color),
        ("sp13/first_letter_font_size", sp13_first_letter_font_size),
        ("sp13/first_line_font_weight", sp13_first_line_font_weight),
        ("sp13/first_line_color", sp13_first_line_color),
        // ── SP13 Inline Advanced ─────────────────────────────────────────────
        ("sp13/initial_letter_basic", sp13_initial_letter_basic),
        ("sp13/ruby_basic", sp13_ruby_basic),
        ("sp13/text_combine_upright", sp13_text_combine_upright),
        ("sp13/inline_direction_rtl", sp13_inline_direction_rtl),
        ("sp13/tab_size_4", sp13_tab_size_4),
        ("sp13/text_align_last_center", sp13_text_align_last_center),
        // ── SP11 Text Emphasis ───────────────────────────────────────────────
        ("sp11/text_emphasis_dot", sp11_text_emphasis_dot),
        ("sp11/text_emphasis_circle", sp11_text_emphasis_circle),
        (
            "sp11/text_emphasis_position_over",
            sp11_text_emphasis_position_over,
        ),
        ("sp11/text_emphasis_color_red", sp11_text_emphasis_color_red),
    ];
    // Extend with auto-generated WPT tests
    tests.extend(wpt::all_wpt_registry());
    tests
}

// ═══════════════════════════════════════════════════════════════════════
// ── Helpers ─────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// Create a Document with viewport padding and default text styles,
/// matching the HTML test file defaults:
///   body { margin: 0; padding: 20px; font-family: DejaVu Sans; font-size: 16px; }
pub fn base_doc(viewport_metrics: ViewportMetrics) -> (FixtureEngine, NodeId) {
    let mut doc = FixtureEngine::new(viewport_metrics).expect("fixture engine");
    let viewport = doc.root();
    // Viewport: no margin/padding, just a container matching screen dimensions
    doc.set_style(viewport, RendererStyleValue::Display(Display::Block));
    // BODY_STYLE also sets `html { overflow: hidden; }`; the body itself
    // remains visible below so its background and overflowing ink can propagate.
    doc.set_style(viewport, RendererStyleValue::OverflowX(Overflow::Hidden));
    doc.set_style(viewport, RendererStyleValue::OverflowY(Overflow::Hidden));
    // The raster surface itself supplies the initial white canvas. Keep the
    // synthetic viewport transparent so an explicitly emitted legacy `html`
    // background can be distinguished from that initial canvas color.
    doc.set_style(
        viewport,
        RendererStyleValue::BackgroundColor(Color::TRANSPARENT),
    );

    // Body: child of viewport, carries the default body padding.
    // Use display:flow-root (not overflow:hidden) to establish a BFC.
    // Per CSS spec, <body>'s overflow is propagated to the viewport, so
    // the body itself has overflow:visible — it must NOT clip content.
    // The viewport/canvas clips at 800×600 like Chrome's viewport does.
    let body = doc.create_node(ElementTag::Div);
    doc.set_style(body, RendererStyleValue::Display(Display::FlowRoot));
    doc.set_style(body, RendererStyleValue::PaddingTop(Length::px(20.0)));
    doc.set_style(body, RendererStyleValue::PaddingRight(Length::px(20.0)));
    doc.set_style(body, RendererStyleValue::PaddingBottom(Length::px(20.0)));
    doc.set_style(body, RendererStyleValue::PaddingLeft(Length::px(20.0)));
    doc.set_style(
        body,
        RendererStyleValue::FontFamily(FontFamilyList::single("DejaVu Sans")),
    );
    doc.set_style(body, RendererStyleValue::FontSize(16.0));
    doc.set_style(body, RendererStyleValue::Color(Color::BLACK));
    doc.append_child(viewport, body);
    (doc, body)
}

/// Create the opt-in browser-shaped document used by root/body WPT ports.
///
/// Unlike `base_doc()`, this keeps the viewport, document element, and body as
/// three distinct nodes.  Existing generated builders intentionally continue
/// to use the historical two-node helper.
pub fn root_doc(viewport_metrics: ViewportMetrics) -> (FixtureEngine, NodeId, NodeId) {
    let mut doc = FixtureEngine::new(viewport_metrics).expect("fixture engine");
    let viewport = doc.root();
    doc.set_style(viewport, RendererStyleValue::Display(Display::Block));

    let html = doc.create_node(ElementTag::Html);
    doc.set_style(html, RendererStyleValue::Display(Display::Block));
    doc.append_child(viewport, html);

    let body = doc.create_node(ElementTag::Body);
    doc.set_style(body, RendererStyleValue::Display(Display::Block));
    doc.append_child(html, body);
    (doc, html, body)
}

fn add_block(doc: &mut FixtureEngine, parent: NodeId, w: f32, h: f32, color: Color) -> NodeId {
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(w)));
    doc.set_style(div, RendererStyleValue::Height(Length::px(h)));
    doc.set_style(div, RendererStyleValue::BackgroundColor(color));
    doc.append_child(parent, div);
    div
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Display Tests ──────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_display_outer_block(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    add_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    Ok(doc.into_engine())
}

fn sp12_display_outer_inline(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    // Use inline-block colored boxes instead of text to avoid font-rendering diffs
    let span = doc.create_node(ElementTag::Span);
    doc.set_style(span, RendererStyleValue::Display(Display::InlineBlock));
    doc.set_style(span, RendererStyleValue::Width(Length::px(120.0)));
    doc.set_style(span, RendererStyleValue::Height(Length::px(30.0)));
    doc.set_style(
        span,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(0, 128, 0, 255)),
    );
    doc.append_child(vp, span);
    Ok(doc.into_engine())
}

fn sp12_display_outer_inline_block(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::InlineBlock));
    doc.set_style(div, RendererStyleValue::Width(Length::px(150.0)));
    doc.set_style(div, RendererStyleValue::Height(Length::px(80.0)));
    doc.set_style(div, RendererStyleValue::BackgroundColor(Color::BLUE));
    doc.append_child(vp, div);
    Ok(doc.into_engine())
}

fn sp12_display_outer_none(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::None));
    doc.set_style(div, RendererStyleValue::Width(Length::px(200.0)));
    doc.set_style(div, RendererStyleValue::Height(Length::px(100.0)));
    doc.set_style(div, RendererStyleValue::BackgroundColor(Color::RED));
    doc.append_child(vp, div);
    Ok(doc.into_engine())
}

fn sp12_display_inner_flow_root(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::FlowRoot));
    doc.set_style(div, RendererStyleValue::Width(Length::px(200.0)));
    doc.set_style(div, RendererStyleValue::Height(Length::px(100.0)));
    doc.set_style(
        div,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(0, 128, 128, 255)),
    );
    doc.append_child(vp, div);
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Position Tests ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_position_static(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    add_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    add_block(&mut doc, vp, 200.0, 100.0, Color::BLUE);
    Ok(doc.into_engine())
}

fn sp12_position_relative(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = add_block(&mut doc, vp, 100.0, 100.0, Color::BLUE);
    doc.set_style(div, RendererStyleValue::Position(Position::Relative));
    doc.set_style(div, RendererStyleValue::Top(Length::px(20.0)));
    doc.set_style(div, RendererStyleValue::Left(Length::px(30.0)));
    Ok(doc.into_engine())
}

fn sp12_position_absolute(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    doc.set_style(vp, RendererStyleValue::Position(Position::Relative));
    let div = add_block(&mut doc, vp, 100.0, 100.0, Color::RED);
    doc.set_style(div, RendererStyleValue::Position(Position::Absolute));
    doc.set_style(div, RendererStyleValue::Top(Length::px(50.0)));
    doc.set_style(div, RendererStyleValue::Left(Length::px(50.0)));
    Ok(doc.into_engine())
}

fn sp12_position_fixed(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = add_block(&mut doc, vp, 100.0, 100.0, Color::RED);
    doc.set_style(div, RendererStyleValue::Position(Position::Fixed));
    doc.set_style(div, RendererStyleValue::Top(Length::px(10.0)));
    doc.set_style(div, RendererStyleValue::Right(Length::px(10.0)));
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Float Tests ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_float_left(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let f = add_block(
        &mut doc,
        vp,
        100.0,
        100.0,
        Color::from_rgba8(0, 128, 0, 255),
    );
    doc.set_style(f, RendererStyleValue::Float(Float::Left));
    // BFC companion (overflow:hidden) next to float — displaced correctly by both engines
    let companion = doc.create_node(ElementTag::Div);
    doc.set_style(companion, RendererStyleValue::Display(Display::Block));
    doc.set_style(companion, RendererStyleValue::Height(Length::px(50.0)));
    doc.set_style(companion, RendererStyleValue::OverflowX(Overflow::Hidden));
    doc.set_style(companion, RendererStyleValue::OverflowY(Overflow::Hidden));
    doc.set_style(
        companion,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(33, 150, 243, 255)),
    );
    doc.append_child(vp, companion);
    Ok(doc.into_engine())
}

fn sp12_float_right(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let f = add_block(
        &mut doc,
        vp,
        100.0,
        100.0,
        Color::from_rgba8(0, 128, 0, 255),
    );
    doc.set_style(f, RendererStyleValue::Float(Float::Right));
    // BFC companion (overflow:hidden) next to float — displaced correctly by both engines
    let companion = doc.create_node(ElementTag::Div);
    doc.set_style(companion, RendererStyleValue::Display(Display::Block));
    doc.set_style(companion, RendererStyleValue::Height(Length::px(50.0)));
    doc.set_style(companion, RendererStyleValue::OverflowX(Overflow::Hidden));
    doc.set_style(companion, RendererStyleValue::OverflowY(Overflow::Hidden));
    doc.set_style(
        companion,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(33, 150, 243, 255)),
    );
    doc.append_child(vp, companion);
    Ok(doc.into_engine())
}

fn sp12_float_none(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let f = add_block(
        &mut doc,
        vp,
        100.0,
        100.0,
        Color::from_rgba8(0, 128, 0, 255),
    );
    doc.set_style(f, RendererStyleValue::Float(Float::None));
    Ok(doc.into_engine())
}

fn sp12_clear_left(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let f = add_block(&mut doc, vp, 100.0, 80.0, Color::RED);
    doc.set_style(f, RendererStyleValue::Float(Float::Left));
    let div = add_block(&mut doc, vp, 200.0, 50.0, Color::BLUE);
    doc.set_style(div, RendererStyleValue::Clear(Clear::Left));
    Ok(doc.into_engine())
}

fn sp12_clear_right(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let f = add_block(&mut doc, vp, 100.0, 80.0, Color::RED);
    doc.set_style(f, RendererStyleValue::Float(Float::Right));
    let div = add_block(&mut doc, vp, 200.0, 50.0, Color::BLUE);
    doc.set_style(div, RendererStyleValue::Clear(Clear::Right));
    Ok(doc.into_engine())
}

fn sp12_clear_both(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let fl = add_block(&mut doc, vp, 100.0, 80.0, Color::RED);
    doc.set_style(fl, RendererStyleValue::Float(Float::Left));
    let fr = add_block(&mut doc, vp, 100.0, 60.0, Color::from_rgba8(0, 128, 0, 255));
    doc.set_style(fr, RendererStyleValue::Float(Float::Right));
    let div = add_block(&mut doc, vp, 200.0, 50.0, Color::BLUE);
    doc.set_style(div, RendererStyleValue::Clear(Clear::Both));
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Box Model Tests ────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_margin_positive(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = add_block(&mut doc, vp, 100.0, 100.0, Color::RED);
    doc.set_style(div, RendererStyleValue::MarginTop(Length::px(20.0)));
    doc.set_style(div, RendererStyleValue::MarginRight(Length::px(20.0)));
    doc.set_style(div, RendererStyleValue::MarginBottom(Length::px(20.0)));
    doc.set_style(div, RendererStyleValue::MarginLeft(Length::px(20.0)));
    // Reference: second box to show spacing
    add_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    Ok(doc.into_engine())
}

fn sp12_margin_negative(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    add_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    let div2 = add_block(&mut doc, vp, 200.0, 100.0, Color::BLUE);
    doc.set_style(div2, RendererStyleValue::MarginTop(Length::px(-30.0)));
    Ok(doc.into_engine())
}

fn sp12_margin_auto(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = add_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    doc.set_style(div, RendererStyleValue::MarginLeft(Length::auto()));
    doc.set_style(div, RendererStyleValue::MarginRight(Length::auto()));
    Ok(doc.into_engine())
}

fn sp12_margin_collapsing_siblings(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let a = add_block(&mut doc, vp, 200.0, 50.0, Color::RED);
    doc.set_style(a, RendererStyleValue::MarginBottom(Length::px(30.0)));
    let b = add_block(&mut doc, vp, 200.0, 50.0, Color::BLUE);
    doc.set_style(b, RendererStyleValue::MarginTop(Length::px(20.0)));
    // Collapsed margin = max(30, 20) = 30px gap
    Ok(doc.into_engine())
}

fn sp12_padding_basic(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let outer = doc.create_node(ElementTag::Div);
    doc.set_style(outer, RendererStyleValue::Display(Display::Block));
    doc.set_style(
        outer,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(200, 200, 200, 255)),
    );
    doc.set_style(outer, RendererStyleValue::PaddingTop(Length::px(20.0)));
    doc.set_style(outer, RendererStyleValue::PaddingRight(Length::px(30.0)));
    doc.set_style(outer, RendererStyleValue::PaddingBottom(Length::px(20.0)));
    doc.set_style(outer, RendererStyleValue::PaddingLeft(Length::px(30.0)));
    doc.append_child(vp, outer);
    add_block(&mut doc, outer, 100.0, 60.0, Color::RED);
    Ok(doc.into_engine())
}

fn sp12_border_basic(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = add_block(
        &mut doc,
        vp,
        150.0,
        100.0,
        Color::from_rgba8(240, 240, 240, 255),
    );
    doc.set_style(div, RendererStyleValue::BorderTopWidth(3.0 as i32));
    doc.set_style(div, RendererStyleValue::BorderRightWidth(3.0 as i32));
    doc.set_style(div, RendererStyleValue::BorderBottomWidth(3.0 as i32));
    doc.set_style(div, RendererStyleValue::BorderLeftWidth(3.0 as i32));
    doc.set_style(div, RendererStyleValue::BorderTopStyle(BorderStyle::Solid));
    doc.set_style(
        div,
        RendererStyleValue::BorderRightStyle(BorderStyle::Solid),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderBottomStyle(BorderStyle::Solid),
    );
    doc.set_style(div, RendererStyleValue::BorderLeftStyle(BorderStyle::Solid));
    doc.set_style(
        div,
        RendererStyleValue::BorderTopColor(StyleColor::Resolved(Color::BLACK)),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderRightColor(StyleColor::Resolved(Color::BLACK)),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderBottomColor(StyleColor::Resolved(Color::BLACK)),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderLeftColor(StyleColor::Resolved(Color::BLACK)),
    );
    Ok(doc.into_engine())
}

fn sp12_box_sizing_content_box(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = add_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    doc.set_style(div, RendererStyleValue::BoxSizing(BoxSizing::ContentBox));
    doc.set_style(div, RendererStyleValue::PaddingTop(Length::px(10.0)));
    doc.set_style(div, RendererStyleValue::PaddingRight(Length::px(10.0)));
    doc.set_style(div, RendererStyleValue::PaddingBottom(Length::px(10.0)));
    doc.set_style(div, RendererStyleValue::PaddingLeft(Length::px(10.0)));
    doc.set_style(div, RendererStyleValue::BorderTopWidth(2.0 as i32));
    doc.set_style(div, RendererStyleValue::BorderRightWidth(2.0 as i32));
    doc.set_style(div, RendererStyleValue::BorderBottomWidth(2.0 as i32));
    doc.set_style(div, RendererStyleValue::BorderLeftWidth(2.0 as i32));
    doc.set_style(div, RendererStyleValue::BorderTopStyle(BorderStyle::Solid));
    doc.set_style(
        div,
        RendererStyleValue::BorderRightStyle(BorderStyle::Solid),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderBottomStyle(BorderStyle::Solid),
    );
    doc.set_style(div, RendererStyleValue::BorderLeftStyle(BorderStyle::Solid));
    doc.set_style(
        div,
        RendererStyleValue::BorderTopColor(StyleColor::Resolved(Color::BLACK)),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderRightColor(StyleColor::Resolved(Color::BLACK)),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderBottomColor(StyleColor::Resolved(Color::BLACK)),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderLeftColor(StyleColor::Resolved(Color::BLACK)),
    );
    // Total width: 200 + 20 + 4 = 224px (content-box)
    Ok(doc.into_engine())
}

fn sp12_box_sizing_border_box(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = add_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    doc.set_style(div, RendererStyleValue::BoxSizing(BoxSizing::BorderBox));
    doc.set_style(div, RendererStyleValue::PaddingTop(Length::px(10.0)));
    doc.set_style(div, RendererStyleValue::PaddingRight(Length::px(10.0)));
    doc.set_style(div, RendererStyleValue::PaddingBottom(Length::px(10.0)));
    doc.set_style(div, RendererStyleValue::PaddingLeft(Length::px(10.0)));
    doc.set_style(div, RendererStyleValue::BorderTopWidth(2.0 as i32));
    doc.set_style(div, RendererStyleValue::BorderRightWidth(2.0 as i32));
    doc.set_style(div, RendererStyleValue::BorderBottomWidth(2.0 as i32));
    doc.set_style(div, RendererStyleValue::BorderLeftWidth(2.0 as i32));
    doc.set_style(div, RendererStyleValue::BorderTopStyle(BorderStyle::Solid));
    doc.set_style(
        div,
        RendererStyleValue::BorderRightStyle(BorderStyle::Solid),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderBottomStyle(BorderStyle::Solid),
    );
    doc.set_style(div, RendererStyleValue::BorderLeftStyle(BorderStyle::Solid));
    doc.set_style(
        div,
        RendererStyleValue::BorderTopColor(StyleColor::Resolved(Color::BLACK)),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderRightColor(StyleColor::Resolved(Color::BLACK)),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderBottomColor(StyleColor::Resolved(Color::BLACK)),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderLeftColor(StyleColor::Resolved(Color::BLACK)),
    );
    // Total width: 200px exactly (border-box, content = 200-20-4 = 176px)
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Sizing Tests ───────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_width_fixed_px(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    add_block(&mut doc, vp, 300.0, 100.0, Color::RED);
    Ok(doc.into_engine())
}

fn sp12_height_fixed_px(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    add_block(&mut doc, vp, 200.0, 150.0, Color::BLUE);
    Ok(doc.into_engine())
}

fn sp12_width_percent(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::percent(50.0)));
    doc.set_style(div, RendererStyleValue::Height(Length::px(100.0)));
    doc.set_style(div, RendererStyleValue::BackgroundColor(Color::RED));
    doc.append_child(vp, div);
    Ok(doc.into_engine())
}

fn sp12_min_width(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(50.0)));
    doc.set_style(div, RendererStyleValue::MinWidth(Length::px(200.0)));
    doc.set_style(div, RendererStyleValue::Height(Length::px(100.0)));
    doc.set_style(div, RendererStyleValue::BackgroundColor(Color::RED));
    doc.append_child(vp, div);
    Ok(doc.into_engine())
}

fn sp12_max_width(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(500.0)));
    doc.set_style(div, RendererStyleValue::MaxWidth(Length::px(200.0)));
    doc.set_style(div, RendererStyleValue::Height(Length::px(100.0)));
    doc.set_style(div, RendererStyleValue::BackgroundColor(Color::RED));
    doc.append_child(vp, div);
    Ok(doc.into_engine())
}

fn sp12_min_height(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(200.0)));
    doc.set_style(div, RendererStyleValue::Height(Length::px(30.0)));
    doc.set_style(div, RendererStyleValue::MinHeight(Length::px(100.0)));
    doc.set_style(div, RendererStyleValue::BackgroundColor(Color::BLUE));
    doc.append_child(vp, div);
    Ok(doc.into_engine())
}

fn sp12_max_height(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(200.0)));
    doc.set_style(div, RendererStyleValue::Height(Length::px(500.0)));
    doc.set_style(div, RendererStyleValue::MaxHeight(Length::px(100.0)));
    doc.set_style(div, RendererStyleValue::BackgroundColor(Color::BLUE));
    doc.append_child(vp, div);
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Overflow Tests ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_overflow_visible(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let outer = doc.create_node(ElementTag::Div);
    doc.set_style(outer, RendererStyleValue::Display(Display::Block));
    doc.set_style(outer, RendererStyleValue::Width(Length::px(100.0)));
    doc.set_style(outer, RendererStyleValue::Height(Length::px(50.0)));
    doc.set_style(outer, RendererStyleValue::OverflowX(Overflow::Visible));
    doc.set_style(outer, RendererStyleValue::OverflowY(Overflow::Visible));
    doc.set_style(
        outer,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(200, 200, 200, 255)),
    );
    doc.append_child(vp, outer);
    add_block(&mut doc, outer, 200.0, 200.0, Color::RED);
    Ok(doc.into_engine())
}

fn sp12_overflow_hidden(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let outer = doc.create_node(ElementTag::Div);
    doc.set_style(outer, RendererStyleValue::Display(Display::Block));
    doc.set_style(outer, RendererStyleValue::Width(Length::px(100.0)));
    doc.set_style(outer, RendererStyleValue::Height(Length::px(50.0)));
    doc.set_style(outer, RendererStyleValue::OverflowX(Overflow::Hidden));
    doc.set_style(outer, RendererStyleValue::OverflowY(Overflow::Hidden));
    doc.set_style(
        outer,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(200, 200, 200, 255)),
    );
    doc.append_child(vp, outer);
    add_block(&mut doc, outer, 200.0, 200.0, Color::RED);
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Flexbox Tests ──────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_flex_direction_row(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(flex, RendererStyleValue::FlexDirection(FlexDirection::Row));
    doc.set_style(flex, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(100.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 80.0, 60.0, Color::RED);
    add_block(&mut doc, flex, 80.0, 60.0, Color::BLUE);
    add_block(
        &mut doc,
        flex,
        80.0,
        60.0,
        Color::from_rgba8(0, 128, 0, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_flex_direction_column(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(
        flex,
        RendererStyleValue::FlexDirection(FlexDirection::Column),
    );
    doc.set_style(flex, RendererStyleValue::Width(Length::px(200.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(300.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 80.0, 60.0, Color::RED);
    add_block(&mut doc, flex, 80.0, 60.0, Color::BLUE);
    add_block(
        &mut doc,
        flex,
        80.0,
        60.0,
        Color::from_rgba8(0, 128, 0, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_flex_justify_start(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(
        flex,
        RendererStyleValue::JustifyContent(ContentAlignment::new(ContentPosition::FlexStart)),
    );
    doc.set_style(flex, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(80.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 60.0, 60.0, Color::RED);
    add_block(&mut doc, flex, 60.0, 60.0, Color::BLUE);
    Ok(doc.into_engine())
}

fn sp12_flex_justify_center(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(
        flex,
        RendererStyleValue::JustifyContent(ContentAlignment::new(ContentPosition::Center)),
    );
    doc.set_style(flex, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(80.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 60.0, 60.0, Color::RED);
    add_block(&mut doc, flex, 60.0, 60.0, Color::BLUE);
    Ok(doc.into_engine())
}

fn sp12_flex_justify_space_between(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(
        flex,
        RendererStyleValue::JustifyContent(ContentAlignment::with_distribution(
            ContentDistribution::SpaceBetween,
        )),
    );
    doc.set_style(flex, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(80.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 60.0, 60.0, Color::RED);
    add_block(&mut doc, flex, 60.0, 60.0, Color::BLUE);
    add_block(
        &mut doc,
        flex,
        60.0,
        60.0,
        Color::from_rgba8(0, 128, 0, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_flex_align_center(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(
        flex,
        RendererStyleValue::AlignItems(ItemAlignment::new(ItemPosition::Center)),
    );
    doc.set_style(flex, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(150.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 80.0, 40.0, Color::RED);
    add_block(&mut doc, flex, 80.0, 80.0, Color::BLUE);
    add_block(
        &mut doc,
        flex,
        80.0,
        60.0,
        Color::from_rgba8(0, 128, 0, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_flex_align_stretch(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(
        flex,
        RendererStyleValue::AlignItems(ItemAlignment::new(ItemPosition::Stretch)),
    );
    doc.set_style(flex, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(150.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    // Items have no explicit height — should stretch to container height
    let a = doc.create_node(ElementTag::Div);
    doc.set_style(a, RendererStyleValue::Display(Display::Block));
    doc.set_style(a, RendererStyleValue::Width(Length::px(80.0)));
    doc.set_style(a, RendererStyleValue::BackgroundColor(Color::RED));
    doc.append_child(flex, a);
    let b = doc.create_node(ElementTag::Div);
    doc.set_style(b, RendererStyleValue::Display(Display::Block));
    doc.set_style(b, RendererStyleValue::Width(Length::px(80.0)));
    doc.set_style(b, RendererStyleValue::BackgroundColor(Color::BLUE));
    doc.append_child(flex, b);
    Ok(doc.into_engine())
}

fn sp12_flex_wrap_basic(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(flex, RendererStyleValue::FlexWrap(FlexWrap::Wrap));
    doc.set_style(flex, RendererStyleValue::Width(Length::px(200.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    // Four 80px items in 200px container → wraps after 2
    add_block(&mut doc, flex, 80.0, 50.0, Color::RED);
    add_block(&mut doc, flex, 80.0, 50.0, Color::BLUE);
    add_block(
        &mut doc,
        flex,
        80.0,
        50.0,
        Color::from_rgba8(0, 128, 0, 255),
    );
    add_block(
        &mut doc,
        flex,
        80.0,
        50.0,
        Color::from_rgba8(255, 165, 0, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_flex_grow_equal(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(flex, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(80.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    // Three items each with flex_grow=1 should share space equally
    for color in [Color::RED, Color::BLUE, Color::from_rgba8(0, 128, 0, 255)] {
        let item = doc.create_node(ElementTag::Div);
        doc.set_style(item, RendererStyleValue::Display(Display::Block));
        doc.set_style(item, RendererStyleValue::FlexGrow(1.0));
        doc.set_style(item, RendererStyleValue::Height(Length::px(60.0)));
        doc.set_style(item, RendererStyleValue::BackgroundColor(color));
        doc.append_child(flex, item);
    }
    Ok(doc.into_engine())
}

fn sp12_flex_gap(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(flex, RendererStyleValue::ColumnGap(Some(Length::px(20.0))));
    doc.set_style(flex, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(80.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 80.0, 60.0, Color::RED);
    add_block(&mut doc, flex, 80.0, 60.0, Color::BLUE);
    add_block(
        &mut doc,
        flex,
        80.0,
        60.0,
        Color::from_rgba8(0, 128, 0, 255),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Visual / Stacking Tests ────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_z_index_stacking(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    doc.set_style(vp, RendererStyleValue::Position(Position::Relative));
    // Lower box (z-index: 1)
    let a = add_block(&mut doc, vp, 150.0, 150.0, Color::RED);
    doc.set_style(a, RendererStyleValue::Position(Position::Absolute));
    doc.set_style(a, RendererStyleValue::Top(Length::px(20.0)));
    doc.set_style(a, RendererStyleValue::Left(Length::px(20.0)));
    doc.set_style(a, RendererStyleValue::ZIndex(Some(1)));
    // Upper box (z-index: 2) — overlaps and should render on top
    let b = add_block(&mut doc, vp, 150.0, 150.0, Color::BLUE);
    doc.set_style(b, RendererStyleValue::Position(Position::Absolute));
    doc.set_style(b, RendererStyleValue::Top(Length::px(60.0)));
    doc.set_style(b, RendererStyleValue::Left(Length::px(60.0)));
    doc.set_style(b, RendererStyleValue::ZIndex(Some(2)));
    Ok(doc.into_engine())
}

fn sp12_opacity_basic(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = add_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    doc.set_style(div, RendererStyleValue::Opacity(0.5));
    // Fully opaque reference below
    add_block(&mut doc, vp, 200.0, 100.0, Color::BLUE);
    Ok(doc.into_engine())
}

fn sp12_border_radius(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = add_block(&mut doc, vp, 200.0, 200.0, Color::RED);
    doc.set_style(div, RendererStyleValue::BorderTopLeftRadius((20.0, 20.0)));
    doc.set_style(div, RendererStyleValue::BorderTopRightRadius((20.0, 20.0)));
    doc.set_style(
        div,
        RendererStyleValue::BorderBottomRightRadius((20.0, 20.0)),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderBottomLeftRadius((20.0, 20.0)),
    );
    Ok(doc.into_engine())
}

fn sp12_visibility_hidden(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    add_block(&mut doc, vp, 200.0, 50.0, Color::RED);
    let hidden = add_block(&mut doc, vp, 200.0, 50.0, Color::BLUE);
    doc.set_style(hidden, RendererStyleValue::Visibility(Visibility::Hidden));
    // Third box should appear after the gap left by the hidden box
    add_block(&mut doc, vp, 200.0, 50.0, Color::from_rgba8(0, 128, 0, 255));
    Ok(doc.into_engine())
}

fn sp12_nested_blocks(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let outer = doc.create_node(ElementTag::Div);
    doc.set_style(outer, RendererStyleValue::Display(Display::Block));
    doc.set_style(outer, RendererStyleValue::Width(Length::px(300.0)));
    doc.set_style(outer, RendererStyleValue::PaddingTop(Length::px(10.0)));
    doc.set_style(outer, RendererStyleValue::PaddingRight(Length::px(10.0)));
    doc.set_style(outer, RendererStyleValue::PaddingBottom(Length::px(10.0)));
    doc.set_style(outer, RendererStyleValue::PaddingLeft(Length::px(10.0)));
    doc.set_style(
        outer,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(200, 200, 200, 255)),
    );
    doc.append_child(vp, outer);
    let mid = doc.create_node(ElementTag::Div);
    doc.set_style(mid, RendererStyleValue::Display(Display::Block));
    doc.set_style(mid, RendererStyleValue::PaddingTop(Length::px(10.0)));
    doc.set_style(mid, RendererStyleValue::PaddingRight(Length::px(10.0)));
    doc.set_style(mid, RendererStyleValue::PaddingBottom(Length::px(10.0)));
    doc.set_style(mid, RendererStyleValue::PaddingLeft(Length::px(10.0)));
    doc.set_style(
        mid,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(150, 150, 200, 255)),
    );
    doc.append_child(outer, mid);
    add_block(&mut doc, mid, 100.0, 60.0, Color::RED);
    add_block(&mut doc, mid, 100.0, 60.0, Color::BLUE);
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Sticky Positioning Tests ───────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_position_sticky_top(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let container = doc.create_node(ElementTag::Div);
    doc.set_style(container, RendererStyleValue::Display(Display::Block));
    doc.set_style(container, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(container, RendererStyleValue::Height(Length::px(300.0)));
    doc.set_style(
        container,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(240, 240, 240, 255)),
    );
    doc.append_child(vp, container);
    let child = add_block(
        &mut doc,
        container,
        100.0,
        30.0,
        Color::from_rgba8(76, 175, 80, 255),
    );
    doc.set_style(child, RendererStyleValue::Position(Position::Sticky));
    doc.set_style(child, RendererStyleValue::Top(Length::px(10.0)));
    Ok(doc.into_engine())
}

fn sp12_position_sticky_bottom(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let container = doc.create_node(ElementTag::Div);
    doc.set_style(container, RendererStyleValue::Display(Display::Block));
    doc.set_style(container, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(container, RendererStyleValue::Height(Length::px(300.0)));
    doc.set_style(
        container,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(240, 240, 240, 255)),
    );
    doc.append_child(vp, container);
    let child = add_block(
        &mut doc,
        container,
        100.0,
        30.0,
        Color::from_rgba8(76, 175, 80, 255),
    );
    doc.set_style(child, RendererStyleValue::Position(Position::Sticky));
    doc.set_style(child, RendererStyleValue::Bottom(Length::px(10.0)));
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Multicol Tests ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_multicol_2_columns(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let container = doc.create_node(ElementTag::Div);
    doc.set_style(container, RendererStyleValue::Display(Display::Block));
    doc.set_style(container, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(container, RendererStyleValue::ColumnCount(Some(2)));
    doc.set_style(
        container,
        RendererStyleValue::ColumnGap(Some(Length::px(20.0))),
    );
    doc.set_style(
        container,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(240, 240, 240, 255)),
    );
    doc.append_child(vp, container);
    add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(244, 67, 54, 255),
    );
    add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(76, 175, 80, 255),
    );
    add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(33, 150, 243, 255),
    );
    add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(255, 152, 0, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_multicol_column_width(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let container = doc.create_node(ElementTag::Div);
    doc.set_style(container, RendererStyleValue::Display(Display::Block));
    doc.set_style(container, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(
        container,
        RendererStyleValue::ColumnWidth(Some(Length::px(150.0))),
    );
    doc.set_style(
        container,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(240, 240, 240, 255)),
    );
    doc.append_child(vp, container);
    add_block(
        &mut doc,
        container,
        140.0,
        50.0,
        Color::from_rgba8(244, 67, 54, 255),
    );
    add_block(
        &mut doc,
        container,
        140.0,
        50.0,
        Color::from_rgba8(76, 175, 80, 255),
    );
    add_block(
        &mut doc,
        container,
        140.0,
        50.0,
        Color::from_rgba8(33, 150, 243, 255),
    );
    add_block(
        &mut doc,
        container,
        140.0,
        50.0,
        Color::from_rgba8(255, 152, 0, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_multicol_column_gap(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let container = doc.create_node(ElementTag::Div);
    doc.set_style(container, RendererStyleValue::Display(Display::Block));
    doc.set_style(container, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(container, RendererStyleValue::ColumnCount(Some(2)));
    doc.set_style(
        container,
        RendererStyleValue::ColumnGap(Some(Length::px(40.0))),
    );
    doc.set_style(
        container,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(240, 240, 240, 255)),
    );
    doc.append_child(vp, container);
    add_block(
        &mut doc,
        container,
        170.0,
        50.0,
        Color::from_rgba8(244, 67, 54, 255),
    );
    add_block(
        &mut doc,
        container,
        170.0,
        50.0,
        Color::from_rgba8(76, 175, 80, 255),
    );
    add_block(
        &mut doc,
        container,
        170.0,
        50.0,
        Color::from_rgba8(33, 150, 243, 255),
    );
    add_block(
        &mut doc,
        container,
        170.0,
        50.0,
        Color::from_rgba8(255, 152, 0, 255),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Flex Advanced Tests ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_flex_direction_row_reverse(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(
        flex,
        RendererStyleValue::FlexDirection(FlexDirection::RowReverse),
    );
    doc.set_style(flex, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(100.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    add_block(
        &mut doc,
        flex,
        80.0,
        60.0,
        Color::from_rgba8(244, 67, 54, 255),
    );
    add_block(
        &mut doc,
        flex,
        80.0,
        60.0,
        Color::from_rgba8(76, 175, 80, 255),
    );
    add_block(
        &mut doc,
        flex,
        80.0,
        60.0,
        Color::from_rgba8(33, 150, 243, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_flex_direction_column_reverse(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(
        flex,
        RendererStyleValue::FlexDirection(FlexDirection::ColumnReverse),
    );
    doc.set_style(flex, RendererStyleValue::Width(Length::px(200.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(300.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    add_block(
        &mut doc,
        flex,
        80.0,
        60.0,
        Color::from_rgba8(244, 67, 54, 255),
    );
    add_block(
        &mut doc,
        flex,
        80.0,
        60.0,
        Color::from_rgba8(76, 175, 80, 255),
    );
    add_block(
        &mut doc,
        flex,
        80.0,
        60.0,
        Color::from_rgba8(33, 150, 243, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_flex_wrap_reverse(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(flex, RendererStyleValue::FlexWrap(FlexWrap::WrapReverse));
    doc.set_style(flex, RendererStyleValue::Width(Length::px(200.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    add_block(
        &mut doc,
        flex,
        80.0,
        60.0,
        Color::from_rgba8(244, 67, 54, 255),
    );
    add_block(
        &mut doc,
        flex,
        80.0,
        60.0,
        Color::from_rgba8(76, 175, 80, 255),
    );
    add_block(
        &mut doc,
        flex,
        80.0,
        60.0,
        Color::from_rgba8(33, 150, 243, 255),
    );
    add_block(
        &mut doc,
        flex,
        80.0,
        60.0,
        Color::from_rgba8(255, 152, 0, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_flex_justify_space_around(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(
        flex,
        RendererStyleValue::JustifyContent(ContentAlignment::with_distribution(
            ContentDistribution::SpaceAround,
        )),
    );
    doc.set_style(flex, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(80.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    add_block(
        &mut doc,
        flex,
        60.0,
        40.0,
        Color::from_rgba8(244, 67, 54, 255),
    );
    add_block(
        &mut doc,
        flex,
        60.0,
        40.0,
        Color::from_rgba8(76, 175, 80, 255),
    );
    add_block(
        &mut doc,
        flex,
        60.0,
        40.0,
        Color::from_rgba8(33, 150, 243, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_flex_justify_space_evenly(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(
        flex,
        RendererStyleValue::JustifyContent(ContentAlignment::with_distribution(
            ContentDistribution::SpaceEvenly,
        )),
    );
    doc.set_style(flex, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(80.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    add_block(
        &mut doc,
        flex,
        60.0,
        40.0,
        Color::from_rgba8(244, 67, 54, 255),
    );
    add_block(
        &mut doc,
        flex,
        60.0,
        40.0,
        Color::from_rgba8(76, 175, 80, 255),
    );
    add_block(
        &mut doc,
        flex,
        60.0,
        40.0,
        Color::from_rgba8(33, 150, 243, 255),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Margin Collapsing Advanced Tests ───────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_margin_collapsing_parent_child(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    // Parent with no border/padding — child's margin-top should collapse with parent
    let parent = doc.create_node(ElementTag::Div);
    doc.set_style(parent, RendererStyleValue::Display(Display::Block));
    doc.set_style(parent, RendererStyleValue::Width(Length::px(200.0)));
    doc.set_style(
        parent,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(200, 200, 200, 255)),
    );
    doc.append_child(vp, parent);
    let child = add_block(
        &mut doc,
        parent,
        100.0,
        50.0,
        Color::from_rgba8(244, 67, 54, 255),
    );
    doc.set_style(child, RendererStyleValue::MarginTop(Length::px(30.0)));
    // Reference block below to visualize the gap
    add_block(
        &mut doc,
        vp,
        200.0,
        50.0,
        Color::from_rgba8(33, 150, 243, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_margin_collapsing_through_empty(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    // First sibling with margin-bottom
    let a = add_block(
        &mut doc,
        vp,
        200.0,
        50.0,
        Color::from_rgba8(244, 67, 54, 255),
    );
    doc.set_style(a, RendererStyleValue::MarginBottom(Length::px(20.0)));
    // Empty block between — its own margins collapse through
    let empty = doc.create_node(ElementTag::Div);
    doc.set_style(empty, RendererStyleValue::Display(Display::Block));
    doc.set_style(empty, RendererStyleValue::MarginTop(Length::px(15.0)));
    doc.set_style(empty, RendererStyleValue::MarginBottom(Length::px(25.0)));
    doc.append_child(vp, empty);
    // Second sibling with margin-top
    let b = add_block(
        &mut doc,
        vp,
        200.0,
        50.0,
        Color::from_rgba8(33, 150, 243, 255),
    );
    doc.set_style(b, RendererStyleValue::MarginTop(Length::px(10.0)));
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Overflow Axes Tests ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_overflow_scroll(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let outer = doc.create_node(ElementTag::Div);
    doc.set_style(outer, RendererStyleValue::Display(Display::Block));
    doc.set_style(outer, RendererStyleValue::Width(Length::px(200.0)));
    doc.set_style(outer, RendererStyleValue::Height(Length::px(100.0)));
    doc.set_style(outer, RendererStyleValue::OverflowX(Overflow::Scroll));
    doc.set_style(outer, RendererStyleValue::OverflowY(Overflow::Scroll));
    doc.set_style(
        outer,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(240, 240, 240, 255)),
    );
    doc.append_child(vp, outer);
    add_block(
        &mut doc,
        outer,
        180.0,
        200.0,
        Color::from_rgba8(244, 67, 54, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_overflow_auto(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let outer = doc.create_node(ElementTag::Div);
    doc.set_style(outer, RendererStyleValue::Display(Display::Block));
    doc.set_style(outer, RendererStyleValue::Width(Length::px(200.0)));
    doc.set_style(outer, RendererStyleValue::Height(Length::px(100.0)));
    doc.set_style(outer, RendererStyleValue::OverflowX(Overflow::Auto));
    doc.set_style(outer, RendererStyleValue::OverflowY(Overflow::Auto));
    doc.set_style(
        outer,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(240, 240, 240, 255)),
    );
    doc.append_child(vp, outer);
    add_block(
        &mut doc,
        outer,
        180.0,
        200.0,
        Color::from_rgba8(244, 67, 54, 255),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Aspect Ratio Tests ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_aspect_ratio_basic(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(200.0)));
    doc.set_style(
        div,
        RendererStyleValue::AspectRatio(Some(AspectRatio {
            ratio: (2.0, 1.0),
            auto_flag: false,
        })),
    );
    doc.set_style(
        div,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(156, 39, 176, 255)),
    );
    doc.append_child(vp, div);
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Decoration Tests ──────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// Helper: create a text block with given text and return its node id.
fn add_text_block(doc: &mut FixtureEngine, parent: NodeId, text: &str) -> NodeId {
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::MarginBottom(Length::px(10.0)));
    doc.set_node_state(div, RendererNodeState::Text(Some(text.to_string())));
    doc.append_child(parent, div);
    div
}

fn sp11_text_decoration_underline(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "This text has an underline decoration");
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationLine(TextDecorationLine::UNDERLINE),
    );
    Ok(doc.into_engine())
}

fn sp11_text_decoration_overline(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "This text has an overline decoration");
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationLine(TextDecorationLine::OVERLINE),
    );
    Ok(doc.into_engine())
}

fn sp11_text_decoration_line_through(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "This text has a line-through decoration");
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationLine(TextDecorationLine::LINE_THROUGH),
    );
    Ok(doc.into_engine())
}

fn sp11_text_decoration_combined(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(
        &mut doc,
        vp,
        "This text has underline + overline + line-through",
    );
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationLine(TextDecorationLine(
            TextDecorationLine::UNDERLINE.0
                | TextDecorationLine::OVERLINE.0
                | TextDecorationLine::LINE_THROUGH.0,
        )),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Font Weight & Style Tests ──────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_font_weight_normal(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Normal weight (400) text sample");
    doc.set_style(t, RendererStyleValue::FontWeight(FontWeight::NORMAL));
    Ok(doc.into_engine())
}

fn sp11_font_weight_bold(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Bold weight (700) text sample");
    doc.set_style(t, RendererStyleValue::FontWeight(FontWeight::BOLD));
    Ok(doc.into_engine())
}

fn sp11_font_style_normal(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Normal style text sample");
    doc.set_style(t, RendererStyleValue::FontStyle(FontStyleEnum::Normal));
    Ok(doc.into_engine())
}

fn sp11_font_style_italic(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Italic style text sample");
    doc.set_style(t, RendererStyleValue::FontStyle(FontStyleEnum::Italic));
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Font Size Tests ────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_font_size_small(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Small text at 12px font size");
    doc.set_style(t, RendererStyleValue::FontSize(12.0));
    Ok(doc.into_engine())
}

fn sp11_font_size_medium(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Medium text at 16px font size (default)");
    doc.set_style(t, RendererStyleValue::FontSize(16.0));
    Ok(doc.into_engine())
}

fn sp11_font_size_large(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Large text at 24px font size");
    doc.set_style(t, RendererStyleValue::FontSize(24.0));
    Ok(doc.into_engine())
}

fn sp11_font_size_xlarge(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Extra large text at 32px");
    doc.set_style(t, RendererStyleValue::FontSize(32.0));
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Align Tests ───────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_text_align_left(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Left-aligned text in a block");
    doc.set_style(t, RendererStyleValue::TextAlign(TextAlign::Left));
    doc.set_style(t, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(
        t,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    Ok(doc.into_engine())
}

fn sp11_text_align_center(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Center-aligned text in a block");
    doc.set_style(t, RendererStyleValue::TextAlign(TextAlign::Center));
    doc.set_style(t, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(
        t,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    Ok(doc.into_engine())
}

fn sp11_text_align_right(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Right-aligned text in a block");
    doc.set_style(t, RendererStyleValue::TextAlign(TextAlign::Right));
    doc.set_style(t, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(
        t,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    Ok(doc.into_engine())
}

fn sp11_text_align_justify(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(
        &mut doc,
        vp,
        "Justified text stretches words across the full width of the container block so that both edges are flush.",
    );
    doc.set_style(t, RendererStyleValue::TextAlign(TextAlign::Justify));
    doc.set_style(t, RendererStyleValue::Width(Length::px(300.0)));
    doc.set_style(
        t,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Transform Tests ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_text_transform_uppercase(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "this text should be uppercase");
    doc.set_style(
        t,
        RendererStyleValue::TextTransform(TextTransform::Uppercase),
    );
    Ok(doc.into_engine())
}

fn sp11_text_transform_lowercase(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "THIS TEXT SHOULD BE LOWERCASE");
    doc.set_style(
        t,
        RendererStyleValue::TextTransform(TextTransform::Lowercase),
    );
    Ok(doc.into_engine())
}

fn sp11_text_transform_capitalize(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "capitalize each word in this sentence");
    doc.set_style(
        t,
        RendererStyleValue::TextTransform(TextTransform::Capitalize),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Indent Tests ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_text_indent_positive(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(
        &mut doc,
        vp,
        "This paragraph has a positive 40px text-indent on the first line. The second line wraps normally without indent.",
    );
    doc.set_style(t, RendererStyleValue::TextIndent(Length::px(40.0)));
    doc.set_style(t, RendererStyleValue::Width(Length::px(300.0)));
    doc.set_style(
        t,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    Ok(doc.into_engine())
}

fn sp11_text_indent_negative(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(
        &mut doc,
        vp,
        "This paragraph has a negative -20px text-indent (hanging indent) on the first line.",
    );
    doc.set_style(t, RendererStyleValue::TextIndent(Length::px(-20.0)));
    doc.set_style(t, RendererStyleValue::Width(Length::px(300.0)));
    doc.set_style(t, RendererStyleValue::PaddingLeft(Length::px(30.0)));
    doc.set_style(
        t,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Letter & Word Spacing Tests ────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_letter_spacing_positive(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Wide letter spacing");
    doc.set_style(t, RendererStyleValue::LetterSpacing(5.0));
    add_text_block(&mut doc, vp, "Normal letter spacing for comparison");
    Ok(doc.into_engine())
}

fn sp11_letter_spacing_negative(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Tight letter spacing");
    doc.set_style(t, RendererStyleValue::LetterSpacing(-1.0));
    add_text_block(&mut doc, vp, "Normal letter spacing for comparison");
    Ok(doc.into_engine())
}

fn sp11_word_spacing_positive(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Extra space between words in this sentence");
    doc.set_style(t, RendererStyleValue::WordSpacing(15.0));
    add_text_block(&mut doc, vp, "Normal word spacing for comparison");
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Line Height Tests ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_line_height_normal(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(
        &mut doc,
        vp,
        "Line height normal. This is a multi-line paragraph to demonstrate the default line spacing between lines of text.",
    );
    doc.set_style(t, RendererStyleValue::LineHeight(LineHeight::Normal));
    doc.set_style(t, RendererStyleValue::Width(Length::px(300.0)));
    doc.set_style(
        t,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    Ok(doc.into_engine())
}

fn sp11_line_height_number(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(
        &mut doc,
        vp,
        "Line height 2.0. This is a multi-line paragraph to demonstrate double line spacing between lines of text.",
    );
    doc.set_style(t, RendererStyleValue::LineHeight(LineHeight::Number(2.0)));
    doc.set_style(t, RendererStyleValue::Width(Length::px(300.0)));
    doc.set_style(
        t,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    Ok(doc.into_engine())
}

fn sp11_line_height_length(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(
        &mut doc,
        vp,
        "Line height 30px. This is a multi-line paragraph to demonstrate fixed 30px line spacing between lines.",
    );
    doc.set_style(t, RendererStyleValue::LineHeight(LineHeight::Length(30.0)));
    doc.set_style(t, RendererStyleValue::Width(Length::px(300.0)));
    doc.set_style(
        t,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 White Space Tests ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_white_space_normal(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(
        &mut doc,
        vp,
        "White space   normal:   multiple    spaces   and\nnewlines   collapse   into   single   spaces.",
    );
    doc.set_style(t, RendererStyleValue::WhiteSpace(WhiteSpace::Normal));
    doc.set_style(t, RendererStyleValue::Width(Length::px(300.0)));
    doc.set_style(
        t,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    Ok(doc.into_engine())
}

fn sp11_white_space_nowrap(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(
        &mut doc,
        vp,
        "White space nowrap: this long text should not wrap to the next line even if it overflows the container.",
    );
    doc.set_style(t, RendererStyleValue::WhiteSpace(WhiteSpace::Nowrap));
    doc.set_style(t, RendererStyleValue::Width(Length::px(200.0)));
    doc.set_style(
        t,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    Ok(doc.into_engine())
}

fn sp11_white_space_pre(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(
        &mut doc,
        vp,
        "White space pre:\n  indented line\n  preserves   spaces\n    and newlines",
    );
    doc.set_style(t, RendererStyleValue::WhiteSpace(WhiteSpace::Pre));
    doc.set_style(
        t,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    Ok(doc.into_engine())
}

fn sp11_white_space_pre_wrap(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(
        &mut doc,
        vp,
        "White space pre-wrap:\n  preserves   spaces\n  but also   wraps   long lines when they exceed the container width limit.",
    );
    doc.set_style(t, RendererStyleValue::WhiteSpace(WhiteSpace::PreWrap));
    doc.set_style(t, RendererStyleValue::Width(Length::px(300.0)));
    doc.set_style(
        t,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    Ok(doc.into_engine())
}

fn sp11_white_space_pre_line(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(
        &mut doc,
        vp,
        "White space pre-line:\n  collapses   spaces\n  but   preserves\n  newlines and wraps.",
    );
    doc.set_style(t, RendererStyleValue::WhiteSpace(WhiteSpace::PreLine));
    doc.set_style(t, RendererStyleValue::Width(Length::px(300.0)));
    doc.set_style(
        t,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Color Tests ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_color_red(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "This text is rendered in red color");
    doc.set_style(t, RendererStyleValue::Color(Color::RED));
    Ok(doc.into_engine())
}

fn sp11_color_blue(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "This text is rendered in blue color");
    doc.set_style(t, RendererStyleValue::Color(Color::BLUE));
    Ok(doc.into_engine())
}

fn sp11_color_green(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "This text is rendered in green color");
    doc.set_style(t, RendererStyleValue::Color(Color::GREEN));
    Ok(doc.into_engine())
}

fn sp11_color_custom(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(
        &mut doc,
        vp,
        "This text is rendered in custom purple (#8B008B)",
    );
    doc.set_style(
        t,
        RendererStyleValue::Color(Color::from_rgba8(139, 0, 139, 255)),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Shadow & Overflow Tests ───────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_text_shadow_basic(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Text with a shadow effect");
    doc.set_style(t, RendererStyleValue::FontSize(24.0));
    doc.set_style(
        t,
        RendererStyleValue::TextShadow(vec![TextShadow {
            offset_x: 2.0,
            offset_y: 2.0,
            blur_radius: 4.0,
            color: Color::from_rgba8(0, 0, 0, 128),
        }]),
    );
    Ok(doc.into_engine())
}

fn sp11_text_overflow_ellipsis(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(
        &mut doc,
        vp,
        "This text overflows its container and should show an ellipsis at the end",
    );
    doc.set_style(t, RendererStyleValue::Width(Length::px(200.0)));
    doc.set_style(t, RendererStyleValue::WhiteSpace(WhiteSpace::Nowrap));
    doc.set_style(t, RendererStyleValue::OverflowX(Overflow::Hidden));
    doc.set_style(t, RendererStyleValue::TextOverflow(TextOverflow::Ellipsis));
    doc.set_style(
        t,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Decoration Style Tests ────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_text_decoration_style_solid(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Hello World");
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationLine(TextDecorationLine::UNDERLINE),
    );
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationStyle(TextDecorationStyle::Solid),
    );
    Ok(doc.into_engine())
}

fn sp11_text_decoration_style_double(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Hello World");
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationLine(TextDecorationLine::UNDERLINE),
    );
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationStyle(TextDecorationStyle::Double),
    );
    Ok(doc.into_engine())
}

fn sp11_text_decoration_style_dotted(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Hello World");
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationLine(TextDecorationLine::UNDERLINE),
    );
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationStyle(TextDecorationStyle::Dotted),
    );
    Ok(doc.into_engine())
}

fn sp11_text_decoration_style_dashed(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Hello World");
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationLine(TextDecorationLine::UNDERLINE),
    );
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationStyle(TextDecorationStyle::Dashed),
    );
    Ok(doc.into_engine())
}

fn sp11_text_decoration_style_wavy(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Hello World");
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationLine(TextDecorationLine::UNDERLINE),
    );
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationStyle(TextDecorationStyle::Wavy),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Decoration Skip-Ink Tests ─────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_text_decoration_skip_ink_auto(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Typography");
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationLine(TextDecorationLine::UNDERLINE),
    );
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationSkipInk(TextDecorationSkipInk::Auto),
    );
    Ok(doc.into_engine())
}

fn sp11_text_decoration_skip_ink_none(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Typography");
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationLine(TextDecorationLine::UNDERLINE),
    );
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationSkipInk(TextDecorationSkipInk::None),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Decoration Metrics Tests ──────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_text_decoration_color_red(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Hello World");
    doc.set_style(t, RendererStyleValue::Color(Color::BLUE));
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationLine(TextDecorationLine::UNDERLINE),
    );
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationColor(StyleColor::Resolved(Color::RED)),
    );
    Ok(doc.into_engine())
}

fn sp11_text_decoration_thickness_3px(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Hello World");
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationLine(TextDecorationLine::UNDERLINE),
    );
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationThickness(TextDecorationThickness::Length(3.0)),
    );
    Ok(doc.into_engine())
}

fn sp11_text_underline_offset_5px(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Hello World");
    doc.set_style(
        t,
        RendererStyleValue::TextDecorationLine(TextDecorationLine::UNDERLINE),
    );
    doc.set_style(t, RendererStyleValue::TextUnderlineOffset(Length::px(5.0)));
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Font Family Tests ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_font_family_serif(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    doc.set_style(
        vp,
        RendererStyleValue::FontFamily(FontFamilyList::generic(GenericFontFamily::Serif)),
    );
    add_text_block(&mut doc, vp, "Hello World");
    Ok(doc.into_engine())
}

fn sp11_font_family_monospace(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    doc.set_style(
        vp,
        RendererStyleValue::FontFamily(FontFamilyList::generic(GenericFontFamily::Monospace)),
    );
    add_text_block(&mut doc, vp, "Hello World");
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Word Spacing Negative Test ─────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_word_spacing_negative(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "The quick brown fox");
    doc.set_style(t, RendererStyleValue::WordSpacing(-3.0));
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Line Height Percentage Test ────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_line_height_percentage(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(
        &mut doc,
        vp,
        "Line height 200%. This is a multi-line paragraph to demonstrate percentage-based line spacing between lines of text.",
    );
    doc.set_style(
        t,
        RendererStyleValue::LineHeight(LineHeight::Percentage(200.0)),
    );
    doc.set_style(t, RendererStyleValue::Width(Length::px(300.0)));
    doc.set_style(
        t,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Shadow Offset Test ────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_text_shadow_offset(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let t = add_text_block(&mut doc, vp, "Shadow");
    doc.set_style(
        t,
        RendererStyleValue::TextShadow(vec![TextShadow {
            offset_x: 3.0,
            offset_y: 3.0,
            blur_radius: 0.0,
            color: Color::RED,
        }]),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Inline Basic Tests ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_inline_single_span(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let span = doc.create_node(ElementTag::Span);
    doc.set_style(span, RendererStyleValue::Display(Display::Inline));
    doc.set_style(
        span,
        RendererStyleValue::Color(Color::from_rgba8(0, 0, 0, 255)),
    );
    doc.set_style(
        span,
        RendererStyleValue::FontFamily(FontFamilyList::single("Ahem")),
    );
    doc.set_style(span, RendererStyleValue::FontSize(20.0));
    let text = doc.create_node(ElementTag::Text);
    doc.set_style(
        text,
        RendererStyleValue::Color(Color::from_rgba8(0, 0, 0, 255)),
    );
    doc.set_style(
        text,
        RendererStyleValue::FontFamily(FontFamilyList::single("Ahem")),
    );
    doc.set_style(text, RendererStyleValue::FontSize(20.0));
    doc.set_node_state(text, RendererNodeState::Text(Some("Xpqg".to_string())));
    doc.append_child(span, text);
    doc.append_child(vp, span);
    Ok(doc.into_engine())
}

fn sp13_inline_multiple_spans(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let colors = [Color::RED, Color::BLUE, Color::from_rgba8(0, 128, 0, 255)];
    let texts = ["First span ", "Second span ", "Third span"];
    for (text, color) in texts.iter().zip(colors.iter()) {
        let span = doc.create_node(ElementTag::Span);
        doc.set_style(span, RendererStyleValue::Display(Display::Inline));
        doc.set_style(span, RendererStyleValue::Color(*color));
        doc.set_node_state(span, RendererNodeState::Text(Some(text.to_string())));
        doc.append_child(vp, span);
    }
    Ok(doc.into_engine())
}

fn sp13_inline_nested_spans(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let outer = doc.create_node(ElementTag::Span);
    doc.set_style(outer, RendererStyleValue::Display(Display::Inline));
    doc.set_style(outer, RendererStyleValue::Color(Color::BLUE));
    doc.set_node_state(outer, RendererNodeState::Text(Some("Outer ".to_string())));
    doc.append_child(vp, outer);

    let inner = doc.create_node(ElementTag::Span);
    doc.set_style(inner, RendererStyleValue::Display(Display::Inline));
    doc.set_style(inner, RendererStyleValue::Color(Color::RED));
    doc.set_style(inner, RendererStyleValue::FontWeight(FontWeight::BOLD));
    doc.set_node_state(
        inner,
        RendererNodeState::Text(Some("inner bold red".to_string())),
    );
    doc.append_child(outer, inner);

    let after = doc.create_node(ElementTag::Span);
    doc.set_style(after, RendererStyleValue::Display(Display::Inline));
    doc.set_style(after, RendererStyleValue::Color(Color::BLUE));
    doc.set_node_state(
        after,
        RendererNodeState::Text(Some(" outer again".to_string())),
    );
    doc.append_child(vp, after);
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Line Breaking Tests ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_line_breaking_normal_wrap(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(200.0)));
    doc.set_style(
        div,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    doc.append_child(vp, div);

    let span = doc.create_node(ElementTag::Span);
    doc.set_style(span, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(span, RendererNodeState::Text(Some("This is a long line of text that should naturally wrap at word boundaries within the container".to_string())));
    doc.append_child(div, span);
    Ok(doc.into_engine())
}

fn sp13_line_breaking_nowrap(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(200.0)));
    doc.set_style(div, RendererStyleValue::WhiteSpace(WhiteSpace::Nowrap));
    doc.set_style(div, RendererStyleValue::OverflowX(Overflow::Hidden));
    doc.set_style(
        div,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    doc.append_child(vp, div);

    let span = doc.create_node(ElementTag::Span);
    doc.set_style(span, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(
        span,
        RendererNodeState::Text(Some(
            "This text should not wrap and may be clipped by overflow hidden".to_string(),
        )),
    );
    doc.append_child(div, span);
    Ok(doc.into_engine())
}

fn sp13_line_breaking_break_word(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(150.0)));
    doc.set_style(
        div,
        RendererStyleValue::OverflowWrap(OverflowWrap::BreakWord),
    );
    doc.set_style(
        div,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    doc.append_child(vp, div);

    let span = doc.create_node(ElementTag::Span);
    doc.set_style(span, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(
        span,
        RendererNodeState::Text(Some(
            "Supercalifragilisticexpialidocious should break mid-word".to_string(),
        )),
    );
    doc.append_child(div, span);
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Vertical Align Tests ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// Helper: build a vertical-align test with a large span and an aligned smaller span.
fn vertical_align_test(viewport: ViewportMetrics, va: VerticalAlign, label: &str) -> FixtureEngine {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(
        div,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    doc.set_style(
        div,
        RendererStyleValue::LineHeight(LineHeight::Length(60.0)),
    );
    doc.append_child(vp, div);

    let big = doc.create_node(ElementTag::Span);
    doc.set_style(big, RendererStyleValue::Display(Display::Inline));
    doc.set_style(big, RendererStyleValue::FontSize(32.0));
    doc.set_node_state(big, RendererNodeState::Text(Some("Big ".to_string())));
    doc.append_child(div, big);

    let small = doc.create_node(ElementTag::Span);
    doc.set_style(small, RendererStyleValue::Display(Display::Inline));
    doc.set_style(small, RendererStyleValue::FontSize(12.0));
    doc.set_style(small, RendererStyleValue::VerticalAlign(va));
    doc.set_style(
        small,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(255, 200, 200, 255)),
    );
    doc.set_node_state(small, RendererNodeState::Text(Some(label.to_string())));
    doc.append_child(div, small);
    doc
}

fn sp13_vertical_align_baseline(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    Ok(vertical_align_test(viewport, VerticalAlign::Baseline, "baseline").into_engine())
}

fn sp13_vertical_align_middle(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    Ok(vertical_align_test(viewport, VerticalAlign::Middle, "middle").into_engine())
}

fn sp13_vertical_align_top(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    Ok(vertical_align_test(viewport, VerticalAlign::Top, "top").into_engine())
}

fn sp13_vertical_align_bottom(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    Ok(vertical_align_test(viewport, VerticalAlign::Bottom, "bottom").into_engine())
}

fn sp13_vertical_align_super(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    Ok(vertical_align_test(viewport, VerticalAlign::Super, "super").into_engine())
}

fn sp13_vertical_align_sub(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    Ok(vertical_align_test(viewport, VerticalAlign::Sub, "sub").into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Inline Block Tests ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_inline_block_basic(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let span = doc.create_node(ElementTag::Span);
    doc.set_style(span, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(
        span,
        RendererNodeState::Text(Some("Text before ".to_string())),
    );
    doc.append_child(vp, span);

    let ib = doc.create_node(ElementTag::Div);
    doc.set_style(ib, RendererStyleValue::Display(Display::InlineBlock));
    doc.set_style(ib, RendererStyleValue::Width(Length::px(80.0)));
    doc.set_style(ib, RendererStyleValue::Height(Length::px(40.0)));
    doc.set_style(ib, RendererStyleValue::BackgroundColor(Color::RED));
    doc.append_child(vp, ib);

    let after = doc.create_node(ElementTag::Span);
    doc.set_style(after, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(
        after,
        RendererNodeState::Text(Some(" text after".to_string())),
    );
    doc.append_child(vp, after);
    Ok(doc.into_engine())
}

fn sp13_inline_block_vertical_align(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(
        div,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    doc.append_child(vp, div);

    let label = doc.create_node(ElementTag::Span);
    doc.set_style(label, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(
        label,
        RendererNodeState::Text(Some("Aligned: ".to_string())),
    );
    doc.append_child(div, label);

    let ib = doc.create_node(ElementTag::Div);
    doc.set_style(ib, RendererStyleValue::Display(Display::InlineBlock));
    doc.set_style(ib, RendererStyleValue::Width(Length::px(60.0)));
    doc.set_style(ib, RendererStyleValue::Height(Length::px(60.0)));
    doc.set_style(ib, RendererStyleValue::BackgroundColor(Color::BLUE));
    doc.set_style(ib, RendererStyleValue::VerticalAlign(VerticalAlign::Middle));
    doc.append_child(div, ib);

    let trail = doc.create_node(ElementTag::Span);
    doc.set_style(trail, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(
        trail,
        RendererNodeState::Text(Some(" middle-aligned inline-block".to_string())),
    );
    doc.append_child(div, trail);
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Mixed Content Tests ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_mixed_block_inline(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    // Block element
    let blk = add_block(
        &mut doc,
        vp,
        300.0,
        40.0,
        Color::from_rgba8(200, 220, 255, 255),
    );
    let blk_txt = doc.create_node(ElementTag::Span);
    doc.set_style(blk_txt, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(
        blk_txt,
        RendererNodeState::Text(Some("Block element with text".to_string())),
    );
    doc.append_child(blk, blk_txt);

    // Inline span following block
    let span = doc.create_node(ElementTag::Span);
    doc.set_style(span, RendererStyleValue::Display(Display::Inline));
    doc.set_style(span, RendererStyleValue::Color(Color::RED));
    doc.set_node_state(
        span,
        RendererNodeState::Text(Some("Inline span after block ".to_string())),
    );
    doc.append_child(vp, span);

    // Another block
    add_block(
        &mut doc,
        vp,
        300.0,
        40.0,
        Color::from_rgba8(220, 255, 200, 255),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 White Space Handling Tests ─────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_white_space_collapsing(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(300.0)));
    doc.set_style(
        div,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    doc.append_child(vp, div);

    // Multiple inline spans with extra whitespace — should collapse
    for text in ["  Multiple  ", "  spaces  ", "  should  ", "  collapse  "] {
        let span = doc.create_node(ElementTag::Span);
        doc.set_style(span, RendererStyleValue::Display(Display::Inline));
        doc.set_node_state(span, RendererNodeState::Text(Some(text.to_string())));
        doc.append_child(div, span);
    }
    Ok(doc.into_engine())
}

fn sp13_white_space_preserving(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(div, RendererStyleValue::WhiteSpace(WhiteSpace::Pre));
    doc.set_style(
        div,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    doc.append_child(vp, div);

    let span = doc.create_node(ElementTag::Span);
    doc.set_style(span, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(
        span,
        RendererNodeState::Text(Some("  Preserved   spaces   and\n  newlines  ".to_string())),
    );
    doc.append_child(div, span);
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Inline Decoration Tests ────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_inline_background_color(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let before = doc.create_node(ElementTag::Span);
    doc.set_style(before, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(
        before,
        RendererNodeState::Text(Some("Normal text ".to_string())),
    );
    doc.append_child(vp, before);

    let highlight = doc.create_node(ElementTag::Span);
    doc.set_style(highlight, RendererStyleValue::Display(Display::Inline));
    doc.set_style(
        highlight,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(255, 255, 0, 255)),
    );
    doc.set_node_state(
        highlight,
        RendererNodeState::Text(Some("highlighted span".to_string())),
    );
    doc.append_child(vp, highlight);

    let after = doc.create_node(ElementTag::Span);
    doc.set_style(after, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(
        after,
        RendererNodeState::Text(Some(" normal text".to_string())),
    );
    doc.append_child(vp, after);
    Ok(doc.into_engine())
}

fn sp13_inline_padding(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let before = doc.create_node(ElementTag::Span);
    doc.set_style(before, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(before, RendererNodeState::Text(Some("Before ".to_string())));
    doc.append_child(vp, before);

    let padded = doc.create_node(ElementTag::Span);
    doc.set_style(padded, RendererStyleValue::Display(Display::Inline));
    doc.set_style(padded, RendererStyleValue::PaddingTop(Length::px(4.0)));
    doc.set_style(padded, RendererStyleValue::PaddingRight(Length::px(12.0)));
    doc.set_style(padded, RendererStyleValue::PaddingBottom(Length::px(4.0)));
    doc.set_style(padded, RendererStyleValue::PaddingLeft(Length::px(12.0)));
    doc.set_style(
        padded,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(200, 230, 255, 255)),
    );
    doc.set_node_state(
        padded,
        RendererNodeState::Text(Some("padded inline".to_string())),
    );
    doc.append_child(vp, padded);

    let after = doc.create_node(ElementTag::Span);
    doc.set_style(after, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(after, RendererNodeState::Text(Some(" after".to_string())));
    doc.append_child(vp, after);
    Ok(doc.into_engine())
}

fn sp13_inline_border(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let before = doc.create_node(ElementTag::Span);
    doc.set_style(before, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(before, RendererNodeState::Text(Some("Before ".to_string())));
    doc.append_child(vp, before);

    let bordered = doc.create_node(ElementTag::Span);
    doc.set_style(bordered, RendererStyleValue::Display(Display::Inline));
    doc.set_style(bordered, RendererStyleValue::BorderTopWidth(2));
    doc.set_style(bordered, RendererStyleValue::BorderRightWidth(2));
    doc.set_style(bordered, RendererStyleValue::BorderBottomWidth(2));
    doc.set_style(bordered, RendererStyleValue::BorderLeftWidth(2));
    doc.set_style(
        bordered,
        RendererStyleValue::BorderTopStyle(BorderStyle::Solid),
    );
    doc.set_style(
        bordered,
        RendererStyleValue::BorderRightStyle(BorderStyle::Solid),
    );
    doc.set_style(
        bordered,
        RendererStyleValue::BorderBottomStyle(BorderStyle::Solid),
    );
    doc.set_style(
        bordered,
        RendererStyleValue::BorderLeftStyle(BorderStyle::Solid),
    );
    doc.set_style(
        bordered,
        RendererStyleValue::BorderTopColor(StyleColor::Resolved(Color::RED)),
    );
    doc.set_style(
        bordered,
        RendererStyleValue::BorderRightColor(StyleColor::Resolved(Color::RED)),
    );
    doc.set_style(
        bordered,
        RendererStyleValue::BorderBottomColor(StyleColor::Resolved(Color::RED)),
    );
    doc.set_style(
        bordered,
        RendererStyleValue::BorderLeftColor(StyleColor::Resolved(Color::RED)),
    );
    doc.set_style(bordered, RendererStyleValue::PaddingLeft(Length::px(6.0)));
    doc.set_style(bordered, RendererStyleValue::PaddingRight(Length::px(6.0)));
    doc.set_node_state(
        bordered,
        RendererNodeState::Text(Some("bordered inline".to_string())),
    );
    doc.append_child(vp, bordered);

    let after = doc.create_node(ElementTag::Span);
    doc.set_style(after, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(after, RendererNodeState::Text(Some(" after".to_string())));
    doc.append_child(vp, after);
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 First-Letter / First-Line Tests ────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// First-letter pseudo-element effect: the opening letter is rendered at 2em
/// in red (#F44336) while the remainder uses the default style.  The Document
/// builder has no direct `::first-letter` API, so we model the visual result
/// with an explicit large-letter span.
fn sp13_first_letter_basic(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.append_child(vp, div);

    let letter = doc.create_node(ElementTag::Span);
    doc.set_style(letter, RendererStyleValue::Display(Display::Inline));
    doc.set_style(letter, RendererStyleValue::FontSize(32.0)); // 2em relative to 16px base
    doc.set_style(
        letter,
        RendererStyleValue::Color(Color::from_rgba8(244, 67, 54, 255)),
    ); // #F44336
    doc.set_node_state(letter, RendererNodeState::Text(Some("L".to_string())));
    doc.append_child(div, letter);

    let rest = doc.create_node(ElementTag::Span);
    doc.set_style(rest, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(
        rest,
        RendererNodeState::Text(Some("orem ipsum dolor sit amet".to_string())),
    );
    doc.append_child(div, rest);
    Ok(doc.into_engine())
}

/// First-line pseudo-element effect: the first line is rendered in blue
/// (#2196F3) and bold.  The Document builder has no `::first-line` API, so
/// we model this with a styled first-line span followed by normal text.
fn sp13_first_line_basic(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let wrapper = doc.create_node(ElementTag::Div);
    doc.set_style(wrapper, RendererStyleValue::Display(Display::Block));
    doc.set_style(wrapper, RendererStyleValue::Width(Length::px(250.0)));
    doc.append_child(vp, wrapper);

    let para = doc.create_node(ElementTag::Div);
    doc.set_style(para, RendererStyleValue::Display(Display::Block));
    doc.set_style(para, RendererStyleValue::Width(Length::px(250.0)));
    doc.append_child(wrapper, para);

    let first = doc.create_node(ElementTag::Span);
    doc.set_style(first, RendererStyleValue::Display(Display::Inline));
    doc.set_style(
        first,
        RendererStyleValue::Color(Color::from_rgba8(33, 150, 243, 255)),
    ); // #2196F3
    doc.set_style(first, RendererStyleValue::FontWeight(FontWeight::BOLD));
    doc.set_node_state(
        first,
        RendererNodeState::Text(Some("The first line is styled differently".to_string())),
    );
    doc.append_child(para, first);

    let rest = doc.create_node(ElementTag::Span);
    doc.set_style(rest, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(
        rest,
        RendererNodeState::Text(Some(
            " and the remaining text uses the default paragraph style for subsequent lines"
                .to_string(),
        )),
    );
    doc.append_child(para, rest);
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Word Break Tests ───────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_word_break_break_all(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(100.0)));
    doc.set_style(div, RendererStyleValue::WordBreak(WordBreak::BreakAll));
    doc.set_style(
        div,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    doc.set_node_state(
        div,
        RendererNodeState::Text(Some("Supercalifragilisticexpialidocious".to_string())),
    );
    doc.append_child(vp, div);
    Ok(doc.into_engine())
}

fn sp13_overflow_wrap_break_word(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(100.0)));
    doc.set_style(
        div,
        RendererStyleValue::OverflowWrap(OverflowWrap::BreakWord),
    );
    doc.set_style(
        div,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    doc.set_node_state(
        div,
        RendererNodeState::Text(Some("Supercalifragilisticexpialidocious".to_string())),
    );
    doc.append_child(vp, div);
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Vertical Align Extended Tests ──────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_vertical_align_text_top(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    Ok(vertical_align_test(viewport, VerticalAlign::TextTop, "text-top").into_engine())
}

fn sp13_vertical_align_text_bottom(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    Ok(vertical_align_test(viewport, VerticalAlign::TextBottom, "text-bottom").into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Line Breaking Extended Tests ───────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_line_breaking_overflow_wrap(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(150.0)));
    doc.set_style(
        div,
        RendererStyleValue::OverflowWrap(OverflowWrap::Anywhere),
    );
    doc.set_style(
        div,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    doc.set_node_state(
        div,
        RendererNodeState::Text(Some(
            "https://example.com/very/long/path/to/resource/that/should/break".to_string(),
        )),
    );
    doc.append_child(vp, div);
    Ok(doc.into_engine())
}

fn sp13_line_breaking_hyphens_auto(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(120.0)));
    doc.set_style(div, RendererStyleValue::Hyphens(Hyphens::Auto));
    doc.set_style(
        div,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    doc.set_node_state(
        div,
        RendererNodeState::Text(Some(
            "Incomprehensibilities and internationalization are long words".to_string(),
        )),
    );
    doc.append_child(vp, div);
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Box Decoration Break Test ──────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_inline_box_multiline(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(200.0)));
    doc.append_child(vp, div);

    let span = doc.create_node(ElementTag::Span);
    doc.set_style(span, RendererStyleValue::Display(Display::Inline));
    doc.set_style(
        span,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(200, 230, 255, 255)),
    );
    doc.set_style(span, RendererStyleValue::PaddingTop(Length::px(4.0)));
    doc.set_style(span, RendererStyleValue::PaddingRight(Length::px(8.0)));
    doc.set_style(span, RendererStyleValue::PaddingBottom(Length::px(4.0)));
    doc.set_style(span, RendererStyleValue::PaddingLeft(Length::px(8.0)));
    doc.set_style(
        span,
        RendererStyleValue::BoxDecorationBreak(BoxDecorationBreak::Clone),
    );
    doc.set_node_state(
        span,
        RendererNodeState::Text(Some(
            "This inline span has background and padding and wraps to multiple lines".to_string(),
        )),
    );
    doc.append_child(div, span);
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Float Interaction Test ─────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

// (sp13_inline_with_float defined below after new tests)

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Sticky Positioning Extended Tests ───────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_position_sticky_left(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let container = doc.create_node(ElementTag::Div);
    doc.set_style(container, RendererStyleValue::Display(Display::Block));
    doc.set_style(container, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(container, RendererStyleValue::Height(Length::px(300.0)));
    doc.set_style(
        container,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(240, 240, 240, 255)),
    );
    doc.append_child(vp, container);
    let child = add_block(
        &mut doc,
        container,
        50.0,
        50.0,
        Color::from_rgba8(76, 175, 80, 255),
    );
    doc.set_style(child, RendererStyleValue::Position(Position::Sticky));
    doc.set_style(child, RendererStyleValue::Left(Length::px(20.0)));
    Ok(doc.into_engine())
}

fn sp12_position_sticky_right(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let container = doc.create_node(ElementTag::Div);
    doc.set_style(container, RendererStyleValue::Display(Display::Block));
    doc.set_style(container, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(container, RendererStyleValue::Height(Length::px(300.0)));
    doc.set_style(
        container,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(240, 240, 240, 255)),
    );
    doc.append_child(vp, container);
    let child = add_block(
        &mut doc,
        container,
        50.0,
        50.0,
        Color::from_rgba8(76, 175, 80, 255),
    );
    doc.set_style(child, RendererStyleValue::Position(Position::Sticky));
    doc.set_style(child, RendererStyleValue::Right(Length::px(20.0)));
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Multicol Extended Tests ────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_multicol_column_rule(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let container = doc.create_node(ElementTag::Div);
    doc.set_style(container, RendererStyleValue::Display(Display::Block));
    doc.set_style(container, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(container, RendererStyleValue::ColumnCount(Some(2)));
    doc.set_style(
        container,
        RendererStyleValue::ColumnGap(Some(Length::px(20.0))),
    );
    doc.set_style(container, RendererStyleValue::ColumnRuleWidth(2));
    doc.set_style(
        container,
        RendererStyleValue::ColumnRuleStyle(BorderStyle::Solid),
    );
    doc.set_style(
        container,
        RendererStyleValue::ColumnRuleColor(StyleColor::Resolved(Color::RED)),
    );
    doc.set_style(
        container,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(240, 240, 240, 255)),
    );
    doc.append_child(vp, container);
    add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(244, 67, 54, 255),
    );
    add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(76, 175, 80, 255),
    );
    add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(33, 150, 243, 255),
    );
    add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(255, 152, 0, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_multicol_column_span(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let container = doc.create_node(ElementTag::Div);
    doc.set_style(container, RendererStyleValue::Display(Display::Block));
    doc.set_style(container, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(container, RendererStyleValue::ColumnCount(Some(2)));
    doc.set_style(
        container,
        RendererStyleValue::ColumnGap(Some(Length::px(20.0))),
    );
    doc.set_style(
        container,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(240, 240, 240, 255)),
    );
    doc.append_child(vp, container);
    let span_el = add_block(
        &mut doc,
        container,
        380.0,
        20.0,
        Color::from_rgba8(33, 150, 243, 255),
    );
    doc.set_style(span_el, RendererStyleValue::ColumnSpan(ColumnSpan::All));
    add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(244, 67, 54, 255),
    );
    add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(76, 175, 80, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_multicol_column_fill_auto(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let container = doc.create_node(ElementTag::Div);
    doc.set_style(container, RendererStyleValue::Display(Display::Block));
    doc.set_style(container, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(container, RendererStyleValue::Height(Length::px(150.0)));
    doc.set_style(container, RendererStyleValue::ColumnCount(Some(2)));
    doc.set_style(container, RendererStyleValue::ColumnFill(ColumnFill::Auto));
    doc.set_style(
        container,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(240, 240, 240, 255)),
    );
    doc.append_child(vp, container);
    add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(244, 67, 54, 255),
    );
    add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(76, 175, 80, 255),
    );
    add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(33, 150, 243, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_multicol_column_fill_balance(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let container = doc.create_node(ElementTag::Div);
    doc.set_style(container, RendererStyleValue::Display(Display::Block));
    doc.set_style(container, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(container, RendererStyleValue::ColumnCount(Some(2)));
    doc.set_style(
        container,
        RendererStyleValue::ColumnFill(ColumnFill::Balance),
    );
    doc.set_style(
        container,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(240, 240, 240, 255)),
    );
    doc.append_child(vp, container);
    add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(244, 67, 54, 255),
    );
    add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(76, 175, 80, 255),
    );
    add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(33, 150, 243, 255),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Position Offset Tests ──────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_position_absolute_top_left(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    doc.set_style(vp, RendererStyleValue::Position(Position::Relative));
    let div = add_block(&mut doc, vp, 100.0, 100.0, Color::RED);
    doc.set_style(div, RendererStyleValue::Position(Position::Absolute));
    doc.set_style(div, RendererStyleValue::Top(Length::px(10.0)));
    doc.set_style(div, RendererStyleValue::Left(Length::px(10.0)));
    Ok(doc.into_engine())
}

fn sp12_position_absolute_bottom_right(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    // Use an explicit containing block with known dimensions
    let container = doc.create_node(ElementTag::Div);
    doc.set_style(container, RendererStyleValue::Display(Display::Block));
    doc.set_style(container, RendererStyleValue::Position(Position::Relative));
    doc.set_style(container, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(container, RendererStyleValue::Height(Length::px(300.0)));
    doc.set_style(
        container,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(240, 240, 240, 255)),
    );
    doc.append_child(vp, container);
    let div = add_block(&mut doc, container, 100.0, 100.0, Color::BLUE);
    doc.set_style(div, RendererStyleValue::Position(Position::Absolute));
    doc.set_style(div, RendererStyleValue::Bottom(Length::px(10.0)));
    doc.set_style(div, RendererStyleValue::Right(Length::px(10.0)));
    Ok(doc.into_engine())
}

fn sp12_position_relative_top_left(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = add_block(
        &mut doc,
        vp,
        100.0,
        100.0,
        Color::from_rgba8(76, 175, 80, 255),
    );
    doc.set_style(div, RendererStyleValue::Position(Position::Relative));
    doc.set_style(div, RendererStyleValue::Top(Length::px(10.0)));
    doc.set_style(div, RendererStyleValue::Left(Length::px(10.0)));
    Ok(doc.into_engine())
}

fn sp12_position_absolute_percent(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    // Use an explicit containing block with known dimensions
    let container = doc.create_node(ElementTag::Div);
    doc.set_style(container, RendererStyleValue::Display(Display::Block));
    doc.set_style(container, RendererStyleValue::Position(Position::Relative));
    doc.set_style(container, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(container, RendererStyleValue::Height(Length::px(300.0)));
    doc.set_style(
        container,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(240, 240, 240, 255)),
    );
    doc.append_child(vp, container);
    let div = add_block(
        &mut doc,
        container,
        100.0,
        100.0,
        Color::from_rgba8(156, 39, 176, 255),
    );
    doc.set_style(div, RendererStyleValue::Position(Position::Absolute));
    doc.set_style(div, RendererStyleValue::Top(Length::percent(10.0)));
    doc.set_style(div, RendererStyleValue::Left(Length::percent(10.0)));
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Overflow Axis Tests ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_overflow_x_hidden(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let outer = doc.create_node(ElementTag::Div);
    doc.set_style(outer, RendererStyleValue::Display(Display::Block));
    doc.set_style(outer, RendererStyleValue::Width(Length::px(150.0)));
    doc.set_style(outer, RendererStyleValue::Height(Length::px(80.0)));
    doc.set_style(outer, RendererStyleValue::OverflowX(Overflow::Hidden));
    doc.set_style(
        outer,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(200, 200, 200, 255)),
    );
    doc.append_child(vp, outer);
    add_block(&mut doc, outer, 300.0, 60.0, Color::RED);
    Ok(doc.into_engine())
}

fn sp12_overflow_y_hidden(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let outer = doc.create_node(ElementTag::Div);
    doc.set_style(outer, RendererStyleValue::Display(Display::Block));
    doc.set_style(outer, RendererStyleValue::Width(Length::px(150.0)));
    doc.set_style(outer, RendererStyleValue::Height(Length::px(80.0)));
    doc.set_style(outer, RendererStyleValue::OverflowY(Overflow::Hidden));
    doc.set_style(
        outer,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(200, 200, 200, 255)),
    );
    doc.append_child(vp, outer);
    add_block(&mut doc, outer, 130.0, 200.0, Color::BLUE);
    Ok(doc.into_engine())
}

fn sp12_overflow_x_scroll(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let outer = doc.create_node(ElementTag::Div);
    doc.set_style(outer, RendererStyleValue::Display(Display::Block));
    doc.set_style(outer, RendererStyleValue::Width(Length::px(150.0)));
    doc.set_style(outer, RendererStyleValue::Height(Length::px(80.0)));
    doc.set_style(outer, RendererStyleValue::OverflowX(Overflow::Scroll));
    doc.set_style(
        outer,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(200, 200, 200, 255)),
    );
    doc.append_child(vp, outer);
    add_block(&mut doc, outer, 300.0, 60.0, Color::RED);
    Ok(doc.into_engine())
}

fn sp12_overflow_y_scroll(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let outer = doc.create_node(ElementTag::Div);
    doc.set_style(outer, RendererStyleValue::Display(Display::Block));
    doc.set_style(outer, RendererStyleValue::Width(Length::px(150.0)));
    doc.set_style(outer, RendererStyleValue::Height(Length::px(80.0)));
    doc.set_style(outer, RendererStyleValue::OverflowY(Overflow::Scroll));
    doc.set_style(
        outer,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(200, 200, 200, 255)),
    );
    doc.append_child(vp, outer);
    add_block(&mut doc, outer, 130.0, 200.0, Color::BLUE);
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Flex Alignment Extended Tests ──────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_flex_align_self_start(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(flex, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(150.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 80.0, 60.0, Color::RED);
    let item = add_block(&mut doc, flex, 80.0, 60.0, Color::BLUE);
    doc.set_style(
        item,
        RendererStyleValue::AlignSelf(ItemAlignment::new(ItemPosition::FlexStart)),
    );
    add_block(
        &mut doc,
        flex,
        80.0,
        60.0,
        Color::from_rgba8(0, 128, 0, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_flex_align_self_end(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(flex, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(150.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 80.0, 60.0, Color::RED);
    let item = add_block(&mut doc, flex, 80.0, 60.0, Color::BLUE);
    doc.set_style(
        item,
        RendererStyleValue::AlignSelf(ItemAlignment::new(ItemPosition::FlexEnd)),
    );
    add_block(
        &mut doc,
        flex,
        80.0,
        60.0,
        Color::from_rgba8(0, 128, 0, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_flex_align_self_center(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(flex, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(150.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 80.0, 60.0, Color::RED);
    let item = add_block(&mut doc, flex, 80.0, 60.0, Color::BLUE);
    doc.set_style(
        item,
        RendererStyleValue::AlignSelf(ItemAlignment::new(ItemPosition::Center)),
    );
    add_block(
        &mut doc,
        flex,
        80.0,
        60.0,
        Color::from_rgba8(0, 128, 0, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_flex_align_content_center(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(flex, RendererStyleValue::FlexWrap(FlexWrap::Wrap));
    doc.set_style(
        flex,
        RendererStyleValue::AlignContent(ContentAlignment::new(ContentPosition::Center)),
    );
    doc.set_style(flex, RendererStyleValue::Width(Length::px(300.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(200.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 100.0, 50.0, Color::RED);
    add_block(&mut doc, flex, 100.0, 50.0, Color::BLUE);
    add_block(
        &mut doc,
        flex,
        100.0,
        50.0,
        Color::from_rgba8(0, 128, 0, 255),
    );
    add_block(
        &mut doc,
        flex,
        100.0,
        50.0,
        Color::from_rgba8(255, 165, 0, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_flex_align_content_space_between(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(flex, RendererStyleValue::FlexWrap(FlexWrap::Wrap));
    doc.set_style(
        flex,
        RendererStyleValue::AlignContent(ContentAlignment::with_distribution(
            ContentDistribution::SpaceBetween,
        )),
    );
    doc.set_style(flex, RendererStyleValue::Width(Length::px(300.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(200.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 100.0, 50.0, Color::RED);
    add_block(&mut doc, flex, 100.0, 50.0, Color::BLUE);
    add_block(
        &mut doc,
        flex,
        100.0,
        50.0,
        Color::from_rgba8(0, 128, 0, 255),
    );
    add_block(
        &mut doc,
        flex,
        100.0,
        50.0,
        Color::from_rgba8(255, 165, 0, 255),
    );
    Ok(doc.into_engine())
}

fn sp12_flex_order(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(flex, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(80.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    // DOM order: red, blue, green — but visual order: green(1), blue(2), red(3)
    let red = add_block(&mut doc, flex, 80.0, 60.0, Color::RED);
    doc.set_style(red, RendererStyleValue::Order(3));
    let blue = add_block(&mut doc, flex, 80.0, 60.0, Color::BLUE);
    doc.set_style(blue, RendererStyleValue::Order(2));
    let green = add_block(
        &mut doc,
        flex,
        80.0,
        60.0,
        Color::from_rgba8(0, 128, 0, 255),
    );
    doc.set_style(green, RendererStyleValue::Order(1));
    Ok(doc.into_engine())
}

fn sp12_flex_basis_100px(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(flex, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(80.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    let item = doc.create_node(ElementTag::Div);
    doc.set_style(item, RendererStyleValue::Display(Display::Block));
    doc.set_style(item, RendererStyleValue::FlexBasis(Length::px(100.0)));
    doc.set_style(item, RendererStyleValue::Height(Length::px(60.0)));
    doc.set_style(item, RendererStyleValue::BackgroundColor(Color::RED));
    doc.append_child(flex, item);
    add_block(&mut doc, flex, 80.0, 60.0, Color::BLUE);
    Ok(doc.into_engine())
}

fn sp12_flex_shrink_basic(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let flex = doc.create_node(ElementTag::Div);
    doc.set_style(flex, RendererStyleValue::Display(Display::Flex));
    doc.set_style(flex, RendererStyleValue::Width(Length::px(200.0)));
    doc.set_style(flex, RendererStyleValue::Height(Length::px(80.0)));
    doc.set_style(
        flex,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(220, 220, 220, 255)),
    );
    doc.append_child(vp, flex);
    let a = doc.create_node(ElementTag::Div);
    doc.set_style(a, RendererStyleValue::Display(Display::Block));
    doc.set_style(a, RendererStyleValue::Width(Length::px(150.0)));
    doc.set_style(a, RendererStyleValue::Height(Length::px(60.0)));
    doc.set_style(a, RendererStyleValue::FlexShrink(1.0));
    doc.set_style(a, RendererStyleValue::BackgroundColor(Color::RED));
    doc.append_child(flex, a);
    let b = doc.create_node(ElementTag::Div);
    doc.set_style(b, RendererStyleValue::Display(Display::Block));
    doc.set_style(b, RendererStyleValue::Width(Length::px(150.0)));
    doc.set_style(b, RendererStyleValue::Height(Length::px(60.0)));
    doc.set_style(b, RendererStyleValue::FlexShrink(2.0));
    doc.set_style(b, RendererStyleValue::BackgroundColor(Color::BLUE));
    doc.append_child(flex, b);
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Border Variant Tests ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_border_style_dashed(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = add_block(
        &mut doc,
        vp,
        200.0,
        100.0,
        Color::from_rgba8(240, 240, 240, 255),
    );
    doc.set_style(div, RendererStyleValue::BorderTopWidth(3));
    doc.set_style(div, RendererStyleValue::BorderRightWidth(3));
    doc.set_style(div, RendererStyleValue::BorderBottomWidth(3));
    doc.set_style(div, RendererStyleValue::BorderLeftWidth(3));
    doc.set_style(div, RendererStyleValue::BorderTopStyle(BorderStyle::Dashed));
    doc.set_style(
        div,
        RendererStyleValue::BorderRightStyle(BorderStyle::Dashed),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderBottomStyle(BorderStyle::Dashed),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderLeftStyle(BorderStyle::Dashed),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderTopColor(StyleColor::Resolved(Color::BLACK)),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderRightColor(StyleColor::Resolved(Color::BLACK)),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderBottomColor(StyleColor::Resolved(Color::BLACK)),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderLeftColor(StyleColor::Resolved(Color::BLACK)),
    );
    Ok(doc.into_engine())
}

fn sp12_border_style_dotted(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = add_block(
        &mut doc,
        vp,
        200.0,
        100.0,
        Color::from_rgba8(240, 240, 240, 255),
    );
    doc.set_style(div, RendererStyleValue::BorderTopWidth(3));
    doc.set_style(div, RendererStyleValue::BorderRightWidth(3));
    doc.set_style(div, RendererStyleValue::BorderBottomWidth(3));
    doc.set_style(div, RendererStyleValue::BorderLeftWidth(3));
    doc.set_style(div, RendererStyleValue::BorderTopStyle(BorderStyle::Dotted));
    doc.set_style(
        div,
        RendererStyleValue::BorderRightStyle(BorderStyle::Dotted),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderBottomStyle(BorderStyle::Dotted),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderLeftStyle(BorderStyle::Dotted),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderTopColor(StyleColor::Resolved(Color::BLACK)),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderRightColor(StyleColor::Resolved(Color::BLACK)),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderBottomColor(StyleColor::Resolved(Color::BLACK)),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderLeftColor(StyleColor::Resolved(Color::BLACK)),
    );
    Ok(doc.into_engine())
}

fn sp12_border_color_per_side(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = add_block(&mut doc, vp, 200.0, 100.0, Color::WHITE);
    doc.set_style(div, RendererStyleValue::BorderTopWidth(5));
    doc.set_style(div, RendererStyleValue::BorderRightWidth(5));
    doc.set_style(div, RendererStyleValue::BorderBottomWidth(5));
    doc.set_style(div, RendererStyleValue::BorderLeftWidth(5));
    doc.set_style(div, RendererStyleValue::BorderTopStyle(BorderStyle::Solid));
    doc.set_style(
        div,
        RendererStyleValue::BorderRightStyle(BorderStyle::Solid),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderBottomStyle(BorderStyle::Solid),
    );
    doc.set_style(div, RendererStyleValue::BorderLeftStyle(BorderStyle::Solid));
    doc.set_style(
        div,
        RendererStyleValue::BorderTopColor(StyleColor::Resolved(Color::RED)),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderRightColor(StyleColor::Resolved(Color::BLUE)),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderBottomColor(StyleColor::Resolved(Color::from_rgba8(
            0, 128, 0, 255,
        ))),
    );
    doc.set_style(
        div,
        RendererStyleValue::BorderLeftColor(StyleColor::Resolved(Color::from_rgba8(
            255, 165, 0, 255,
        ))),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Fragmentation Tests ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_fragmentation_break_before_column(
    viewport: ViewportMetrics,
) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let container = doc.create_node(ElementTag::Div);
    doc.set_style(container, RendererStyleValue::Display(Display::Block));
    doc.set_style(container, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(container, RendererStyleValue::ColumnCount(Some(2)));
    doc.set_style(
        container,
        RendererStyleValue::ColumnGap(Some(Length::px(20.0))),
    );
    doc.set_style(
        container,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(240, 240, 240, 255)),
    );
    doc.append_child(vp, container);
    add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(244, 67, 54, 255),
    );
    let b = add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(33, 150, 243, 255),
    );
    doc.set_style(b, RendererStyleValue::BreakBefore(BreakValue::Column));
    Ok(doc.into_engine())
}

fn sp12_fragmentation_break_inside_avoid(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let container = doc.create_node(ElementTag::Div);
    doc.set_style(container, RendererStyleValue::Display(Display::Block));
    doc.set_style(container, RendererStyleValue::Width(Length::px(400.0)));
    doc.set_style(container, RendererStyleValue::Height(Length::px(120.0)));
    doc.set_style(container, RendererStyleValue::ColumnCount(Some(2)));
    doc.set_style(
        container,
        RendererStyleValue::ColumnGap(Some(Length::px(20.0))),
    );
    doc.set_style(
        container,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(240, 240, 240, 255)),
    );
    doc.append_child(vp, container);
    let a = add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(244, 67, 54, 255),
    );
    doc.set_style(a, RendererStyleValue::BreakInside(BreakInside::Avoid));
    let b = add_block(
        &mut doc,
        container,
        180.0,
        50.0,
        Color::from_rgba8(33, 150, 243, 255),
    );
    doc.set_style(b, RendererStyleValue::BreakInside(BreakInside::Avoid));
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 First-Letter / First-Line Extended Tests ────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_first_letter_color(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.append_child(vp, div);
    let letter = doc.create_node(ElementTag::Span);
    doc.set_style(letter, RendererStyleValue::Display(Display::Inline));
    doc.set_style(letter, RendererStyleValue::Color(Color::RED));
    doc.set_node_state(letter, RendererNodeState::Text(Some("H".to_string())));
    doc.append_child(div, letter);
    let rest = doc.create_node(ElementTag::Span);
    doc.set_style(rest, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(
        rest,
        RendererNodeState::Text(Some("ello World".to_string())),
    );
    doc.append_child(div, rest);
    Ok(doc.into_engine())
}

fn sp13_first_letter_font_size(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.append_child(vp, div);
    let letter = doc.create_node(ElementTag::Span);
    doc.set_style(letter, RendererStyleValue::Display(Display::Inline));
    doc.set_style(letter, RendererStyleValue::FontSize(32.0)); // 2em at 16px base
    doc.set_node_state(letter, RendererNodeState::Text(Some("H".to_string())));
    doc.append_child(div, letter);
    let rest = doc.create_node(ElementTag::Span);
    doc.set_style(rest, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(
        rest,
        RendererNodeState::Text(Some("ello World".to_string())),
    );
    doc.append_child(div, rest);
    Ok(doc.into_engine())
}

fn sp13_first_line_font_weight(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(300.0)));
    doc.append_child(vp, div);
    let first = doc.create_node(ElementTag::Span);
    doc.set_style(first, RendererStyleValue::Display(Display::Inline));
    doc.set_style(first, RendererStyleValue::FontWeight(FontWeight::BOLD));
    doc.set_node_state(
        first,
        RendererNodeState::Text(Some("The first line of text is bold".to_string())),
    );
    doc.append_child(div, first);
    let rest = doc.create_node(ElementTag::Span);
    doc.set_style(rest, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(
        rest,
        RendererNodeState::Text(Some(
            " and the remaining text is normal weight for the second line.".to_string(),
        )),
    );
    doc.append_child(div, rest);
    Ok(doc.into_engine())
}

fn sp13_first_line_color(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(300.0)));
    doc.append_child(vp, div);
    let first = doc.create_node(ElementTag::Span);
    doc.set_style(first, RendererStyleValue::Display(Display::Inline));
    doc.set_style(first, RendererStyleValue::Color(Color::BLUE));
    doc.set_node_state(
        first,
        RendererNodeState::Text(Some("The first line of text is blue colored".to_string())),
    );
    doc.append_child(div, first);
    let rest = doc.create_node(ElementTag::Span);
    doc.set_style(rest, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(
        rest,
        RendererNodeState::Text(Some(
            " and the remaining text is the default black color for subsequent lines.".to_string(),
        )),
    );
    doc.append_child(div, rest);
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Inline Advanced Tests ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_initial_letter_basic(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.append_child(vp, div);
    // Model the drop-cap with a float-left oversized letter
    let letter = doc.create_node(ElementTag::Span);
    doc.set_style(letter, RendererStyleValue::Display(Display::InlineBlock));
    doc.set_style(letter, RendererStyleValue::FontSize(48.0)); // ~3 lines tall
    doc.set_style(letter, RendererStyleValue::Float(Float::Left));
    doc.set_style(
        letter,
        RendererStyleValue::LineHeight(LineHeight::Number(1.0)),
    );
    doc.set_style(letter, RendererStyleValue::MarginRight(Length::px(4.0)));
    doc.set_node_state(letter, RendererNodeState::Text(Some("L".to_string())));
    doc.append_child(div, letter);
    let rest = doc.create_node(ElementTag::Span);
    doc.set_style(rest, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(
        rest,
        RendererNodeState::Text(Some(
            "orem ipsum dolor sit amet, consectetur adipiscing elit.".to_string(),
        )),
    );
    doc.append_child(div, rest);
    Ok(doc.into_engine())
}

fn sp13_ruby_basic(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    // Model ruby annotation as a small span above base text using inline-block stacking
    for (base, ann) in [("漢", "かん"), ("字", "じ")] {
        let wrapper = doc.create_node(ElementTag::Div);
        doc.set_style(wrapper, RendererStyleValue::Display(Display::InlineBlock));
        doc.set_style(wrapper, RendererStyleValue::FontSize(20.0));
        doc.append_child(vp, wrapper);

        let rt = doc.create_node(ElementTag::Div);
        doc.set_style(rt, RendererStyleValue::Display(Display::Block));
        doc.set_style(rt, RendererStyleValue::FontSize(10.0));
        doc.set_node_state(rt, RendererNodeState::Text(Some(ann.to_string())));
        doc.append_child(wrapper, rt);

        let base_span = doc.create_node(ElementTag::Div);
        doc.set_style(base_span, RendererStyleValue::Display(Display::Block));
        doc.set_node_state(base_span, RendererNodeState::Text(Some(base.to_string())));
        doc.append_child(wrapper, base_span);
    }
    Ok(doc.into_engine())
}

fn sp13_text_combine_upright(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(
        div,
        RendererStyleValue::TextCombineUpright(TextCombineUpright::All),
    );
    doc.set_node_state(div, RendererNodeState::Text(Some("年".to_string())));
    doc.append_child(vp, div);
    Ok(doc.into_engine())
}

fn sp13_inline_direction_rtl(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Direction(Direction::Rtl));
    doc.set_style(div, RendererStyleValue::Width(Length::px(300.0)));
    doc.set_node_state(
        div,
        RendererNodeState::Text(Some("Hello World".to_string())),
    );
    doc.append_child(vp, div);
    Ok(doc.into_engine())
}

fn sp13_tab_size_4(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::WhiteSpace(WhiteSpace::Pre));
    doc.set_style(div, RendererStyleValue::TabSize(TabSize::Spaces(4)));
    doc.set_node_state(
        div,
        RendererNodeState::Text(Some("\tIndented with tab".to_string())),
    );
    doc.append_child(vp, div);
    Ok(doc.into_engine())
}

fn sp13_text_align_last_center(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(300.0)));
    doc.set_style(
        div,
        RendererStyleValue::TextAlignLast(TextAlignLast::Center),
    );
    doc.set_style(
        div,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(230, 230, 230, 255)),
    );
    doc.set_node_state(
        div,
        RendererNodeState::Text(Some("Last line centered".to_string())),
    );
    doc.append_child(vp, div);
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Emphasis Tests ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_text_emphasis_dot(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = add_text_block(&mut doc, vp, "Hello World");
    doc.set_style(
        div,
        RendererStyleValue::TextEmphasisMark(TextEmphasisMark::Dot),
    );
    doc.set_style(
        div,
        RendererStyleValue::TextEmphasisFill(TextEmphasisFill::Filled),
    );
    Ok(doc.into_engine())
}

fn sp11_text_emphasis_circle(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = add_text_block(&mut doc, vp, "Hello World");
    doc.set_style(
        div,
        RendererStyleValue::TextEmphasisMark(TextEmphasisMark::Circle),
    );
    doc.set_style(
        div,
        RendererStyleValue::TextEmphasisFill(TextEmphasisFill::Filled),
    );
    Ok(doc.into_engine())
}

fn sp11_text_emphasis_position_over(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = add_text_block(&mut doc, vp, "Hello World");
    doc.set_style(
        div,
        RendererStyleValue::TextEmphasisMark(TextEmphasisMark::Dot),
    );
    doc.set_style(
        div,
        RendererStyleValue::TextEmphasisFill(TextEmphasisFill::Filled),
    );
    doc.set_style(
        div,
        RendererStyleValue::TextEmphasisPosition(TextEmphasisPosition {
            over: true,
            right: true,
        }),
    );
    Ok(doc.into_engine())
}

fn sp11_text_emphasis_color_red(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = add_text_block(&mut doc, vp, "Hello World");
    doc.set_style(
        div,
        RendererStyleValue::TextEmphasisMark(TextEmphasisMark::Dot),
    );
    doc.set_style(
        div,
        RendererStyleValue::TextEmphasisFill(TextEmphasisFill::Filled),
    );
    doc.set_style(
        div,
        RendererStyleValue::TextEmphasisColor(StyleColor::Resolved(Color::RED)),
    );
    Ok(doc.into_engine())
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Float Interaction Test ─────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_inline_with_float(viewport: ViewportMetrics) -> Result<Engine, EngineError> {
    let (mut doc, vp) = base_doc(viewport);
    let div = doc.create_node(ElementTag::Div);
    doc.set_style(div, RendererStyleValue::Display(Display::Block));
    doc.set_style(div, RendererStyleValue::Width(Length::px(300.0)));
    doc.set_style(div, RendererStyleValue::OverflowX(Overflow::Hidden));
    doc.set_style(div, RendererStyleValue::OverflowY(Overflow::Hidden));
    doc.append_child(vp, div);

    let float_box = doc.create_node(ElementTag::Div);
    doc.set_style(float_box, RendererStyleValue::Display(Display::Block));
    doc.set_style(float_box, RendererStyleValue::Float(Float::Left));
    doc.set_style(float_box, RendererStyleValue::Width(Length::px(80.0)));
    doc.set_style(float_box, RendererStyleValue::Height(Length::px(80.0)));
    doc.set_style(
        float_box,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(255, 200, 200, 255)),
    );
    doc.set_style(float_box, RendererStyleValue::MarginRight(Length::px(10.0)));
    doc.append_child(div, float_box);

    let text = doc.create_node(ElementTag::Span);
    doc.set_style(text, RendererStyleValue::Display(Display::Inline));
    doc.set_node_state(text, RendererNodeState::Text(Some(
        "Inline text wraps around the floated box. More text to ensure wrapping below the float."
            .to_string(),
    )));
    doc.append_child(div, text);
    Ok(doc.into_engine())
}

#[cfg(test)]
mod profile_tests {
    use super::*;

    #[test]
    fn adjacent_row_flex_items_keep_their_own_height_across_columns() {
        fn second_or_third_column_items(
            test_id: &str,
            column: usize,
        ) -> Vec<openui_layout::Fragment> {
            let (viewport, _) = parse_viewport_options(&[
                "--viewport".into(),
                "800x600".into(),
                "--scale".into(),
                "1".into(),
            ])
            .unwrap();
            let tests = registry();
            let (_, builder) = tests.iter().find(|(id, _)| *id == test_id).unwrap();
            let mut engine = builder(viewport).unwrap();
            let scene = engine.scene().unwrap();
            fn columns<'a>(
                fragment: &'a openui_layout::Fragment,
                out: &mut Vec<&'a openui_layout::Fragment>,
            ) {
                if fragment.kind == openui_layout::FragmentKind::ColumnBox {
                    out.push(fragment);
                }
                for child in &fragment.children {
                    columns(child, out);
                }
            }
            let mut found = Vec::new();
            columns(scene.fragments(), &mut found);
            found[column].children[0].children.clone()
        }

        let adjacent = second_or_third_column_items(
            "wpt/css_break/flexbox_multi-line-row-flex-fragmentation-001",
            2,
        );
        assert_eq!(
            adjacent[0].size.height,
            openui_geometry::LayoutUnit::from_i32(250)
        );
        assert_eq!(
            adjacent[1].size.height,
            openui_geometry::LayoutUnit::from_i32(250)
        );
        assert_eq!(
            adjacent[2].offset.top,
            openui_geometry::LayoutUnit::from_i32(50)
        );

        // A forced break with no following item still needs a visual
        // continuation. It must keep its green background through the column.
        let continued = second_or_third_column_items(
            "wpt/css_break/flexbox_multi-line-row-flex-fragmentation-023",
            1,
        );
        assert_eq!(
            continued[2].offset.top,
            openui_geometry::LayoutUnit::from_i32(-20)
        );
        assert_eq!(
            continued[2].size.height,
            openui_geometry::LayoutUnit::from_i32(120)
        );
    }

    #[test]
    fn qualification_profile_uses_logical_authority_and_winit_rounding() {
        let options = vec![
            "--viewport".to_string(),
            "375x667".to_string(),
            "--scale".to_string(),
            "1.25".to_string(),
        ];
        let (viewport, raster_configuration) = parse_viewport_options(&options).unwrap();
        assert_eq!(viewport.logical_size(), (375.0, 667.0));
        assert_eq!(viewport.physical_size(), (469, 834));
        assert_eq!(viewport.device_scale_factor(), 1.25);
        assert_eq!(raster_configuration, RasterConfiguration::default());
    }

    #[test]
    fn qualification_profile_rejects_invalid_or_unknown_options() {
        assert!(parse_viewport_options(&["--viewport".into(), "800".into()]).is_err());
        assert!(parse_viewport_options(&["--scale".into(), "0".into()]).is_err());
        assert!(parse_viewport_options(&["--device-pixel-ratio".into()]).is_err());
    }

    #[test]
    fn ganesh_backend_selection_is_explicit() {
        let (_, configuration) = parse_viewport_options(&[
            "--backend".into(),
            "ganesh-gl".into(),
            "--raster-config".into(),
            "chromium-linux-lcd".into(),
        ])
        .unwrap();
        assert_eq!(configuration, RasterConfiguration::chromium_linux_ganesh());
        assert!(parse_viewport_options(&["--backend".into(), "auto".into()]).is_err());
    }
}
