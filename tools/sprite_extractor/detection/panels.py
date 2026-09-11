"""Generic panel (bordered rounded-rect region) detection via edge/contour
analysis - no hardcoded coordinates. Works on the outer page (finds the ~13
top-level panels) and, recursively, inside a panel's body (finds bordered
sub-panels such as ROUPAS' TORSO/BRAÇOS/PERNAS or CORPO BASE's per-direction
boxes).
"""

from __future__ import annotations

import cv2
import numpy as np

from config import PanelDetectionConfig
from utils.geometry import Rect, group_rows, merge_overlapping


def _contour_boxes(gray: np.ndarray, cfg: PanelDetectionConfig, min_area_fraction: float | None = None) -> list[Rect]:
    h, w = gray.shape
    edges = cv2.Canny(gray, cfg.canny_low, cfg.canny_high)
    if cfg.dilate_kernel > 1:
        kernel = np.ones((cfg.dilate_kernel, cfg.dilate_kernel), np.uint8)
        edges = cv2.dilate(edges, kernel, iterations=1)
    contours, _ = cv2.findContours(edges, cv2.RETR_LIST, cv2.CHAIN_APPROX_SIMPLE)

    img_area = w * h
    min_frac = cfg.min_area_fraction if min_area_fraction is None else min_area_fraction
    boxes: list[Rect] = []
    for c in contours:
        x, y, cw, ch = cv2.boundingRect(c)
        area = cw * ch
        if area < min_frac * img_area:
            continue
        if area > cfg.max_area_fraction * img_area:
            continue
        if cw < cfg.min_width or ch < cfg.min_height:
            continue
        boxes.append(Rect(x, y, cw, ch))
    return boxes


def detect_panels(gray: np.ndarray, cfg: PanelDetectionConfig) -> list[Rect]:
    """Detect top-level bordered panels on the full page, deduplicated and
    returned in reading order (row-major, top-to-bottom then left-to-right).
    """
    boxes = _contour_boxes(gray, cfg)
    merged = merge_overlapping(boxes, cfg.merge_iou, cfg.merge_contain_ratio)
    rows = group_rows(merged, cfg.row_overlap_ratio)
    ordered: list[Rect] = [r for row in rows for r in row]
    return ordered


def detect_subpanels(
    gray_region: np.ndarray,
    origin: Rect,
    cfg: PanelDetectionConfig,
    min_area_fraction: float = 0.06,
) -> list[Rect]:
    """Detect bordered sub-panels nested inside a panel's body (e.g. ROUPAS'
    TORSO/BRAÇOS/PERNAS boxes, or CORPO BASE's 4 per-direction boxes).
    `min_area_fraction` is relative to the body region itself and is
    deliberately much higher than `cfg.panels.min_area_fraction` (which is
    tuned for whole-page panels): a true sub-panel tiles a large slice of
    its parent, while an individual grid cell border - which also produces a
    bordered-rect contour - covers only a couple of percent of the parent and
    must not be mistaken for a sub-panel.
    Boxes that are (near) the full size of the region itself are dropped
    (that's just the region's own outer border re-detected, not a nested
    sub-panel). Returned rects are in the *parent image's* coordinate space
    (offset by `origin`)."""
    boxes = _contour_boxes(gray_region, cfg, min_area_fraction=min_area_fraction)
    merged = merge_overlapping(boxes, cfg.merge_iou, cfg.merge_contain_ratio)

    h, w = gray_region.shape
    region_area = w * h
    sub = [b for b in merged if b.area < 0.9 * region_area]
    rows = group_rows(sub, cfg.row_overlap_ratio)
    ordered = [r for row in rows for r in row]
    return [r.offset(origin.x0, origin.y0) for r in ordered]
