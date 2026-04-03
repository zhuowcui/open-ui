// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui_events.cc — SP7 event dispatch (mouse, keyboard, wheel) and
// event callback registration.

#include "openui/openui_events.h"

#include "openui/openui.h"
#include "openui/openui_impl.h"

#include "third_party/blink/public/common/input/web_input_event.h"
#include "third_party/blink/public/common/input/web_keyboard_event.h"
#include "third_party/blink/public/common/input/web_mouse_event.h"
#include "third_party/blink/public/common/input/web_mouse_wheel_event.h"
#include "third_party/blink/renderer/core/dom/events/event.h"
#include "third_party/blink/renderer/core/events/keyboard_event.h"
#include "third_party/blink/renderer/core/events/mouse_event.h"
#include "third_party/blink/renderer/core/frame/local_frame.h"
#include "third_party/blink/renderer/core/input/event_handler.h"
#include "third_party/blink/renderer/platform/wtf/text/atomic_string.h"
#include "third_party/blink/public/platform/web_string.h"

// ═══════════════════════════════════════════════════════════════════════════
// OuiNativeEventListener implementation
// ═══════════════════════════════════════════════════════════════════════════

OuiNativeEventListener::OuiNativeEventListener(OuiEventCallback callback,
                                                void* user_data,
                                                OuiElementImpl* owner)
    : callback_(callback), user_data_(user_data), owner_(owner) {}

void OuiNativeEventListener::Invoke(blink::ExecutionContext*,
                                     blink::Event* event) {
  if (!callback_ || !owner_)
    return;

  OuiEvent oui_event = {};
  std::string type_str = event->type().Utf8();
  oui_event.type = type_str.c_str();
  oui_event.target = reinterpret_cast<OuiElement*>(owner_);

  // Extract mouse event details.
  if (auto* mouse = blink::DynamicTo<blink::MouseEvent>(event)) {
    oui_event.mouse_x = static_cast<float>(mouse->clientX());
    oui_event.mouse_y = static_cast<float>(mouse->clientY());
    oui_event.mouse_button = static_cast<int>(mouse->button());
  }

  // Extract keyboard event details.
  if (auto* key = blink::DynamicTo<blink::KeyboardEvent>(event)) {
    oui_event.key_code = key->keyCode();
    // key_text is transient — store in a local.
    std::string key_text = key->key().Utf8();
    oui_event.key_text = key_text.c_str();
  }

  // Modifiers.
  if (auto* mouse = blink::DynamicTo<blink::MouseEvent>(event)) {
    if (mouse->shiftKey()) oui_event.modifiers |= OUI_MOD_SHIFT;
    if (mouse->ctrlKey()) oui_event.modifiers |= OUI_MOD_CTRL;
    if (mouse->altKey()) oui_event.modifiers |= OUI_MOD_ALT;
    if (mouse->metaKey()) oui_event.modifiers |= OUI_MOD_META;
  } else if (auto* key = blink::DynamicTo<blink::KeyboardEvent>(event)) {
    if (key->shiftKey()) oui_event.modifiers |= OUI_MOD_SHIFT;
    if (key->ctrlKey()) oui_event.modifiers |= OUI_MOD_CTRL;
    if (key->altKey()) oui_event.modifiers |= OUI_MOD_ALT;
    if (key->metaKey()) oui_event.modifiers |= OUI_MOD_META;
  }

  oui_event.default_prevented = 0;
  callback_(&oui_event, user_data_);

  if (oui_event.default_prevented)
    event->preventDefault();
}

void OuiNativeEventListener::Trace(blink::Visitor* visitor) const {
  blink::NativeEventListener::Trace(visitor);
}

// ═══════════════════════════════════════════════════════════════════════════
// Helper: map OUI modifier flags → WebInputEvent::Modifiers
// ═══════════════════════════════════════════════════════════════════════════

