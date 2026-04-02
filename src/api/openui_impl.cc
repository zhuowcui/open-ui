// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui_impl.cc — C API implementation wrapping blink's rendering pipeline.

#include "openui/openui.h"
#include "openui/openui_element_factory.h"
#include "openui/openui_impl.h"
#include "openui/openui_init.h"

#include <stdlib.h>
#include <string.h>
#include <string>
#include <vector>

#include "third_party/blink/renderer/core/css/css_primitive_value.h"
#include "third_party/blink/renderer/core/css/css_property_names.h"
#include "third_party/blink/renderer/core/css/css_value_id_mappings.h"
#include "third_party/blink/renderer/core/css/properties/css_property.h"
#include "third_party/blink/renderer/core/css/resolver/style_resolver.h"
#include "third_party/blink/renderer/core/dom/document.h"
#include "third_party/blink/renderer/core/dom/element.h"
#include "third_party/blink/renderer/core/dom/text.h"
#include "third_party/blink/renderer/core/frame/local_frame_view.h"
#include "third_party/blink/renderer/core/html/html_body_element.h"
#include "third_party/blink/renderer/core/layout/hit_test_location.h"
#include "third_party/blink/renderer/core/layout/hit_test_request.h"
#include "third_party/blink/renderer/core/layout/hit_test_result.h"
#include "third_party/blink/renderer/core/layout/layout_box.h"
#include "third_party/blink/renderer/core/layout/layout_object.h"
#include "third_party/blink/renderer/core/layout/layout_view.h"
#include "third_party/blink/renderer/core/style/computed_style.h"
#include "third_party/blink/renderer/platform/wtf/text/atomic_string.h"
#include "third_party/blink/renderer/platform/wtf/text/wtf_string.h"
#include "ui/gfx/geometry/point_f.h"
#include "ui/gfx/geometry/size.h"

// Out-of-line constructors/destructors for complex structs (chromium-style).
OuiDocumentImpl::OuiDocumentImpl() = default;
OuiDocumentImpl::~OuiDocumentImpl() = default;
OuiElementImpl::OuiElementImpl() = default;
OuiElementImpl::~OuiElementImpl() = default;

namespace {

// --------------------------------------------------------------------------
// Helper: OuiLength → blink SetInlineStyleProperty with typed unit.
// --------------------------------------------------------------------------
void SetLengthProperty(blink::Element* elem,
                       blink::CSSPropertyID prop,
                       OuiLength len) {
  if (!elem) {
    return;
  }
  if (len.unit == OUI_UNIT_AUTO) {
    elem->SetInlineStyleProperty(prop, blink::CSSValueID::kAuto);
    return;
  }
  if (len.unit == OUI_UNIT_NONE) {
    elem->SetInlineStyleProperty(prop, blink::CSSValueID::kNone);
    return;
  }

  blink::CSSPrimitiveValue::UnitType unit;
  switch (len.unit) {
    case OUI_UNIT_PX:
      unit = blink::CSSPrimitiveValue::UnitType::kPixels;
      break;
    case OUI_UNIT_PERCENT:
      unit = blink::CSSPrimitiveValue::UnitType::kPercentage;
      break;
    case OUI_UNIT_EM:
      unit = blink::CSSPrimitiveValue::UnitType::kEms;
      break;
    case OUI_UNIT_REM:
      unit = blink::CSSPrimitiveValue::UnitType::kRems;
      break;
    case OUI_UNIT_VW:
      unit = blink::CSSPrimitiveValue::UnitType::kViewportWidth;
      break;
    case OUI_UNIT_VH:
      unit = blink::CSSPrimitiveValue::UnitType::kViewportHeight;
      break;
    case OUI_UNIT_FR:
      unit = blink::CSSPrimitiveValue::UnitType::kFlex;
      break;
    default:
      unit = blink::CSSPrimitiveValue::UnitType::kPixels;
      break;
  }
  elem->SetInlineStyleProperty(prop, static_cast<double>(len.value), unit);
}

// --------------------------------------------------------------------------
// Helper: RGBA uint32_t → CSS color string "rgba(r, g, b, a)".
// --------------------------------------------------------------------------
std::string RGBAToCSSString(uint32_t rgba) {
  int r = (rgba >> 24) & 0xFF;
  int g = (rgba >> 16) & 0xFF;
  int b = (rgba >> 8) & 0xFF;
  float a = static_cast<float>(rgba & 0xFF) / 255.0f;
  char buf[64];
  snprintf(buf, sizeof(buf), "rgba(%d, %d, %d, %.4g)", r, g, b, a);
  return buf;
}

// --------------------------------------------------------------------------
// Helper: CSSPropertyID from property name string.
// Uses blink's generated lookup instead of passing through ExecutionContext.
// --------------------------------------------------------------------------
blink::CSSPropertyID LookupPropertyID(const char* property) {
  blink::String prop_str = blink::String(property);
  // CssPropertyID requires an ExecutionContext for custom properties.
  // For standard properties, passing nullptr works.
  blink::CSSPropertyID id = blink::CssPropertyID(nullptr, prop_str);
  if (id == blink::CSSPropertyID::kInvalid) {
    return blink::CSSPropertyID::kInvalid;
  }
  return id;
}

// --------------------------------------------------------------------------
// Element tracker: maintains a map of raw addresses → OuiElementImpl*
// so we can reverse-lookup for hit testing and DOM traversal.
// Uses void* to avoid blink GC plugin restrictions on Element* in STL.
// --------------------------------------------------------------------------
using ElementMap = std::unordered_map<void*, OuiElementImpl*>;

ElementMap& GetElementMap() {
  static ElementMap* map = new ElementMap();
  return *map;
}

OuiElementImpl* FindWrapper(blink::Element* elem) {
  auto& map = GetElementMap();
  auto it = map.find(static_cast<void*>(elem));
  return it != map.end() ? it->second : nullptr;
}

void RegisterWrapper(blink::Element* elem, OuiElementImpl* wrapper) {
  GetElementMap()[static_cast<void*>(elem)] = wrapper;
}

void UnregisterWrapper(blink::Element* elem) {
  GetElementMap().erase(static_cast<void*>(elem));
}

}  // namespace

