//! Safe, single-thread-affine document API over `openui-engine`.

use crate::element::Element;
use crate::events::{
    Event, EventPhase, KeyEventType, Listener, Modifiers, MouseButton, MouseEventType,
};
use crate::style::{Bitmap, Error};
use openui_compositor::SoftwareCompositor;
use openui_engine::{Engine, NodeHandle, Viewport};
use openui_style::ImageResourceId;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

type ListenerKey = (NodeHandle, String);
type ResourceProvider = dyn Fn(&str) -> Option<Vec<u8>>;

pub(crate) struct DocumentInner {
    pub engine: RefCell<Engine>,
    pub listeners: RefCell<HashMap<ListenerKey, Vec<Listener>>>,
    pub resource_provider: RefCell<Option<Box<ResourceProvider>>>,
    transaction_depth: Cell<usize>,
}

/// Cloneable owner reference for one retained native document.
#[derive(Clone)]
pub struct Document {
    pub(crate) inner: Rc<DocumentInner>,
}

impl Document {
    pub fn new(width: i32, height: i32) -> Result<Self, Error> {
        let width = u32::try_from(width).map_err(|_| Error::InvalidArgument("invalid width"))?;
        let height = u32::try_from(height).map_err(|_| Error::InvalidArgument("invalid height"))?;
        Self::with_viewport(Viewport::new(width, height)?)
    }

    pub fn with_viewport(viewport: Viewport) -> Result<Self, Error> {
        Ok(Self {
            inner: Rc::new(DocumentInner {
                engine: RefCell::new(Engine::new(viewport)?),
                listeners: RefCell::new(HashMap::new()),
                resource_provider: RefCell::new(None),
                transaction_depth: Cell::new(0),
            }),
        })
    }

    pub fn body(&self) -> Element {
        let handle = self.inner.engine.borrow().root();
        Element::from_handle(self.clone(), handle)
    }

