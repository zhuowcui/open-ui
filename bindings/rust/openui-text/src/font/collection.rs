//! Document-owned application and system font collection.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use openui_style::{FontFamily, FontStyleEnum, GenericFontFamily};
use skia_safe::{FontMgr, Typeface};

use super::description::FontDescription;
use super::platform::FontPlatformData;

const MAX_FACE_BYTES: usize = 64 * 1024 * 1024;
const MAX_COLLECTION_BYTES: usize = 256 * 1024 * 1024;
const MAX_REGISTERED_FACES: usize = 512;
const MAX_INSTANCE_CACHE: usize = 256;

static NEXT_COLLECTION_ID: AtomicU64 = AtomicU64::new(1);

/// Stable identifier for an application-registered font face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontFaceHandle {
    collection_id: u64,
    face_id: u64,
}

impl FontFaceHandle {
    pub const fn collection_id(self) -> u64 {
        self.collection_id
    }

    pub const fn face_id(self) -> u64 {
        self.face_id
    }
}

/// Inclusive numeric range used by a font-face descriptor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontAxisRange {
    pub min: f32,
    pub max: f32,
}

impl FontAxisRange {
    pub const fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }
}

/// Style range advertised by an application face.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontStyleRange {
    Normal,
    Italic,
    Oblique(FontAxisRange),
}

/// Inclusive Unicode scalar range covered by an application face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontUnicodeRange {
    pub start: u32,
    pub end: u32,
}

/// Default OpenType feature applied by an application font face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontFeatureDefault {
    pub tag: [u8; 4],
    pub value: u32,
}

/// CSS font metric overrides. Values are multipliers of the used font size.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FontMetricOverrides {
    pub ascent: Option<f32>,
    pub descent: Option<f32>,
    pub line_gap: Option<f32>,
}

/// Validated application font-face metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct FontFaceDescriptor {
    pub family: String,
    pub face_index: u32,
    pub style: FontStyleRange,
    pub weight: FontAxisRange,
    pub stretch: FontAxisRange,
    pub unicode_ranges: Vec<FontUnicodeRange>,
    pub feature_defaults: Vec<FontFeatureDefault>,
    pub size_adjust: Option<f32>,
    pub metric_overrides: FontMetricOverrides,
}

impl FontFaceDescriptor {
    pub fn new(family: impl Into<String>) -> Self {
        Self {
            family: family.into(),
            face_index: 0,
            style: FontStyleRange::Normal,
            weight: FontAxisRange::new(400.0, 400.0),
            stretch: FontAxisRange::new(100.0, 100.0),
            unicode_ranges: Vec::new(),
            feature_defaults: Vec::new(),
            size_adjust: None,
            metric_overrides: FontMetricOverrides::default(),
        }
    }
}

/// Font container accepted by the registration API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontContainerFormat {
    Ttf,
    Otf,
    Collection,
    Woff,
    Woff2,
}

/// A queried immutable face record. Font bytes intentionally remain private.
#[derive(Debug, Clone, PartialEq)]
pub struct FontFaceInfo {
    pub handle: FontFaceHandle,
    pub descriptor: FontFaceDescriptor,
    pub format: FontContainerFormat,
    pub byte_length: usize,
    pub sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontCollectionError {
    EmptyFamily,
    InvalidDescriptor(&'static str),
    UnsupportedContainer,
    MalformedFont,
    InvalidFaceIndex,
    DuplicateDescriptor,
    InputTooLarge,
    CollectionLimit,
    AllocationFailed,
    WrongCollection,
    UnknownFace,
}

impl std::fmt::Display for FontCollectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyFamily => formatter.write_str("font family must not be empty"),
            Self::InvalidDescriptor(reason) => {
                write!(formatter, "invalid font descriptor: {reason}")
            }
            Self::UnsupportedContainer => formatter.write_str("unsupported font container"),
            Self::MalformedFont => formatter.write_str("font bytes are malformed"),
            Self::InvalidFaceIndex => formatter.write_str("font collection index is invalid"),
            Self::DuplicateDescriptor => {
                formatter.write_str("font descriptor is already registered")
            }
            Self::InputTooLarge => {
                formatter.write_str("font input exceeds the per-face byte limit")
            }
            Self::CollectionLimit => formatter.write_str("font collection resource limit reached"),
            Self::AllocationFailed => formatter.write_str("font registration allocation failed"),
            Self::WrongCollection => {
                formatter.write_str("font handle belongs to another collection")
            }
            Self::UnknownFace => formatter.write_str("font handle is not registered"),
        }
    }
}

