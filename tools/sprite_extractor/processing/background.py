"""Background/transparency removal via flood fill from the region border,
inward - NOT a global "delete pixels near color X" threshold. This preserves
dark outlines, hair strands, shadows and antialiasing that a naive color-key
would eat into, because only pixels *connected* to the border and similar in
color to it are cleared.
"""

from __future__ import annotations

import cv2
import numpy as np


def flood_fill_background_alpha(rgb: np.ndarray, tolerance: int, seed_points: list[tuple[int, int]] | None = None) -> np.ndarray:
    """Return a uint8 alpha mask (255 = keep, 0 = background) for `rgb`
    (HxWx3), computed by flood-filling from the region's border pixels
    inward, tolerant of `tolerance` per-channel color drift. Multiple seeds
    (default: all four corners + edge midpoints) so a border that isn't a
    single flat color still floods correctly.
    """
    h, w = rgb.shape[:2]
    if h == 0 or w == 0:
        return np.zeros((h, w), dtype=np.uint8)

    bgr = cv2.cvtColor(rgb, cv2.COLOR_RGB2BGR)
    background = np.zeros((h, w), dtype=np.uint8)

    if seed_points is None:
        seed_points = [
            (0, 0),
            (w - 1, 0),
            (0, h - 1),
            (w - 1, h - 1),
            (w // 2, 0),
            (w // 2, h - 1),
            (0, h // 2),
            (w - 1, h // 2),
        ]

    flood_mask = np.zeros((h + 2, w + 2), dtype=np.uint8)
    filled_any = np.zeros((h, w), dtype=np.uint8)
    for (sx, sy) in seed_points:
        if filled_any[sy, sx]:
            continue
        work = bgr.copy()
        local_mask = np.zeros((h + 2, w + 2), dtype=np.uint8)
        cv2.floodFill(
            work,
            local_mask,
            (sx, sy),
            (0, 0, 0),
            loDiff=(tolerance, tolerance, tolerance),
            upDiff=(tolerance, tolerance, tolerance),
            flags=4 | cv2.FLOODFILL_MASK_ONLY | (255 << 8),
        )
        region = local_mask[1:-1, 1:-1] > 0
        filled_any |= region.astype(np.uint8)

    background = filled_any * 255
    alpha = np.where(background > 0, 0, 255).astype(np.uint8)
    return alpha


def apply_background_removal(rgba: np.ndarray, tolerance: int) -> np.ndarray:
    """Return a copy of `rgba` with the background flood-filled to
    transparent, combined (min) with any pre-existing alpha channel."""
    rgb = rgba[:, :, :3]
    new_alpha = flood_fill_background_alpha(rgb, tolerance)
    out = rgba.copy()
    if rgba.shape[2] == 4:
        out[:, :, 3] = np.minimum(rgba[:, :, 3], new_alpha)
    else:
        out = np.dstack([rgb, new_alpha])
    return out