// ═══════════════════════════════════════════════════════════════════════════
// Initialization
// ═══════════════════════════════════════════════════════════════════════════

OuiStatus oui_init(const OuiInitConfig* config) {
  return openui_runtime_init(config);
}

void oui_shutdown() {
  openui_runtime_shutdown();
}

// ═══════════════════════════════════════════════════════════════════════════
// Document
// ═══════════════════════════════════════════════════════════════════════════

OuiDocument* oui_document_create(int viewport_width, int viewport_height) {
  if (!openui_runtime_is_initialized()) {
    return nullptr;
  }

  auto* impl = new OuiDocumentImpl();
  // Only create per-document TaskEnvironment when running standalone.
  // Under a test harness, the test runner provides the task environment.
  if (!openui_runtime_has_external_task_env()) {
    impl->task_env = std::make_unique<blink::test::TaskEnvironment>();
  }
  impl->page_holder = std::make_unique<blink::DummyPageHolder>(
      gfx::Size(viewport_width, viewport_height));
  return reinterpret_cast<OuiDocument*>(impl);
}

void oui_document_destroy(OuiDocument* doc) {
  if (!doc) {
    return;
  }
  auto* impl = reinterpret_cast<OuiDocumentImpl*>(doc);

  // Delete all element wrappers associated with this document.
  // This releases their Persistent<Element> handles before we tear down
  // the page holder. After this, any OuiElement* handles the caller still
  // holds are dangling — using them is undefined behavior (same contract
  // as free() in any C API).
  {
    auto& map = GetElementMap();
    std::vector<void*> keys_to_remove;
    for (auto& [key, wrapper] : map) {
      if (wrapper->doc == impl) {
        keys_to_remove.push_back(key);
      }
    }
    for (void* key : keys_to_remove) {
      auto it = map.find(key);
      if (it != map.end()) {
        delete it->second;
        map.erase(it);
      }
    }
  }

  // Reset page_holder before task_env (order matters for cleanup).
  impl->page_holder.reset();
  impl->task_env.reset();
  delete impl;
}

void oui_document_set_viewport(OuiDocument* doc, int width, int height) {
  if (!doc) {
    return;
  }
  auto* impl = reinterpret_cast<OuiDocumentImpl*>(doc);
  auto& view = impl->page_holder->GetFrameView();
  // DummyPageHolder fixes layout size to frame size by default.
  // Unlock it before resizing.
  view.SetLayoutSizeFixedToFrameSize(false);
  view.SetLayoutSize(gfx::Size(width, height));
}

OuiStatus oui_document_layout(OuiDocument* doc) {
  if (!doc) {
    return OUI_ERROR_INVALID_ARGUMENT;
  }
  auto* impl = reinterpret_cast<OuiDocumentImpl*>(doc);
  impl->GetDocument().UpdateStyleAndLayout(
      blink::DocumentUpdateReason::kTest);
  return OUI_OK;
}

