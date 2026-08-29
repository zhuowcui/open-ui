//! Deterministic decoding for document-owned CSS image resources.

use std::cell::RefCell;
use std::collections::HashMap;

use openui_dom::{Document, EncodedImageResource};
use openui_style::ImageResourceId;
use skia_safe::image::CachingHint;
use skia_safe::{surfaces, Canvas, ClipOp, Data, Image, ImageInfo, Paint, PaintStyle, Rect};

thread_local! {
    static IMAGE_CACHE: RefCell<HashMap<String, Image>> = RefCell::new(HashMap::new());
    static QUANTIZED_REPEAT_CACHE: RefCell<HashMap<(String, i32, i32), Image>> =
        RefCell::new(HashMap::new());
    static SVG_CACHE: RefCell<HashMap<String, StaticSvg>> = RefCell::new(HashMap::new());
}

#[derive(Clone, Debug)]
struct StaticSvg {
    width: Option<f32>,
    height: Option<f32>,
    view_box: Option<Rect>,
    shapes: Vec<StaticSvgShape>,
}

#[derive(Clone, Debug)]
struct StaticSvgShape {
    rect: Rect,
    color: skia_safe::Color,
}

/// Decode one registered PNG or limited static SVG resource.
///
/// Cache identity uses the frozen SHA-256 supplied by the asset manifest, so
/// repeated uses across nodes and documents share identical decoded pixels.
pub fn decode_image_resource(doc: &Document, id: ImageResourceId) -> Result<Image, String> {
    let resource = doc
        .image_resource(id)
        .ok_or_else(|| format!("missing image resource {}", id.0))?;
    if let Some(image) = IMAGE_CACHE.with(|cache| cache.borrow().get(&resource.sha256).cloned()) {
        return Ok(image);
    }

    let image = match resource.mime_type.as_str() {
        "image/png" => Image::from_encoded(Data::new_copy(&resource.bytes))
            .ok_or_else(|| format!("failed to decode PNG resource {}", resource.source))?,
        "image/svg+xml" => {
            let svg = parsed_svg_resource(resource)?;
            rasterize_static_svg(&svg, &resource.source, None)?
        }
        mime => {
            return Err(format!(
                "unsupported image MIME type {mime} for {}",
                resource.source
            ))
        }
    };
    IMAGE_CACHE.with(|cache| {
        cache
            .borrow_mut()
            .insert(resource.sha256.clone(), image.clone());
    });
    Ok(image)
}

/// Decode an SVG at its CSS concrete object size. Raster resources retain
/// their intrinsic pixels; vector border images are rasterized for the border
/// image area before numeric slices are applied.
pub fn decode_image_resource_at_size(
    doc: &Document,
    id: ImageResourceId,
    width: i32,
    height: i32,
) -> Result<Image, String> {
    let resource = doc
        .image_resource(id)
        .ok_or_else(|| format!("missing image resource {}", id.0))?;
    if resource.mime_type != "image/svg+xml" {
        return decode_image_resource(doc, id);
    }
    if svg_has_explicit_dimension(&resource.bytes) {
        return decode_image_resource(doc, id);
    }
    let width = width.max(1);
    let height = height.max(1);
    let key = format!("{}@{}x{}", resource.sha256, width, height);
    if let Some(image) = IMAGE_CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
        return Ok(image);
    }
    let svg = parsed_svg_resource(resource)?;
    let image = rasterize_static_svg(&svg, &resource.source, Some((width, height)))?;
    IMAGE_CACHE.with(|cache| {
        cache.borrow_mut().insert(key, image.clone());
    });
    Ok(image)
}

