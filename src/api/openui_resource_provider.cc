// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui_resource_provider.cc — Implementation of resource provider callback
// and direct image injection APIs.

#include "openui/openui_resource_provider.h"
#include "openui/openui_impl.h"

#include <cstring>
#include <string>

#include "base/task/single_thread_task_runner.h"
#include "base/time/time.h"
#include "mojo/public/cpp/system/data_pipe.h"
#include "net/base/net_errors.h"
#include "services/network/public/cpp/resource_request.h"
#include "third_party/blink/public/platform/web_url.h"
#include "third_party/blink/public/platform/web_url_error.h"
#include "third_party/blink/public/platform/web_url_response.h"
#include "third_party/blink/renderer/core/html/html_image_element.h"
#include "third_party/blink/renderer/core/loader/resource/image_resource_content.h"
#include "third_party/blink/renderer/platform/graphics/bitmap_image.h"
#include "third_party/blink/renderer/platform/graphics/image.h"
#include "third_party/blink/renderer/platform/graphics/unaccelerated_static_bitmap_image.h"
#include "third_party/blink/renderer/platform/heap/garbage_collected.h"
#include "third_party/blink/renderer/platform/loader/fetch/url_loader/url_loader_client.h"
#include "third_party/blink/renderer/platform/scheduler/test/fake_task_runner.h"
#include "third_party/blink/renderer/platform/weborigin/kurl.h"
#include "third_party/blink/renderer/platform/wtf/shared_buffer.h"
#include "third_party/skia/include/core/SkData.h"
#include "third_party/skia/include/core/SkImage.h"
#include "third_party/skia/include/core/SkImageInfo.h"
#include "third_party/skia/include/core/SkPixmap.h"