OuiStatus oui_document_update_all(OuiDocument* doc) {
  if (!doc) {
    return OUI_ERROR_INVALID_ARGUMENT;
  }
  auto* impl = reinterpret_cast<OuiDocumentImpl*>(doc);
  impl->GetDocument().View()->UpdateAllLifecyclePhasesForTest();
  return OUI_OK;
}

// ═══════════════════════════════════════════════════════════════════════════
// Element lifecycle
// ═══════════════════════════════════════════════════════════════════════════

OuiElement* oui_element_create(OuiDocument* doc, const char* tag) {
  if (!doc || !tag) {
    return nullptr;
  }
  auto* doc_impl = reinterpret_cast<OuiDocumentImpl*>(doc);
  blink::Element* elem =
      openui::CreateElementForTag(doc_impl->GetDocument(), tag);
  if (!elem) {
    return nullptr;
  }

  auto* impl = new OuiElementImpl();
  impl->element = elem;
  impl->doc = doc_impl;
  impl->is_body = false;

  RegisterWrapper(elem, impl);
  return reinterpret_cast<OuiElement*>(impl);
}

void oui_element_destroy(OuiElement* e) {
  if (!e) {
    return;
  }
  auto* impl = reinterpret_cast<OuiElementImpl*>(e);

  if (impl->is_body) {
    // The <body> wrapper is owned by the document — not user-destroyable.
    return;
  }

  // Remove from DOM if still attached.
  if (impl->element && impl->element->parentNode()) {
    impl->element->parentNode()->RemoveChild(impl->element.Get());
  }

  UnregisterWrapper(impl->element.Get());
  impl->element = nullptr;
  delete impl;
}

OuiElement* oui_document_body(OuiDocument* doc) {
  if (!doc) {
    return nullptr;
  }
  auto* doc_impl = reinterpret_cast<OuiDocumentImpl*>(doc);
  blink::Element* body = doc_impl->GetDocument().body();
  if (!body) {
    return nullptr;
  }

  // Check if we already have a wrapper.
  OuiElementImpl* existing = FindWrapper(body);
  if (existing) {
    return reinterpret_cast<OuiElement*>(existing);
  }

  // Create a wrapper for <body>. Marked as is_body so it won't be destroyed.
  auto* impl = new OuiElementImpl();
  impl->element = body;
  impl->doc = doc_impl;
  impl->is_body = true;

  RegisterWrapper(body, impl);
  return reinterpret_cast<OuiElement*>(impl);
}

// ═══════════════════════════════════════════════════════════════════════════
// DOM tree manipulation
// ═══════════════════════════════════════════════════════════════════════════

void oui_element_append_child(OuiElement* parent, OuiElement* child) {
  if (!parent || !child) {
    return;
  }
  auto* p = reinterpret_cast<OuiElementImpl*>(parent);
  auto* c = reinterpret_cast<OuiElementImpl*>(child);
  // Reject cross-document insertion to avoid dangling doc back-pointers.
  if (p->doc != c->doc) {
    return;
  }
  if (!p->element || !c->element) {
    return;
  }
  p->element->AppendChild(c->element.Get());
}

void oui_element_remove_child(OuiElement* parent, OuiElement* child) {
  if (!parent || !child) {
    return;
  }
  auto* p = reinterpret_cast<OuiElementImpl*>(parent);
  auto* c = reinterpret_cast<OuiElementImpl*>(child);
  if (!p->element || !c->element) {
    return;
  }
  p->element->RemoveChild(c->element.Get());
}

void oui_element_insert_before(OuiElement* parent,
                               OuiElement* child,
                               OuiElement* before) {
  if (!parent || !child) {
    return;
  }
  auto* p = reinterpret_cast<OuiElementImpl*>(parent);
  auto* c = reinterpret_cast<OuiElementImpl*>(child);
  if (!p->element || !c->element) {
    return;
  }
  // Reject cross-document insertion to avoid dangling doc back-pointers.
  if (p->doc != c->doc) {
    return;
  }
  if (before) {
    auto* b = reinterpret_cast<OuiElementImpl*>(before);
    if (!b->element || b->doc != p->doc) {
      return;
    }
  }
  blink::Node* ref_node = before
                              ? reinterpret_cast<OuiElementImpl*>(before)
                                    ->element.Get()
                              : nullptr;
  p->element->InsertBefore(c->element.Get(), ref_node);
}

