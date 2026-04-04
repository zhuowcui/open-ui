// Stubs for Chromium's Rust alloc shim and fontconfig FFI.
//
// Chromium's Rust toolchain uses a custom allocator shim
// (`__rust_no_alloc_shim_is_unstable_v2`) which is normally provided by
// Chromium's allocator infrastructure. Since our binary uses Rust's standard
// allocator, we provide a no-op definition here.
//
// `add_patterns_to_fontset` is a Rust FFI function called by Chromium's
// fontconfig integration. It's safe to no-op because our offscreen rendering
// pipeline doesn't rely on fontconfig pattern matching.

#include <stdint.h>

// Chromium Rust alloc shim marker — signals the allocator is ready.
uint8_t __rust_no_alloc_shim_is_unstable_v2 = 0;

// Fontconfig Rust FFI — called during font cache rebuild.
int add_patterns_to_fontset(void *a, void *b, void *c, void *d) {
    (void)a; (void)b; (void)c; (void)d;
    return 0;
}
