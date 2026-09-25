//! Document-owned application and system font collection.

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use openui_style::{
    FontFamily, FontOpticalSizing, FontPalette, FontSizeAdjust, FontStyleEnum, FontSynthesis,
    FontVariantEmoji, GenericFontFamily,
};
use skia_safe::{
    font_arguments::{variation_position::Coordinate, Palette, VariationPosition},
    Font as SkFont, FontArguments, FontMgr, Typeface,
};

use super::description::FontDescription;
use super::platform::{FontPlatformData, ResolvedFontConfiguration};
use crate::hyphenation::HyphenationRegistry;

const MAX_FACE_BYTES: usize = 64 * 1024 * 1024;
const MAX_COLLECTION_BYTES: usize = 256 * 1024 * 1024;
const MAX_REGISTERED_FACES: usize = 512;
const MAX_REGISTERED_PALETTES: usize = 512;
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

/// Base palette selected by an application `@font-palette-values` resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontPaletteBase {
    Normal,
    Light,
    Dark,
    Index(u16),
}

/// One CPAL entry replacement in an application palette resource.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontPaletteEntryOverride {
    pub index: u16,
    pub color: openui_style::Color,
}

/// Native equivalent of CSS `@font-palette-values`.
#[derive(Debug, Clone, PartialEq)]
pub struct FontPaletteValuesDescriptor {
    pub name: String,
    pub family: String,
    pub base_palette: FontPaletteBase,
    pub overrides: Vec<FontPaletteEntryOverride>,
}

/// Stable identifier for a registered palette resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontPaletteHandle {
    collection_id: u64,
    palette_id: u64,
}

impl FontPaletteHandle {
    pub const fn collection_id(self) -> u64 {
        self.collection_id
    }

    pub const fn palette_id(self) -> u64 {
        self.palette_id
    }
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
    UnknownPalette,
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
            Self::UnknownPalette => formatter.write_str("font palette handle is not registered"),
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

struct RegisteredPalette {
    handle: FontPaletteHandle,
    descriptor: FontPaletteValuesDescriptor,
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct FontInstanceKey {
    family: String,
    /// Collision-free, canonical encoding of every value that can affect
    /// face selection, instantiation, metrics, or raster policy.
    description: String,
}

struct CachedInstance {
    data: Arc<FontPlatformData>,
    last_used: u64,
}

struct CollectionState {
    generation: u64,
    next_face_id: u64,
    next_palette_id: u64,
    clock: u64,
    registered_bytes: usize,
    faces: Vec<RegisteredFace>,
    palettes: Vec<RegisteredPalette>,
    instances: HashMap<FontInstanceKey, CachedInstance>,
    cache_hits: u64,
    cache_misses: u64,
}

/// Font registry and bounded instance cache owned by one document/engine.
pub struct FontCollection {
    id: u64,
    system: SendFontMgr,
    hyphenation_registry: Arc<HyphenationRegistry>,
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
            hyphenation_registry: HyphenationRegistry::bundled(),
            state: Mutex::new(CollectionState {
                generation: 1,
                next_face_id: 1,
                next_palette_id: 1,
                clock: 0,
                registered_bytes: 0,
                faces: Vec::new(),
                palettes: Vec::new(),
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
            // A manifest-scoped raster profile can intentionally load an
            // older FreeType build than the one used to compile Skia. Such a
            // backend may reject a newer optional color-font table while
            // still accepting every face required by that profile. Keep the
            // explicit test collection usable and let its normal resolution
            // path fall through to system fallback; public registration still
            // returns the precise error to its caller.
            let _ = collection.register(Arc::<[u8]>::from(*bytes), descriptor);
        }
        collection
    }

    pub const fn id(&self) -> u64 {
        self.id
    }