OuiElement* oui_element_first_child(const OuiElement* parent) {
  if (!parent) {
    return nullptr;
  }
  auto* p = reinterpret_cast<const OuiElementImpl*>(parent);
  if (!p->element) {
    return nullptr;
  }
  for (blink::Node* child = p->element->firstChild(); child;
       child = child->nextSibling()) {
    if (auto* elem = blink::DynamicTo<blink::Element>(child)) {
      OuiElementImpl* wrapper = FindWrapper(elem);
      if (wrapper) {
        return reinterpret_cast<OuiElement*>(wrapper);
      }
    }
  }
  return nullptr;
}

OuiElement* oui_element_next_sibling(const OuiElement* e) {
  if (!e) {
    return nullptr;
  }
  auto* impl = reinterpret_cast<const OuiElementImpl*>(e);
  if (!impl->element) {
    return nullptr;
  }
  for (blink::Node* sib = impl->element->nextSibling(); sib;
       sib = sib->nextSibling()) {
    if (auto* elem = blink::DynamicTo<blink::Element>(sib)) {
      OuiElementImpl* wrapper = FindWrapper(elem);
      if (wrapper) {
        return reinterpret_cast<OuiElement*>(wrapper);
      }
    }
  }
  return nullptr;
}

OuiElement* oui_element_parent(const OuiElement* e) {
  if (!e) {
    return nullptr;
  }
  auto* impl = reinterpret_cast<const OuiElementImpl*>(e);
  if (!impl->element) {
    return nullptr;
  }
  blink::Element* parent = impl->element->parentElement();
  if (!parent) {
    return nullptr;
  }
  OuiElementImpl* wrapper = FindWrapper(parent);
  return wrapper ? reinterpret_cast<OuiElement*>(wrapper) : nullptr;
}

// ═══════════════════════════════════════════════════════════════════════════
// Generic style API
// ═══════════════════════════════════════════════════════════════════════════

OuiStatus oui_element_set_style(OuiElement* e,
                                const char* property,
                                const char* value) {
  if (!e || !property || !value) {
    return OUI_ERROR_INVALID_ARGUMENT;
  }
  auto* impl = reinterpret_cast<OuiElementImpl*>(e);
  if (!impl->element) {
    return OUI_ERROR_INVALID_ARGUMENT;
  }

  blink::CSSPropertyID id = LookupPropertyID(property);
  if (id == blink::CSSPropertyID::kInvalid) {
    return OUI_ERROR_UNKNOWN_PROPERTY;
  }

  bool ok = impl->element->SetInlineStyleProperty(
      id, blink::String(value));
  return ok ? OUI_OK : OUI_ERROR_INVALID_VALUE;
}

OuiStatus oui_element_remove_style(OuiElement* e, const char* property) {
  if (!e || !property) {
    return OUI_ERROR_INVALID_ARGUMENT;
  }
  auto* impl = reinterpret_cast<OuiElementImpl*>(e);
  if (!impl->element) {
    return OUI_ERROR_INVALID_ARGUMENT;
  }

  blink::CSSPropertyID id = LookupPropertyID(property);
  if (id == blink::CSSPropertyID::kInvalid) {
    return OUI_ERROR_UNKNOWN_PROPERTY;
  }

  impl->element->RemoveInlineStyleProperty(id);
  return OUI_OK;
}

void oui_element_clear_styles(OuiElement* e) {
  if (!e) {
    return;
  }
  auto* impl = reinterpret_cast<OuiElementImpl*>(e);
  if (!impl->element) {
    return;
  }
  impl->element->RemoveAllInlineStyleProperties();
}

// ═══════════════════════════════════════════════════════════════════════════
// Typed convenience setters — layout dimensions
// ═══════════════════════════════════════════════════════════════════════════

void oui_element_set_width(OuiElement* e, OuiLength len) {
  if (!e) return;
  SetLengthProperty(reinterpret_cast<OuiElementImpl*>(e)->element.Get(),
                    blink::CSSPropertyID::kWidth, len);
}

void oui_element_set_height(OuiElement* e, OuiLength len) {
  if (!e) return;
  SetLengthProperty(reinterpret_cast<OuiElementImpl*>(e)->element.Get(),
                    blink::CSSPropertyID::kHeight, len);
}

void oui_element_set_min_width(OuiElement* e, OuiLength len) {
  if (!e) return;
  SetLengthProperty(reinterpret_cast<OuiElementImpl*>(e)->element.Get(),
                    blink::CSSPropertyID::kMinWidth, len);
}

