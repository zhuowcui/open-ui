//! Immutable, backend-independent scene snapshots.

use openui_layout::Fragment;
use openui_paint::{rasterize_picture, RecordedPicture};
use skia_safe::image::CachingHint;
use std::sync::Arc;

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
}

impl SoftwareCompositor {
    pub fn render(&mut self, scene: &SceneSnapshot) -> Result<Frame, CompositorError> {
        let mut surface = rasterize_picture(&scene.picture).map_err(CompositorError::Raster)?;
        let image = surface.image_snapshot();
        let info = image.image_info();
        let stride = scene.viewport.0 as usize * 4;
        let mut pixels = vec![0; stride * scene.viewport.1 as usize];
        if !image.read_pixels(&info, &mut pixels, stride, (0, 0), CachingHint::Allow) {
            return Err(CompositorError::ReadPixels);
        }
        self.last_presented = Some(scene.generation);
        Ok(Frame {
            width: scene.viewport.0,
            height: scene.viewport.1,
            stride,
            pixels,
            scene_generation: scene.generation,
        })
    }

    pub fn last_presented(&self) -> Option<SceneGeneration> {
        self.last_presented
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
}
