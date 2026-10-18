//! Framework-only consumption of owned font facts and pure optional settings.
//! Provider queries, physical strikes, and rendering policies are separate.

use std::sync::Arc;

use openui::{
    App, AppliedPlatformFontSettings, AppliedSkFontSettings, Document, FontCollection,
    FontCollectionError, FontFaceDescriptor, FontFaceSourceInfo, FontInstantiation,
    FontProviderConstruction, FontRenderHintStyle, FontRenderPreference, FontRenderQueryIntent,
    FontRenderStyle, FontSelectionOrigin, FontSelectionSource, ViewportMetrics, WindowOptions,
};

fn app(fonts: Arc<FontCollection>) -> App {
    let document = Document::with_font_collection(
        ViewportMetrics::from_logical_size(800.0, 600.0, 1.0).unwrap(),
        fonts,
    )
    .unwrap();
    App::from_document(document, WindowOptions::default()).unwrap()
}

fn bytes() -> Arc<[u8]> {
    Arc::from(include_bytes!("../../openui-text/fonts/Ahem.ttf").as_slice())
}

#[test]
fn app_document_registration_views_survive_removal_and_registry_drop() {
    let fonts = FontCollection::system();
    let weak_fonts = Arc::downgrade(&fonts);
    let application = app(Arc::clone(&fonts));
    let bytes = bytes();
    let handle = application
        .document()
        .register_font_face(
            Arc::clone(&bytes),
            FontFaceDescriptor::new("Public font alias"),
        )
        .unwrap();
    let retained: Arc<FontFaceSourceInfo> = fonts.query_source_info(handle).unwrap();
    let generation = retained.registration_generation();
    assert_eq!(
        retained.origin(),
        FontSelectionOrigin::ApplicationRegistration
    );
    assert_eq!(
        retained.provider_construction(),
        FontProviderConstruction::SystemManagerDataLoader
    );
    assert_eq!(retained.face_info().handle, handle);
    assert_eq!(retained.face_info().descriptor.family, "Public font alias");
    assert_eq!(
        application.document().font_face_info(handle).unwrap(),
        *retained.face_info()
    );
    assert!(Arc::ptr_eq(retained.bytes(), &bytes));
    assert_eq!(generation, fonts.generation());

    let other = FontCollection::system();
    assert!(matches!(
        other.query_source_info(handle),
        Err(FontCollectionError::WrongCollection)
    ));
    application.document().unregister_font_face(handle).unwrap();
    assert!(application.document().font_face_info(handle).is_err());
    assert!(matches!(
        fonts.query_source_info(handle),
        Err(FontCollectionError::UnknownFace)
    ));
    assert_eq!(retained.face_info().handle, handle);
    assert_eq!(retained.registration_generation(), generation);
    assert_eq!(retained.bytes().as_ref(), bytes.as_ref());

    let replacement = application
        .document()
        .register_font_face(
            Arc::clone(&bytes),
            FontFaceDescriptor::new("Public font alias"),
        )
        .unwrap();
    let current = fonts.query_source_info(replacement).unwrap();
    assert_ne!(replacement, handle);
    assert!(current.registration_generation() > generation);
    assert_eq!(retained.face_info().handle, handle);
    assert_eq!(current.face_info().handle, replacement);

    drop(application);
    drop(fonts);
    assert!(weak_fonts.upgrade().is_none());
    drop(bytes);
    let expected_hash = retained.face_info().sha256;
    std::thread::spawn(move || {
        assert_eq!(retained.face_info().sha256, expected_hash);
        assert_eq!(retained.registration_generation(), generation);
        assert_eq!(retained.bytes().len(), retained.face_info().byte_length);
        assert_eq!(current.face_info().handle, replacement);
        assert_eq!(current.bytes().as_ref(), retained.bytes().as_ref());
    })
    .join()
    .unwrap();
}

#[test]
fn framework_borrowed_registration_reads_preserve_collection_counters() {
    let fonts = FontCollection::system();
    let application = app(Arc::clone(&fonts));
    let handle = application
        .document()
        .register_font_face(bytes(), FontFaceDescriptor::new("Borrowed public metadata"))
        .unwrap();
    let view: Arc<FontFaceSourceInfo> = fonts.query_source_info(handle).unwrap();
    let before = application.document().font_collection_stats().unwrap();
    let byte_owners = Arc::strong_count(view.bytes());
    for _ in 0..64 {
        let _ = std::hint::black_box(view.face_info());
        let _ = std::hint::black_box(view.bytes());
        let _ = std::hint::black_box(view.registration_generation());
        let _ = std::hint::black_box(view.origin());
        let _ = std::hint::black_box(view.provider_construction());
    }
    assert_eq!(Arc::strong_count(view.bytes()), byte_owners);
    assert_eq!(fonts.stats(), before);
    assert_eq!(
        application.document().font_collection_stats().unwrap(),
        before
    );
}

// This signature checks the public facade's borrowed view types without
// manufacturing a platform font or claiming an executed settings query.
fn public_borrowed_views<'a>(
    source: &'a FontSelectionSource,
    settings: &'a AppliedPlatformFontSettings,
) -> (
    &'a FontInstantiation,
    &'a AppliedSkFontSettings,
    &'a AppliedSkFontSettings,
) {
    (
        source.instantiation(),
        settings.source_font(),
        settings.shaping_font(),
    )
}

#[test]
fn framework_optional_preferences_keep_absent_settings_distinct() {
    let _public_getters = public_borrowed_views;
    let absent = FontInstantiation::default();
    assert!(absent.collection_index().is_none());
    assert!(absent.palette_index().is_none());
    assert!(!absent.clone_attempted());
    assert!(!absent.clone_succeeded());
    assert_eq!(
        FontRenderPreference::from_code(2),
        Some(FontRenderPreference::NoPreference)
    );
    assert!(FontRenderPreference::from_code(3).is_none());
    assert_eq!(
        FontRenderHintStyle::from_code(3),
        Some(FontRenderHintStyle::Full)
    );

    let named = FontRenderStyle::default()
        .with_hinting(FontRenderPreference::On, FontRenderHintStyle::Slight)
        .with_bitmaps(FontRenderPreference::Off)
        .with_anti_alias(FontRenderPreference::On);
    let unspecified_hint = FontRenderStyle::default()
        .with_hinting(
            FontRenderPreference::NoPreference,
            FontRenderHintStyle::Full,
        )
        .with_subpixel_rendering(FontRenderPreference::Off);
    let merged = named.override_with(unspecified_hint);
    assert_eq!(merged.use_hinting(), FontRenderPreference::On);
    assert_eq!(merged.hint_style(), FontRenderHintStyle::Slight);
    assert_eq!(merged.use_bitmaps(), FontRenderPreference::Off);
    assert_eq!(merged.use_anti_alias(), FontRenderPreference::On);
    assert_eq!(merged.use_subpixel_rendering(), FontRenderPreference::Off);
    assert_eq!(
        merged.use_subpixel_positioning(),
        FontRenderPreference::NoPreference
    );
    let disabled = merged.override_with(
        FontRenderStyle::default()
            .with_hinting(FontRenderPreference::Off, FontRenderHintStyle::None),
    );
    assert_eq!(disabled.use_hinting(), FontRenderPreference::Off);
    assert_eq!(disabled.hint_style(), FontRenderHintStyle::None);

    // Intent remains a named public source fact; it is not a queried style.
    let intent = FontRenderQueryIntent::NamedSelectionFamily;
    assert_ne!(intent, FontRenderQueryIntent::PlatformDefaultEmptyFamily);
    assert_eq!(named.hint_style(), FontRenderHintStyle::Slight);
}
