// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui_element_factory.cc — Tag name → HTMLElement constructor mapping.

#include "openui/openui_element_factory.h"

#include <string.h>
#include <string_view>

#include "base/compiler_specific.h"
#include "third_party/blink/renderer/core/html/html_anchor_element.h"
#include "third_party/blink/renderer/core/html/html_body_element.h"
#include "third_party/blink/renderer/core/html/html_br_element.h"
#include "third_party/blink/renderer/core/html/html_details_element.h"
#include "third_party/blink/renderer/core/html/html_dialog_element.h"
#include "third_party/blink/renderer/core/html/html_div_element.h"
#include "third_party/blink/renderer/core/html/html_dlist_element.h"
#include "third_party/blink/renderer/core/html/html_heading_element.h"
#include "third_party/blink/renderer/core/html/html_hr_element.h"
#include "third_party/blink/renderer/core/html/html_html_element.h"
#include "third_party/blink/renderer/core/html/html_image_element.h"
#include "third_party/blink/renderer/core/html/html_li_element.h"
#include "third_party/blink/renderer/core/html/html_meter_element.h"
#include "third_party/blink/renderer/core/html/html_olist_element.h"
#include "third_party/blink/renderer/core/html/html_paragraph_element.h"
#include "third_party/blink/renderer/core/html/html_pre_element.h"
#include "third_party/blink/renderer/core/html/html_progress_element.h"
#include "third_party/blink/renderer/core/html/html_quote_element.h"
#include "third_party/blink/renderer/core/html/html_span_element.h"
#include "third_party/blink/renderer/core/html/html_summary_element.h"
#include "third_party/blink/renderer/core/html/html_table_caption_element.h"
#include "third_party/blink/renderer/core/html/html_table_cell_element.h"
#include "third_party/blink/renderer/core/html/html_table_col_element.h"
#include "third_party/blink/renderer/core/html/html_table_element.h"
#include "third_party/blink/renderer/core/html/html_table_row_element.h"
#include "third_party/blink/renderer/core/html/html_table_section_element.h"
#include "third_party/blink/renderer/core/html/html_ulist_element.h"
#include "third_party/blink/renderer/core/html/html_unknown_element.h"
#include "third_party/blink/renderer/core/html/html_wbr_element.h"
#include "third_party/blink/renderer/platform/heap/garbage_collected.h"

// Form elements.
#include "third_party/blink/renderer/core/html/forms/html_button_element.h"
#include "third_party/blink/renderer/core/html/forms/html_field_set_element.h"
#include "third_party/blink/renderer/core/html/forms/html_form_element.h"
#include "third_party/blink/renderer/core/html/forms/html_input_element.h"
#include "third_party/blink/renderer/core/html/forms/html_label_element.h"
#include "third_party/blink/renderer/core/html/forms/html_legend_element.h"
#include "third_party/blink/renderer/core/html/forms/html_option_element.h"
#include "third_party/blink/renderer/core/html/forms/html_select_element.h"
#include "third_party/blink/renderer/core/html/forms/html_text_area_element.h"

