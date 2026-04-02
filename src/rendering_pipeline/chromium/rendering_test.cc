// OpenUI rendering pipeline tests — exercises Chromium's
// style → layout pipeline via blink::DummyPageHolder.

#include "base/command_line.h"
#include "base/containers/span.h"
#include "base/feature_list.h"
#include "base/files/file_path.h"
#include "base/functional/bind.h"
#include "base/path_service.h"
#include "base/test/icu_test_util.h"
#include "base/memory/discardable_memory_allocator.h"
#include "base/test/launcher/unit_test_launcher.h"
#include "base/test/null_task_runner.h"
#include "base/test/test_discardable_memory_allocator.h"
#include "base/test/test_io_thread.h"
#include "base/test/test_suite.h"
#include "base/task/single_thread_task_runner.h"
#include "gin/v8_initializer.h"
#include "mojo/core/embedder/embedder.h"
#include "mojo/core/embedder/scoped_ipc_support.h"
#include "mojo/public/cpp/bindings/binder_map.h"
#include "ui/base/resource/resource_bundle.h"
#include "v8/include/v8.h"

#include "third_party/blink/public/platform/platform.h"
#include "third_party/blink/public/platform/web_runtime_features.h"
#include "third_party/blink/public/platform/scheduler/test/renderer_scheduler_test_support.h"
#include "third_party/blink/public/platform/scheduler/web_thread_scheduler.h"
#include "third_party/blink/public/web/blink.h"
#include "third_party/blink/renderer/platform/testing/task_environment.h"

#include "third_party/blink/renderer/core/testing/dummy_page_holder.h"
#include "third_party/blink/renderer/core/dom/document.h"
#include "third_party/blink/renderer/core/dom/element.h"
#include "third_party/blink/renderer/core/dom/text.h"
#include "third_party/blink/renderer/core/html/html_div_element.h"
#include "third_party/blink/renderer/core/html/html_body_element.h"
#include "third_party/blink/renderer/core/frame/local_frame_view.h"
#include "third_party/blink/renderer/core/layout/layout_object.h"
#include "third_party/blink/renderer/core/layout/layout_box.h"
#include "third_party/blink/renderer/core/style/computed_style.h"
#include "third_party/blink/renderer/platform/graphics/paint/paint_artifact.h"
#include "third_party/blink/renderer/platform/wtf/text/atomic_string.h"

#include "testing/gtest/include/gtest/gtest.h"
#include "ui/gfx/geometry/size.h"

// ---------------------------------------------------------------------------
// Platform subclass: routes resource loading to ui::ResourceBundle and
// provides a valid locale for language initialization.
// ---------------------------------------------------------------------------
class OpenUIPlatform : public blink::Platform {
 public:
  blink::WebString DefaultLocale() override {
    return blink::WebString::FromUTF8("en-US");
  }

  std::string GetDataResourceString(int resource_id) override {
    if (ui::ResourceBundle::HasSharedInstance()) {
      return ui::ResourceBundle::GetSharedInstance()
          .LoadDataResourceString(resource_id);
    }
    return std::string();
  }

  blink::WebData GetDataResource(
      int resource_id,
      ui::ResourceScaleFactor scale_factor) override {
    if (ui::ResourceBundle::HasSharedInstance()) {
      std::string_view data =
          ui::ResourceBundle::GetSharedInstance().GetRawDataResourceForScale(
              resource_id, scale_factor);
      return blink::WebData(base::as_byte_span(data));
    }
    return blink::WebData();
  }

  bool HasDataResource(int resource_id) const override {
    if (ui::ResourceBundle::HasSharedInstance()) {
      return !ui::ResourceBundle::GetSharedInstance()
                  .GetRawDataResource(resource_id)
                  .empty();
    }
    return false;
  }
};

// ---------------------------------------------------------------------------
// Test fixture — each test gets its own blink TaskEnvironment + DummyPage
// ---------------------------------------------------------------------------
class OpenUIRenderingTest : public testing::Test {
 protected:
  void SetUp() override {
    task_environment_ = std::make_unique<blink::test::TaskEnvironment>();
    page_holder_ = std::make_unique<blink::DummyPageHolder>(
        gfx::Size(800, 600));
  }

