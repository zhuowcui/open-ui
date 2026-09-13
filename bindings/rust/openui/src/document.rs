//! Safe, single-thread-affine document API over `openui-engine`.

use crate::element::Element;
use crate::events::{
    Event, EventPhase, KeyEventType, Listener, Modifiers, MouseButton, MouseEventType,
};
use crate::style::{Bitmap, Error};
use openui_compositor::SoftwareCompositor;
use openui_dom::FormControlRole;
use openui_engine::{
    AccessibilityAction, AccessibilityTreeUpdate, AnimationEvent, AnimationEventKind, AnimationId,
    AnimationState, ControlAdjustment, EditCommand, Engine, EventPhase as EngineEventPhase,
    FocusOrigin, NodeHandle, PointerEventKind, ScrollAnimationId, TextDirection, TextUnit,
    Viewport,
};
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
    pub clipboard: RefCell<String>,
    animation_events: RefCell<Vec<AnimationEvent>>,
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
                clipboard: RefCell::new(String::new()),
                animation_events: RefCell::new(Vec::new()),
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

    pub fn set_viewport_with_scale(
        &self,
        width: u32,
        height: u32,
        scale_factor: f64,
    ) -> Result<(), Error> {
        let mut viewport = Viewport::new(width, height)?;
        viewport.scale_factor = scale_factor;
        self.with_engine_mut(|engine| engine.set_viewport(viewport))
    }

    pub fn update_all(&self) -> Result<(), Error> {
        self.with_engine_mut(|engine| engine.update().map(|_| ()))
    }

    pub fn accessibility_update(&self) -> Result<AccessibilityTreeUpdate, Error> {
        self.with_engine_mut(Engine::accessibility_update)
    }

    pub fn prefers_reduced_motion(&self) -> Result<bool, Error> {
        self.with_engine(Engine::prefers_reduced_motion)
    }

    pub fn set_prefers_reduced_motion(&self, reduced: bool) -> Result<(), Error> {
        self.with_engine_mut(|engine| {
            engine.set_prefers_reduced_motion(reduced);
            Ok(())
        })
    }

    pub(crate) fn perform_accessibility_action(
        &self,
        target: NodeHandle,
        action: AccessibilityAction,
    ) -> Result<(), Error> {
        if action == AccessibilityAction::Click {
            let event = Event::keyboard("click", 0, None, Modifiers::NONE);
            self.dispatch_to(target, &event)?;
            if event.default_prevented() {
                return Ok(());
            }
        }
        let previous_focus = self.with_engine(Engine::focused)?;
        let changes = self.with_engine_mut(|engine| {
            engine.perform_accessibility_action(target, action.clone())
        })?;
        let next_focus = self.with_engine(Engine::focused)?;
        if previous_focus != next_focus {
            self.dispatch_focus_change(previous_focus, next_focus)?;
        }
        if matches!(
            action,
            AccessibilityAction::Click
                | AccessibilityAction::Increment
                | AccessibilityAction::Decrement
                | AccessibilityAction::Expand
                | AccessibilityAction::Collapse
                | AccessibilityAction::SetValue(_)
                | AccessibilityAction::ReplaceSelectedText(_)
        ) {
            for changed in changes.changed {
                self.dispatch_to(changed, &Event::keyboard("input", 0, None, Modifiers::NONE))?;
                self.dispatch_to(
                    changed,
                    &Event::keyboard("change", 0, None, Modifiers::NONE),
                )?;
            }
        }
        Ok(())
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
        self.with_engine_mut(|engine| engine.set_animation_time(time_ms))?;
        self.dispatch_animation_events()
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

    pub fn is_animating(&self) -> Result<bool, Error> {
        self.with_engine(Engine::is_animating)
    }

    pub fn animation_state(&self, animation: AnimationId) -> Result<AnimationState, Error> {
        self.with_engine(|engine| engine.animation_state(animation))?
            .map_err(Into::into)
    }

    pub fn pause_animation(&self, animation: AnimationId) -> Result<(), Error> {
        self.with_engine_mut(|engine| engine.pause_animation(animation))
    }

    pub fn play_animation(&self, animation: AnimationId) -> Result<(), Error> {
        self.with_engine_mut(|engine| engine.play_animation(animation))
    }

    pub fn seek_animation(
        &self,
        animation: AnimationId,
        current_time_ms: f64,
    ) -> Result<(), Error> {
        self.with_engine_mut(|engine| engine.seek_animation(animation, current_time_ms))?;
        self.dispatch_animation_events()
    }

    pub fn set_animation_playback_rate(
        &self,
        animation: AnimationId,
        playback_rate: f64,
    ) -> Result<(), Error> {
        self.with_engine_mut(|engine| engine.set_animation_playback_rate(animation, playback_rate))
    }

    pub fn finish_animation(&self, animation: AnimationId) -> Result<(), Error> {
        self.with_engine_mut(|engine| engine.finish_animation(animation))?;
        self.dispatch_animation_events()
    }

    pub fn cancel_animation(&self, animation: AnimationId) -> Result<(), Error> {
        self.with_engine_mut(|engine| engine.cancel_animation(animation))?;
        self.dispatch_animation_events()
    }

    pub fn cancel_smooth_scroll(&self, animation: ScrollAnimationId) -> Result<(), Error> {
        self.with_engine_mut(|engine| engine.cancel_smooth_scroll(animation))
    }

    pub fn drain_animation_events(&self) -> Result<Vec<AnimationEvent>, Error> {
        let mut events = self
            .inner
            .animation_events
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantMutation)?;
        Ok(std::mem::take(&mut *events))
    }

    pub fn begin_frame(&self, time_ms: f64) -> Result<(), Error> {
        self.advance_time(time_ms)?;
        self.update_all()
    }

    fn dispatch_animation_events(&self) -> Result<(), Error> {
        let events = self.with_engine_mut(|engine| Ok(engine.drain_animation_events()))?;
        for animation_event in &events {
            if !self.with_engine(|engine| engine.event_route(animation_event.target).is_ok())? {
                continue;
            }
            let event_type = match animation_event.kind {
                AnimationEventKind::Start => "animationstart",
                AnimationEventKind::Iteration => "animationiteration",
                AnimationEventKind::End => "animationend",
                AnimationEventKind::Cancel => "animationcancel",
            };
            self.dispatch_to(
                animation_event.target,
                &Event::keyboard(event_type, 0, None, Modifiers::NONE),
            )?;
        }
        self.inner
            .animation_events
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantMutation)?
            .extend(events);
        Ok(())
    }

    pub fn dispatch_mouse_event(
        &self,
        event_type: MouseEventType,
        x: f32,
        y: f32,
        button: MouseButton,
        modifiers: Modifiers,
    ) -> Result<(), Error> {
        self.dispatch_pointer_event(
            0,
            match event_type {
                MouseEventType::Down => PointerEventKind::Down,
                MouseEventType::Up => PointerEventKind::Up,
                MouseEventType::Move => PointerEventKind::Move,
            },
            x,
            y,
            button,
            modifiers,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn dispatch_pointer_event(
        &self,
        pointer_id: u64,
        event_type: PointerEventKind,
        x: f32,
        y: f32,
        button: MouseButton,
        modifiers: Modifiers,
        pointer_names: bool,
    ) -> Result<(), Error> {
        let update =
            self.with_engine_mut(|engine| engine.pointer_event(pointer_id, event_type, x, y))?;
        let prefix = if pointer_names { "pointer" } else { "mouse" };
        for node in update.left {
            let event = Event::pointer(
                format!("{prefix}leave"),
                pointer_id,
                x,
                y,
                button,
                modifiers,
            );
            event.set_phase(EventPhase::Target);
            self.invoke(node, &event, None)?;
        }
        for node in update.entered {
            let event = Event::pointer(
                format!("{prefix}enter"),
                pointer_id,
                x,
                y,
                button,
                modifiers,
            );
            event.set_phase(EventPhase::Target);
            self.invoke(node, &event, None)?;
        }
        let Some(target) = update.target else {
            return Ok(());
        };
        let event_name = match (pointer_names, event_type) {
            (true, PointerEventKind::Down) => "pointerdown",
            (true, PointerEventKind::Up) => "pointerup",
            (true, PointerEventKind::Move) => "pointermove",
            (true, PointerEventKind::Cancel) => "pointercancel",
            (false, PointerEventKind::Down) => "mousedown",
            (false, PointerEventKind::Up) => "mouseup",
            (false, PointerEventKind::Move) => "mousemove",
            (false, PointerEventKind::Cancel) => "mousecancel",
        };
        let event = Event::pointer(event_name, pointer_id, x, y, button, modifiers);
        self.dispatch_to(target, &event)?;
        if event.default_prevented() {
            return Ok(());
        }
        if event_type == PointerEventKind::Down {
            self.focus_from(target, FocusOrigin::Pointer)?;
        }
        if let Some((range, fraction)) = update.range_value {
            if self.with_engine_mut(|engine| engine.set_range_fraction(range, fraction))? {
                self.dispatch_to(range, &Event::keyboard("input", 0, None, modifiers))?;
            }
            if event_type == PointerEventKind::Up {
                self.dispatch_to(range, &Event::keyboard("change", 0, None, modifiers))?;
            }
        }
        let activation = update
            .activation
            .or_else(|| (!pointer_names && event_type == PointerEventKind::Up).then_some(target));
        if let Some(activation) = activation {
            let click = Event::pointer("click", pointer_id, x, y, button, modifiers);
            self.dispatch_to(activation, &click)?;
            if !click.default_prevented() {
                let changed = self.with_engine_mut(|engine| engine.activate(activation))?;
                for changed in changed.changed {
                    self.dispatch_to(changed, &Event::keyboard("input", 0, None, modifiers))?;
                    self.dispatch_to(changed, &Event::keyboard("change", 0, None, modifiers))?;
                }
            }
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
        self.dispatch_to(target, &event)?;
        if event.default_prevented() {
            return Ok(());
        }
        if event_type == KeyEventType::Char {
            if let Some(text) = key_text.filter(|text| !text.is_empty()) {
                if self
                    .with_engine_mut(|engine| engine.insert_text(target, text))
                    .is_ok()
                {
                    self.dispatch_to(target, &Event::keyboard("input", 0, Some(text), modifiers))?;
                }
            }
            return Ok(());
        }
        if event_type != KeyEventType::Down {
            return Ok(());
        }
        let key = key_text.unwrap_or_default();
        if key_code == 9 || key.eq_ignore_ascii_case("tab") {
            return self.advance_focus(if modifiers.contains(Modifiers::SHIFT) {
                -1
            } else {
                1
            });
        }
        if modifiers.contains(Modifiers::CTRL) || modifiers.contains(Modifiers::META) {
            match key.to_ascii_lowercase().as_str() {
                "a" => return self.edit_focused(EditCommand::SelectAll),
                "z" if modifiers.contains(Modifiers::SHIFT) => {
                    return self.edit_focused(EditCommand::Redo)
                }
                "z" => return self.edit_focused(EditCommand::Undo),
                "y" => return self.edit_focused(EditCommand::Redo),
                "c" => return self.copy_selection(false),
                "x" => return self.copy_selection(true),
                "v" => return self.paste_clipboard(),
                _ => {}
            }
        }
        let extend = modifiers.contains(Modifiers::SHIFT);
        let adjustment = match (key_code, key.to_ascii_lowercase().as_str()) {
            (37 | 38, _) | (_, "arrowleft" | "arrowup") => Some(ControlAdjustment::Previous),
            (39 | 40, _) | (_, "arrowright" | "arrowdown") => Some(ControlAdjustment::Next),
            (33, _) | (_, "pageup") => Some(ControlAdjustment::PageBackward),
            (34, _) | (_, "pagedown") => Some(ControlAdjustment::PageForward),
            (36, _) | (_, "home") => Some(ControlAdjustment::Minimum),
            (35, _) | (_, "end") => Some(ControlAdjustment::Maximum),
            _ => None,
        };
        let adjustable = self.with_engine(|engine| {
            engine
                .control_state(target)
                .ok()
                .flatten()
                .is_some_and(|state| {
                    matches!(
                        state.role,
                        FormControlRole::Range | FormControlRole::Radio | FormControlRole::Select
                    )
                })
        })?;
        if adjustable {
            if let Some(adjustment) = adjustment {
                return self.adjust_focused(adjustment);
            }
        }
        let command = match (key_code, key.to_ascii_lowercase().as_str()) {
            (8, _) | (_, "backspace") => Some(EditCommand::Delete {
                direction: TextDirection::Backward,
                unit: if modifiers.contains(Modifiers::CTRL) {
                    TextUnit::Word
                } else {
                    TextUnit::Grapheme
                },
            }),
            (46, _) | (_, "delete") => Some(EditCommand::Delete {
                direction: TextDirection::Forward,
                unit: if modifiers.contains(Modifiers::CTRL) {
                    TextUnit::Word
                } else {
                    TextUnit::Grapheme
                },
            }),
            (37, _) | (_, "arrowleft") => Some(EditCommand::Move {
                direction: TextDirection::Backward,
                unit: if modifiers.contains(Modifiers::CTRL) {
                    TextUnit::Word
                } else {
                    TextUnit::Grapheme
                },
                extend,
            }),
            (39, _) | (_, "arrowright") => Some(EditCommand::Move {
                direction: TextDirection::Forward,
                unit: if modifiers.contains(Modifiers::CTRL) {
                    TextUnit::Word
                } else {
                    TextUnit::Grapheme
                },
                extend,
            }),
            (36, _) | (_, "home") => Some(EditCommand::Move {
                direction: TextDirection::Backward,
                unit: TextUnit::Line,
                extend,
            }),
            (35, _) | (_, "end") => Some(EditCommand::Move {
                direction: TextDirection::Forward,
                unit: TextUnit::Line,
                extend,
            }),
            _ => None,
        };
        if let Some(command) = command {
            return self.edit_focused(command);
        }
        if key_code == 13 || key_code == 32 || key == "Enter" || key == " " {
            let click = Event::keyboard("click", key_code, key_text, modifiers);
            self.dispatch_to(target, &click)?;
            if !click.default_prevented() {
                let changed = self.with_engine_mut(|engine| engine.activate(target))?;
                for changed in changed.changed {
                    self.dispatch_to(changed, &Event::keyboard("input", 0, None, modifiers))?;
                    self.dispatch_to(changed, &Event::keyboard("change", 0, None, modifiers))?;
                }
            }
        }
        Ok(())
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
            let event = Event::wheel(x, y, delta_x, delta_y, modifiers);
            self.dispatch_to(target, &event)?;
            if !event.default_prevented() {
                let mut current = Some(target);
                while let Some(node) = current {
                    let scrollable = self.with_engine(|engine| {
                        engine.computed_style(node).is_ok_and(|style| {
                            style.overflow_x.is_scrollable() || style.overflow_y.is_scrollable()
                        })
                    })?;
                    if scrollable {
                        self.with_engine_mut(|engine| {
                            let (left, top) = engine.scroll_offset(node)?;
                            engine.scroll_to(node, left + delta_x as f64, top + delta_y as f64)
                        })?;
                        break;
                    }
                    current = self.with_engine(|engine| engine.parent(node))??;
                }
            }
        }
        Ok(())
    }

    pub fn focused_element(&self) -> Result<Option<Element>, Error> {
        let handle = self.with_engine(|engine| engine.focused())?;
        Ok(handle.map(|handle| Element::from_handle(self.clone(), handle)))
    }

    pub fn set_modal_root(&self, root: Option<&Element>) -> Result<(), Error> {
        if let Some(root) = root {
            if !Rc::ptr_eq(&self.inner, &root.document.inner) {
                return Err(openui_engine::EngineError::WrongDocument.into());
            }
        }
        self.with_engine_mut(|engine| engine.set_modal_root(root.map(|root| root.handle)))
    }

    pub fn advance_focus(&self, direction: i32) -> Result<(), Error> {
        if direction != -1 && direction != 1 {
            return Err(Error::InvalidArgument("focus direction must be -1 or 1"));
        }
        let previous = self.with_engine(|engine| engine.focused())?;
        let next = self.with_engine_mut(|engine| engine.advance_focus(direction))?;
        self.dispatch_focus_change(previous, next)
    }

    pub fn dispatch_text_input(&self, text: &str) -> Result<(), Error> {
        let Some(target) = self.with_engine(|engine| engine.focused())? else {
            return Ok(());
        };
        let before = Event::keyboard("beforeinput", 0, Some(text), Modifiers::NONE);
        self.dispatch_to(target, &before)?;
        if !before.default_prevented() {
            self.with_engine_mut(|engine| engine.insert_text(target, text))?;
            self.dispatch_to(
                target,
                &Event::keyboard("input", 0, Some(text), Modifiers::NONE),
            )?;
        }
        Ok(())
    }

    pub fn dispatch_composition_start(&self) -> Result<(), Error> {
        if let Some(target) = self.with_engine(|engine| engine.focused())? {
            self.dispatch_to(target, &Event::composition("compositionstart", ""))?;
        }
        Ok(())
    }

    pub fn dispatch_composition_update(&self, text: &str) -> Result<(), Error> {
        let Some(target) = self.with_engine(|engine| engine.focused())? else {
            return Ok(());
        };
        let event = Event::composition("compositionupdate", text);
        self.dispatch_to(target, &event)?;
        if !event.default_prevented() {
            self.with_engine_mut(|engine| engine.update_composition(target, text))?;
        }
        Ok(())
    }

    pub fn dispatch_composition_end(&self, text: &str) -> Result<(), Error> {
        let Some(target) = self.with_engine(|engine| engine.focused())? else {
            return Ok(());
        };
        self.with_engine_mut(|engine| engine.finish_composition(target))?;
        self.dispatch_to(target, &Event::composition("compositionend", text))?;
        self.dispatch_to(
            target,
            &Event::keyboard("input", 0, Some(text), Modifiers::NONE),
        )
    }

    pub fn clipboard_text(&self) -> Result<String, Error> {
        Ok(self
            .inner
            .clipboard
            .try_borrow()
            .map_err(|_| Error::ReentrantMutation)?
            .clone())
    }

    pub fn set_clipboard_text(&self, text: impl Into<String>) -> Result<(), Error> {
        *self
            .inner
            .clipboard
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantMutation)? = text.into();
        Ok(())
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

    fn dispatch_to(&self, target: NodeHandle, event: &Event) -> Result<(), Error> {
        let route = self.with_engine(|engine| engine.event_route(target))??;
        for step in route.steps {
            let (phase, capture) = match step.phase {
                EngineEventPhase::Capture => (EventPhase::Capture, Some(true)),
                EngineEventPhase::Target => (EventPhase::Target, None),
                EngineEventPhase::Bubble => (EventPhase::Bubble, Some(false)),
            };
            event.set_phase(phase);
            self.invoke(step.node, event, capture)?;
            if event.propagation_stopped() {
                return Ok(());
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
            if event.immediate_propagation_stopped() {
                break;
            }
        }
        Ok(())
    }

    fn focus_from(&self, target: NodeHandle, origin: FocusOrigin) -> Result<(), Error> {
        let outcome = self.with_engine_mut(|engine| engine.focus_with_origin(target, origin));
        let Ok(previous) = outcome else {
            return Ok(());
        };
        self.dispatch_focus_change(previous, Some(target))
    }

    fn dispatch_focus_change(
        &self,
        previous: Option<NodeHandle>,
        next: Option<NodeHandle>,
    ) -> Result<(), Error> {
        if previous == next {
            return Ok(());
        }
        if let Some(previous) = previous {
            self.dispatch_to(previous, &Event::keyboard("blur", 0, None, Modifiers::NONE))?;
        }
        if let Some(next) = next {
            self.dispatch_to(next, &Event::keyboard("focus", 0, None, Modifiers::NONE))?;
        }
        Ok(())
    }

    fn edit_focused(&self, command: EditCommand) -> Result<(), Error> {
        let Some(target) = self.with_engine(|engine| engine.focused())? else {
            return Ok(());
        };
        let changed = self.with_engine(|engine| {
            engine
                .control_state(target)
                .map(|state| state.map(|state| state.value.clone()))
        })??;
        if self
            .with_engine_mut(|engine| engine.edit_text(target, command))
            .is_err()
        {
            return Ok(());
        }
        let after = self.with_engine(|engine| {
            engine
                .control_state(target)
                .map(|state| state.map(|state| state.value.clone()))
        })??;
        if changed != after {
            self.dispatch_to(target, &Event::keyboard("input", 0, None, Modifiers::NONE))?;
        }
        Ok(())
    }

    fn copy_selection(&self, cut: bool) -> Result<(), Error> {
        let Some(target) = self.with_engine(|engine| engine.focused())? else {
            return Ok(());
        };
        let selection = self.with_engine(|engine| {
            engine.control_state(target).map(|state| {
                state.map(|state| {
                    let (start, end) = state.selection();
                    state.value[start..end].to_owned()
                })
            })
        })??;
        let Some(selection) = selection else {
            return Ok(());
        };
        self.set_clipboard_text(selection)?;
        if cut {
            self.edit_focused(EditCommand::Delete {
                direction: TextDirection::Backward,
                unit: TextUnit::Grapheme,
            })?;
        }
        Ok(())
    }

    fn paste_clipboard(&self) -> Result<(), Error> {
        let text = self.clipboard_text()?;
        self.dispatch_text_input(&text)
    }

    fn adjust_focused(&self, adjustment: ControlAdjustment) -> Result<(), Error> {
        let Some(target) = self.with_engine(|engine| engine.focused())? else {
            return Ok(());
        };
        let result = self.with_engine_mut(|engine| engine.adjust_control(target, adjustment))?;
        for changed in result.changed {
            self.dispatch_to(changed, &Event::keyboard("input", 0, None, Modifiers::NONE))?;
            self.dispatch_to(
                changed,
                &Event::keyboard("change", 0, None, Modifiers::NONE),
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Element, StyleProperty};
    use openui_style::{Display, LengthValue};
    use std::cell::Cell;
    use std::rc::Rc;

    fn mounted(document: &Document, tag: &str) -> Element {
        let element = Element::create(document, tag).unwrap();
        document.body().append_child(&element).unwrap();
        element
            .set_property(StyleProperty::Display, Display::Block.into())
            .unwrap();
        element
            .set_property(StyleProperty::Width, LengthValue::px(40.0).into())
            .unwrap();
        element
            .set_property(StyleProperty::Height, LengthValue::px(40.0).into())
            .unwrap();
        element
    }

    #[test]
    fn headless_text_input_handles_graphemes_clipboard_undo_and_composition() {
        let document = Document::new(200, 100).unwrap();
        let input = mounted(&document, "input");
        input.focus().unwrap();
        document.dispatch_text_input("a👩‍💻").unwrap();
        assert_eq!(input.control_value().unwrap().as_deref(), Some("a👩‍💻"));

        document
            .dispatch_key_event(KeyEventType::Down, 8, Some("Backspace"), Modifiers::NONE)
            .unwrap();
        assert_eq!(input.control_value().unwrap().as_deref(), Some("a"));
        document
            .dispatch_key_event(KeyEventType::Down, 90, Some("z"), Modifiers::CTRL)
            .unwrap();
        assert_eq!(input.control_value().unwrap().as_deref(), Some("a👩‍💻"));

        input.set_selection(0, 1).unwrap();
        document
            .dispatch_key_event(KeyEventType::Down, 67, Some("c"), Modifiers::CTRL)
            .unwrap();
        assert_eq!(document.clipboard_text().unwrap(), "a");
        let end = input.control_value().unwrap().unwrap().len();
        input.set_selection(end, end).unwrap();
        document
            .dispatch_key_event(KeyEventType::Down, 86, Some("v"), Modifiers::CTRL)
            .unwrap();
        assert_eq!(input.control_value().unwrap().as_deref(), Some("a👩‍💻a"));

        document.dispatch_composition_start().unwrap();
        document.dispatch_composition_update("é").unwrap();
        document.dispatch_composition_update("東").unwrap();
        document.dispatch_composition_end("東").unwrap();
        assert_eq!(input.control_value().unwrap().as_deref(), Some("a👩‍💻a東"));
    }

    #[test]
    fn pointer_and_keyboard_defaults_drive_controls_after_dispatch() {
        let document = Document::new(200, 100).unwrap();
        let checkbox = mounted(&document, "input");
        checkbox.set_attribute("type", "checkbox").unwrap();
        let changes = Rc::new(Cell::new(0));
        let observed = changes.clone();
        checkbox
            .on("change", move |_| observed.set(observed.get() + 1))
            .unwrap();
        document
            .dispatch_pointer_event(
                4,
                PointerEventKind::Down,
                10.0,
                10.0,
                MouseButton::Left,
                Modifiers::NONE,
                true,
            )
            .unwrap();
        document
            .dispatch_pointer_event(
                4,
                PointerEventKind::Up,
                10.0,
                10.0,
                MouseButton::Left,
                Modifiers::NONE,
                true,
            )
            .unwrap();
        assert!(checkbox.is_checked().unwrap());
        assert_eq!(changes.get(), 1);

        let range = mounted(&document, "input");
        range.set_attribute("type", "range").unwrap();
        range.set_attribute("min", "0").unwrap();
        range.set_attribute("max", "10").unwrap();
        range.set_attribute("step", "2").unwrap();
        range.set_control_value("4").unwrap();
        range.focus().unwrap();
        document
            .dispatch_key_event(KeyEventType::Down, 39, Some("ArrowRight"), Modifiers::NONE)
            .unwrap();
        assert_eq!(range.control_value().unwrap().as_deref(), Some("6"));
        document
            .dispatch_pointer_event(
                5,
                PointerEventKind::Down,
                38.0,
                60.0,
                MouseButton::Left,
                Modifiers::NONE,
                true,
            )
            .unwrap();
        document
            .dispatch_pointer_event(
                5,
                PointerEventKind::Up,
                38.0,
                60.0,
                MouseButton::Left,
                Modifiers::NONE,
                true,
            )
            .unwrap();
        assert_eq!(range.control_value().unwrap().as_deref(), Some("10"));
    }

    #[test]
    fn tab_order_skips_disabled_controls_and_modal_focus_restores() {
        let document = Document::new(200, 200).unwrap();
        let first = mounted(&document, "button");
        let disabled = mounted(&document, "button");
        disabled.set_attribute("disabled", "").unwrap();
        let last = mounted(&document, "button");
        document.advance_focus(1).unwrap();
        assert!(first.has_focus().unwrap());
        document.advance_focus(1).unwrap();
        assert!(last.has_focus().unwrap());

        let modal = mounted(&document, "div");
        let modal_button = Element::create(&document, "button").unwrap();
        modal.append_child(&modal_button).unwrap();
        document.set_modal_root(Some(&modal)).unwrap();
        assert!(modal_button.has_focus().unwrap());
        document.set_modal_root(None).unwrap();
        assert!(last.has_focus().unwrap());
    }

    #[test]
    fn accessibility_tree_and_actions_use_the_same_control_state() {
        let document = Document::new(200, 100).unwrap();
        let checkbox = mounted(&document, "input");
        checkbox.set_attribute("type", "checkbox").unwrap();
        checkbox.set_accessibility_label("Ship").unwrap();
        let clicks = Rc::new(Cell::new(0));
        let observed_clicks = clicks.clone();
        checkbox
            .on("click", move |_| {
                observed_clicks.set(observed_clicks.get() + 1)
            })
            .unwrap();
        let initial = document.accessibility_update().unwrap();
        let checkbox_id = document
            .with_engine(|engine| engine.accessibility_node_id(checkbox.handle))
            .unwrap()
            .unwrap();
        let node = &initial
            .nodes
            .iter()
            .find(|(id, _)| *id == checkbox_id)
            .unwrap()
            .1;
        assert_eq!(node.role(), openui_engine::AccessibilityRole::CheckBox);
        assert_eq!(node.label(), Some("Ship"));
        checkbox
            .perform_accessibility_action(openui_engine::AccessibilityAction::Click)
            .unwrap();
        assert_eq!(clicks.get(), 1);
        assert!(checkbox.is_checked().unwrap());
        assert_eq!(document.accessibility_update().unwrap().nodes.len(), 1);
    }
}