namespace openui {

// ═══════════════════════════════════════════════════════════════════════════
// ResourceProviderURLLoader
// ═══════════════════════════════════════════════════════════════════════════

ResourceProviderURLLoader::ResourceProviderURLLoader(
    ResourceProviderState* state)
    : state_(state) {}

ResourceProviderURLLoader::~ResourceProviderURLLoader() = default;

void ResourceProviderURLLoader::LoadSynchronously(
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
        resource_load_info_notifier_wrapper) {
  if (!state_ || !state_->callback) {
    error = blink::WebURLError(net::ERR_FAILED,
                               blink::WebURL(blink::KURL(request->url)));
    return;
  }

  std::string url_string = request->url.spec();
  OuiResourceResponse res = {};

  int found = state_->callback(url_string.c_str(), &res, state_->user_data);
  if (!found || !res.data || res.length == 0) {
    error = blink::WebURLError(net::ERR_FILE_NOT_FOUND,
                               blink::WebURL(blink::KURL(request->url)));
    // Free response data if the provider set it despite returning 0.
    if (res.free_func && res.data) {
      res.free_func(res.data, res.free_user_data);
    }
    return;
  }

  // Build the response.
  blink::WebURL web_url(blink::KURL(request->url));
  response = blink::WebURLResponse(web_url);
  response.SetHttpStatusCode(200);
  response.SetHttpStatusText(blink::WebString::FromLatin1("OK"));
  response.SetExpectedContentLength(static_cast<int64_t>(res.length));

  if (res.mime_type) {
    response.SetMimeType(blink::WebString::FromUTF8(res.mime_type));
  } else {
    // Auto-detect common image formats by magic bytes.
    response.SetMimeType(
        blink::WebString::FromLatin1("application/octet-stream"));
    if (res.length >= 8) {
      // PNG: 89 50 4E 47
      if (res.data[0] == 0x89 && res.data[1] == 0x50 &&
          res.data[2] == 0x4E && res.data[3] == 0x47) {
        response.SetMimeType(blink::WebString::FromLatin1("image/png"));
      }
      // JPEG: FF D8 FF
      else if (res.data[0] == 0xFF && res.data[1] == 0xD8 &&
               res.data[2] == 0xFF) {
        response.SetMimeType(blink::WebString::FromLatin1("image/jpeg"));
      }
      // GIF: 47 49 46 38
      else if (res.data[0] == 0x47 && res.data[1] == 0x49 &&
               res.data[2] == 0x46 && res.data[3] == 0x38) {
        response.SetMimeType(blink::WebString::FromLatin1("image/gif"));
      }
      // WebP: RIFF....WEBP
      else if (res.data[0] == 0x52 && res.data[1] == 0x49 &&
               res.data[2] == 0x46 && res.data[3] == 0x46 &&
               res.length >= 12 &&
               res.data[8] == 0x57 && res.data[9] == 0x45 &&
               res.data[10] == 0x42 && res.data[11] == 0x50) {
        response.SetMimeType(blink::WebString::FromLatin1("image/webp"));
      }
    }
  }

  // Copy resource data into a SharedBuffer.
  data = blink::SharedBuffer::Create(
      base::span<const char>(reinterpret_cast<const char*>(res.data),
                             res.length));
  encoded_data_length = static_cast<int64_t>(res.length);
  encoded_body_length = res.length;

  // Free the provider's data.
  if (res.free_func) {
    res.free_func(res.data, res.free_user_data);
  }
}

void ResourceProviderURLLoader::LoadAsynchronously(
    std::unique_ptr<network::ResourceRequest> request,
    scoped_refptr<const blink::SecurityOrigin> top_frame_origin,
    bool no_mime_sniffing,
    std::unique_ptr<blink::ResourceLoadInfoNotifierWrapper>
        resource_load_info_notifier_wrapper,
    blink::CodeCacheHost* code_cache_host,
    blink::URLLoaderClient* client) {
  if (!client) {
    return;
  }

  if (!state_ || !state_->callback) {
    client->DidFail(
        blink::WebURLError(net::ERR_FAILED,
                           blink::WebURL(blink::KURL(request->url))),
        base::TimeTicks::Now(),
        /*total_encoded_data_length=*/0,
        /*total_encoded_body_length=*/0,
        /*total_decoded_body_length=*/0);
    return;
  }

  std::string url_string = request->url.spec();
  OuiResourceResponse res = {};

  int found = state_->callback(url_string.c_str(), &res, state_->user_data);
  if (!found || !res.data || res.length == 0) {
    // Free response data if the provider set it despite returning 0.
    if (res.free_func && res.data) {
      res.free_func(res.data, res.free_user_data);
    }
    client->DidFail(
        blink::WebURLError(net::ERR_FILE_NOT_FOUND,
                           blink::WebURL(blink::KURL(request->url))),
        base::TimeTicks::Now(),
        /*total_encoded_data_length=*/0,
        /*total_encoded_body_length=*/0,
        /*total_decoded_body_length=*/0);
    return;
  }

  // Build the response.
  blink::WebURL web_url(blink::KURL(request->url));
  blink::WebURLResponse response(web_url);
  response.SetHttpStatusCode(200);
  response.SetHttpStatusText(blink::WebString::FromLatin1("OK"));
  response.SetExpectedContentLength(static_cast<int64_t>(res.length));

  if (res.mime_type) {
    response.SetMimeType(blink::WebString::FromUTF8(res.mime_type));
  } else {
    response.SetMimeType(
        blink::WebString::FromLatin1("application/octet-stream"));
    if (res.length >= 8) {
      if (res.data[0] == 0x89 && res.data[1] == 0x50 &&
          res.data[2] == 0x4E && res.data[3] == 0x47) {
        response.SetMimeType(blink::WebString::FromLatin1("image/png"));
      } else if (res.data[0] == 0xFF && res.data[1] == 0xD8 &&
                 res.data[2] == 0xFF) {
        response.SetMimeType(blink::WebString::FromLatin1("image/jpeg"));
      } else if (res.data[0] == 0x47 && res.data[1] == 0x49 &&
                 res.data[2] == 0x46 && res.data[3] == 0x38) {
        response.SetMimeType(blink::WebString::FromLatin1("image/gif"));
      } else if (res.data[0] == 0x52 && res.data[1] == 0x49 &&
                 res.data[2] == 0x46 && res.data[3] == 0x46 &&
                 res.length >= 12 &&
                 res.data[8] == 0x57 && res.data[9] == 0x45 &&
                 res.data[10] == 0x42 && res.data[11] == 0x50) {
        response.SetMimeType(blink::WebString::FromLatin1("image/webp"));
      }
    }
  }

  // Build the data buffer from the provider's response.
  scoped_refptr<blink::SharedBuffer> data = blink::SharedBuffer::Create(
      base::span<const char>(reinterpret_cast<const char*>(res.data),
                             res.length));
  size_t data_length = res.length;

  // Free the provider's data now that we've copied it.
  if (res.free_func) {
    res.free_func(res.data, res.free_user_data);
  }

  // Deliver response to the client. We deliver synchronously since we already
  // have all the data (no network I/O). This is what URLLoaderMock does via
  // URLLoaderTestDelegate: pass an empty data pipe handle, then deliver data
  // chunks via DidReceiveDataForTesting.
  client->DidReceiveResponse(response,
                             /*body=*/mojo::ScopedDataPipeConsumerHandle(),
                             /*cached_metadata=*/std::nullopt);

  // Deliver body data.
  for (const auto& span : *data) {
    client->DidReceiveDataForTesting(span);
  }

  // Signal completion.
  client->DidFinishLoading(
      base::TimeTicks::Now(),
      /*total_encoded_data_length=*/static_cast<int64_t>(data_length),
      /*total_encoded_body_length=*/static_cast<uint64_t>(data_length),
      /*total_decoded_body_length=*/static_cast<int64_t>(data_length));
}

void ResourceProviderURLLoader::Freeze(blink::LoaderFreezeMode mode) {
  // No-op: we don't support freezing since all data is delivered immediately.
}

void ResourceProviderURLLoader::DidChangePriority(
    blink::WebURLRequest::Priority new_priority,
    int intra_priority_value) {
  // No-op: priority changes are irrelevant for synchronous delivery.
}

scoped_refptr<base::SingleThreadTaskRunner>
ResourceProviderURLLoader::GetTaskRunnerForBodyLoader() {
  return base::MakeRefCounted<blink::scheduler::FakeTaskRunner>();
}

// ═══════════════════════════════════════════════════════════════════════════
// ResourceProviderFrameClient
// ═══════════════════════════════════════════════════════════════════════════

ResourceProviderFrameClient::ResourceProviderFrameClient(
    ResourceProviderState* state)
    : state_(state) {}

std::unique_ptr<blink::URLLoader>
ResourceProviderFrameClient::CreateURLLoaderForTesting() {
  return std::make_unique<ResourceProviderURLLoader>(state_);
}

}  // namespace openui