  void TearDown() override {
    page_holder_.reset();
    task_environment_.reset();
  }

  blink::Document& GetDocument() { return page_holder_->GetDocument(); }

 private:
  std::unique_ptr<blink::test::TaskEnvironment> task_environment_;
  std::unique_ptr<blink::DummyPageHolder> page_holder_;
};

// ---------------------------------------------------------------------------
// Box model: width + padding gives larger offset dimensions
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, BoxModelLayout) {
  auto& doc = GetDocument();
  auto* div = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  div->setAttribute(blink::html_names::kStyleAttr,
                    blink::AtomicString("width: 200px; height: 100px; "
                                        "background-color: red; "
                                        "margin: 10px; padding: 5px;"));
  doc.body()->AppendChild(div);
  doc.UpdateStyleAndLayout(blink::DocumentUpdateReason::kTest);

  auto* layout_obj = div->GetLayoutObject();
  ASSERT_NE(layout_obj, nullptr) << "div has no LayoutObject";
  auto* box = blink::DynamicTo<blink::LayoutBox>(layout_obj);
  ASSERT_NE(box, nullptr) << "LayoutObject is not a LayoutBox";
  EXPECT_EQ(box->OffsetWidth().ToInt(), 210);   // 200 + 2*5 padding
  EXPECT_EQ(box->OffsetHeight().ToInt(), 110);   // 100 + 2*5 padding
}

// ---------------------------------------------------------------------------
// Flexbox: children share available space according to flex ratios
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, FlexboxLayout) {
  auto& doc = GetDocument();
  auto* container = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  container->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString(
          "display: flex; width: 390px; height: 200px; gap: 10px;"));

  auto* c1 = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  c1->setAttribute(blink::html_names::kStyleAttr,
                   blink::AtomicString("flex: 1; background: blue;"));
  auto* c2 = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  c2->setAttribute(blink::html_names::kStyleAttr,
                   blink::AtomicString("flex: 2; background: green;"));

  container->AppendChild(c1);
  container->AppendChild(c2);
  doc.body()->AppendChild(container);
  doc.UpdateStyleAndLayout(blink::DocumentUpdateReason::kTest);

  auto* c1_box = blink::To<blink::LayoutBox>(c1->GetLayoutObject());
  auto* c2_box = blink::To<blink::LayoutBox>(c2->GetLayoutObject());
  ASSERT_NE(c1_box, nullptr);
  ASSERT_NE(c2_box, nullptr);

  int w1 = c1_box->OffsetWidth().ToInt();
  int w2 = c2_box->OffsetWidth().ToInt();
  EXPECT_NEAR(w1 + w2, 380, 1);  // 390 − 10 gap
  double ratio = static_cast<double>(w2) / w1;
  EXPECT_NEAR(ratio, 2.0, 0.1);
}

// ---------------------------------------------------------------------------
// CSS Grid: explicit column tracks
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, CSSGridLayout) {
  auto& doc = GetDocument();
  auto* grid = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  grid->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString("display: grid; width: 300px; "
                          "grid-template-columns: 100px 200px; gap: 0;"));

  auto* g1 = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  g1->setAttribute(blink::html_names::kStyleAttr,
                   blink::AtomicString("height: 50px;"));
  auto* g2 = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  g2->setAttribute(blink::html_names::kStyleAttr,
                   blink::AtomicString("height: 50px;"));

  grid->AppendChild(g1);
  grid->AppendChild(g2);
  doc.body()->AppendChild(grid);
  doc.UpdateStyleAndLayout(blink::DocumentUpdateReason::kTest);

  auto* g1_box = blink::To<blink::LayoutBox>(g1->GetLayoutObject());
  auto* g2_box = blink::To<blink::LayoutBox>(g2->GetLayoutObject());
  ASSERT_NE(g1_box, nullptr);
  ASSERT_NE(g2_box, nullptr);
  EXPECT_EQ(g1_box->OffsetWidth().ToInt(), 100);
  EXPECT_EQ(g2_box->OffsetWidth().ToInt(), 200);
}

