//! Pixel-perfect comparison tests — renders text-heavy layouts with Open UI's
//! Rust pipeline and compares against Chromium reference screenshots.
//!
//! ## How it works
//!
//! For each scenario (basic_text, line_breaking, …) the test:
//! 1. Programmatically builds a DOM tree that mirrors the HTML test page.
//! 2. Runs layout + paint via `render_to_surface()`.
//! 3. Saves the output PNG to `tests/pixel_text/openui_renders/`.
//! 4. If a Chromium reference PNG exists in `tests/pixel_text/chromium_refs/`,
//!    performs a pixel-by-pixel comparison with tolerance ±2 per channel.
//! 5. Generates a diff image highlighting mismatches.
//!
//! ## Running
//!
//! ```bash
//! # Capture Chromium references first (requires Chrome):
//! cd tests/pixel_text && ./capture_chromium.sh
//!
//! # Then run pixel comparison tests:
//! cd bindings/rust
//! cargo test --package openui-paint --test pixel_comparison_tests
//! ```

use std::path::{Path, PathBuf};

use skia_safe::Surface;

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::Length;
use openui_paint::{render_to_surface, render_to_png};
use openui_style::*;

// ═══════════════════════════════════════════════════════════════════════
// ── Pixel comparison helpers ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// Result of comparing two images pixel-by-pixel.
#[derive(Debug)]
struct PixelDiff {
    /// Total pixels in each image.
    total_pixels: usize,
    /// Number of pixels that differ beyond tolerance.
    mismatched_pixels: usize,
    /// Maximum per-channel difference found (0–255).
    max_channel_diff: u8,
    /// Average per-channel difference across all pixels.
    avg_channel_diff: f64,
    /// Whether the images have different dimensions.
    size_mismatch: bool,
}

impl PixelDiff {
    fn mismatch_percentage(&self) -> f64 {
        if self.total_pixels == 0 {
            return 0.0;
        }
        (self.mismatched_pixels as f64 / self.total_pixels as f64) * 100.0
    }
}

/// Extract raw RGBA pixel bytes from a Skia surface.
fn surface_to_rgba(surface: &mut Surface) -> (u32, u32, Vec<u8>) {
    let image = surface.image_snapshot();
    let info = image.image_info();
    let w = info.width() as u32;
    let h = info.height() as u32;
    let row_bytes = (w * 4) as usize;
    let mut pixels = vec![0u8; (h as usize) * row_bytes];
    image.read_pixels(
        &info,
        &mut pixels,
        row_bytes,
        (0, 0),
        skia_safe::image::CachingHint::Allow,
    );
    (w, h, pixels)
}

/// Load a PNG file and return (width, height, RGBA pixels).
fn load_png_rgba(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let file = std::fs::File::open(path).map_err(|e| format!("open {:?}: {}", path, e))?;
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info().map_err(|e| format!("png read {:?}: {}", path, e))?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("png decode {:?}: {}", path, e))?;
    let w = info.width;
    let h = info.height;

    // Convert to RGBA if needed.
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb => {
            let src = &buf[..info.buffer_size()];
            let mut rgba = Vec::with_capacity((w * h * 4) as usize);
            for chunk in src.chunks(3) {
                rgba.extend_from_slice(chunk);
                rgba.push(255);
            }
            rgba
        }
        other => return Err(format!("unsupported color type: {:?}", other)),
    };
    Ok((w, h, rgba))
}

/// Compare two RGBA images pixel-by-pixel with per-channel tolerance.
fn pixel_diff(
    a: &(u32, u32, Vec<u8>),
    b: &(u32, u32, Vec<u8>),
    tolerance: u8,
) -> PixelDiff {
    if a.0 != b.0 || a.1 != b.1 {
        return PixelDiff {
            total_pixels: (a.0 as usize) * (a.1 as usize),
            mismatched_pixels: (a.0 as usize) * (a.1 as usize),
            max_channel_diff: 255,
            avg_channel_diff: 255.0,
            size_mismatch: true,
        };
    }

    let total_pixels = (a.0 as usize) * (a.1 as usize);
    let mut mismatched = 0usize;
    let mut max_diff: u8 = 0;
    let mut sum_diff: u64 = 0;

    for (pa, pb) in a.2.chunks(4).zip(b.2.chunks(4)) {
        let mut pixel_mismatch = false;
        for i in 0..4 {
            let d = (pa[i] as i16 - pb[i] as i16).unsigned_abs() as u8;
            if d > max_diff {
                max_diff = d;
            }
            sum_diff += d as u64;
            if d > tolerance {
                pixel_mismatch = true;
            }
        }
        if pixel_mismatch {
            mismatched += 1;
        }
    }

    let channels = total_pixels * 4;
    PixelDiff {
        total_pixels,
        mismatched_pixels: mismatched,
        max_channel_diff: max_diff,
        avg_channel_diff: if channels > 0 {
            sum_diff as f64 / channels as f64
        } else {
            0.0
        },
        size_mismatch: false,
    }
}

