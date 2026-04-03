// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui_impl.h — Internal C++ types backing the opaque C handles.

#ifndef OPENUI_OPENUI_IMPL_H_
#define OPENUI_OPENUI_IMPL_H_

#include <memory>

#include "base/memory/raw_ptr.h"
#include "openui/openui_resource_provider.h"
#include "third_party/blink/renderer/core/dom/document.h"
#include "third_party/blink/renderer/core/dom/element.h"
#include "third_party/blink/renderer/core/testing/dummy_page_holder.h"
#include "third_party/blink/renderer/platform/heap/persistent.h"
#include "third_party/blink/renderer/platform/testing/task_environment.h"
#include "ui/gfx/geometry/size.h"

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

  blink::Document& GetDocument() { return page_holder->GetDocument(); }
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
};

#endif  // OPENUI_OPENUI_IMPL_H_
