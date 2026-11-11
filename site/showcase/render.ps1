# Rebuild the site's screenshots with Open UI (Windows).
#
#   powershell -File site\showcase\render.ps1 [scene...]
#
# Builds openui-site-showcase in release mode with the CI Skia flags and the
# shared prebuilt Skia (docs/DEVELOPMENT.md), then writes every scene, light
# and dark, to site/src/assets/showcase/. Commit the PNGs it changes. The
# site's own build never runs this: it only reads the committed images.
$Scenes = $args
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path "$PSScriptRoot\..\..").Path
$main = (git -C $root worktree list --porcelain | Select-Object -First 1) -replace '^worktree ', ''

if (-not (Get-Command clang -ErrorAction SilentlyContinue) -and (Test-Path 'C:\Program Files\LLVM\bin')) {
    $env:Path = "C:\Program Files\LLVM\bin;$env:Path"
    $env:LIBCLANG_PATH = 'C:\Program Files\LLVM\bin'
}
$env:SKIA_GN_ARGS = 'extra_cflags += [ "-DSK_SUPPORT_LEGACY_CONIC_CHOP", "-DSK_ENABLE_LEGACY_SHADERCONTEXT", "-DSK_USE_FREETYPE_EMBOLDEN", "-DSK_GAMMA_EXPONENT=1.2", "-DSK_GAMMA_CONTRAST=0.2", "-USK_GAMMA_APPLY_TO_A8", "-fno-strict-aliasing", "-msse3" ]'
$env:SKIA_NINJA_COMMAND = 'ninja'
$env:RUST_MIN_STACK = '4194304'
if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = Join-Path $main 'out\cargo-site' }
python (Join-Path $root 'tools\skia_prebuilt.py') env | Out-String | Invoke-Expression

cargo build --release --locked --manifest-path (Join-Path $PSScriptRoot 'Cargo.toml')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$out = Join-Path $root 'site\src\assets\showcase'
& (Join-Path $env:CARGO_TARGET_DIR 'release\openui-site-showcase.exe') $out @Scenes
exit $LASTEXITCODE