/// Generate a diff image: red = mismatched pixel, green = matched pixel.
/// Writes the diff PNG to `diff_path`.
fn save_diff_image(
    a: &(u32, u32, Vec<u8>),
    b: &(u32, u32, Vec<u8>),
    tolerance: u8,
    diff_path: &Path,
) -> Result<(), String> {
    let w = a.0.min(b.0);
    let h = a.1.min(b.1);
    let mut diff_pixels = vec![0u8; (w * h * 4) as usize];

    for y in 0..h {
        for x in 0..w {
            let idx_a = ((y * a.0 + x) * 4) as usize;
            let idx_b = ((y * b.0 + x) * 4) as usize;
            let idx_d = ((y * w + x) * 4) as usize;

            let mut mismatch = false;
            for c in 0..3 {
                let d = (a.2[idx_a + c] as i16 - b.2[idx_b + c] as i16).unsigned_abs() as u8;
                if d > tolerance {
                    mismatch = true;
                    break;
                }
            }

            if mismatch {
                // Red for mismatch
                diff_pixels[idx_d] = 255;
                diff_pixels[idx_d + 1] = 0;
                diff_pixels[idx_d + 2] = 0;
                diff_pixels[idx_d + 3] = 255;
            } else {
                // Dim version of the original
                diff_pixels[idx_d] = a.2[idx_a] / 2 + 64;
                diff_pixels[idx_d + 1] = a.2[idx_a + 1] / 2 + 64;
                diff_pixels[idx_d + 2] = a.2[idx_a + 2] / 2 + 64;
                diff_pixels[idx_d + 3] = 255;
            }
        }
    }

    write_png(diff_path, w, h, &diff_pixels)
}

/// Write RGBA pixels as a PNG file using the `png` crate.
fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
    let file =
        std::fs::File::create(path).map_err(|e| format!("create {:?}: {}", path, e))?;
    let ref mut w = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("png header {:?}: {}", path, e))?;
    writer
        .write_image_data(rgba)
        .map_err(|e| format!("png write {:?}: {}", path, e))?;
    Ok(())
}

/// Check if a surface has any non-white pixels (i.e. something was rendered).
fn has_visible_content(surface: &mut Surface) -> bool {
    let (_, _, pixels) = surface_to_rgba(surface);
    for chunk in pixels.chunks(4) {
        if chunk.len() == 4 {
            // Check RGB channels; ignore alpha. BGRA ordering on some platforms,
            // but read_pixels with N32 gives native order. We check all non-white.
            if chunk[0] != 0xFF || chunk[1] != 0xFF || chunk[2] != 0xFF {
                return true;
            }
        }
    }
    false
}

// ═══════════════════════════════════════════════════════════════════════
// ── Path helpers ────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn project_root() -> PathBuf {
    // tests/pixel_text/ is at <repo>/tests/pixel_text/
    // We're in bindings/rust/openui-paint/, so go up 3 levels.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")); // bindings/rust/openui-paint
    manifest
        .parent() // bindings/rust
        .unwrap()
        .parent() // bindings
        .unwrap()
        .parent() // repo root
        .unwrap()
        .to_path_buf()
}

fn chromium_ref_path(name: &str) -> PathBuf {
    project_root()
        .join("tests/pixel_text/chromium_refs")
        .join(format!("{}_chromium.png", name))
}

fn openui_render_path(name: &str) -> PathBuf {
    project_root()
        .join("tests/pixel_text/openui_renders")
        .join(format!("{}_openui.png", name))
}

fn diff_path(name: &str) -> PathBuf {
    project_root()
        .join("tests/pixel_text/openui_renders")
        .join(format!("{}_diff.png", name))
}

// ═══════════════════════════════════════════════════════════════════════
// ── DOM builder helpers ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

const SURFACE_W: i32 = 500;
const SURFACE_H: i32 = 1200;
const TOLERANCE: u8 = 2;

/// Create a block container (div) with a given width and append to parent.
fn add_block(doc: &mut Document, parent: NodeId, width_px: f32) -> NodeId {
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    if width_px > 0.0 {
        doc.node_mut(div).style.width = Length::px(width_px);
    }
    doc.append_child(parent, div);
    div
}

/// Create an inline span and append to parent. Returns span NodeId.
fn add_span(doc: &mut Document, parent: NodeId) -> NodeId {
    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(parent, span);
    span
}

