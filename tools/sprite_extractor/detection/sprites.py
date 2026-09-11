"""Connected-component analysis used for: tight sprite bounding boxes,
text-vs-sprite classification, and multi-sprite-per-cell clustering."""

from __future__ import annotations

from dataclasses import dataclass

import cv2
import numpy as np

from config import TextFilterConfig
from utils.geometry import Rect


@dataclass
class Component:
    rect: Rect
    area: int


def connected_components(mask: np.ndarray, dilate_kernel: int = 1) -> list[Component]:
    """Foreground boolean mask -> list of connected components (8-connectivity).
    `dilate_kernel` > 1 fuses nearby strokes (e.g. an eye's pupil + outline)
    into one component before labeling."""
    m = mask.astype(np.uint8) * 255
    if dilate_kernel > 1:
        kernel = np.ones((dilate_kernel, dilate_kernel), np.uint8)
        m = cv2.dilate(m, kernel, iterations=1)
    n, labels, stats, _ = cv2.connectedComponentsWithStats(m, connectivity=8)
    comps = []
    for i in range(1, n):  # skip background label 0
        x, y, w, h, area = stats[i]
        comps.append(Component(Rect(int(x), int(y), int(w), int(h)), int(area)))
    return comps


def bbox_of_mask(mask: np.ndarray) -> Rect | None:
    ys, xs = np.where(mask)
    if len(xs) == 0:
        return None
    x0, x1 = int(xs.min()), int(xs.max()) + 1
    y0, y1 = int(ys.min()), int(ys.max()) + 1
    return Rect(x0, y0, x1 - x0, y1 - y0)


def looks_like_text(mask_region: np.ndarray, cfg: TextFilterConfig) -> bool:
    """Heuristic (no OCR): text is many small, similarly-short components
    roughly aligned into horizontal rows - glyphs of a title/subtitle/legend/
    label line. A sprite is typically one (or a couple of) larger, taller
    blob(s) not sharing a row with many peers."""
    h, w = mask_region.shape
    if h == 0 or w == 0:
        return False
    comps = connected_components(mask_region, dilate_kernel=2)
    comps = [c for c in comps if c.area >= 2]
    if len(comps) < cfg.min_components_for_text:
        return False

    # A region dominated by one substantial blob is a real sprite (a solid
    # silhouette, a thin curved stroke that's still one connected shape, a
    # low-contrast edge that fragmented into a few pieces) even if it also
    # has a handful of small aligned specks - text never has a blob this
    # large relative to the region.
    total_area = mask_region.shape[0] * mask_region.shape[1]
    if comps and max(c.area for c in comps) >= cfg.dominant_blob_area_fraction * total_area:
        return False

    max_glyph_h = cfg.max_glyph_height_fraction * h
    small = [c for c in comps if c.rect.h <= max_glyph_h and c.rect.w <= max_glyph_h * cfg.max_component_aspect_for_text]
    if len(small) < cfg.min_components_for_text:
        return False

    # bucket by row (component vertical center) and check that a large
    # fraction of the small components share a handful of rows -> text lines
    centers = sorted(c.rect.cy for c in small)
    row_groups = 0
    used = [False] * len(centers)
    aligned_count = 0
    i = 0
    while i < len(centers):
        if used[i]:
            i += 1
            continue
        group = [centers[i]]
        used[i] = True
        for j in range(i + 1, len(centers)):
            if used[j]:
                continue
            if abs(centers[j] - centers[i]) <= cfg.row_alignment_tolerance_px:
                group.append(centers[j])
                used[j] = True
        if len(group) >= 3:
            aligned_count += len(group)
        row_groups += 1
        i += 1

    return (aligned_count / len(small)) >= cfg.min_row_fill_ratio if small else False


def horizontal_clusters(mask: np.ndarray, dilate_kernel: int, min_gap: int) -> list[Rect]:
    """Group foreground content into horizontally-separated clusters (used to
    split a grid cell that actually contains N side-by-side sprites). Returns
    cluster bounding boxes sorted left-to-right, in `mask`'s own coordinates.
    """
    comps = connected_components(mask, dilate_kernel=dilate_kernel)
    comps = [c for c in comps if c.area >= 1]
    if not comps:
        return []
    comps.sort(key=lambda c: c.rect.x0)

    clusters: list[list[Component]] = [[comps[0]]]
    for c in comps[1:]:
        last = clusters[-1][-1]
        gap = c.rect.x0 - last.rect.x1
        if gap > min_gap:
            clusters.append([c])
        else:
            clusters[-1].append(c)

    boxes = []
    for group in clusters:
        x0 = min(c.rect.x0 for c in group)
        y0 = min(c.rect.y0 for c in group)
        x1 = max(c.rect.x1 for c in group)
        y1 = max(c.rect.y1 for c in group)
        boxes.append(Rect(x0, y0, x1 - x0, y1 - y0))
    return boxes
