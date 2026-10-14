//! Actual source ownership and pre-device settings, without provider/query activation.
//! Pixel/provider/final-strike qualification is separate and remains open.

use std::sync::Arc;

use openui_engine::{Engine, EngineOptions, ViewportMetrics};
use openui_geometry::{RasterConfiguration, TextHinting};
use openui_style::{Color, FontFamilyList, FontPalette, FontVariation, RendererStyleValue};
use openui_text::font::{
    FontProviderConstruction, FontRenderHintStyle, FontRenderPreference, FontRenderQueryIntent,
    FontRenderStyle, FontSelectionOrigin,
};
use openui_text::{
    FontAxisRange, FontCache, FontCollection, FontCollectionError, FontCollectionStats,
    FontContainerFormat, FontDescription, FontFaceDescriptor, FontFaceInfo, FontMetricOverrides,
    FontPaletteBase, FontPaletteEntryOverride, FontPaletteValuesDescriptor, FontPlatformData,
    FontStyleRange, FontUnicodeRange, ShapeResult, ShapeResultCharacterData, ShapeResultRun,
    TextDirection,
};

fn bytes() -> Arc<[u8]> {
    Arc::from(include_bytes!("../../openui-text/fonts/Ahem.ttf").as_slice())
}

fn resolve(
    collection: &Arc<FontCollection>,
    family: &str,
    description: &FontDescription,
) -> Arc<FontPlatformData> {
    FontCache::from_collection(Arc::clone(collection))
        .get_font_platform_data(family, description)
        .unwrap()
}

#[test]
fn application_alias_installed_and_external_keep_the_actual_constructor_source() {
    let application = FontCollection::system();
    let handle = application
        .register(bytes(), FontFaceDescriptor::new("sans-serif"))
        .unwrap();
    let description = FontDescription::default();
    let app = resolve(&application, "sans-serif", &description);
    let installed_collection = FontCollection::system();
    // The native test environment exposes the same installed generic used by
    // existing collection guards. No caller-supplied Typeface is tagged installed.
    let installed = resolve(&installed_collection, "sans-serif", &description);
    assert_eq!(
        app.source_info().origin(),
        FontSelectionOrigin::ApplicationRegistration
    );
    assert_eq!(
        app.source_info().provider_construction(),
        FontProviderConstruction::SystemManagerDataLoader
    );
    assert_eq!(
        app.source_info()
            .application_face()
            .unwrap()
            .face_info()
            .handle,
        handle
    );
    assert_eq!(app.source_info().selection_family(), Some("sans-serif"));
    assert_eq!(app.source_info().resolved_family(), Some("Ahem"));
    assert_eq!(
        app.source_info().render_query_intent(),
        FontRenderQueryIntent::PlatformDefaultEmptyFamily
    );
    assert_eq!(app.source_info().render_query_family_intent(), Some(""));
    assert_eq!(
        installed.source_info().origin(),
        FontSelectionOrigin::InstalledFamilySelection
    );
    assert_eq!(
        installed.source_info().provider_construction(),
        FontProviderConstruction::SystemManagerFamilyMatcher
    );
    assert_eq!(
        installed.source_info().collection_id(),
        Some(installed_collection.id())
    );
    assert!(installed.source_info().application_face().is_none());
    assert_eq!(
        installed.source_info().render_query_intent(),
        FontRenderQueryIntent::NamedSelectionFamily
    );
    assert_eq!(
        installed.source_info().render_query_family_intent(),
        Some("sans-serif")
    );
    assert_ne!(app.typeface().unique_id(), installed.typeface().unique_id());

    let external = FontPlatformData::new(app.typeface().clone(), 16.0);
    assert_eq!(
        external.source_info().origin(),
        FontSelectionOrigin::ExternalTypefaceUnspecified
    );
    assert_eq!(
        external.source_info().provider_construction(),
        FontProviderConstruction::ExternalTypefaceUnspecified
    );
    assert!(external.source_info().application_face().is_none());
    assert!(external.source_info().collection_id().is_none());
    assert!(external.source_info().selection_generation().is_none());
    assert!(external
        .source_info()
        .render_query_family_intent()
        .is_none());
    assert_eq!(
        external.source_info().instance_typeface_id(),
        Some(external.typeface().unique_id())
    );
    for data in [&*app, &*installed, &external] {
        assert!(data.applied_font_settings().query_result().is_none());
    }
    let bundled = FontCollection::deterministic_test();
    let bundled_face = resolve(&bundled, "Ahem", &description);
    assert_eq!(
        bundled_face.source_info().origin(),
        FontSelectionOrigin::ApplicationRegistration
    );
    assert!(bundled_face.source_info().application_face().is_some());
}