/// Paint a limited static SVG directly into one destination patch.
///
/// Vector border images are nine-sliced before rasterization. Rendering the
/// complete SVG to an intermediate bitmap would blend adjacent vector shapes
/// together before the slice transform and loses Chromium's independent edge
/// coverage at fractional destination boundaries.
pub fn paint_svg_resource_patch(
    canvas: &Canvas,
    doc: &Document,
    id: ImageResourceId,
    concrete_width: f32,
    concrete_height: f32,
    source: Rect,
    destination: Rect,
    opacity: f32,
) -> Result<(), String> {
    let resource = doc
        .image_resource(id)
        .ok_or_else(|| format!("missing image resource {}", id.0))?;
    if resource.mime_type != "image/svg+xml" {
        return Err(format!("resource {} is not an SVG", id.0));
    }
    if source.width() <= 0.0
        || source.height() <= 0.0
        || destination.width() <= 0.0
        || destination.height() <= 0.0
    {
        return Ok(());
    }

    let svg = parsed_svg_resource(resource)?;

    canvas.save();
    canvas.clip_rect(destination, ClipOp::Intersect, false);
    if opacity < 1.0 {
        canvas.save_layer_alpha_f(destination, opacity);
    }
    canvas.translate((destination.left, destination.top));
    canvas.scale((
        destination.width() / source.width(),
        destination.height() / source.height(),
    ));
    canvas.translate((-source.left, -source.top));
    paint_static_svg(canvas, &svg, concrete_width, concrete_height);
    if opacity < 1.0 {
        canvas.restore();
    }
    canvas.restore();
    Ok(())
}

/// Resize a repeated raster image with Chromium's 4-bit bilinear phase.
///
/// Skia's raster backend otherwise uses full-precision weights, while Chromium
/// floors each axis' phase to sixteenths in the two-axis repeated bitmap path.
/// Interpolating premultiplied native pixels preserves transparent-edge behavior.
pub fn quantized_repeated_image(
    doc: &Document,
    id: ImageResourceId,
    width: i32,
    height: i32,
) -> Result<Image, String> {
    if width <= 0 || height <= 0 {
        return Err("invalid repeated image dimensions".to_string());
    }
    let resource = doc
        .image_resource(id)
        .ok_or_else(|| format!("missing image resource {}", id.0))?;
    let key = (resource.sha256.clone(), width, height);
    if let Some(image) = QUANTIZED_REPEAT_CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
        return Ok(image);
    }

    let source = decode_image_resource(doc, id)?;
    let source_width = source.width();
    let source_height = source.height();
    let source_info = ImageInfo::new_n32_premul((source_width, source_height), None);
    let source_row_bytes = source_width as usize * 4;
    let mut source_pixels = vec![0_u8; source_row_bytes * source_height as usize];
    if !source.read_pixels(
        &source_info,
        &mut source_pixels,
        source_row_bytes,
        (0, 0),
        CachingHint::Allow,
    ) {
        return Err(format!("failed to read image resource {}", id.0));
    }

    let target_info = ImageInfo::new_n32_premul((width, height), None);
    let target_row_bytes = width as usize * 4;
    let mut target_pixels = vec![0_u8; target_row_bytes * height as usize];
    for target_y in 0..height {
        let source_y = (target_y as f32 + 0.5) * source_height as f32 / height as f32 - 0.5;
        let y0 = source_y.floor() as i32;
        let fy = ((source_y - y0 as f32) * 16.0).floor() / 16.0;
        for target_x in 0..width {
            let source_x = (target_x as f32 + 0.5) * source_width as f32 / width as f32 - 0.5;
            let x0 = source_x.floor() as i32;
            let fx = ((source_x - x0 as f32) * 16.0).floor() / 16.0;
            let x1 = (x0 + 1).rem_euclid(source_width) as usize;
            let x0 = x0.rem_euclid(source_width) as usize;
            let y1 = (y0 + 1).rem_euclid(source_height) as usize;
            let y0 = y0.rem_euclid(source_height) as usize;
            for channel in 0..4 {
                let at = |x: usize, y: usize| {
                    source_pixels[y * source_row_bytes + x * 4 + channel] as f32
                };
                let top = at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx;
                let bottom = at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx;
                target_pixels
                    [target_y as usize * target_row_bytes + target_x as usize * 4 + channel] =
                    (top * (1.0 - fy) + bottom * fy).round() as u8;
            }
        }
    }

    let mut surface = surfaces::raster_n32_premul((width, height))
        .ok_or_else(|| "failed to allocate repeated image surface".to_string())?;
    if !surface
        .canvas()
        .write_pixels(&target_info, &target_pixels, target_row_bytes, (0, 0))
    {
        return Err("failed to write repeated image pixels".to_string());
    }
    let image = surface.image_snapshot();
    QUANTIZED_REPEAT_CACHE.with(|cache| {
        cache.borrow_mut().insert(key, image.clone());
    });
    Ok(image)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImagePatchPhase {
    /// A CSS image tile is sampled in tile-local space. Downscaled, non-repeated
    /// tiles close their fixed-point phase against the far edge, while each
    /// repeated axis samples across the opposite source edge.
    Tile {
        close_downscale: bool,
        wrap_x: bool,
        wrap_y: bool,
    },
    /// Nine-slice patches retain the absolute device origin used by Skia's
    /// inverse image matrix.
    Device,
}

