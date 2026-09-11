"""High-level transparency pipeline: flood-fill background removal +
alpha clean-up, applied to a cropped region before trimming/export."""

from __future__ import annotations

import numpy as np

from config import BackgroundConfig
from processing.background import apply_background_removal


def make_transparent(region_rgba: np.ndarray, cfg: BackgroundConfig) -> np.ndarray:
    """Flood-fill the region's background to transparent. Isolated
    background-colored specks fully enclosed by foreground (e.g. a bright
    highlight surrounded by an outline) are correctly kept opaque because
    flood fill only clears pixels *reachable* from the border.
    """
    return apply_background_removal(region_rgba, cfg.flood_fill_tolerance)


def alpha_coverage(rgba: np.ndarray, alpha_threshold: int) -> float:
    """Fraction of pixels considered "content" (alpha above threshold)."""
    if rgba.size == 0:
        return 0.0
    a = rgba[:, :, 3]
    return float((a > alpha_threshold).sum()) / a.size