#[test]
fn actual_character_fallback_and_cached_instances_preserve_each_source() {
    let collection = FontCollection::system();
    let mut descriptor = FontFaceDescriptor::new("Only A");
    descriptor.unicode_ranges = vec![FontUnicodeRange {
        start: u32::from('A'),
        end: u32::from('A'),
    }];
    let handle = collection.register(bytes(), descriptor).unwrap();
    let mut description = FontDescription::default();
    description.family = FontFamilyList::single("Only A");
    let mut cache = FontCache::from_collection(Arc::clone(&collection));
    let a = cache
        .platform_fallback_for_character('A', &description)
        .unwrap();
    let b = cache
        .platform_fallback_for_character('B', &description)
        .unwrap();
    assert_eq!(
        a.source_info()
            .application_face()
            .unwrap()
            .face_info()
            .handle,
        handle
    );
    assert_eq!(
        a.source_info().origin(),
        FontSelectionOrigin::ApplicationRegistration
    );
    assert_eq!(
        b.source_info().origin(),
        FontSelectionOrigin::InstalledCharacterFallback
    );
    assert_eq!(
        b.source_info().provider_construction(),
        FontProviderConstruction::SystemManagerCharacterMatcher
    );
    assert_eq!(
        b.source_info().render_query_intent(),
        FontRenderQueryIntent::NamedResolvedFallbackFamily
    );
    assert_eq!(
        b.source_info().render_query_family_intent(),
        Some(b.typeface().family_name().as_str())
    );
    assert!(b.source_info().application_face().is_none());
    assert_ne!(a.sk_font().unichar_to_glyph('A' as i32), 0);
    assert_ne!(b.sk_font().unichar_to_glyph('B' as i32), 0);
    let previous = collection.stats();
    let b_again = cache
        .platform_fallback_for_character('B', &description)
        .unwrap();
    assert!(Arc::ptr_eq(&b, &b_again));
    assert_eq!(collection.stats().cache_hits, previous.cache_hits + 1);
    assert_eq!(collection.stats().cache_misses, previous.cache_misses);
    // Existing literal-compatible runs carry their own resolved Arc; no whole-
    // blob profile or intrinsic family manufactures an origin for another run.
    let run = |data: Arc<FontPlatformData>| ShapeResultRun {
        font_data: data,
        glyphs: vec![],
        advances: vec![],
        offsets: vec![],
        clusters: vec![],
        start_index: 0,
        num_characters: 0,
        num_glyphs: 0,
        direction: TextDirection::Ltr,
    };
    let result = ShapeResult {
        runs: vec![run(a), run(b)],
        width: 0.0,
        num_characters: 0,
        direction: TextDirection::Ltr,
        character_data: vec![ShapeResultCharacterData {
            x_position: 0.0,
            is_cluster_base: true,
            safe_to_break_before: true,
        }],
    };
    assert_eq!(
        result.runs[0].font_data.source_info().origin(),
        FontSelectionOrigin::ApplicationRegistration
    );
    assert_eq!(
        result.runs[1].font_data.source_info().origin(),
        FontSelectionOrigin::InstalledCharacterFallback
    );
}