    pub fn hyphenation_registry(&self) -> &Arc<HyphenationRegistry> {
        &self.hyphenation_registry
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

    pub fn register_palette_values(
        &self,
        descriptor: FontPaletteValuesDescriptor,
    ) -> Result<FontPaletteHandle, FontCollectionError> {
        if descriptor.name.trim().is_empty()
            || descriptor.family.trim().is_empty()
            || descriptor.name.contains('\0')
            || descriptor.family.contains('\0')
            || descriptor.overrides.len() > u16::MAX as usize
            || descriptor.overrides.iter().any(|entry| {
                [entry.color.r, entry.color.g, entry.color.b, entry.color.a]
                    .iter()
                    .any(|component| !component.is_finite() || !(0.0..=1.0).contains(component))
            })
        {
            return Err(FontCollectionError::InvalidDescriptor(
                "font palette values",
            ));
        }
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.palettes.iter().any(|palette| {
            palette.descriptor.name == descriptor.name
                && palette
                    .descriptor
                    .family
                    .eq_ignore_ascii_case(&descriptor.family)
        }) {
            return Err(FontCollectionError::DuplicateDescriptor);
        }
        if state.palettes.len() >= MAX_REGISTERED_PALETTES {
            return Err(FontCollectionError::CollectionLimit);
        }
        let handle = FontPaletteHandle {
            collection_id: self.id,
            palette_id: state.next_palette_id,
        };
        state.next_palette_id = state.next_palette_id.saturating_add(1);
        state
            .palettes
            .push(RegisteredPalette { handle, descriptor });
        registry_changed(&mut state);
        Ok(handle)
    }

    pub fn unregister_palette_values(
        &self,
        handle: FontPaletteHandle,
    ) -> Result<(), FontCollectionError> {
        if handle.collection_id != self.id {
            return Err(FontCollectionError::WrongCollection);
        }
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let index = state
            .palettes
            .iter()
            .position(|palette| palette.handle == handle)
            .ok_or(FontCollectionError::UnknownPalette)?;
        state.palettes.remove(index);
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
        let selected = best_application_face(&state.faces, family_name, description)
            .map(|face| (face.typeface.clone(), Some(face.info.descriptor.clone())))
            .or_else(|| {
                let mut family = self.system.0.match_family(family_name);
                let count = family.count();
                let exact = (0..count).find(|&index| family.style(index).0 == requested);
                exact
                    .and_then(|index| family.new_typeface(index))
                    .or_else(|| (count == 1).then(|| family.new_typeface(0)).flatten())
                    .or_else(|| family.match_style(requested))
                    .map(|typeface| (typeface, None))
            })?;
        let palette = resolve_palette(
            &state.palettes,
            &selected.0,
            selected.1.as_ref(),
            description,
        );
        let data = make_platform_data(
            selected.0,
            selected.1.as_ref(),
            palette.as_ref(),
            description,
        );
        insert_instance(&mut state, key, Arc::clone(&data), clock);
        Some(data)
    }

    pub(crate) fn fallback_for_character(
        &self,
        codepoint: char,
        description: &FontDescription,
    ) -> Option<Arc<FontPlatformData>> {
        let mut buffer = [0_u8; 4];
        self.fallback_for_text(codepoint.encode_utf8(&mut buffer), description)
    }

    /// Resolve one face for an entire extended grapheme cluster. Application
    /// faces are considered only through authored families and a candidate
    /// must cover every painted scalar, preventing combining marks and emoji
    /// sequences from being split across unrelated faces.
    pub(crate) fn fallback_for_text(
        &self,
        text: &str,
        description: &FontDescription,
    ) -> Option<Arc<FontPlatformData>> {
        let key = FontInstanceKey::new(&format!("\0fallback:{text}"), description);
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
        let painted: Vec<char> = text
            .chars()
            .filter(|character| glyph_required(*character))
            .collect();
        let selected = best_application_fallback(&state.faces, &painted, description)
            .map(|face| (face.typeface.clone(), Some(face.info.descriptor.clone())))
            .or_else(|| {
                let families: &[&str] = match description.variant_emoji {
                    FontVariantEmoji::Emoji => &["emoji", ""],
                    FontVariantEmoji::Text => &["sans-serif", ""],
                    FontVariantEmoji::Normal | FontVariantEmoji::Unicode => &[""],
                };
                families.iter().find_map(|family| {
                    painted.iter().find_map(|codepoint| {
                        self.system
                            .0
                            .match_family_style_character(
                                family,
                                requested,
                                &locales,
                                *codepoint as i32,
                            )
                            .filter(|typeface| {
                                painted.iter().all(|character| {
                                    typeface.unichar_to_glyph(*character as i32) != 0
                                })
                            })
                            .map(|typeface| (typeface, None))
                    })
                })
            })?;
        let palette = resolve_palette(
            &state.palettes,
            &selected.0,
            selected.1.as_ref(),
            description,
        );
        let data = make_platform_data(
            selected.0,
            selected.1.as_ref(),
            palette.as_ref(),
            description,
        );
        insert_instance(&mut state, key, Arc::clone(&data), clock);
        Some(data)
    }
}

impl Default for FontCollection {
    fn default() -> Self {
        Self {
            id: NEXT_COLLECTION_ID.fetch_add(1, Ordering::Relaxed),
            system: SendFontMgr(FontMgr::default()),
            hyphenation_registry: HyphenationRegistry::bundled(),
            state: Mutex::new(CollectionState {
                generation: 1,
                next_face_id: 1,
                next_palette_id: 1,
                clock: 0,
                registered_bytes: 0,
                faces: Vec::new(),
                palettes: Vec::new(),
                instances: HashMap::new(),
                cache_hits: 0,
                cache_misses: 0,
            }),
        }
    }
}

impl FontInstanceKey {
    fn new(family: &str, description: &FontDescription) -> Self {
        Self {
            family: family.to_ascii_lowercase(),
            // `FontDescription` is a closed typed structure. Its derived
            // representation includes enum payloads, float values, ordered
            // feature/axis lists, locale, palette, synthesis, and harness
            // policy, unlike the former seven-field cache key.
            description: format!("{description:?}"),
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
        .min_by_key(|face| face_rank(&face.info.descriptor, description, face.registration_order))
}

fn best_application_fallback<'a>(
    faces: &'a [RegisteredFace],
    characters: &[char],
    description: &FontDescription,
) -> Option<&'a RegisteredFace> {
    description.family.families.iter().find_map(|family| {
        let family = family_name(family);
        faces
            .iter()
            .filter(|face| face.info.descriptor.family.eq_ignore_ascii_case(family))
            .filter(|face| {
                characters.iter().all(|character| {
                    descriptor_covers(&face.info.descriptor, *character)
                        && face.typeface.unichar_to_glyph(*character as i32) != 0
                })
            })
            .min_by_key(|face| {
                face_rank(&face.info.descriptor, description, face.registration_order)
            })
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct FaceRank {
    style_class: u8,
    style_distance: u32,
    stretch_class: u8,
    stretch_distance: u32,
    weight_class: u8,
    weight_distance: u32,
    registration_order: u64,
}

fn face_rank(
    descriptor: &FontFaceDescriptor,
    description: &FontDescription,
    registration_order: u64,
) -> FaceRank {
    let (style_class, style_distance) = style_rank(descriptor.style, description.style);
    let (stretch_class, stretch_distance) = directional_axis_rank(
        descriptor.stretch,
        description.stretch.0,
        description.stretch.0 <= 100.0,
    );
    let (weight_class, weight_distance) = weight_rank(descriptor.weight, description.weight.0);
    FaceRank {
        style_class,
        style_distance,
        stretch_class,
        stretch_distance,
        weight_class,
        weight_distance,
        registration_order,
    }
}

fn style_rank(range: FontStyleRange, requested: FontStyleEnum) -> (u8, u32) {
    let rank = match requested {
        FontStyleEnum::Normal => match range {
            FontStyleRange::Normal => (0, 0.0),
            FontStyleRange::Oblique(angle) => (1, distance_to_range(0.0, angle)),
            FontStyleRange::Italic => (2, 0.0),
        },
        FontStyleEnum::Italic => match range {
            FontStyleRange::Italic => (0, 0.0),
            FontStyleRange::Oblique(angle) => (1, distance_to_range(14.0, angle)),
            FontStyleRange::Normal => (2, 0.0),
        },
        FontStyleEnum::Oblique(angle) => match range {
            FontStyleRange::Oblique(candidate) => (0, distance_to_range(angle, candidate)),
            FontStyleRange::Italic => (1, 0.0),
            FontStyleRange::Normal => (2, 0.0),
        },
    };
    (rank.0, quantize_distance(rank.1))
}

fn directional_axis_rank(range: FontAxisRange, requested: f32, prefer_lower: bool) -> (u8, u32) {
    if range.min <= requested && requested <= range.max {
        return (0, 0);
    }
    let candidate = if range.max < requested {
        range.max
    } else {
        range.min
    };
    let preferred_side = if prefer_lower {
        candidate < requested
    } else {
        candidate > requested
    };
    (
        if preferred_side { 1 } else { 2 },
        quantize_distance((candidate - requested).abs()),
    )
}

fn weight_rank(range: FontAxisRange, requested: f32) -> (u8, u32) {
    if range.min <= requested && requested <= range.max {
        return (0, 0);
    }
    let candidate = if range.max < requested {
        range.max
    } else {
        range.min
    };
    let class = if (400.0..=500.0).contains(&requested) {
        if candidate > requested && candidate <= 500.0 {
            1
        } else if candidate < requested {
            2
        } else {
            3
        }
    } else if requested < 400.0 {
        if candidate < requested {
            1
        } else {
            2
        }
    } else if candidate > requested {
        1
    } else {
        2
    };
    (class, quantize_distance((candidate - requested).abs()))
}

fn quantize_distance(value: f32) -> u32 {
    (value.abs() * 1024.0).round().min(u32::MAX as f32) as u32
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

fn make_platform_data(
    typeface: Typeface,
    descriptor: Option<&FontFaceDescriptor>,
    palette: Option<&ResolvedPalette>,
    description: &FontDescription,
) -> Arc<FontPlatformData> {
    let typeface = instantiate_typeface(typeface, palette, description);
    let size = adjusted_font_size(&typeface, descriptor, description);
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
    let configuration = ResolvedFontConfiguration {
        allow_synthetic_weight: description.font_synthesis_weight == FontSynthesis::Auto,
        allow_synthetic_style: description.font_synthesis_style == FontSynthesis::Auto,
        feature_defaults: descriptor
            .map(|descriptor| descriptor.feature_defaults.clone())
            .unwrap_or_default(),
        metric_overrides: descriptor
            .map(|descriptor| descriptor.metric_overrides)
            .unwrap_or_default(),
    };
    Arc::new(FontPlatformData::with_resolved_configuration(
        typeface,
        size,
        oblique_angle,
        skia_safe::font_style::Weight::from(description.weight.0 as i32),
        description.native_control_text,
        description.embedded_document_text,
        description.native_button_text_metrics,
        description.raster_configuration,
        description.device_scale_factor,
        configuration,
    ))
}

struct ResolvedPalette {
    base: FontPaletteBase,
    overrides: Vec<skia_safe::font_arguments::palette::Override>,
}

fn resolve_palette(
    palettes: &[RegisteredPalette],
    typeface: &Typeface,
    face: Option<&FontFaceDescriptor>,
    description: &FontDescription,
) -> Option<ResolvedPalette> {
    let FontPalette::Custom(name) = &description.palette else {
        return None;
    };
    let system_family;
    let family = if let Some(face) = face {
        face.family.as_str()
    } else {
        system_family = typeface.family_name();
        &system_family
    };
    let descriptor = palettes.iter().rev().find(|palette| {
        palette.descriptor.name == *name && palette.descriptor.family.eq_ignore_ascii_case(family)
    })?;
    Some(ResolvedPalette {
        base: descriptor.descriptor.base_palette,
        overrides: descriptor
            .descriptor
            .overrides
            .iter()
            .map(|entry| skia_safe::font_arguments::palette::Override {
                index: entry.index,
                color: skia_safe::Color::from_argb(
                    (entry.color.a * 255.0).round() as u8,
                    (entry.color.r * 255.0).round() as u8,
                    (entry.color.g * 255.0).round() as u8,
                    (entry.color.b * 255.0).round() as u8,
                ),
            })
            .collect(),
    })
}

fn instantiate_typeface(
    typeface: Typeface,
    resolved_palette: Option<&ResolvedPalette>,
    description: &FontDescription,
) -> Typeface {
    let mut axes = BTreeMap::<u32, f32>::new();
    axes.insert(u32::from_be_bytes(*b"wght"), description.weight.0);
    axes.insert(u32::from_be_bytes(*b"wdth"), description.stretch.0);
    match description.style {
        FontStyleEnum::Normal => {
            axes.insert(u32::from_be_bytes(*b"ital"), 0.0);
            axes.insert(u32::from_be_bytes(*b"slnt"), 0.0);
        }
        FontStyleEnum::Italic => {
            axes.insert(u32::from_be_bytes(*b"ital"), 1.0);
        }
        FontStyleEnum::Oblique(angle) => {
            axes.insert(u32::from_be_bytes(*b"ital"), 0.0);
            axes.insert(u32::from_be_bytes(*b"slnt"), -angle);
        }
    }
    if description.font_optical_sizing == FontOpticalSizing::Auto {
        axes.insert(u32::from_be_bytes(*b"opsz"), description.size);
    }
    // CSS `font-variation-settings` has precedence over every CSS-derived
    // registered axis, including optical sizing.
    for variation in &description.variation_settings {
        axes.insert(u32::from_be_bytes(variation.tag), variation.value);
    }

    let parameters = typeface.variation_design_parameters().unwrap_or_default();
    axes.retain(|tag, value| {
        let Some(parameter) = parameters.iter().find(|axis| *axis.tag == *tag) else {
            return false;
        };
        if !value.is_finite() {
            return false;
        }
        *value = value.clamp(parameter.min, parameter.max);
        true
    });

    if axes.is_empty()
        && resolved_palette.is_none()
        && matches!(
            description.palette,
            FontPalette::Normal | FontPalette::Custom(_)
        )
    {
        return typeface;
    }

    let coordinates: Vec<Coordinate> = axes
        .into_iter()
        .map(|(axis, value)| Coordinate {
            axis: axis.into(),
            value,
        })
        .collect();
    let arguments = FontArguments::new().set_variation_design_position(VariationPosition {
        coordinates: &coordinates,
    });
    let palette_index = resolved_palette.map_or_else(
        || select_palette_index(&typeface, &description.palette),
        |palette| match palette.base {
            FontPaletteBase::Normal => 0,
            FontPaletteBase::Light => cpal_palette_for_background(&typeface, 1).unwrap_or(0),
            FontPaletteBase::Dark => cpal_palette_for_background(&typeface, 2).unwrap_or(0),
            FontPaletteBase::Index(index) => i32::from(index),
        },
    );
    let arguments = arguments.set_palette(Palette {
        index: palette_index,
        overrides: resolved_palette.map_or(&[], |palette| palette.overrides.as_slice()),
    });
    typeface
        .clone_with_arguments(&arguments)
        .unwrap_or(typeface)
}

fn select_palette_index(typeface: &Typeface, palette: &FontPalette) -> i32 {
    match palette {
        FontPalette::Normal | FontPalette::Custom(_) => 0,
        FontPalette::Light => cpal_palette_for_background(typeface, 0x0000_0001).unwrap_or(0),
        FontPalette::Dark => cpal_palette_for_background(typeface, 0x0000_0002).unwrap_or(0),
    }
}

/// Read CPAL v1 palette-type flags. Skia then applies that palette uniformly
/// to COLR v0/v1; bitmap, sbix, and SVG faces continue through their native
/// Skia glyph painters without a separate renderer path.
fn cpal_palette_for_background(typeface: &Typeface, requested_flag: u32) -> Option<i32> {
    const CPAL: u32 = u32::from_be_bytes(*b"CPAL");
    let table = typeface.copy_table_data(CPAL)?;
    let data = table.as_bytes();
    if read_u16_be(data, 0)? < 1 {
        return None;
    }
    let palette_count = read_u16_be(data, 4)? as usize;
    let type_offset = read_u32_be(data, 12 + palette_count * 2)? as usize;
    (0..palette_count)
        .find(|index| {
            read_u32_be(data, type_offset + index * 4)
                .is_some_and(|flags| flags & requested_flag != 0)
        })
        .and_then(|index| i32::try_from(index).ok())
}

fn read_u16_be(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes([
        *data.get(offset)?,
        *data.get(offset + 1)?,
    ]))
}

fn read_u32_be(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *data.get(offset)?,
        *data.get(offset + 1)?,
        *data.get(offset + 2)?,
        *data.get(offset + 3)?,
    ]))
}

fn adjusted_font_size(
    typeface: &Typeface,
    descriptor: Option<&FontFaceDescriptor>,
    description: &FontDescription,
) -> f32 {
    let descriptor_scale = descriptor.and_then(|face| face.size_adjust).unwrap_or(1.0);
    let base_size = description.size * descriptor_scale;
    let requested_aspect = match description.size_adjust {
        FontSizeAdjust::None => return base_size,
        FontSizeAdjust::FromFont => match description.resolved_from_font_aspect {
            Some(value) => value,
            None => return base_size,
        },
        FontSizeAdjust::ExHeight(value)
        | FontSizeAdjust::CapHeight(value)
        | FontSizeAdjust::ChWidth(value)
        | FontSizeAdjust::IcWidth(value)
        | FontSizeAdjust::IcHeight(value) => value,
    };
    let font = SkFont::from_typeface(typeface, base_size);
    let (_, metrics) = font.metrics();
    let actual_aspect = match description.size_adjust {
        FontSizeAdjust::ExHeight(_) => metrics.x_height / base_size,
        FontSizeAdjust::CapHeight(_) => metrics.cap_height / base_size,
        FontSizeAdjust::ChWidth(_) => font.measure_str("0", None).0 / base_size,
        FontSizeAdjust::IcWidth(_) => font.measure_str("水", None).0 / base_size,
        FontSizeAdjust::IcHeight(_) => 1.0,
        FontSizeAdjust::FromFont => metrics.x_height / base_size,
        FontSizeAdjust::None => 1.0,
    };
    if actual_aspect.is_finite() && actual_aspect > 0.0 {
        base_size * requested_aspect / actual_aspect
    } else {
        base_size
    }
}

fn glyph_required(character: char) -> bool {
    !character.is_control()
        && !matches!(
            character as u32,
            0x200C..=0x200D | 0x202A..=0x202E | 0x2066..=0x2069 | 0xFE00..=0xFE0F
        )
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
    fn pinned_wpt_font_containers_report_identical_owned_metadata() {
        let fixtures: &[(&str, &[u8], FontContainerFormat)] = &[
            (
                "Certification TTF",
                include_bytes!("../../fonts/certification/fixture.ttf"),
                FontContainerFormat::Ttf,
            ),
            (
                "Certification OTF",
                include_bytes!("../../fonts/certification/fixture.otf"),
                FontContainerFormat::Otf,
            ),
            (
                "Certification WOFF",
                include_bytes!("../../fonts/certification/fixture.woff"),
                FontContainerFormat::Woff,
            ),
            (
                "Certification WOFF2",
                include_bytes!("../../fonts/certification/fixture.woff2"),
                FontContainerFormat::Woff2,
            ),
            (
                "Certification TTC",
                include_bytes!("../../fonts/certification/fixture.ttc"),
                FontContainerFormat::Collection,
            ),
            (
                "Certification OTC",
                include_bytes!("../../fonts/certification/fixture.otc"),
                FontContainerFormat::Collection,
            ),
        ];
        let collection = FontCollection::system();
        for (family, bytes, format) in fixtures {
            let descriptor = FontFaceDescriptor::new(*family);
            let owned: Arc<[u8]> = Arc::from(*bytes);
            let handle = collection
                .register(Arc::clone(&owned), descriptor.clone())
                .unwrap_or_else(|error| panic!("{family} registration failed: {error:?}"));
            let info = collection.query(handle).unwrap();
            assert_eq!(info.descriptor, descriptor);
            assert_eq!(info.descriptor.face_index, 0);
            assert_eq!(info.format, *format);
            assert_eq!(info.byte_length, bytes.len());
            assert_eq!(info.sha256, sha256(bytes));
            drop(owned);
            // Registration owns its bytes and remains queryable after the
            // caller releases the source buffer.
            assert_eq!(collection.query(handle).unwrap(), info);
            collection.unregister(handle).unwrap();
            assert_eq!(
                collection.query(handle),
                Err(FontCollectionError::UnknownFace)
            );
        }
    }

    #[test]
    fn font_container_signatures_do_not_bypass_decoder_validation() {
        let collection = FontCollection::system();
        for signature in [b"OTTO", b"ttcf", b"wOFF", b"wOF2"] {
            let mut truncated = vec![0_u8; 12];
            truncated[..4].copy_from_slice(signature);
            assert!(matches!(
                collection.register(
                    Arc::from(truncated),
                    FontFaceDescriptor::new(format!("Truncated {signature:?}")),
                ),
                Err(FontCollectionError::MalformedFont | FontCollectionError::InvalidFaceIndex)
            ));
        }
    }

    #[test]
    fn collection_face_indices_are_validated_for_ttc_and_otc() {
        let collection = FontCollection::system();
        let ttc = include_bytes!("../../fonts/certification/fixture.ttc");
        for face_index in 0..2 {
            let mut descriptor = FontFaceDescriptor::new(format!("TTC face {face_index}"));
            descriptor.face_index = face_index;
            let handle = collection
                .register(Arc::from(ttc.as_slice()), descriptor)
                .unwrap();
            assert_eq!(
                collection.query(handle).unwrap().descriptor.face_index,
                face_index
            );
        }
        let mut missing_ttc_face = FontFaceDescriptor::new("Missing TTC face");
        missing_ttc_face.face_index = 2;
        assert_eq!(
            collection.register(Arc::from(ttc.as_slice()), missing_ttc_face),
            Err(FontCollectionError::InvalidFaceIndex)
        );

        let otc = include_bytes!("../../fonts/certification/fixture.otc");
        let mut missing_otc_face = FontFaceDescriptor::new("Missing OTC face");
        missing_otc_face.face_index = 1;
        assert_eq!(
            collection.register(Arc::from(otc.as_slice()), missing_otc_face),
            Err(FontCollectionError::InvalidFaceIndex)
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

    #[test]
    fn deterministic_collection_tolerates_optional_backend_format_gaps() {
        // In particular, the pinned real-font raster profile uses a FreeType
        // build that predates one vendored color face. Construction itself is
        // required to remain infallible; required text faces still resolve.
        let collection = FontCollection::deterministic_test();
        assert!(collection
            .resolve_family("DejaVu Sans", &FontDescription::default())
            .is_some());
        assert!(collection
            .resolve_family("DejaVu Sans Mono", &FontDescription::default())
            .is_some());
    }

    #[test]
    fn css_face_matching_is_lexicographic_and_directional() {
        let mut description = FontDescription::default();
        description.weight.0 = 400.0;
        let weight_300 = FontFaceDescriptor {
            weight: FontAxisRange::new(300.0, 300.0),
            ..FontFaceDescriptor::new("Match")
        };
        let weight_500 = FontFaceDescriptor {
            weight: FontAxisRange::new(500.0, 500.0),
            ..FontFaceDescriptor::new("Match")
        };
        assert!(face_rank(&weight_500, &description, 2) < face_rank(&weight_300, &description, 1));

        description.stretch.0 = 80.0;
        let narrow = FontFaceDescriptor {
            stretch: FontAxisRange::new(75.0, 75.0),
            ..FontFaceDescriptor::new("Match")
        };
        let normal = FontFaceDescriptor {
            stretch: FontAxisRange::new(100.0, 100.0),
            ..FontFaceDescriptor::new("Match")
        };
        assert!(face_rank(&narrow, &description, 3) < face_rank(&normal, &description, 1));

        description.style = FontStyleEnum::Italic;
        let italic = FontFaceDescriptor {
            style: FontStyleRange::Italic,
            ..FontFaceDescriptor::new("Match")
        };
        assert!(face_rank(&italic, &description, 9) < face_rank(&narrow, &description, 1));
    }

    #[test]
    fn instance_key_includes_variations_palette_synthesis_scale_and_raster_policy() {
        let base = FontDescription::default();
        let mut varied = base.clone();
        varied.variation_settings.push(openui_style::FontVariation {
            tag: *b"wght",
            value: 625.0,
        });
        let mut palette = base.clone();
        palette.palette = FontPalette::Dark;
        let mut synthesis = base.clone();
        synthesis.font_synthesis_weight = FontSynthesis::None;
        let mut scaled = base.clone();
        scaled.device_scale_factor = 2.0;
        let mut raster = base.clone();
        raster.raster_configuration =
            openui_geometry::RasterConfiguration::deterministic_aliased(false);
        let base = FontInstanceKey::new("family", &base);
        assert_ne!(base, FontInstanceKey::new("family", &varied));
        assert_ne!(base, FontInstanceKey::new("family", &palette));
        assert_ne!(base, FontInstanceKey::new("family", &synthesis));
        assert_ne!(base, FontInstanceKey::new("family", &scaled));
        assert_ne!(base, FontInstanceKey::new("family", &raster));
    }

    #[test]
    fn face_metrics_defaults_and_synthesis_are_applied() {
        let collection = FontCollection::system();
        let mut descriptor = FontFaceDescriptor::new("Configured Ahem");
        descriptor.size_adjust = Some(1.25);
        descriptor.metric_overrides = FontMetricOverrides {
            ascent: Some(0.7),
            descent: Some(0.2),
            line_gap: Some(0.1),
        };
        descriptor.feature_defaults.push(FontFeatureDefault {
            tag: *b"kern",
            value: 0,
        });
        collection
            .register(
                Arc::from(include_bytes!("../../fonts/Ahem.ttf").as_slice()),
                descriptor,
            )
            .unwrap();
        let mut description = FontDescription::default();
        description.family = openui_style::FontFamilyList::single("Configured Ahem");
        description.size = 20.0;
        description.weight.0 = 700.0;
        description.font_synthesis_weight = FontSynthesis::None;
        let data = collection
            .resolve_family("Configured Ahem", &description)
            .unwrap();
        assert!((data.size() - 25.0).abs() < 0.01);
        assert!((data.metrics().ascent - 17.5).abs() < 0.01);
        assert!((data.metrics().descent - 5.0).abs() < 0.01);
        assert!((data.metrics().line_gap - 2.5).abs() < 0.01);
        assert!(!data.is_synthetic_bold());
        assert_eq!(data.feature_defaults()[0].tag, *b"kern");
    }

    #[test]
    fn custom_palette_values_are_owned_and_invalidate_instances() {
        let collection = FontCollection::deterministic_test();
        let generation = collection.generation();
        let descriptor = FontPaletteValuesDescriptor {
            name: "brand".into(),
            family: "Noto Color Emoji".into(),
            base_palette: FontPaletteBase::Index(0),
            overrides: vec![FontPaletteEntryOverride {
                index: 2,
                color: openui_style::Color::RED,
            }],
        };
        let handle = collection.register_palette_values(descriptor).unwrap();
        assert!(collection.generation() > generation);

        let state = collection
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let face = state
            .faces
            .iter()
            .find(|face| face.info.descriptor.family == "Noto Color Emoji")
            .unwrap();
        let mut description = FontDescription::default();
        description.palette = FontPalette::Custom("brand".into());
        let palette = resolve_palette(
            &state.palettes,
            &face.typeface,
            Some(&face.info.descriptor),
            &description,
        )
        .unwrap();
        assert_eq!(palette.overrides.len(), 1);
        assert_eq!(palette.overrides[0].index, 2);
        drop(state);

        collection.unregister_palette_values(handle).unwrap();
        assert_eq!(
            collection.unregister_palette_values(handle),
            Err(FontCollectionError::UnknownPalette)
        );
    }
}