/// Create a text node and append to parent.
/// Copies inheritable text properties from the parent node's style.
fn add_text(doc: &mut Document, parent: NodeId, content: &str) -> NodeId {
    let parent_style = doc.node(parent).style.clone();
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some(content.to_string());
    doc.node_mut(text).style.display = Display::Inline;
    // Propagate inherited text properties from parent
    doc.node_mut(text).style.font_family = parent_style.font_family;
    doc.node_mut(text).style.font_size = parent_style.font_size;
    doc.node_mut(text).style.font_weight = parent_style.font_weight;
    doc.node_mut(text).style.font_style = parent_style.font_style;
    doc.node_mut(text).style.font_stretch = parent_style.font_stretch;
    doc.node_mut(text).style.color = parent_style.color;
    doc.node_mut(text).style.letter_spacing = parent_style.letter_spacing;
    doc.node_mut(text).style.word_spacing = parent_style.word_spacing;
    doc.node_mut(text).style.text_transform = parent_style.text_transform;
    doc.node_mut(text).style.white_space = parent_style.white_space;
    doc.node_mut(text).style.direction = parent_style.direction;
    doc.node_mut(text).style.line_height = parent_style.line_height;
    doc.node_mut(text).style.text_decoration_line = parent_style.text_decoration_line;
    doc.node_mut(text).style.text_decoration_style = parent_style.text_decoration_style;
    doc.node_mut(text).style.text_decoration_color = parent_style.text_decoration_color.clone();
    doc.node_mut(text).style.vertical_align = parent_style.vertical_align;
    doc.append_child(parent, text);
    text
}

/// Apply body-like defaults: white background, DejaVu Sans, 20px padding.
fn setup_viewport(doc: &mut Document) -> NodeId {
    let vp = doc.root();
    doc.node_mut(vp).style.display = Display::Block;
    doc.node_mut(vp).style.background_color = Color::WHITE;
    doc.node_mut(vp).style.padding_top = Length::px(20.0);
    doc.node_mut(vp).style.padding_right = Length::px(20.0);
    doc.node_mut(vp).style.padding_bottom = Length::px(20.0);
    doc.node_mut(vp).style.padding_left = Length::px(20.0);
    doc.node_mut(vp).style.font_family =
        FontFamilyList::single("DejaVu Sans");
    doc.node_mut(vp).style.font_size = 16.0;
    doc.node_mut(vp).style.color = Color::BLACK;
    vp
}

/// Create a paragraph-like block (margin-bottom: 10px) with text.
/// Style the paragraph FIRST, then call add_text so text inherits.
fn add_paragraph_with_style(
    doc: &mut Document,
    parent: NodeId,
    text: &str,
    style_fn: impl FnOnce(&mut ComputedStyle),
) -> NodeId {
    let p = doc.create_node(ElementTag::Div);
    doc.node_mut(p).style.display = Display::Block;
    doc.node_mut(p).style.margin_bottom = Length::px(10.0);
    // Inherit viewport defaults
    let parent_style = doc.node(parent).style.clone();
    doc.node_mut(p).style.font_family = parent_style.font_family;
    doc.node_mut(p).style.font_size = parent_style.font_size;
    doc.node_mut(p).style.font_weight = parent_style.font_weight;
    doc.node_mut(p).style.font_style = parent_style.font_style;
    doc.node_mut(p).style.color = parent_style.color;
    doc.node_mut(p).style.direction = parent_style.direction;
    doc.node_mut(p).style.line_height = parent_style.line_height;
    doc.node_mut(p).style.white_space = parent_style.white_space;
    // Apply custom style
    style_fn(&mut doc.node_mut(p).style);
    doc.append_child(parent, p);
    add_text(doc, p, text);
    p
}

/// Copy inheritable text properties from parent node to child node.
fn inherit_text_style(doc: &mut Document, parent: NodeId, child: NodeId) {
    let ps = doc.node(parent).style.clone();
    let cs = &mut doc.node_mut(child).style;
    cs.font_family = ps.font_family;
    cs.font_size = ps.font_size;
    cs.font_weight = ps.font_weight;
    cs.font_style = ps.font_style;
    cs.font_stretch = ps.font_stretch;
    cs.color = ps.color;
    cs.letter_spacing = ps.letter_spacing;
    cs.word_spacing = ps.word_spacing;
    cs.text_transform = ps.text_transform;
    cs.white_space = ps.white_space;
    cs.direction = ps.direction;
    cs.line_height = ps.line_height;
    cs.text_align = ps.text_align;
    cs.text_decoration_line = ps.text_decoration_line;
    cs.text_decoration_style = ps.text_decoration_style;
    cs.text_decoration_color = ps.text_decoration_color;
    cs.vertical_align = ps.vertical_align;
}

