// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui_impl.h — Internal C++ types backing the opaque C handles.

#ifndef OPENUI_OPENUI_IMPL_H_
#define OPENUI_OPENUI_IMPL_H_

#include <map>
#include <memory>
#include <string>

#include "base/memory/raw_ptr.h"
#include "base/time/time.h"
#include "openui/openui_resource_provider.h"
#include "third_party/blink/renderer/core/dom/document.h"
#include "third_party/blink/renderer/core/dom/element.h"
#include "third_party/blink/renderer/core/testing/dummy_page_holder.h"
#include "third_party/blink/renderer/platform/heap/persistent.h"
#include "third_party/blink/renderer/platform/testing/task_environment.h"
#include "ui/gfx/geometry/size.h"

// Forward-declare the GC'd listener so we don't pull Blink event headers here.
namespace blink {
class NativeEventListener;
}

// OuiDocumentImpl — backs the opaque OuiDocument* handle.
// Owns a DummyPageHolder (which owns a Page, Frame, Document, FrameView)
// and a TaskEnvironment (required for blink's threading infrastructure).
struct OuiDocumentImpl {
  OuiDocumentImpl();
  ~OuiDocumentImpl();

  std::unique_ptr<blink::test::TaskEnvironment> task_env;
  std::unique_ptr<blink::DummyPageHolder> page_holder;

  // Resource provider callback state. When callback is non-null, the document
  // was created with a ResourceProviderFrameClient that intercepts URL loads.
  openui::ResourceProviderState resource_provider;

  // SP7: Animation time epoch. Set on first call to any time function.
  bool time_initialized = false;
  base::TimeTicks time_epoch;
  double current_time_ms = 0.0;

  blink::Document& GetDocument() { return page_holder->GetDocument(); }
};

// Per-event-type callback registration stored on OuiElementImpl.
struct OuiCallbackEntry {
  OuiCallbackEntry();
  ~OuiCallbackEntry();

  __attribute__((annotate("raw_ptr_exclusion")))
  void* callback = nullptr;   // OuiEventCallback cast to void*
  __attribute__((annotate("raw_ptr_exclusion")))
  void* user_data = nullptr;
  blink::Persistent<blink::NativeEventListener> listener;
};

// OuiElementImpl — backs the opaque OuiElement* handle.
// Holds a GC-safe persistent reference to a blink Element.
// The `doc` pointer is non-owning — the element must not outlive its document.
struct OuiElementImpl {
  OuiElementImpl();
  ~OuiElementImpl();

  blink::Persistent<blink::Element> element;
  raw_ptr<OuiDocumentImpl> doc;  // Non-owning back-reference.
  bool is_body = false;  // True for the <body> wrapper (not user-destroyable).

  // SP7: Event callbacks keyed by event type string.
  std::map<std::string, OuiCallbackEntry> callbacks;
};

// Invalidate and delete all OuiElementImpl wrappers associated with |impl|.
// Call before destroying/recreating the DummyPageHolder so that stale
// Persistent<Element> handles don't dangle.
void OuiInvalidateElementWrappers(OuiDocumentImpl* impl);

// Look up an OuiElementImpl* from a raw blink::Element*.
// Returns nullptr if no wrapper is registered for this element.
OuiElementImpl* LookupElementWrapper(blink::Element* elem);

// Register a wrapper in the global element map.
void RegisterWrapper(blink::Element* elem, OuiElementImpl* wrapper);

// Unregister a wrapper from the global element map.
void UnregisterWrapper(blink::Element* elem);

#endif  // OPENUI_OPENUI_IMPL_H_
