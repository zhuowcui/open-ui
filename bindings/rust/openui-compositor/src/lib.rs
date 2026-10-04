//! Immutable, backend-independent scene snapshots.

use openui_geometry::{RasterConfiguration, RasterSnapping, ViewportMetrics};
use openui_layout::Fragment;
use openui_paint::{rasterize_picture, RecordedPicture};
use skia_safe::image::CachingHint;
use skia_safe::{Canvas, EncodedImageFormat};
use std::sync::{Arc, Condvar, Mutex};

#[cfg(feature = "ganesh-gl")]
mod ganesh_gl;
#[cfg(feature = "ganesh-gl")]
pub use ganesh_gl::GaneshGlCompositor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RasterBackendIdentity {
    pub backend: &'static str,
    pub gl_renderer: Option<String>,
    pub gl_version: Option<String>,
    pub driver: Option<String>,
    pub color_type: &'static str,
    pub sample_count: usize,
    pub surface_properties: &'static str,
}

impl RasterBackendIdentity {
    pub fn cpu_skia() -> Self {
        Self {
            backend: "cpu-skia",
            gl_renderer: None,
            gl_version: None,
            driver: None,
            color_type: "N32-premultiplied",
            sample_count: 0,
            surface_properties: "scene-raster-configuration-v1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Damage bounds snapped outward to physical surface pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalSceneRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneGeneration(pub u64);

/// A complete immutable frame. It contains no callbacks or mutable DOM state.
#[derive(Clone)]
pub struct SceneSnapshot {
    generation: SceneGeneration,
    viewport: ViewportMetrics,
    raster_configuration: RasterConfiguration,
    picture: Arc<RecordedPicture>,
    fragments: Arc<Fragment>,
    logical_damage: Arc<[SceneRect]>,
    physical_damage: Arc<[PhysicalSceneRect]>,
}

impl SceneSnapshot {
    pub fn new(
        generation: SceneGeneration,
        picture: RecordedPicture,
        fragments: Arc<Fragment>,
        damage: Vec<SceneRect>,
    ) -> Self {
        let viewport = picture.viewport;
        let physical_damage = damage
            .iter()
            .filter_map(|rect| physical_damage_rect(*rect, viewport))
            .collect::<Vec<_>>();
        Self {
            generation,
            viewport,
            raster_configuration: picture.raster_configuration,
            picture: Arc::new(picture),
            fragments,
            logical_damage: damage.into(),
            physical_damage: physical_damage.into(),
        }
    }

    pub fn generation(&self) -> SceneGeneration {
        self.generation
    }
    pub fn viewport(&self) -> ViewportMetrics {
        self.viewport
    }
    pub fn raster_configuration(&self) -> RasterConfiguration {
        self.raster_configuration
    }
    pub fn fragments(&self) -> &Arc<Fragment> {
        &self.fragments
    }
    pub fn damage(&self) -> &[SceneRect] {
        &self.logical_damage
    }
    pub fn physical_damage(&self) -> &[PhysicalSceneRect] {
        &self.physical_damage
    }
    /// Number of immutable application-font byte buffers retained by this scene.
    pub fn retained_font_face_count(&self) -> usize {
        self.picture.retained_font_face_count()
    }

    /// Replay this immutable scene into a caller-owned Skia canvas.
    ///
    /// GPU and software presenters use this exact recording, so backend
    /// selection cannot affect layout or scene construction.
    pub fn replay(&self, canvas: &Canvas) {
        let scale = self.viewport.device_scale_factor() as f32;
        canvas.scale((scale, scale));
        canvas.draw_picture(&self.picture.picture, None, None);
    }
}

fn physical_damage_rect(rect: SceneRect, viewport: ViewportMetrics) -> Option<PhysicalSceneRect> {
    if !rect.x.is_finite()
        || !rect.y.is_finite()
        || !rect.width.is_finite()
        || !rect.height.is_finite()
        || rect.width <= 0.0
        || rect.height <= 0.0
    {
        return None;
    }
    let snapping = RasterSnapping::new(viewport.device_scale_factor());
    let max_x = f64::from(viewport.physical_width());
    let max_y = f64::from(viewport.physical_height());
    let (left, right) = snapping.outward_physical_span(rect.x, rect.width)?;
    let (top, bottom) = snapping.outward_physical_span(rect.y, rect.height)?;
    let left = (left as f64).clamp(0.0, max_x) as u32;
    let top = (top as f64).clamp(0.0, max_y) as u32;
    let right = (right as f64).clamp(0.0, max_x) as u32;
    let bottom = (bottom as f64).clamp(0.0, max_y) as u32;
    (right > left && bottom > top).then_some(PhysicalSceneRect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// Physical raster width.
    pub width: u32,
    /// Physical raster height.
    pub height: u32,
    pub viewport: ViewportMetrics,
    pub raster_configuration: RasterConfiguration,
    pub raster_backend_identity: RasterBackendIdentity,
    pub stride: usize,
    /// Top-to-bottom RGBA8888 rows with premultiplied color channels.
    pub pixels: Vec<u8>,
    pub scene_generation: SceneGeneration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositorError {
    Raster(String),
    ReadPixels,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CompositorStats {
    pub submitted: u64,
    pub rasterized: u64,
    pub reused: u64,
    pub dropped_before_present: u64,
}

impl std::fmt::Display for CompositorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Raster(value) => write!(f, "rasterization failed: {value}"),
            Self::ReadPixels => f.write_str("could not read raster pixels"),
        }
    }
}

impl std::error::Error for CompositorError {}

/// Retain the immutable recording while its frame is cached. Scene generations
/// belong to individual documents, so a generation alone cannot identify pixels.
struct CachedFrame {
    picture: Arc<RecordedPicture>,
    frame: Frame,
}

impl CachedFrame {
    fn new(scene: &SceneSnapshot, frame: Frame) -> Self {
        Self {
            picture: Arc::clone(&scene.picture),
            frame,
        }
    }

    fn frame_for(&self, scene: &SceneSnapshot) -> Option<Frame> {
        (self.frame.scene_generation == scene.generation
            && Arc::ptr_eq(&self.picture, &scene.picture))
        .then(|| self.frame.clone())
    }
}

#[derive(Default)]
pub struct SoftwareCompositor {
    last_presented: Option<SceneGeneration>,
    last_frame: Option<CachedFrame>,
    stats: CompositorStats,
}

impl SoftwareCompositor {
    pub fn render(&mut self, scene: &SceneSnapshot) -> Result<Frame, CompositorError> {
        if scene.raster_configuration.backend == openui_geometry::RasterBackend::GaneshGl {
            return Err(CompositorError::Raster(
                "Ganesh scene requires GaneshGlCompositor".into(),
            ));
        }
        self.stats.submitted += 1;
        if let Some(frame) = self
            .last_frame
            .as_ref()
            .and_then(|cached| cached.frame_for(scene))
        {
            self.stats.reused += 1;
            return Ok(frame);
        }
        let mut surface = rasterize_picture(&scene.picture).map_err(CompositorError::Raster)?;
        let image = surface.image_snapshot();
        // Raster surfaces use platform-native N32 (BGRA on our Linux hosts).
        // Native apps and both Linux presenters consume explicit RGBA bytes.
        let info = image
            .image_info()
            .with_color_type(skia_safe::ColorType::RGBA8888);
        let width = scene.viewport.physical_width();
        let height = scene.viewport.physical_height();
        let stride = (width as usize)
            .checked_mul(4)
            .ok_or_else(|| CompositorError::Raster("frame stride overflow".into()))?;
        let pixel_len = stride
            .checked_mul(height as usize)
            .ok_or_else(|| CompositorError::Raster("frame allocation overflow".into()))?;
        let mut pixels = vec![0; pixel_len];
        if !image.read_pixels(&info, &mut pixels, stride, (0, 0), CachingHint::Allow) {
            return Err(CompositorError::ReadPixels);
        }
        let frame = Frame {
            width,
            height,
            viewport: scene.viewport,
            raster_configuration: scene.raster_configuration,
            raster_backend_identity: RasterBackendIdentity::cpu_skia(),
            stride,
            pixels,
            scene_generation: scene.generation,
        };
        self.stats.rasterized += 1;
        self.last_presented = Some(scene.generation);
        self.last_frame = Some(CachedFrame::new(scene, frame.clone()));
        Ok(frame)
    }

    pub fn last_presented(&self) -> Option<SceneGeneration> {
        self.last_presented
    }

    pub fn stats(&self) -> CompositorStats {
        self.stats
    }

    pub fn render_png(&mut self, scene: &SceneSnapshot) -> Result<Vec<u8>, CompositorError> {
        if scene.raster_configuration.backend == openui_geometry::RasterBackend::GaneshGl {
            return Err(CompositorError::Raster(
                "Ganesh scene requires GaneshGlCompositor".into(),
            ));
        }
        self.stats.submitted += 1;
        let mut surface = rasterize_picture(&scene.picture).map_err(CompositorError::Raster)?;
        let data = surface
            .image_snapshot()
            .encode(None, EncodedImageFormat::PNG, None)
            .ok_or(CompositorError::ReadPixels)?;
        self.last_presented = Some(scene.generation);
        self.last_frame = None;
        self.stats.rasterized += 1;
        Ok(data.as_bytes().to_vec())
    }
}

/// A one-slot scene mailbox for a render thread.
///
/// Submissions replace older unconsumed snapshots, ensuring the renderer
/// always receives the newest complete immutable scene and never observes a
/// partially-mutated document.
#[derive(Default)]
pub struct SceneMailbox {
    state: Mutex<MailboxState>,
    ready: Condvar,
}

#[derive(Default)]
struct MailboxState {
    latest: Option<SceneSnapshot>,
    closed: bool,
    stats: CompositorStats,
}

impl SceneMailbox {
    pub fn submit(&self, scene: SceneSnapshot) -> bool {
        let mut state = self.state.lock().expect("scene mailbox poisoned");
        if state.closed {
            return false;
        }
        state.stats.submitted += 1;
        if state.latest.replace(scene).is_some() {
            state.stats.dropped_before_present += 1;
        }
        self.ready.notify_one();
        true
    }

    pub fn take_latest(&self) -> Option<SceneSnapshot> {
        self.state
            .lock()
            .expect("scene mailbox poisoned")
            .latest
            .take()
    }

    pub fn wait_latest(&self) -> Option<SceneSnapshot> {
        let mut state = self.state.lock().expect("scene mailbox poisoned");
        while state.latest.is_none() && !state.closed {
            state = self.ready.wait(state).expect("scene mailbox poisoned");
        }
        state.latest.take()
    }

    pub fn close(&self) {
        let mut state = self.state.lock().expect("scene mailbox poisoned");
        state.closed = true;
        self.ready.notify_all();
    }

    pub fn stats(&self) -> CompositorStats {
        self.state.lock().expect("scene mailbox poisoned").stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recorded_scene(
        viewport: ViewportMetrics,
        configuration: RasterConfiguration,
    ) -> SceneSnapshot {
        let mut document = openui_dom::Document::new();
        document.set_raster_context(configuration, viewport.device_scale_factor());
        let (fragment, picture) = openui_paint::record_document(&document, viewport).unwrap();
        SceneSnapshot::new(SceneGeneration(1), picture, Arc::new(fragment), Vec::new())
    }

    #[test]
    fn immutable_scenes_can_cross_render_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SceneSnapshot>();
    }

    #[test]
    fn mailbox_coalesces_to_the_newest_complete_scene() {
        use openui_dom::Document;
        use openui_geometry::ViewportMetrics;
        use openui_paint::record_document;

        let document = Document::new();
        let viewport = ViewportMetrics::from_logical_size(10.0, 10.0, 1.0).unwrap();
        let (fragment, first) = record_document(&document, viewport).unwrap();
        let first = SceneSnapshot::new(
            SceneGeneration(1),
            first,
            Arc::new(fragment.clone()),
            Vec::new(),
        );
        let (_, second) = record_document(&document, viewport).unwrap();
        let second = SceneSnapshot::new(SceneGeneration(2), second, Arc::new(fragment), Vec::new());
        let mailbox = SceneMailbox::default();
        assert!(mailbox.submit(first));
        assert!(mailbox.submit(second));
        assert_eq!(
            mailbox.take_latest().unwrap().generation(),
            SceneGeneration(2)
        );
        assert_eq!(mailbox.stats().dropped_before_present, 1);
    }

    #[test]
    fn software_compositor_reuses_unchanged_frames() {
        use openui_dom::Document;
        use openui_geometry::ViewportMetrics;
        use openui_paint::record_document;

        let document = Document::new();
        let viewport = ViewportMetrics::from_logical_size(10.0, 10.0, 1.0).unwrap();
        let (fragment, picture) = record_document(&document, viewport).unwrap();
        let scene = SceneSnapshot::new(SceneGeneration(1), picture, Arc::new(fragment), Vec::new());
        let mut compositor = SoftwareCompositor::default();
        assert_eq!(
            compositor.render(&scene).unwrap(),
            compositor.render(&scene).unwrap()
        );
        assert_eq!(compositor.stats().rasterized, 1);
        assert_eq!(compositor.stats().reused, 1);
    }

    #[test]
    fn software_frame_cache_accepts_new_viewports_with_equal_generations() {
        let first = recorded_scene(
            ViewportMetrics::from_logical_size(10.0, 10.0, 1.0).unwrap(),
            RasterConfiguration::default(),
        );
        let second = recorded_scene(
            ViewportMetrics::from_logical_size(16.0, 12.0, 2.0).unwrap(),
            RasterConfiguration::chromium_linux_lcd(),
        );
        let mut compositor = SoftwareCompositor::default();
        assert_eq!(compositor.render(&first).unwrap().width, 10);
        let frame = compositor.render(&second).unwrap();
        assert_eq!((frame.width, frame.height), (32, 24));
        assert_eq!(frame.viewport, second.viewport());
        assert_eq!(frame.raster_configuration, second.raster_configuration());
        assert_eq!(compositor.render(&second.clone()).unwrap(), frame);
        assert_eq!(compositor.stats().rasterized, 2);
        assert_eq!(compositor.stats().reused, 1);
    }

    #[test]
    fn frame_cache_releases_recordings_on_replacement_png_and_drop() {
        let viewport = ViewportMetrics::from_logical_size(10.0, 10.0, 1.0).unwrap();
        let first = recorded_scene(viewport, RasterConfiguration::default());
        let first_picture = Arc::downgrade(&first.picture);
        let mut compositor = SoftwareCompositor::default();
        compositor.render(&first).unwrap();
        drop(first);
        assert!(first_picture.upgrade().is_some());

        let second = recorded_scene(viewport, RasterConfiguration::default());
        let second_picture = Arc::downgrade(&second.picture);
        compositor.render(&second).unwrap();
        assert!(first_picture.upgrade().is_none());
        drop(second);
        assert!(second_picture.upgrade().is_some());

        let third = recorded_scene(viewport, RasterConfiguration::default());
        let third_picture = Arc::downgrade(&third.picture);
        compositor.render_png(&third).unwrap();
        assert!(second_picture.upgrade().is_none());
        drop(third);
        assert!(third_picture.upgrade().is_none());

        let fourth = recorded_scene(viewport, RasterConfiguration::default());
        let fourth_picture = Arc::downgrade(&fourth.picture);
        compositor.render(&fourth).unwrap();
        drop(fourth);
        assert!(fourth_picture.upgrade().is_some());
        drop(compositor);
        assert!(fourth_picture.upgrade().is_none());
    }

    #[cfg(feature = "ganesh-gl")]
    #[test]
    fn ganesh_frame_cache_distinguishes_recordings_and_rejects_cpu_scenes() {
        let first = recorded_scene(
            ViewportMetrics::from_logical_size(10.0, 10.0, 1.0).unwrap(),
            RasterConfiguration::chromium_linux_ganesh(),
        );
        let second = recorded_scene(
            ViewportMetrics::from_logical_size(16.0, 12.0, 2.0).unwrap(),
            RasterConfiguration::chromium_linux_ganesh(),
        );
        let cpu = recorded_scene(first.viewport(), RasterConfiguration::default());
        let mut compositor = GaneshGlCompositor::new().unwrap();
        let first_frame = compositor.render(&first).unwrap();
        assert_eq!(compositor.render(&first.clone()).unwrap(), first_frame);
        assert!(matches!(
            compositor.render(&cpu),
            Err(CompositorError::Raster(_))
        ));
        assert_eq!(compositor.stats().rasterized, 1);
        assert_eq!(compositor.stats().reused, 1);

        let second_frame = compositor.render(&second).unwrap();
        assert_eq!((second_frame.width, second_frame.height), (32, 24));
        assert_eq!(compositor.render(&second.clone()).unwrap(), second_frame);
        assert_eq!(compositor.stats().rasterized, 2);
        assert_eq!(compositor.stats().reused, 2);
        compositor.render_png(&first).unwrap();
        assert_eq!(compositor.render(&second).unwrap(), second_frame);
        assert_eq!(compositor.stats().rasterized, 4);
        assert_eq!(compositor.stats().reused, 2);
    }

    #[test]
    fn damage_is_converted_outward_to_physical_pixels() {
        use openui_dom::Document;
        use openui_geometry::ViewportMetrics;
        use openui_paint::record_document;

        let document = Document::new();
        let viewport = ViewportMetrics::from_logical_size(10.0, 10.0, 1.25).unwrap();
        let (fragment, picture) = record_document(&document, viewport).unwrap();
        let scene = SceneSnapshot::new(
            SceneGeneration(1),
            picture,
            Arc::new(fragment),
            vec![SceneRect {
                x: 0.25,
                y: 1.25,
                width: 2.0,
                height: 3.0,
            }],
        );
        assert_eq!(
            scene.physical_damage(),
            &[PhysicalSceneRect {
                x: 0,
                y: 1,
                width: 3,
                height: 5,
            }]
        );
    }
}
