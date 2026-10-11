use openui_compositor::SoftwareCompositor;
use openui_engine::{Engine, ViewportMetrics};

#[test]
fn saved_scenes_replay_after_engine_drop_and_unchanged_frames_reuse_raster() {
    let viewport = ViewportMetrics::from_logical_size(160.0, 64.0, 1.25).unwrap();
    let mut engine = Engine::new(viewport).unwrap();
    let text = engine.create_text("Original native text").unwrap();
    engine.append_child(engine.root(), text).unwrap();
    let scene = engine.scene().unwrap();
    let stats = engine.stats();
    let unchanged = engine.scene().unwrap();
    assert_eq!(engine.stats(), stats);
    assert_eq!(scene.generation(), unchanged.generation());
    let mut compositor = SoftwareCompositor::default();
    let expected = compositor.render(&scene).unwrap();
    assert_eq!(compositor.render(&unchanged).unwrap(), expected);
    assert_eq!(compositor.stats().rasterized, 1);
    assert_eq!(compositor.stats().reused, 1);

    engine.set_text(text, "Changed native text").unwrap();
    let changed = engine.scene().unwrap();
    let changed_frame = compositor.render(&changed).unwrap();
    assert_ne!(changed_frame.pixels, expected.pixels);
    drop(unchanged);
    drop(compositor);
    drop(engine);
    let (replayed, changed_replayed) = std::thread::spawn(move || {
        // Fresh compositors force real drawing after all mutable state is gone.
        (
            SoftwareCompositor::default().render(&scene).unwrap(),
            SoftwareCompositor::default().render(&changed).unwrap(),
        )
    })
    .join()
    .unwrap();
    assert_eq!(replayed, expected);
    assert_eq!(changed_replayed, changed_frame);
}
