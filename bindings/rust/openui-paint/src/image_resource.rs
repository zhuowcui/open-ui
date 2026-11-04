//! Deterministic decoding for document-owned CSS image resources.

use std::cell::RefCell;
use std::collections::HashMap;

use openui_dom::{Document, EncodedImageResource};
use openui_style::{parse_literal, Color, ImageResourceId, StyleProperty, StyleValue};
use skia_safe::image::CachingHint;
use skia_safe::{
    surfaces, AlphaType, Canvas, ClipOp, ColorSpace, ColorType, Data, Image, ImageInfo, Paint,
    PaintStyle, Rect,
};

thread_local! {
    static IMAGE_CACHE: RefCell<HashMap<String, Image>> = RefCell::new(HashMap::new());
    static MIP_DECODE_CACHE: RefCell<HashMap<(String, i32, i32), Image>> =
        RefCell::new(HashMap::new());
    static QUANTIZED_REPEAT_CACHE: RefCell<HashMap<(String, i32, i32), Image>> =
        RefCell::new(HashMap::new());
    static SVG_CACHE: RefCell<HashMap<String, StaticSvg>> = RefCell::new(HashMap::new());
}

/// Return the software-raster decode selected for a raster image draw.
///
/// Chromium's software image cache decodes at the largest power-of-two mip
/// whose dimensions are no smaller than the requested draw. PNG cannot decode
/// natively at that size, so the cache creates it with Skia's medium-quality
/// `scalePixels` path before the final image draw.
///
/// Chromium builds Skia with `SK_SUPPORT_LEGACY_ANISOTROPIC_MIPMAP_SCALE`,
/// which selects a mip from the geometric mean of the two scales. rust-skia
/// uses the smaller scale instead. Reproduce Chromium's selected level first,
/// then perform the remaining bilinear scale explicitly. Keeping this as a
/// document-resource operation also gives repeated draws deterministic cache
/// identity without making painting depend on a test or source filename.
pub fn decode_raster_resource_for_draw(
    doc: &Document,
    id: ImageResourceId,
    requested_width: i32,
    requested_height: i32,
) -> Result<Image, String> {
    let resource = doc
        .image_resource(id)
        .ok_or_else(|| format!("missing image resource {}", id.0))?;
    let image = decode_image_resource(doc, id)?;
    if !matches!(resource.mime_type.as_str(), "image/png" | "image/jpeg")
        || requested_width <= 0
        || requested_height <= 0
    {
        return Ok(image);
    }

    let source_width = image.width();
    let source_height = image.height();
    let mut level = 0_u32;
    loop {
        let next_level = level + 1;
        let round = (1_i64 << next_level) - 1;
        let next_width = ((source_width as i64 + round) >> next_level).max(1) as i32;
        let next_height = ((source_height as i64 + round) >> next_level).max(1) as i32;
        if next_width < requested_width || next_height < requested_height {
            break;
        }
        level = next_level;
        if next_width == 1 && next_height == 1 {
            break;
        }
    }
    if level == 0 {
        return Ok(image);
    }

    let round = (1_i64 << level) - 1;
    let width = ((source_width as i64 + round) >> level).max(1) as i32;
    let height = ((source_height as i64 + round) >> level).max(1) as i32;
    let key = (resource.sha256.clone(), width, height);
    if let Some(decoded) = MIP_DECODE_CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
        return Ok(decoded);
    }

    // Retain the encoded image's color space through the intermediate cache.
    // Tagging profiled source pixels as sRGB here would bypass the color
    // conversion that occurs when the cached image reaches the sRGB canvas.
    let resource_color_space = image.color_space();
    let source_info =
        ImageInfo::new_n32_premul((source_width, source_height), resource_color_space.clone());
    let source_row_bytes = source_width as usize * 4;
    let mut source_pixels = vec![0_u8; source_row_bytes * source_height as usize];
    if !image.read_pixels(
        &source_info,
        &mut source_pixels,
        source_row_bytes,
        (0, 0),
        CachingHint::Allow,
    ) {
        return Err(format!("failed to read image resource {}", id.0));
    }
    let source_pixmap = skia_safe::Pixmap::new(&source_info, &mut source_pixels, source_row_bytes)
        .ok_or_else(|| "failed to create raster source pixmap".to_string())?;

    let info = ImageInfo::new_n32_premul((width, height), resource_color_space.clone());
    let row_bytes = width as usize * 4;
    let mut pixels = vec![0_u8; row_bytes * height as usize];
    let mut pixmap = skia_safe::Pixmap::new(&info, &mut pixels, row_bytes)
        .ok_or_else(|| "failed to allocate raster decode pixmap".to_string())?;
    let scale =
        ((width as f32 / source_width as f32) * (height as f32 / source_height as f32)).sqrt();
    let selected_level = ((-scale.log2() - 0.5).max(0.0) + 0.5).floor() as u32;
    let scaled = if selected_level == 0 {
        source_pixmap.scale_pixels(
            &mut pixmap,
            skia_safe::SamplingOptions::new(
                skia_safe::FilterMode::Linear,
                skia_safe::MipmapMode::None,
            ),
        )
    } else {
        let mip_width = (source_width >> selected_level).max(1);
        let mip_height = (source_height >> selected_level).max(1);
        let mip_info = ImageInfo::new_n32_premul((mip_width, mip_height), resource_color_space);
        let mip_row_bytes = mip_width as usize * 4;
        let mut mip_pixels = vec![0_u8; mip_row_bytes * mip_height as usize];
        let mut mip_pixmap = skia_safe::Pixmap::new(&mip_info, &mut mip_pixels, mip_row_bytes)
            .ok_or_else(|| "failed to allocate raster mip pixmap".to_string())?;
        if !source_pixmap.scale_pixels(
            &mut mip_pixmap,
            skia_safe::SamplingOptions::new(
                skia_safe::FilterMode::Linear,
                skia_safe::MipmapMode::Nearest,
            ),
        ) {
            return Err(format!("failed to build image mip for resource {}", id.0));
        }
        mip_pixmap.scale_pixels(
            &mut pixmap,
            skia_safe::SamplingOptions::new(
                skia_safe::FilterMode::Linear,
                skia_safe::MipmapMode::None,
            ),
        )
    };
    if !scaled {
        return Err(format!("failed to scale image resource {}", id.0));
    }
    let decoded = skia_safe::images::raster_from_data(&info, Data::new_copy(&pixels), row_bytes)
        .ok_or_else(|| format!("failed to retain scaled image resource {}", id.0))?;
    MIP_DECODE_CACHE.with(|cache| {
        cache.borrow_mut().insert(key, decoded.clone());
    });
    Ok(decoded)
}

#[derive(Clone, Debug)]
struct StaticSvg {
    width: Option<f32>,
    height: Option<f32>,
    view_box: Option<Rect>,
    background: Option<skia_safe::Color>,
    shapes: Vec<StaticSvgShape>,
}

#[derive(Clone, Debug)]
struct StaticSvgShape {
    rect: Rect,
    color: skia_safe::Color,
    stroke_color: Option<skia_safe::Color>,
    stroke_width: f32,
}

/// Decode one registered raster, decoded media frame, or limited static SVG
/// resource.
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

    let mut image = match resource.mime_type.as_str() {
        "image/png" | "image/jpeg" | "image/gif" => {
            Image::from_encoded(Data::new_copy(&resource.bytes))
                .ok_or_else(|| format!("failed to decode raster resource {}", resource.source))?
        }
        "image/x-openui-rgba8" => decode_rgba8_resource(resource)?,
        "image/svg+xml" => {
            let svg = parsed_svg_resource(resource)?;
            rasterize_static_svg(&svg, &resource.source, None)?
        }
        mime => {
            return Err(format!(
                "unsupported image MIME type {mime} for {}",
                resource.source
            ));
        }
    };
    // Chromium's SkPngRustCodec omits a custom profile when gAMA is neutral
    // and no higher-precedence profile or chromaticity is present. Blink then
    // tags those decoded source samples as sRGB. The libpng codec used by
    // rust-skia otherwise gives these same bytes a power-2.2 profile, changing
    // their colors at draw time. Reinterpret the source metadata, retaining
    // the encoded channel values rather than converting them first.
    if resource.mime_type == "image/png"
        && png_has_neutral_gamma_without_color_profile(&resource.bytes)
    {
        image = image
            .reinterpret_color_space(ColorSpace::new_srgb())
            .ok_or_else(|| {
                format!(
                    "failed to retain PNG source color space {}",
                    resource.source
                )
            })?;
    }
    IMAGE_CACHE.with(|cache| {
        cache
            .borrow_mut()
            .insert(resource.sha256.clone(), image.clone());
    });
    Ok(image)
}

fn png_chunk_crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0_u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

fn png_has_neutral_gamma_without_color_profile(bytes: &[u8]) -> bool {
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return false;
    }
    let mut offset = 8_usize;
    let mut gamma = None;
    while let Some(header) = bytes.get(offset..).and_then(|tail| tail.get(..8)) {
        let length = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
        let kind = &header[4..];
        let Some(data_end) = offset
            .checked_add(8)
            .and_then(|start| start.checked_add(length))
        else {
            return false;
        };
        let Some(chunk_end) = data_end.checked_add(4).filter(|end| *end <= bytes.len()) else {
            return false;
        };
        match kind {
            b"cICP" | b"iCCP" | b"sRGB" | b"cHRM" => return false,
            b"gAMA" if length == 4 => {
                let expected = u32::from_be_bytes(bytes[data_end..chunk_end].try_into().unwrap());
                if png_chunk_crc32(&bytes[offset + 4..data_end]) != expected {
                    return false;
                }
                gamma = Some(u32::from_be_bytes(
                    bytes[offset + 8..data_end].try_into().unwrap(),
                ));
            }
            b"IDAT" => break,
            _ => {}
        }
        offset = chunk_end;
    }
    gamma.is_some_and(|gamma| {
        let relative_gamma = gamma as f32 / 100_000.0 * 2.2;
        relative_gamma > 0.95 && relative_gamma < 1.05
    })
}

/// A fully opaque one-pixel raster has the same sample at every repeated
/// position. Read it in the destination sRGB space before replacing an
/// otherwise unbounded sequence of image draws with one solid fill.
pub(crate) fn opaque_single_pixel_raster_color(
    doc: &Document,
    id: ImageResourceId,
) -> Option<Color> {
    let image = decode_image_resource(doc, id).ok()?;
    if image.width() != 1 || image.height() != 1 {
        return None;
    }
    let info = ImageInfo::new(
        (1, 1),
        ColorType::RGBA8888,
        AlphaType::Unpremul,
        Some(ColorSpace::new_srgb()),
    );
    let mut rgba = [0_u8; 4];
    if !image.read_pixels(&info, &mut rgba, 4, (0, 0), CachingHint::Allow) || rgba[3] != 255 {
        return None;
    }
    Some(Color::from_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]))
}

/// Whether Chromium's decoded-image generator retains the complete inverse
/// transform phase when the resource is used by a repeating background
/// shader.
///
/// Low-bit indexed PNGs are expanded through the packed palette path before
/// sampling. Eight-bit indexed images retain the floating-point shader phase;
/// truecolor and already-decoded sources retain it only for a one-to-one
/// physical transform. Keep this decision tied to the encoded representation
/// and concrete transform rather than to a particular background or test.
pub(crate) fn retains_full_precision_background_phase(
    doc: &Document,
    id: ImageResourceId,
    physical_scale_x: f32,
    physical_scale_y: f32,
) -> bool {
    let Some(resource) = doc.image_resource(id) else {
        return false;
    };
    let unit_physical_scale =
        (physical_scale_x - 1.0).abs() <= 1.0e-5 && (physical_scale_y - 1.0).abs() <= 1.0e-5;
    match resource.mime_type.as_str() {
        "image/png" => {
            let bytes = resource.bytes.as_slice();
            bytes.get(..8) == Some(b"\x89PNG\r\n\x1a\n")
                && bytes.get(12..16) == Some(b"IHDR")
                && bytes.get(24).is_some_and(|bit_depth| *bit_depth >= 8)
                && bytes
                    .get(25)
                    .is_some_and(|color_type| *color_type == 3 || unit_physical_scale)
        }
        "image/jpeg" | "image/x-openui-rgba8" => unit_physical_scale,
        _ => false,
    }
}

/// Decode the stable transport used by generated first-frame media fixtures.
///
/// The format is deliberately smaller than an image codec: `OUIR`, version
/// byte 1, three reserved zero bytes, little-endian width and height, followed
/// by tightly packed unpremultiplied sRGB RGBA8 pixels. Media decoding belongs
/// to fixture generation; paint only consumes the hash-pinned decoded result.
fn decode_rgba8_resource(resource: &EncodedImageResource) -> Result<Image, String> {
    const HEADER_LEN: usize = 16;
    let bytes = resource.bytes.as_slice();
    if bytes.len() < HEADER_LEN
        || &bytes[..4] != b"OUIR"
        || bytes[4] != 1
        || bytes[5..8] != [0, 0, 0]
    {
        return Err(format!(
            "invalid decoded RGBA8 header for {}",
            resource.source
        ));
    }
    let width = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    let height = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
    let pixel_len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|count| count.checked_mul(4))
        .ok_or_else(|| format!("decoded RGBA8 dimensions overflow for {}", resource.source))?;
    if width == 0 || height == 0 || bytes.len() != HEADER_LEN + pixel_len {
        return Err(format!(
            "invalid decoded RGBA8 payload length for {}",
            resource.source
        ));
    }
    let info = ImageInfo::new(
        (width as i32, height as i32),
        ColorType::RGBA8888,
        AlphaType::Unpremul,
        Some(ColorSpace::new_srgb()),
    );
    skia_safe::images::raster_from_data(
        &info,
        Data::new_copy(&bytes[HEADER_LEN..]),
        width as usize * 4,
    )
    .ok_or_else(|| {
        format!(
            "failed to retain decoded RGBA8 resource {}",
            resource.source
        )
    })
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
    predecessor_owned_seams_scale: Option<f32>,
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
    if let Some(device_scale) = predecessor_owned_seams_scale {
        clip_svg_patch_to_predecessor(
            canvas,
            source,
            concrete_width,
            concrete_height,
            destination,
            device_scale,
        );
    } else {
        canvas.clip_rect(destination, ClipOp::Intersect, true);
    }

    let legacy_right = predecessor_owned_seams_scale.and_then(|device_scale| {
        (source.right >= concrete_width - 1.0 / 1024.0)
            .then(|| {
                legacy_svg_outer_edge(
                    source.left,
                    source.width(),
                    destination.left,
                    destination.width(),
                    concrete_width,
                    device_scale,
                )
            })
            .flatten()
            .map(|edge| (edge, device_scale))
    });
    let legacy_bottom = predecessor_owned_seams_scale.and_then(|device_scale| {
        (source.bottom >= concrete_height - 1.0 / 1024.0)
            .then(|| {
                legacy_svg_outer_edge(
                    source.top,
                    source.height(),
                    destination.top,
                    destination.height(),
                    concrete_height,
                    device_scale,
                )
            })
            .flatten()
            .map(|edge| (edge, device_scale))
    });
    let unbounded = 1_000_000.0_f32;
    let x_segments = if let Some((edge, device_scale)) = legacy_right {
        vec![
            (-unbounded, edge - 1.0 / device_scale, 1.0),
            (edge - 1.0 / device_scale, unbounded, 253.0 / 255.0),
        ]
    } else {
        vec![(-unbounded, unbounded, 1.0)]
    };
    let y_segments = if let Some((edge, device_scale)) = legacy_bottom {
        vec![
            (-unbounded, edge - 1.0 / device_scale, 1.0),
            (edge - 1.0 / device_scale, unbounded, 253.0 / 255.0),
        ]
    } else {
        vec![(-unbounded, unbounded, 1.0)]
    };
    for (left, right, horizontal_coverage) in x_segments {
        for &(top, bottom, vertical_coverage) in &y_segments {
            canvas.save();
            canvas.clip_rect(
                Rect::from_ltrb(left, top, right, bottom),
                ClipOp::Intersect,
                false,
            );
            let segment_opacity = opacity * horizontal_coverage * vertical_coverage;
            if segment_opacity < 1.0 {
                canvas.save_layer_alpha_f(destination, segment_opacity);
            }
            canvas.translate((destination.left, destination.top));
            canvas.scale((
                destination.width() / source.width(),
                destination.height() / source.height(),
            ));
            canvas.translate((-source.left, -source.top));
            paint_static_svg(canvas, &svg, concrete_width, concrete_height, Some(source));
            if segment_opacity < 1.0 {
                canvas.restore();
            }
            canvas.restore();
        }
    }
    canvas.restore();
    Ok(())
}