impl std::error::Error for FontCollectionError {}

/// Cache and resource counters used by renderer accountability tests.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FontCollectionStats {
    pub generation: u64,
    pub registered_faces: usize,
    pub registered_bytes: usize,
    pub cached_instances: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

struct SendFontMgr(FontMgr);

// SAFETY: SkFontMgr is immutable after construction and uses atomic reference
// counting internally. Open UI never mutates the system manager.
unsafe impl Send for SendFontMgr {}
unsafe impl Sync for SendFontMgr {}

struct RegisteredFace {
    info: FontFaceInfo,
    bytes: Arc<[u8]>,
    typeface: Typeface,
    registration_order: u64,
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct FontInstanceKey {
    family: String,
    size_bits: u32,
    weight_bits: u32,
    stretch_bits: u32,
    style_tag: u8,
    oblique_angle_bits: u32,
    native_control_text: bool,
    embedded_document_text: bool,
    native_button_text_metrics: bool,
}

struct CachedInstance {
    data: Arc<FontPlatformData>,
    last_used: u64,
}

struct CollectionState {
    generation: u64,
    next_face_id: u64,
    clock: u64,
    registered_bytes: usize,
    faces: Vec<RegisteredFace>,
    instances: HashMap<FontInstanceKey, CachedInstance>,
    cache_hits: u64,
    cache_misses: u64,
}

/// Font registry and bounded instance cache owned by one document/engine.
pub struct FontCollection {
    id: u64,
    system: SendFontMgr,
    state: Mutex<CollectionState>,
}

impl std::fmt::Debug for FontCollection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FontCollection")
            .field("id", &self.id)
            .field("stats", &self.stats())
            .finish()
    }
}

impl FontCollection {
    /// Construct a production collection backed by installed system fonts.
    pub fn system() -> Arc<Self> {
        Arc::new(Self {
            id: NEXT_COLLECTION_ID.fetch_add(1, Ordering::Relaxed),
            system: SendFontMgr(FontMgr::default()),
            state: Mutex::new(CollectionState {
                generation: 1,
                next_face_id: 1,
                clock: 0,
                registered_bytes: 0,
                faces: Vec::new(),
                instances: HashMap::new(),
                cache_hits: 0,
                cache_misses: 0,
            }),
        })
    }

    /// Construct the explicit byte-pinned collection used by parity tests.
    pub fn deterministic_test() -> Arc<Self> {
        let collection = Self::system();
        for (family, weight, bytes) in TEST_FONTS {
            let mut descriptor = FontFaceDescriptor::new(*family);
            descriptor.weight = FontAxisRange::new(*weight, *weight);
            collection
                .register(Arc::<[u8]>::from(*bytes), descriptor)
                .expect("vendored renderer font must be valid");
        }
        collection
    }

    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Register immutable application bytes after validating metadata and face index.
    pub fn register(
        &self,
        bytes: Arc<[u8]>,
        descriptor: FontFaceDescriptor,
    ) -> Result<FontFaceHandle, FontCollectionError> {
        validate_descriptor(&descriptor)?;
        if bytes.len() > MAX_FACE_BYTES {
            return Err(FontCollectionError::InputTooLarge);
        }
        let format = detect_format(&bytes)?;
        if descriptor.face_index != 0 && format != FontContainerFormat::Collection {
            return Err(FontCollectionError::InvalidFaceIndex);
        }
        let typeface = self
            .system
            .0
            .new_from_data(bytes.as_ref(), Some(descriptor.face_index as usize))
            .ok_or_else(|| {
                if format == FontContainerFormat::Collection {
                    FontCollectionError::InvalidFaceIndex
                } else {
                    FontCollectionError::MalformedFont
                }
            })?;
        let hash = sha256(&bytes);
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.faces.len() >= MAX_REGISTERED_FACES
            || state
                .registered_bytes
                .checked_add(bytes.len())
                .is_none_or(|total| total > MAX_COLLECTION_BYTES)
        {
            return Err(FontCollectionError::CollectionLimit);
        }
        if state
            .faces
            .iter()
            .any(|face| face.info.descriptor == descriptor)
        {
            return Err(FontCollectionError::DuplicateDescriptor);
        }
        state
            .faces
            .try_reserve(1)
            .map_err(|_| FontCollectionError::AllocationFailed)?;
        let handle = FontFaceHandle {
            collection_id: self.id,
            face_id: state.next_face_id,
        };
        state.next_face_id = state.next_face_id.saturating_add(1);
        state.clock = state.clock.saturating_add(1);
        let registration_order = state.clock;
        state.registered_bytes += bytes.len();
        state.faces.push(RegisteredFace {
            info: FontFaceInfo {
                handle,
                descriptor,
                format,
                byte_length: bytes.len(),
                sha256: hash,
            },
            bytes,
            typeface,
            registration_order,
        });
        registry_changed(&mut state);
        Ok(handle)
    }