// ---------------------------------------------------------------------------
// Text layout: rendered text has non-zero height
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, TextLayout) {
  auto& doc = GetDocument();
  auto* text_div = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  text_div->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString(
          "font-size: 16px; line-height: 24px; width: 300px;"));
  text_div->appendChild(blink::Text::Create(doc, "Hello from OpenUI!"));
  doc.body()->AppendChild(text_div);
  doc.UpdateStyleAndLayout(blink::DocumentUpdateReason::kTest);

  auto* text_box = blink::To<blink::LayoutBox>(text_div->GetLayoutObject());
  ASSERT_NE(text_box, nullptr);
  EXPECT_GT(text_box->OffsetHeight().ToInt(), 0);
}

// ---------------------------------------------------------------------------
// Absolute positioning: child respects top/left offsets
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, AbsolutePositioning) {
  auto& doc = GetDocument();
  auto* outer = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  outer->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString(
          "position: relative; width: 400px; height: 300px;"));

  auto* inner = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  inner->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString("position: absolute; top: 10px; left: 20px; "
                          "width: 100px; height: 50px;"));

  outer->AppendChild(inner);
  doc.body()->AppendChild(outer);
  doc.UpdateStyleAndLayout(blink::DocumentUpdateReason::kTest);

  auto* inner_box = blink::To<blink::LayoutBox>(inner->GetLayoutObject());
  ASSERT_NE(inner_box, nullptr);
  EXPECT_EQ(inner_box->OffsetWidth().ToInt(), 100);
  EXPECT_EQ(inner_box->OffsetHeight().ToInt(), 50);
  EXPECT_EQ(inner_box->OffsetTop(nullptr).ToInt(), 10);
  EXPECT_EQ(inner_box->OffsetLeft(nullptr).ToInt(), 20);
}

// ---------------------------------------------------------------------------
// border-box sizing: padding + border included in declared size
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, BorderBoxSizing) {
  auto& doc = GetDocument();
  auto* div = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  div->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString("box-sizing: border-box; width: 200px; "
                          "height: 100px; padding: 20px; border: 5px solid;"));
  doc.body()->AppendChild(div);
  doc.UpdateStyleAndLayout(blink::DocumentUpdateReason::kTest);

  auto* box = blink::To<blink::LayoutBox>(div->GetLayoutObject());
  ASSERT_NE(box, nullptr);
  EXPECT_EQ(box->OffsetWidth().ToInt(), 200);
  EXPECT_EQ(box->OffsetHeight().ToInt(), 100);
}

// ---------------------------------------------------------------------------
// Computed style: style properties are correctly resolved
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, ComputedStyleAccess) {
  auto& doc = GetDocument();
  auto* div = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  div->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString("color: rgb(255, 0, 0); font-size: 20px;"));
  doc.body()->AppendChild(div);
  doc.UpdateStyleAndLayout(blink::DocumentUpdateReason::kTest);

  const blink::ComputedStyle* style = div->GetComputedStyle();
  ASSERT_NE(style, nullptr);
  EXPECT_EQ(style->FontSize(), 20);
}

// ---------------------------------------------------------------------------
// Nested layout: deeply nested divs accumulate padding/margin
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, NestedLayout) {
  auto& doc = GetDocument();
  auto* outer = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  outer->setAttribute(blink::html_names::kStyleAttr,
                      blink::AtomicString("width: 300px; padding: 10px;"));

  auto* middle = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  middle->setAttribute(blink::html_names::kStyleAttr,
                       blink::AtomicString("padding: 10px;"));

  auto* inner = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  inner->setAttribute(blink::html_names::kStyleAttr,
                      blink::AtomicString("height: 50px;"));

  middle->AppendChild(inner);
  outer->AppendChild(middle);
  doc.body()->AppendChild(outer);
  doc.UpdateStyleAndLayout(blink::DocumentUpdateReason::kTest);

  auto* outer_box = blink::To<blink::LayoutBox>(outer->GetLayoutObject());
  auto* inner_box = blink::To<blink::LayoutBox>(inner->GetLayoutObject());
  ASSERT_NE(outer_box, nullptr);
  ASSERT_NE(inner_box, nullptr);
  // outer: 300 + 2*10 padding = 320
  EXPECT_EQ(outer_box->OffsetWidth().ToInt(), 320);
  // inner width: 300 (outer content) - 2*10 (middle padding) = 280
  EXPECT_EQ(inner_box->OffsetWidth().ToInt(), 280);
  EXPECT_EQ(inner_box->OffsetHeight().ToInt(), 50);
}

