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
            audit.sha256(image),
            "174e56f70e4481e1db0a3ec1e981c3e60265e8f5c4185746ffdaf3d54647311b",
        )


if __name__ == "__main__":
    unittest.main()
