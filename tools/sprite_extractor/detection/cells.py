"""Per-cell sprite extraction: tight bbox + optional multi-sprite split."""

from __future__ import annotations

import numpy as np

from config import SubsplitConfig
from detection.sprites import bbox_of_mask, horizontal_clusters
from utils.geometry import Rect


def split_cell_sprites(cell_mask: np.ndarray, cell_origin: Rect, cfg: SubsplitConfig) -> list[Rect]:
    """Given a cell's foreground mask (cell-local coordinates), return one or
    more sprite bboxes in parent-image coordinates. Splits into multiple
    sprites when the content forms >=2 well-separated horizontal clusters
    that are each a plausible sprite size (e.g. CORPO BASE's two side-by-side
    characters per labeled cell). Falls back to a single tight bbox.
    """
    h, w = cell_mask.shape
    if not cfg.enabled:
        bbox = bbox_of_mask(cell_mask)
        return [bbox.offset(cell_origin.x0, cell_origin.y0)] if bbox else []

    min_gap = max(1, int(w * cfg.min_gap_fraction_of_cell))
    clusters = horizontal_clusters(cell_mask, cfg.dilate_kernel, min_gap)

    min_cluster_area = cfg.min_cluster_fraction_of_cell * w * h
    clusters = [c for c in clusters if c.area >= min_cluster_area]

    if len(clusters) <= 1:
        bbox = bbox_of_mask(cell_mask)
        return [bbox.offset(cell_origin.x0, cell_origin.y0)] if bbox else []

    return [c.offset(cell_origin.x0, cell_origin.y0) for c in clusters]