void oui_element_set_min_height(OuiElement* e, OuiLength len) {
  if (!e) return;
  SetLengthProperty(reinterpret_cast<OuiElementImpl*>(e)->element.Get(),
                    blink::CSSPropertyID::kMinHeight, len);
}

void oui_element_set_max_width(OuiElement* e, OuiLength len) {
  if (!e) return;
  SetLengthProperty(reinterpret_cast<OuiElementImpl*>(e)->element.Get(),
                    blink::CSSPropertyID::kMaxWidth, len);
}

void oui_element_set_max_height(OuiElement* e, OuiLength len) {
  if (!e) return;
  SetLengthProperty(reinterpret_cast<OuiElementImpl*>(e)->element.Get(),
                    blink::CSSPropertyID::kMaxHeight, len);
}

// ═══════════════════════════════════════════════════════════════════════════
// Typed convenience setters — box model
// ═══════════════════════════════════════════════════════════════════════════

void oui_element_set_margin(OuiElement* e,
                            OuiLength top,
                            OuiLength right,
                            OuiLength bottom,
                            OuiLength left) {
  if (!e) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;
  SetLengthProperty(elem, blink::CSSPropertyID::kMarginTop, top);
  SetLengthProperty(elem, blink::CSSPropertyID::kMarginRight, right);
  SetLengthProperty(elem, blink::CSSPropertyID::kMarginBottom, bottom);
  SetLengthProperty(elem, blink::CSSPropertyID::kMarginLeft, left);
}

void oui_element_set_padding(OuiElement* e,
                             OuiLength top,
                             OuiLength right,
                             OuiLength bottom,
                             OuiLength left) {
  if (!e) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;
  SetLengthProperty(elem, blink::CSSPropertyID::kPaddingTop, top);
  SetLengthProperty(elem, blink::CSSPropertyID::kPaddingRight, right);
  SetLengthProperty(elem, blink::CSSPropertyID::kPaddingBottom, bottom);
  SetLengthProperty(elem, blink::CSSPropertyID::kPaddingLeft, left);
}

// ═══════════════════════════════════════════════════════════════════════════
// Typed convenience setters — display & positioning
// ═══════════════════════════════════════════════════════════════════════════

void oui_element_set_display(OuiElement* e, OuiDisplay display) {
  if (!e) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;

  const char* value = "block";
  switch (display) {
    case OUI_DISPLAY_BLOCK:        value = "block"; break;
    case OUI_DISPLAY_INLINE:       value = "inline"; break;
    case OUI_DISPLAY_INLINE_BLOCK: value = "inline-block"; break;
    case OUI_DISPLAY_FLEX:         value = "flex"; break;
    case OUI_DISPLAY_INLINE_FLEX:  value = "inline-flex"; break;
    case OUI_DISPLAY_GRID:         value = "grid"; break;
    case OUI_DISPLAY_INLINE_GRID:  value = "inline-grid"; break;
    case OUI_DISPLAY_TABLE:        value = "table"; break;
    case OUI_DISPLAY_TABLE_ROW:    value = "table-row"; break;
    case OUI_DISPLAY_TABLE_CELL:   value = "table-cell"; break;
    case OUI_DISPLAY_NONE:         value = "none"; break;
    case OUI_DISPLAY_CONTENTS:     value = "contents"; break;
  }
  elem->SetInlineStyleProperty(blink::CSSPropertyID::kDisplay,
                               blink::String(value));
}

void oui_element_set_position(OuiElement* e, OuiPosition pos) {
  if (!e) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;

  const char* value = "static";
  switch (pos) {
    case OUI_POSITION_STATIC:   value = "static"; break;
    case OUI_POSITION_RELATIVE: value = "relative"; break;
    case OUI_POSITION_ABSOLUTE: value = "absolute"; break;
    case OUI_POSITION_FIXED:    value = "fixed"; break;
    case OUI_POSITION_STICKY:   value = "sticky"; break;
  }
  elem->SetInlineStyleProperty(blink::CSSPropertyID::kPosition,
                               blink::String(value));
}

void oui_element_set_overflow(OuiElement* e, OuiOverflow overflow) {
  if (!e) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;

  const char* value = "visible";
  switch (overflow) {
    case OUI_OVERFLOW_VISIBLE: value = "visible"; break;
    case OUI_OVERFLOW_HIDDEN:  value = "hidden"; break;
    case OUI_OVERFLOW_SCROLL:  value = "scroll"; break;
    case OUI_OVERFLOW_AUTO:    value = "auto"; break;
  }
  elem->SetInlineStyleProperty(blink::CSSPropertyID::kOverflow,
                               blink::String(value));
}

