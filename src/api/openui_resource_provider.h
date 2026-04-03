// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui_resource_provider.h — Resource provider callback and direct image
// injection APIs. Provides two image-loading mechanisms:
//
//   A) Resource Provider Callback — user-provided callback invoked by Blink's
//      resource loader whenever it needs to fetch a URL (e.g., <img src>,
//      CSS background-image, SVG <image>).
//
//   B) Direct Image Injection — user pushes raw RGBA pixels or encoded image
//      bytes directly onto an element.
//
// Usage:
//   These functions are designed to be called from the C API layer. They
//   operate on OuiDocumentImpl / OuiElementImpl which back the opaque C
//   handles OuiDocument* / OuiElement*. Integration into openui.h and
//   openui_impl.cc is done separately; this file is self-contained.

#ifndef OPENUI_OPENUI_RESOURCE_PROVIDER_H_
#define OPENUI_OPENUI_RESOURCE_PROVIDER_H_

#include <stddef.h>
#include <stdint.h>

#include "openui/openui.h"

// C API types (OuiResourceResponse, OuiResourceProviderFunc, etc.)
// and function declarations are in openui.h.  This header extends them
// with the C++ internals used by openui_resource_provider.cc.

// ═══════════════════════════════════════════════════════════════════════════
// C++ internals (only visible to .cc files that #include this header)
// ═══════════════════════════════════════════════════════════════════════════
#ifdef __cplusplus

#include <memory>
#include <string>

#include "base/memory/raw_ptr.h"
#include "base/memory/scoped_refptr.h"
#include "third_party/blink/renderer/core/loader/empty_clients.h"
#include "third_party/blink/renderer/platform/loader/fetch/url_loader/url_loader.h"

struct OuiDocumentImpl;

namespace openui {

// Stored on OuiDocumentImpl to hold the user's resource provider callback.
struct ResourceProviderState {
  OuiResourceProviderFunc callback = nullptr;
  __attribute__((annotate("raw_ptr_exclusion")))
  void* user_data = nullptr;
};

// Custom URLLoader that intercepts resource requests and calls the user's
// resource provider callback. If the callback returns data, we deliver it
// directly to the URLLoaderClient. If it returns 0 (not found), we signal
// a 404 error.
class ResourceProviderURLLoader : public blink::URLLoader {
 public:
  explicit ResourceProviderURLLoader(ResourceProviderState* state);
  ~ResourceProviderURLLoader() override;

  // URLLoader overrides:
  void LoadSynchronously(
      std::unique_ptr<network::ResourceRequest> request,
      scoped_refptr<const blink::SecurityOrigin> top_frame_origin,
      bool download_to_blob,
      bool no_mime_sniffing,
      base::TimeDelta timeout_interval,
      blink::URLLoaderClient* client,
      blink::WebURLResponse& response,
      std::optional<blink::WebURLError>& error,
      scoped_refptr<blink::SharedBuffer>& data,
      int64_t& encoded_data_length,
      uint64_t& encoded_body_length,
      scoped_refptr<blink::BlobDataHandle>& downloaded_blob,
      std::unique_ptr<blink::ResourceLoadInfoNotifierWrapper>
          resource_load_info_notifier_wrapper) override;

  void LoadAsynchronously(
      std::unique_ptr<network::ResourceRequest> request,
      scoped_refptr<const blink::SecurityOrigin> top_frame_origin,
      bool no_mime_sniffing,
      std::unique_ptr<blink::ResourceLoadInfoNotifierWrapper>
          resource_load_info_notifier_wrapper,
      blink::CodeCacheHost* code_cache_host,
      blink::URLLoaderClient* client) override;

  void Freeze(blink::LoaderFreezeMode mode) override;
  void DidChangePriority(blink::WebURLRequest::Priority new_priority,
                         int intra_priority_value) override;
  scoped_refptr<base::SingleThreadTaskRunner> GetTaskRunnerForBodyLoader()
      override;

 private:
  raw_ptr<ResourceProviderState> state_;
  raw_ptr<blink::URLLoaderClient> client_ = nullptr;
};

// Custom LocalFrameClient that returns our ResourceProviderURLLoader.
// This replaces DummyPageHolder's default DummyLocalFrameClient when a
// resource provider is installed.
class ResourceProviderFrameClient : public blink::EmptyLocalFrameClient {
 public:
  explicit ResourceProviderFrameClient(ResourceProviderState* state);

 private:
  std::unique_ptr<blink::URLLoader> CreateURLLoaderForTesting() override;
  raw_ptr<ResourceProviderState> state_;
};

}  // namespace openui

#endif  // __cplusplus

#endif  // OPENUI_OPENUI_RESOURCE_PROVIDER_H_
