// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui_element_factory.cc — Tag name → Element constructor mapping.

#include "openui/openui_element_factory.h"

#include <string.h>
#include <string_view>

#include "base/compiler_specific.h"

// Base HTMLElement (used for generic semantic elements).
#include "third_party/blink/renderer/core/html/html_element.h"

// Specific HTML element headers.
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
#include "third_party/blink/renderer/core/html/html_picture_element.h"
#include "third_party/blink/renderer/core/html/html_pre_element.h"
#include "third_party/blink/renderer/core/html/html_progress_element.h"
#include "third_party/blink/renderer/core/html/html_quote_element.h"
#include "third_party/blink/renderer/core/html/html_source_element.h"
#include "third_party/blink/renderer/core/html/html_span_element.h"
#include "third_party/blink/renderer/core/html/html_summary_element.h"
#include "third_party/blink/renderer/core/html/html_table_caption_element.h"
#include "third_party/blink/renderer/core/html/html_table_cell_element.h"
#include "third_party/blink/renderer/core/html/html_table_col_element.h"
#include "third_party/blink/renderer/core/html/html_table_element.h"
#include "third_party/blink/renderer/core/html/html_table_row_element.h"
#include "third_party/blink/renderer/core/html/html_table_section_element.h"
#include "third_party/blink/renderer/core/html/html_time_element.h"
#include "third_party/blink/renderer/core/html/html_ulist_element.h"
#include "third_party/blink/renderer/core/html/html_unknown_element.h"
#include "third_party/blink/renderer/core/html/html_wbr_element.h"
#include "third_party/blink/renderer/platform/heap/garbage_collected.h"

// Canvas element.
#include "third_party/blink/renderer/core/html/canvas/html_canvas_element.h"

// Form elements.
#include "third_party/blink/renderer/core/html/forms/html_button_element.h"
#include "third_party/blink/renderer/core/html/forms/html_data_list_element.h"
#include "third_party/blink/renderer/core/html/forms/html_field_set_element.h"
#include "third_party/blink/renderer/core/html/forms/html_form_element.h"
#include "third_party/blink/renderer/core/html/forms/html_input_element.h"
#include "third_party/blink/renderer/core/html/forms/html_label_element.h"
#include "third_party/blink/renderer/core/html/forms/html_legend_element.h"
#include "third_party/blink/renderer/core/html/forms/html_opt_group_element.h"
#include "third_party/blink/renderer/core/html/forms/html_option_element.h"
#include "third_party/blink/renderer/core/html/forms/html_output_element.h"
#include "third_party/blink/renderer/core/html/forms/html_select_element.h"
#include "third_party/blink/renderer/core/html/forms/html_text_area_element.h"

// Media elements.
#include "third_party/blink/renderer/core/html/media/html_audio_element.h"
#include "third_party/blink/renderer/core/html/media/html_video_element.h"

// SVG elements.
#include "third_party/blink/renderer/core/svg/svg_animate_element.h"
#include "third_party/blink/renderer/core/svg/svg_animate_motion_element.h"
#include "third_party/blink/renderer/core/svg/svg_animate_transform_element.h"
#include "third_party/blink/renderer/core/svg/svg_circle_element.h"
#include "third_party/blink/renderer/core/svg/svg_clip_path_element.h"
#include "third_party/blink/renderer/core/svg/svg_defs_element.h"
#include "third_party/blink/renderer/core/svg/svg_ellipse_element.h"
#include "third_party/blink/renderer/core/svg/svg_fe_color_matrix_element.h"
#include "third_party/blink/renderer/core/svg/svg_fe_composite_element.h"
#include "third_party/blink/renderer/core/svg/svg_fe_flood_element.h"
#include "third_party/blink/renderer/core/svg/svg_fe_gaussian_blur_element.h"
#include "third_party/blink/renderer/core/svg/svg_fe_merge_element.h"
#include "third_party/blink/renderer/core/svg/svg_fe_merge_node_element.h"
#include "third_party/blink/renderer/core/svg/svg_fe_offset_element.h"
#include "third_party/blink/renderer/core/svg/svg_filter_element.h"
#include "third_party/blink/renderer/core/svg/svg_foreign_object_element.h"
#include "third_party/blink/renderer/core/svg/svg_g_element.h"
#include "third_party/blink/renderer/core/svg/svg_image_element.h"
#include "third_party/blink/renderer/core/svg/svg_line_element.h"
#include "third_party/blink/renderer/core/svg/svg_linear_gradient_element.h"
#include "third_party/blink/renderer/core/svg/svg_marker_element.h"
#include "third_party/blink/renderer/core/svg/svg_mask_element.h"
#include "third_party/blink/renderer/core/svg/svg_path_element.h"
#include "third_party/blink/renderer/core/svg/svg_pattern_element.h"
#include "third_party/blink/renderer/core/svg/svg_polygon_element.h"
#include "third_party/blink/renderer/core/svg/svg_polyline_element.h"
#include "third_party/blink/renderer/core/svg/svg_radial_gradient_element.h"
#include "third_party/blink/renderer/core/svg/svg_rect_element.h"
#include "third_party/blink/renderer/core/svg/svg_stop_element.h"
#include "third_party/blink/renderer/core/svg/svg_svg_element.h"
#include "third_party/blink/renderer/core/svg/svg_symbol_element.h"
#include "third_party/blink/renderer/core/svg/svg_text_element.h"
#include "third_party/blink/renderer/core/svg/svg_text_path_element.h"
#include "third_party/blink/renderer/core/svg/svg_tspan_element.h"
#include "third_party/blink/renderer/core/svg/svg_use_element.h"

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

