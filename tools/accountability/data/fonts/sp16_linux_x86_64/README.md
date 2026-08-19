# SP16 pinned FreeType runtime

`libfreetype.so.6` is the Linux x86-64 FreeType runtime used only by the
SP16 real-font comparison profile. Chromium 147 statically links this exact
FreeType revision while the Rust Skia build otherwise loads the host runtime;
pinning both sides removes a host-dependent raster difference without changing
the legacy or Ahem profiles.

- Chromium: `147.0.7727.50`
- FreeType: `VER-2-14-2-12-g45556a19a`
- Revision: `45556a19aab9502b91d6f30931e0cb5256f683f8`
- Architecture: Linux x86-64
- SONAME: `libfreetype.so.6`
- SHA-256: `accea5cff7580ef0be412ac566ad2bd825ec699947d23a1a1a6602b58adba0ec`
- License: FreeType License; see `LICENSE-FTL.txt`

The shared object is linked from the pinned Chromium Release build's
position-independent FreeType and zlib objects. Public `FT_*` symbols are
restored to default visibility because Chromium compiles its static archive
with hidden visibility. Linker symbol binding is local (`-Bsymbolic`), matching
the static-library call path. The artifact depends only on `libm` and `libc`.

This runtime is not a consumer-facing Open UI dependency. The accountability
runner prepends this directory to `LD_LIBRARY_PATH` only for IDs in
`sp16_real_font_tests.json`.