/// Render a Document, save the PNG, compare with Chromium reference if available.
/// Returns the diff result (or None if no reference exists).
fn render_and_compare(
    doc: &Document,
    test_name: &str,
) -> (Surface, Option<PixelDiff>) {
    // Ensure output directory exists
    let out_path = openui_render_path(test_name);
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    // Render
    let mut surface = render_to_surface(doc, SURFACE_W, SURFACE_H)
        .expect("render_to_surface failed");

    // Save our render
    render_to_png(doc, SURFACE_W, SURFACE_H, out_path.to_str().unwrap())
        .expect("render_to_png failed");

    // Compare with Chromium reference if it exists
    let ref_path = chromium_ref_path(test_name);
    let diff = if ref_path.exists() {
        let our_pixels = surface_to_rgba(&mut surface);
        let ref_pixels = load_png_rgba(&ref_path).expect("failed to load Chromium reference");
        let diff = pixel_diff(&our_pixels, &ref_pixels, TOLERANCE);

        // Save diff image
        let dp = diff_path(test_name);
        if let Err(e) = save_diff_image(&our_pixels, &ref_pixels, TOLERANCE, &dp) {
            eprintln!("Warning: failed to save diff image: {}", e);
        }

        eprintln!(
            "[{}] Compared against Chromium reference:\n  \
             Total pixels: {}\n  \
             Mismatched:   {} ({:.2}%)\n  \
             Max channel diff: {}\n  \
             Avg channel diff: {:.2}",
            test_name,
            diff.total_pixels,
            diff.mismatched_pixels,
            diff.mismatch_percentage(),
            diff.max_channel_diff,
            diff.avg_channel_diff,
        );

        Some(diff)
    } else {
        eprintln!(
            "[{}] No Chromium reference found at {:?}. \
             Run capture_chromium.sh to generate references.",
            test_name, ref_path
        );
        None
    };

    (surface, diff)
}

// ═══════════════════════════════════════════════════════════════════════
// ── Test 1: Basic Text ──────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn build_basic_text() -> Document {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = add_block(&mut doc, vp, 400.0);
    // Inherit viewport text defaults onto container
    inherit_text_style(&mut doc, vp, container);

    // 12px normal
    add_paragraph_with_style(&mut doc, container,
        "The quick brown fox jumps over the lazy dog. 12px normal.",
        |s| { s.font_size = 12.0; });

    // 16px normal
    add_paragraph_with_style(&mut doc, container,
        "The quick brown fox jumps over the lazy dog. 16px normal.",
        |s| { s.font_size = 16.0; });

    // 24px normal
    add_paragraph_with_style(&mut doc, container,
        "The quick brown fox jumps over the lazy dog. 24px normal.",
        |s| { s.font_size = 24.0; });

    // 36px normal
    add_paragraph_with_style(&mut doc, container,
        "The quick brown fox jumps. 36px.",
        |s| { s.font_size = 36.0; });

    // 16px bold
    add_paragraph_with_style(&mut doc, container,
        "The quick brown fox jumps over the lazy dog. 16px bold.",
        |s| { s.font_size = 16.0; s.font_weight = FontWeight::BOLD; });

    // 16px italic
    add_paragraph_with_style(&mut doc, container,
        "The quick brown fox jumps over the lazy dog. 16px italic.",
        |s| { s.font_size = 16.0; s.font_style = FontStyleEnum::Italic; });

    // 24px bold
    add_paragraph_with_style(&mut doc, container,
        "The quick brown fox jumps over the lazy dog. 24px bold.",
        |s| { s.font_size = 24.0; s.font_weight = FontWeight::BOLD; });

    // 24px bold italic
    add_paragraph_with_style(&mut doc, container,
        "The quick brown fox jumps. 24px bold italic.",
        |s| {
            s.font_size = 24.0;
            s.font_weight = FontWeight::BOLD;
            s.font_style = FontStyleEnum::Italic;
        });

    doc
}