    pub fn unregister(&self, handle: FontFaceHandle) -> Result<(), FontCollectionError> {
        if handle.collection_id != self.id {
            return Err(FontCollectionError::WrongCollection);
        }
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let index = state
            .faces
            .iter()
            .position(|face| face.info.handle == handle)
            .ok_or(FontCollectionError::UnknownFace)?;
        let face = state.faces.remove(index);
        state.registered_bytes -= face.bytes.len();
        registry_changed(&mut state);
        Ok(())
    }

    pub fn query(&self, handle: FontFaceHandle) -> Result<FontFaceInfo, FontCollectionError> {
        if handle.collection_id != self.id {
            return Err(FontCollectionError::WrongCollection);
        }
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .faces
            .iter()
            .find(|face| face.info.handle == handle)
            .map(|face| face.info.clone())
            .ok_or(FontCollectionError::UnknownFace)
    }

    pub fn generation(&self) -> u64 {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .generation
    }

    pub fn stats(&self) -> FontCollectionStats {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        FontCollectionStats {
            generation: state.generation,
            registered_faces: state.faces.len(),
            registered_bytes: state.registered_bytes,
            cached_instances: state.instances.len(),
            cache_hits: state.cache_hits,
            cache_misses: state.cache_misses,
        }
    }

    pub fn clear_instances(&self) {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .instances
            .clear();
    }

    /// Immutable application bytes retained by submitted scenes.
    pub fn retained_face_bytes(&self) -> Arc<[Arc<[u8]>]> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .faces
            .iter()
            .map(|face| Arc::clone(&face.bytes))
            .collect::<Vec<_>>()
            .into()
    }

    pub(crate) fn resolve_family(
        &self,
        family_name: &str,
        description: &FontDescription,
    ) -> Option<Arc<FontPlatformData>> {
        let key = FontInstanceKey::new(family_name, description);
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.clock = state.clock.saturating_add(1);
        let clock = state.clock;
        if let Some(data) = state.instances.get_mut(&key).map(|cached| {
            cached.last_used = clock;
            Arc::clone(&cached.data)
        }) {
            state.cache_hits = state.cache_hits.saturating_add(1);
            return Some(data);
        }
        state.cache_misses = state.cache_misses.saturating_add(1);

        let requested = FontPlatformData::to_sk_font_style(
            description.weight.0,
            description.stretch.0,
            &description.style,
        );
        let typeface = best_application_face(&state.faces, family_name, description)
            .map(|face| face.typeface.clone())
            .or_else(|| {
                let mut family = self.system.0.match_family(family_name);
                let count = family.count();
                let exact = (0..count).find(|&index| family.style(index).0 == requested);
                exact
                    .and_then(|index| family.new_typeface(index))
                    .or_else(|| (count == 1).then(|| family.new_typeface(0)).flatten())
                    .or_else(|| family.match_style(requested))
            })?;
        let data = make_platform_data(typeface, description);
        insert_instance(&mut state, key, Arc::clone(&data), clock);
        Some(data)
    }

    pub(crate) fn fallback_for_character(
        &self,
        codepoint: char,
        description: &FontDescription,
    ) -> Option<Arc<FontPlatformData>> {
        let key = FontInstanceKey::new(&format!("\0fallback:{:X}", codepoint as u32), description);
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.clock = state.clock.saturating_add(1);
        let clock = state.clock;
        if let Some(data) = state.instances.get_mut(&key).map(|cached| {
            cached.last_used = clock;
            Arc::clone(&cached.data)
        }) {
            state.cache_hits = state.cache_hits.saturating_add(1);
            return Some(data);
        }
        state.cache_misses = state.cache_misses.saturating_add(1);
        let requested = FontPlatformData::to_sk_font_style(
            description.weight.0,
            description.stretch.0,
            &description.style,
        );
        let locales = description
            .locale
            .as_deref()
            .filter(|locale| !locale.is_empty())
            .map_or_else(|| vec!["und"], |locale| vec![locale]);
        let typeface = best_application_fallback(&state.faces, codepoint, description)
            .map(|face| face.typeface.clone())
            .or_else(|| {
                self.system.0.match_family_style_character(
                    "",
                    requested,
                    &locales,
                    codepoint as i32,
                )
            })?;
        let data = make_platform_data(typeface, description);
        insert_instance(&mut state, key, Arc::clone(&data), clock);
        Some(data)
    }
}