/// Clip internal vector-image patch seams in physical space.
///
/// Blink assigns the touched device cell at a sliced-image seam to the
/// preceding patch.  The preceding vector geometry supplies its analytic
/// coverage; the following patch begins at the next physical cell.  Keeping
/// the clip hard also avoids multiplying the geometry coverage by a second
/// antialiased mask.
pub(crate) fn clip_svg_patch_to_predecessor(
    canvas: &Canvas,
    source: Rect,
    concrete_width: f32,
    concrete_height: f32,
    destination: Rect,
    device_scale: f32,
) {
    let scale = device_scale.max(f32::EPSILON);
    let snap_after = |value: f32| (value * scale).ceil() / scale;
    let epsilon = 1.0 / 1024.0;
    let unbounded = 1_000_000.0_f32;
    let clip = Rect::from_ltrb(
        if source.left > epsilon {
            snap_after(destination.left)
        } else {
            -unbounded
        },
        if source.top > epsilon {
            snap_after(destination.top)
        } else {
            -unbounded
        },
        if source.right < concrete_width - epsilon {
            snap_after(destination.right)
        } else {
            unbounded
        },
        if source.bottom < concrete_height - epsilon {
            snap_after(destination.bottom)
        } else {
            unbounded
        },
    );
    canvas.clip_rect(clip, ClipOp::Intersect, false);
}

fn legacy_svg_outer_edge(
    source_start: f32,
    source_extent: f32,
    destination_start: f32,
    destination_extent: f32,
    concrete_edge: f32,
    device_scale: f32,
) -> Option<f32> {
    // Cast through f64 after each operation to make the intended f32 matrix
    // boundaries observable even on targets where LLVM contracts `a * b + c`.
    let f32_product = |left: f32, right: f32| (left as f64 * right as f64) as f32;
    let f32_sum = |left: f32, right: f32| (left as f64 + right as f64) as f32;
    let patch_scale = (destination_extent as f64 / source_extent as f64) as f32;
    let physical_scale = f32_product(device_scale, patch_scale);
    let local_translate = f32_sum(destination_start, -f32_product(patch_scale, source_start));
    let physical_translate = f32_product(device_scale, local_translate);
    let projected = f32_sum(
        f32_product(physical_scale, concrete_edge),
        physical_translate,
    );
    let integral = projected.round();
    let shortfall = integral - projected;
    (shortfall > 0.0 && shortfall < 1.0 / 1024.0).then_some(integral / device_scale)
}

/// Return the opaque color of a single rectangle that covers an SVG's full
/// user viewport. Replaced-content painting uses this to reproduce Chromium's
/// composited edge coverage without keying behavior to a resource name.
pub fn static_svg_full_viewport_color(
    doc: &Document,
    id: ImageResourceId,
) -> Option<skia_safe::Color> {
    let resource = doc.image_resource(id)?;
    if resource.mime_type != "image/svg+xml" {
        return None;
    }
    let svg = parsed_svg_resource(resource).ok()?;
    if svg.background.is_some() || svg.shapes.len() != 1 {
        return None;
    }
    let shape = &svg.shapes[0];
    if shape.stroke_color.is_some() || shape.color.a() != 255 {
        return None;
    }
    let viewport = svg.view_box.unwrap_or_else(|| {
        let (width, height) = static_svg_intrinsic_size(&svg);
        Rect::from_xywh(0.0, 0.0, width, height)
    });
    let close = |left: f32, right: f32| (left - right).abs() <= 1.0 / 1024.0;
    (close(shape.rect.left, viewport.left)
        && close(shape.rect.top, viewport.top)
        && close(shape.rect.right, viewport.right)
        && close(shape.rect.bottom, viewport.bottom))
    .then_some(shape.color)
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
    /// Replaced raster content retains the pixel-snapped inverse matrix and
    /// Chromium's historical packed-pixel interpolation arithmetic.
    Replaced,
}

/// Pixels per `SkBitmapProcShader` legacy `shadeSpan` chunk for a bilinear
/// scale-translate matrix: `maxCountForBufferSize` turns the 128-word buffer
/// into `(512 - 4) / 2` coordinates, two per bilinear pixel.
const LEGACY_BILERP_SPAN_CHUNK: i32 = 127;

/// Pixels per `blitAntiH` run when `SkScan::AntiFillRect` blits a partly
/// covered row: `call_hline_blitter` sends at most `HLINE_STACK_BUFFER`.
const ANTI_FILL_RECT_HLINE_RUN: i32 = 100;

fn compositor_raster_tile_origin(device_pixel: i32) -> i32 {
    // Chromium's 256px software-raster tiles overlap their predecessor by two
    // texels. Coverage switches to the next tile after the first 255 pixels,
    // so later tile origins advance by 254px.
    if device_pixel < 255 {
        0
    } else {
        ((device_pixel - 1) / 254) * 254
    }
}

fn physical_color_coverage_rounding_bias(
    has_coverage_bounds: bool,
    legacy_packed_coverage: bool,
    repeating: bool,
    replaced: bool,
    geometric_coverage: u32,
    target_x: i32,
    target_y: i32,
    raw_x0: i32,
    fractional_destination_width: bool,
) -> u32 {
    if has_coverage_bounds && !legacy_packed_coverage {
        255
    } else if has_coverage_bounds
        && repeating
        && geometric_coverage >= 128
        && geometric_coverage != 129
        && target_x > 0
        && target_y > 0
        && (raw_x0 != 0 || !fractional_destination_width)
    {
        255
    } else if replaced {
        // Chromium folds analytic edge coverage into its packed replaced-
        // image samples before storing the premultiplied channel. Using the
        // coverage itself is equivalent to rounding that packed product to
        // the nearest representable value.
        geometric_coverage
    } else if geometric_coverage < 128 {
        127
    } else {
        128
    }
}

fn opaque_border_repeated_color_coverage_bias(geometric_coverage: u32) -> u32 {
    if geometric_coverage < 128 {
        geometric_coverage + geometric_coverage / 2
    } else {
        geometric_coverage
    }
}

fn physical_image_axis_coverage_bounds(
    destination_start: f32,
    destination_end: f32,
    clip_bounds: Option<(f32, f32)>,
    repeating: bool,
) -> (f32, f32) {
    let Some((clip_start, clip_end)) = clip_bounds else {
        return (destination_start, destination_end);
    };
    if repeating {
        (clip_start, clip_end)
    } else {
        (
            destination_start.max(clip_start),
            destination_end.min(clip_end),
        )
    }
}

fn physical_axis_coverage(
    pixel: i32,
    start: f32,
    end: f32,
    close_nonempty_span_upward: bool,
) -> u32 {
    let overlap = (end.min(pixel as f32 + 1.0) - start.max(pixel as f32)).clamp(0.0, 1.0);
    if overlap <= 0.0 {
        0
    } else if overlap >= 1.0 {
        255
    } else if close_nonempty_span_upward {
        (overlap * 256.0).floor() as u32 + 1
    } else {
        (overlap * 256.0).ceil().min(255.0) as u32
    }
}

/// Whether a replaced image patch keeps its full samples for the caller to
/// draw as an antialiased rect at the fractional destination.
///
/// Blink paints an unclipped replaced image with one antialiased
/// `drawImageRect`, which Skia's bitmap device draws as a rect filled with
/// the image shader. The rect fill lerps each partly covered edge pixel
/// toward the shaded color, so the result depends on the device color
/// below. Baking the coverage into the patch and compositing it source-over
/// is one level off over a colored device. A clip owns the edge otherwise,
/// and full-precision (JPEG) sampling keeps its own edge model.
pub fn replaced_image_patch_draws_as_skia_rect_fill(
    full_precision_sampling: bool,
    coverage_bounds: Option<Rect>,
) -> bool {
    !full_precision_sampling && coverage_bounds.is_none()
}

/// `SkFixedToFDot8(SkScalarToFixed(value))`: Skia's analytic rect fill rounds
/// each device edge to 1/256 of a pixel.
fn skia_fdot8(value: f32) -> i32 {
    ((value * 65536.0) as i32 + 0x80) >> 8
}

/// Whether `antifilldot8` (`SkScan_Antihair.cpp`) gives the pixel nonzero
/// coverage on one axis of an antialiased rect without a clip. A span inside
/// one pixel has coverage `R - L - 1`.
fn skia_rect_axis_touches(pixel: i32, start: f32, end: f32) -> bool {
    let (start, end) = (skia_fdot8(start), skia_fdot8(end));
    if start >= end || pixel < start >> 8 || pixel > (end - 1) >> 8 {
        false
    } else {
        start >> 8 != (end - 1) >> 8 || end - start > 1
    }
}

pub(crate) fn combine_physical_axis_coverage(horizontal: u32, vertical: u32) -> u32 {
    match (horizontal, vertical) {
        (255, coverage) | (coverage, 255) => coverage,
        // Analytic X/Y coverage is packed into the compositor's eight-bit
        // mask with nearest rounding when either span covers less than half a
        // pixel. Ceiling every non-empty corner is too bright, and rounding
        // high/high products makes two three-quarter trailing edges one step
        // too dark.
        _ => {
            let bias = u32::from(horizontal.min(vertical) < 128) * 127;
            (horizontal * vertical + bias) >> 8
        }
    }
}

fn blink_layout_unit(value: f32) -> f32 {
    // Blink stores background geometry in LayoutUnit (six fractional bits).
    // Its floating-point constructor truncates toward zero.
    (value * 64.0).trunc() / 64.0
}

fn blink_pixel_snapped_rect(rect: Rect) -> Rect {
    let left = blink_layout_unit(rect.left);
    let top = blink_layout_unit(rect.top);
    let right = blink_layout_unit(rect.right);
    let bottom = blink_layout_unit(rect.bottom);
    Rect::from_ltrb(left.round(), top.round(), right.round(), bottom.round())
}

fn blink_single_tile_subset(source: Rect, destination: Rect, clip: Rect) -> Option<(Rect, Rect)> {
    let visible = Rect::from_ltrb(
        destination.left.max(clip.left),
        destination.top.max(clip.top),
        destination.right.min(clip.right),
        destination.bottom.min(clip.bottom),
    );
    if visible.width() <= 0.0 || visible.height() <= 0.0 {
        return None;
    }

    // Blink's single-tile background optimization derives the source subset
    // from the unsnapped intersection, then maps that subset to a CSS-pixel
    // snapped destination. The distinction is observable when a no-repeat
    // image with an auto aspect-ratio dimension is clipped before its far
    // edge: the snapped destination is slightly larger, so its source phase
    // advances more slowly along that axis.
    let scale_x = destination.width() / source.width();
    let scale_y = destination.height() / source.height();
    let subset = Rect::from_ltrb(
        source.left + (visible.left - destination.left) / scale_x,
        source.top + (visible.top - destination.top) / scale_y,
        source.left + (visible.right - destination.left) / scale_x,
        source.top + (visible.bottom - destination.top) / scale_y,
    );
    let snapped = blink_pixel_snapped_rect(visible);
    (snapped.width() > 0.0 && snapped.height() > 0.0).then_some((subset, snapped))
}

fn replaced_sample_fixed(
    source_start: f32,
    source_extent: f32,
    destination_start: f32,
    destination_extent: f32,
    target_index: i32,
    accumulate_scanline: bool,
) -> i64 {
    replaced_sample_fixed_at(
        source_start,
        source_extent,
        destination_start,
        destination_extent,
        1.0,
        destination_start.round() as i32 + target_index,
        accumulate_scanline,
        false,
    )
}

/// Return the first device pixel of the legacy shader span that samples
/// `pixel` along one axis of an antialiased replaced-image rect.
///
/// `SkScan::AntiFillRect` blits a partly covered edge column on its own and
/// the interior from `ceil(left)` to `floor(right)`. The raster tile clips
/// the interior at its origin, and `SkBitmapProcShader` maps the first pixel
/// of every `LEGACY_BILERP_SPAN_CHUNK` pixels again. A partly covered row
/// (see `anti_fill_rect_partial_row`) reaches the shader in runs of
/// `ANTI_FILL_RECT_HLINE_RUN` pixels instead.
fn legacy_shader_span_start(
    device_start: f32,
    device_end: f32,
    pixel: i32,
    partial_row: bool,
) -> i32 {
    let (left, right) = (skia_fdot8(device_start), skia_fdot8(device_end));
    let interior_start = (left + 0xff) >> 8;
    let interior_end = right >> 8;
    if left >> 8 == (right - 1) >> 8 || pixel < interior_start || pixel >= interior_end {
        return pixel;
    }
    let span_start = interior_start.max(compositor_raster_tile_origin(pixel));
    let chunk = if partial_row {
        ANTI_FILL_RECT_HLINE_RUN
    } else {
        LEGACY_BILERP_SPAN_CHUNK
    };
    span_start + (pixel - span_start) / chunk * chunk
}

/// Return whether `SkScan::AntiFillRect` blits device row `pixel` of a rect
/// from `device_start` to `device_end` as one partly covered scanline.
///
/// `antifilldot8` sends a rect one row high, and a top or bottom row with a
/// fractional 24.8 edge, through `do_scanline`. Full rows go through
/// `blitRect`.
fn anti_fill_rect_partial_row(device_start: f32, device_end: f32, pixel: i32) -> bool {
    let (top, bottom) = (skia_fdot8(device_start), skia_fdot8(device_end));
    if top >= bottom {
        return false;
    }
    if top >> 8 == (bottom - 1) >> 8 {
        return pixel == top >> 8;
    }
    (top & 0xff != 0 && pixel == top >> 8) || (bottom & 0xff != 0 && pixel == bottom >> 8)
}

