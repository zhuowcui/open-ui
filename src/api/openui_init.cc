// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui_init.cc — Blink runtime bootstrap.
// Extracts the initialization sequence from rendering_test.cc main() into
// reusable init/shutdown functions for the shared library.

#include "openui/openui_init.h"

#include <atomic>

#include "base/at_exit.h"
#include "base/base_paths.h"
#include "base/command_line.h"
#include "base/feature_list.h"
#include "base/files/file_path.h"
#include "base/memory/discardable_memory_allocator.h"
#include "base/path_service.h"
#include "base/task/single_thread_task_runner.h"
#include "base/test/icu_test_util.h"
#include "base/test/null_task_runner.h"
#include "base/test/test_discardable_memory_allocator.h"
#include "base/test/test_timeouts.h"
#include "gin/v8_initializer.h"
#include "mojo/core/embedder/embedder.h"
#include "mojo/public/cpp/bindings/binder_map.h"
#include "ui/base/resource/resource_bundle.h"
#include "v8/include/v8.h"

#include "third_party/blink/public/platform/platform.h"
#include "third_party/blink/public/platform/scheduler/test/renderer_scheduler_test_support.h"
#include "third_party/blink/public/platform/scheduler/web_thread_scheduler.h"
#include "third_party/blink/public/platform/web_runtime_features.h"
#include "third_party/blink/public/web/blink.h"
#include "third_party/blink/renderer/platform/testing/task_environment.h"
#include "third_party/blink/renderer/platform/wtf/text/wtf_string.h"

namespace {

// Platform subclass: routes resource loading to ui::ResourceBundle.
class OpenUIPlatform : public blink::Platform {
 public:
  blink::WebString DefaultLocale() override {
    return blink::WebString::FromUTF8("en-US");
  }

  std::string GetDataResourceString(int resource_id) override {
    if (ui::ResourceBundle::HasSharedInstance()) {
      return ui::ResourceBundle::GetSharedInstance().LoadDataResourceString(
          resource_id);
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

std::atomic<bool> g_initialized{false};
bool g_has_external_task_env{false};  // True when running under a test harness.

// These are leaked intentionally — must survive until process exit.
base::AtExitManager* g_at_exit = nullptr;
base::TestDiscardableMemoryAllocator* g_discardable_allocator = nullptr;
std::unique_ptr<blink::scheduler::WebThreadScheduler>* g_scheduler = nullptr;
OpenUIPlatform* g_platform = nullptr;
blink::test::TaskEnvironment* g_task_env = nullptr;

}  // namespace

OuiStatus openui_runtime_init(const OuiInitConfig* config) {
  if (g_initialized.exchange(true)) {
    return OUI_ERROR_ALREADY_INITIALIZED;
  }

  // Phase 1: base infrastructure.
  // Skip AtExitManager if a test harness already created one (indicated by
  // CommandLine already being initialized).
  if (!base::CommandLine::InitializedForCurrentProcess()) {
    g_at_exit = new base::AtExitManager();
    base::CommandLine::Init(0, nullptr);
  }

  base::test::InitializeICUForTesting();

  g_discardable_allocator = new base::TestDiscardableMemoryAllocator();
  base::DiscardableMemoryAllocator::SetInstance(g_discardable_allocator);

  if (!base::FeatureList::GetInstance()) {
    auto feature_list = std::make_unique<base::FeatureList>();
    feature_list->InitFromCommandLine(
        base::CommandLine::ForCurrentProcess()->GetSwitchValueASCII(
            "enable-features"),
        base::CommandLine::ForCurrentProcess()->GetSwitchValueASCII(
            "disable-features"));
    base::FeatureList::SetInstance(std::move(feature_list));
  }

  // Phase 2: resource bundle.
  {
    base::FilePath pak_path;
    if (config && config->resource_pak_path) {
      pak_path = base::FilePath::FromUTF8Unsafe(config->resource_pak_path);
    } else {
      base::PathService::Get(base::DIR_ASSETS, &pak_path);
      pak_path = pak_path.Append(FILE_PATH_LITERAL("content_shell.pak"));
    }
    ui::ResourceBundle::InitSharedInstanceWithPakPath(pak_path);
  }

  // Phase 3: mojo + V8 snapshot.
  mojo::core::Init();

#if defined(V8_USE_EXTERNAL_STARTUP_DATA)
  gin::V8Initializer::LoadV8Snapshot();
#endif

  // Phase 4: blink platform init.
  blink::Platform::InitializeBlink();

  g_scheduler = new std::unique_ptr<blink::scheduler::WebThreadScheduler>(
      blink::scheduler::CreateWebMainThreadSchedulerForTests());

  const char kV8Flags[] = "--expose-gc --no-freeze-flags-after-init";
  v8::V8::SetFlagsFromString(kV8Flags, sizeof(kV8Flags) - 1);

  g_platform = new OpenUIPlatform();

  {
    auto dummy_task_runner = base::MakeRefCounted<base::NullTaskRunner>();
    base::SingleThreadTaskRunner::CurrentDefaultHandle dummy_handle(
        dummy_task_runner);

    mojo::BinderMap binders;
    blink::InitializeWithoutIsolateForTesting(g_platform, &binders,
                                              g_scheduler->get());
  }

  // Phase 5: enable experimental features.
  blink::WebRuntimeFeatures::EnableExperimentalFeatures(true);
  blink::WebRuntimeFeatures::EnableTestOnlyFeatures(true);

  // Phase 6: Create a global TaskEnvironment for standalone mode.
  // oui_init() always owns the task environment. The only case where we
  // skip this is when the caller uses openui_runtime_mark_initialized_externally()
  // (test harnesses that manage their own TaskEnvironment per-test).
  TestTimeouts::Initialize();
  g_task_env = new blink::test::TaskEnvironment();
  g_has_external_task_env = true;  // Documents should not create their own.

  return OUI_OK;
}

void openui_runtime_shutdown() {
  // Shutdown is terminal — blink and V8 do not support clean re-initialization
  // within a single process. After this call, oui_init() will return
  // OUI_ERROR_ALREADY_INITIALIZED on any subsequent attempt.
  // We clean up what we can; remaining singletons are process-lifetime.
  if (g_task_env) {
    delete g_task_env;
    g_task_env = nullptr;
  }
  // Note: g_initialized stays true to prevent re-init attempts.
}

bool openui_runtime_is_initialized() {
  return g_initialized.load();
}

bool openui_runtime_has_external_task_env() {
  return g_has_external_task_env;
}

void openui_runtime_mark_initialized_externally() {
  g_initialized.store(true);
  g_has_external_task_env = true;
}