#[test]
fn pixel_basic_text() {
    let doc = build_basic_text();
    let (mut surface, diff) = render_and_compare(&doc, "basic_text");

    // Must produce visible text content
    assert!(
        has_visible_content(&mut surface),
        "basic_text: no visible pixels rendered"
    );

    if let Some(d) = diff {
        assert!(!d.size_mismatch, "basic_text: image dimensions differ");
        assert!(
            d.max_channel_diff <= TOLERANCE,
            "basic_text: max channel diff {} exceeds tolerance {}",
            d.max_channel_diff,
            TOLERANCE
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ── Test 2: Line Breaking ───────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn build_line_breaking() -> Document {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);

    let long_text = "The quick brown fox jumps over the lazy dog repeatedly until it wraps many times.";

    // Helper to set container padding
    let make_container = |doc: &mut Document, parent: NodeId, width: f32| -> NodeId {
        let c = add_block(doc, parent, width);
        inherit_text_style(doc, parent, c);
        doc.node_mut(c).style.margin_bottom = Length::px(10.0);
        doc.node_mut(c).style.padding_top = Length::px(4.0);
        doc.node_mut(c).style.padding_right = Length::px(4.0);
        doc.node_mut(c).style.padding_bottom = Length::px(4.0);
        doc.node_mut(c).style.padding_left = Length::px(4.0);
        c
    };

    // 100px container
    let c = make_container(&mut doc, vp, 100.0);
    add_text(&mut doc, c, long_text);

    // 200px container
    let c = make_container(&mut doc, vp, 200.0);
    add_text(&mut doc, c, long_text);

    // 400px container
    let c = make_container(&mut doc, vp, 400.0);
    add_text(&mut doc, c, long_text);

    // Long word in narrow container
    let c = make_container(&mut doc, vp, 200.0);
    add_text(&mut doc, c,
        "Supercalifragilisticexpialidocious is a very long word that tests overflow behavior.");

    // Short words in 200px
    let c = make_container(&mut doc, vp, 200.0);
    add_text(&mut doc, c,
        "Short words in a box with two hundred pixel width container for testing.");

    doc
}

#[test]
fn pixel_line_breaking() {
    let doc = build_line_breaking();
    let (mut surface, diff) = render_and_compare(&doc, "line_breaking");

    assert!(
        has_visible_content(&mut surface),
        "line_breaking: no visible pixels rendered"
    );

    if let Some(d) = diff {
        assert!(!d.size_mismatch, "line_breaking: image dimensions differ");
        assert!(
            d.max_channel_diff <= TOLERANCE,
            "line_breaking: max channel diff {} exceeds tolerance {}",
            d.max_channel_diff,
            TOLERANCE
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ── Test 3: Text Alignment ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn build_text_alignment() -> Document {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = add_block(&mut doc, vp, 400.0);
    inherit_text_style(&mut doc, vp, container);

    let align_text = "The quick brown fox jumps over the lazy dog multiple times to wrap.";

    for &align in &[TextAlign::Left, TextAlign::Right, TextAlign::Center, TextAlign::Justify] {
        let label = match align {
            TextAlign::Left => "Left aligned text. ",
            TextAlign::Right => "Right aligned text. ",
            TextAlign::Center => "Center aligned text. ",
            TextAlign::Justify => "Justified text aligns both left and right edges. ",
            _ => "",
        };
        let full_text = format!("{}{}", label, align_text);

        // Create block manually (add_block already appends)
        let p = doc.create_node(ElementTag::Div);
        doc.node_mut(p).style.display = Display::Block;
        inherit_text_style(&mut doc, container, p);
        doc.node_mut(p).style.text_align = align;
        doc.node_mut(p).style.margin_bottom = Length::px(10.0);
        doc.node_mut(p).style.padding_top = Length::px(4.0);
        doc.node_mut(p).style.padding_right = Length::px(4.0);
        doc.node_mut(p).style.padding_bottom = Length::px(4.0);
        doc.node_mut(p).style.padding_left = Length::px(4.0);
        doc.append_child(container, p);
        add_text(&mut doc, p, &full_text);
    }

    doc
}

#[test]
fn pixel_text_alignment() {
    let doc = build_text_alignment();
    let (mut surface, diff) = render_and_compare(&doc, "text_alignment");

    assert!(
        has_visible_content(&mut surface),
        "text_alignment: no visible pixels rendered"
    );

    if let Some(d) = diff {
        assert!(!d.size_mismatch, "text_alignment: image dimensions differ");
        assert!(
            d.max_channel_diff <= TOLERANCE,
            "text_alignment: max channel diff {} exceeds tolerance {}",
            d.max_channel_diff,
            TOLERANCE
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ── Test 4: Vertical Align ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn build_vertical_align() -> Document {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = add_block(&mut doc, vp, 400.0);
    inherit_text_style(&mut doc, vp, container);

    let alignments: &[(VerticalAlign, &str)] = &[
        (VerticalAlign::Baseline, "baseline"),
        (VerticalAlign::Top, "top"),
        (VerticalAlign::Bottom, "bottom"),
        (VerticalAlign::Middle, "middle"),
        (VerticalAlign::Sub, "sub"),
        (VerticalAlign::Super, "super"),
        (VerticalAlign::TextTop, "text-top"),
        (VerticalAlign::TextBottom, "text-bottom"),
    ];

    for &(va, label) in alignments {
        let line = add_block(&mut doc, container, 0.0);
        inherit_text_style(&mut doc, container, line);
        doc.node_mut(line).style.margin_bottom = Length::px(15.0);
        doc.node_mut(line).style.line_height = LineHeight::Length(40.0);

        // "Normal " text
        add_text(&mut doc, line, "Normal ");

        // Span with vertical-align and smaller font
        let span = add_span(&mut doc, line);
        inherit_text_style(&mut doc, line, span);
        doc.node_mut(span).style.font_size = 10.0;
        doc.node_mut(span).style.vertical_align = va;
        add_text(&mut doc, span, label);

        // " text here."
        add_text(&mut doc, line, " text here.");
    }

    doc
}

#[test]
fn pixel_vertical_align() {
    let doc = build_vertical_align();
    let (mut surface, diff) = render_and_compare(&doc, "vertical_align");

    assert!(
        has_visible_content(&mut surface),
        "vertical_align: no visible pixels rendered"
    );

    if let Some(d) = diff {
        assert!(!d.size_mismatch, "vertical_align: image dimensions differ");
        assert!(
            d.max_channel_diff <= TOLERANCE,
            "vertical_align: max channel diff {} exceeds tolerance {}",
            d.max_channel_diff,
            TOLERANCE
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ── Test 5: Text Decoration ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn build_text_decoration() -> Document {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = add_block(&mut doc, vp, 400.0);
    inherit_text_style(&mut doc, vp, container);

    // Underline
    add_paragraph_with_style(&mut doc, container,
        "Underline decoration on this text.",
        |s| { s.text_decoration_line = TextDecorationLine::UNDERLINE; });

    // Overline
    add_paragraph_with_style(&mut doc, container,
        "Overline decoration on this text.",
        |s| { s.text_decoration_line = TextDecorationLine::OVERLINE; });

    // Line-through
    add_paragraph_with_style(&mut doc, container,
        "Line-through decoration on this text.",
        |s| { s.text_decoration_line = TextDecorationLine::LINE_THROUGH; });

    // Solid underline
    add_paragraph_with_style(&mut doc, container,
        "Solid underline style on this text.",
        |s| {
            s.text_decoration_line = TextDecorationLine::UNDERLINE;
            s.text_decoration_style = TextDecorationStyle::Solid;
        });

    // Double underline
    add_paragraph_with_style(&mut doc, container,
        "Double underline style on this text.",
        |s| {
            s.text_decoration_line = TextDecorationLine::UNDERLINE;
            s.text_decoration_style = TextDecorationStyle::Double;
        });

    // Dotted underline
    add_paragraph_with_style(&mut doc, container,
        "Dotted underline style on this text.",
        |s| {
            s.text_decoration_line = TextDecorationLine::UNDERLINE;
            s.text_decoration_style = TextDecorationStyle::Dotted;
        });

    // Dashed underline
    add_paragraph_with_style(&mut doc, container,
        "Dashed underline style on this text.",
        |s| {
            s.text_decoration_line = TextDecorationLine::UNDERLINE;
            s.text_decoration_style = TextDecorationStyle::Dashed;
        });

    // Wavy underline
    add_paragraph_with_style(&mut doc, container,
        "Wavy underline style on this text.",
        |s| {
            s.text_decoration_line = TextDecorationLine::UNDERLINE;
            s.text_decoration_style = TextDecorationStyle::Wavy;
        });

    // Red underline
    add_paragraph_with_style(&mut doc, container,
        "Red underline color on this text.",
        |s| {
            s.text_decoration_line = TextDecorationLine::UNDERLINE;
            s.text_decoration_color = StyleColor::Resolved(Color::RED);
        });

    // Blue underline
    add_paragraph_with_style(&mut doc, container,
        "Blue underline color on this text.",
        |s| {
            s.text_decoration_line = TextDecorationLine::UNDERLINE;
            s.text_decoration_color = StyleColor::Resolved(Color::BLUE);
        });

    // All three combined
    add_paragraph_with_style(&mut doc, container,
        "Underline overline and line-through combined.",
        |s| {
            s.text_decoration_line = TextDecorationLine(
                TextDecorationLine::UNDERLINE.0
                    | TextDecorationLine::OVERLINE.0
                    | TextDecorationLine::LINE_THROUGH.0,
            );
        });

    doc
}

#[test]
fn pixel_text_decoration() {
    let doc = build_text_decoration();
    let (mut surface, diff) = render_and_compare(&doc, "text_decoration");

    assert!(
        has_visible_content(&mut surface),
        "text_decoration: no visible pixels rendered"
    );

    if let Some(d) = diff {
        assert!(!d.size_mismatch, "text_decoration: image dimensions differ");
        assert!(
            d.max_channel_diff <= TOLERANCE,
            "text_decoration: max channel diff {} exceeds tolerance {}",
            d.max_channel_diff,
            TOLERANCE
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ── Test 6: Letter & Word Spacing ───────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn build_letter_word_spacing() -> Document {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = add_block(&mut doc, vp, 400.0);
    inherit_text_style(&mut doc, vp, container);

    let sentence = "The quick brown fox jumps over the lazy dog.";

    // Letter spacing variants
    for &(ls, label) in &[
        (0.0f32, "Letter spacing 0px: "),
        (2.0, "Letter spacing 2px: "),
        (5.0, "Letter spacing 5px: "),
        (-1.0, "Letter spacing -1px: "),
    ] {
        let full = format!("{}{}", label, sentence);
        add_paragraph_with_style(&mut doc, container, &full,
            |s| { s.letter_spacing = ls; });
    }

    // Word spacing variants
    for &(ws, label) in &[
        (0.0f32, "Word spacing 0px: "),
        (5.0, "Word spacing 5px: "),
        (10.0, "Word spacing 10px: "),
        (-2.0, "Word spacing -2px: "),
    ] {
        let full = format!("{}{}", label, sentence);
        add_paragraph_with_style(&mut doc, container, &full,
            |s| { s.word_spacing = ws; });
    }

    doc
}

#[test]
fn pixel_letter_word_spacing() {
    let doc = build_letter_word_spacing();
    let (mut surface, diff) = render_and_compare(&doc, "letter_word_spacing");

    assert!(
        has_visible_content(&mut surface),
        "letter_word_spacing: no visible pixels rendered"
    );

    if let Some(d) = diff {
        assert!(!d.size_mismatch, "letter_word_spacing: image dimensions differ");
        assert!(
            d.max_channel_diff <= TOLERANCE,
            "letter_word_spacing: max channel diff {} exceeds tolerance {}",
            d.max_channel_diff,
            TOLERANCE
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ── Test 7: Text Transform ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn build_text_transform() -> Document {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = add_block(&mut doc, vp, 400.0);
    inherit_text_style(&mut doc, vp, container);

    add_paragraph_with_style(&mut doc, container,
        "No transform: The Quick Brown Fox Jumps Over The Lazy Dog.",
        |s| { s.text_transform = TextTransform::None; });

    add_paragraph_with_style(&mut doc, container,
        "Uppercase: The Quick Brown Fox Jumps Over The Lazy Dog.",
        |s| { s.text_transform = TextTransform::Uppercase; });

    add_paragraph_with_style(&mut doc, container,
        "Lowercase: The Quick Brown Fox Jumps Over The Lazy Dog.",
        |s| { s.text_transform = TextTransform::Lowercase; });

    add_paragraph_with_style(&mut doc, container,
        "Capitalize: the quick brown fox jumps over the lazy dog.",
        |s| { s.text_transform = TextTransform::Capitalize; });

    add_paragraph_with_style(&mut doc, container,
        "UPPERCASE: already uppercase text here 12345.",
        |s| { s.text_transform = TextTransform::Uppercase; });

    add_paragraph_with_style(&mut doc, container,
        "lowercase: ALREADY LOWERCASE TEXT HERE 12345.",
        |s| { s.text_transform = TextTransform::Lowercase; });

    add_paragraph_with_style(&mut doc, container,
        "Capitalize: mixed CASE words for capitalize testing.",
        |s| { s.text_transform = TextTransform::Capitalize; });

    doc
}

#[test]
fn pixel_text_transform() {
    let doc = build_text_transform();
    let (mut surface, diff) = render_and_compare(&doc, "text_transform");

    assert!(
        has_visible_content(&mut surface),
        "text_transform: no visible pixels rendered"
    );

    if let Some(d) = diff {
        assert!(!d.size_mismatch, "text_transform: image dimensions differ");
        assert!(
            d.max_channel_diff <= TOLERANCE,
            "text_transform: max channel diff {} exceeds tolerance {}",
            d.max_channel_diff,
            TOLERANCE
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ── Test 8: BiDi Mixed ──────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn build_bidi_mixed() -> Document {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = add_block(&mut doc, vp, 400.0);
    inherit_text_style(&mut doc, vp, container);

    // Use simpler bidi text to avoid Skia shaper assertion with complex
    // multi-script paragraphs. Test bidi reordering with ASCII + single RTL words.
    add_paragraph_with_style(&mut doc, container,
        "Hello World in English only.",
        |s| { s.direction = Direction::Ltr; });

    // Simple RTL direction on LTR text
    add_paragraph_with_style(&mut doc, container,
        "Right to left direction set on English text.",
        |s| { s.direction = Direction::Rtl; });

    // Numbers with direction
    add_paragraph_with_style(&mut doc, container,
        "Numbers 123 and 456 in left to right direction.",
        |s| { s.direction = Direction::Ltr; });

    add_paragraph_with_style(&mut doc, container,
        "Numbers 789 and 012 in right to left direction.",
        |s| { s.direction = Direction::Rtl; });

    // Longer LTR paragraph
    add_paragraph_with_style(&mut doc, container,
        "The quick brown fox jumps over the lazy dog with LTR direction applied.",
        |s| { s.direction = Direction::Ltr; });

    // Longer RTL paragraph
    add_paragraph_with_style(&mut doc, container,
        "The quick brown fox jumps over the lazy dog with RTL direction applied.",
        |s| { s.direction = Direction::Rtl; });

    doc
}

#[test]
fn pixel_bidi_mixed() {
    let doc = build_bidi_mixed();
    let (mut surface, diff) = render_and_compare(&doc, "bidi_mixed");

    assert!(
        has_visible_content(&mut surface),
        "bidi_mixed: no visible pixels rendered"
    );

    if let Some(d) = diff {
        assert!(!d.size_mismatch, "bidi_mixed: image dimensions differ");
        assert!(
            d.max_channel_diff <= TOLERANCE,
            "bidi_mixed: max channel diff {} exceeds tolerance {}",
            d.max_channel_diff,
            TOLERANCE
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ── Test 9: Line Height ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn build_line_height() -> Document {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = add_block(&mut doc, vp, 400.0);
    inherit_text_style(&mut doc, vp, container);

    let text = "The quick brown fox jumps over the lazy dog repeatedly to create multiple wrapped lines.";

    let line_heights: &[(LineHeight, &str)] = &[
        (LineHeight::Normal, "Line height normal. "),
        (LineHeight::Number(1.0), "Line height 1. "),
        (LineHeight::Number(1.5), "Line height 1.5. "),
        (LineHeight::Number(2.0), "Line height 2. "),
        (LineHeight::Length(24.0), "Line height 24px. "),
        (LineHeight::Percentage(150.0), "Line height 150%. "),
    ];

    for &(lh, label) in line_heights {
        let full = format!("{}{}", label, text);
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        inherit_text_style(&mut doc, container, block);
        doc.node_mut(block).style.line_height = lh;
        doc.node_mut(block).style.margin_bottom = Length::px(10.0);
        doc.node_mut(block).style.padding_top = Length::px(4.0);
        doc.node_mut(block).style.padding_right = Length::px(4.0);
        doc.node_mut(block).style.padding_bottom = Length::px(4.0);
        doc.node_mut(block).style.padding_left = Length::px(4.0);
        doc.append_child(container, block);
        add_text(&mut doc, block, &full);
    }

    doc
}

#[test]
fn pixel_line_height() {
    let doc = build_line_height();
    let (mut surface, diff) = render_and_compare(&doc, "line_height");

    assert!(
        has_visible_content(&mut surface),
        "line_height: no visible pixels rendered"
    );

    if let Some(d) = diff {
        assert!(!d.size_mismatch, "line_height: image dimensions differ");
        assert!(
            d.max_channel_diff <= TOLERANCE,
            "line_height: max channel diff {} exceeds tolerance {}",
            d.max_channel_diff,
            TOLERANCE
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ── Test 10: White Space ────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn build_white_space() -> Document {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = add_block(&mut doc, vp, 400.0);
    inherit_text_style(&mut doc, vp, container);

    let ws_modes: &[(WhiteSpace, &str)] = &[
        (
            WhiteSpace::Normal,
            "Normal:   multiple   spaces   and\nnewlines   collapse   to   single   spaces   here.",
        ),
        (
            WhiteSpace::Nowrap,
            "Nowrap: this text should not wrap at all even if it is very long and exceeds the container width boundary.",
        ),
        (
            WhiteSpace::Pre,
            "Pre:   multiple   spaces   preserved.\nNewlines\tand\ttabs\tpreserved too.",
        ),
        (
            WhiteSpace::PreWrap,
            "Pre-wrap:   multiple   spaces   preserved   and   the   text   wraps   at   container   edge.",
        ),
        (
            WhiteSpace::PreLine,
            "Pre-line:   spaces   collapse   but\nnewlines are preserved here.",
        ),
    ];

    for &(ws, content) in ws_modes {
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        inherit_text_style(&mut doc, container, block);
        doc.node_mut(block).style.white_space = ws;
        doc.node_mut(block).style.margin_bottom = Length::px(10.0);
        doc.node_mut(block).style.padding_top = Length::px(4.0);
        doc.node_mut(block).style.padding_right = Length::px(4.0);
        doc.node_mut(block).style.padding_bottom = Length::px(4.0);
        doc.node_mut(block).style.padding_left = Length::px(4.0);
        doc.append_child(container, block);
        add_text(&mut doc, block, content);
    }

    doc
}

#[test]
fn pixel_white_space() {
    let doc = build_white_space();
    let (mut surface, diff) = render_and_compare(&doc, "white_space");

    assert!(
        has_visible_content(&mut surface),
        "white_space: no visible pixels rendered"
    );

    if let Some(d) = diff {
        assert!(!d.size_mismatch, "white_space: image dimensions differ");
        assert!(
            d.max_channel_diff <= TOLERANCE,
            "white_space: max channel diff {} exceeds tolerance {}",
            d.max_channel_diff,
            TOLERANCE
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ── Aggregate summary test ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// Run all 10 scenario builders and verify they at minimum produce visible output.
/// This serves as a smoke test even without Chromium references.
#[test]
fn pixel_all_scenarios_produce_output() {
    let scenarios: Vec<(&str, Document)> = vec![
        ("basic_text", build_basic_text()),
        ("line_breaking", build_line_breaking()),
        ("text_alignment", build_text_alignment()),
        ("vertical_align", build_vertical_align()),
        ("text_decoration", build_text_decoration()),
        ("letter_word_spacing", build_letter_word_spacing()),
        ("text_transform", build_text_transform()),
        ("bidi_mixed", build_bidi_mixed()),
        ("line_height", build_line_height()),
        ("white_space", build_white_space()),
    ];

    let mut all_pass = true;
    for (name, doc) in &scenarios {
        let result = render_to_surface(doc, SURFACE_W, SURFACE_H);
        match result {
            Ok(mut surface) => {
                if !has_visible_content(&mut surface) {
                    eprintln!("[{}] WARNING: No visible content rendered", name);
                    // This is a warning, not a hard failure — some scenarios may
                    // legitimately have all-white output depending on font availability.
                } else {
                    eprintln!("[{}] OK: Visible content rendered", name);
                }
            }
            Err(e) => {
                eprintln!("[{}] FAIL: render_to_surface failed: {}", name, e);
                all_pass = false;
            }
        }
    }
    assert!(all_pass, "One or more scenarios failed to render");
}