// ═══════════════════════════════════════════════════════════════════════════
// Typed convenience setters — flexbox
// ═══════════════════════════════════════════════════════════════════════════

void oui_element_set_flex_direction(OuiElement* e, OuiFlexDirection dir) {
  if (!e) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;

  const char* value = "row";
  switch (dir) {
    case OUI_FLEX_ROW:            value = "row"; break;
    case OUI_FLEX_ROW_REVERSE:    value = "row-reverse"; break;
    case OUI_FLEX_COLUMN:         value = "column"; break;
    case OUI_FLEX_COLUMN_REVERSE: value = "column-reverse"; break;
  }
  elem->SetInlineStyleProperty(blink::CSSPropertyID::kFlexDirection,
                               blink::String(value));
}

void oui_element_set_flex_wrap(OuiElement* e, OuiFlexWrap wrap) {
  if (!e) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;

  const char* value = "nowrap";
  switch (wrap) {
    case OUI_FLEX_WRAP_NOWRAP:       value = "nowrap"; break;
    case OUI_FLEX_WRAP_WRAP:         value = "wrap"; break;
    case OUI_FLEX_WRAP_WRAP_REVERSE: value = "wrap-reverse"; break;
  }
  elem->SetInlineStyleProperty(blink::CSSPropertyID::kFlexWrap,
                               blink::String(value));
}

void oui_element_set_flex_grow(OuiElement* e, float grow) {
  if (!e) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;
  elem->SetInlineStyleProperty(blink::CSSPropertyID::kFlexGrow,
                               static_cast<double>(grow),
                               blink::CSSPrimitiveValue::UnitType::kNumber);
}

void oui_element_set_flex_shrink(OuiElement* e, float shrink) {
  if (!e) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;
  elem->SetInlineStyleProperty(blink::CSSPropertyID::kFlexShrink,
                               static_cast<double>(shrink),
                               blink::CSSPrimitiveValue::UnitType::kNumber);
}

void oui_element_set_flex_basis(OuiElement* e, OuiLength basis) {
  if (!e) return;
  SetLengthProperty(reinterpret_cast<OuiElementImpl*>(e)->element.Get(),
                    blink::CSSPropertyID::kFlexBasis, basis);
}

void oui_element_set_align_items(OuiElement* e, OuiAlignItems align) {
  if (!e) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;

  const char* value = "stretch";
  switch (align) {
    case OUI_ALIGN_STRETCH:    value = "stretch"; break;
    case OUI_ALIGN_FLEX_START: value = "flex-start"; break;
    case OUI_ALIGN_FLEX_END:   value = "flex-end"; break;
    case OUI_ALIGN_CENTER:     value = "center"; break;
    case OUI_ALIGN_BASELINE:   value = "baseline"; break;
  }
  elem->SetInlineStyleProperty(blink::CSSPropertyID::kAlignItems,
                               blink::String(value));
}

void oui_element_set_justify_content(OuiElement* e, OuiJustifyContent jc) {
  if (!e) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;

  const char* value = "flex-start";
  switch (jc) {
    case OUI_JUSTIFY_FLEX_START:    value = "flex-start"; break;
    case OUI_JUSTIFY_FLEX_END:      value = "flex-end"; break;
    case OUI_JUSTIFY_CENTER:        value = "center"; break;
    case OUI_JUSTIFY_SPACE_BETWEEN: value = "space-between"; break;
    case OUI_JUSTIFY_SPACE_AROUND:  value = "space-around"; break;
    case OUI_JUSTIFY_SPACE_EVENLY:  value = "space-evenly"; break;
  }
  elem->SetInlineStyleProperty(blink::CSSPropertyID::kJustifyContent,
                               blink::String(value));
}

// ═══════════════════════════════════════════════════════════════════════════
// Typed convenience setters — colors & visuals
// ═══════════════════════════════════════════════════════════════════════════

void oui_element_set_color(OuiElement* e, uint32_t rgba) {
  if (!e) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;
  std::string css = RGBAToCSSString(rgba);
  elem->SetInlineStyleProperty(blink::CSSPropertyID::kColor,
                               blink::String(css.c_str()));
}

void oui_element_set_background_color(OuiElement* e, uint32_t rgba) {
  if (!e) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;
  std::string css = RGBAToCSSString(rgba);
  elem->SetInlineStyleProperty(blink::CSSPropertyID::kBackgroundColor,
                               blink::String(css.c_str()));
}

