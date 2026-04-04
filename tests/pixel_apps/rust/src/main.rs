//! Render all 10 pixel-test web apps through Open UI's rendering pipeline.
//!
//! This binary loads each HTML file using the same Blink engine that powers
//! our Rust framework, renders them at 1200×800, and saves PNG screenshots
//! for pixel comparison against the Playwright-captured browser screenshots.

use openui::prelude::*;
use std::fs;
use std::path::PathBuf;

fn find_project_root() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap();
    loop {
        if dir.join("BUILD.gn").exists() && dir.join("src").is_dir() {
            return dir;
        }
        if !dir.pop() {
            panic!("Cannot find project root (directory containing BUILD.gn)");
        }
    }
}

fn main() {
    let root = find_project_root();
    let apps_dir = root.join("tests/pixel_apps/apps");
    let out_dir = root.join("tests/pixel_apps/screenshots/rust");

    fs::create_dir_all(&out_dir).expect("failed to create output directory");

    let mut entries: Vec<_> = fs::read_dir(&apps_dir)
        .expect("failed to read apps directory")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map_or(false, |ext| ext == "html")
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    println!("Rendering {} apps through Open UI pipeline...", entries.len());

    for entry in &entries {
        let path = entry.path();
        let name = path.file_stem().unwrap().to_string_lossy();
        let html = fs::read_to_string(&path).expect("failed to read HTML file");

        let png_path = out_dir.join(format!("{name}.png"));

        let mut app = App::new(1200, 800);
        app.load_html(&html);
        app.run_frames(1);
        app.render_to_png(png_path.to_str().unwrap());

        println!("  Rendered: {name}");
    }

    println!(
        "Done! {} screenshots saved to {}",
        entries.len(),
        out_dir.display()
    );
}