    pub fn transaction<T>(
        &self,
        operation: impl FnOnce(&Document) -> Result<T, Error>,
    ) -> Result<T, Error> {
        struct DepthGuard<'a> {
            depth: &'a Cell<usize>,
        }
        impl Drop for DepthGuard<'_> {
            fn drop(&mut self) {
                self.depth.set(self.depth.get() - 1);
            }
        }

        self.inner
            .transaction_depth
            .set(self.inner.transaction_depth.get() + 1);
        let _guard = DepthGuard {
            depth: &self.inner.transaction_depth,
        };
        operation(self)
    }

    pub fn set_viewport(&self, width: u32, height: u32) -> Result<(), Error> {
        self.with_engine_mut(|engine| engine.set_viewport(Viewport::new(width, height)?))
    }

    pub fn update_all(&self) -> Result<(), Error> {
        self.with_engine_mut(|engine| engine.update().map(|_| ()))
    }

    pub fn render_to_bitmap(&self) -> Result<Bitmap, Error> {
        let scene = self.with_engine_mut(|engine| engine.scene())?;
        let frame = SoftwareCompositor::default().render(&scene)?;
        Ok(Bitmap {
            pixels: frame.pixels,
            width: frame.width,
            height: frame.height,
            stride: frame.stride,
        })
    }

    pub fn render_to_png_buffer(&self) -> Result<Vec<u8>, Error> {
        let scene = self.with_engine_mut(|engine| engine.scene())?;
        Ok(SoftwareCompositor::default().render_png(&scene)?)
    }

    pub fn render_to_png(&self, path: impl AsRef<std::path::Path>) -> Result<(), Error> {
        std::fs::write(path, self.render_to_png_buffer()?)?;
        Ok(())
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Result<Option<Element>, Error> {
        let handle = self.with_engine_mut(|engine| engine.hit_test(x, y))?;
        Ok(handle.map(|handle| Element::from_handle(self.clone(), handle)))
    }

    pub fn advance_time(&self, time_ms: f64) -> Result<(), Error> {
        self.with_engine_mut(|engine| engine.set_animation_time(time_ms))
    }

    pub fn advance_time_by(&self, delta_ms: f64) -> Result<(), Error> {
        if !delta_ms.is_finite() {
            return Err(Error::InvalidArgument("animation delta must be finite"));
        }
        self.with_engine_mut(|engine| {
            let next = engine.animation_time() + delta_ms;
            engine.set_animation_time(next)
        })
    }

    pub fn get_time(&self) -> Result<f64, Error> {
        self.with_engine(|engine| engine.animation_time())
    }

    pub fn begin_frame(&self, time_ms: f64) -> Result<(), Error> {
        self.advance_time(time_ms)?;
        self.update_all()
    }

    pub fn dispatch_mouse_event(
        &self,
        event_type: MouseEventType,
        x: f32,
        y: f32,
        button: MouseButton,
        modifiers: Modifiers,
    ) -> Result<(), Error> {
        let target = self.with_engine_mut(|engine| {
            if let Some(captured) = engine.pointer_capture(0) {
                Ok(Some(captured))
            } else {
                engine.hit_test(x, y)
            }
        })?;
        let Some(target) = target else {
            return Ok(());
        };
        let event = Event::pointer(event_type.name(), x, y, button, modifiers);
        self.dispatch_to(target, &event)?;
        if event_type == MouseEventType::Up && !event.default_prevented() {
            let click = Event::pointer("click", x, y, button, modifiers);
            self.dispatch_to(target, &click)?;
        }
        Ok(())
    }

    pub fn dispatch_key_event(
        &self,
        event_type: KeyEventType,
        key_code: i32,
        key_text: Option<&str>,
        modifiers: Modifiers,
    ) -> Result<(), Error> {
        let target =
            self.with_engine(|engine| engine.focused().unwrap_or_else(|| engine.root()))?;
        let event = Event::keyboard(event_type.name(), key_code, key_text, modifiers);
        self.dispatch_to(target, &event)
    }

    pub fn dispatch_wheel_event(
        &self,
        x: f32,
        y: f32,
        delta_x: f32,
        delta_y: f32,
        modifiers: Modifiers,
    ) -> Result<(), Error> {
        let target = self.with_engine_mut(|engine| engine.hit_test(x, y))?;
        if let Some(target) = target {
            self.dispatch_to(target, &Event::wheel(x, y, delta_x, delta_y, modifiers))?;
        }
        Ok(())
    }

    pub fn focused_element(&self) -> Result<Option<Element>, Error> {
        let handle = self.with_engine(|engine| engine.focused())?;
        Ok(handle.map(|handle| Element::from_handle(self.clone(), handle)))
    }

    pub fn advance_focus(&self, direction: i32) -> Result<(), Error> {
        if direction != -1 && direction != 1 {
            return Err(Error::InvalidArgument("focus direction must be -1 or 1"));
        }
        let candidates = self.focus_candidates()?;
        if candidates.is_empty() {
            return Ok(());
        }
        let focused = self.with_engine(|engine| engine.focused())?;
        let index = focused
            .and_then(|focused| candidates.iter().position(|item| *item == focused))
            .map(|index| {
                if direction > 0 {
                    (index + 1) % candidates.len()
                } else {
                    (index + candidates.len() - 1) % candidates.len()
                }
            })
            .unwrap_or_else(|| {
                if direction > 0 {
                    0
                } else {
                    candidates.len() - 1
                }
            });
        self.with_engine_mut(|engine| engine.focus(candidates[index]))
    }

    pub fn set_resource_provider<F>(&self, callback: F) -> Result<(), Error>
    where
        F: Fn(&str) -> Option<Vec<u8>> + 'static,
    {
        *self
            .inner
            .resource_provider
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantMutation)? = Some(Box::new(callback));
        Ok(())
    }

    pub fn register_image_resource(
        &self,
        source: impl Into<String>,
        mime_type: impl Into<String>,
        sha256: impl Into<String>,
        bytes: impl Into<Vec<u8>>,
    ) -> Result<ImageResourceId, Error> {
        let source = source.into();
        let mime_type = mime_type.into();
        let sha256 = sha256.into();
        let bytes = bytes.into();
        self.with_engine_mut(|engine| {
            Ok(engine.register_image_resource(source, mime_type, sha256, bytes))
        })
    }

    pub fn load_image_resource(
        &self,
        source: &str,
        mime_type: &str,
        sha256: &str,
    ) -> Result<Option<ImageResourceId>, Error> {
        let bytes = {
            let provider = self
                .inner
                .resource_provider
                .try_borrow()
                .map_err(|_| Error::ReentrantMutation)?;
            provider.as_ref().and_then(|provider| provider(source))
        };
        bytes
            .map(|bytes| self.register_image_resource(source, mime_type, sha256, bytes))
            .transpose()
    }

    pub(crate) fn add_listener(
        &self,
        handle: NodeHandle,
        event_type: &str,
        listener: Listener,
    ) -> Result<(), Error> {
        self.with_engine(|engine| handle.downgrade().upgrade(engine))??;
        self.inner
            .listeners
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantMutation)?
            .entry((handle, event_type.to_owned()))
            .or_default()
            .push(listener);
        Ok(())
    }

    pub(crate) fn remove_listeners(
        &self,
        handle: NodeHandle,
        event_type: &str,
    ) -> Result<(), Error> {
        self.inner
            .listeners
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantMutation)?
            .remove(&(handle, event_type.to_owned()));
        Ok(())
    }

    pub(crate) fn remove_node(&self, handle: NodeHandle) -> Result<(), Error> {
        let handles = self.subtree_handles(handle)?;
        self.with_engine_mut(|engine| engine.remove(handle))?;
        self.inner
            .listeners
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantMutation)?
            .retain(|(node, _), _| !handles.contains(node));
        Ok(())
    }

    pub(crate) fn with_engine<T>(&self, f: impl FnOnce(&Engine) -> T) -> Result<T, Error> {
        let engine = self
            .inner
            .engine
            .try_borrow()
            .map_err(|_| Error::ReentrantMutation)?;
        Ok(f(&engine))
    }

    pub(crate) fn with_engine_mut<T>(
        &self,
        f: impl FnOnce(&mut Engine) -> Result<T, openui_engine::EngineError>,
    ) -> Result<T, Error> {
        let mut engine = self
            .inner
            .engine
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantMutation)?;
        Ok(f(&mut engine)?)
    }

    fn subtree_handles(&self, root: NodeHandle) -> Result<Vec<NodeHandle>, Error> {
        self.with_engine(|engine| {
            fn collect(engine: &Engine, node: NodeHandle, result: &mut Vec<NodeHandle>) {
                result.push(node);
                if let Ok(children) = engine.children(node) {
                    for child in children {
                        collect(engine, child, result);
                    }
                }
            }
            let mut result = Vec::new();
            collect(engine, root, &mut result);
            result
        })
    }

    fn focus_candidates(&self) -> Result<Vec<NodeHandle>, Error> {
        self.with_engine(|engine| {
            fn collect(engine: &Engine, node: NodeHandle, result: &mut Vec<NodeHandle>) {
                let focusable_tag = engine.element_tag(node).is_ok_and(|tag| {
                    matches!(
                        tag,
                        openui_dom::ElementTag::Button
                            | openui_dom::ElementTag::Input
                            | openui_dom::ElementTag::TextArea
                            | openui_dom::ElementTag::Select
                    )
                });
                let explicit = engine
                    .attribute(node, "tabindex")
                    .ok()
                    .flatten()
                    .and_then(|value| value.parse::<i32>().ok())
                    .is_some_and(|value| value >= 0);
                if focusable_tag || explicit {
                    result.push(node);
                }
                if let Ok(children) = engine.children(node) {
                    for child in children {
                        collect(engine, child, result);
                    }
                }
            }
            let mut result = Vec::new();
            collect(engine, engine.root(), &mut result);
            result
        })
    }

    fn dispatch_to(&self, target: NodeHandle, event: &Event) -> Result<(), Error> {
        let path = self.with_engine(|engine| {
            let mut path = vec![target];
            let mut current = target;
            while let Ok(Some(parent)) = engine.parent(current) {
                path.push(parent);
                current = parent;
            }
            path
        })?;

        for node in path.iter().skip(1).rev() {
            event.set_phase(EventPhase::Capture);
            self.invoke(*node, event, Some(true))?;
            if event.propagation_stopped() {
                return Ok(());
            }
        }
        event.set_phase(EventPhase::Target);
        self.invoke(target, event, Some(true))?;
        self.invoke(target, event, Some(false))?;
        if event.propagation_stopped() {
            return Ok(());
        }
        for node in path.iter().skip(1) {
            event.set_phase(EventPhase::Bubble);
            self.invoke(*node, event, Some(false))?;
            if event.propagation_stopped() {
                break;
            }
        }
        Ok(())
    }

    fn invoke(&self, node: NodeHandle, event: &Event, capture: Option<bool>) -> Result<(), Error> {
        let callbacks: Vec<_> = self
            .inner
            .listeners
            .try_borrow()
            .map_err(|_| Error::ReentrantMutation)?
            .get(&(node, event.event_type.clone()))
            .into_iter()
            .flat_map(|listeners| listeners.iter())
            .filter(|listener| capture.is_none_or(|capture| listener.capture == capture))
            .map(|listener| listener.callback.clone())
            .collect();
        for callback in callbacks {
            callback(event);
        }
        Ok(())
    }
}
