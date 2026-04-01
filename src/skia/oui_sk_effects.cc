// src/skia/oui_sk_effects.cc — Shader, image filter, mask filter, color filter wrappers

#include "src/skia/oui_sk_types_internal.h"

#include "include/core/SkColorFilter.h"
#include "include/effects/SkGradient.h"
#include "include/effects/SkImageFilters.h"

extern "C" {

/* ─── Shaders ───────────────────────────────────────────────────── */

OuiSkShader oui_sk_shader_linear_gradient(
    OuiSkPoint start, OuiSkPoint end,
    const OuiSkColor* colors, const float* positions, int count,
    OuiSkTileMode tile_mode) {
    if (!colors || count < 2) return nullptr;

    try {
        SkPoint pts[2] = {{start.x, start.y}, {end.x, end.y}};

        std::vector<SkColor4f> sk_colors(count);
        for (int i = 0; i < count; i++) {
            sk_colors[i] = SkColor4f::FromColor(to_sk_color(colors[i]));
        }

        std::vector<float> pos;
        if (positions) {
            pos.assign(positions, positions + count);
        }

        SkGradient::Colors grad_colors(
            SkSpan<const SkColor4f>(sk_colors.data(), count),
            positions ? SkSpan<const float>(pos.data(), count) : SkSpan<const float>(),
            to_sk_tile_mode(tile_mode));
        SkGradient grad(grad_colors, {});

        sk_sp<SkShader> shader = SkShaders::LinearGradient(pts, grad);
        if (!shader) return nullptr;

        auto* s = new(std::nothrow) OuiSkShader_t();
        if (!s) return nullptr;
        s->shader = std::move(shader);
        return s;
    } catch (...) {
        return nullptr;
    }
}

OuiSkShader oui_sk_shader_radial_gradient(
    OuiSkPoint center, float radius,
    const OuiSkColor* colors, const float* positions, int count,
    OuiSkTileMode tile_mode) {
    if (!colors || count < 2 || radius <= 0) return nullptr;

    try {
        SkPoint sk_center = {center.x, center.y};

        std::vector<SkColor4f> sk_colors(count);
        for (int i = 0; i < count; i++) {
            sk_colors[i] = SkColor4f::FromColor(to_sk_color(colors[i]));
        }

        std::vector<float> pos;
        if (positions) {
            pos.assign(positions, positions + count);
        }

        SkGradient::Colors grad_colors(
            SkSpan<const SkColor4f>(sk_colors.data(), count),
            positions ? SkSpan<const float>(pos.data(), count) : SkSpan<const float>(),
            to_sk_tile_mode(tile_mode));
        SkGradient grad(grad_colors, {});

        sk_sp<SkShader> shader = SkShaders::RadialGradient(sk_center, radius, grad);
        if (!shader) return nullptr;

        auto* s = new(std::nothrow) OuiSkShader_t();
        if (!s) return nullptr;
        s->shader = std::move(shader);
        return s;
    } catch (...) {
        return nullptr;
    }
}

OuiSkShader oui_sk_shader_sweep_gradient(
    OuiSkPoint center,
    const OuiSkColor* colors, const float* positions, int count) {
    if (!colors || count < 2) return nullptr;

    try {
        SkPoint sk_center = {center.x, center.y};

        std::vector<SkColor4f> sk_colors(count);
        for (int i = 0; i < count; i++) {
            sk_colors[i] = SkColor4f::FromColor(to_sk_color(colors[i]));
        }

        std::vector<float> pos;
        if (positions) {
            pos.assign(positions, positions + count);
        }

        SkGradient::Colors grad_colors(
            SkSpan<const SkColor4f>(sk_colors.data(), count),
            positions ? SkSpan<const float>(pos.data(), count) : SkSpan<const float>(),
            SkTileMode::kClamp);
        SkGradient grad(grad_colors, {});

        sk_sp<SkShader> shader = SkShaders::SweepGradient(sk_center, grad);
        if (!shader) return nullptr;

        auto* s = new(std::nothrow) OuiSkShader_t();
        if (!s) return nullptr;
        s->shader = std::move(shader);
        return s;
    } catch (...) {
        return nullptr;
    }
}

OuiSkShader oui_sk_shader_image(
    OuiSkImage image, OuiSkTileMode tile_x, OuiSkTileMode tile_y) {
    if (!image || !image->image) return nullptr;

    sk_sp<SkShader> shader = image->image->makeShader(
        to_sk_tile_mode(tile_x), to_sk_tile_mode(tile_y),
        SkSamplingOptions());
    if (!shader) return nullptr;

    auto* s = new(std::nothrow) OuiSkShader_t();
    if (!s) return nullptr;
    s->shader = std::move(shader);
    return s;
}

void oui_sk_shader_destroy(OuiSkShader shader) {
    delete shader;
}

/* ─── Image Filters ─────────────────────────────────────────────── */

OuiSkImageFilter oui_sk_image_filter_blur(
    float sigma_x, float sigma_y, OuiSkTileMode tile_mode) {
    sk_sp<SkImageFilter> filter = SkImageFilters::Blur(
        sigma_x, sigma_y, to_sk_tile_mode(tile_mode), nullptr);
    if (!filter) return nullptr;

    auto* f = new(std::nothrow) OuiSkImageFilter_t();
    if (!f) return nullptr;
    f->filter = std::move(filter);
    return f;
}

OuiSkImageFilter oui_sk_image_filter_drop_shadow(
    float dx, float dy, float sigma_x, float sigma_y, OuiSkColor color) {
    sk_sp<SkImageFilter> filter = SkImageFilters::DropShadow(
        dx, dy, sigma_x, sigma_y, to_sk_color(color), nullptr);
    if (!filter) return nullptr;

    auto* f = new(std::nothrow) OuiSkImageFilter_t();
    if (!f) return nullptr;
    f->filter = std::move(filter);
    return f;
}

OuiSkImageFilter oui_sk_image_filter_color_filter(
    OuiSkColorFilter color_filter) {
    if (!color_filter || !color_filter->filter) return nullptr;

    sk_sp<SkImageFilter> filter = SkImageFilters::ColorFilter(
        color_filter->filter, nullptr);
    if (!filter) return nullptr;

    auto* f = new(std::nothrow) OuiSkImageFilter_t();
    if (!f) return nullptr;
    f->filter = std::move(filter);
    return f;
}

OuiSkImageFilter oui_sk_image_filter_compose(
    OuiSkImageFilter outer, OuiSkImageFilter inner) {
    if (!outer || !inner) return nullptr;

    sk_sp<SkImageFilter> filter = SkImageFilters::Compose(
        outer->filter, inner->filter);
    if (!filter) return nullptr;

    auto* f = new(std::nothrow) OuiSkImageFilter_t();
    if (!f) return nullptr;
    f->filter = std::move(filter);
    return f;
}

void oui_sk_image_filter_destroy(OuiSkImageFilter filter) {
    delete filter;
}

/* ─── Mask Filters ──────────────────────────────────────────────── */

OuiSkMaskFilter oui_sk_mask_filter_blur(OuiSkBlurStyle style, float sigma) {
    sk_sp<SkMaskFilter> filter = SkMaskFilter::MakeBlur(
        static_cast<SkBlurStyle>(style), sigma);
    if (!filter) return nullptr;

    auto* f = new(std::nothrow) OuiSkMaskFilter_t();
    if (!f) return nullptr;
    f->filter = std::move(filter);
    return f;
}

void oui_sk_mask_filter_destroy(OuiSkMaskFilter filter) {
    delete filter;
}

/* ─── Color Filters ─────────────────────────────────────────────── */

OuiSkColorFilter oui_sk_color_filter_blend(
    OuiSkColor color, OuiSkBlendMode mode) {
    sk_sp<SkColorFilter> filter = SkColorFilters::Blend(
        to_sk_color4f({static_cast<float>(color.r) / 255.0f,
                       static_cast<float>(color.g) / 255.0f,
                       static_cast<float>(color.b) / 255.0f,
                       static_cast<float>(color.a) / 255.0f}),
        nullptr,
        to_sk_blend_mode(mode));
    if (!filter) return nullptr;

    auto* f = new(std::nothrow) OuiSkColorFilter_t();
    if (!f) return nullptr;
    f->filter = std::move(filter);
    return f;
}

OuiSkColorFilter oui_sk_color_filter_matrix(const float matrix[20]) {
    if (!matrix) return nullptr;

    sk_sp<SkColorFilter> filter = SkColorFilters::Matrix(matrix);
    if (!filter) return nullptr;

    auto* f = new(std::nothrow) OuiSkColorFilter_t();
    if (!f) return nullptr;
    f->filter = std::move(filter);
    return f;
}

OuiSkColorFilter oui_sk_color_filter_compose(
    OuiSkColorFilter outer, OuiSkColorFilter inner) {
    if (!outer || !inner) return nullptr;

    sk_sp<SkColorFilter> filter = SkColorFilters::Compose(
        outer->filter, inner->filter);
    if (!filter) return nullptr;

    auto* f = new(std::nothrow) OuiSkColorFilter_t();
    if (!f) return nullptr;
    f->filter = std::move(filter);
    return f;
}

void oui_sk_color_filter_destroy(OuiSkColorFilter filter) {
    delete filter;
}

}  // extern "C"