// ═══════════════════════════════════════════════════════════════════════════
// C API: Resource provider
// ═══════════════════════════════════════════════════════════════════════════

OuiStatus oui_document_set_resource_provider(
    OuiDocument* doc,
    OuiResourceProviderFunc provider,
    void* user_data) {
  if (!doc) {
    return OUI_ERROR_INVALID_ARGUMENT;
  }
  if (!provider) {
    return OUI_ERROR_INVALID_ARGUMENT;
  }

  auto* impl = reinterpret_cast<OuiDocumentImpl*>(doc);
  impl->resource_provider.callback = provider;
  impl->resource_provider.user_data = user_data;

  // Invalidate all element wrappers before recreating the page holder.
  // The old Document/DOM is destroyed when we replace page_holder.
  OuiInvalidateElementWrappers(impl);

  // Recreate the DummyPageHolder with a ResourceProviderFrameClient so that
  // all future URL fetches (images, CSS, etc.) are routed through the
  // user's callback.  The old page holder is replaced; any elements from
  // a prior DOM are invalidated.
  auto* frame_client = blink::MakeGarbageCollected<
      openui::ResourceProviderFrameClient>(&impl->resource_provider);
  gfx::Size viewport_size = impl->page_holder
      ? impl->page_holder->GetFrameView().GetLayoutSize()
      : gfx::Size(800, 600);
  impl->page_holder = std::make_unique<blink::DummyPageHolder>(
      viewport_size, /*chrome_client=*/nullptr, frame_client);

  return OUI_OK;
}