namespace {

int MapModifiers(int oui_mods) {
  int mods = 0;
  if (oui_mods & OUI_MOD_SHIFT)
    mods |= blink::WebInputEvent::kShiftKey;
  if (oui_mods & OUI_MOD_CTRL)
    mods |= blink::WebInputEvent::kControlKey;
  if (oui_mods & OUI_MOD_ALT)
    mods |= blink::WebInputEvent::kAltKey;
  if (oui_mods & OUI_MOD_META)
    mods |= blink::WebInputEvent::kMetaKey;
  return mods;
}

}  // namespace

// ═══════════════════════════════════════════════════════════════════════════
// Input event dispatch
// ═══════════════════════════════════════════════════════════════════════════

OuiStatus oui_document_dispatch_mouse_event(OuiDocument* doc,
                                             OuiMouseEventType type,
                                             float x,
                                             float y,
                                             OuiMouseButton button,
                                             int modifiers) {
  if (!doc) return OUI_ERROR_INVALID_ARGUMENT;
  auto* impl = reinterpret_cast<OuiDocumentImpl*>(doc);
  auto& frame = impl->page_holder->GetFrame();
  auto& handler = frame.GetEventHandler();

  gfx::PointF pos(x, y);
  int web_mods = MapModifiers(modifiers);

  blink::WebPointerProperties::Button web_button;
  switch (button) {
    case OUI_BUTTON_LEFT:
      web_button = blink::WebPointerProperties::Button::kLeft;
      break;
    case OUI_BUTTON_MIDDLE:
      web_button = blink::WebPointerProperties::Button::kMiddle;
      break;
    case OUI_BUTTON_RIGHT:
      web_button = blink::WebPointerProperties::Button::kRight;
      break;
    default:
      web_button = blink::WebPointerProperties::Button::kLeft;
      break;
  }

  base::TimeTicks ts = base::TimeTicks::Now();

  switch (type) {
    case OUI_MOUSE_DOWN: {
      blink::WebMouseEvent event(
          blink::WebInputEvent::Type::kMouseDown, web_mods, ts);
      event.SetPositionInWidget(pos);
      event.button = web_button;
      event.click_count = 1;
      handler.HandleMousePressEvent(event);
      break;
    }
    case OUI_MOUSE_UP: {
      blink::WebMouseEvent event(
          blink::WebInputEvent::Type::kMouseUp, web_mods, ts);
      event.SetPositionInWidget(pos);
      event.button = web_button;
      event.click_count = 1;
      handler.HandleMouseReleaseEvent(event);
      break;
    }
    case OUI_MOUSE_MOVE: {
      blink::WebMouseEvent event(
          blink::WebInputEvent::Type::kMouseMove, web_mods, ts);
      event.SetPositionInWidget(pos);
      event.button = blink::WebPointerProperties::Button::kNoButton;
      handler.HandleMouseMoveEvent(event, {}, {});
      break;
    }
    default:
      return OUI_ERROR_INVALID_ARGUMENT;
  }
  return OUI_OK;
}

OuiStatus oui_document_dispatch_key_event(OuiDocument* doc,
                                           OuiKeyEventType type,
                                           int key_code,
                                           const char* key_text,
                                           int modifiers) {
  if (!doc) return OUI_ERROR_INVALID_ARGUMENT;
  auto* impl = reinterpret_cast<OuiDocumentImpl*>(doc);
  auto& frame = impl->page_holder->GetFrame();
  auto& handler = frame.GetEventHandler();

  int web_mods = MapModifiers(modifiers);
  base::TimeTicks ts = base::TimeTicks::Now();

  switch (type) {
    case OUI_KEY_DOWN: {
      blink::WebKeyboardEvent event(
          blink::WebInputEvent::Type::kRawKeyDown, web_mods, ts);
      event.windows_key_code = key_code;
      handler.KeyEvent(event);
      break;
    }
    case OUI_KEY_UP: {
      blink::WebKeyboardEvent event(
          blink::WebInputEvent::Type::kKeyUp, web_mods, ts);
      event.windows_key_code = key_code;
      handler.KeyEvent(event);
      break;
    }
    case OUI_KEY_CHAR: {
      blink::WebKeyboardEvent event(
          blink::WebInputEvent::Type::kChar, web_mods, ts);
      event.windows_key_code = key_code;
      if (key_text && key_text[0]) {
        // Decode UTF-8 → UTF-16 for Blink's text field.
        blink::WebString ws = blink::WebString::FromUTF8(key_text);
        std::u16string u16 = ws.Utf16();
        size_t copy_len = std::min(u16.size(),
            static_cast<size_t>(blink::WebKeyboardEvent::kTextLengthCap - 1));
        for (size_t i = 0; i < copy_len; ++i)
          event.text[i] = u16[i];
        event.text[copy_len] = 0;
        // Also populate unmodified_text for consistency.
        for (size_t i = 0; i < copy_len; ++i)
          event.unmodified_text[i] = u16[i];
        event.unmodified_text[copy_len] = 0;
      }
      handler.KeyEvent(event);
      break;
    }
    default:
      return OUI_ERROR_INVALID_ARGUMENT;
  }
  return OUI_OK;
}