// ---------------------------------------------------------------------------
// Overflow hidden: clipped children don't affect parent size
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, OverflowHidden) {
  auto& doc = GetDocument();
  auto* container = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  container->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString(
          "width: 100px; height: 100px; overflow: hidden;"));

  auto* child = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  child->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString("width: 200px; height: 200px;"));

  container->AppendChild(child);
  doc.body()->AppendChild(container);
  doc.UpdateStyleAndLayout(blink::DocumentUpdateReason::kTest);

  auto* container_box =
      blink::To<blink::LayoutBox>(container->GetLayoutObject());
  ASSERT_NE(container_box, nullptr);
  EXPECT_EQ(container_box->OffsetWidth().ToInt(), 100);
  EXPECT_EQ(container_box->OffsetHeight().ToInt(), 100);
}

// ---------------------------------------------------------------------------
// Min/max constraints: min-width overrides width when larger
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, MinMaxConstraints) {
  auto& doc = GetDocument();
  auto* div = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  div->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString(
          "width: 50px; min-width: 100px; height: 30px; max-height: 20px;"));
  doc.body()->AppendChild(div);
  doc.UpdateStyleAndLayout(blink::DocumentUpdateReason::kTest);

  auto* box = blink::To<blink::LayoutBox>(div->GetLayoutObject());
  ASSERT_NE(box, nullptr);
  EXPECT_EQ(box->OffsetWidth().ToInt(), 100);   // min-width wins
  EXPECT_EQ(box->OffsetHeight().ToInt(), 20);   // max-height wins
}

// ---------------------------------------------------------------------------
// Inline-block elements side by side
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, InlineBlockLayout) {
  auto& doc = GetDocument();
  auto* wrapper = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  wrapper->setAttribute(blink::html_names::kStyleAttr,
                        blink::AtomicString("width: 400px; font-size: 0;"));

  auto* a = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  a->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString(
          "display: inline-block; width: 150px; height: 50px;"));
  auto* b = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  b->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString(
          "display: inline-block; width: 200px; height: 50px;"));

  wrapper->AppendChild(a);
  wrapper->AppendChild(b);
  doc.body()->AppendChild(wrapper);
  doc.UpdateStyleAndLayout(blink::DocumentUpdateReason::kTest);

  auto* a_box = blink::To<blink::LayoutBox>(a->GetLayoutObject());
  auto* b_box = blink::To<blink::LayoutBox>(b->GetLayoutObject());
  ASSERT_NE(a_box, nullptr);
  ASSERT_NE(b_box, nullptr);
  // Both fit on one line (150 + 200 = 350 < 400)
  EXPECT_EQ(a_box->OffsetWidth().ToInt(), 150);
  EXPECT_EQ(b_box->OffsetWidth().ToInt(), 200);
}

// ---------------------------------------------------------------------------
// Percentage-based sizing
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, PercentageSizing) {
  auto& doc = GetDocument();
  auto* parent = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  parent->setAttribute(blink::html_names::kStyleAttr,
                       blink::AtomicString("width: 400px; height: 200px;"));

  auto* child = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  child->setAttribute(blink::html_names::kStyleAttr,
                      blink::AtomicString("width: 50%; height: 25%;"));

  parent->AppendChild(child);
  doc.body()->AppendChild(parent);
  doc.UpdateStyleAndLayout(blink::DocumentUpdateReason::kTest);

  auto* child_box = blink::To<blink::LayoutBox>(child->GetLayoutObject());
  ASSERT_NE(child_box, nullptr);
  EXPECT_EQ(child_box->OffsetWidth().ToInt(), 200);
  EXPECT_EQ(child_box->OffsetHeight().ToInt(), 50);
}