#[test]
fn registration_views_survive_removal_generation_changes_and_collection_drop() {
    let collection = FontCollection::system();
    let weak = Arc::downgrade(&collection);
    let original = bytes();
    let handle = collection
        .register(
            Arc::clone(&original),
            FontFaceDescriptor::new("Owned alias"),
        )
        .unwrap();
    let source = collection.query_source_info(handle).unwrap();
    assert!(Arc::ptr_eq(
        &source,
        &collection.query_source_info(handle).unwrap()
    ));
    assert!(Arc::ptr_eq(source.bytes(), &original));
    assert_eq!(source.face_info(), &collection.query(handle).unwrap());
    assert_eq!(source.registration_generation(), collection.generation());
    let data = resolve(&collection, "Owned alias", &FontDescription::default());
    let generation = data.source_info().selection_generation().unwrap();
    assert_eq!(generation, collection.generation());
    assert_eq!(
        data.source_info()
            .application_face()
            .unwrap()
            .registration_generation(),
        source.registration_generation()
    );
    let wrong = FontCollection::system();
    assert!(matches!(
        wrong.query_source_info(handle),
        Err(FontCollectionError::WrongCollection)
    ));
    collection.unregister(handle).unwrap();
    assert!(matches!(
        collection.query_source_info(handle),
        Err(FontCollectionError::UnknownFace)
    ));
    assert!(collection.generation() > generation);
    let replacement = collection
        .register(bytes(), FontFaceDescriptor::new("Owned alias"))
        .unwrap();
    assert_ne!(replacement, handle);
    let newer = resolve(&collection, "Owned alias", &FontDescription::default());
    assert_eq!(data.source_info().selection_generation(), Some(generation));
    assert_eq!(
        data.source_info()
            .application_face()
            .unwrap()
            .face_info()
            .handle,
        handle
    );
    assert_eq!(
        newer
            .source_info()
            .application_face()
            .unwrap()
            .face_info()
            .handle,
        replacement
    );
    assert!(newer.source_info().selection_generation().unwrap() > generation);
    drop(collection);
    assert!(
        weak.upgrade().is_none(),
        "source metadata must not retain its registry"
    );
    std::thread::spawn(move || {
        assert_eq!(source.face_info().handle, handle);
        assert_eq!(source.bytes().as_ref(), original.as_ref());
        assert_eq!(data.source_info().selection_generation(), Some(generation));
        assert_ne!(data.sk_font().unichar_to_glyph('A' as i32), 0);
    })
    .join()
    .unwrap();
}

