"""Grid detection: locates row/column cell boundaries inside a panel/
sub-panel body, and strips the title/subtitle header text band above it.

Two complementary signals are used:

- **Edge-line boundaries** (primary): a cell grid's borders are thin,
  mostly-continuous lines - Canny edge pixels are dense along them and
  sparse everywhere else, including inside cells whose content bleeds all
  the way to the cell edge (bright glow/particle effects) and would leave
  *no* real background gap for a plain density-valley method to find. Real
  boundaries are recovered as peaks in the per-column (or per-row) edge
  pixel count, then fit to a uniform pitch so a boundary occluded/broken at
  one spot (content slightly overlapping the border) is still recovered
  from its neighbors' consistent spacing.
- **Foreground-density valleys** (fallback): for a body with genuine
  whitespace gaps between cells but no drawn border at all, projection-
  profile valleys (normalized against the profile's own high percentile, so
  one relative threshold works across panels of very different size/content)
  still find the split. Used only when the edge-line signal comes back
  empty for a given axis.

Neither method assumes a fixed N x M layout - both discover bands from the
image itself, tolerant of irregular spacing.
"""

from __future__ import annotations

from dataclasses import dataclass

import cv2
import numpy as np

from config import GridDetectionConfig, HeaderStripConfig
from utils.geometry import Rect, split_bands


# ---------------------------------------------------------------------------
# shared: normalized projection-profile density (fallback path + header strip)
# ---------------------------------------------------------------------------


def _min_filter1d(a: np.ndarray, k: int) -> np.ndarray:
    """Sliding-window minimum - bridges single-pixel bright specks
    (antialiasing/glow) that would otherwise fragment one real gap into
    pieces shorter than min_gap and cause it to be missed entirely."""
    if k <= 1 or a.size == 0:
        return a
    r = k // 2
    padded = np.pad(a, (r, r), mode="edge")
    out = np.empty_like(a)
    for i in range(a.shape[0]):
        out[i] = padded[i : i + k].min()
    return out


def _normalized_density(sums: np.ndarray, percentile: float) -> np.ndarray:
    """Normalize a 1D projection profile by a high percentile of itself
    (rather than by the region's raw pixel span), so one relative
    valley_density_threshold works across panels of very different size,
    border thickness, and content color."""
    if sums.size == 0:
        return sums
    ref = np.percentile(sums, percentile)
    if ref <= 0:
        ref = float(sums.max()) or 1.0
    return sums / ref


def _row_density(mask: np.ndarray, percentile: float, smoothing_window: int = 3) -> np.ndarray:
    sums = mask.sum(axis=1).astype(np.float64)
    return _min_filter1d(_normalized_density(sums, percentile), smoothing_window)


def _col_density(mask: np.ndarray, percentile: float, smoothing_window: int = 3) -> np.ndarray:
    sums = mask.sum(axis=0).astype(np.float64)
    return _min_filter1d(_normalized_density(sums, percentile), smoothing_window)


def strip_header(mask: np.ndarray, cfg: HeaderStripConfig) -> int:
    """Return the y offset (relative to `mask`) where the panel body starts,
    i.e. the row just after the title/subtitle text block at the top.

    The title+subtitle text block does not reliably band-split as a single
    clean run (a bold title, a thinner italic subtitle, and the small gap
    between the two lines can each register as their own short band even
    after smoothing), so this doesn't assume "the header is exactly the
    first band". Instead: band-split the whole row density profile, always
    treat the very first band as (part of) the header - real grid/sprite
    content is never flush against y=0 - and, among the remaining bands,
    return the start of the first one that is substantial (>=
    `min_body_band_fraction` of the region height). Real content bands
    (a full grid row, a single large sprite) are always much taller than an
    individual text-line fragment, so this reliably skips past a header
    that fragmented into several short bands without needing them to merge
    into one first. If nothing qualifies, returns 0 (no header detected)."""
    h = mask.shape[0]
    if h == 0:
        return 0
    density = list(_row_density(mask, cfg.density_percentile))
    min_gap = cfg.min_gap_px
    if isinstance(min_gap, str):
        min_gap = max(2, int(h * cfg.min_gap_fraction_of_height))
    else:
        min_gap = int(min_gap)

    bands = split_bands(density, cfg.valley_density_threshold, min_gap, min_band=1)
    if len(bands) < 2:
        return 0
    max_header = int(h * cfg.max_header_fraction_of_height)
    min_body_size = cfg.min_body_band_fraction * h
    for start, end in bands[1:]:
        if start > max_header:
            break
        if (end - start) >= min_body_size:
            return start
    return 0


