// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui_element_factory.h — Maps tag name strings to blink HTML element
// constructors.

#ifndef OPENUI_OPENUI_ELEMENT_FACTORY_H_
#define OPENUI_OPENUI_ELEMENT_FACTORY_H_

#include "third_party/blink/renderer/core/dom/document.h"
#include "third_party/blink/renderer/core/dom/element.h"

namespace openui {

// Creates a blink HTML element for the given tag name (e.g., "div", "span").
// Returns nullptr if the tag is not supported.
blink::Element* CreateElementForTag(blink::Document& doc, const char* tag);

}  // namespace openui

#endif  // OPENUI_OPENUI_ELEMENT_FACTORY_H_