/// Return the 32.32 source coordinate, less half a texel, that Chromium's
/// legacy bilinear shader samples for one device pixel of a replaced image.
///
/// Blink draws `drawImageRect(src, dst)` with `dst` in CSS pixels, under the
/// raster tile's device scale and translation. Skia concatenates
/// `RectToRect(src, dst)` with that matrix and inverts it in `f32`, so the
/// step is the reciprocal of `device_scale * (dst / src)`, not of the
/// physical extent over the source. A cached mip decode is drawn the same
/// way with its scaled source rect. The step then accumulates from the first
/// pixel of each shader span.
fn replaced_sample_fixed_at(
    source_start: f32,
    source_extent: f32,
    destination_start: f32,
    destination_extent: f32,
    device_scale: f32,
    destination_pixel: i32,
    accumulate_scanline: bool,
    partial_row: bool,
) -> i64 {
    const FIXED_ONE: f32 = 4_294_967_296.0;
    const FILTER_BIAS: i64 = 1_i64 << 31;

    let tile_origin = compositor_raster_tile_origin(destination_pixel);
    let local_scale = destination_extent / source_extent;
    let local_translate = destination_start - source_start * local_scale;
    let device_scale_axis = device_scale * local_scale;
    let device_translate = device_scale * local_translate + -(tile_origin as f32);
    let inverse_scale = 1.0_f32 / device_scale_axis;
    let inverse_translate = -device_translate * inverse_scale;
    let segment_start = if accumulate_scanline {
        let device_start = destination_start * device_scale;
        let device_end = (destination_start + destination_extent) * device_scale;
        legacy_shader_span_start(device_start, device_end, destination_pixel, partial_row)
    } else {
        destination_pixel
    };
    let mapped_start = (segment_start - tile_origin) as f32 + 0.5;
    let mapped_start = mapped_start * inverse_scale + inverse_translate;
    let fixed_start = (mapped_start * FIXED_ONE) as i64 - FILTER_BIAS;
    let fixed_step = (inverse_scale * FIXED_ONE) as i64;
    fixed_start + fixed_step * i64::from(destination_pixel - segment_start)
}

/// Return the low/high texel and four-bit weight used by Skia's normalized
/// repeat matrix for one physical destination pixel.
///
/// Repeat shaders divide their inverse matrix by the source extent before
/// mapping device coordinates. They then convert that normalized coordinate
/// to 32.32 fixed point, subtract half of a truncated 16.16 texel, and only
/// then recover the source texel and packed interpolation weight. Performing
/// the equivalent arithmetic directly in source-pixel space loses observable
/// borrows at exact integer and half-texel phases.
/// One axis of the repeating bitmap shader that Chromium's legacy software
/// raster fills a background with, in the CSS pixels Blink records it in.
#[derive(Clone, Copy, Debug)]
struct LegacyRepeatScanline {
    /// CSS origin and extent of one repeated tile.
    tile_start: f32,
    tile_extent: f32,
    device_scale: f32,
    /// Device extent of the rect that the shader fills.
    fill_start: f32,
    fill_end: f32,
    /// Whether the fill blits this row as one partly covered scanline.
    partial_row: bool,
}

impl LegacyRepeatScanline {
    /// Return the 16.16 normalized coordinate, less half a texel, that the
    /// shader reaches at `pixel`.
    ///
    /// Skia concatenates the tile's `translate * scale` local matrix with
    /// the raster tile's device matrix, inverts it in `f32`, and divides it
    /// by the image extent. The first pixel of each shader span is mapped
    /// through that matrix and later pixels add its scale in 32.32 fixed
    /// point, so a half-texel tie closes up or down with the rounding of
    /// the `f32` inverse.
    fn fixed_16(self, source_extent: i32, pixel: i32) -> i64 {
        const FIXED_32_ONE: f64 = 4_294_967_296.0;
        let tile_origin = compositor_raster_tile_origin(pixel);
        let extent = source_extent as f32;
        let scale = self.device_scale * (self.tile_extent / extent);
        let translate = self.device_scale * self.tile_start - tile_origin as f32;
        let inverse_scale = 1.0_f32 / scale;
        let inverse_translate = -translate * inverse_scale;
        let normalized_scale = inverse_scale / extent;
        let normalized_translate = inverse_translate / extent;
        let span_start =
            legacy_shader_span_start(self.fill_start, self.fill_end, pixel, self.partial_row);
        let mapped =
            ((span_start - tile_origin) as f32 + 0.5) * normalized_scale + normalized_translate;
        let filter_one = (1_i64 << 16) / i64::from(source_extent);
        let start = (f64::from(mapped) * FIXED_32_ONE) as i64 - (filter_one >> 1) * (1_i64 << 16);
        let step = (f64::from(normalized_scale) * FIXED_32_ONE) as i64;
        (start + step * i64::from(pixel - span_start)) >> 16
    }
}

fn repeated_subrect_sample_fixed_at(
    source_start: i32,
    source_extent: i32,
    destination_start: f32,
    destination_extent: f32,
    destination_pixel: i32,
    rebase_across_raster_tiles: bool,
    scanline: Option<LegacyRepeatScanline>,
) -> (i32, i32, u32) {
    const FIXED_16_ONE: i64 = 1_i64 << 16;
    const FIXED_32_ONE: f32 = 4_294_967_296.0;

    let repeat_phase = (f64::from(destination_pixel) + 0.5 - f64::from(destination_start))
        / f64::from(destination_extent);
    if (repeat_phase - repeat_phase.round()).abs() <= 1.0e-9 {
        // Skia's normalized repeat matrix subtracts the half-texel in 16.16
        // fixed point before extracting its four-bit bilinear weight. The
        // first software-raster tile retains the exact 8/16 phase; after the
        // overlapping tile origin advances, matrix rebasing closes that tie
        // downward to 7/16.
        let weight = if rebase_across_raster_tiles
            && compositor_raster_tile_origin(destination_pixel) != 0
        {
            7
        } else {
            8
        };
        return (source_start + source_extent - 1, source_start, weight);
    }

    let filter_one = FIXED_16_ONE / i64::from(source_extent);
    // The horizontal bitmap proc advances each scanline from its span start.
    // Vertical sampling evaluates the matrix directly at each row.
    let fixed_16 = scanline.map_or_else(
        || {
            let normalized_scale = 1.0_f32 / destination_extent;
            let normalized_translate = -destination_start / destination_extent;
            let normalized =
                (destination_pixel as f32 + 0.5) * normalized_scale + normalized_translate;
            ((normalized * FIXED_32_ONE) as i64 - (filter_one >> 1) * FIXED_16_ONE) >> 16
        },
        |scanline| scanline.fixed_16(source_extent, destination_pixel),
    );
    let tile = |fixed: i64| -> i32 {
        ((((fixed & 0xffff) * i64::from(source_extent)) >> 16) as i32).clamp(0, source_extent - 1)
    };
    let low = source_start + tile(fixed_16);
    let high = source_start + tile(fixed_16 + filter_one);
    let weight = (((fixed_16 & 0xffff) * i64::from(source_extent)) >> 12) as u32 & 0xf;
    (low, high, weight)
}

/// Evaluate the floating-point inverse matrix used by a one-axis repeated
/// image shader in global device space.
///
/// Skia fuses the device-coordinate multiply and inverse translation. At an
/// exact half-texel phase this can leave the first repeat one representable
/// float above the tie while a later repeat lands exactly on it. Rebasing the
/// same calculation into each logical tile erases that observable distinction.
fn full_precision_repeated_source_coordinate(
    source_start: f32,
    source_extent: f32,
    repeat_origin: f32,
    repeat_extent: f32,
    destination_pixel: i32,
) -> f64 {
    let inverse_scale = source_extent / repeat_extent;
    let inverse_translate = source_start - repeat_origin * inverse_scale;
    f64::from((destination_pixel as f32 + 0.5).mul_add(inverse_scale, inverse_translate) - 0.5)
}

#[cfg(test)]
fn repeated_sample_fixed_at(
    source_extent: i32,
    destination_start: f32,
    destination_extent: f32,
    destination_pixel: i32,
) -> (i32, i32, u32) {
    repeated_subrect_sample_fixed_at(
        0,
        source_extent,
        destination_start,
        destination_extent,
        destination_pixel,
        true,
        None,
    )
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
    // Nine-slice border patches follow the canvas-device conversion path.
    // Reading those pixels in the encoded profile applies the profile twice
    // when the patch is drawn back to the sRGB surface. Replaced/background
    // resources retain their source profile through their dedicated paths.
    let patch_color_space = if phase == ImagePatchPhase::Device {
        None
    } else {
        image.color_space()
    };
    let source_info =
        ImageInfo::new_n32_premul((image_width, image_height), patch_color_space.clone());
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
    let target_info = ImageInfo::new_n32_premul((width, height), patch_color_space);
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
            ImagePatchPhase::Replaced => 0.0,
        };
        let replaced_fixed_y = (phase == ImagePatchPhase::Replaced).then(|| {
            replaced_sample_fixed(
                source.top,
                source.height(),
                destination.top,
                destination.height(),
                target_y,
                false,
            )
        });
        let raw_y0 = replaced_fixed_y
            .map(|fixed| (fixed >> 32) as i32)
            .unwrap_or_else(|| source_y.floor() as i32);
        let fy = replaced_fixed_y
            .map(|fixed| ((fixed >> 28) & 0xf) as f32 / 16.0)
            .unwrap_or_else(|| {
                let fraction_y = source_y - raw_y0 as f32;
                (fraction_y * 16.0).floor() / 16.0
            });
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
                ImagePatchPhase::Replaced => 0.0,
            };
            let replaced_fixed_x = (phase == ImagePatchPhase::Replaced).then(|| {
                replaced_sample_fixed(
                    source.left,
                    source.width(),
                    destination.left,
                    destination.width(),
                    target_x,
                    true,
                )
            });
            let raw_x0 = replaced_fixed_x
                .map(|fixed| (fixed >> 32) as i32)
                .unwrap_or_else(|| source_x.floor() as i32);
            let fx = replaced_fixed_x
                .map(|fixed| ((fixed >> 28) & 0xf) as f32 / 16.0)
                .unwrap_or_else(|| {
                    let fraction_x = source_x - raw_x0 as f32;
                    (fraction_x * 16.0).floor() / 16.0
                });
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
                let value = if phase == ImagePatchPhase::Replaced {
                    let wx = (fx * 16.0) as u32;
                    let wy = (fy * 16.0) as u32;
                    let at_u32 = |x: usize, y: usize| at(x, y) as u32;
                    ((at_u32(x0, y0) * (16 - wx) * (16 - wy)
                        + at_u32(x1, y0) * wx * (16 - wy)
                        + at_u32(x0, y1) * (16 - wx) * wy
                        + at_u32(x1, y1) * wx * wy)
                        >> 8) as u8
                } else {
                    let top = at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx;
                    let bottom = at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx;
                    (top * (1.0 - fy) + bottom * fy).round() as u8
                };
                target_pixels
                    [target_y as usize * target_row_bytes + target_x as usize * 4 + channel] =
                    value;
            }
        }
    }

    skia_safe::images::raster_from_data(
        &target_info,
        Data::new_copy(&target_pixels),
        target_row_bytes,
    )
    .ok_or_else(|| "failed to retain image patch pixels".to_string())
}

/// Pre-sample an image tile on the physical pixel grid used by Chromium.
///
/// Ganesh's low-precision bilinear path quantizes each source phase to four
/// fractional bits and truncates the packed interpolation result. Software
/// Skia instead keeps a floating-point phase and rounds the result, which can
/// make every interpolated channel one value brighter at fractional device
/// scales. Resolve that backend difference once, then let the caller replay
/// the returned image at the physical-pixel-aligned destination with nearest
/// sampling.
pub fn physical_quantized_image_patch(
    image: &Image,
    source: Rect,
    destination: Rect,
    device_scale: f32,
    wrap_x: bool,
    wrap_y: bool,
    rebase_repeat_x_across_raster_tiles: bool,
    rebase_repeat_y_across_raster_tiles: bool,
    coverage_repeats_x: bool,
    coverage_repeats_y: bool,
    repeat_origin: Option<(f32, f32)>,
    replaced: bool,
    replaced_mip_decoded: bool,
    full_precision_sampling: bool,
    close_exact_phase_to_predecessor: bool,
    optimize_single_tile: bool,
    floor_color_coverage: bool,
    coverage_bounds: Option<Rect>,
) -> Result<(Image, Rect, Option<(Rect, Rect)>), String> {
    physical_quantized_image_patch_with_color_order(
        image,
        source,
        destination,
        device_scale,
        wrap_x,
        wrap_y,
        rebase_repeat_x_across_raster_tiles,
        rebase_repeat_y_across_raster_tiles,
        coverage_repeats_x,
        coverage_repeats_y,
        repeat_origin,
        replaced,
        replaced_mip_decoded,
        full_precision_sampling,
        close_exact_phase_to_predecessor,
        optimize_single_tile,
        floor_color_coverage,
        coverage_bounds,
        true,
        false,
    )
}

/// Resolve the broken-image slot with Chromium's legacy
/// software image coordinate path. Its first sample is mapped through a f32
/// inverse matrix at each analytic coverage span, then subsequent X samples
/// advance in signed 32.32 fixed point. Coverage is applied by the draw, so
/// the returned texels keep their sampled premultiplied colors.
pub fn physical_quantized_broken_image_patch(
    image: &Image,
    source: Rect,
    destination: Rect,
    device_scale: f32,
) -> Result<(Image, Rect, Option<(Rect, Rect)>), String> {
    physical_quantized_broken_image_patch_with_clip(image, source, destination, device_scale, None)
}

pub(crate) fn physical_quantized_broken_image_patch_with_clip(
    image: &Image,
    source: Rect,
    destination: Rect,
    device_scale: f32,
    clip: Option<Rect>,
) -> Result<(Image, Rect, Option<(Rect, Rect)>), String> {
    physical_quantized_image_patch_with_color_order(
        image,
        source,
        destination,
        device_scale,
        false,
        false,
        false,
        false,
        false,
        false,
        None,
        false,
        false,
        false,
        false,
        false,
        true,
        clip,
        true,
        true,
    )
}

/// Return only the two-axis corner samples of a color-managed replaced image.
///
/// The companion `physical_quantized_image_patch` owns the interior and the
/// one-axis edges. Drawing this patch immediately afterward supplies the
/// profile-space corner samples without overlapping either pass.
pub fn physical_quantized_replaced_profile_corners(
    image: &Image,
    source: Rect,
    destination: Rect,
    device_scale: f32,
    replaced_mip_decoded: bool,
    full_precision_sampling: bool,
    coverage_bounds: Option<Rect>,
) -> Result<(Image, Rect), String> {
    let (patch, aligned_destination, _) = physical_quantized_image_patch_with_color_order(
        image,
        source,
        destination,
        device_scale,
        false,
        false,
        false,
        false,
        false,
        false,
        None,
        true,
        replaced_mip_decoded,
        full_precision_sampling,
        false,
        false,
        false,
        coverage_bounds,
        false,
        false,
    )?;
    Ok((patch, aligned_destination))
}

