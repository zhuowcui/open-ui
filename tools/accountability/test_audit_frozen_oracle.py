"""Guard the immutable evidence that invalidates historical exactness."""

import json
import unittest

from tools.accountability import audit_frozen_oracle as audit
from tools.accountability import restore_frozen_openui_archive as frozen


class FrozenOracleAuditTests(unittest.TestCase):
    def test_historical_passes_do_not_prove_exact_pixels(self) -> None:
        ids = json.loads(audit.MANIFEST.read_text())
        result = audit.historical_audit(ids)
        self.assertEqual(result["reported_passes"], 5731)
        self.assertEqual(result["passes_with_nonzero_compared_channel_delta"], 186)
        self.assertEqual(result["excluded_right_strip_px"], 15)
        self.assertFalse(result["establishes_zero_tolerance_equality"])

    def test_minimized_frozen_image_remains_byte_pinned(self) -> None:
        images = frozen.frozen_images()
        image = images[f"{audit.EXAMPLE_ID}/openui.png"]
        self.assertEqual(
            audit.rgba_sha256(image),
            "a53d2d217b7d1cd83f5f2a0d3ba0c7b6b7746f0f1c8970b89421c60293fd7f8b",
        )


if __name__ == "__main__":
    unittest.main()
