#!/usr/bin/env bash
# Setup Chromium source tree for Open UI development.
#
# Downloads only the directories needed for extraction using git's
# partial-clone and sparse-checkout features. This reduces the checkout
# from ~30GB to ~2-3GB.
#
# Usage:
#   ./tools/setup-chromium.sh [--full]
#
# Options:
#   --full    Clone the entire source tree (for reference builds)
#
# Prerequisites:
#   - git >= 2.25 (for sparse-checkout support)
#   - depot_tools in PATH (for gclient)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CHROMIUM_DIR="$REPO_ROOT/third_party/chromium"
VERSION_FILE="$REPO_ROOT/CHROMIUM_VERSION"

# Read pinned version
if [ -f "$VERSION_FILE" ]; then
    CHROMIUM_VERSION=$(cat "$VERSION_FILE" | tr -d '[:space:]')
    echo "Pinned Chromium version: $CHROMIUM_VERSION"
else
    echo "WARNING: CHROMIUM_VERSION file not found, using HEAD"
    CHROMIUM_VERSION=""
fi

# Directories needed for Open UI extraction
SPARSE_DIRS=(
    # Core rendering pipeline
    "base/"
    "cc/"
    "gpu/"
    "ui/gfx/"
    "ui/gl/"
    "ui/base/"
    "ui/events/"
    "ui/platform_window/"

    # Blink rendering internals
    "third_party/blink/renderer/core/layout/"
    "third_party/blink/renderer/core/css/"
    "third_party/blink/renderer/core/style/"
    "third_party/blink/renderer/core/paint/"
    "third_party/blink/renderer/core/animation/"
    "third_party/blink/renderer/platform/graphics/"
    "third_party/blink/renderer/platform/fonts/"
    "third_party/blink/renderer/platform/text/"
    "third_party/blink/renderer/platform/geometry/"
    "third_party/blink/public/"

    # Skia
    "third_party/skia/"

    # Compositor dependencies
    "components/viz/"

    # Key third-party dependencies
    "third_party/abseil-cpp/"
    "third_party/icu/"
    "third_party/harfbuzz-ng/"
    "third_party/freetype/"
    "third_party/libpng/"
    "third_party/zlib/"
    "third_party/perfetto/"
    "third_party/angle/"

    # Build infrastructure (needed for GN)
    "build/"
    "buildtools/"
    "tools/gn/"
    "testing/"
    "url/"

    # Root build files
    "BUILD.gn"
    "DEPS"
    ".gn"
)

full_clone=false
if [ "${1:-}" = "--full" ]; then
    full_clone=true
fi

echo "============================================"
echo "  Open UI — Chromium Source Setup"
echo "============================================"
echo ""

# Check prerequisites
if ! command -v git &>/dev/null; then
    echo "ERROR: git not found"
    exit 1
fi

git_version=$(git --version | grep -oP '\d+\.\d+')
echo "Git version: $git_version"

if [ -d "$CHROMIUM_DIR/.git" ] || [ -d "$CHROMIUM_DIR/base" ]; then
    echo ""
    echo "Chromium source already exists at: $CHROMIUM_DIR"
    echo "To re-download, remove it first: rm -rf $CHROMIUM_DIR"
    echo ""

    # Update sparse-checkout config if needed
    if [ -f "$CHROMIUM_DIR/.git/info/sparse-checkout" ] && [ "$full_clone" = false ]; then
        echo "Updating sparse-checkout configuration..."
        cd "$CHROMIUM_DIR"
        git sparse-checkout set "${SPARSE_DIRS[@]}"
        echo "Sparse-checkout updated."
    fi

    exit 0
fi

echo ""
mkdir -p "$(dirname "$CHROMIUM_DIR")"

if [ "$full_clone" = true ]; then
    echo "Performing FULL clone (this will download ~15-20GB)..."
    echo ""
    git clone --depth 1 --no-tags \
        https://chromium.googlesource.com/chromium/src.git \
        "$CHROMIUM_DIR"
else
    echo "Performing SPARSE clone (only extraction-relevant directories)..."
    echo "This will download ~2-3GB instead of ~30GB."
    echo ""

    # Step 1: Initialize with partial clone (no blobs downloaded yet)
    git clone --filter=blob:none --no-checkout --depth 1 --no-tags \
        https://chromium.googlesource.com/chromium/src.git \
        "$CHROMIUM_DIR"

    cd "$CHROMIUM_DIR"

    # Step 2: Configure sparse-checkout
    git sparse-checkout init --cone
    git sparse-checkout set "${SPARSE_DIRS[@]}"

    # Step 3: Checkout (only fetches blobs for sparse dirs)
    git checkout
fi

cd "$CHROMIUM_DIR"

# Pin to specific version if available
if [ -n "$CHROMIUM_VERSION" ]; then
    echo ""
    echo "Note: Source is at HEAD (shallow clone). Pinned version $CHROMIUM_VERSION"
    echo "is recorded for reference. For exact version matching, use --full and"
    echo "git checkout tags/$CHROMIUM_VERSION"
fi

echo ""
echo "============================================"
echo "  Setup complete!"
echo "============================================"
echo ""
echo "Chromium source: $CHROMIUM_DIR"
du -sh "$CHROMIUM_DIR" 2>/dev/null || true
echo ""

# Verify key directories exist
missing=0
for dir in base cc third_party/skia ui/gfx; do
    if [ -d "$CHROMIUM_DIR/$dir" ]; then
        echo "  ✓ $dir"
    else
        echo "  ✗ $dir (MISSING)"
        missing=$((missing + 1))
    fi
done

if [ $missing -gt 0 ]; then
    echo ""
    echo "WARNING: $missing directories missing. The sparse-checkout may need adjustment."
    exit 1
fi

echo ""
echo "NOTE: Third-party dependencies (Skia, ICU, HarfBuzz, etc.) are separate"
echo "git repos managed by Chromium's gclient. To fetch them:"
echo ""
echo "  cd third_party/chromium"
echo "  # Create a .gclient file:"
echo "  cat > ../.gclient << 'EOF'"
echo "  solutions = [{"
echo '    "name": ".",'
echo '    "url": "https://chromium.googlesource.com/chromium/src.git",'
echo '    "managed": False,'
echo "  }]"
echo "  EOF"
echo "  gclient sync --nohooks --no-history"
echo "  gclient runhooks"
echo ""
echo "For development without gclient, use the full reference checkout:"
echo "  ./tools/setup-chromium.sh --full"