/// Resize a strict source patch with Chromium's four-bit bilinear phase.
pub fn quantized_image_patch(
    image: &Image,
    source: Rect,
    destination: Rect,
    width: i32,
    height: i32,
    phase: ImagePatchPhase,
) -> Result<Image, String> {
    if width <= 0 || height <= 0 || source.width() <= 0.0 || source.height() <= 0.0 {
        return Err("invalid image patch dimensions".to_string());
    }
    let image_width = image.width();
    let image_height = image.height();
    let source_info = ImageInfo::new_n32_premul((image_width, image_height), None);
    let source_row_bytes = image_width as usize * 4;
    let mut source_pixels = vec![0_u8; source_row_bytes * image_height as usize];
    if !image.read_pixels(
        &source_info,
        &mut source_pixels,
        source_row_bytes,
        (0, 0),
        CachingHint::Allow,
    ) {
        return Err("failed to read image patch".to_string());
    }

    let min_x = source.left.floor().max(0.0) as i32;
    let max_x = (source.right.ceil() as i32 - 1).min(image_width - 1);
    let min_y = source.top.floor().max(0.0) as i32;
    let max_y = (source.bottom.ceil() as i32 - 1).min(image_height - 1);
    let target_info = ImageInfo::new_n32_premul((width, height), None);
    let target_row_bytes = width as usize * 4;
    let mut target_pixels = vec![0_u8; target_row_bytes * height as usize];
    let forward_scale_y = destination.height() / source.height();
    let forward_scale_x = destination.width() / source.width();
    let inverse_scale_y = 1.0_f32 / forward_scale_y;
    let inverse_scale_x = 1.0_f32 / forward_scale_x;
    let inverse_translate_y = source.top - destination.top * inverse_scale_y;
    let inverse_translate_x = source.left - destination.left * inverse_scale_x;
    let direct_inverse_y = source.height() / destination.height();
    let direct_inverse_x = source.width() / destination.width();
    let device_bias_y = if inverse_scale_y.to_bits() == direct_inverse_y.to_bits() {
        0.0
    } else {
        1.0 / 65536.0
    };
    let device_bias_x = if inverse_scale_x.to_bits() == direct_inverse_x.to_bits() {
        0.0
    } else {
        1.0 / 65536.0
    };
    for target_y in 0..height {
        let endpoint_phase_y = if matches!(
            phase,
            ImagePatchPhase::Tile {
                close_downscale: true,
                ..
            }
        ) && source.height() > destination.height()
            && inverse_scale_y.to_bits() != direct_inverse_y.to_bits()
            && height > 1
        {
            (height - 1 - target_y) as f32 / (height - 1) as f32 / 16.0
        } else {
            0.0
        };
        let source_y = match phase {
            ImagePatchPhase::Tile { .. } => {
                source.top + (target_y as f32 + 0.5) * source.height() / height as f32 - 0.5
                    + endpoint_phase_y
            }
            ImagePatchPhase::Device => {
                let device_y = destination.top + target_y as f32 + 0.5 - device_bias_y;
                device_y * inverse_scale_y + inverse_translate_y - 0.5
            }
        };
        let raw_y0 = source_y.floor() as i32;
        let fy = ((source_y - raw_y0 as f32) * 16.0).floor() / 16.0;
        let wrap_y = matches!(phase, ImagePatchPhase::Tile { wrap_y: true, .. });
        let y0 = if wrap_y {
            raw_y0.rem_euclid(image_height) as usize
        } else {
            raw_y0.clamp(min_y, max_y) as usize
        };
        let y1 = if wrap_y {
            (raw_y0 + 1).rem_euclid(image_height) as usize
        } else {
            (raw_y0 + 1).clamp(min_y, max_y) as usize
        };
        for target_x in 0..width {
            let endpoint_phase_x = if matches!(
                phase,
                ImagePatchPhase::Tile {
                    close_downscale: true,
                    ..
                }
            ) && source.width() > destination.width()
                && inverse_scale_x.to_bits() != direct_inverse_x.to_bits()
                && width > 1
            {
                (width - 1 - target_x) as f32 / (width - 1) as f32 / 16.0
            } else {
                0.0
            };
            let source_x = match phase {
                ImagePatchPhase::Tile { .. } => {
                    source.left + (target_x as f32 + 0.5) * source.width() / width as f32 - 0.5
                        + endpoint_phase_x
                }
                ImagePatchPhase::Device => {
                    let device_x = destination.left + target_x as f32 + 0.5 - device_bias_x;
                    device_x * inverse_scale_x + inverse_translate_x - 0.5
                }
            };
            let raw_x0 = source_x.floor() as i32;
            let fx = ((source_x - raw_x0 as f32) * 16.0).floor() / 16.0;
            let wrap_x = matches!(phase, ImagePatchPhase::Tile { wrap_x: true, .. });
            let x0 = if wrap_x {
                raw_x0.rem_euclid(image_width) as usize
            } else {
                raw_x0.clamp(min_x, max_x) as usize
            };
            let x1 = if wrap_x {
                (raw_x0 + 1).rem_euclid(image_width) as usize
            } else {
                (raw_x0 + 1).clamp(min_x, max_x) as usize
            };
            for channel in 0..4 {
                let at = |x: usize, y: usize| {
                    source_pixels[y * source_row_bytes + x * 4 + channel] as f32
                };
                let top = at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx;
                let bottom = at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx;
                target_pixels
                    [target_y as usize * target_row_bytes + target_x as usize * 4 + channel] =
                    (top * (1.0 - fy) + bottom * fy).round() as u8;
            }
        }
    }

    let mut surface = surfaces::raster_n32_premul((width, height))
        .ok_or_else(|| "failed to allocate image patch surface".to_string())?;
    if !surface
        .canvas()
        .write_pixels(&target_info, &target_pixels, target_row_bytes, (0, 0))
    {
        return Err("failed to write image patch pixels".to_string());
    }
    Ok(surface.image_snapshot())
}

