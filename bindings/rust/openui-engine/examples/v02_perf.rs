//! Deterministic Linux/headless v0.2 performance qualification harness.
//!
//! Run with `cargo run -p openui-engine --release --example v02_perf`.

use openui_compositor::SoftwareCompositor;
use openui_dom::ElementTag;
use openui_engine::{AnimationTimeline, Engine, ViewportMetrics};
use openui_style::{
    AnimationOptions, Display, FillMode, Keyframes, LengthValue, PropertyKeyframes, StyleProperty,
};
use std::time::Instant;

const INTERACTIONS: usize = 10_000;
const WARMUP_INTERACTIONS: usize = 50_000;
const LATENCY_SAMPLES: usize = 200;
const ANIMATION_FRAMES: usize = 120;

fn resident_kib() -> Option<u64> {
    std::fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

fn percentile_95(samples: &mut [f64]) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[((samples.len() as f64 * 0.95).ceil() as usize).saturating_sub(1)]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = Engine::new(ViewportMetrics::from_logical_size(128.0, 128.0, 1.0)?)?;
    let root = engine.root();
    let mut nodes = Vec::with_capacity(100);
    for _ in 0..100 {
        let node = engine.create_element(ElementTag::Div)?;
        engine.append_child(root, node)?;
        engine.set_property(node, StyleProperty::Display, Display::Block.into())?;
        engine.set_property(node, StyleProperty::Width, LengthValue::px(8.0).into())?;
        engine.set_property(node, StyleProperty::Height, LengthValue::px(1.0).into())?;
        nodes.push(node);
    }

    let initial = engine.scene()?;
    let settled_stats = engine.stats();
    for _ in 0..1_000 {
        let unchanged = engine.scene()?;
        assert_eq!(unchanged.generation(), initial.generation());
    }
    let unchanged_work_is_zero = engine.stats() == settled_stats;

    let mut compositor = SoftwareCompositor::default();
    compositor.render(&initial)?;
    let before_raster = compositor.stats();
    compositor.render(&initial)?;
    let unchanged_raster_is_zero = compositor.stats().rasterized == before_raster.rasterized;

    // Warm all allocator and raster paths before recording latency or RSS.
    for iteration in 0..WARMUP_INTERACTIONS {
        engine.set_property(
            nodes[0],
            StyleProperty::Opacity,
            ((iteration & 1) as f32).into(),
        )?;
        let scene = engine.scene()?;
        compositor.render(&scene)?;
    }

    let rss_before = resident_kib();
    let objects_before = engine.object_counts();
    let mut latencies_ms = Vec::with_capacity(LATENCY_SAMPLES);
    let interaction_start = Instant::now();
    for iteration in 0..INTERACTIONS {
        let started = Instant::now();
        engine.set_property(
            nodes[0],
            StyleProperty::Opacity,
            ((iteration & 1) as f32).into(),
        )?;
        let scene = engine.scene()?;
        compositor.render(&scene)?;
        if iteration < LATENCY_SAMPLES {
            latencies_ms.push(started.elapsed().as_secs_f64() * 1_000.0);
        }
    }
    let interactions_per_second = INTERACTIONS as f64 / interaction_start.elapsed().as_secs_f64();
    let p95_input_to_present_ms = percentile_95(&mut latencies_ms);
    let rss_after = resident_kib();
    let rss_growth_percent = match (rss_before, rss_after) {
        (Some(before), Some(after)) if before > 0 => {
            Some((after.saturating_sub(before)) as f64 * 100.0 / before as f64)
        }
        _ => None,
    };
    let objects_after = engine.object_counts();
    let owned_object_leak = objects_before != objects_after;

    let options = AnimationOptions {
        duration_ms: 2_000.0,
        fill: FillMode::Both,
        ..AnimationOptions::default()
    };
    for node in &nodes {
        let frames = PropertyKeyframes::typed(
            StyleProperty::Opacity,
            Keyframes::from_values(0.0_f32, 1.0_f32),
        )?;
        engine.animate(*node, frames, options.clone(), AnimationTimeline::Document)?;
    }
    let animation_start = Instant::now();
    for frame in 0..ANIMATION_FRAMES {
        engine.set_animation_time(frame as f64 * (1_000.0 / 60.0))?;
        let scene = engine.scene()?;
        compositor.render(&scene)?;
    }
    let animation_fps = ANIMATION_FRAMES as f64 / animation_start.elapsed().as_secs_f64();

    // JSON is emitted without a serialization dependency so the qualification
    // harness cannot perturb the audited dependency graph.
    println!(
        concat!(
            "{{\n",
            "  \"schema_version\": 1,\n",
            "  \"profile\": \"local-linux-headless-smoke\",\n",
            "  \"warmup_interactions\": {WARMUP_INTERACTIONS},\n",
            "  \"interactions\": {INTERACTIONS},\n",
            "  \"interactions_per_second\": {interactions_per_second:.3},\n",
            "  \"p95_input_to_present_ms\": {p95_input_to_present_ms:.3},\n",
            "  \"animation_frames\": {ANIMATION_FRAMES},\n",
            "  \"hundred_ui_thread_animations_fps\": {animation_fps:.3},\n",
            "  \"unchanged_layout_and_paint_work_zero\": {unchanged_work_is_zero},\n",
            "  \"unchanged_raster_work_zero\": {unchanged_raster_is_zero},\n",
            "  \"owned_object_leak\": {owned_object_leak},\n",
            "  \"rss_before_kib\": {rss_before},\n",
            "  \"rss_after_kib\": {rss_after},\n",
            "  \"rss_growth_percent\": {rss_growth_percent}\n",
            "}}"
        ),
        INTERACTIONS = INTERACTIONS,
        WARMUP_INTERACTIONS = WARMUP_INTERACTIONS,
        interactions_per_second = interactions_per_second,
        p95_input_to_present_ms = p95_input_to_present_ms,
        ANIMATION_FRAMES = ANIMATION_FRAMES,
        animation_fps = animation_fps,
        unchanged_work_is_zero = unchanged_work_is_zero,
        unchanged_raster_is_zero = unchanged_raster_is_zero,
        owned_object_leak = owned_object_leak,
        rss_before = rss_before.map_or("null".to_owned(), |value| value.to_string()),
        rss_after = rss_after.map_or("null".to_owned(), |value| value.to_string()),
        rss_growth_percent =
            rss_growth_percent.map_or("null".to_owned(), |value| format!("{value:.3}")),
    );
    Ok(())
}
