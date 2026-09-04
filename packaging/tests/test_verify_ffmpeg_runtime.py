import pathlib
import unittest

from packaging.verify_ffmpeg_runtime import REQUIRED_ENCODERS, encoder_names


class VerifyFfmpegRuntimeTest(unittest.TestCase):
    def test_parses_encoder_names_from_ffmpeg_output(self) -> None:
        output = """Encoders:\n V..... h264_videotoolbox VideoToolbox H.264 Encoder\n V..... libopenh264 OpenH264\n"""

        self.assertEqual(encoder_names(output), {"h264_videotoolbox", "libopenh264"})

    def test_each_platform_requires_cpu_fallback(self) -> None:
        for platform, encoders in REQUIRED_ENCODERS.items():
            with self.subTest(platform=platform):
                self.assertIn("libopenh264", encoders)

    def test_macos_requires_apple_hardware_encoder(self) -> None:
        self.assertIn("h264_videotoolbox", REQUIRED_ENCODERS["macos"])


if __name__ == "__main__":
    unittest.main()