fn parsed_svg_resource(resource: &EncodedImageResource) -> Result<StaticSvg, String> {
    if let Some(svg) = SVG_CACHE.with(|cache| cache.borrow().get(&resource.sha256).cloned()) {
        return Ok(svg);
    }
    let svg = parse_static_svg(&resource.bytes).map_err(|reason| {
        format!(
            "failed to decode SVG resource {}: {reason}",
            resource.source
        )
    })?;
    SVG_CACHE.with(|cache| {
        cache
            .borrow_mut()
            .insert(resource.sha256.clone(), svg.clone());
    });
    Ok(svg)
}

fn rasterize_static_svg(
    svg: &StaticSvg,
    source: &str,
    concrete_size: Option<(i32, i32)>,
) -> Result<Image, String> {
    let (intrinsic_width, intrinsic_height) = static_svg_intrinsic_size(svg);
    let (width, height) = concrete_size.unwrap_or((
        intrinsic_width.ceil().max(1.0) as i32,
        intrinsic_height.ceil().max(1.0) as i32,
    ));
    let mut surface = surfaces::raster_n32_premul((width, height))
        .ok_or_else(|| format!("failed to allocate SVG surface for {source}"))?;
    surface.canvas().clear(skia_safe::Color::TRANSPARENT);
    paint_static_svg(surface.canvas(), svg, width as f32, height as f32);
    Ok(surface.image_snapshot())
}

