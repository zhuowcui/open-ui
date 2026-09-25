//! Explicit offscreen EGL/Mesa Ganesh compositor.

use super::{
    CompositorError, CompositorStats, Frame, RasterBackendIdentity, SceneGeneration, SceneSnapshot,
};
use glutin::api::egl::{context::PossiblyCurrentContext, device::Device, display::Display};
use glutin::config::{ConfigSurfaceTypes, ConfigTemplateBuilder, GlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::display::GlDisplay;
use openui_geometry::RasterBackend;
use skia_safe::gpu::{self, Budgeted, SurfaceOrigin, SyncCpu};
use skia_safe::{
    images, AlphaType, Color, ColorSpace, ColorType, Data, EncodedImageFormat, ImageInfo,
};
use std::ffi::CStr;

pub struct GaneshGlCompositor {
    // Field order is intentional: Ganesh must be destroyed before EGL.
    direct_context: gpu::DirectContext,
    _gl_context: PossiblyCurrentContext,
    identity: RasterBackendIdentity,
    last_presented: Option<SceneGeneration>,
    last_frame: Option<Frame>,
    stats: CompositorStats,
}

impl GaneshGlCompositor {
    pub fn new() -> Result<Self, CompositorError> {
        let mut devices = Device::query_devices()
            .map_err(raster_error)?
            .collect::<Vec<_>>();
        devices.sort_by_key(|device| {
            (
                device.vendor().unwrap_or_default().to_owned(),
                device.name().unwrap_or_default().to_owned(),
                device
                    .drm_render_device_node_path()
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            )
        });
        let device = devices
            .into_iter()
            .next()
            .ok_or_else(|| CompositorError::Raster("EGL exposed no devices".into()))?;
        // SAFETY: the EGL device is process-owned and glutin retains the
        // display for every object created from it.
        let display = unsafe { Display::with_device(&device, None) }.map_err(raster_error)?;
        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_stencil_size(8)
            .with_surface_type(ConfigSurfaceTypes::PBUFFER)
            .build();
        // SAFETY: the template contains no native window handle and the
        // returned configurations remain owned by `display`.
        let configurations = unsafe { display.find_configs(template) }.map_err(raster_error)?;
        let config = configurations
            .min_by_key(|config| {
                (
                    config.num_samples(),
                    config.depth_size(),
                    config.stencil_size(),
                    config.alpha_size(),
                )
            })
            .ok_or_else(|| CompositorError::Raster("EGL exposed no RGBA pbuffer config".into()))?;
        let desktop = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 2))))
            .build(None);
        let embedded = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(Some(Version::new(3, 0))))
            .build(None);
        // SAFETY: both context descriptions are headless and refer only to
        // the retained EGL configuration.
        let context = unsafe {
            display
                .create_context(&config, &desktop)
                .or_else(|_| display.create_context(&config, &embedded))
        }
        .map_err(raster_error)?
        .make_current_surfaceless()
        .map_err(raster_error)?;
        gl::load_with(|name| {
            std::ffi::CString::new(name)
                .ok()
                .map_or(std::ptr::null(), |name| display.get_proc_address(&name))
        });
        let interface =
            gpu::gl::Interface::new_load_with_cstr(|name| display.get_proc_address(name))
                .ok_or_else(|| {
                    CompositorError::Raster("Skia rejected the EGL GL interface".into())
                })?;
        let direct_context = gpu::direct_contexts::make_gl(interface, None)
            .ok_or_else(|| CompositorError::Raster("could not create Ganesh GL context".into()))?;
        let gl_vendor = gl_string(gl::VENDOR)?;
        let gl_renderer = gl_string(gl::RENDERER)?;
        let gl_version = gl_string(gl::VERSION)?;
        if !format!("{gl_vendor} {gl_renderer}")
            .to_ascii_lowercase()
            .contains("mesa")
            && !gl_renderer.to_ascii_lowercase().contains("llvmpipe")
            && !gl_renderer.to_ascii_lowercase().contains("softpipe")
        {
            return Err(CompositorError::Raster(format!(
                "qualification Ganesh requires Mesa, got {gl_vendor} / {gl_renderer}"
            )));
        }
        let driver = format!(
            "{}; EGL device vendor={}; name={}; node={}",
            display.version_string(),
            device.vendor().unwrap_or("unknown"),
            device.name().unwrap_or("unknown"),
            device
                .drm_render_device_node_path()
                .map(|path| path.to_string_lossy())
                .unwrap_or_else(|| "none".into()),
        );
        Ok(Self {
            direct_context,
            _gl_context: context,
            identity: RasterBackendIdentity {
                backend: "ganesh-gl",
                gl_renderer: Some(gl_renderer),
                gl_version: Some(gl_version),
                driver: Some(driver),
                color_type: "RGBA8888-premultiplied-sRGB",
                sample_count: 0,
                surface_properties: "scene-raster-configuration-v1",
            },
            last_presented: None,
            last_frame: None,
            stats: CompositorStats::default(),
        })
    }

    pub fn identity(&self) -> &RasterBackendIdentity {
        &self.identity
    }

    pub fn stats(&self) -> CompositorStats {
        self.stats
    }

    fn render_surface(
        &mut self,
        scene: &SceneSnapshot,
    ) -> Result<skia_safe::Surface, CompositorError> {
        if scene.raster_configuration.backend != RasterBackend::GaneshGl {
            return Err(CompositorError::Raster(
                "CPU scene cannot be replayed by GaneshGlCompositor".into(),
            ));
        }
        let width = i32::try_from(scene.viewport.physical_width())
            .map_err(|_| CompositorError::Raster("frame width exceeds Skia limit".into()))?;
        let height = i32::try_from(scene.viewport.physical_height())
            .map_err(|_| CompositorError::Raster("frame height exceeds Skia limit".into()))?;
        let image_info = ImageInfo::new(
            (width, height),
            ColorType::RGBA8888,
            AlphaType::Premul,
            Some(ColorSpace::new_srgb()),
        );
        let props = openui_paint::raster_surface_properties(
            scene.raster_configuration,
            scene.picture.uses_lcd_surface(),
        );
        let mut surface = gpu::surfaces::render_target(
            &mut self.direct_context,
            Budgeted::No,
            &image_info,
            Some(0),
            Some(SurfaceOrigin::TopLeft),
            props.as_ref(),
            Some(false),
            Some(false),
        )
        .ok_or_else(|| CompositorError::Raster("could not allocate Ganesh surface".into()))?;
        surface.canvas().clear(Color::WHITE);
        scene.replay(surface.canvas());
        self.direct_context
            .flush_and_submit_surface(&mut surface, SyncCpu::Yes);
        Ok(surface)
    }

    pub fn render(&mut self, scene: &SceneSnapshot) -> Result<Frame, CompositorError> {
        self.stats.submitted += 1;
        if self.last_presented == Some(scene.generation) {
            if let Some(frame) = self.last_frame.clone() {
                self.stats.reused += 1;
                return Ok(frame);
            }
        }
        let mut surface = self.render_surface(scene)?;
        let width = scene.viewport.physical_width();
        let height = scene.viewport.physical_height();
        let stride = width as usize * 4;
        let mut pixels = vec![0; stride * height as usize];
        if !surface.read_pixels(
            &ImageInfo::new(
                (width as i32, height as i32),
                ColorType::RGBA8888,
                AlphaType::Premul,
                Some(ColorSpace::new_srgb()),
            ),
            &mut pixels,
            stride,
            (0, 0),
        ) {
            return Err(CompositorError::ReadPixels);
        }
        let frame = Frame {
            width,
            height,
            viewport: scene.viewport,
            raster_configuration: scene.raster_configuration,
            raster_backend_identity: self.identity.clone(),
            stride,
            pixels,
            scene_generation: scene.generation,
        };
        self.stats.rasterized += 1;
        self.last_presented = Some(scene.generation);
        self.last_frame = Some(frame.clone());
        Ok(frame)
    }

    pub fn render_png(&mut self, scene: &SceneSnapshot) -> Result<Vec<u8>, CompositorError> {
        self.stats.submitted += 1;
        let mut surface = self.render_surface(scene)?;
        let width = scene.viewport.physical_width();
        let height = scene.viewport.physical_height();
        let stride = width as usize * 4;
        let info = ImageInfo::new(
            (width as i32, height as i32),
            ColorType::RGBA8888,
            AlphaType::Premul,
            Some(ColorSpace::new_srgb()),
        );
        let mut pixels = vec![0; stride * height as usize];
        if !surface.read_pixels(&info, &mut pixels, stride, (0, 0)) {
            return Err(CompositorError::ReadPixels);
        }
        let data = images::raster_from_data(&info, Data::new_copy(&pixels), stride)
            .ok_or(CompositorError::ReadPixels)?
            .encode(None, EncodedImageFormat::PNG, None)
            .ok_or(CompositorError::ReadPixels)?;
        self.stats.rasterized += 1;
        self.last_presented = Some(scene.generation);
        self.last_frame = None;
        Ok(data.as_bytes().to_vec())
    }
}

fn raster_error(error: impl std::fmt::Display) -> CompositorError {
    CompositorError::Raster(error.to_string())
}

fn gl_string(name: gl::types::GLenum) -> Result<String, CompositorError> {
    // SAFETY: an EGL context is current for this compositor's construction
    // thread, and OpenGL owns the returned static NUL-terminated bytes.
    let value = unsafe { gl::GetString(name) };
    if value.is_null() {
        return Err(CompositorError::Raster(format!(
            "OpenGL string {name:#x} is unavailable"
        )));
    }
    Ok(unsafe { CStr::from_ptr(value.cast()) }
        .to_string_lossy()
        .into_owned())
}