#[derive(Clone, Copy)]
struct LegacyFixedImageAxis {
    inverse_scale: f32,
    inverse_translation: f32,
    step_fixed: i64,
}

impl LegacyFixedImageAxis {
    const FIXED_ONE: f64 = 4_294_967_296.0;

    fn new(source_start: f32, source_extent: f32, device_start: f32, device_extent: f32) -> Self {
        let forward_scale = device_extent / source_extent;
        let inverse_scale = (1.0 / f64::from(forward_scale)) as f32;
        let inverse_translation =
            (f64::from(source_start) - f64::from(device_start) * f64::from(inverse_scale)) as f32;
        let step_fixed = (inverse_scale * Self::FIXED_ONE as f32) as i64;
        Self {
            inverse_scale,
            inverse_translation,
            step_fixed,
        }
    }

    fn first_fixed(self, device_pixel: i32) -> i64 {
        let center = device_pixel as f32 + 0.5;
        let mapped = center * self.inverse_scale + self.inverse_translation;
        (mapped * Self::FIXED_ONE as f32) as i64 - (1_i64 << 31)
    }

    fn sample(self, device_pixel: i32) -> f64 {
        self.first_fixed(device_pixel) as f64 / Self::FIXED_ONE
    }

    fn sample_from_first(self, first_device_pixel: i32, index: i32) -> f64 {
        (self.first_fixed(first_device_pixel) + self.step_fixed * i64::from(index)) as f64
            / Self::FIXED_ONE
    }
}