void oui_element_set_opacity(OuiElement* e, float opacity) {
  if (!e) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;
  elem->SetInlineStyleProperty(blink::CSSPropertyID::kOpacity,
                               static_cast<double>(opacity),
                               blink::CSSPrimitiveValue::UnitType::kNumber);
}

void oui_element_set_z_index(OuiElement* e, int z) {
  if (!e) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;
  elem->SetInlineStyleProperty(blink::CSSPropertyID::kZIndex,
                               static_cast<double>(z),
                               blink::CSSPrimitiveValue::UnitType::kInteger);
}

// ═══════════════════════════════════════════════════════════════════════════
// Text content
// ═══════════════════════════════════════════════════════════════════════════

void oui_element_set_text_content(OuiElement* e, const char* text) {
  if (!e) return;
  auto* impl = reinterpret_cast<OuiElementImpl*>(e);
  if (!impl->element) return;
  impl->element->setTextContent(blink::String(text));
}

// ═══════════════════════════════════════════════════════════════════════════
// Font convenience setters
// ═══════════════════════════════════════════════════════════════════════════

void oui_element_set_font_family(OuiElement* e, const char* family) {
  if (!e || !family) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;
  elem->SetInlineStyleProperty(blink::CSSPropertyID::kFontFamily,
                               blink::String(family));
}

void oui_element_set_font_size(OuiElement* e, OuiLength size) {
  if (!e) return;
  SetLengthProperty(reinterpret_cast<OuiElementImpl*>(e)->element.Get(),
                    blink::CSSPropertyID::kFontSize, size);
}

void oui_element_set_font_weight(OuiElement* e, int weight) {
  if (!e) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;
  elem->SetInlineStyleProperty(blink::CSSPropertyID::kFontWeight,
                               static_cast<double>(weight),
                               blink::CSSPrimitiveValue::UnitType::kNumber);
}

void oui_element_set_font_style(OuiElement* e, OuiFontStyle style) {
  if (!e) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;

  const char* value = "normal";
  switch (style) {
    case OUI_FONT_STYLE_NORMAL:  value = "normal"; break;
    case OUI_FONT_STYLE_ITALIC:  value = "italic"; break;
    case OUI_FONT_STYLE_OBLIQUE: value = "oblique"; break;
  }
  elem->SetInlineStyleProperty(blink::CSSPropertyID::kFontStyle,
                               blink::String(value));
}

void oui_element_set_line_height(OuiElement* e, OuiLength lh) {
  if (!e) return;
  SetLengthProperty(reinterpret_cast<OuiElementImpl*>(e)->element.Get(),
                    blink::CSSPropertyID::kLineHeight, lh);
}

void oui_element_set_text_align(OuiElement* e, OuiTextAlign align) {
  if (!e) return;
  auto* elem = reinterpret_cast<OuiElementImpl*>(e)->element.Get();
  if (!elem) return;

  const char* value = "left";
  switch (align) {
    case OUI_TEXT_ALIGN_LEFT:    value = "left"; break;
    case OUI_TEXT_ALIGN_RIGHT:   value = "right"; break;
    case OUI_TEXT_ALIGN_CENTER:  value = "center"; break;
    case OUI_TEXT_ALIGN_JUSTIFY: value = "justify"; break;
  }
  elem->SetInlineStyleProperty(blink::CSSPropertyID::kTextAlign,
                               blink::String(value));
}

// ═══════════════════════════════════════════════════════════════════════════
// Geometry queries
// ═══════════════════════════════════════════════════════════════════════════

float oui_element_get_offset_x(const OuiElement* e) {
  if (!e) return 0.0f;
  auto* impl = reinterpret_cast<const OuiElementImpl*>(e);
  if (!impl->element) return 0.0f;
  auto* mutable_elem = const_cast<blink::Element*>(impl->element.Get());
  return static_cast<float>(mutable_elem->OffsetLeft());
}

float oui_element_get_offset_y(const OuiElement* e) {
  if (!e) return 0.0f;
  auto* impl = reinterpret_cast<const OuiElementImpl*>(e);
  if (!impl->element) return 0.0f;
  auto* mutable_elem = const_cast<blink::Element*>(impl->element.Get());
  return static_cast<float>(mutable_elem->OffsetTop());
}

float oui_element_get_width(const OuiElement* e) {
  if (!e) return 0.0f;
  auto* impl = reinterpret_cast<const OuiElementImpl*>(e);
  if (!impl->element) return 0.0f;
  blink::LayoutObject* lo = impl->element->GetLayoutObject();
  if (!lo || !lo->IsBox()) {
    return 0.0f;
  }
  return static_cast<float>(
      blink::To<blink::LayoutBox>(lo)->OffsetWidth().ToFloat());
}