OuiStatus oui_document_dispatch_wheel_event(OuiDocument* doc,
                                             float x,
                                             float y,
                                             float delta_x,
                                             float delta_y,
                                             int modifiers) {
  if (!doc) return OUI_ERROR_INVALID_ARGUMENT;
  auto* impl = reinterpret_cast<OuiDocumentImpl*>(doc);
  auto& frame = impl->page_holder->GetFrame();
  auto& handler = frame.GetEventHandler();

  int web_mods = MapModifiers(modifiers);
  base::TimeTicks ts = base::TimeTicks::Now();

  blink::WebMouseWheelEvent event(
      blink::WebInputEvent::Type::kMouseWheel, web_mods, ts);
  event.SetPositionInWidget(gfx::PointF(x, y));
  event.delta_x = delta_x;
  event.delta_y = delta_y;
  // Use kPhaseNone to indicate discrete (non-trackpad) wheel ticks.
  // This avoids breaking Blink's scroll snap/momentum tracking which
  // expects proper phase sequences for trackpad gestures.
  event.phase = blink::WebMouseWheelEvent::kPhaseNone;
  handler.HandleWheelEvent(event);

  return OUI_OK;
}

// ═══════════════════════════════════════════════════════════════════════════
// Event callbacks
// ═══════════════════════════════════════════════════════════════════════════

OuiStatus oui_element_set_event_callback(OuiElement* elem,
                                          const char* event_type,
                                          OuiEventCallback callback,
                                          void* user_data) {
  if (!elem || !event_type || !callback)
    return OUI_ERROR_INVALID_ARGUMENT;

  auto* impl = reinterpret_cast<OuiElementImpl*>(elem);
  if (!impl->element)
    return OUI_ERROR_INVALID_ARGUMENT;

  std::string type(event_type);

  // Remove existing listener for this event type if present.
  auto it = impl->callbacks.find(type);
  if (it != impl->callbacks.end()) {
    impl->element->removeEventListener(
        blink::AtomicString(event_type), it->second.listener.Get(), false);
    impl->callbacks.erase(it);
  }

  // Create new GC'd listener.
  auto* listener =
      blink::MakeGarbageCollected<OuiNativeEventListener>(
          callback, user_data, impl);

  impl->element->addEventListener(
      blink::AtomicString(event_type), listener);

  OuiCallbackEntry entry;
  entry.callback = reinterpret_cast<void*>(callback);
  entry.user_data = user_data;
  entry.listener = listener;
  impl->callbacks[type] = std::move(entry);

  return OUI_OK;
}

OuiStatus oui_element_remove_event_callback(OuiElement* elem,
                                             const char* event_type) {
  if (!elem || !event_type)
    return OUI_ERROR_INVALID_ARGUMENT;

  auto* impl = reinterpret_cast<OuiElementImpl*>(elem);
  if (!impl->element)
    return OUI_ERROR_INVALID_ARGUMENT;

  std::string type(event_type);
  auto it = impl->callbacks.find(type);
  if (it != impl->callbacks.end()) {
    impl->element->removeEventListener(
        blink::AtomicString(event_type), it->second.listener.Get(), false);
    impl->callbacks.erase(it);
  }

  return OUI_OK;
}