#[allow(clippy::too_many_arguments)]
fn physical_quantized_image_patch_with_color_order(
    image: &Image,
    source: Rect,
    destination: Rect,
    device_scale: f32,
    wrap_x: bool,
    wrap_y: bool,
    rebase_repeat_x_across_raster_tiles: bool,
    rebase_repeat_y_across_raster_tiles: bool,
    coverage_repeats_x: bool,
    coverage_repeats_y: bool,
    repeat_origin: Option<(f32, f32)>,
    replaced: bool,
    _replaced_mip_decoded: bool,
    full_precision_sampling: bool,
    close_exact_phase_to_predecessor: bool,
    optimize_single_tile: bool,
    floor_color_coverage: bool,
    coverage_bounds: Option<Rect>,
    convert_before_coverage: bool,
    legacy_float_matrix: bool,
) -> Result<(Image, Rect, Option<(Rect, Rect)>), String> {
    if !device_scale.is_finite()
        || device_scale <= 0.0
        || source.width() <= 0.0
        || source.height() <= 0.0
        || destination.width() <= 0.0
        || destination.height() <= 0.0
    {
        return Err("invalid physical image patch dimensions".to_string());
    }

    let (sampling_source, sampling_destination, sampling_coverage_bounds, single_tile_subset) =
        if optimize_single_tile && !replaced && !coverage_repeats_x && !coverage_repeats_y {
            coverage_bounds
                .and_then(|clip| blink_single_tile_subset(source, destination, clip))
                .map_or(
                    (source, destination, coverage_bounds, false),
                    |(subset, snapped)| (subset, snapped, Some(snapped), true),
                )
        } else {
            (source, destination, coverage_bounds, false)
        };

    let physical_destination = Rect::from_ltrb(
        sampling_destination.left * device_scale,
        sampling_destination.top * device_scale,
        sampling_destination.right * device_scale,
        sampling_destination.bottom * device_scale,
    );
    let physical_coverage_bounds = sampling_coverage_bounds.map(|bounds| {
        Rect::from_ltrb(
            bounds.left * device_scale,
            bounds.top * device_scale,
            bounds.right * device_scale,
            bounds.bottom * device_scale,
        )
    });
    let physical_repeat_origin = repeat_origin.map(|(x, y)| (x * device_scale, y * device_scale));
    let (coverage_left, coverage_right) = physical_image_axis_coverage_bounds(
        physical_destination.left,
        physical_destination.right,
        physical_coverage_bounds.map(|bounds| (bounds.left, bounds.right)),
        coverage_repeats_x,
    );
    let (coverage_top, coverage_bottom) = physical_image_axis_coverage_bounds(
        physical_destination.top,
        physical_destination.bottom,
        physical_coverage_bounds.map(|bounds| (bounds.top, bounds.bottom)),
        coverage_repeats_y,
    );
    let repeats = coverage_repeats_x || coverage_repeats_y;
    let downscaled_x = physical_destination.width() + 1.0e-5 < source.width();
    let downscaled_y = physical_destination.height() + 1.0e-5 < source.height();
    let legacy_packed_coverage_x =
        (replaced && !full_precision_sampling) || coverage_repeats_x || downscaled_x;
    let legacy_packed_coverage_y =
        (replaced && !full_precision_sampling) || coverage_repeats_y || downscaled_y;
    let legacy_packed_coverage = legacy_packed_coverage_x || legacy_packed_coverage_y;
    let physical_left = physical_destination.left.floor() as i32;
    let physical_top = physical_destination.top.floor() as i32;
    let physical_right = physical_destination.right.ceil() as i32;
    let physical_bottom = physical_destination.bottom.ceil() as i32;
    let width = physical_right - physical_left;
    let height = physical_bottom - physical_top;
    if width <= 0 || height <= 0 {
        return Err("empty physical image patch".to_string());
    }

    let image_width = image.width();
    let image_height = image.height();
    // Physical patches are already-resolved destination texels. Convert
    // color-managed replaced resources to the destination sRGB space before
    // folding fractional edge coverage into their premultiplied channels.
    // Applying the ICC transform after premultiplication loses low non-zero
    // channels at half-covered edges, while opaque interior pixels happen to
    // survive and conceal the ordering error.
    let color_managed_replaced = replaced && image.color_space().is_some();
    let source_color_space = if color_managed_replaced && convert_before_coverage {
        Some(ColorSpace::new_srgb())
    } else if replaced {
        image.color_space()
    } else {
        None
    };
    let patch_color_space = source_color_space.clone();
    let source_info = ImageInfo::new_n32_premul((image_width, image_height), source_color_space);
    let source_row_bytes = image_width as usize * 4;
    let mut source_pixels = vec![0_u8; source_row_bytes * image_height as usize];
    if !image.read_pixels(
        &source_info,
        &mut source_pixels,
        source_row_bytes,
        (0, 0),
        CachingHint::Allow,
    ) {
        return Err("failed to read physical image patch".to_string());
    }

    // A clipped replaced image uses strict source-rectangle sampling. Without
    // shrinking the clamp domain together with the destination clip, the
    // bilinear footprint can read the first texel outside the visible part of
    // the object (most visibly where a clipped JPEG color quadrant begins).
    // Work in physical destination space so the source domain and the packed
    // coverage below consume the identical resolved clip.
    let strict_source_domain = (replaced && physical_coverage_bounds.is_some()).then(|| {
        let bounds = physical_coverage_bounds.expect("checked above");
        let visible_left = bounds.left.max(physical_destination.left);
        let visible_top = bounds.top.max(physical_destination.top);
        let visible_right = bounds.right.min(physical_destination.right);
        let visible_bottom = bounds.bottom.min(physical_destination.bottom);
        Rect::from_ltrb(
            sampling_source.left
                + (visible_left - physical_destination.left) / physical_destination.width()
                    * sampling_source.width(),
            sampling_source.top
                + (visible_top - physical_destination.top) / physical_destination.height()
                    * sampling_source.height(),
            sampling_source.left
                + (visible_right - physical_destination.left) / physical_destination.width()
                    * sampling_source.width(),
            sampling_source.top
                + (visible_bottom - physical_destination.top) / physical_destination.height()
                    * sampling_source.height(),
        )
    });
    // Blink draws a single background tile with DrawImage and its default
    // kClampToSourceRect, so sampling stays inside the visible subset.
    let strict_source_domain =
        strict_source_domain.or(single_tile_subset.then_some(sampling_source));
    let min_x = strict_source_domain
        .map_or(source.left, |domain| domain.left)
        .floor()
        .max(source.left.floor())
        .max(0.0) as i32;
    let max_x = (strict_source_domain
        .map_or(source.right, |domain| domain.right)
        .ceil() as i32
        - 1)
    .min(source.right.ceil() as i32 - 1)
    .min(image_width - 1);
    let min_y = strict_source_domain
        .map_or(source.top, |domain| domain.top)
        .floor()
        .max(source.top.floor())
        .max(0.0) as i32;
    let max_y = (strict_source_domain
        .map_or(source.bottom, |domain| domain.bottom)
        .ceil() as i32
        - 1)
    .min(source.bottom.ceil() as i32 - 1)
    .min(image_height - 1);
    let target_info = ImageInfo::new_n32_premul((width, height), patch_color_space.clone());
    let target_row_bytes = width as usize * 4;
    let mut target_pixels = vec![0_u8; target_row_bytes * height as usize];
    // Derive the mathematical sample phase in f64 before reducing it to
    // Ganesh's four fractional bits. Common exact ratios such as 60/53 can
    // fall just below a half in f32, changing a packed weight from 8 to 7.
    let inverse_scale_x =
        f64::from(sampling_source.width()) / f64::from(physical_destination.width());
    let inverse_scale_y =
        f64::from(sampling_source.height()) / f64::from(physical_destination.height());
    let legacy_x = legacy_float_matrix.then(|| {
        LegacyFixedImageAxis::new(
            sampling_source.left,
            sampling_source.width(),
            physical_destination.left,
            physical_destination.width(),
        )
    });
    let legacy_y = legacy_float_matrix.then(|| {
        LegacyFixedImageAxis::new(
            sampling_source.top,
            sampling_source.height(),
            physical_destination.top,
            physical_destination.height(),
        )
    });
    // An integral one-to-one image mapping is a texel copy. The predecessor
    // closure used by translated nine-slice/filter paths must not turn exact
    // source coordinates into a 15/16 bilinear blend in this case.
    let bit_preserving_x = !wrap_x
        && (physical_destination.width() - sampling_source.width()).abs() <= 1.0e-5
        && physical_destination.left.rem_euclid(1.0).abs() <= 1.0e-5
        && sampling_source.left.rem_euclid(1.0).abs() <= 1.0e-5;
    let bit_preserving_y = !wrap_y
        && (physical_destination.height() - sampling_source.height()).abs() <= 1.0e-5
        && physical_destination.top.rem_euclid(1.0).abs() <= 1.0e-5
        && sampling_source.top.rem_euclid(1.0).abs() <= 1.0e-5;
    let single_axis_background_repeat = !replaced && (coverage_repeats_x ^ coverage_repeats_y);
    // The caller draws this patch as an antialiased rect at the fractional
    // destination, so Skia applies the edge coverage. See
    // `replaced_image_patch_draws_as_skia_rect_fill`.
    let skia_rect_edges = replaced
        && !legacy_float_matrix
        && replaced_image_patch_draws_as_skia_rect_fill(
            full_precision_sampling,
            sampling_coverage_bounds,
        );
    for target_y in 0..height {
        let device_y = f64::from(physical_top + target_y) + 0.5;
        let source_y = if let Some(axis) = legacy_y {
            axis.sample(physical_top + target_y)
        } else if full_precision_sampling && single_axis_background_repeat {
            full_precision_repeated_source_coordinate(
                sampling_source.top,
                sampling_source.height(),
                physical_repeat_origin.map_or(physical_destination.top, |(_, y)| y),
                physical_destination.height(),
                physical_top + target_y,
            )
        } else {
            f64::from(sampling_source.top)
                + (device_y - f64::from(physical_destination.top)) * inverse_scale_y
                - 0.5
        };
        let replaced_fixed_y = (replaced && !full_precision_sampling).then(|| {
            replaced_sample_fixed_at(
                sampling_source.top,
                sampling_source.height(),
                sampling_destination.top,
                sampling_destination.height(),
                device_scale,
                physical_top + target_y,
                false,
                false,
            )
        });
        // Skia blits a partly covered top or bottom row in shorter runs, so
        // the horizontal scanline restarts more often there.
        let replaced_partial_row = anti_fill_rect_partial_row(
            sampling_destination.top * device_scale,
            sampling_destination.bottom * device_scale,
            physical_top + target_y,
        );
        let fill_partial_row =
            anti_fill_rect_partial_row(coverage_top, coverage_bottom, physical_top + target_y);
        let repeated_fixed_y = wrap_y.then(|| {
            repeated_subrect_sample_fixed_at(
                min_y,
                max_y - min_y + 1,
                physical_repeat_origin.map_or(physical_destination.top, |(_, y)| y),
                physical_destination.height(),
                physical_top + target_y,
                rebase_repeat_y_across_raster_tiles,
                None,
            )
        });
        let closes_y_to_predecessor = close_exact_phase_to_predecessor
            && !bit_preserving_y
            && !replaced
            && !wrap_y
            && sampling_source.top.abs() <= f32::EPSILON
            && (source_y - source_y.round()).abs() <= 1.0e-9;
        let raw_y0 = if full_precision_sampling {
            source_y.floor() as i32
        } else {
            replaced_fixed_y
                .map(|fixed| (fixed >> 32) as i32)
                .or_else(|| repeated_fixed_y.map(|(low, _, _)| low))
                .unwrap_or_else(|| source_y.floor() as i32 - i32::from(closes_y_to_predecessor))
        };
        let wy = replaced_fixed_y
            .map(|fixed| ((fixed >> 28) & 0xf) as u32)
            .or_else(|| repeated_fixed_y.map(|(_, _, weight)| weight))
            .unwrap_or_else(|| {
                if closes_y_to_predecessor {
                    15
                } else {
                    ((source_y - f64::from(raw_y0)) * 16.0)
                        .floor()
                        .clamp(0.0, 15.0) as u32
                }
            });
        let y0 = if full_precision_sampling && wrap_y {
            raw_y0.rem_euclid(image_height) as usize
        } else if full_precision_sampling {
            raw_y0.clamp(min_y, max_y) as usize
        } else if let Some((low, _, _)) = repeated_fixed_y {
            low as usize
        } else if wrap_y {
            raw_y0.rem_euclid(image_height) as usize
        } else {
            raw_y0.clamp(min_y, max_y) as usize
        };
        let y1 = if full_precision_sampling && wrap_y {
            (raw_y0 + 1).rem_euclid(image_height) as usize
        } else if full_precision_sampling {
            (raw_y0 + 1).clamp(min_y, max_y) as usize
        } else if let Some((_, high, _)) = repeated_fixed_y {
            high as usize
        } else if wrap_y {
            (raw_y0 + 1).rem_euclid(image_height) as usize
        } else {
            (raw_y0 + 1).clamp(min_y, max_y) as usize
        };

        for target_x in 0..width {
            let destination_x = physical_left + target_x;
            let destination_y = physical_top + target_y;
            let geometric_coverage = if single_tile_subset {
                // Blink keeps the sampled single-tile image opaque and lets
                // DrawImage's analytic destination edge apply coverage after
                // the shader. The caller replays this policy with the clip
                // returned below; baking coverage here changes integer
                // interpolation order at the leading and trailing pixels.
                255
            } else if skia_rect_edges {
                // Keep the full sample wherever Skia's rect fill touches.
                if skia_rect_axis_touches(destination_x, coverage_left, coverage_right)
                    && skia_rect_axis_touches(destination_y, coverage_top, coverage_bottom)
                {
                    255
                } else {
                    0
                }
            } else {
                combine_physical_axis_coverage(
                    physical_axis_coverage(
                        destination_x,
                        coverage_left,
                        coverage_right,
                        legacy_packed_coverage_x,
                    ),
                    physical_axis_coverage(
                        destination_y,
                        coverage_top,
                        coverage_bottom,
                        legacy_packed_coverage_y,
                    ),
                )
            };
            let device_x = f64::from(physical_left + target_x) + 0.5;
            let source_x = if let Some(axis) = legacy_x {
                let pixel = physical_left + target_x;
                let span_start = coverage_left.ceil() as i32;
                if pixel < span_start
                    || (coverage_right.fract() != 0.0 && pixel >= coverage_right.floor() as i32)
                {
                    axis.sample(pixel)
                } else {
                    axis.sample_from_first(span_start, pixel - span_start)
                }
            } else if full_precision_sampling && single_axis_background_repeat {
                full_precision_repeated_source_coordinate(
                    sampling_source.left,
                    sampling_source.width(),
                    physical_repeat_origin.map_or(physical_destination.left, |(x, _)| x),
                    physical_destination.width(),
                    physical_left + target_x,
                )
            } else {
                f64::from(sampling_source.left)
                    + (device_x - f64::from(physical_destination.left)) * inverse_scale_x
                    - 0.5
            };
            let replaced_fixed_x = (replaced && !full_precision_sampling).then(|| {
                replaced_sample_fixed_at(
                    sampling_source.left,
                    sampling_source.width(),
                    sampling_destination.left,
                    sampling_destination.width(),
                    device_scale,
                    physical_left + target_x,
                    true,
                    replaced_partial_row,
                )
            });
            let repeated_fixed_x = wrap_x.then(|| {
                // A clipped repeat fill advances each scanline from its span
                // start, which decides a half-texel tie by the rounding of
                // Skia's f32 inverse matrix.
                let scanline = (close_exact_phase_to_predecessor && coverage_bounds.is_some())
                    .then_some(LegacyRepeatScanline {
                        tile_start: repeat_origin.map_or(sampling_destination.left, |(x, _)| x),
                        tile_extent: sampling_destination.width(),
                        device_scale,
                        fill_start: coverage_left,
                        fill_end: coverage_right,
                        partial_row: fill_partial_row,
                    });
                repeated_subrect_sample_fixed_at(
                    min_x,
                    max_x - min_x + 1,
                    physical_repeat_origin.map_or(physical_destination.left, |(x, _)| x),
                    physical_destination.width(),
                    physical_left + target_x,
                    rebase_repeat_x_across_raster_tiles,
                    scanline,
                )
            });
            let closes_x_to_predecessor = close_exact_phase_to_predecessor
                && !bit_preserving_x
                && !replaced
                && !wrap_x
                && (source_x - source_x.round()).abs() <= 1.0e-9;
            let raw_x0 = if full_precision_sampling {
                source_x.floor() as i32
            } else {
                replaced_fixed_x
                    .map(|fixed| (fixed >> 32) as i32)
                    .or_else(|| repeated_fixed_x.map(|(low, _, _)| low))
                    .unwrap_or_else(|| source_x.floor() as i32 - i32::from(closes_x_to_predecessor))
            };
            let wx = replaced_fixed_x
                .map(|fixed| ((fixed >> 28) & 0xf) as u32)
                .or_else(|| repeated_fixed_x.map(|(_, _, weight)| weight))
                .unwrap_or_else(|| {
                    if closes_x_to_predecessor {
                        15
                    } else {
                        ((source_x - f64::from(raw_x0)) * 16.0)
                            .floor()
                            .clamp(0.0, 15.0) as u32
                    }
                });
            let x0 = if full_precision_sampling && wrap_x {
                raw_x0.rem_euclid(image_width) as usize
            } else if full_precision_sampling {
                raw_x0.clamp(min_x, max_x) as usize
            } else if let Some((low, _, _)) = repeated_fixed_x {
                low as usize
            } else if wrap_x {
                raw_x0.rem_euclid(image_width) as usize
            } else {
                raw_x0.clamp(min_x, max_x) as usize
            };
            let x1 = if full_precision_sampling && wrap_x {
                (raw_x0 + 1).rem_euclid(image_width) as usize
            } else if full_precision_sampling {
                (raw_x0 + 1).clamp(min_x, max_x) as usize
            } else if let Some((_, high, _)) = repeated_fixed_x {
                high as usize
            } else if wrap_x {
                (raw_x0 + 1).rem_euclid(image_width) as usize
            } else {
                (raw_x0 + 1).clamp(min_x, max_x) as usize
            };

            for channel in 0..4 {
                let at = |x: usize, y: usize| {
                    source_pixels[y * source_row_bytes + x * 4 + channel] as u32
                };
                let value = if full_precision_sampling {
                    // Eligible decoded-image paths retain the complete
                    // inverse-matrix phase through bilinear sampling. This is
                    // distinct from the four-bit phase used by packed palette,
                    // mip-backed, and adjusted-repeat draws.
                    let fx = (source_x - f64::from(raw_x0)).clamp(0.0, 1.0);
                    let fy = (source_y - f64::from(raw_y0)).clamp(0.0, 1.0);
                    let top = f64::from(at(x0, y0)) * (1.0 - fx) + f64::from(at(x1, y0)) * fx;
                    let bottom = f64::from(at(x0, y1)) * (1.0 - fx) + f64::from(at(x1, y1)) * fx;
                    let interpolated = top * (1.0 - fy) + bottom * fy;
                    let first_repeat_y = physical_repeat_origin.is_some_and(|(_, origin_y)| {
                        (f64::from(physical_destination.top) - f64::from(origin_y)).abs() <= 1.0e-9
                    });
                    let vertical_half_tie = !replaced
                        && !single_axis_background_repeat
                        && wrap_y
                        && first_repeat_y
                        && (interpolated.fract() - 0.5).abs() <= 1.0e-9
                        && (top - bottom).abs() > 1.0e-9;
                    if vertical_half_tie {
                        interpolated.floor() as u32
                    } else if replaced {
                        interpolated.round() as u32
                    } else {
                        let stable_interpolated = if (interpolated.fract() - 0.5).abs() <= 1.0e-9 {
                            interpolated.floor() + 0.5
                        } else {
                            interpolated
                        };
                        stable_interpolated.round_ties_even() as u32
                    }
                } else {
                    (at(x0, y0) * (16 - wx) * (16 - wy)
                        + at(x1, y0) * wx * (16 - wy)
                        + at(x0, y1) * (16 - wx) * wy
                        + at(x1, y1) * wx * wy)
                        >> 8
                };
                // Replaced images carry their outer analytic coverage in the
                // physical patch itself. Chromium's packed pipeline stores
                // color channels with nearest premultiplication while alpha
                // closes a non-empty fractional span upward. Replaying these
                // pixels without another AA contour avoids double coverage.
                let value = if skia_rect_edges {
                    if geometric_coverage == 0 {
                        0
                    } else {
                        value
                    }
                } else if !legacy_float_matrix
                    && (replaced || coverage_bounds.is_some())
                    && geometric_coverage < 255
                {
                    if channel == 3 {
                        ((value * geometric_coverage + 255) >> 8).min(255)
                    } else if !replaced
                        && coverage_bounds.is_some()
                        && (physical_destination.width() - physical_destination.width().round())
                            .abs()
                            > 1.0e-5
                        && raw_x0 == 0
                        && wx >= 8
                    {
                        (value * geometric_coverage + 127) / 255
                    } else {
                        let rounding_bias = if floor_color_coverage {
                            // Transparent native resources are premultiplied
                            // into the analytic edge mask with truncation.
                            // Their alpha still closes a non-empty span above,
                            // so composition over the canvas reconstructs the
                            // browser's one-step-darker resource contour.
                            0
                        } else if full_precision_sampling {
                            // Full-phase sampling premultiplies after color
                            // interpolation and closes every non-empty
                            // eight-bit product upward. The packed path uses
                            // geometric coverage itself as the product bias.
                            255
                        } else if close_exact_phase_to_predecessor && repeats && (wrap_x || wrap_y)
                        {
                            // Opaque-border bleed avoidance carries repeated
                            // bitmap color through the legacy packed clip
                            // ramp. Its low half retains one extra half-span
                            // closure; its high half uses the packed coverage
                            // itself as the product bias.
                            opaque_border_repeated_color_coverage_bias(geometric_coverage)
                        } else {
                            physical_color_coverage_rounding_bias(
                                coverage_bounds.is_some(),
                                legacy_packed_coverage,
                                repeats && (wrap_x || wrap_y),
                                replaced,
                                geometric_coverage,
                                target_x,
                                target_y,
                                raw_x0,
                                (physical_destination.width()
                                    - physical_destination.width().round())
                                .abs()
                                    > 1.0e-5,
                            )
                        };
                        (value * geometric_coverage + rounding_bias) >> 8
                    }
                } else {
                    value
                };
                target_pixels
                    [target_y as usize * target_row_bytes + target_x as usize * 4 + channel] =
                    value as u8;
            }
        }
    }

    if color_managed_replaced && !skia_rect_edges {
        // Chromium converts a color-managed replaced image before applying a
        // one-axis edge mask, but converts the already-premultiplied sample at
        // the intersection of two analytic edges. Keep those two color-space
        // operations in separate image draws so neither pass is interpreted
        // in the other's color space.
        for target_y in 0..height {
            let vertical_coverage = physical_axis_coverage(
                physical_top + target_y,
                coverage_top,
                coverage_bottom,
                legacy_packed_coverage_y,
            );
            for target_x in 0..width {
                let horizontal_coverage = physical_axis_coverage(
                    physical_left + target_x,
                    coverage_left,
                    coverage_right,
                    legacy_packed_coverage_x,
                );
                let is_corner = horizontal_coverage != 0
                    && horizontal_coverage != 255
                    && vertical_coverage != 0
                    && vertical_coverage != 255;
                if is_corner != convert_before_coverage {
                    continue;
                }
                let pixel = target_y as usize * target_row_bytes + target_x as usize * 4;
                target_pixels[pixel..pixel + 4].fill(0);
            }
        }
    }

    let patch = skia_safe::images::raster_from_data(
        &target_info,
        Data::new_copy(&target_pixels),
        target_row_bytes,
    )
    .ok_or_else(|| "failed to retain physical image patch pixels".to_string())?;
    let aligned_destination = Rect::from_ltrb(
        physical_left as f32 / device_scale,
        physical_top as f32 / device_scale,
        physical_right as f32 / device_scale,
        physical_bottom as f32 / device_scale,
    );
    Ok((
        patch,
        aligned_destination,
        single_tile_subset.then_some((sampling_source, sampling_destination)),
    ))
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
    paint_static_svg(surface.canvas(), svg, width as f32, height as f32, None);
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

fn paint_static_svg(
    canvas: &Canvas,
    svg: &StaticSvg,
    viewport_width: f32,
    viewport_height: f32,
    concrete_subset: Option<Rect>,
) {
    if let Some(color) = svg.background {
        let mut paint = Paint::default();
        paint.set_style(PaintStyle::Fill);
        paint.set_color(color);
        canvas.draw_rect(
            Rect::from_xywh(0.0, 0.0, viewport_width, viewport_height),
            &paint,
        );
    }

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
        if concrete_subset.is_some_and(|subset| {
            let stroke_outset = if shape.stroke_color.is_some() {
                shape.stroke_width.max(0.0) * scale * 0.5
            } else {
                0.0
            };
            let concrete_shape = Rect::from_ltrb(
                translate_x + shape.rect.left * scale - stroke_outset,
                translate_y + shape.rect.top * scale - stroke_outset,
                translate_x + shape.rect.right * scale + stroke_outset,
                translate_y + shape.rect.bottom * scale + stroke_outset,
            );
            concrete_shape.right <= subset.left
                || concrete_shape.left >= subset.right
                || concrete_shape.bottom <= subset.top
                || concrete_shape.top >= subset.bottom
        }) {
            continue;
        }
        let mut paint = Paint::default();
        paint.set_style(PaintStyle::Fill);
        paint.set_anti_alias(true);
        paint.set_color(shape.color);
        canvas.draw_rect(shape.rect, &paint);
        if let Some(stroke_color) = shape.stroke_color {
            if shape.stroke_width > 0.0 {
                paint.set_style(PaintStyle::Stroke);
                paint.set_stroke_width(shape.stroke_width);
                paint.set_anti_alias(false);
                paint.set_color(stroke_color);
                canvas.draw_rect(shape.rect, &paint);
            }
        }
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
    let background = svg_attribute(root, "style")
        .and_then(|style| svg_style_property(style, "background-color"))
        .or_else(|| {
            svg_attribute(root, "style").and_then(|style| svg_style_property(style, "background"))
        })
        .and_then(parse_svg_color);

    let root_fill = svg_attribute(root, "fill")
        .and_then(parse_svg_color)
        .or(Some(skia_safe::Color::BLACK));
    let percentage_width = view_box.map(|rect| rect.width()).or(width).unwrap_or(300.0);
    let percentage_height = view_box
        .map(|rect| rect.height())
        .or(height)
        .unwrap_or(150.0);
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
            .or_else(|| {
                svg_attribute(tag, "style").and_then(|style| svg_style_property(style, "fill"))
            })
            .map(parse_svg_color)
            .unwrap_or(inherited_fill);
        match name {
            "g" => fill_stack.push(fill),
            "rect" => {
                let Some(color) = fill else {
                    continue;
                };
                let x = svg_attribute(tag, "x")
                    .and_then(|value| parse_svg_number_or_percent(value, percentage_width))
                    .unwrap_or(0.0);
                let y = svg_attribute(tag, "y")
                    .and_then(|value| parse_svg_number_or_percent(value, percentage_height))
                    .unwrap_or(0.0);
                let Some(width) = svg_attribute(tag, "width")
                    .and_then(|value| parse_svg_number_or_percent(value, percentage_width))
                else {
                    continue;
                };
                let Some(height) = svg_attribute(tag, "height")
                    .and_then(|value| parse_svg_number_or_percent(value, percentage_height))
                else {
                    continue;
                };
                if width > 0.0 && height > 0.0 {
                    let stroke_color = svg_attribute(tag, "stroke")
                        .or_else(|| {
                            svg_attribute(tag, "style")
                                .and_then(|style| svg_style_property(style, "stroke"))
                        })
                        .and_then(parse_svg_color);
                    let stroke_width = svg_attribute(tag, "stroke-width")
                        .or_else(|| {
                            svg_attribute(tag, "style")
                                .and_then(|style| svg_style_property(style, "stroke-width"))
                        })
                        .and_then(parse_svg_number)
                        .unwrap_or(1.0);
                    shapes.push(StaticSvgShape {
                        rect: Rect::from_xywh(x, y, width, height),
                        color,
                        stroke_color,
                        stroke_width,
                    });
                }
            }
            "path" => {
                let (Some(color), Some(path)) = (fill, svg_attribute(tag, "d")) else {
                    continue;
                };
                shapes.extend(parse_axis_aligned_svg_path(path).into_iter().map(|rect| {
                    StaticSvgShape {
                        rect,
                        color,
                        stroke_color: None,
                        stroke_width: 0.0,
                    }
                }));
            }
            _ => {}
        }
    }
    if shapes.is_empty() && background.is_none() {
        return Err("no supported static shapes".to_string());
    }
    Ok(StaticSvg {
        width,
        height,
        view_box,
        background,
        shapes,
    })
}