#[test]
fn retained_instance_request_is_distinct_from_actual_filtered_clone_arguments() {
    let collection = FontCollection::system();
    let mut descriptor = FontFaceDescriptor::new("Indexed Ahem");
    descriptor.face_index = 1;
    let handle = collection
        .register(
            Arc::from(
                include_bytes!("../../openui-text/fonts/certification/fixture.ttc").as_slice(),
            ),
            descriptor,
        )
        .unwrap();
    let mut description = FontDescription::default();
    description.variation_settings = vec![FontVariation {
        tag: *b"wght",
        value: 800.0,
    }];
    let plain = resolve(&collection, "Indexed Ahem", &description);
    assert_eq!(
        plain
            .source_info()
            .application_face()
            .unwrap()
            .face_info()
            .descriptor
            .face_index,
        1
    );
    assert_eq!(
        plain
            .source_info()
            .application_face()
            .unwrap()
            .face_info()
            .handle,
        handle
    );
    assert_eq!(
        plain
            .source_info()
            .instance_description()
            .unwrap()
            .variation_settings,
        description.variation_settings
    );
    // Both byte-pinned TTC faces lack fvar: the existing axis filter drops wght.
    assert!(plain.source_info().instantiation().coordinates().is_empty());
    assert!(!plain.source_info().instantiation().clone_attempted());
    assert!(!plain.source_info().instantiation().clone_succeeded());
    assert!(plain
        .source_info()
        .instantiation()
        .collection_index()
        .is_none());
    assert_eq!(
        plain.source_info().selected_typeface_id(),
        plain.source_info().instance_typeface_id()
    );
    collection
        .register_palette_values(FontPaletteValuesDescriptor {
            name: "owned-palette".into(),
            family: "Indexed Ahem".into(),
            base_palette: FontPaletteBase::Index(3),
            overrides: vec![FontPaletteEntryOverride {
                index: 2,
                color: Color::RED,
            }],
        })
        .unwrap();
    description.palette = FontPalette::Custom("owned-palette".into());
    let palette = resolve(&collection, "Indexed Ahem", &description);
    assert_eq!(
        palette
            .source_info()
            .instance_description()
            .unwrap()
            .palette,
        description.palette
    );
    let arguments = palette.source_info().instantiation();
    assert!(arguments.clone_attempted());
    // The existing clone arguments default to index0, independently of the
    // selected TTC registration's index1. Metadata must not hide this operand.
    assert_eq!(arguments.collection_index(), Some(0));
    assert_eq!(arguments.palette_index(), Some(3));
    assert_eq!(arguments.palette_overrides().len(), 1);
    assert_eq!(arguments.palette_overrides()[0].0, 2);
    assert_eq!(
        palette.source_info().instance_typeface_id(),
        Some(palette.typeface().unique_id())
    );
    // Clone success is an observed outcome, never an assumed palette capability.
    if !arguments.clone_succeeded() {
        assert_eq!(
            palette.source_info().selected_typeface_id(),
            palette.source_info().instance_typeface_id()
        );
    }
}

#[test]
fn actual_source_and_shaping_settings_distinguish_metric_positioning() {
    let collection = FontCollection::deterministic_test();
    for full_hinting in [false, true] {
        let mut description = FontDescription::default();
        description.raster_configuration = RasterConfiguration::deterministic_aliased(false);
        if full_hinting {
            description.raster_configuration.author_text.hinting = TextHinting::Full;
        }
        description.device_scale_factor = 1.25;
        let data = resolve(&collection, "Ahem", &description);
        let applied = data.applied_font_settings();
        let actual = data.sk_font();
        let snapshot = applied.source_font();
        assert_eq!(snapshot.size().to_bits(), actual.size().to_bits());
        assert_eq!(snapshot.scale_x().to_bits(), actual.scale_x().to_bits());
        assert_eq!(snapshot.skew_x().to_bits(), actual.skew_x().to_bits());
        assert_eq!(snapshot.edging(), actual.edging());
        assert_eq!(snapshot.hinting(), actual.hinting());
        assert_eq!(snapshot.subpixel(), actual.is_subpixel());
        assert_eq!(snapshot.linear_metrics(), actual.is_linear_metrics());
        assert_eq!(
            snapshot.force_auto_hinting(),
            actual.is_force_auto_hinting()
        );
        assert_eq!(snapshot.embedded_bitmaps(), actual.is_embedded_bitmaps());
        assert_eq!(snapshot.embolden(), actual.is_embolden());
        assert_eq!(snapshot.baseline_snap(), actual.is_baseline_snap());
        assert_eq!(
            applied.requested_raster_configuration(),
            description.raster_configuration
        );
        assert_eq!(
            applied.selected_raster_configuration(),
            description.raster_configuration.author_text
        );
        assert_eq!(applied.device_scale_factor().to_bits(), 1.25_f64.to_bits());
        assert!(!snapshot.subpixel());
        assert!(!snapshot.linear_metrics());
        assert_eq!(applied.shaping_font().subpixel(), !full_hinting);
        assert!(!applied.shaping_font().linear_metrics());
        assert_eq!(applied.shaping_font().hinting(), actual.hinting());
        assert!(applied.query_result().is_none());
    }
    let mut description = FontDescription::default();
    description.raster_configuration = RasterConfiguration::deterministic_aliased(false);
    description.native_control_text = true;
    description.native_button_text_metrics = true;
    let data = resolve(&collection, "Ahem", &description);
    let applied = data.applied_font_settings();
    assert!(applied.native_control_text());
    assert!(applied.native_button_text_metrics());
    assert!(!applied.embedded_document_text());
    assert_eq!(
        applied.selected_raster_configuration(),
        description.raster_configuration.native_text
    );
    assert!(!applied.source_font().subpixel());
    assert!(!applied.source_font().linear_metrics());
    assert!(applied.shaping_font().subpixel());
    assert!(!applied.shaping_font().linear_metrics());
}