namespace openui {

namespace {

struct TagEntry {
  const char* tag;
  blink::Element* (*create)(blink::Document&);
};

template <typename T>
blink::Element* Create(blink::Document& doc) {
  return blink::MakeGarbageCollected<T>(doc);
}

// Heading elements need a special factory (they take a QualifiedName).
blink::Element* CreateH1(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLHeadingElement>(
      blink::html_names::kH1Tag, doc);
}
blink::Element* CreateH2(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLHeadingElement>(
      blink::html_names::kH2Tag, doc);
}
blink::Element* CreateH3(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLHeadingElement>(
      blink::html_names::kH3Tag, doc);
}
blink::Element* CreateH4(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLHeadingElement>(
      blink::html_names::kH4Tag, doc);
}
blink::Element* CreateH5(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLHeadingElement>(
      blink::html_names::kH5Tag, doc);
}
blink::Element* CreateH6(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLHeadingElement>(
      blink::html_names::kH6Tag, doc);
}

// Table section elements need QualifiedName too.
blink::Element* CreateThead(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLTableSectionElement>(
      blink::html_names::kTheadTag, doc);
}
blink::Element* CreateTbody(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLTableSectionElement>(
      blink::html_names::kTbodyTag, doc);
}
blink::Element* CreateTfoot(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLTableSectionElement>(
      blink::html_names::kTfootTag, doc);
}

// Table cell elements.
blink::Element* CreateTd(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLTableCellElement>(
      blink::html_names::kTdTag, doc);
}
blink::Element* CreateTh(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLTableCellElement>(
      blink::html_names::kThTag, doc);
}

// Quote elements.
blink::Element* CreateBlockquote(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLQuoteElement>(
      blink::html_names::kBlockquoteTag, doc);
}
blink::Element* CreateQ(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLQuoteElement>(
      blink::html_names::kQTag, doc);
}

// Pre element variants.
blink::Element* CreatePre(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLPreElement>(
      blink::html_names::kPreTag, doc);
}

// Col elements.
blink::Element* CreateCol(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLTableColElement>(
      blink::html_names::kColTag, doc);
}
blink::Element* CreateColgroup(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLTableColElement>(
      blink::html_names::kColgroupTag, doc);
}

// Sorted by tag name for binary search.
const TagEntry kTagTable[] = {
    {"a", Create<blink::HTMLAnchorElement>},
    {"blockquote", CreateBlockquote},
    {"body", Create<blink::HTMLBodyElement>},
    {"br", Create<blink::HTMLBRElement>},
    {"button", Create<blink::HTMLButtonElement>},
    {"caption", Create<blink::HTMLTableCaptionElement>},
    {"col", CreateCol},
    {"colgroup", CreateColgroup},
    {"details", Create<blink::HTMLDetailsElement>},
    {"dialog", Create<blink::HTMLDialogElement>},
    {"div", Create<blink::HTMLDivElement>},
    {"dl", Create<blink::HTMLDListElement>},
    {"fieldset", Create<blink::HTMLFieldSetElement>},
    {"form", Create<blink::HTMLFormElement>},
    {"h1", CreateH1},
    {"h2", CreateH2},
    {"h3", CreateH3},
    {"h4", CreateH4},
    {"h5", CreateH5},
    {"h6", CreateH6},
    {"hr", Create<blink::HTMLHRElement>},
    {"img", Create<blink::HTMLImageElement>},
    {"input", Create<blink::HTMLInputElement>},
    {"label", Create<blink::HTMLLabelElement>},
    {"legend", Create<blink::HTMLLegendElement>},
    {"li", Create<blink::HTMLLIElement>},
    {"meter", Create<blink::HTMLMeterElement>},
    {"ol", Create<blink::HTMLOListElement>},
    {"option", Create<blink::HTMLOptionElement>},
    {"p", Create<blink::HTMLParagraphElement>},
    {"pre", CreatePre},
    {"progress", Create<blink::HTMLProgressElement>},
    {"q", CreateQ},
    {"select", Create<blink::HTMLSelectElement>},
    {"span", Create<blink::HTMLSpanElement>},
    {"summary", Create<blink::HTMLSummaryElement>},
    {"table", Create<blink::HTMLTableElement>},
    {"tbody", CreateTbody},
    {"td", CreateTd},
    {"textarea", Create<blink::HTMLTextAreaElement>},
    {"tfoot", CreateTfoot},
    {"th", CreateTh},
    {"thead", CreateThead},
    {"tr", Create<blink::HTMLTableRowElement>},
    {"ul", Create<blink::HTMLUListElement>},
    {"wbr", Create<blink::HTMLWBRElement>},
};

constexpr size_t kTagTableSize = sizeof(kTagTable) / sizeof(kTagTable[0]);

}  // namespace

blink::Element* CreateElementForTag(blink::Document& doc, const char* tag) {
  if (!tag) {
    return nullptr;
  }

  std::string_view tag_view(tag);
  for (size_t i = 0; i < kTagTableSize; i++) {
    // SAFETY: kTagTable is a compile-time constant array with kTagTableSize
    // entries. The loop bound ensures i < kTagTableSize.
    if (tag_view == UNSAFE_BUFFERS(kTagTable[i].tag)) {
      return UNSAFE_BUFFERS(kTagTable[i].create(doc));
    }
  }
  return nullptr;
}

}  // namespace openui