impl Default for FontCollection {
    fn default() -> Self {
        Self {
            id: NEXT_COLLECTION_ID.fetch_add(1, Ordering::Relaxed),
            system: SendFontMgr(FontMgr::default()),
            state: Mutex::new(CollectionState {
                generation: 1,
                next_face_id: 1,
                clock: 0,
                registered_bytes: 0,
                faces: Vec::new(),
                instances: HashMap::new(),
                cache_hits: 0,
                cache_misses: 0,
            }),
        }
    }
}

impl FontInstanceKey {
    fn new(family: &str, description: &FontDescription) -> Self {
        let (style_tag, oblique_angle_bits) = match description.style {
            FontStyleEnum::Normal => (0, 0),
            FontStyleEnum::Italic => (1, 0),
            FontStyleEnum::Oblique(angle) => (2, angle.to_bits()),
        };
        Self {
            family: family.to_ascii_lowercase(),
            size_bits: description.size.to_bits(),
            weight_bits: description.weight.0.to_bits(),
            stretch_bits: description.stretch.0.to_bits(),
            style_tag,
            oblique_angle_bits,
            native_control_text: description.native_control_text,
            embedded_document_text: description.embedded_document_text,
            native_button_text_metrics: description.native_button_text_metrics,
        }
    }
}

fn registry_changed(state: &mut CollectionState) {
    state.generation = state.generation.saturating_add(1);
    state.instances.clear();
}

fn insert_instance(
    state: &mut CollectionState,
    key: FontInstanceKey,
    data: Arc<FontPlatformData>,
    clock: u64,
) {
    if state.instances.len() >= MAX_INSTANCE_CACHE {
        if let Some(oldest) = state
            .instances
            .iter()
            .min_by_key(|(_, value)| value.last_used)
            .map(|(key, _)| key.clone())
        {
            state.instances.remove(&oldest);
        }
    }
    state.instances.insert(
        key,
        CachedInstance {
            data,
            last_used: clock,
        },
    );
}

fn best_application_face<'a>(
    faces: &'a [RegisteredFace],
    family: &str,
    description: &FontDescription,
) -> Option<&'a RegisteredFace> {
    faces
        .iter()
        .filter(|face| face.info.descriptor.family.eq_ignore_ascii_case(family))
        .min_by(|left, right| {
            face_score(&left.info.descriptor, description, left.registration_order).total_cmp(
                &face_score(
                    &right.info.descriptor,
                    description,
                    right.registration_order,
                ),
            )
        })
}

fn best_application_fallback<'a>(
    faces: &'a [RegisteredFace],
    character: char,
    description: &FontDescription,
) -> Option<&'a RegisteredFace> {
    faces
        .iter()
        .filter(|face| descriptor_covers(&face.info.descriptor, character))
        .filter(|face| face.typeface.unichar_to_glyph(character as i32) != 0)
        .min_by(|left, right| {
            face_score(&left.info.descriptor, description, left.registration_order).total_cmp(
                &face_score(
                    &right.info.descriptor,
                    description,
                    right.registration_order,
                ),
            )
        })
}

