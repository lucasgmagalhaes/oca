import json
import pathlib
import tempfile
import unittest

from packaging.generate_third_party_notices import generate


REPO = pathlib.Path(__file__).parents[2]


class GenerateThirdPartyNoticesTest(unittest.TestCase):
    def test_generates_full_texts_and_preserves_notice(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            package = root / "crate"
            package.mkdir()
            (package / "Cargo.toml").write_text("[package]\nname='fixture'\nversion='1.0.0'\n", encoding="utf-8")
            (package / "NOTICE").write_text("Fixture attribution\n", encoding="utf-8")
            metadata = root / "metadata.json"
            metadata.write_text(
                json.dumps(
                    {
                        "packages": [
                            {
                                "id": "registry+fixture#fixture@1.0.0",
                                "name": "fixture",
                                "version": "1.0.0",
                                "license": "MIT OR Apache-2.0",
                                "license_file": None,
                                "manifest_path": str(package / "Cargo.toml"),
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            output = root / "licenses"

            generate(REPO, output, metadata)

            notices = (output / "THIRD_PARTY_NOTICES.txt").read_text(encoding="utf-8")
            self.assertIn("Fixture attribution", notices)
            self.assertIn("Apache License", notices)
            self.assertIn("MIT License", notices)
            self.assertIn("THIRD PARTY SOFTWARE NOTICES AND INFORMATION", notices)
            self.assertEqual(
                (output / "packages" / "fixture-1.0.0" / "NOTICE").read_text(encoding="utf-8"),
                "Fixture attribution\n",
            )

    def test_rejects_package_without_license_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            package = root / "crate"
            package.mkdir()
            manifest = package / "Cargo.toml"
            manifest.touch()
            metadata = root / "metadata.json"
            metadata.write_text(
                json.dumps(
                    {
                        "packages": [
                            {
                                "id": "registry+fixture#fixture@1.0.0",
                                "name": "fixture",
                                "version": "1.0.0",
                                "license": None,
                                "license_file": None,
                                "manifest_path": str(manifest),
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(ValueError, "no license metadata"):
                generate(REPO, root / "licenses", metadata)


if __name__ == "__main__":
    unittest.main()
