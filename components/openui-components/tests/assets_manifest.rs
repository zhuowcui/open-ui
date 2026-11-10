//! The bundled fonts match the assets manifest the reference pages use, and
//! Open UI registers each with the reference pages' `@font-face` descriptor.

use openui::prelude::*;
use openui::{FontAxisRange, FontStyleRange};
use openui_components::fonts::{register_fonts, BUNDLED_FONTS};

const MANIFEST: &str = include_str!("../../assets/manifest.json");

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn bundled_fonts_are_the_manifest_faces() {
    let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
    let faces = manifest["font_faces"].as_array().unwrap();
    assert_eq!(faces.len(), BUNDLED_FONTS.len());
    for (face, font) in faces.iter().zip(BUNDLED_FONTS) {
        assert_eq!(face["family"], font.family);
        assert_eq!(face["file"], font.file);
        assert_eq!(face["style"], "normal");
        let weight = face["weight"].as_array().unwrap();
        let descriptor = font.descriptor();
        assert_eq!(descriptor.style, FontStyleRange::Normal);
        assert_eq!(
            descriptor.weight,
            FontAxisRange::new(
                weight[0].as_f64().unwrap() as f32,
                weight[1].as_f64().unwrap() as f32
            ),
            "{}",
            font.family
        );
    }
}

#[test]
fn registered_fonts_are_the_manifest_files() {
    let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
    let files = manifest["files"].as_array().unwrap();
    let app = HeadlessApp::with_options(
        ViewportMetrics::from_logical_size(100.0, 100.0, 1.0).unwrap(),
        EngineOptions::default(),
    )
    .unwrap();
    let handles = register_fonts(app.document()).unwrap();
    assert_eq!(handles.len(), BUNDLED_FONTS.len());
    for (handle, font) in handles.into_iter().zip(BUNDLED_FONTS) {
        let info = app.document().font_face_info(handle).unwrap();
        let entry = files
            .iter()
            .find(|entry| entry["file"] == font.file)
            .unwrap_or_else(|| panic!("{} is not in the manifest", font.file));
        assert_eq!(entry["sha256"], hex(&info.sha256), "{}", font.file);
        assert_eq!(info.byte_length, font.bytes.len());
        assert_eq!(info.descriptor.family, font.family);
    }
}