fn face_score(descriptor: &FontFaceDescriptor, description: &FontDescription, order: u64) -> f64 {
    let style = match (descriptor.style, description.style) {
        (FontStyleRange::Normal, FontStyleEnum::Normal)
        | (FontStyleRange::Italic, FontStyleEnum::Italic) => 0.0,
        (FontStyleRange::Oblique(range), FontStyleEnum::Oblique(angle)) => {
            f64::from(distance_to_range(angle, range))
        }
        (FontStyleRange::Italic, FontStyleEnum::Oblique(_))
        | (FontStyleRange::Oblique(_), FontStyleEnum::Italic) => 1.0,
        _ => 2.0,
    };
    let stretch = f64::from(distance_to_range(description.stretch.0, descriptor.stretch));
    let weight = f64::from(distance_to_range(description.weight.0, descriptor.weight));
    style * 1.0e15 + stretch * 1.0e10 + weight * 1.0e5 + order as f64
}

fn distance_to_range(value: f32, range: FontAxisRange) -> f32 {
    if value < range.min {
        range.min - value
    } else if value > range.max {
        value - range.max
    } else {
        0.0
    }
}

fn descriptor_covers(descriptor: &FontFaceDescriptor, character: char) -> bool {
    descriptor.unicode_ranges.is_empty()
        || descriptor
            .unicode_ranges
            .iter()
            .any(|range| range.start <= character as u32 && character as u32 <= range.end)
}

fn make_platform_data(typeface: Typeface, description: &FontDescription) -> Arc<FontPlatformData> {
    let oblique_angle = match description.style {
        FontStyleEnum::Italic
            if typeface.font_style().slant() == skia_safe::font_style::Slant::Upright =>
        {
            14.036_243
        }
        FontStyleEnum::Oblique(angle)
            if typeface.font_style().slant() == skia_safe::font_style::Slant::Upright =>
        {
            angle
        }
        _ => 0.0,
    };
    Arc::new(FontPlatformData::with_synthetic_styles_and_native_metrics(
        typeface,
        description.size,
        oblique_angle,
        skia_safe::font_style::Weight::from(description.weight.0 as i32),
        description.native_control_text,
        description.embedded_document_text,
        description.native_button_text_metrics,
    ))
}

fn validate_descriptor(descriptor: &FontFaceDescriptor) -> Result<(), FontCollectionError> {
    if descriptor.family.trim().is_empty()
        || descriptor.family.contains('\0')
        || descriptor.family.len() > 255
    {
        return Err(FontCollectionError::EmptyFamily);
    }
    validate_axis(descriptor.weight, 1.0, 1000.0, "weight range")?;
    validate_axis(descriptor.stretch, 1.0, 1000.0, "stretch range")?;
    if let FontStyleRange::Oblique(range) = descriptor.style {
        validate_axis(range, -90.0, 90.0, "oblique range")?;
    }
    for range in &descriptor.unicode_ranges {
        if range.start > range.end || range.end > 0x10_FFFF {
            return Err(FontCollectionError::InvalidDescriptor("Unicode range"));
        }
    }
    for feature in &descriptor.feature_defaults {
        if feature.tag.iter().any(|byte| !(0x20..=0x7e).contains(byte)) {
            return Err(FontCollectionError::InvalidDescriptor(
                "OpenType feature tag",
            ));
        }
    }
    if descriptor
        .size_adjust
        .is_some_and(|value| !value.is_finite() || value <= 0.0)
    {
        return Err(FontCollectionError::InvalidDescriptor("size adjustment"));
    }
    for value in [
        descriptor.metric_overrides.ascent,
        descriptor.metric_overrides.descent,
        descriptor.metric_overrides.line_gap,
    ]
    .into_iter()
    .flatten()
    {
        if !value.is_finite() || value < 0.0 {
            return Err(FontCollectionError::InvalidDescriptor("metric override"));
        }
    }
    Ok(())
}

fn validate_axis(
    range: FontAxisRange,
    minimum: f32,
    maximum: f32,
    name: &'static str,
) -> Result<(), FontCollectionError> {
    if !range.min.is_finite()
        || !range.max.is_finite()
        || range.min < minimum
        || range.max > maximum
        || range.min > range.max
    {
        return Err(FontCollectionError::InvalidDescriptor(name));
    }
    Ok(())
}

