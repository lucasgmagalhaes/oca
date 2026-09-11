"""Tight-bbox cropping (trimmed export mode) and padding."""

from __future__ import annotations

import numpy as np

from utils.geometry import Rect


def bbox_from_alpha(rgba: np.ndarray, alpha_threshold: int) -> Rect | None:
    alpha = rgba[:, :, 3]
    ys, xs = np.where(alpha > alpha_threshold)
    if len(xs) == 0:
        return None
    x0, x1 = int(xs.min()), int(xs.max()) + 1
    y0, y1 = int(ys.min()), int(ys.max()) + 1
    return Rect(x0, y0, x1 - x0, y1 - y0)


def trim(rgba: np.ndarray, padding: int, alpha_threshold: int) -> tuple[np.ndarray, Rect] | None:
    """Crop `rgba` tightly to its non-transparent content plus `padding` px.
    Returns (cropped_array, bbox_in_source_coords) or None if the region is
    fully transparent."""
    bounds = Rect(0, 0, rgba.shape[1], rgba.shape[0])
    bbox = bbox_from_alpha(rgba, alpha_threshold)
    if bbox is None:
        return None
    padded = bbox.expand(padding, bounds)
    cropped = rgba[padded.y0 : padded.y1, padded.x0 : padded.x1]
    return cropped, padded