fn static_svg_intrinsic_size(svg: &StaticSvg) -> (f32, f32) {
    let view_box_size = svg
        .view_box
        .map(|view_box| (view_box.width(), view_box.height()));
    match (svg.width, svg.height, view_box_size) {
        (Some(width), Some(height), _) => (width, height),
        (Some(width), None, Some((view_width, view_height))) => {
            (width, width * view_height / view_width)
        }
        (None, Some(height), Some((view_width, view_height))) => {
            (height * view_width / view_height, height)
        }
        (None, None, Some((view_width, view_height))) => {
            // CSS Images' default object size is 300x150. With only a natural
            // ratio, contain that ratio in the default rectangle.
            let scale = (300.0 / view_width).min(150.0 / view_height);
            (view_width * scale, view_height * scale)
        }
        (Some(width), None, None) => (width, 150.0),
        (None, Some(height), None) => (300.0, height),
        (None, None, None) => (300.0, 150.0),
    }
}

fn paint_static_svg(canvas: &Canvas, svg: &StaticSvg, viewport_width: f32, viewport_height: f32) {
    let user_viewport = svg.view_box.unwrap_or_else(|| {
        let (width, height) = static_svg_intrinsic_size(svg);
        Rect::from_xywh(0.0, 0.0, width, height)
    });
    let scale =
        (viewport_width / user_viewport.width()).min(viewport_height / user_viewport.height());
    let translate_x =
        (viewport_width - user_viewport.width() * scale) * 0.5 - user_viewport.left * scale;
    let translate_y =
        (viewport_height - user_viewport.height() * scale) * 0.5 - user_viewport.top * scale;

    canvas.save();
    canvas.translate((translate_x, translate_y));
    canvas.scale((scale, scale));
    for shape in &svg.shapes {
        let mut paint = Paint::default();
        paint.set_style(PaintStyle::Fill);
        paint.set_anti_alias(true);
        paint.set_color(shape.color);
        canvas.draw_rect(shape.rect, &paint);
    }
    canvas.restore();
}

fn parse_static_svg(bytes: &[u8]) -> Result<StaticSvg, String> {
    let text = sanitize_static_svg(bytes);
    let root_start = text.find("<svg").ok_or("missing svg root")?;
    let root_end = text[root_start..]
        .find('>')
        .map(|offset| root_start + offset)
        .ok_or("unterminated svg root")?;
    let root = &text[root_start + 1..root_end];
    let width = svg_attribute(root, "width").and_then(parse_svg_number);
    let height = svg_attribute(root, "height").and_then(parse_svg_number);
    let view_box = svg_attribute(root, "viewBox").and_then(parse_svg_view_box);

    let root_fill = svg_attribute(root, "fill")
        .and_then(parse_svg_color)
        .or(Some(skia_safe::Color::BLACK));
    let mut fill_stack = vec![root_fill];
    let mut shapes = Vec::new();
    let mut rest = &text[root_end + 1..];
    while let Some(open) = rest.find('<') {
        rest = &rest[open + 1..];
        if rest.starts_with("!--") {
            let Some(end) = rest.find("-->") else {
                break;
            };
            rest = &rest[end + 3..];
            continue;
        }
        let Some(close) = rest.find('>') else {
            break;
        };
        let tag = rest[..close].trim();
        rest = &rest[close + 1..];
        if tag.starts_with("/g") {
            if fill_stack.len() > 1 {
                fill_stack.pop();
            }
            continue;
        }
        if tag.starts_with('/') {
            continue;
        }
        let name_end = tag
            .find(|character: char| character.is_ascii_whitespace() || character == '/')
            .unwrap_or(tag.len());
        let name = &tag[..name_end];
        let inherited_fill = fill_stack.last().copied().flatten();
        let fill = svg_attribute(tag, "fill")
            .map(parse_svg_color)
            .unwrap_or(inherited_fill);
        match name {
            "g" => fill_stack.push(fill),
            "rect" => {
                let Some(color) = fill else {
                    continue;
                };
                let x = svg_attribute(tag, "x")
                    .and_then(parse_svg_number)
                    .unwrap_or(0.0);
                let y = svg_attribute(tag, "y")
                    .and_then(parse_svg_number)
                    .unwrap_or(0.0);
                let Some(width) = svg_attribute(tag, "width").and_then(parse_svg_number) else {
                    continue;
                };
                let Some(height) = svg_attribute(tag, "height").and_then(parse_svg_number) else {
                    continue;
                };
                if width > 0.0 && height > 0.0 {
                    shapes.push(StaticSvgShape {
                        rect: Rect::from_xywh(x, y, width, height),
                        color,
                    });
                }
            }
            "path" => {
                let (Some(color), Some(path)) = (fill, svg_attribute(tag, "d")) else {
                    continue;
                };
                shapes.extend(
                    parse_axis_aligned_svg_path(path)
                        .into_iter()
                        .map(|rect| StaticSvgShape { rect, color }),
                );
            }
            _ => {}
        }
    }
    if shapes.is_empty() {
        return Err("no supported static shapes".to_string());
    }
    Ok(StaticSvg {
        width,
        height,
        view_box,
        shapes,
    })
}

