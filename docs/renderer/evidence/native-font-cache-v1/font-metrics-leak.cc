#include "include/core/SkFontMgr.h"
#include "include/core/SkFont.h"
#include "include/core/SkFontMetrics.h"
#include "include/core/SkTypeface.h"
#include "include/ports/SkFontMgr_fontconfig.h"
#include "include/ports/SkFontScanner_FreeType.h"
#include <cstdio>
#include <cstdlib>

int main(int argc, char **argv) {
    int count = argc > 1 ? std::atoi(argv[1]) : 1;
    const char *family_name = argc > 2 ? argv[2] : "sans-serif";
    if (count < 1 || count > 100) return 2;
    for (int i = 0; i < count; ++i) {
        auto manager = SkFontMgr_New_FontConfig(nullptr, SkFontScanner_Make_FreeType());
        auto family = manager->matchFamily(family_name);
        const int faces = family->count();
        if (!faces) return 3;
        SkFontStyle requested;
        auto face = family->matchStyle(requested);
        if (!face) return 4;
        SkFont font(face, 16.0f);
        SkFontMetrics metrics;
        float spacing = font.getMetrics(&metrics);
        std::printf("font-spacing=%g\n", static_cast<double>(spacing));
        std::printf("manager=%d family=%s faces=%d glyphs=%d\n", i, family_name,
                    faces, face->countGlyphs());
    }
    return 0;
}