# ---------------------------------------------------------------------------
# primary: edge-line boundary detection + uniform-pitch fitting
# ---------------------------------------------------------------------------


def _edge_peak_candidates(edges: np.ndarray, axis: int, perpendicular: int, cfg: GridDetectionConfig) -> list[int]:
    """Positions along `axis` (0=row boundaries scanning down, 1=col
    boundaries scanning across) whose Canny edge-pixel count is both a
    local maximum and tall enough (relative to the perpendicular extent) to
    plausibly be a cell-grid border line."""
    sums = edges.sum(axis=1 - axis).astype(np.float64) / 255.0
    n = sums.shape[0]
    thresh = cfg.edge_peak_fraction * perpendicular
    window = cfg.edge_peak_window
    raw = []
    for i in range(n):
        if sums[i] < thresh:
            continue
        lo, hi = max(0, i - window), min(n, i + window + 1)
        if sums[i] >= sums[lo:hi].max() - 1e-9:
            raw.append(i)
    # merge adjacent/near-duplicate peaks (a >1px-thick line produces
    # several consecutive/close local maxima) into one boundary position
    merged: list[list[int]] = []
    for p in raw:
        if merged and p - merged[-1][-1] <= cfg.edge_peak_merge_px:
            merged[-1].append(p)
        else:
            merged.append([p])
    return [int(round(sum(g) / len(g))) for g in merged]


