//! Normalized framework input events.

use crate::element::{Element, WeakElement};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The native editing operation reported by beforeinput and input callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InputType {
    InsertText,
    InsertLineBreak,
    DeleteContentBackward,
    DeleteContentForward,
    HistoryUndo,
    HistoryRedo,
    DeleteWordBackward,
    DeleteWordForward,
    DeleteByCut,
    InsertFromPaste,
}

impl InputType {
    /// Name used by the pinned Chromium reference for the equivalent operation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InsertText => "insertText",
            Self::InsertLineBreak => "insertLineBreak",
            Self::DeleteContentBackward => "deleteContentBackward",
            Self::DeleteContentForward => "deleteContentForward",
            Self::HistoryUndo => "historyUndo",
            Self::HistoryRedo => "historyRedo",
            Self::DeleteWordBackward => "deleteWordBackward",
            Self::DeleteWordForward => "deleteWordForward",
            Self::DeleteByCut => "deleteByCut",
            Self::InsertFromPaste => "insertFromPaste",
        }
    }
}

/// Immutable, owned metadata for a native editing callback.
/// Clones retain the original intent and nullable data after dispatch returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputEventInfo {
    input_type: InputType,
    data: Option<String>,
    is_composing: bool,
}

impl InputEventInfo {
    pub fn input_type(&self) -> InputType {
        self.input_type
    }

    /// Inserted text, or None for deletion, history and line-break operations.
    pub fn data(&self) -> Option<&str> {
        self.data.as_deref()
    }

    pub fn is_composing(&self) -> bool {
        self.is_composing
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPhase {
    Capture,
    Target,
    Bubble,
}

#[derive(Debug, Default)]
struct EventState {
    bubbles: bool,
    cancelable: bool,
    input_info: Option<InputEventInfo>,
    timestamp_ns: u64,
    default_prevented: Cell<bool>,
    propagation_stopped: Cell<bool>,
    immediate_propagation_stopped: Cell<bool>,
    phase: Cell<Option<EventPhase>>,
    target: RefCell<Option<WeakElement>>,
    current_target: RefCell<Option<WeakElement>>,
    related_target: RefCell<Option<WeakElement>>,
}

impl EventState {
    fn for_type(event_type: &str) -> Rc<Self> {
        Rc::new(Self {
            bubbles: !matches!(
                event_type,
                "focus" | "blur" | "mouseenter" | "mouseleave" | "pointerenter" | "pointerleave"
            ),
            // Native input/change notifications report a completed mutation.
            // Chromium creates them as noncancelable; beforeinput remains
            // the application callback that may reject the pending edit.
            cancelable: !matches!(
                event_type,
                "focus"
                    | "blur"
                    | "focusin"
                    | "focusout"
                    | "input"
                    | "change"
                    | "select"
                    | "selectionchange"
            ),
            ..Self::default()
        })
    }
}

pub(crate) struct ListenerScope<'a> {
    event: &'a Event,
}

impl Drop for ListenerScope<'_> {
    fn drop(&mut self) {
        *self.event.state.current_target.borrow_mut() = None;
        self.event.state.phase.set(None);
    }
}

/// Event value shared by capture, target, and bubble listeners.
#[derive(Debug, Clone)]
pub struct Event {
    pub event_type: String,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub delta_x: f32,
    pub delta_y: f32,
    pub mouse_button: i32,
    pub key_code: i32,
    pub key_text: String,
    pub modifiers: i32,
    pub pointer_id: u64,
    pub is_composing: bool,
    state: Rc<EventState>,
}

impl Event {
    pub(crate) fn pointer(
        event_type: impl Into<String>,
        pointer_id: u64,
        x: f32,
        y: f32,
        button: MouseButton,
        modifiers: Modifiers,
    ) -> Self {
        let event_type = event_type.into();
        let state = EventState::for_type(&event_type);
        Self {
            event_type,
            mouse_x: x,
            mouse_y: y,
            delta_x: 0.0,
            delta_y: 0.0,
            mouse_button: button as i32,
            key_code: 0,
            key_text: String::new(),
            modifiers: modifiers.bits() as i32,
            pointer_id,
            is_composing: false,
            state,
        }
    }

    pub(crate) fn keyboard(
        event_type: impl Into<String>,
        key_code: i32,
        key_text: Option<&str>,
        modifiers: Modifiers,
    ) -> Self {
        let event_type = event_type.into();
        let state = EventState::for_type(&event_type);
        Self {
            event_type,
            mouse_x: 0.0,
            mouse_y: 0.0,
            delta_x: 0.0,
            delta_y: 0.0,
            mouse_button: -1,
            key_code,
            key_text: key_text.unwrap_or_default().to_owned(),
            modifiers: modifiers.bits() as i32,
            pointer_id: 0,
            is_composing: false,
            state,
        }
    }

    #[cfg(feature = "ffi-integration")]
    #[doc(hidden)]
    pub fn click_for_native_facade(timestamp_ns: u64) -> Self {
        let mut event = Self::pointer("click", 0, 0.0, 0.0, MouseButton::Left, Modifiers::NONE);
        Rc::get_mut(&mut event.state)
            .expect("new event state")
            .timestamp_ns = timestamp_ns;
        event
    }

    /// Timestamp supplied by the native event source, or zero when unavailable.
    pub fn timestamp_ns(&self) -> u64 {
        self.state.timestamp_ns
    }

    pub(crate) fn input(
        event_type: &str,
        key_code: i32,
        key_text: Option<&str>,
        modifiers: Modifiers,
        input_type: InputType,
        data: Option<&str>,
        is_composing: bool,
    ) -> Self {
        let mut event = Self::keyboard(event_type, key_code, key_text, modifiers);
        event.is_composing = is_composing;
        Rc::get_mut(&mut event.state)
            .expect("new event state is uniquely owned")
            .input_info = Some(InputEventInfo {
            input_type,
            data: data.map(str::to_owned),
            is_composing,
        });
        event
    }