#[test]
fn borrowed_metadata_reads_do_no_cache_selection_or_engine_work() {
    let collection = FontCollection::system();
    let handle = collection
        .register(bytes(), FontFaceDescriptor::new("Read-only alias"))
        .unwrap();
    let data = resolve(&collection, "Read-only alias", &FontDescription::default());
    let mut engine = Engine::new_with_font_collection_and_options(
        ViewportMetrics::from_logical_size(160.0, 96.0, 1.25).unwrap(),
        Arc::clone(&collection),
        EngineOptions {
            raster_configuration: RasterConfiguration::deterministic_aliased(false),
        },
    )
    .unwrap();
    let root = engine.root();
    engine
        .set_renderer_style(
            root,
            RendererStyleValue::FontFamily(FontFamilyList::single("Read-only alias")),
        )
        .unwrap();
    let text = engine.create_text("A").unwrap();
    engine.append_child(root, text).unwrap();
    let generation = engine.scene().unwrap().generation();
    let stats = engine.stats();
    let cache = collection.stats();
    let owners = Arc::strong_count(&collection);
    let source_ptr = data.source_info() as *const _;
    let settings_ptr = data.applied_font_settings() as *const _;
    let byte_owners = Arc::strong_count(data.source_info().application_face().unwrap().bytes());
    for _ in 0..64 {
        assert_eq!(data.source_info() as *const _, source_ptr);
        assert_eq!(data.applied_font_settings() as *const _, settings_ptr);
        assert_eq!(
            data.source_info()
                .application_face()
                .unwrap()
                .face_info()
                .handle,
            handle
        );
        let _ = std::hint::black_box(data.source_info().instance_description());
        std::hint::black_box(data.applied_font_settings().shaping_font());
        let _ = std::hint::black_box(data.applied_font_settings().query_result());
    }
    assert_eq!(Arc::strong_count(&collection), owners);
    assert_eq!(
        Arc::strong_count(data.source_info().application_face().unwrap().bytes()),
        byte_owners
    );
    assert_eq!(collection.stats(), cache);
    assert_eq!(engine.scene().unwrap().generation(), generation);
    assert_eq!(engine.stats(), stats);
    assert_eq!(collection.stats(), cache);
}

