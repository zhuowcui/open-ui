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

// ═══════════════════════════════════════════════════════════════════════════
// C API types
// ═══════════════════════════════════════════════════════════════════════════

#ifdef __cplusplus
extern "C" {
#endif

// Callback to free resource response data when Blink is done with it.
typedef void (*OuiResourceFreeFunc)(uint8_t* data, void* user_data);

// Response data returned by the resource provider callback.
typedef struct {
  // Pointer to the resource data (image bytes, CSS text, etc.).
  // Ownership: the provider retains ownership until free_func is called.
  __attribute__((annotate("raw_ptr_exclusion")))
  uint8_t* data;
  size_t length;

  // MIME type hint (e.g. "image/png"). NULL = auto-detect from data.
  __attribute__((annotate("raw_ptr_exclusion")))
  const char* mime_type;

  // Called when Blink is done with |data|. NULL = Blink won't free.
  OuiResourceFreeFunc free_func;
  __attribute__((annotate("raw_ptr_exclusion")))
  void* free_user_data;
} OuiResourceResponse;

// Resource provider callback type.
// Called by Blink when it needs to fetch a URL.
// |url| — the URL being requested (e.g. "asset://logo.png").
// |response| — out-param: fill in data/length/mime_type on success.
// |user_data| — the user_data passed to oui_document_set_resource_provider.
// Return 1 if the resource was found (response filled), 0 if not found.
typedef int (*OuiResourceProviderFunc)(
    const char* url,
    OuiResourceResponse* response,
    void* user_data);

// ═══════════════════════════════════════════════════════════════════════════
// Resource provider API
// ═══════════════════════════════════════════════════════════════════════════

// Set the resource provider callback for a document. The callback will be
// invoked on the main thread whenever Blink's resource loader needs to fetch
// a URL. Only one provider per document; setting a new one replaces the old.
//
// NOTE: The resource provider must be set BEFORE loading HTML that references
// images. The provider is installed by passing a custom LocalFrameClient to
// DummyPageHolder, so oui_document_create must be updated to accept this.
// For now this function stores the callback on the document impl so it can
// be wired in during document creation.
//
// Returns OUI_OK on success.
OUI_EXPORT OuiStatus oui_document_set_resource_provider(
    OuiDocument* doc,
    OuiResourceProviderFunc provider,
    void* user_data);

// ═══════════════════════════════════════════════════════════════════════════
// Direct image injection
// ═══════════════════════════════════════════════════════════════════════════

// Set raw RGBA pixel data on an <img> element. The element must have been
// created with tag "img". The pixels are copied — the caller can free
// rgba_pixels after this call returns. The image will have the given
// dimensions. Stride is assumed to be width * 4.
OUI_EXPORT OuiStatus oui_element_set_image_data(
    OuiElement* elem,
    const uint8_t* rgba_pixels,
    int width,
    int height);

// Set encoded image data (PNG, JPEG, WebP, GIF, etc.) on an <img> element.
// The data is copied. Blink's image decoder will decode it.
OUI_EXPORT OuiStatus oui_element_set_image_encoded(
    OuiElement* elem,
    const uint8_t* data,
    size_t length);

#ifdef __cplusplus
}  // extern "C"
#endif

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
