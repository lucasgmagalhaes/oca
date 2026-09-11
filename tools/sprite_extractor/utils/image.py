"""Thin image I/O and array-conversion helpers (Pillow + numpy + cv2)."""

from __future__ import annotations

import numpy as np
from PIL import Image

from .geometry import Rect


def load_rgba(path: str) -> np.ndarray:
    """Load any image as an HxWx4 uint8 RGBA numpy array."""
    img = Image.open(path).convert("RGBA")
    return np.array(img)


def save_rgba(arr: np.ndarray, path: str) -> None:
    if arr.shape[2] == 3:
        arr = np.dstack([arr, np.full(arr.shape[:2], 255, dtype=np.uint8)])
    Image.fromarray(arr, mode="RGBA").save(path)


def to_gray(rgba: np.ndarray) -> np.ndarray:
    """RGB(A) -> single-channel uint8 luminance, ignoring alpha."""
    rgb = rgba[:, :, :3].astype(np.float32)
    gray = 0.299 * rgb[:, :, 0] + 0.587 * rgb[:, :, 1] + 0.114 * rgb[:, :, 2]
    return gray.astype(np.uint8)


def crop(arr: np.ndarray, rect: Rect) -> np.ndarray:
    bounds = Rect(0, 0, arr.shape[1], arr.shape[0])
    r = rect.clip(bounds)
    return arr[r.y0 : r.y1, r.x0 : r.x1]


def image_bounds(arr: np.ndarray) -> Rect:
    return Rect(0, 0, arr.shape[1], arr.shape[0])


def estimate_background_color(rgba: np.ndarray, sample_border_px: int = 6) -> tuple[int, int, int]:
    """Sample the four image corners/edges to estimate the dominant page
    background color, used as the reference for foreground-content masks.
    Generic (no hardcoded color) so it adapts to future sheets/themes.
    """
    h, w = rgba.shape[:2]
    b = min(sample_border_px, h // 4 or 1, w // 4 or 1)
    strips = [
        rgba[0:b, :, :3].reshape(-1, 3),
        rgba[h - b : h, :, :3].reshape(-1, 3),
        rgba[:, 0:b, :3].reshape(-1, 3),
        rgba[:, w - b : w, :3].reshape(-1, 3),
    ]
    pixels = np.concatenate(strips, axis=0)
    # Mode via rounding to reduce noise from antialiasing, then pick the most
    # common color bucket.
    colors, counts = np.unique(pixels, axis=0, return_counts=True)
    dominant = colors[np.argmax(counts)]
    return int(dominant[0]), int(dominant[1]), int(dominant[2])


def foreground_mask(rgba: np.ndarray, bg_color: tuple[int, int, int], tolerance: int) -> np.ndarray:
    """Boolean mask of pixels that differ from `bg_color` by more than
    `tolerance` (L1 distance on RGB). Cheap structural mask used for
    panel/grid/text detection - NOT the final transparency mechanism (see
    processing/background.py for the flood-fill based one used at export time).
    """
    rgb = rgba[:, :, :3].astype(np.int32)
    bg = np.array(bg_color, dtype=np.int32)
    diff = np.abs(rgb - bg).sum(axis=2)
    mask = diff > tolerance
    if rgba.shape[2] == 4:
        mask &= rgba[:, :, 3] > 8
    return mask