fn detect_format(bytes: &[u8]) -> Result<FontContainerFormat, FontCollectionError> {
    if bytes.len() < 12 {
        return Err(FontCollectionError::MalformedFont);
    }
    match &bytes[..4] {
        [0, 1, 0, 0] | b"true" => Ok(FontContainerFormat::Ttf),
        b"OTTO" => Ok(FontContainerFormat::Otf),
        b"ttcf" => Ok(FontContainerFormat::Collection),
        b"wOFF" => Ok(FontContainerFormat::Woff),
        b"wOF2" => Ok(FontContainerFormat::Woff2),
        _ => Err(FontCollectionError::UnsupportedContainer),
    }
}

/// Generic families are passed to Fontconfig/Skia rather than rewritten to a
/// bundled face in production.
pub(crate) fn generic_family_name(generic: GenericFontFamily) -> &'static str {
    match generic {
        GenericFontFamily::Serif => "serif",
        GenericFontFamily::SansSerif => "sans-serif",
        GenericFontFamily::Monospace => "monospace",
        GenericFontFamily::Cursive => "cursive",
        GenericFontFamily::Fantasy => "fantasy",
        GenericFontFamily::SystemUi => "system-ui",
        GenericFontFamily::UiSerif => "ui-serif",
        GenericFontFamily::UiSansSerif => "ui-sans-serif",
        GenericFontFamily::UiMonospace => "ui-monospace",
        GenericFontFamily::UiRounded => "ui-rounded",
        GenericFontFamily::Math => "math",
        GenericFontFamily::Emoji => "emoji",
        GenericFontFamily::FangSong => "fangsong",
        GenericFontFamily::None => "sans-serif",
    }
}

pub(crate) fn family_name(family: &FontFamily) -> &str {
    match family {
        FontFamily::Named(name) => name,
        FontFamily::Generic(generic) => generic_family_name(*generic),
    }
}

const TEST_FONTS: &[(&str, f32, &[u8])] = &[
    ("Ahem", 400.0, include_bytes!("../../fonts/Ahem.ttf")),
    (
        "Droid Sans Fallback",
        400.0,
        include_bytes!("../../fonts/DroidSansFallback-reduced.ttf"),
    ),
    (
        "Noto Sans Devanagari",
        400.0,
        include_bytes!("../../fonts/NotoSansDevanagari-Regular.ttf"),
    ),
    (
        "Noto Color Emoji",
        400.0,
        include_bytes!("../../fonts/NotoColorEmoji.ttf"),
    ),
    (
        "DejaVu Sans",
        400.0,
        include_bytes!("../../fonts/DejaVuSans.ttf"),
    ),
    (
        "DejaVu Sans",
        700.0,
        include_bytes!("../../fonts/DejaVuSans-Bold.ttf"),
    ),
    (
        "DejaVu Sans Mono",
        400.0,
        include_bytes!("../../fonts/DejaVuSansMono.ttf"),
    ),
    (
        "DejaVu Sans Mono",
        700.0,
        include_bytes!("../../fonts/DejaVuSansMono-Bold.ttf"),
    ),
    (
        "DejaVu Serif",
        400.0,
        include_bytes!("../../fonts/DejaVuSerif.ttf"),
    ),
    (
        "DejaVu Serif",
        700.0,
        include_bytes!("../../fonts/DejaVuSerif-Bold.ttf"),
    ),
    (
        "sans-serif",
        400.0,
        include_bytes!("../../fonts/DejaVuSans.ttf"),
    ),
    (
        "sans-serif",
        700.0,
        include_bytes!("../../fonts/DejaVuSans-Bold.ttf"),
    ),
    (
        "system-ui",
        400.0,
        include_bytes!("../../fonts/DejaVuSans.ttf"),
    ),
    (
        "ui-sans-serif",
        400.0,
        include_bytes!("../../fonts/DejaVuSans.ttf"),
    ),
    (
        "monospace",
        400.0,
        include_bytes!("../../fonts/DejaVuSansMono.ttf"),
    ),
    (
        "monospace",
        700.0,
        include_bytes!("../../fonts/DejaVuSansMono-Bold.ttf"),
    ),
    (
        "ui-monospace",
        400.0,
        include_bytes!("../../fonts/DejaVuSansMono.ttf"),
    ),
    (
        "serif",
        400.0,
        include_bytes!("../../fonts/DejaVuSerif.ttf"),
    ),
    (
        "serif",
        700.0,
        include_bytes!("../../fonts/DejaVuSerif-Bold.ttf"),
    ),
    (
        "ui-serif",
        400.0,
        include_bytes!("../../fonts/DejaVuSerif.ttf"),
    ),
];