fn svg_attribute<'a>(tag: &'a str, requested: &str) -> Option<&'a str> {
    let bytes = tag.as_bytes();
    let mut index = tag
        .find(|character: char| character.is_ascii_whitespace())
        .unwrap_or(tag.len());
    while index < bytes.len() {
        while index < bytes.len() && (bytes[index].is_ascii_whitespace() || bytes[index] == b'/') {
            index += 1;
        }
        let name_start = index;
        while index < bytes.len()
            && (bytes[index].is_ascii_alphanumeric() || matches!(bytes[index], b'-' | b'_' | b':'))
        {
            index += 1;
        }
        if name_start == index {
            index += 1;
            continue;
        }
        let name = &tag[name_start..index];
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'=' {
            continue;
        }
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() || !matches!(bytes[index], b'\'' | b'"') {
            continue;
        }
        let quote = bytes[index];
        index += 1;
        let value_start = index;
        while index < bytes.len() && bytes[index] != quote {
            index += 1;
        }
        let value = &tag[value_start..index.min(bytes.len())];
        if index < bytes.len() {
            index += 1;
        }
        if name == requested {
            return Some(value);
        }
    }
    None
}

fn parse_svg_number(value: &str) -> Option<f32> {
    value
        .trim()
        .strip_suffix("px")
        .unwrap_or(value.trim())
        .parse()
        .ok()
}

fn parse_svg_view_box(value: &str) -> Option<Rect> {
    let numbers: Vec<f32> = value
        .split(|character: char| character.is_ascii_whitespace() || character == ',')
        .filter(|part| !part.is_empty())
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    if numbers.len() == 4 && numbers[2] > 0.0 && numbers[3] > 0.0 {
        Some(Rect::from_xywh(
            numbers[0], numbers[1], numbers[2], numbers[3],
        ))
    } else {
        None
    }
}

fn parse_svg_color(value: &str) -> Option<skia_safe::Color> {
    let rgb = match value.trim().to_ascii_lowercase().as_str() {
        "none" => return None,
        "black" => (0, 0, 0),
        "blue" => (0, 0, 255),
        "aqua" => (0, 255, 255),
        "fuchsia" => (255, 0, 255),
        "green" => (0, 128, 0),
        "orange" => (255, 165, 0),
        "purple" => (128, 0, 128),
        "red" => (255, 0, 0),
        "teal" => (0, 128, 128),
        "white" => (255, 255, 255),
        "yellow" => (255, 255, 0),
        _ => return None,
    };
    Some(skia_safe::Color::from_argb(255, rgb.0, rgb.1, rgb.2))
}

#[derive(Clone, Copy)]
enum SvgPathToken {
    Command(u8),
    Number(f32),
}