// ═══════════════════════════════════════════════════════════════════════════
// C API: Direct image injection
// ═══════════════════════════════════════════════════════════════════════════

OuiStatus oui_element_set_image_data(
    OuiElement* elem,
    const uint8_t* rgba_pixels,
    int width,
    int height) {
  if (!elem || !rgba_pixels || width <= 0 || height <= 0) {
    return OUI_ERROR_INVALID_ARGUMENT;
  }

  auto* elem_impl = reinterpret_cast<OuiElementImpl*>(elem);
  if (!elem_impl->element) {
    return OUI_ERROR_INVALID_ARGUMENT;
  }

  // Verify the element is an <img>.
  auto* img_element =
      blink::DynamicTo<blink::HTMLImageElement>(elem_impl->element.Get());
  if (!img_element) {
    return OUI_ERROR_INVALID_ARGUMENT;
  }

  // Build an SkImage from raw RGBA pixels.
  // SkImages::RasterFromData takes ownership of the SkData but we need to
  // copy the pixels since the caller owns them.
  size_t row_bytes = static_cast<size_t>(width) * 4;
  size_t total_bytes = row_bytes * static_cast<size_t>(height);

  sk_sp<SkData> sk_data = SkData::MakeWithCopy(rgba_pixels, total_bytes);
  SkImageInfo info = SkImageInfo::Make(
      width, height, kRGBA_8888_SkColorType, kUnpremul_SkAlphaType);

  sk_sp<SkImage> sk_image = SkImages::RasterFromData(info, sk_data, row_bytes);
  if (!sk_image) {
    return OUI_ERROR_INTERNAL;
  }

  // Wrap in a blink StaticBitmapImage → ImageResourceContent.
  scoped_refptr<blink::UnacceleratedStaticBitmapImage> blink_image =
      blink::UnacceleratedStaticBitmapImage::Create(std::move(sk_image));
  if (!blink_image) {
    return OUI_ERROR_INTERNAL;
  }

  blink::ImageResourceContent* content =
      blink::ImageResourceContent::CreateLoaded(std::move(blink_image));
  if (!content) {
    return OUI_ERROR_INTERNAL;
  }

  // Inject the image into the HTMLImageElement.
  img_element->SetImageForTest(content);

  return OUI_OK;
}

OuiStatus oui_element_set_image_encoded(
    OuiElement* elem,
    const uint8_t* data,
    size_t length) {
  if (!elem || !data || length == 0) {
    return OUI_ERROR_INVALID_ARGUMENT;
  }

  auto* elem_impl = reinterpret_cast<OuiElementImpl*>(elem);
  if (!elem_impl->element) {
    return OUI_ERROR_INVALID_ARGUMENT;
  }

  // Verify the element is an <img>.
  auto* img_element =
      blink::DynamicTo<blink::HTMLImageElement>(elem_impl->element.Get());
  if (!img_element) {
    return OUI_ERROR_INVALID_ARGUMENT;
  }

  // Create a BitmapImage and feed it the encoded data.
  scoped_refptr<blink::BitmapImage> bitmap_image =
      blink::BitmapImage::Create();

  scoped_refptr<blink::SharedBuffer> buffer = blink::SharedBuffer::Create(
      base::span<const char>(reinterpret_cast<const char*>(data), length));

  // SetData with all_data_received=true decodes the image.
  blink::Image::SizeAvailability size_status =
      bitmap_image->SetData(std::move(buffer), /*all_data_received=*/true);

  if (size_status == blink::Image::kSizeUnavailable) {
    // The image data couldn't be decoded.
    return OUI_ERROR_INVALID_VALUE;
  }

  // Wrap in ImageResourceContent and inject into the element.
  blink::ImageResourceContent* content =
      blink::ImageResourceContent::CreateLoaded(std::move(bitmap_image));
  if (!content) {
    return OUI_ERROR_INTERNAL;
  }

  img_element->SetImageForTest(content);

  return OUI_OK;
}
