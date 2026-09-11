"""Writes one exported sprite PNG (+ sidecar metadata dict) in either
`trimmed` or `canvas` mode."""

from __future__ import annotations

import os

import numpy as np

from processing.trimming import trim
from processing.transparency import make_transparent
from utils.geometry import Rect
from utils.image import save_rgba


class AssetExporter:
    def __init__(self, mode: str, padding: int, alpha_threshold: int, background_cfg):
        assert mode in ("trimmed", "canvas")
        self.mode = mode
        self.padding = padding
        self.alpha_threshold = alpha_threshold
        self.background_cfg = background_cfg

    def export(self, source_rgba: np.ndarray, cell_rect: Rect, out_path: str) -> dict | None:
        """`cell_rect` is the source-cell region (in the full-image coordinate
        space) that fully contains the sprite. Returns a metadata dict (without
        category/naming fields, which the caller fills in) or None if the
        region turned out empty after background removal."""
        region = source_rgba[cell_rect.y0 : cell_rect.y1, cell_rect.x0 : cell_rect.x1].copy()
        if region.size == 0:
            return None
        transparent = make_transparent(region, self.background_cfg)

        trimmed = trim(transparent, self.padding, self.alpha_threshold)
        if trimmed is None:
            return None
        cropped, bbox_in_region = trimmed
        sprite_bounds_abs = Rect(
            cell_rect.x0 + bbox_in_region.x0,
            cell_rect.y0 + bbox_in_region.y0,
            bbox_in_region.w,
            bbox_in_region.h,
        )

        os.makedirs(os.path.dirname(out_path), exist_ok=True)

        if self.mode == "trimmed":
            save_rgba(cropped, out_path)
            anchor = {"x": cropped.shape[1] // 2, "y": cropped.shape[0] - 1}
            meta = {
                "source_cell": cell_rect.as_dict(),
                "sprite_bounds": sprite_bounds_abs.as_dict(),
                "anchor": anchor,
                "offset_x": bbox_in_region.x0,
                "offset_y": bbox_in_region.y0,
                "original_canvas_width": cell_rect.w,
                "original_canvas_height": cell_rect.h,
                "exported_width": cropped.shape[1],
                "exported_height": cropped.shape[0],
            }
        else:  # canvas
            save_rgba(transparent, out_path)
            anchor = {"x": transparent.shape[1] // 2, "y": transparent.shape[0] - 1}
            meta = {
                "source_cell": cell_rect.as_dict(),
                "sprite_bounds": sprite_bounds_abs.as_dict(),
                "anchor": anchor,
                "exported_width": transparent.shape[1],
                "exported_height": transparent.shape[0],
            }
        return meta