float oui_element_get_height(const OuiElement* e) {
  if (!e) return 0.0f;
  auto* impl = reinterpret_cast<const OuiElementImpl*>(e);
  if (!impl->element) return 0.0f;
  blink::LayoutObject* lo = impl->element->GetLayoutObject();
  if (!lo || !lo->IsBox()) {
    return 0.0f;
  }
  return static_cast<float>(
      blink::To<blink::LayoutBox>(lo)->OffsetHeight().ToFloat());
}

OuiRect oui_element_get_bounding_rect(const OuiElement* e) {
  OuiRect rect = {0, 0, 0, 0};
  if (!e) return rect;
  auto* impl = reinterpret_cast<const OuiElementImpl*>(e);
  if (!impl->element) return rect;
  auto* mutable_elem = const_cast<blink::Element*>(impl->element.Get());

  rect.x = static_cast<float>(mutable_elem->OffsetLeft());
  rect.y = static_cast<float>(mutable_elem->OffsetTop());

  blink::LayoutObject* lo = impl->element->GetLayoutObject();
  if (lo && lo->IsBox()) {
    auto* box = blink::To<blink::LayoutBox>(lo);
    rect.width = box->OffsetWidth().ToFloat();
    rect.height = box->OffsetHeight().ToFloat();
  }
  return rect;
}

// ═══════════════════════════════════════════════════════════════════════════
// Computed style readback
// ═══════════════════════════════════════════════════════════════════════════

char* oui_element_get_computed_style(const OuiElement* e,
                                     const char* property) {
  if (!e || !property) {
    return nullptr;
  }
  auto* impl = reinterpret_cast<const OuiElementImpl*>(e);
  if (!impl->element) {
    return nullptr;
  }

  const blink::ComputedStyle* style = impl->element->GetComputedStyle();
  if (!style) {
    return nullptr;
  }

  blink::CSSPropertyID id = LookupPropertyID(property);
  if (id == blink::CSSPropertyID::kInvalid) {
    return nullptr;
  }

  const blink::CSSProperty& css_prop = blink::CSSProperty::Get(id);
  const blink::CSSValue* css_value =
      css_prop.CSSValueFromComputedStyleInternal(
          *style, nullptr /* layout_object */, false /* allow_visited_style */,
          blink::CSSValuePhase::kComputedValue);
  if (!css_value) {
    return nullptr;
  }

  blink::String str = css_value->CssText();
  std::string utf8 = str.Utf8();
  return strdup(utf8.c_str());
}

// ═══════════════════════════════════════════════════════════════════════════
// Hit testing
// ═══════════════════════════════════════════════════════════════════════════

OuiElement* oui_document_hit_test(OuiDocument* doc, float x, float y) {
  if (!doc) {
    return nullptr;
  }
  auto* doc_impl = reinterpret_cast<OuiDocumentImpl*>(doc);

  // Ensure paint is up to date.
  doc_impl->GetDocument().View()->UpdateAllLifecyclePhasesForTest();

  blink::HitTestLocation location(gfx::PointF(x, y));
  blink::HitTestResult result;
  blink::LayoutView* layout_view =
      doc_impl->GetDocument().GetLayoutView();
  if (!layout_view) {
    return nullptr;
  }

  layout_view->HitTest(location, result);

  blink::Element* hit_elem = result.InnerElement();
  if (!hit_elem) {
    return nullptr;
  }

  return reinterpret_cast<OuiElement*>(FindWrapper(hit_elem));
}

// ═══════════════════════════════════════════════════════════════════════════
// Scroll geometry
// ═══════════════════════════════════════════════════════════════════════════

float oui_element_get_scroll_width(const OuiElement* e) {
  if (!e) return 0.0f;
  auto* impl = reinterpret_cast<const OuiElementImpl*>(e);
  if (!impl->element) return 0.0f;
  auto* mutable_elem = const_cast<blink::Element*>(impl->element.Get());
  return static_cast<float>(mutable_elem->scrollWidth());
}

float oui_element_get_scroll_height(const OuiElement* e) {
  if (!e) return 0.0f;
  auto* impl = reinterpret_cast<const OuiElementImpl*>(e);
  if (!impl->element) return 0.0f;
  auto* mutable_elem = const_cast<blink::Element*>(impl->element.Get());
  return static_cast<float>(mutable_elem->scrollHeight());
}