// ---------------------------------------------------------------------------
// Flex wrap: items wrap to next line when they exceed container width
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, FlexWrap) {
  auto& doc = GetDocument();
  auto* flex = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  flex->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString(
          "display: flex; flex-wrap: wrap; width: 200px;"));

  auto* c1 = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  c1->setAttribute(blink::html_names::kStyleAttr,
                   blink::AtomicString("width: 150px; height: 40px;"));
  auto* c2 = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  c2->setAttribute(blink::html_names::kStyleAttr,
                   blink::AtomicString("width: 150px; height: 40px;"));

  flex->AppendChild(c1);
  flex->AppendChild(c2);
  doc.body()->AppendChild(flex);
  doc.UpdateStyleAndLayout(blink::DocumentUpdateReason::kTest);

  auto* flex_box = blink::To<blink::LayoutBox>(flex->GetLayoutObject());
  ASSERT_NE(flex_box, nullptr);
  // Two items each 150px wide in a 200px container with wrap:
  // they should stack vertically, so container height >= 80
  EXPECT_GE(flex_box->OffsetHeight().ToInt(), 80);
}

// ---------------------------------------------------------------------------
// CSS transforms: transform doesn't change layout size
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, CSSTransform) {
  auto& doc = GetDocument();
  auto* div = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  div->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString("width: 100px; height: 100px; "
                          "transform: scale(2);"));
  doc.body()->AppendChild(div);
  doc.UpdateStyleAndLayout(blink::DocumentUpdateReason::kTest);

  auto* box = blink::To<blink::LayoutBox>(div->GetLayoutObject());
  ASSERT_NE(box, nullptr);
  // Transform doesn't affect layout dimensions
  EXPECT_EQ(box->OffsetWidth().ToInt(), 100);
  EXPECT_EQ(box->OffsetHeight().ToInt(), 100);
}

// ---------------------------------------------------------------------------
// Z-index + stacking: elements with z-index create stacking contexts
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, ZIndexStacking) {
  auto& doc = GetDocument();
  auto* parent = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  parent->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString("position: relative; width: 300px; height: 300px;"));

  auto* back = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  back->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString("position: absolute; z-index: 1; "
                          "width: 100px; height: 100px;"));

  auto* front = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  front->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString("position: absolute; z-index: 2; "
                          "width: 50px; height: 50px;"));

  parent->AppendChild(back);
  parent->AppendChild(front);
  doc.body()->AppendChild(parent);
  doc.UpdateStyleAndLayout(blink::DocumentUpdateReason::kTest);

  auto* back_box = blink::To<blink::LayoutBox>(back->GetLayoutObject());
  auto* front_box = blink::To<blink::LayoutBox>(front->GetLayoutObject());
  ASSERT_NE(back_box, nullptr);
  ASSERT_NE(front_box, nullptr);

  const auto* back_style = back->GetComputedStyle();
  const auto* front_style = front->GetComputedStyle();
  ASSERT_NE(back_style, nullptr);
  ASSERT_NE(front_style, nullptr);
  EXPECT_EQ(back_style->ZIndex(), 1);
  EXPECT_EQ(front_style->ZIndex(), 2);
}

// ---------------------------------------------------------------------------
// Multi-column layout
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, MultiColumnLayout) {
  auto& doc = GetDocument();
  auto* mcol = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  mcol->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString("width: 300px; column-count: 3; column-gap: 0;"));

  auto* child = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  child->setAttribute(blink::html_names::kStyleAttr,
                      blink::AtomicString("height: 50px;"));
  mcol->AppendChild(child);
  doc.body()->AppendChild(mcol);
  doc.UpdateStyleAndLayout(blink::DocumentUpdateReason::kTest);

  auto* mcol_box = blink::To<blink::LayoutBox>(mcol->GetLayoutObject());
  ASSERT_NE(mcol_box, nullptr);
  EXPECT_EQ(mcol_box->OffsetWidth().ToInt(), 300);
}