// ---------------------------------------------------------------------------
// Generic HTMLElement factories (elements that use the base HTMLElement class
// with a QualifiedName tag).
// ---------------------------------------------------------------------------

blink::Element* CreateAbbr(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kAbbrTag, doc);
}
blink::Element* CreateAddress(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kAddressTag, doc);
}
blink::Element* CreateArticle(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kArticleTag, doc);
}
blink::Element* CreateAside(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kAsideTag, doc);
}
blink::Element* CreateB(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kBTag, doc);
}
blink::Element* CreateCode(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kCodeTag, doc);
}
blink::Element* CreateDd(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kDdTag, doc);
}
blink::Element* CreateDt(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kDtTag, doc);
}
blink::Element* CreateEm(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kEmTag, doc);
}
blink::Element* CreateFigcaption(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kFigcaptionTag, doc);
}
blink::Element* CreateFigure(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kFigureTag, doc);
}
blink::Element* CreateFooter(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kFooterTag, doc);
}
blink::Element* CreateHeader(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kHeaderTag, doc);
}
blink::Element* CreateI(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kITag, doc);
}
blink::Element* CreateKbd(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kKbdTag, doc);
}
blink::Element* CreateMain(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kMainTag, doc);
}
blink::Element* CreateMark(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kMarkTag, doc);
}
blink::Element* CreateNav(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kNavTag, doc);
}
blink::Element* CreateS(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kSTag, doc);
}
blink::Element* CreateSamp(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kSampTag, doc);
}
blink::Element* CreateSection(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kSectionTag, doc);
}
blink::Element* CreateSmall(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kSmallTag, doc);
}
blink::Element* CreateStrong(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kStrongTag, doc);
}
blink::Element* CreateSub(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kSubTag, doc);
}
blink::Element* CreateSup(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kSupTag, doc);
}
blink::Element* CreateU(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kUTag, doc);
}
blink::Element* CreateVar(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLElement>(
      blink::html_names::kVarTag, doc);
}

// ---------------------------------------------------------------------------
// Heading elements (QualifiedName variants).
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Table elements (QualifiedName variants).
// ---------------------------------------------------------------------------

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
blink::Element* CreateTd(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLTableCellElement>(
      blink::html_names::kTdTag, doc);
}
blink::Element* CreateTh(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLTableCellElement>(
      blink::html_names::kThTag, doc);
}
blink::Element* CreateCol(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLTableColElement>(
      blink::html_names::kColTag, doc);
}
blink::Element* CreateColgroup(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLTableColElement>(
      blink::html_names::kColgroupTag, doc);
}

// ---------------------------------------------------------------------------
// Quote / pre elements (QualifiedName variants).
// ---------------------------------------------------------------------------

blink::Element* CreateBlockquote(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLQuoteElement>(
      blink::html_names::kBlockquoteTag, doc);
}
blink::Element* CreateQ(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLQuoteElement>(
      blink::html_names::kQTag, doc);
}
blink::Element* CreatePre(blink::Document& doc) {
  return blink::MakeGarbageCollected<blink::HTMLPreElement>(
      blink::html_names::kPreTag, doc);
}

// ---------------------------------------------------------------------------
// Tag table — sorted alphabetically for binary search.
// ---------------------------------------------------------------------------