#[test]
fn pure_optional_style_keeps_no_preference_and_hint_style_presence() {
    use FontRenderHintStyle::{Full, None, Normal};
    use FontRenderPreference::{NoPreference, Off, On};
    assert_eq!([Off.code(), On.code(), NoPreference.code()], [0, 1, 2]);
    assert_eq!([None.code(), Normal.code(), Full.code()], [0, 2, 3]);
    assert_eq!(FontRenderPreference::from_code(2), Some(NoPreference));
    assert!(FontRenderPreference::from_code(3).is_none());
    assert!(FontRenderHintStyle::from_code(4).is_none());
    let initial = FontRenderStyle::default();
    assert_eq!(initial.hint_style(), None);
    assert_eq!(initial.use_hinting(), NoPreference);
    assert_eq!(initial.use_bitmaps(), NoPreference);
    assert_eq!(initial.use_auto_hint(), NoPreference);
    assert_eq!(initial.use_anti_alias(), NoPreference);
    assert_eq!(initial.use_subpixel_rendering(), NoPreference);
    assert_eq!(initial.use_subpixel_positioning(), NoPreference);
    let cases = [
        (Off, Off, Off),
        (Off, On, On),
        (Off, NoPreference, Off),
        (On, Off, Off),
        (On, On, On),
        (On, NoPreference, On),
        (NoPreference, Off, Off),
        (NoPreference, On, On),
        (NoPreference, NoPreference, NoPreference),
    ];
    let style = |value, hint| {
        initial
            .with_hinting(value, hint)
            .with_bitmaps(value)
            .with_auto_hint(value)
            .with_anti_alias(value)
            .with_subpixel_rendering(value)
            .with_subpixel_positioning(value)
    };
    for (base, query, expected) in cases {
        let resolved = style(base, Normal).override_with(style(query, Full));
        assert_eq!(resolved.use_hinting(), expected);
        assert_eq!(resolved.use_bitmaps(), expected);
        assert_eq!(resolved.use_auto_hint(), expected);
        assert_eq!(resolved.use_anti_alias(), expected);
        assert_eq!(resolved.use_subpixel_rendering(), expected);
        assert_eq!(resolved.use_subpixel_positioning(), expected);
        assert_eq!(
            resolved.hint_style(),
            if query == NoPreference { Normal } else { Full }
        );
    }
    // C++ truthiness of source NoPreference=2 is preserved as a separate fact,
    // without turning an unavailable platform query into anti-alias Off.
    assert_ne!(initial.use_anti_alias().code(), 0);
}

#[test]
fn existing_public_literals_and_exhaustive_error_variants_remain_compatible() {
    let collection = FontCollection::system();
    let descriptor = FontFaceDescriptor {
        family: "Literal alias".into(),
        face_index: 0,
        style: FontStyleRange::Normal,
        weight: FontAxisRange {
            min: 400.0,
            max: 400.0,
        },
        stretch: FontAxisRange {
            min: 100.0,
            max: 100.0,
        },
        unicode_ranges: vec![],
        feature_defaults: vec![],
        size_adjust: None,
        metric_overrides: FontMetricOverrides {
            ascent: None,
            descent: None,
            line_gap: None,
        },
    };
    let handle = collection.register(bytes(), descriptor.clone()).unwrap();
    let actual = collection.query(handle).unwrap();
    let literal = FontFaceInfo {
        handle,
        descriptor,
        format: FontContainerFormat::Ttf,
        byte_length: actual.byte_length,
        sha256: actual.sha256,
    };
    assert_eq!(actual, literal);
    let stats = collection.stats();
    assert_eq!(
        stats,
        FontCollectionStats {
            generation: stats.generation,
            registered_faces: 1,
            registered_bytes: actual.byte_length,
            cached_instances: 0,
            cache_hits: 0,
            cache_misses: 0
        }
    );
    let error_code = |error: FontCollectionError| match error {
        FontCollectionError::EmptyFamily => 0,
        FontCollectionError::InvalidDescriptor(_) => 1,
        FontCollectionError::UnsupportedContainer => 2,
        FontCollectionError::MalformedFont => 3,
        FontCollectionError::InvalidFaceIndex => 4,
        FontCollectionError::DuplicateDescriptor => 5,
        FontCollectionError::InputTooLarge => 6,
        FontCollectionError::CollectionLimit => 7,
        FontCollectionError::AllocationFailed => 8,
        FontCollectionError::WrongCollection => 9,
        FontCollectionError::UnknownFace => 10,
        FontCollectionError::UnknownPalette => 11,
    };
    assert_eq!(error_code(FontCollectionError::UnknownFace), 10);
    assert_eq!(error_code(FontCollectionError::UnknownPalette), 11);
}
