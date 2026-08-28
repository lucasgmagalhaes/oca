import json
import pathlib
import tempfile
import unittest

from packaging.collect_gstreamer_plugins import collect


class CollectGStreamerPluginsTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.temp.name)
        self.source = self.root / "source"
        self.destination = self.root / "destination"
        self.source.mkdir()
        self.allowlist = self.root / "allowlist.json"
        self.allowlist.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "common": ["app", "playback"],
                    "platforms": {"linux": {"one_of": [["pulseaudio", "alsa"]]}},
                }
            ),
            encoding="utf-8",
        )

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_copies_only_allowlisted_plugins(self) -> None:
        for name in ("libgstapp.so", "libgstplayback.so", "libgstalsa.so", "libgstunsafe.so"):
            (self.source / name).write_text(name, encoding="utf-8")

        copied = collect(self.source, self.destination, self.allowlist, "linux")

        self.assertEqual(
            {path.name for path in copied},
            {"libgstapp.so", "libgstplayback.so", "libgstalsa.so"},
        )

    def test_rejects_missing_required_plugin(self) -> None:
        (self.source / "libgstapp.so").touch()
        (self.source / "libgstalsa.so").touch()
        (self.source / "libgstplayback.so.untrusted").touch()

        with self.assertRaisesRegex(FileNotFoundError, "playback"):
            collect(self.source, self.destination, self.allowlist, "linux")

    def test_rejects_symlink_escaping_source(self) -> None:
        outside = self.root / "libgstapp.so"
        outside.touch()
        (self.source / "libgstapp.so").symlink_to(outside)
        (self.source / "libgstplayback.so").touch()
        (self.source / "libgstalsa.so").touch()

        with self.assertRaisesRegex(ValueError, "outside source"):
            collect(self.source, self.destination, self.allowlist, "linux")

    def test_rejects_invalid_plugin_identifier(self) -> None:
        self.allowlist.write_text(
            json.dumps({"schema_version": 1, "common": ["../app"], "platforms": {}}),
            encoding="utf-8",
        )

        with self.assertRaisesRegex(ValueError, "invalid"):
            collect(self.source, self.destination, self.allowlist, "linux")


if __name__ == "__main__":
    unittest.main()
