// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui_events.h — Internal C++ helpers for SP7 event dispatch and callbacks.

#ifndef OPENUI_OPENUI_EVENTS_H_
#define OPENUI_OPENUI_EVENTS_H_

#include "openui/openui.h"
#include "openui/openui_impl.h"

#include "third_party/blink/renderer/core/dom/events/native_event_listener.h"
#include "third_party/blink/renderer/platform/heap/garbage_collected.h"

// OuiNativeEventListener — GarbageCollected Blink EventListener that
// delegates to a C function pointer (OuiEventCallback).
class OuiNativeEventListener final : public blink::NativeEventListener {
 public:
  OuiNativeEventListener(OuiEventCallback callback,
                          void* user_data,
                          OuiElementImpl* owner);

  void Invoke(blink::ExecutionContext*, blink::Event*) override;
  void Trace(blink::Visitor*) const override;

  // Called during element destruction to null the owner pointer,
  // preventing use-after-free if this listener outlives its owner.
  void ClearOwner() { owner_ = nullptr; }

 private:
  OuiEventCallback callback_;
  __attribute__((annotate("raw_ptr_exclusion")))
  void* user_data_;
  // Non-GC raw pointer back to the OuiElementImpl that owns this listener.
  __attribute__((annotate("raw_ptr_exclusion")))
  OuiElementImpl* owner_;
};

#endif  // OPENUI_OPENUI_EVENTS_H_