// Small allocation-free SHA-256 implementation used for resource identity.
fn sha256(input: &[u8]) -> [u8; 32] {
    const INITIAL: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut state = INITIAL;
    let bit_length = (input.len() as u64).wrapping_mul(8);
    let mut offset = 0;
    while offset + 64 <= input.len() {
        sha256_block(&mut state, &input[offset..offset + 64], &K);
        offset += 64;
    }
    let mut tail = [0u8; 128];
    let remaining = &input[offset..];
    tail[..remaining.len()].copy_from_slice(remaining);
    tail[remaining.len()] = 0x80;
    let padded = if remaining.len() < 56 { 64 } else { 128 };
    tail[padded - 8..padded].copy_from_slice(&bit_length.to_be_bytes());
    sha256_block(&mut state, &tail[..64], &K);
    if padded == 128 {
        sha256_block(&mut state, &tail[64..], &K);
    }
    let mut output = [0; 32];
    for (chunk, word) in output.chunks_exact_mut(4).zip(state) {
        chunk.copy_from_slice(&word.to_be_bytes());
    }
    output
}

fn sha256_block(state: &mut [u32; 8], block: &[u8], constants: &[u32; 64]) {
    let mut schedule = [0u32; 64];
    for (index, chunk) in block.chunks_exact(4).take(16).enumerate() {
        schedule[index] = u32::from_be_bytes(chunk.try_into().unwrap());
    }
    for index in 16..64 {
        let s0 = schedule[index - 15].rotate_right(7)
            ^ schedule[index - 15].rotate_right(18)
            ^ (schedule[index - 15] >> 3);
        let s1 = schedule[index - 2].rotate_right(17)
            ^ schedule[index - 2].rotate_right(19)
            ^ (schedule[index - 2] >> 10);
        schedule[index] = schedule[index - 16]
            .wrapping_add(s0)
            .wrapping_add(schedule[index - 7])
            .wrapping_add(s1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for index in 0..64 {
        let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let choice = (e & f) ^ (!e & g);
        let temporary1 = h
            .wrapping_add(sum1)
            .wrapping_add(choice)
            .wrapping_add(constants[index])
            .wrapping_add(schedule[index]);
        let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let majority = (a & b) ^ (a & c) ^ (b & c);
        let temporary2 = sum0.wrapping_add(majority);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(temporary1);
        d = c;
        c = b;
        b = a;
        a = temporary1.wrapping_add(temporary2);
    }
    for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *slot = slot.wrapping_add(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_hash_is_sha256() {
        assert_eq!(
            sha256(b"abc"),
            [
                0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
                0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
                0xf2, 0x00, 0x15, 0xad,
            ]
        );
    }

    #[test]
    fn registration_is_owned_bounded_and_generation_checked() {
        let collection = FontCollection::system();
        let bytes: Arc<[u8]> = Arc::from(include_bytes!("../../fonts/Ahem.ttf").as_slice());
        let descriptor = FontFaceDescriptor::new("Application Ahem");
        let handle = collection
            .register(Arc::clone(&bytes), descriptor.clone())
            .unwrap();
        let info = collection.query(handle).unwrap();
        assert_eq!(info.descriptor, descriptor);
        assert_eq!(info.byte_length, bytes.len());
        assert_eq!(
            info.sha256,
            [
                0xb7, 0x19, 0xec, 0xb3, 0x1c, 0x5b, 0x21, 0xfc, 0x57, 0x3c, 0x03, 0xf6, 0x42, 0x1c,
                0x74, 0xac, 0x63, 0xc2, 0x71, 0xa5, 0xa3, 0xff, 0x84, 0x1e, 0x34, 0xf9, 0x70, 0x5f,
                0xb9, 0x4b, 0x84, 0x48,
            ]
        );
        assert_eq!(collection.stats().registered_faces, 1);
        assert_eq!(
            collection.register(bytes, descriptor),
            Err(FontCollectionError::DuplicateDescriptor)
        );
        let other = FontCollection::system();
        assert_eq!(
            other.unregister(handle),
            Err(FontCollectionError::WrongCollection)
        );
        let generation = collection.generation();
        collection.unregister(handle).unwrap();
        assert!(collection.generation() > generation);
        assert_eq!(
            collection.query(handle),
            Err(FontCollectionError::UnknownFace)
        );
    }

    #[test]
    fn malformed_inputs_and_descriptors_are_rejected() {
        let collection = FontCollection::system();
        assert_eq!(
            collection.register(
                Arc::from(b"not a font".as_slice()),
                FontFaceDescriptor::new("Bad")
            ),
            Err(FontCollectionError::MalformedFont)
        );
        let mut descriptor = FontFaceDescriptor::new("Bad Range");
        descriptor.weight = FontAxisRange::new(700.0, 400.0);
        assert!(matches!(
            collection.register(
                Arc::from(include_bytes!("../../fonts/Ahem.ttf").as_slice()),
                descriptor
            ),
            Err(FontCollectionError::InvalidDescriptor(_))
        ));

        let mut invalid_unicode = FontFaceDescriptor::new("Bad Unicode");
        invalid_unicode.unicode_ranges = vec![FontUnicodeRange {
            start: 0x11_0000,
            end: 0x11_0000,
        }];
        assert!(matches!(
            collection.register(
                Arc::from(include_bytes!("../../fonts/Ahem.ttf").as_slice()),
                invalid_unicode,
            ),
            Err(FontCollectionError::InvalidDescriptor(_))
        ));

        let mut invalid_feature = FontFaceDescriptor::new("Bad Feature");
        invalid_feature.feature_defaults = vec![FontFeatureDefault {
            tag: [0, b'e', b'r', b'n'],
            value: 1,
        }];
        assert!(matches!(
            collection.register(
                Arc::from(include_bytes!("../../fonts/Ahem.ttf").as_slice()),
                invalid_feature,
            ),
            Err(FontCollectionError::InvalidDescriptor(_))
        ));

        let bogus_collection: Arc<[u8]> = Arc::from(b"ttcf\0\x01\0\0\0\0\0\0".as_slice());
        assert_eq!(
            collection.register(bogus_collection, FontFaceDescriptor::new("Bad Collection"),),
            Err(FontCollectionError::InvalidFaceIndex)
        );
    }

    #[test]
    fn registered_faces_precede_installed_families_without_cross_collection_leakage() {
        let application = FontCollection::system();
        let descriptor = FontFaceDescriptor::new("sans-serif");
        application
            .register(
                Arc::from(include_bytes!("../../fonts/Ahem.ttf").as_slice()),
                descriptor,
            )
            .unwrap();
        let description = FontDescription::default();
        let app_face = application
            .resolve_family("sans-serif", &description)
            .unwrap();
        let isolated = FontCollection::system();
        let system_face = isolated.resolve_family("sans-serif", &description).unwrap();
        assert_ne!(
            app_face.typeface().unique_id(),
            system_face.typeface().unique_id()
        );
        assert_eq!(isolated.stats().registered_faces, 0);
    }

    #[test]
    fn fallback_instances_are_cached_and_registry_changes_evict_them() {
        let collection = FontCollection::system();
        let mut descriptor = FontFaceDescriptor::new("Application fallback");
        descriptor.unicode_ranges = vec![FontUnicodeRange {
            start: u32::from('A'),
            end: u32::from('A'),
        }];
        let handle = collection
            .register(
                Arc::from(include_bytes!("../../fonts/Ahem.ttf").as_slice()),
                descriptor,
            )
            .unwrap();
        let description = FontDescription::default();
        let first = collection
            .fallback_for_character('A', &description)
            .unwrap();
        let second = collection
            .fallback_for_character('A', &description)
            .unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(collection.stats().cache_hits, 1);
        assert_eq!(collection.stats().cache_misses, 1);
        collection.unregister(handle).unwrap();
        assert_eq!(collection.stats().cached_instances, 0);
    }

    #[test]
    fn deterministic_collection_owns_generic_aliases_explicitly() {
        let collection = FontCollection::deterministic_test();
        let face = collection
            .resolve_family("sans-serif", &FontDescription::default())
            .unwrap();
        assert_eq!(face.typeface().family_name(), "DejaVu Sans");
        assert_eq!(collection.stats().registered_faces, TEST_FONTS.len());
    }
}
