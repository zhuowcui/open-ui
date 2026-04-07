#!/usr/bin/env python3
"""
pixel_diff.py — Compare two PNG images pixel-by-pixel and output diff statistics.

Usage:
    python3 pixel_diff.py <image_a> <image_b> <diff_out> <json_out> [--tolerance N]

Output JSON:
    {
        "total_pixels": 480000,
        "mismatched_pixels": 0,
        "mismatch_pct": 0.0,
        "max_channel_diff": 0,
        "avg_channel_diff": 0.0,
        "size_match": true,
        "image_a_size": [800, 600],
        "image_b_size": [800, 600],
        "status": "pass"  // or "fail" or "size_mismatch"
    }
"""

import argparse
import json
import sys

try:
    from PIL import Image
except ImportError:
    print("ERROR: Pillow not installed. Run: pip install Pillow", file=sys.stderr)
    sys.exit(1)


def compare_images(path_a: str, path_b: str, diff_path: str, tolerance: int = 2) -> dict:
    """Compare two images pixel-by-pixel and generate a diff image."""
    img_a = Image.open(path_a).convert("RGBA")
    img_b = Image.open(path_b).convert("RGBA")

    result = {
        "image_a_size": list(img_a.size),
        "image_b_size": list(img_b.size),
        "size_match": img_a.size == img_b.size,
    }

    if not result["size_match"]:
        result["status"] = "size_mismatch"
        result["total_pixels"] = 0
        result["mismatched_pixels"] = 0
        result["mismatch_pct"] = 100.0
        result["max_channel_diff"] = 255
        result["avg_channel_diff"] = 0.0
        # Create a red diff image at max of both sizes
        w = max(img_a.size[0], img_b.size[0])
        h = max(img_a.size[1], img_b.size[1])
        diff = Image.new("RGBA", (w, h), (255, 0, 0, 255))
        diff.save(diff_path)
        return result

    w, h = img_a.size
    total = w * h
    pixels_a = img_a.load()
    pixels_b = img_b.load()

    diff = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    diff_pixels = diff.load()

    mismatched = 0
    max_diff = 0
    total_diff = 0
    channel_count = 0

    for y in range(h):
        for x in range(w):
            pa = pixels_a[x, y]
            pb = pixels_b[x, y]

            # Compare RGB channels (ignore alpha for now)
            dr = abs(pa[0] - pb[0])
            dg = abs(pa[1] - pb[1])
            db = abs(pa[2] - pb[2])

            max_ch = max(dr, dg, db)
            max_diff = max(max_diff, max_ch)
            total_diff += dr + dg + db
            channel_count += 3

            if max_ch > tolerance:
                mismatched += 1
                # Highlight mismatch in diff image: red intensity = diff magnitude
                intensity = min(255, max_ch * 3)
                diff_pixels[x, y] = (intensity, 0, 0, 255)
            else:
                # Matching pixel: show faint green
                diff_pixels[x, y] = (0, 40, 0, 255)

    diff.save(diff_path)

    mismatch_pct = (mismatched / total * 100) if total > 0 else 0.0
    avg_diff = (total_diff / channel_count) if channel_count > 0 else 0.0

    result.update({
        "total_pixels": total,
        "mismatched_pixels": mismatched,
        "mismatch_pct": round(mismatch_pct, 6),
        "max_channel_diff": max_diff,
        "avg_channel_diff": round(avg_diff, 4),
        "status": "pass" if mismatched == 0 else "fail",
    })

    return result


def main():
    parser = argparse.ArgumentParser(description="Pixel-by-pixel image comparison")
    parser.add_argument("image_a", help="First image (Chromium reference)")
    parser.add_argument("image_b", help="Second image (OpenUI render)")
    parser.add_argument("diff_out", help="Output diff image path")
    parser.add_argument("json_out", help="Output JSON result path")
    parser.add_argument("--tolerance", type=int, default=2,
                        help="Per-channel tolerance (default: 2)")
    args = parser.parse_args()

    result = compare_images(args.image_a, args.image_b, args.diff_out, args.tolerance)

    with open(args.json_out, "w") as f:
        json.dump(result, f, indent=2)

    # Also print summary to stdout
    if result["status"] == "pass":
        print(f"PASS: 0 mismatched pixels out of {result['total_pixels']}")
    elif result["status"] == "size_mismatch":
        print(f"SIZE MISMATCH: {result['image_a_size']} vs {result['image_b_size']}")
    else:
        print(f"FAIL: {result['mismatched_pixels']} / {result['total_pixels']}"
              f" ({result['mismatch_pct']:.4f}%) max_diff={result['max_channel_diff']}")

    sys.exit(0 if result["status"] == "pass" else 1)


if __name__ == "__main__":
    main()