fn tokenize_svg_path(path: &str) -> Vec<SvgPathToken> {
    let bytes = path.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index].is_ascii_alphabetic() {
            tokens.push(SvgPathToken::Command(bytes[index]));
            index += 1;
            continue;
        }
        if bytes[index].is_ascii_whitespace() || bytes[index] == b',' {
            index += 1;
            continue;
        }
        let start = index;
        if matches!(bytes[index], b'+' | b'-') {
            index += 1;
        }
        while index < bytes.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'.') {
            index += 1;
        }
        if index < bytes.len() && matches!(bytes[index], b'e' | b'E') {
            index += 1;
            if index < bytes.len() && matches!(bytes[index], b'+' | b'-') {
                index += 1;
            }
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
        }
        if start == index {
            index += 1;
        } else if let Ok(number) = path[start..index].parse() {
            tokens.push(SvgPathToken::Number(number));
        }
    }
    tokens
}

fn parse_axis_aligned_svg_path(path: &str) -> Vec<Rect> {
    let tokens = tokenize_svg_path(path);
    let mut rectangles = Vec::new();
    let mut index = 0;
    let mut command = b'M';
    let mut current = (0.0_f32, 0.0_f32);
    let mut start = current;
    let mut points: Vec<(f32, f32)> = Vec::new();
    let number_at = |index: usize| match tokens.get(index) {
        Some(SvgPathToken::Number(value)) => Some(*value),
        _ => None,
    };
    while index < tokens.len() {
        if let SvgPathToken::Command(next) = tokens[index] {
            command = next;
            index += 1;
            if matches!(command, b'Z' | b'z') {
                if points.len() >= 3 {
                    let min_x = points
                        .iter()
                        .map(|point| point.0)
                        .fold(f32::INFINITY, f32::min);
                    let max_x = points
                        .iter()
                        .map(|point| point.0)
                        .fold(f32::NEG_INFINITY, f32::max);
                    let min_y = points
                        .iter()
                        .map(|point| point.1)
                        .fold(f32::INFINITY, f32::min);
                    let max_y = points
                        .iter()
                        .map(|point| point.1)
                        .fold(f32::NEG_INFINITY, f32::max);
                    if max_x > min_x && max_y > min_y {
                        rectangles.push(Rect::from_ltrb(min_x, min_y, max_x, max_y));
                    }
                }
                current = start;
                points.clear();
                continue;
            }
        }
        match command {
            b'M' | b'm' | b'L' | b'l' => {
                let (Some(x), Some(y)) = (number_at(index), number_at(index + 1)) else {
                    break;
                };
                index += 2;
                let relative = matches!(command, b'm' | b'l');
                current = if relative {
                    (current.0 + x, current.1 + y)
                } else {
                    (x, y)
                };
                if matches!(command, b'M' | b'm') {
                    start = current;
                    points.clear();
                    command = if command == b'M' { b'L' } else { b'l' };
                }
                points.push(current);
            }
            b'H' | b'h' => {
                let Some(x) = number_at(index) else {
                    break;
                };
                index += 1;
                current.0 = if command == b'h' { current.0 + x } else { x };
                points.push(current);
            }
            b'V' | b'v' => {
                let Some(y) = number_at(index) else {
                    break;
                };
                index += 1;
                current.1 = if command == b'v' { current.1 + y } else { y };
                points.push(current);
            }
            _ => break,
        }
    }
    rectangles
}