// ---------------------------------------------------------------------------
// Table layout: basic table with rows and cells
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, TableLayout) {
  auto& doc = GetDocument();
  auto* table = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  table->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString("display: table; width: 300px;"));

  auto* row = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  row->setAttribute(blink::html_names::kStyleAttr,
                    blink::AtomicString("display: table-row;"));

  auto* cell1 = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  cell1->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString("display: table-cell; width: 100px; height: 40px;"));
  auto* cell2 = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  cell2->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString("display: table-cell; width: 200px; height: 40px;"));

  row->AppendChild(cell1);
  row->AppendChild(cell2);
  table->AppendChild(row);
  doc.body()->AppendChild(table);
  doc.UpdateStyleAndLayout(blink::DocumentUpdateReason::kTest);

  auto* table_box = blink::To<blink::LayoutBox>(table->GetLayoutObject());
  ASSERT_NE(table_box, nullptr);
  EXPECT_EQ(table_box->OffsetWidth().ToInt(), 300);
}

// ---------------------------------------------------------------------------
// UA stylesheet: div is block by default (no explicit display: block)
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, UAStylesheetDefaults) {
  auto& doc = GetDocument();
  auto* div = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  div->setAttribute(blink::html_names::kStyleAttr,
                    blink::AtomicString("width: 100px; height: 50px;"));
  doc.body()->AppendChild(div);
  doc.UpdateStyleAndLayout(blink::DocumentUpdateReason::kTest);

  auto* layout_obj = div->GetLayoutObject();
  ASSERT_NE(layout_obj, nullptr);
  EXPECT_TRUE(layout_obj->IsBox());
  EXPECT_TRUE(layout_obj->IsLayoutBlock());
  EXPECT_FALSE(layout_obj->IsInline());

  auto* box = blink::To<blink::LayoutBox>(layout_obj);
  EXPECT_EQ(box->OffsetWidth().ToInt(), 100);
  EXPECT_EQ(box->OffsetHeight().ToInt(), 50);
}

// ---------------------------------------------------------------------------
// Full lifecycle: style → layout → paint completes without crash
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, FullPaintLifecycle) {
  auto& doc = GetDocument();
  auto* div = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  div->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString("width: 200px; height: 100px; "
                          "background-color: blue; border: 2px solid red;"));
  doc.body()->AppendChild(div);

  // Advance through ALL lifecycle phases including paint.
  doc.View()->UpdateAllLifecyclePhasesForTest();

  auto* box = blink::To<blink::LayoutBox>(div->GetLayoutObject());
  ASSERT_NE(box, nullptr);
  EXPECT_EQ(box->OffsetWidth().ToInt(), 204);  // 200 + 2*2 border
  EXPECT_EQ(box->OffsetHeight().ToInt(), 104); // 100 + 2*2 border
}

// ---------------------------------------------------------------------------
// Paint artifact exists after full lifecycle
// ---------------------------------------------------------------------------
TEST_F(OpenUIRenderingTest, PaintArtifactGenerated) {
  auto& doc = GetDocument();
  auto* div = blink::MakeGarbageCollected<blink::HTMLDivElement>(doc);
  div->setAttribute(
      blink::html_names::kStyleAttr,
      blink::AtomicString("width: 100px; height: 100px; background: green;"));
  doc.body()->AppendChild(div);

  doc.View()->UpdateAllLifecyclePhasesForTest();

  // After full paint lifecycle, the paint artifact should exist.
  const auto& artifact = doc.View()->GetPaintArtifact();
  EXPECT_FALSE(artifact.IsEmpty());
}

