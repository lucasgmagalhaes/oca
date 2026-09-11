"""Per-asset sidecar JSON + the aggregate run-level metadata.json."""

from __future__ import annotations

import json
import os


class MetadataCollector:
    def __init__(self, layer_order: list[str], mode: str, source_image: str):
        self.layer_order = layer_order
        self.mode = mode
        self.source_image = source_image
        self.assets: list[dict] = []

    def add(self, category: str, relative_path: str, meta: dict) -> None:
        entry = {"category": category, "path": relative_path, **meta}
        self.assets.append(entry)

    def write_sidecar(self, png_path: str, category: str, meta: dict) -> None:
        json_path = os.path.splitext(png_path)[0] + ".json"
        with open(json_path, "w", encoding="utf-8") as fh:
            json.dump({"category": category, **meta}, fh, indent=2)

    def write_aggregate(self, out_dir: str) -> str:
        path = os.path.join(out_dir, "metadata.json")
        payload = {
            "source_image": self.source_image,
            "mode": self.mode,
            "layer_order": self.layer_order,
            "asset_count": len(self.assets),
            "assets": self.assets,
        }
        with open(path, "w", encoding="utf-8") as fh:
            json.dump(payload, fh, indent=2)
        return path