/// Blink ignores a trailing, incomplete moveto while retaining the preceding
/// valid subpath. SkSVG rejects the entire `d` attribute. Normalize that SVG
/// error-recovery case before deterministic static-resource decoding.
fn sanitize_static_svg(bytes: &[u8]) -> String {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return String::new();
    };
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(attribute) = rest.find(" d=") {
        output.push_str(&rest[..attribute + 3]);
        rest = &rest[attribute + 3..];
        let Some(quote) = rest
            .chars()
            .next()
            .filter(|quote| *quote == '\'' || *quote == '"')
        else {
            continue;
        };
        output.push(quote);
        rest = &rest[quote.len_utf8()..];
        let Some(end) = rest.find(quote) else {
            output.push_str(rest);
            return output;
        };
        let path_data = &rest[..end];
        let trailing_moveto = path_data
            .rfind(['M', 'm'])
            .filter(|index| path_data[*index + 1..].trim().parse::<f32>().is_ok());
        if let Some(index) = trailing_moveto {
            output.push_str(&path_data[..index]);
        } else {
            output.push_str(path_data);
        }
        output.push(quote);
        rest = &rest[end + quote.len_utf8()..];
    }
    output.push_str(rest);
    output
}

fn svg_has_explicit_dimension(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let Some(start) = text.find("<svg") else {
        return false;
    };
    let Some(end) = text[start..].find('>') else {
        return false;
    };
    let root = &text[start..start + end];
    ["width", "height"].iter().any(|name| {
        root.find(name)
            .is_some_and(|index| root[index + name.len()..].trim_start().starts_with('='))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_resource_is_an_error() {
        let doc = Document::new();
        assert!(decode_image_resource(&doc, ImageResourceId::new(9)).is_err());
    }

    #[test]
    fn static_svg_has_deterministic_intrinsic_dimensions_and_cache() {
        let mut doc = Document::new();
        let id = doc.register_image_resource(
            "inline.svg",
            "image/svg+xml",
            "fixture-green-7x5",
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="7" height="5"><rect width="7" height="5" fill="green"/></svg>"#.to_vec(),
        );
        let first = decode_image_resource(&doc, id).unwrap();
        let second = decode_image_resource(&doc, id).unwrap();
        assert_eq!((first.width(), first.height()), (7, 5));
        assert_eq!(first.unique_id(), second.unique_id());
    }

    #[test]
    fn static_svg_axis_aligned_paths_ignore_an_incomplete_trailing_moveto() {
        let mut doc = Document::new();
        let id = doc.register_image_resource(
            "inline-path.svg",
            "image/svg+xml",
            "fixture-blue-frame-48",
            br#"<svg width='48' height='48'><g fill='blue'><path d='M2 2h4v44H2z'/><path d='M2 2h44v4H2z'/><path d='M42 2h4v44h-4z'/><path d='M2 42h44v4H2zM8'/></g></svg>"#.to_vec(),
        );
        let image = decode_image_resource(&doc, id).expect("decode limited static SVG path");
        let info = ImageInfo::new_n32_premul((48, 48), None);
        let mut pixels = vec![0_u8; 48 * 48 * 4];
        assert!(image.read_pixels(&info, &mut pixels, 48 * 4, (0, 0), CachingHint::Allow));
        let pixel = |x: usize, y: usize| &pixels[(y * 48 + x) * 4..][..4];
        assert_eq!(pixel(2, 2), [255, 0, 0, 255]);
        assert_eq!(pixel(20, 20), [0, 0, 0, 0]);
        assert_eq!(pixel(45, 45), [255, 0, 0, 255]);
    }

    #[test]
    fn png_decoding_preserves_intrinsic_pixels_and_cache_identity() {
        let mut doc = Document::new();
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/accountability/data/wpt_assets/sp13p/1x1-green.png"
        ));
        let id = doc.register_image_resource(
            "css-backgrounds/support/1x1-green.png",
            "image/png",
            "a236213916dd30bd771a233aa1d66381eabf335bf8885304b75a4e2e370d68ce",
            bytes.to_vec(),
        );
        let first = decode_image_resource(&doc, id).expect("decode PNG");
        let second = decode_image_resource(&doc, id).expect("cached PNG");
        assert_eq!((first.width(), first.height()), (1, 1));
        assert_eq!(first.unique_id(), second.unique_id());
        let info = ImageInfo::new_n32_premul((1, 1), None);
        let mut pixels = [0_u8; 4];
        assert!(first.read_pixels(&info, &mut pixels, 4, (0, 0), CachingHint::Allow));
        assert_eq!(pixels, [0, 128, 0, 255]);
    }
}