const TagEntry kTagTable[] = {
    // --- A ---
    {"a", Create<blink::HTMLAnchorElement>},
    {"abbr", CreateAbbr},
    {"address", CreateAddress},
    {"animate", Create<blink::SVGAnimateElement>},
    {"animateMotion", Create<blink::SVGAnimateMotionElement>},
    {"animateTransform", Create<blink::SVGAnimateTransformElement>},
    {"article", CreateArticle},
    {"aside", CreateAside},
    {"audio", Create<blink::HTMLAudioElement>},
    // --- B ---
    {"b", CreateB},
    {"blockquote", CreateBlockquote},
    {"body", Create<blink::HTMLBodyElement>},
    {"br", Create<blink::HTMLBRElement>},
    {"button", Create<blink::HTMLButtonElement>},
    // --- C ---
    {"canvas", Create<blink::HTMLCanvasElement>},
    {"caption", Create<blink::HTMLTableCaptionElement>},
    {"circle", Create<blink::SVGCircleElement>},
    {"clipPath", Create<blink::SVGClipPathElement>},
    {"code", CreateCode},
    {"col", CreateCol},
    {"colgroup", CreateColgroup},
    // --- D ---
    {"datalist", Create<blink::HTMLDataListElement>},
    {"dd", CreateDd},
    {"defs", Create<blink::SVGDefsElement>},
    {"details", Create<blink::HTMLDetailsElement>},
    {"dialog", Create<blink::HTMLDialogElement>},
    {"div", Create<blink::HTMLDivElement>},
    {"dl", Create<blink::HTMLDListElement>},
    {"dt", CreateDt},
    // --- E ---
    {"ellipse", Create<blink::SVGEllipseElement>},
    {"em", CreateEm},
    // --- F ---
    {"feColorMatrix", Create<blink::SVGFEColorMatrixElement>},
    {"feComposite", Create<blink::SVGFECompositeElement>},
    {"feFlood", Create<blink::SVGFEFloodElement>},
    {"feGaussianBlur", Create<blink::SVGFEGaussianBlurElement>},
    {"feMerge", Create<blink::SVGFEMergeElement>},
    {"feMergeNode", Create<blink::SVGFEMergeNodeElement>},
    {"feOffset", Create<blink::SVGFEOffsetElement>},
    {"fieldset", Create<blink::HTMLFieldSetElement>},
    {"figcaption", CreateFigcaption},
    {"figure", CreateFigure},
    {"filter", Create<blink::SVGFilterElement>},
    {"footer", CreateFooter},
    {"foreignObject", Create<blink::SVGForeignObjectElement>},
    {"form", Create<blink::HTMLFormElement>},
    // --- G ---
    {"g", Create<blink::SVGGElement>},
    // --- H ---
    {"h1", CreateH1},
    {"h2", CreateH2},
    {"h3", CreateH3},
    {"h4", CreateH4},
    {"h5", CreateH5},
    {"h6", CreateH6},
    {"header", CreateHeader},
    {"hr", Create<blink::HTMLHRElement>},
    // --- I ---
    {"i", CreateI},
    {"image", Create<blink::SVGImageElement>},
    {"img", Create<blink::HTMLImageElement>},
    {"input", Create<blink::HTMLInputElement>},
    // --- K ---
    {"kbd", CreateKbd},
    // --- L ---
    {"label", Create<blink::HTMLLabelElement>},
    {"legend", Create<blink::HTMLLegendElement>},
    {"li", Create<blink::HTMLLIElement>},
    {"line", Create<blink::SVGLineElement>},
    {"linearGradient", Create<blink::SVGLinearGradientElement>},
    // --- M ---
    {"main", CreateMain},
    {"mark", CreateMark},
    {"marker", Create<blink::SVGMarkerElement>},
    {"mask", Create<blink::SVGMaskElement>},
    {"meter", Create<blink::HTMLMeterElement>},
    // --- N ---
    {"nav", CreateNav},
    // --- O ---
    {"ol", Create<blink::HTMLOListElement>},
    {"optgroup", Create<blink::HTMLOptGroupElement>},
    {"option", Create<blink::HTMLOptionElement>},
    {"output", Create<blink::HTMLOutputElement>},
    // --- P ---
    {"p", Create<blink::HTMLParagraphElement>},
    {"path", Create<blink::SVGPathElement>},
    {"pattern", Create<blink::SVGPatternElement>},
    {"picture", Create<blink::HTMLPictureElement>},
    {"polygon", Create<blink::SVGPolygonElement>},
    {"polyline", Create<blink::SVGPolylineElement>},
    {"pre", CreatePre},
    {"progress", Create<blink::HTMLProgressElement>},
    // --- Q ---
    {"q", CreateQ},
    // --- R ---
    {"radialGradient", Create<blink::SVGRadialGradientElement>},
    {"rect", Create<blink::SVGRectElement>},
    // --- S ---
    {"s", CreateS},
    {"samp", CreateSamp},
    {"section", CreateSection},
    {"select", Create<blink::HTMLSelectElement>},
    {"small", CreateSmall},
    {"source", Create<blink::HTMLSourceElement>},
    {"span", Create<blink::HTMLSpanElement>},
    {"stop", Create<blink::SVGStopElement>},
    {"strong", CreateStrong},
    {"sub", CreateSub},
    {"summary", Create<blink::HTMLSummaryElement>},
    {"sup", CreateSup},
    {"svg", Create<blink::SVGSVGElement>},
    {"symbol", Create<blink::SVGSymbolElement>},
    // --- T ---
    {"table", Create<blink::HTMLTableElement>},
    {"tbody", CreateTbody},
    {"td", CreateTd},
    {"text", Create<blink::SVGTextElement>},
    {"textPath", Create<blink::SVGTextPathElement>},
    {"textarea", Create<blink::HTMLTextAreaElement>},
    {"tfoot", CreateTfoot},
    {"th", CreateTh},
    {"thead", CreateThead},
    {"time", Create<blink::HTMLTimeElement>},
    {"tr", Create<blink::HTMLTableRowElement>},
    {"tspan", Create<blink::SVGTSpanElement>},
    // --- U ---
    {"u", CreateU},
    {"ul", Create<blink::HTMLUListElement>},
    {"use", Create<blink::SVGUseElement>},
    // --- V ---
    {"var", CreateVar},
    {"video", Create<blink::HTMLVideoElement>},
    // --- W ---
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
