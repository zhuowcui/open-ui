// src/skia/oui_sk_surface.cc — Surface & GPU context C API wrappers

#include "src/skia/oui_sk_types_internal.h"

#include "include/core/SkBitmap.h"
#include "include/core/SkColorSpace.h"
#include "include/core/SkImageInfo.h"
#include "include/core/SkPixmap.h"
#include "include/core/SkSurface.h"

#ifdef SK_GL
#include "include/gpu/ganesh/GrDirectContext.h"
#include "include/gpu/ganesh/SkSurfaceGanesh.h"
#include "include/gpu/ganesh/gl/GrGLDirectContext.h"
#include "include/gpu/ganesh/gl/GrGLInterface.h"
#include "include/gpu/ganesh/gl/egl/GrGLMakeEGLInterface.h"
#include "include/gpu/ganesh/gl/glx/GrGLMakeGLXInterface.h"
#endif

extern "C" {

OuiSkSurface oui_sk_surface_create_raster(int width, int height) {
    if (width <= 0 || height <= 0) return nullptr;

    SkImageInfo info = SkImageInfo::MakeN32Premul(width, height);
    sk_sp<SkSurface> surface = SkSurfaces::Raster(info);
    if (!surface) return nullptr;

    auto* s = new(std::nothrow) OuiSkSurface_t();
    if (!s) return nullptr;
    s->surface = std::move(surface);
    return s;
}

OuiSkSurface oui_sk_surface_create_gpu(
    OuiSkGpuContext gpu_ctx, int width, int height) {
    if (!gpu_ctx || width <= 0 || height <= 0) return nullptr;

#ifdef SK_GANESH
    SkImageInfo info = SkImageInfo::MakeN32Premul(width, height);
    sk_sp<SkSurface> surface = SkSurfaces::RenderTarget(
        gpu_ctx->context.get(), skgpu::Budgeted::kNo, info);
    if (!surface) return nullptr;

    auto* s = new(std::nothrow) OuiSkSurface_t();
    if (!s) return nullptr;
    s->surface = std::move(surface);
    return s;
#else
    return nullptr;
#endif
}

void oui_sk_surface_destroy(OuiSkSurface surface) {
    delete surface;
}

OuiSkCanvas oui_sk_surface_get_canvas(OuiSkSurface surface) {
    if (!surface || !surface->surface) return nullptr;

    if (!surface->canvas_valid) {
        surface->canvas_handle.canvas = surface->surface->getCanvas();
        surface->canvas_valid = true;
    }
    return &surface->canvas_handle;
}

OuiSkStatus oui_sk_surface_read_pixels(
    OuiSkSurface surface, void* dst, size_t dst_row_bytes,
    int src_x, int src_y, int width, int height) {
    if (!surface || !dst) return OUI_SK_ERROR_NULL_POINTER;

    SkImageInfo info = SkImageInfo::MakeN32Premul(width, height);
    bool ok = surface->surface->readPixels(info, dst, dst_row_bytes, src_x, src_y);
    return ok ? OUI_SK_OK : OUI_SK_ERROR_INVALID_ARGUMENT;
}

OuiSkImage oui_sk_surface_make_image_snapshot(OuiSkSurface surface) {
    if (!surface || !surface->surface) return nullptr;

    sk_sp<SkImage> img = surface->surface->makeImageSnapshot();
    if (!img) return nullptr;

    auto* i = new(std::nothrow) OuiSkImage_t();
    if (!i) return nullptr;
    i->image = std::move(img);
    return i;
}

OuiSkGpuContext oui_sk_gpu_context_create_gl(void) {
#ifdef SK_GL
    sk_sp<const GrGLInterface> interface = GrGLInterfaces::MakeEGL();
    if (!interface) {
        interface = GrGLInterfaces::MakeGLX();
    }
    if (!interface) return nullptr;

    sk_sp<GrDirectContext> ctx = GrDirectContexts::MakeGL(interface);
    if (!ctx) return nullptr;

    auto* c = new(std::nothrow) OuiSkGpuContext_t();
    if (!c) return nullptr;
    c->context = std::move(ctx);
    return c;
#else
    return nullptr;
#endif
}

void oui_sk_gpu_context_destroy(OuiSkGpuContext ctx) {
    if (ctx) {
#ifdef SK_GANESH
        if (ctx->context) {
            ctx->context->abandonContext();
        }
#endif
        delete ctx;
    }
}

}  // extern "C"