def _fit_uniform_bands(candidates: list[int], n: int, cfg: GridDetectionConfig) -> list[tuple[int, int]]:
    """Turn boundary candidates into row/col bands. Candidates are fit to a
    uniform pitch (the median spacing between consecutive candidates) so a
    boundary missed at one spot (occluded by content) is recovered from its
    neighbors, then any resulting sliver band - typically leading/trailing
    panel padding rather than a real cell - is merged into its nearest
    neighbor band.
    """
    pts = sorted(set([0] + candidates + [n]))
    clean = [pts[0]]
    for x in pts[1:]:
        if x - clean[-1] > cfg.edge_peak_merge_px:
            clean.append(x)
    if len(clean) < 3:
        return []

    gaps = [b - a for a, b in zip(clean, clean[1:])]
    gaps_sorted = sorted(gaps)
    pitch = gaps_sorted[len(gaps_sorted) // 2]
    if pitch <= 0:
        return []

    bounds = [clean[0]]
    x = clean[0]
    while x < clean[-1] - pitch * 0.5:
        target = x + pitch
        best = min(clean, key=lambda c: abs(c - target))
        if abs(best - target) > pitch * cfg.edge_pitch_snap_tolerance:
            best = int(round(target))
        if best <= bounds[-1]:
            best = bounds[-1] + max(1, int(pitch * 0.5))
        bounds.append(best)
        x = bounds[-1]
    if bounds[-1] < clean[-1] - pitch * (1 - cfg.edge_pitch_snap_tolerance):
        bounds.append(clean[-1])
    else:
        bounds[-1] = clean[-1]

    bands = [(a, b) for a, b in zip(bounds, bounds[1:]) if b > a]
    if len(bands) < 2:
        return bands

    # merge slivers (leading/trailing padding, stray extra boundary) into
    # whichever neighbor they are closer to
    sizes = sorted(b - a for a, b in bands)
    median_size = sizes[len(sizes) // 2]
    changed = True
    while changed and len(bands) > 1:
        changed = False
        for i, (a, b) in enumerate(bands):
            if (b - a) >= median_size * cfg.min_band_ratio_of_median:
                continue
            if i == 0:
                bands[1] = (a, bands[1][1])
            elif i == len(bands) - 1:
                bands[i - 1] = (bands[i - 1][0], b)
            else:
                # merge into the neighbor sharing the closer boundary
                left_gap = a - bands[i - 1][0]
                right_gap = bands[i + 1][1] - b
                if left_gap <= right_gap:
                    bands[i - 1] = (bands[i - 1][0], b)
                else:
                    bands[i + 1] = (a, bands[i + 1][1])
            del bands[i]
            changed = True
            break
    return bands


def _edge_bands(gray_region: np.ndarray, axis: int, cfg: GridDetectionConfig) -> list[tuple[int, int]]:
    h, w = gray_region.shape
    if h == 0 or w == 0:
        return []
    edges = cv2.Canny(gray_region, cfg.edge_canny_low, cfg.edge_canny_high)
    n = h if axis == 0 else w
    perpendicular = w if axis == 0 else h
    candidates = _edge_peak_candidates(edges, axis, perpendicular, cfg)
    return _fit_uniform_bands(candidates, n, cfg)


# ---------------------------------------------------------------------------
# public API
# ---------------------------------------------------------------------------


@dataclass
class GridResult:
    row_bands: list[tuple[int, int]]
    col_bands: list[tuple[int, int]]

    @property
    def is_grid(self) -> bool:
        return len(self.row_bands) >= 1 and len(self.col_bands) >= 1 and (
            len(self.row_bands) * len(self.col_bands) >= 2
        )


def detect_grid(mask: np.ndarray, cfg: GridDetectionConfig, gray_region: np.ndarray | None = None) -> GridResult:
    """Detect row/col cell bands for the body region described by `mask`
    (foreground/background boolean mask) and, when available, `gray_region`
    (same region, grayscale - enables the edge-line primary method; without
    it, only the density-valley fallback runs)."""
    h, w = mask.shape
    if h == 0 or w == 0:
        return GridResult([], [])

    row_bands: list[tuple[int, int]] = []
    col_bands: list[tuple[int, int]] = []
    if gray_region is not None:
        row_bands = _edge_bands(gray_region, axis=0, cfg=cfg)
        col_bands = _edge_bands(gray_region, axis=1, cfg=cfg)

    if len(row_bands) < 2:
        row_bands = _fallback_bands(mask, axis=0, cfg=cfg)
    if len(col_bands) < 2:
        col_bands = _fallback_bands(mask, axis=1, cfg=cfg)

    return GridResult(row_bands, col_bands)


def _fallback_bands(mask: np.ndarray, axis: int, cfg: GridDetectionConfig) -> list[tuple[int, int]]:
    h, w = mask.shape
    density = _row_density(mask, cfg.density_percentile) if axis == 0 else _col_density(mask, cfg.density_percentile)
    dim = h if axis == 0 else w
    gap = cfg.min_gap_px if not isinstance(cfg.min_gap_px, str) else max(1, int(dim * cfg.min_gap_fraction))
    min_band = cfg.min_band_px if not isinstance(cfg.min_band_px, str) else max(3, int(dim * cfg.min_band_fraction))
    return split_bands(list(density), cfg.valley_density_threshold, int(gap), int(min_band))


def uniform_bands_within(auto_bands: list[tuple[int, int]], n: int, count: int) -> list[tuple[int, int]]:
    """Like `uniform_bands`, but spans only the auto-detected content extent
    (the union of `auto_bands`) rather than the full [0, n) region, when
    that's available. A forced row/col *count* override should still
    respect where the auto-detector found real content starting/ending -
    e.g. CORPO BASE's row-label column sits inside the panel body to the
    left of the actual per-direction grid, and blindly re-slicing the full
    body width into N equal pieces would let the labels bleed into the
    first column. Falls back to the full [0, n) span when no bands were
    auto-detected at all."""
    if not auto_bands:
        return uniform_bands(n, count)
    start = min(a for a, _ in auto_bands)
    end = max(b for _, b in auto_bands)
    span = end - start
    return [(a + start, b + start) for a, b in uniform_bands(span, count)]


def uniform_bands(n: int, count: int) -> list[tuple[int, int]]:
    """Split [0, n) into `count` equal-ish bands. Used for a manual
    `rows`/`columns` config override (e.g. `detection.panel_overrides`) on a
    panel whose grid structure is known but not reliably auto-detectable -
    the "type: grid, rows: N, columns: M" case the spec calls for."""
    if count <= 0 or n <= 0:
        return []
    edges = [round(i * n / count) for i in range(count + 1)]
    return [(edges[i], edges[i + 1]) for i in range(count) if edges[i + 1] > edges[i]]


def bands_are_regular(bands: list[tuple[int, int]], tolerance: float) -> bool:
    if len(bands) < 2:
        return True
    sizes = [b - a for a, b in bands]
    mean = sum(sizes) / len(sizes)
    if mean <= 0:
        return False
    spread = max(abs(s - mean) for s in sizes) / mean
    return spread <= tolerance


def cells_from_bands(row_bands: list[tuple[int, int]], col_bands: list[tuple[int, int]], origin: Rect) -> list[list[Rect]]:
    """Row-major 2D list of cell Rects in parent-image coordinates."""
    grid: list[list[Rect]] = []
    for (ry0, ry1) in row_bands:
        row: list[Rect] = []
        for (cx0, cx1) in col_bands:
            row.append(Rect(origin.x0 + cx0, origin.y0 + ry0, cx1 - cx0, ry1 - ry0))
        grid.append(row)
    return grid
