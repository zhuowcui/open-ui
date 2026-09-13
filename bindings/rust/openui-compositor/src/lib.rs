//! Immutable, backend-independent scene snapshots.

use openui_layout::Fragment;
use openui_paint::{rasterize_picture, RecordedPicture};
use skia_safe::image::CachingHint;
use skia_safe::{Canvas, EncodedImageFormat};
use std::sync::{Arc, Condvar, Mutex};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneGeneration(pub u64);

/// A complete immutable frame. It contains no callbacks or mutable DOM state.
#[derive(Clone)]
pub struct SceneSnapshot {
    generation: SceneGeneration,
    viewport: (u32, u32),
    picture: Arc<RecordedPicture>,
    fragments: Arc<Fragment>,
    damage: Arc<[SceneRect]>,
}

impl SceneSnapshot {
    pub fn new(
        generation: SceneGeneration,
        picture: RecordedPicture,
        fragments: Arc<Fragment>,
        damage: Vec<SceneRect>,
    ) -> Self {
        let viewport = (picture.width as u32, picture.height as u32);
        Self {
            generation,
            viewport,
            picture: Arc::new(picture),
            fragments,
            damage: damage.into(),
        }
    }

    pub fn generation(&self) -> SceneGeneration {
        self.generation
    }
    pub fn viewport(&self) -> (u32, u32) {
        self.viewport
    }
    pub fn fragments(&self) -> &Arc<Fragment> {
        &self.fragments
    }
    pub fn damage(&self) -> &[SceneRect] {
        &self.damage
    }

    /// Replay this immutable scene into a caller-owned Skia canvas.
    ///
    /// GPU and software presenters use this exact recording, so backend
    /// selection cannot affect layout or scene construction.
    pub fn replay(&self, canvas: &Canvas) {
        canvas.draw_picture(&self.picture.picture, None, None);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub stride: usize,
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

#[derive(Default)]
pub struct SoftwareCompositor {
    last_presented: Option<SceneGeneration>,
    last_frame: Option<Frame>,
    stats: CompositorStats,
}

impl SoftwareCompositor {
    pub fn render(&mut self, scene: &SceneSnapshot) -> Result<Frame, CompositorError> {
        self.stats.submitted += 1;
        if self.last_presented == Some(scene.generation) {
            if let Some(frame) = self.last_frame.clone() {
                self.stats.reused += 1;
                return Ok(frame);
            }
        }
        let mut surface = rasterize_picture(&scene.picture).map_err(CompositorError::Raster)?;
        let image = surface.image_snapshot();
        let info = image.image_info();
        let stride = scene.viewport.0 as usize * 4;
        let mut pixels = vec![0; stride * scene.viewport.1 as usize];
        if !image.read_pixels(info, &mut pixels, stride, (0, 0), CachingHint::Allow) {
            return Err(CompositorError::ReadPixels);
        }
        let frame = Frame {
            width: scene.viewport.0,
            height: scene.viewport.1,
            stride,
            pixels,
            scene_generation: scene.generation,
        };
        self.stats.rasterized += 1;
        self.last_presented = Some(scene.generation);
        self.last_frame = Some(frame.clone());
        Ok(frame)
    }

    pub fn last_presented(&self) -> Option<SceneGeneration> {
        self.last_presented
    }

    pub fn stats(&self) -> CompositorStats {
        self.stats
    }

    pub fn render_png(&mut self, scene: &SceneSnapshot) -> Result<Vec<u8>, CompositorError> {
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

    #[test]
    fn immutable_scenes_can_cross_render_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SceneSnapshot>();
    }

    #[test]
    fn mailbox_coalesces_to_the_newest_complete_scene() {
        use openui_dom::Document;
        use openui_paint::record_document;

        let document = Document::new();
        let (fragment, first) = record_document(&document, 10, 10).unwrap();
        let first = SceneSnapshot::new(
            SceneGeneration(1),
            first,
            Arc::new(fragment.clone()),
            Vec::new(),
        );
        let (_, second) = record_document(&document, 10, 10).unwrap();
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
        use openui_paint::record_document;

        let document = Document::new();
        let (fragment, picture) = record_document(&document, 10, 10).unwrap();
        let scene = SceneSnapshot::new(SceneGeneration(1), picture, Arc::new(fragment), Vec::new());
        let mut compositor = SoftwareCompositor::default();
        assert_eq!(
            compositor.render(&scene).unwrap(),
            compositor.render(&scene).unwrap()
        );
        assert_eq!(compositor.stats().rasterized, 1);
        assert_eq!(compositor.stats().reused, 1);
    }
}
