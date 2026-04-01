// src/skia/oui_sk_image.cc — Image decode/encode C API wrappers

#include "src/skia/oui_sk_types_internal.h"

#include "include/codec/SkCodec.h"
#include "include/core/SkBitmap.h"
#include "include/core/SkData.h"
#include "include/core/SkImage.h"
#include "include/core/SkPixmap.h"
#include "include/core/SkStream.h"
#include "include/encode/SkJpegEncoder.h"
#include "include/encode/SkPngEncoder.h"
#include "include/encode/SkWebpEncoder.h"

#include <cstdio>

extern "C" {

OuiSkImage oui_sk_image_decode(const void* data, size_t size) {
    if (!data || size == 0) return nullptr;

    sk_sp<SkData> sk_data = SkData::MakeWithCopy(data, size);
    sk_sp<SkImage> img = SkImages::DeferredFromEncodedData(sk_data);
    if (!img) return nullptr;

    // Force raster decode
    sk_sp<SkImage> raster = img->makeRasterImage();
    if (!raster) raster = std::move(img);

    auto* i = new(std::nothrow) OuiSkImage_t();
    if (!i) return nullptr;
    i->image = std::move(raster);
    return i;
}

OuiSkImage oui_sk_image_load_file(const char* path) {
    if (!path) return nullptr;

    sk_sp<SkData> data = SkData::MakeFromFileName(path);
    if (!data) return nullptr;

    sk_sp<SkImage> img = SkImages::DeferredFromEncodedData(data);
    if (!img) return nullptr;

    sk_sp<SkImage> raster = img->makeRasterImage();
    if (!raster) raster = std::move(img);

    auto* i = new(std::nothrow) OuiSkImage_t();
    if (!i) return nullptr;
    i->image = std::move(raster);
    return i;
}

void oui_sk_image_destroy(OuiSkImage image) {
    delete image;
}

int oui_sk_image_width(OuiSkImage image) {
    if (!image || !image->image) return 0;
    return image->image->width();
}

int oui_sk_image_height(OuiSkImage image) {
    if (!image || !image->image) return 0;
    return image->image->height();
}

OuiSkStatus oui_sk_image_encode(
    OuiSkImage image, OuiSkImageFormat format, int quality,
    void** out_data, size_t* out_size) {
    if (!image || !image->image || !out_data || !out_size) {
        return OUI_SK_ERROR_NULL_POINTER;
    }

    // Get pixel data from the image
    SkPixmap pixmap;
    SkBitmap bm;
    if (!image->image->peekPixels(&pixmap)) {
        // Image doesn't expose pixels directly — make a raster copy
        if (!bm.tryAllocPixels(image->image->imageInfo())) {
            return OUI_SK_ERROR_ENCODE_FAILED;
        }
        if (!image->image->readPixels(bm.pixmap(), 0, 0)) {
            return OUI_SK_ERROR_ENCODE_FAILED;
        }
        pixmap = bm.pixmap();
    }

    sk_sp<SkData> encoded;
    switch (format) {
        case OUI_SK_IMAGE_FORMAT_PNG: {
            SkPngEncoder::Options opts;
            encoded = SkPngEncoder::Encode(pixmap, opts);
            break;
        }
        case OUI_SK_IMAGE_FORMAT_JPEG: {
            SkJpegEncoder::Options opts;
            opts.fQuality = quality > 0 ? quality : 90;
            encoded = SkJpegEncoder::Encode(pixmap, opts);
            break;
        }
        case OUI_SK_IMAGE_FORMAT_WEBP: {
            SkWebpEncoder::Options opts;
            opts.fQuality = quality > 0 ? static_cast<float>(quality) : 90.0f;
            encoded = SkWebpEncoder::Encode(pixmap, opts);
            break;
        }
        default:
            return OUI_SK_ERROR_INVALID_ARGUMENT;
    }

    if (!encoded) return OUI_SK_ERROR_ENCODE_FAILED;

    *out_size = encoded->size();
    *out_data = malloc(encoded->size());
    if (!*out_data) return OUI_SK_ERROR_OUT_OF_MEMORY;
    memcpy(*out_data, encoded->data(), encoded->size());
    return OUI_SK_OK;
}

void oui_sk_image_encode_free(void* data) {
    free(data);
}

}  // extern "C"