// ===========================================================================
// Blink environment — matches the initialization sequence used by the real
// blink_unittests (content::BlinkTestEnvironment + TestBlinkWebUnitTestSupport)
//
// Key insight: InitializeWithoutIsolateForTesting() calls both
// Platform::InitializeMainThread() (sets up scheduler + main thread) and
// InitializeCommon() (initializes core blink modules + V8 IsolateHolder).
// This is the ONLY correct init path for tests that use
// blink::test::TaskEnvironment (which creates a V8 isolate per test).
// ===========================================================================

namespace {

// Leaked intentionally — must survive until process exit; Chromium's
// -Wexit-time-destructors forbids non-trivial static destructors.
base::TestDiscardableMemoryAllocator* g_discardable_allocator = nullptr;
std::unique_ptr<blink::scheduler::WebThreadScheduler>* g_scheduler = nullptr;

}  // namespace

int main(int argc, char** argv) {
  // TestSuite constructor creates AtExitManager + CommandLine::Init.
  // Must be created first since ResourceBundle needs AtExitManager.
  base::TestSuite test_suite(argc, argv);

  // --- Phase 1: base infrastructure ---
  base::test::InitializeICUForTesting();

  g_discardable_allocator = new base::TestDiscardableMemoryAllocator();
  base::DiscardableMemoryAllocator::SetInstance(g_discardable_allocator);

  // Initialize FeatureList from command-line flags so that feature-gated
  // code in blink init observes --enable-features/--disable-features.
  {
    auto feature_list = std::make_unique<base::FeatureList>();
    feature_list->InitFromCommandLine(
        base::CommandLine::ForCurrentProcess()->GetSwitchValueASCII("enable-features"),
        base::CommandLine::ForCurrentProcess()->GetSwitchValueASCII("disable-features"));
    base::FeatureList::SetInstance(std::move(feature_list));
  }

  // --- Phase 2: resource bundle (must precede blink init) ---
  // Load content_shell.pak which contains the user-agent stylesheet
  // (IDR_UASTYLE_HTML_CSS) and other blink resources.
  {
    base::FilePath pak_path;
    base::PathService::Get(base::DIR_ASSETS, &pak_path);
    pak_path = pak_path.Append(FILE_PATH_LITERAL("content_shell.pak"));
    ui::ResourceBundle::InitSharedInstanceWithPakPath(pak_path);
  }

  // --- Phase 3: mojo + V8 snapshot (must precede blink init) ---
  mojo::core::Init();

#if defined(V8_USE_EXTERNAL_STARTUP_DATA)
  gin::V8Initializer::LoadV8Snapshot();
#endif

  // --- Phase 4: blink platform init ---
  blink::Platform::InitializeBlink();

  g_scheduler = new std::unique_ptr<blink::scheduler::WebThreadScheduler>(
      blink::scheduler::CreateWebMainThreadSchedulerForTests());

  const char kV8Flags[] = "--expose-gc --no-freeze-flags-after-init";
  v8::V8::SetFlagsFromString(kV8Flags, sizeof(kV8Flags) - 1);

  // Use our Platform subclass that provides resource loading + locale.
  auto* platform = new OpenUIPlatform();

  {
    auto dummy_task_runner = base::MakeRefCounted<base::NullTaskRunner>();
    base::SingleThreadTaskRunner::CurrentDefaultHandle dummy_handle(
        dummy_task_runner);

    mojo::BinderMap binders;
    blink::InitializeWithoutIsolateForTesting(
        platform, &binders, g_scheduler->get());
  }

  // --- Phase 5: enable test features ---
  blink::WebRuntimeFeatures::EnableExperimentalFeatures(true);
  blink::WebRuntimeFeatures::EnableTestOnlyFeatures(true);

  // --- Phase 6: run tests ---
  base::TestIOThread test_io_thread(base::TestIOThread::kAutoStart);
  mojo::core::ScopedIPCSupport ipc_support(
      test_io_thread.task_runner(),
      mojo::core::ScopedIPCSupport::ShutdownPolicy::CLEAN);

  return base::LaunchUnitTests(
      argc, argv,
      base::BindOnce(&base::TestSuite::Run,
                     base::Unretained(&test_suite)));
}