fn svg_style_property<'a>(style: &'a str, requested: &str) -> Option<&'a str> {
    style.split(';').find_map(|declaration| {
        let (name, value) = declaration.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case(requested)
            .then_some(value.trim())
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

fn parse_svg_number_or_percent(value: &str, percentage_base: f32) -> Option<f32> {
    let value = value.trim();
    if let Some(percentage) = value.strip_suffix('%') {
        return percentage
            .trim()
            .parse::<f32>()
            .ok()
            .map(|number| number * percentage_base / 100.0);
    }
    parse_svg_number(value)
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
    let Ok(StyleValue::Color(color)) = parse_literal(StyleProperty::Color, value) else {
        // StaticSvg currently carries fixed paints only. Preserve the existing
        // none/currentColor/invalid context policy; adding resolution requires
        // a retained SVG paint context, not a fabricated fallback color.
        return None;
    };
    // StaticSvg stores packed 8-bit sRGB paints. Match pinned Skia's rounded
    // conversion instead of rust-skia Color4f::to_color's truncating helper.
    // Wider-gamut/relative/missing-channel colors are not represented here.
    let byte = |value: f32| (value * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
    Some(skia_safe::Color::from_argb(
        byte(color.a),
        byte(color.r),
        byte(color.g),
        byte(color.b),
    ))
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
    fn static_svg_fixed_paints_reuse_native_literal_colors() {
        for (literal, argb) in [
            ("#008080", 0xff008080),
            ("#088", 0xff008888),
            ("#00808080", 0x80008080),
            ("#0888", 0x88008888),
            ("teal", 0xff008080),
            ("RebeccaPurple", 0xff663399),
            ("transparent", 0x00000000),
            ("rgb(0,128,128)", 0xff008080),
            ("rgba(0 128 128 / .5)", 0x80008080),
            ("rgb(12.5 127.49 300 / .1)", 0x1a0d7fff),
        ] {
            assert_eq!(
                parse_svg_color(literal),
                Some(skia_safe::Color::from(argb)),
                "{literal}"
            );
        }
        for literal in [
            "none",
            "currentColor",
            "unknown-color",
            "rgb(1,2)",
            "color(display-p3 1 0 0)",
            "rgb(from teal r g b)",
        ] {
            assert_eq!(parse_svg_color(literal), None, "{literal}");
        }
    }

    #[test]
    fn static_svg_fixed_hex_paints_preserve_every_channel_byte() {
        for value in 0_u8..=255 {
            let literal = format!("#{value:02x}{value:02x}{value:02x}{value:02x}");
            assert_eq!(
                parse_svg_color(&literal),
                Some(skia_safe::Color::from_argb(value, value, value, value))
            );
        }
    }

    #[test]
    fn static_svg_hex_resource_decodes_unchanged_native_teal_pixels_and_cache() {
        const RESOURCE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1" fill="#008080"/></svg>"##;
        let mut doc = Document::new();
        let id = doc.register_image_resource(
            "native-teal.svg",
            "image/svg+xml",
            "809bab3d90547e75eda8d473aa72a4f4f20b9e23ea7885f2e900eb0cf61aa6de",
            RESOURCE.to_vec(),
        );
        let first = decode_image_resource(&doc, id).expect("generic hex SVG paint");
        let second = decode_image_resource(&doc, id).unwrap();
        assert_eq!((first.width(), first.height()), (1, 1));
        assert_eq!(first.unique_id(), second.unique_id());
        let info = ImageInfo::new((1, 1), ColorType::RGBA8888, AlphaType::Unpremul, None);
        let mut rgba = [0_u8; 4];
        assert!(first.read_pixels(&info, &mut rgba, 4, (0, 0), CachingHint::Allow));
        assert_eq!(rgba, [0, 128, 128, 255]);
    }

    #[test]
    fn static_svg_fixed_color_repair_keeps_existing_paint_context_selection() {
        let source = br#"<svg width="4" height="1"><rect width="1" height="1"/><rect x="1" width="1" height="1" fill="none"/><rect x="2" width="1" height="1" fill="currentColor"/><rect x="3" width="1" height="1" fill="invalid"/></svg>"#;
        let svg = parse_static_svg(source).unwrap();
        assert_eq!(svg.shapes.len(), 1);
        assert_eq!(svg.shapes[0].color, skia_safe::Color::BLACK);
        // These are existing decoder outcomes, not a Chromium conformance
        // assertion. Correct currentColor/default/invalid-declaration behavior
        // needs the separately required SVG paint context and cascade repair.
        let root_none = parse_static_svg(
            br#"<svg width="1" height="1" fill="none"><rect width="1" height="1"/></svg>"#,
        )
        .unwrap();
        assert_eq!(root_none.shapes[0].color, skia_safe::Color::BLACK);
    }

    fn png_color_chunks(chunks: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        for (kind, data) in chunks {
            bytes.extend_from_slice(&(data.len() as u32).to_be_bytes());
            let crc_start = bytes.len();
            bytes.extend_from_slice(*kind);
            bytes.extend_from_slice(data);
            let crc = png_chunk_crc32(&bytes[crc_start..]);
            bytes.extend_from_slice(&crc.to_be_bytes());
        }
        bytes
    }

    #[test]
    fn png_neutral_gamma_respects_color_profile_precedence() {
        let neutral = 45_455_u32.to_be_bytes();
        assert!(png_has_neutral_gamma_without_color_profile(
            &png_color_chunks(&[(b"gAMA", &neutral), (b"IDAT", &[]),])
        ));
        for kind in [b"cICP", b"iCCP", b"sRGB", b"cHRM"] {
            for chunks in [
                vec![(kind, &[][..]), (b"gAMA", &neutral[..])],
                vec![(b"gAMA", &neutral[..]), (kind, &[][..])],
            ] {
                assert!(!png_has_neutral_gamma_without_color_profile(
                    &png_color_chunks(&chunks)
                ));
            }
        }
        for gamma in [0_u32, 25_000, 100_000] {
            assert!(!png_has_neutral_gamma_without_color_profile(
                &png_color_chunks(&[(b"gAMA", &gamma.to_be_bytes()),])
            ));
        }
    }

    #[test]
    fn png_neutral_gamma_rejects_corrupt_or_truncated_metadata() {
        let bytes = png_color_chunks(&[(b"gAMA", &45_455_u32.to_be_bytes())]);
        for length in 0..bytes.len() {
            assert!(!png_has_neutral_gamma_without_color_profile(
                &bytes[..length]
            ));
        }
        let mut corrupt = bytes.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        assert!(!png_has_neutral_gamma_without_color_profile(&corrupt));
        let mut oversized = bytes;
        oversized[8..12].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(!png_has_neutral_gamma_without_color_profile(&oversized));
    }

    #[test]
    fn broken_image_fixed_matrix_preserves_fractional_filter_phases() {
        let weight = |coordinate: f64| ((coordinate.rem_euclid(1.0) * 16.0).floor()) as u32;
        let at_150 = LegacyFixedImageAxis::new(0.0, 14.0, 30.0, 24.0);
        assert_eq!(weight(at_150.sample_from_first(30, 1)), 5);
        assert_eq!(weight(at_150.sample_from_first(30, 4)), 1);
        assert_eq!(weight(at_150.sample_from_first(30, 7)), 13);

        let at_125 = LegacyFixedImageAxis::new(0.0, 14.0, 25.0, 20.0);
        assert_eq!(weight(at_125.sample_from_first(25, 7)), 12);
    }

    #[test]
    fn svg_outer_edge_shortfall_tracks_the_composed_f32_phase() {
        let edge =
            |device_scale| legacy_svg_outer_edge(70.0, 30.0, 70.0, 50.0, 100.0, device_scale);
        assert_eq!(edge(1.0), Some(120.0));
        assert_eq!(edge(2.0), Some(120.0));
        assert_eq!(edge(1.25), None);
        assert_eq!(edge(1.5), None);
        assert_eq!(edge(3.0), None);
    }

    #[test]
    fn missing_resource_is_an_error() {
        let doc = Document::new();
        assert!(decode_image_resource(&doc, ImageResourceId::new(9)).is_err());
    }

    #[test]
    fn decoded_rgba8_media_transport_is_strict_and_cacheable() {
        let mut doc = Document::new();
        let bytes = vec![
            0x4f, 0x55, 0x49, 0x52, 0x01, 0x00, 0x00, 0x00, // OUIR v1
            0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, // 2x1
            0x00, 0x7f, 0x00, 0xff, 0x12, 0x34, 0x56, 0xff,
        ];
        let id = doc.register_image_resource(
            "fixture.webm#first-frame",
            "image/x-openui-rgba8",
            "decoded-rgba8-fixture",
            bytes,
        );
        let first = decode_image_resource(&doc, id).expect("decode RGBA8 transport");
        let second = decode_image_resource(&doc, id).expect("reuse decoded RGBA8 transport");
        assert_eq!((first.width(), first.height()), (2, 1));
        assert_eq!(first.unique_id(), second.unique_id());

        let info = ImageInfo::new(
            (2, 1),
            ColorType::RGBA8888,
            AlphaType::Premul,
            Some(ColorSpace::new_srgb()),
        );
        let mut pixels = [0_u8; 8];
        assert!(first.read_pixels(&info, &mut pixels, 8, (0, 0), CachingHint::Allow));
        assert_eq!(pixels, [0x00, 0x7f, 0x00, 0xff, 0x12, 0x34, 0x56, 0xff]);

        let malformed = doc.register_image_resource(
            "truncated.webm#first-frame",
            "image/x-openui-rgba8",
            "truncated-rgba8-fixture",
            b"OUIR\x01\0\0\0\x02\0\0\0\x01\0\0\0\0\x7f\0\xff".to_vec(),
        );
        assert!(decode_image_resource(&doc, malformed).is_err());
    }

    #[test]
    fn constant_raster_color_requires_one_opaque_pixel() {
        let mut doc = Document::new();
        let green = doc.register_image_resource(
            "css-backgrounds/support/1x1-green.png",
            "image/png",
            "a236213916dd30bd771a233aa1d66381eabf335bf8885304b75a4e2e370d68ce",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../tools/accountability/data/wpt_assets/sp13p/1x1-green.png"
            ))
            .to_vec(),
        );
        assert_eq!(
            opaque_single_pixel_raster_color(&doc, green),
            Some(Color::GREEN)
        );

        let transparent = doc.register_image_resource(
            "transparent.rgba",
            "image/x-openui-rgba8",
            "transparent-one-pixel-fixture",
            vec![
                b'O', b'U', b'I', b'R', 1, 0, 0, 0, // OUIR v1
                1, 0, 0, 0, 1, 0, 0, 0, // 1x1
                0, 128, 0, 128,
            ],
        );
        assert_eq!(opaque_single_pixel_raster_color(&doc, transparent), None);

        let two_pixels = doc.register_image_resource(
            "two-pixels.rgba",
            "image/x-openui-rgba8",
            "two-pixel-fixture",
            vec![
                b'O', b'U', b'I', b'R', 1, 0, 0, 0, // OUIR v1
                2, 0, 0, 0, 1, 0, 0, 0, // 2x1
                0, 128, 0, 255, 0, 128, 0, 255,
            ],
        );
        assert_eq!(opaque_single_pixel_raster_color(&doc, two_pixels), None);
    }

    #[test]
    fn background_phase_precision_follows_encoded_sample_depth() {
        fn png_header(bit_depth: u8, color_type: u8) -> Vec<u8> {
            let mut bytes = vec![0_u8; 26];
            bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
            bytes[12..16].copy_from_slice(b"IHDR");
            bytes[24] = bit_depth;
            bytes[25] = color_type;
            bytes
        }

        let mut doc = Document::new();
        let low_bit = doc.register_image_resource(
            "palette-2bit.png",
            "image/png",
            "palette-2bit",
            png_header(2, 3),
        );
        let eight_bit = doc.register_image_resource(
            "palette-8bit.png",
            "image/png",
            "palette-8bit",
            png_header(8, 3),
        );
        let truecolor = doc.register_image_resource(
            "truecolor-8bit.png",
            "image/png",
            "truecolor-8bit",
            png_header(8, 2),
        );
        let jpeg = doc.register_image_resource(
            "photo.jpg",
            "image/jpeg",
            "photo-jpeg",
            vec![0xff, 0xd8, 0xff, 0xd9],
        );

        assert!(!retains_full_precision_background_phase(
            &doc, low_bit, 1.0, 1.0
        ));
        assert!(retains_full_precision_background_phase(
            &doc, eight_bit, 1.5, 2.0
        ));
        assert!(retains_full_precision_background_phase(
            &doc, truecolor, 1.0, 1.0
        ));
        assert!(!retains_full_precision_background_phase(
            &doc, truecolor, 1.25, 1.25
        ));
        assert!(retains_full_precision_background_phase(
            &doc, jpeg, 1.0, 1.0
        ));
    }

    #[test]
    fn repeated_sample_matches_normalized_skia_fixed_point_phases() {
        assert_eq!(repeated_sample_fixed_at(60, 131.0, 53.0, 157), (29, 30, 8));
        assert_eq!(repeated_sample_fixed_at(60, 196.5, 102.0, 281), (49, 50, 7));
        assert_eq!(repeated_sample_fixed_at(60, 393.0, 204.0, 401), (1, 2, 15));
        assert_eq!(repeated_sample_fixed_at(96, 40.0, 168.0, 204), (93, 94, 7));
        assert_eq!(repeated_sample_fixed_at(60, 393.0, 159.0, 790), (29, 30, 7));
    }

    #[test]
    fn full_precision_repeat_retains_global_fused_matrix_phase() {
        let first = full_precision_repeated_source_coordinate(0.0, 32.0, 21.0, 36.0, 34);
        let second = full_precision_repeated_source_coordinate(0.0, 32.0, 21.0, 36.0, 70);
        assert!(first > 11.5 && first < 11.500_01);
        assert_eq!(second, 43.5);
        assert_eq!(second.rem_euclid(32.0), 11.5);
    }

    #[test]
    fn repeated_sample_rebases_exact_paired_edge_phase_across_raster_tiles() {
        assert_eq!(repeated_sample_fixed_at(60, 196.5, 79.5, 196), (59, 0, 8));
        assert_eq!(repeated_sample_fixed_at(60, 196.5, 79.5, 355), (59, 0, 7));
    }

    #[test]
    fn repeated_scanline_closes_half_texel_with_its_f32_inverse() {
        // A 32px image in a 36px tile at CSS x 21. At 1.25x the f32 inverse
        // step accumulates below the half-texel tie at device x 37 and closes
        // it to 7/16. At 1.5x it accumulates above the tie at device x 58 and
        // keeps 8/16, as Chromium draws `background-repeat-round-1-ref`.
        let scanline = |device_scale: f32, fill_start: f32, fill_end: f32| LegacyRepeatScanline {
            tile_start: 21.0,
            tile_extent: 36.0,
            device_scale,
            fill_start,
            fill_end,
            partial_row: false,
        };
        assert_eq!(
            repeated_subrect_sample_fixed_at(
                0,
                32,
                26.25,
                45.0,
                37,
                false,
                Some(scanline(1.25, 25.0, 117.5)),
            ),
            (7, 8, 7)
        );
        assert_eq!(
            repeated_subrect_sample_fixed_at(0, 32, 26.25, 45.0, 37, false, None),
            (7, 8, 8)
        );
        assert_eq!(
            repeated_subrect_sample_fixed_at(
                0,
                32,
                31.5,
                54.0,
                58,
                false,
                Some(scanline(1.5, 30.0, 141.0)),
            ),
            (15, 16, 8)
        );
    }

    #[test]
    fn compositor_scanline_restarts_at_each_overlapping_tile_origin() {
        assert_eq!(compositor_raster_tile_origin(0), 0);
        assert_eq!(compositor_raster_tile_origin(254), 0);
        assert_eq!(compositor_raster_tile_origin(255), 254);
        assert_eq!(compositor_raster_tile_origin(508), 254);
        assert_eq!(compositor_raster_tile_origin(509), 508);
    }

    #[test]
    fn replaced_sample_follows_chromium_composed_matrix_and_shader_spans() {
        let weight = |device_scale: f32, source_extent, css_start, pixel| {
            let fixed = replaced_sample_fixed_at(
                0.0,
                source_extent,
                css_start,
                75.0,
                device_scale,
                pixel,
                true,
                false,
            );
            ((fixed >> 28) & 0xf) as u32
        };

        // A quarter-pixel origin starts its interior span at ceil(left), so an
        // exact half phase after several raster tiles still closes upward.
        assert_eq!(weight(1.25, 300.0, 293.0, 377), 8);
        // A 125px mip decode at 1.25x: 1.25 * (75 / 125) rounds above 0.75 in
        // f32, so the step falls short of 4/3 and an early tie closes
        // downward. A later tie has crossed it.
        assert_eq!(weight(1.25, 125.0, 1021.0, 1287), 7);
        assert_eq!(weight(1.25, 125.0, 1021.0, 1317), 8);
        // box-sizing-replaced-002 at 1.5x, a 250px source decoded at 125px
        // from x = 1876.5: Chrome closes the ties at 1885 and 1921 upward and
        // at 1930 and 1984 downward.
        assert_eq!(weight(1.5, 125.0, 1251.0, 1885), 8);
        assert_eq!(weight(1.5, 125.0, 1251.0, 1921), 8);
        assert_eq!(weight(1.5, 125.0, 1251.0, 1930), 7);
        assert_eq!(weight(1.5, 125.0, 1251.0, 1984), 7);
        // At 2x a 25px source drawn 6x from x = 504 enters the raster tile at
        // 508. The shader maps pixel 635, 127 pixels later, again, which
        // moves the exact 12/16 ties from 637 on to 11/16.
        assert_eq!(weight(2.0, 25.0, 252.0, 631), 12);
        assert_eq!(weight(2.0, 25.0, 252.0, 634), 4);
        assert_eq!(weight(2.0, 25.0, 252.0, 637), 11);
        assert_eq!(weight(2.0, 25.0, 252.0, 640), 3);
    }

    #[test]
    fn legacy_shader_spans_split_at_edges_tiles_and_buffer_chunks() {
        // Partly covered edge columns are their own spans.
        assert_eq!(legacy_shader_span_start(1876.5, 1989.0, 1876, false), 1876);
        assert_eq!(legacy_shader_span_start(1876.5, 1989.0, 1877, false), 1877);
        assert_eq!(legacy_shader_span_start(1876.5, 1989.0, 1988, false), 1877);
        assert_eq!(legacy_shader_span_start(10.25, 20.5, 20, false), 20);
        // The interior restarts at a raster tile origin and every 127 pixels.
        assert_eq!(legacy_shader_span_start(504.0, 654.0, 508, false), 504);
        assert_eq!(legacy_shader_span_start(504.0, 654.0, 509, false), 508);
        assert_eq!(legacy_shader_span_start(504.0, 654.0, 634, false), 508);
        assert_eq!(legacy_shader_span_start(504.0, 654.0, 635, false), 635);
        // A partly covered row reaches the shader in 100-pixel runs from the
        // interior start, or from the raster tile origin that clips it.
        assert_eq!(legacy_shader_span_start(339.75, 452.25, 439, true), 340);
        assert_eq!(legacy_shader_span_start(339.75, 452.25, 440, true), 440);
        assert_eq!(legacy_shader_span_start(339.75, 452.25, 451, true), 440);
        assert_eq!(legacy_shader_span_start(339.75, 452.25, 452, true), 452);
        assert_eq!(legacy_shader_span_start(339.75, 452.25, 339, true), 339);
        assert_eq!(legacy_shader_span_start(2309.5, 2422.5, 2409, true), 2310);
        assert_eq!(legacy_shader_span_start(2309.5, 2422.5, 2410, true), 2410);
        assert_eq!(legacy_shader_span_start(504.0, 654.0, 607, true), 508);
        assert_eq!(legacy_shader_span_start(504.0, 654.0, 608, true), 608);
    }

    #[test]
    fn anti_fill_rect_blits_only_fractional_edge_rows_as_scanlines() {
        // Fractional top and bottom edges are partly covered rows.
        assert!(anti_fill_rect_partial_row(60.75, 180.75, 60));
        assert!(!anti_fill_rect_partial_row(60.75, 180.75, 61));
        assert!(!anti_fill_rect_partial_row(60.75, 180.75, 179));
        assert!(anti_fill_rect_partial_row(60.75, 180.75, 180));
        // Integral edges are full rows of `blitRect`.
        assert!(!anti_fill_rect_partial_row(60.0, 180.0, 60));
        assert!(!anti_fill_rect_partial_row(60.0, 180.0, 179));
        assert!(!anti_fill_rect_partial_row(60.0, 180.0, 180));
        // A rect inside one device row is one scanline.
        assert!(anti_fill_rect_partial_row(60.0, 60.5, 60));
        assert!(anti_fill_rect_partial_row(60.0, 61.0, 60));
        assert!(!anti_fill_rect_partial_row(60.0, 61.0, 61));
    }

    #[test]
    fn replaced_corner_coverage_rounds_only_low_span_products() {
        assert_eq!(combine_physical_axis_coverage(255, 65), 65);
        assert_eq!(combine_physical_axis_coverage(65, 65), 17);
        assert_eq!(combine_physical_axis_coverage(65, 129), 33);
        assert_eq!(combine_physical_axis_coverage(129, 129), 65);
        assert_eq!(combine_physical_axis_coverage(193, 193), 145);
    }

    #[test]
    fn replaced_color_coverage_uses_packed_nearest_rounding() {
        for coverage in [1, 64, 127, 128, 129, 192, 254] {
            assert_eq!(
                physical_color_coverage_rounding_bias(
                    false, false, false, true, coverage, 0, 0, 0, false,
                ),
                coverage
            );
        }
        assert_eq!(
            physical_color_coverage_rounding_bias(false, false, false, false, 127, 0, 0, 0, false),
            127
        );
        assert_eq!(
            physical_color_coverage_rounding_bias(false, false, false, false, 128, 0, 0, 0, false),
            128
        );
        assert_eq!(
            physical_color_coverage_rounding_bias(true, false, false, false, 128, 0, 0, 0, false),
            255
        );
        assert_eq!(
            physical_color_coverage_rounding_bias(true, true, false, false, 128, 0, 0, 0, false),
            128
        );
        assert_eq!(
            physical_color_coverage_rounding_bias(true, true, true, false, 65, 1, 1, 1, false),
            127
        );
        assert_eq!(
            physical_color_coverage_rounding_bias(true, true, true, false, 193, 1, 1, 1, false),
            255
        );
    }

    #[test]
    fn opaque_border_repeat_clip_retains_legacy_color_closure() {
        assert_eq!(opaque_border_repeated_color_coverage_bias(65), 97);
        assert_eq!(opaque_border_repeated_color_coverage_bias(127), 190);
        assert_eq!(opaque_border_repeated_color_coverage_bias(128), 128);
        assert_eq!(opaque_border_repeated_color_coverage_bias(193), 193);
    }

    #[test]
    fn full_precision_replaced_patch_retains_phase_and_closes_corner_coverage() {
        let info = ImageInfo::new_n32_premul((2, 1), None);
        let image = skia_safe::images::raster_from_data(
            &info,
            Data::new_copy(&[
                254, 0, 0, 255, // first decoded texel
                1, 128, 0, 255, // second decoded texel
            ]),
            2 * 4,
        )
        .expect("retain tiny decoded image");
        let (patch, _, _) = physical_quantized_image_patch(
            &image,
            Rect::from_xywh(0.0, 0.0, 2.0, 1.0),
            Rect::from_xywh(0.0, 0.0, 2.5, 1.0),
            1.0,
            false,
            false,
            false,
            false,
            false,
            false,
            None,
            true,
            false,
            true,
            false,
            false,
            false,
            None,
        )
        .expect("pre-sample full-phase replacement");
        let mut pixels = vec![0_u8; patch.width() as usize * patch.height() as usize * 4];
        assert!(patch.read_pixels(
            &ImageInfo::new_n32_premul((patch.width(), patch.height()), None),
            &mut pixels,
            patch.width() as usize * 4,
            (0, 0),
            CachingHint::Allow,
        ));
        assert_eq!(&pixels[4..8], &[77, 90, 0, 255]);

        let one_pixel = skia_safe::images::raster_from_data(
            &ImageInfo::new_n32_premul((1, 1), None),
            Data::new_copy(&[254, 0, 0, 255]),
            4,
        )
        .expect("retain one-pixel decoded image");
        let (corner_patch, _, _) = physical_quantized_image_patch(
            &one_pixel,
            Rect::from_xywh(0.0, 0.0, 1.0, 1.0),
            Rect::from_xywh(0.25, 0.5, 1.0, 1.0),
            1.0,
            false,
            false,
            false,
            false,
            false,
            false,
            None,
            true,
            false,
            true,
            false,
            false,
            false,
            None,
        )
        .expect("pre-sample corner coverage");
        let mut corner_pixels =
            vec![0_u8; corner_patch.width() as usize * corner_patch.height() as usize * 4];
        assert!(corner_patch.read_pixels(
            &ImageInfo::new_n32_premul((corner_patch.width(), corner_patch.height()), None),
            &mut corner_pixels,
            corner_patch.width() as usize * 4,
            (0, 0),
            CachingHint::Allow,
        ));
        assert_eq!(&corner_pixels[..4], &[96, 0, 0, 96]);
    }

    #[test]
    fn single_tile_background_samples_only_its_visible_subset() {
        // Columns 0-2 are red, columns 3-15 blue. A 3px-wide clip at 1.5x
        // shows source columns [0, 3); the half-covered edge pixel centre
        // maps to x = 3.0, which DrawImage clamps to the subset's last texel.
        let mut bytes = Vec::new();
        for column in 0..16 {
            bytes.extend_from_slice(if column < 3 {
                &[0, 0, 255, 255]
            } else {
                &[255, 0, 0, 255]
            });
        }
        let image = skia_safe::images::raster_from_data(
            &ImageInfo::new_n32_premul((16, 1), None),
            Data::new_copy(&bytes),
            64,
        )
        .expect("retain decoded image");
        let (patch, aligned, analytic_clip) = physical_quantized_image_patch(
            &image,
            Rect::from_xywh(0.0, 0.0, 16.0, 1.0),
            Rect::from_xywh(20.0, 20.0, 16.0, 16.0),
            1.5,
            false,
            false,
            false,
            false,
            false,
            false,
            Some((20.0, 20.0)),
            false,
            false,
            false,
            true,
            true,
            false,
            Some(Rect::from_xywh(20.0, 20.0, 3.0, 16.0)),
        )
        .expect("single-tile patch");
        assert!(analytic_clip.is_some());
        let mut pixels = vec![0_u8; patch.width() as usize * patch.height() as usize * 4];
        assert!(patch.read_pixels(
            &ImageInfo::new_n32_premul((patch.width(), patch.height()), None),
            &mut pixels,
            patch.width() as usize * 4,
            (0, 0),
            CachingHint::Allow,
        ));
        let edge = (34 - (aligned.left * 1.5).round() as usize) * 4;
        let row = 8 * patch.width() as usize * 4;
        assert_eq!(pixels[row + edge], 0, "edge pixel mixes the hidden column");
        assert!(
            pixels[row + edge + 2] > 0,
            "edge pixel keeps the visible column"
        );
    }

    #[test]
    fn full_precision_repeated_background_retains_inverse_transform_phase() {
        let image = skia_safe::images::raster_from_data(
            &ImageInfo::new_n32_premul((2, 1), None),
            Data::new_copy(&[
                254, 0, 0, 255, // first decoded texel
                1, 128, 0, 255, // second decoded texel
            ]),
            2 * 4,
        )
        .expect("retain repeated-background sampling oracle");
        let sample = |full_precision| {
            let (patch, _, _) = physical_quantized_image_patch(
                &image,
                Rect::from_xywh(0.0, 0.0, 2.0, 1.0),
                Rect::from_xywh(0.0, 0.0, 2.5, 1.0),
                1.0,
                true,
                false,
                true,
                false,
                true,
                false,
                Some((0.0, 0.0)),
                false,
                false,
                full_precision,
                false,
                false,
                false,
                None,
            )
            .expect("pre-sample repeated background");
            let mut pixels = vec![0_u8; patch.width() as usize * patch.height() as usize * 4];
            assert!(patch.read_pixels(
                &ImageInfo::new_n32_premul((patch.width(), patch.height()), None),
                &mut pixels,
                patch.width() as usize * 4,
                (0, 0),
                CachingHint::Allow,
            ));
            pixels
        };

        let full_precision = sample(true);
        let packed = sample(false);
        assert_eq!(&full_precision[4..8], &[77, 90, 0, 255]);
        assert_ne!(full_precision, packed);
    }

    #[test]
    fn clipped_replaced_patch_uses_a_strict_visible_source_domain() {
        let image = skia_safe::images::raster_from_data(
            &ImageInfo::new_n32_premul((1, 2), None),
            Data::new_copy(&[
                0, 0, 254, 255, // visible texel
                254, 0, 0, 255, // first texel beyond the clip
            ]),
            4,
        )
        .expect("retain clipped sampling oracle");
        let (patch, _, _) = physical_quantized_image_patch(
            &image,
            Rect::from_xywh(0.0, 0.0, 1.0, 2.0),
            Rect::from_xywh(0.0, 0.0, 1.0, 4.0),
            1.25,
            false,
            false,
            false,
            false,
            false,
            false,
            None,
            true,
            false,
            true,
            false,
            false,
            false,
            Some(Rect::from_xywh(0.0, 0.0, 1.0, 2.0)),
        )
        .expect("pre-sample strictly clipped replacement");
        let row_bytes = patch.width() as usize * 4;
        let mut pixels = vec![0_u8; row_bytes * patch.height() as usize];
        assert!(patch.read_pixels(
            &ImageInfo::new_n32_premul((patch.width(), patch.height()), None),
            &mut pixels,
            row_bytes,
            (0, 0),
            CachingHint::Allow,
        ));
        assert_eq!(&pixels[row_bytes..row_bytes + 4], &[0, 0, 254, 255]);
    }

    fn unclipped_replaced_patch(image: &Image, destination: Rect) -> (Image, Rect) {
        let (patch, aligned, _) = physical_quantized_image_patch(
            image,
            Rect::from_xywh(0.0, 0.0, 15.0, 15.0),
            destination,
            1.25,
            false,
            false,
            false,
            false,
            false,
            false,
            None,
            true,
            false,
            false,
            false,
            false,
            false,
            None,
        )
        .expect("sample an unclipped replaced image");
        (patch, aligned)
    }

    fn solid_image(color: skia_safe::Color4f) -> Image {
        let mut surface = skia_safe::surfaces::raster_n32_premul((15, 15)).expect("swatch");
        surface.canvas().clear(color);
        surface.image_snapshot()
    }

    fn patch_texel(patch: &Image, x: usize) -> [u8; 4] {
        let mut pixels = vec![0_u8; patch.width() as usize * 4];
        assert!(patch.read_pixels(
            &ImageInfo::new(
                (patch.width(), 1),
                skia_safe::ColorType::RGBA8888,
                skia_safe::AlphaType::Premul,
                None,
            ),
            &mut pixels,
            patch.width() as usize * 4,
            (0, 0),
            CachingHint::Allow,
        ));
        pixels[x * 4..x * 4 + 4].try_into().unwrap()
    }

    /// Replay the painter's unclipped replaced image draw on a 1.25x canvas.
    fn draw_unclipped_replaced(canvas: &skia_safe::Canvas, image: &Image, destination: Rect) {
        let (patch, aligned) = unclipped_replaced_patch(image, destination);
        let mut paint = skia_safe::Paint::default();
        paint.set_shader(patch.to_shader(
            (skia_safe::TileMode::Clamp, skia_safe::TileMode::Clamp),
            skia_safe::SamplingOptions::from(skia_safe::FilterMode::Nearest),
            &skia_safe::Matrix::scale_translate(
                (
                    aligned.width() / patch.width() as f32,
                    aligned.height() / patch.height() as f32,
                ),
                (aligned.left, aligned.top),
            ),
        ));
        paint.set_anti_alias(true);
        canvas.draw_rect(destination, &paint);
    }

    fn device_pixel(surface: &mut skia_safe::Surface, x: i32) -> [u8; 4] {
        let mut pixels = [0_u8; 4];
        assert!(surface.read_pixels(
            &ImageInfo::new(
                (1, 1),
                skia_safe::ColorType::RGBA8888,
                skia_safe::AlphaType::Premul,
                None,
            ),
            &mut pixels,
            4,
            (x, 10),
        ));
        pixels
    }

    #[test]
    fn unclipped_replaced_image_patch_keeps_full_edge_samples() {
        // At 1.25x, CSS 69..82 is device 86.25..102.5. The rect fill owns
        // the coverage of pixels 86 and 102, so the patch keeps the full
        // sample there.
        assert!(replaced_image_patch_draws_as_skia_rect_fill(false, None));
        let destination = Rect::from_ltrb(69.0, 0.0, 82.0, 40.0);
        let (patch, _) = unclipped_replaced_patch(
            &solid_image(skia_safe::Color4f::new(1.0, 1.0, 1.0, 1.0)),
            destination,
        );
        assert_eq!(patch.width(), 17);
        assert_eq!(patch_texel(&patch, 0), [255, 255, 255, 255]);
        assert_eq!(patch_texel(&patch, 16), [255, 255, 255, 255]);
        let (patch, _) = unclipped_replaced_patch(
            &solid_image(skia_safe::Color4f::new(1.0, 1.0, 1.0, 128.0 / 255.0)),
            destination,
        );
        assert_eq!(patch_texel(&patch, 16), [128, 128, 128, 128]);
    }

    #[test]
    fn unclipped_replaced_image_edges_lerp_over_the_device_like_chromium() {
        // multicol-rule-color-001-ref at 1.25x: a blue image's right edge
        // covers the first quarter of gray device pixel 101. Chromium gives
        // (143, 143, 207); coverage baked into the patch gives (144, 144, 208).
        // The surfaces are taller than one row: a rect fill clipped to a
        // single scanline scales its edge coverage by 255/256.
        let mut surface = skia_safe::surfaces::raster_n32_premul((110, 20)).expect("device");
        surface
            .canvas()
            .clear(skia_safe::Color::from_rgb(192, 192, 192));
        surface.canvas().scale((1.25, 1.25));
        draw_unclipped_replaced(
            surface.canvas(),
            &solid_image(skia_safe::Color4f::new(0.0, 0.0, 1.0, 1.0)),
            Rect::from_ltrb(69.0, 0.0, 81.0, 40.0),
        );
        assert_eq!(device_pixel(&mut surface, 101), [143, 143, 207, 255]);

        // multicol-count-computed-003-ref at 1.25x: device pixel 156 is a
        // quarter blue, then three quarters yellow, over white. Chromium
        // gives (239, 239, 62); coverage baked into the patch gives
        // (238, 238, 62).
        let mut surface = skia_safe::surfaces::raster_n32_premul((200, 20)).expect("device");
        surface.canvas().clear(skia_safe::Color::WHITE);
        surface.canvas().scale((1.25, 1.25));
        draw_unclipped_replaced(
            surface.canvas(),
            &solid_image(skia_safe::Color4f::new(0.0, 0.0, 1.0, 1.0)),
            Rect::from_ltrb(120.0, 0.0, 125.0, 40.0),
        );
        draw_unclipped_replaced(
            surface.canvas(),
            &solid_image(skia_safe::Color4f::new(1.0, 1.0, 0.0, 1.0)),
            Rect::from_ltrb(125.0, 0.0, 160.0, 40.0),
        );
        assert_eq!(device_pixel(&mut surface, 156), [239, 239, 62, 255]);
    }

    #[test]
    fn transparent_resource_color_coverage_can_truncate_before_composition() {
        let image = skia_safe::images::raster_from_data(
            &ImageInfo::new_n32_premul((1, 1), None),
            Data::new_copy(&[163, 163, 163, 255]),
            4,
        )
        .expect("retain transparent-resource coverage oracle");
        let destination = Rect::from_xywh(0.0, 0.25, 1.0, 1.0);
        let (patch, _, _) = physical_quantized_image_patch(
            &image,
            Rect::from_xywh(0.0, 0.0, 1.0, 1.0),
            destination,
            1.0,
            false,
            false,
            false,
            false,
            false,
            false,
            None,
            false,
            false,
            false,
            false,
            false,
            true,
            Some(destination),
        )
        .expect("pre-sample transparent-resource coverage");
        let mut pixels = vec![0_u8; patch.width() as usize * patch.height() as usize * 4];
        assert!(patch.read_pixels(
            &ImageInfo::new_n32_premul((patch.width(), patch.height()), None),
            &mut pixels,
            patch.width() as usize * 4,
            (0, 0),
            CachingHint::Allow,
        ));
        assert_eq!(&pixels[..4], &[122, 122, 122, 192]);
    }

    #[test]
    fn non_repeating_image_edges_own_coverage_inside_a_wider_clip() {
        assert_eq!(
            physical_image_axis_coverage_bounds(87.5, 462.5, Some((87.5, 562.5)), false),
            (87.5, 462.5)
        );
        assert_eq!(
            physical_image_axis_coverage_bounds(87.5, 462.5, Some((87.5, 562.5)), true),
            (87.5, 562.5)
        );
        assert_eq!(
            physical_image_axis_coverage_bounds(80.0, 600.0, Some((87.5, 562.5)), false),
            (87.5, 562.5)
        );
        assert_eq!(physical_axis_coverage(87, 87.5, 462.5, false), 128);
        assert_eq!(physical_axis_coverage(503, 223.75, 503.75, false), 192);
        assert_eq!(physical_axis_coverage(87, 87.499, 462.5, false), 129);
        assert_eq!(physical_axis_coverage(87, 87.5, 462.5, true), 129);
    }

    #[test]
    fn single_tile_subset_uses_unsnapped_source_and_snapped_destination() {
        let source = Rect::from_xywh(0.0, 0.0, 300.0, 224.0);
        let destination = Rect::from_xywh(70.0, 179.0, 190.0, 141.859375);
        let clip = Rect::from_xywh(100.0, 209.0, 320.0, 240.0);
        let (subset, snapped) =
            blink_single_tile_subset(source, destination, clip).expect("visible subset");
        assert_eq!(snapped, Rect::from_ltrb(100.0, 209.0, 260.0, 321.0));
        assert!((subset.left - 47.36842).abs() < 1.0e-4);
        assert!((subset.top - 47.370857).abs() < 1.0e-4);
        assert!((subset.right - 300.0).abs() < 1.0e-4);
        assert!((subset.bottom - 224.0).abs() < 1.0e-4);
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

    #[test]
    fn profiled_srgb_png_decode_preserves_source_channels() {
        let mut doc = Document::new();
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/accountability/data/wpt_assets/sp13p/css3.png"
        ));
        let id = doc.register_image_resource(
            "css-backgrounds/support/css3.png",
            "image/png",
            "404cf10151727f8165e24ff2c964073511fb857ebbf9e4422f0572c7ddf141ef",
            bytes.to_vec(),
        );
        let image = decode_image_resource(&doc, id).expect("decode profiled PNG");
        let color_space = image.color_space().expect("embedded ICC color space");
        let mut device_pixels = [0_u8; 4];
        let mut srgb_pixels = [0_u8; 4];
        assert!(image.read_pixels(
            &ImageInfo::new_n32_premul((1, 1), None),
            &mut device_pixels,
            4,
            (10, 10),
            CachingHint::Allow,
        ));
        assert!(image.read_pixels(
            &ImageInfo::new_n32_premul((1, 1), Some(skia_safe::ColorSpace::new_srgb())),
            &mut srgb_pixels,
            4,
            (10, 10),
            CachingHint::Allow,
        ));
        assert!(color_space.is_srgb());
        assert_eq!(device_pixels, [48, 44, 70, 255]);
        assert_eq!(srgb_pixels, device_pixels);
    }

    #[test]
    fn physical_patch_uses_global_phase_and_packed_bilinear_truncation() {
        let mut doc = Document::new();
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/accountability/data/wpt_assets/sp13p/css3.png"
        ));
        let id = doc.register_image_resource(
            "css-backgrounds/support/css3.png",
            "image/png",
            "404cf10151727f8165e24ff2c964073511fb857ebbf9e4422f0572c7ddf141ef",
            bytes.to_vec(),
        );
        let image = decode_image_resource(&doc, id).expect("decode profiled PNG");
        let source = Rect::from_xywh(0.0, 0.0, 300.0, 224.0);
        let destination = Rect::from_xywh(70.0, 179.0, 300.0, 224.0);
        let (patch, aligned, analytic_clip) = physical_quantized_image_patch(
            &image,
            source,
            destination,
            1.25,
            false,
            false,
            false,
            false,
            false,
            false,
            None,
            false,
            false,
            false,
            false,
            true,
            false,
            None,
        )
        .expect("pre-sample physical image patch");
        assert_eq!((patch.width(), patch.height()), (376, 281));
        assert_eq!(aligned, Rect::from_ltrb(69.6, 178.4, 370.4, 403.2));
        assert_eq!(analytic_clip, None);

        let info = ImageInfo::new_n32_premul((patch.width(), patch.height()), patch.color_space());
        let row_bytes = patch.width() as usize * 4;
        let mut pixels = vec![0_u8; row_bytes * patch.height() as usize];
        assert!(patch.read_pixels(&info, &mut pixels, row_bytes, (0, 0), CachingHint::Allow,));
        let pixel = |global_x: usize, global_y: usize| {
            let local_x = global_x - 87;
            let local_y = global_y - 223;
            &pixels[(local_y * patch.width() as usize + local_x) * 4..][..4]
        };
        assert_eq!(pixel(100, 240), [47, 46, 70, 255]);
        assert_eq!(pixel(300, 400), [137, 122, 182, 255]);
    }

    #[test]
    fn direct_nine_slice_closes_exact_source_phase_to_predecessor() {
        let info = ImageInfo::new_n32_premul((3, 1), None);
        let image = skia_safe::images::raster_from_data(
            &info,
            Data::new_copy(&[
                0, 0, 0, 255, // predecessor
                16, 16, 16, 255, // exact source coordinate
                32, 32, 32, 255,
            ]),
            3 * 4,
        )
        .expect("retain tiny sampling oracle");
        let sample = |close_exact_phase_to_predecessor| {
            let (patch, _, _) = physical_quantized_image_patch(
                &image,
                Rect::from_xywh(0.0, 0.0, 3.0, 1.0),
                Rect::from_xywh(0.5, 0.0, 2.0, 1.0),
                1.0,
                false,
                false,
                false,
                false,
                false,
                false,
                None,
                false,
                false,
                false,
                close_exact_phase_to_predecessor,
                false,
                false,
                None,
            )
            .expect("pre-sample tiny nine-slice patch");
            let mut pixels = vec![0_u8; patch.width() as usize * 4];
            assert!(patch.read_pixels(
                &ImageInfo::new_n32_premul((patch.width(), patch.height()), None),
                &mut pixels,
                patch.width() as usize * 4,
                (0, 0),
                CachingHint::Allow,
            ));
            pixels[4]
        };

        assert_eq!(sample(false), 16);
        assert_eq!(sample(true), 15);
    }

    #[test]
    fn integral_one_to_one_patch_is_a_bit_preserving_copy() {
        let info = ImageInfo::new_n32_premul((3, 1), None);
        let source_pixels = [
            0, 0, 0, 255, // first texel
            16, 16, 16, 255, // middle texel
            32, 32, 32, 255, // last texel
        ];
        let image =
            skia_safe::images::raster_from_data(&info, Data::new_copy(&source_pixels), 3 * 4)
                .expect("retain tiny copy oracle");
        let (patch, aligned, _) = physical_quantized_image_patch(
            &image,
            Rect::from_xywh(0.0, 0.0, 3.0, 1.0),
            Rect::from_xywh(5.0, 7.0, 3.0, 1.0),
            1.0,
            false,
            false,
            false,
            false,
            false,
            false,
            None,
            false,
            false,
            false,
            true,
            false,
            false,
            None,
        )
        .expect("pre-sample integral one-to-one patch");
        assert_eq!(aligned, Rect::from_xywh(5.0, 7.0, 3.0, 1.0));
        let mut actual = [0_u8; 12];
        assert!(patch.read_pixels(&info, &mut actual, 3 * 4, (0, 0), CachingHint::Allow,));
        assert_eq!(actual, source_pixels);
    }
}