    /// Immutable native editing intent and nullable data.
    /// Returns None for events without a typed editing operation. The owned
    /// metadata remains unchanged in saved clones after dispatch or teardown.
    pub fn input_info(&self) -> Option<&InputEventInfo> {
        self.state.input_info.as_ref()
    }

    pub(crate) fn wheel(x: f32, y: f32, dx: f32, dy: f32, modifiers: Modifiers) -> Self {
        let mut event = Self::pointer("wheel", 0, x, y, MouseButton::Middle, modifiers);
        event.delta_x = dx;
        event.delta_y = dy;
        event
    }

    pub(crate) fn focus(event_type: &str, related_target: Option<WeakElement>) -> Self {
        let event = Self::keyboard(event_type, 0, None, Modifiers::NONE);
        *event.state.related_target.borrow_mut() = related_target;
        event
    }

    pub(crate) fn set_phase(&self, phase: EventPhase) {
        self.state.phase.set(Some(phase));
    }

    pub(crate) fn listener_scope(
        &self,
        target: WeakElement,
        current_target: WeakElement,
    ) -> ListenerScope<'_> {
        *self.state.target.borrow_mut() = Some(target);
        *self.state.current_target.borrow_mut() = Some(current_target);
        ListenerScope { event: self }
    }

    /// The current dispatch phase, or `None` after callbacks have returned.
    pub fn phase(&self) -> Option<EventPhase> {
        self.state.phase.get()
    }

    /// The element that received the event, including a pointer-capture target.
    ///
    /// The event retains a weak, generation-checked handle. A saved event can
    /// still resolve its target while that element and document are live, but
    /// does not keep either alive. Returns `None` after they are destroyed.
    pub fn target(&self) -> Option<Element> {
        self.state.target.borrow().as_ref()?.upgrade()
    }

    /// The element whose listener is currently running.
    ///
    /// This follows the shared capture/target/bubble route and is cleared
    /// when callbacks return, including early returns and panic unwinding.
    /// Saved clones observe the same current listener while dispatch runs.
    pub fn current_target(&self) -> Option<Element> {
        self.state.current_target.borrow().as_ref()?.upgrade()
    }

    /// The other element in a native focus transfer.
    ///
    /// For `blur` and `focusout`, this is the pending focus destination; for
    /// `focus` and `focusin`, it is the previous focused element. Initial
    /// focus, clearing focus, and a transfer redirected by a blur callback
    /// can have no related element. Other event types return `None`.
    ///
    /// Like [`Self::target`], this uses a weak, generation-checked handle.
    /// Saved events do not retain the element or document and return `None`
    /// after either is destroyed.
    pub fn related_target(&self) -> Option<Element> {
        self.state.related_target.borrow().as_ref()?.upgrade()
    }

    #[cfg(feature = "ffi-integration")]
    #[doc(hidden)]
    pub fn related_node_for_native_facade(&self) -> Option<openui_engine::NodeHandle> {
        self.related_target().map(|element| element.handle)
    }

    /// Whether this normalized event visits ancestor bubble listeners.
    pub fn bubbles(&self) -> bool {
        self.state.bubbles
    }

    /// Whether [`Self::prevent_default`] can cancel this event's default action.
    /// Native focus, blur, focusin and focusout notifications are noncancelable.
    pub fn cancelable(&self) -> bool {
        self.state.cancelable
    }

    pub fn prevent_default(&self) {
        if self.cancelable() {
            self.state.default_prevented.set(true);
        }
    }

    pub fn default_prevented(&self) -> bool {
        self.state.default_prevented.get()
    }

    pub fn stop_propagation(&self) {
        self.state.propagation_stopped.set(true);
    }

    pub fn stop_immediate_propagation(&self) {
        self.state.propagation_stopped.set(true);
        self.state.immediate_propagation_stopped.set(true);
    }

    pub fn propagation_stopped(&self) -> bool {
        self.state.propagation_stopped.get()
    }

    pub fn immediate_propagation_stopped(&self) -> bool {
        self.state.immediate_propagation_stopped.get()
    }

    pub(crate) fn composition(event_type: &str, text: &str) -> Self {
        let mut event = Self::keyboard(event_type, 0, Some(text), Modifiers::NONE);
        event.is_composing = event_type != "compositionend";
        event
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventType {
    Down,
    Up,
    Move,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum MouseButton {
    Left = 0,
    Middle = 1,
    Right = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventType {
    Down,
    Up,
    Char,
}

impl KeyEventType {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Down => "keydown",
            Self::Up => "keyup",
            Self::Char => "input",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modifiers(pub u32);

impl Modifiers {
    pub const NONE: Self = Self(0);
    pub const SHIFT: Self = Self(1);
    pub const CTRL: Self = Self(2);
    pub const ALT: Self = Self(4);
    pub const META: Self = Self(8);

    pub fn bits(self) -> u32 {
        self.0
    }

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl std::ops::BitOr for Modifiers {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for Modifiers {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

pub(crate) type EventCallback = Rc<dyn Fn(&Event)>;

#[derive(Clone)]
pub(crate) struct Listener {
    pub capture: bool,
    pub callback: EventCallback,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_control_state_is_shared_by_clones() {
        let event = Event::pointer("click", 0, 1.0, 2.0, MouseButton::Left, Modifiers::CTRL);
        let clone = event.clone();
        clone.prevent_default();
        assert!(event.default_prevented());
        assert_eq!(event.modifiers, Modifiers::CTRL.bits() as i32);
    }
}
