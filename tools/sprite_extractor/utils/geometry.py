"""Small geometry primitives shared across detection/processing/output.

Kept dependency-free (no numpy/cv2) so it can be unit tested in isolation.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Rect:
    """An axis-aligned integer rectangle, top-left origin, width/height in px."""

    x: int
    y: int
    w: int
    h: int

    @property
    def x0(self) -> int:
        return self.x

    @property
    def y0(self) -> int:
        return self.y

    @property
    def x1(self) -> int:
        return self.x + self.w

    @property
    def y1(self) -> int:
        return self.y + self.h

    @property
    def area(self) -> int:
        return max(0, self.w) * max(0, self.h)

    @property
    def cx(self) -> float:
        return self.x + self.w / 2

    @property
    def cy(self) -> float:
        return self.y + self.h / 2

    def as_tuple(self) -> tuple[int, int, int, int]:
        return (self.x, self.y, self.w, self.h)

    def as_dict(self) -> dict:
        return {"x": self.x, "y": self.y, "width": self.w, "height": self.h}

    def expand(self, pad: int, bounds: "Rect | None" = None) -> "Rect":
        """Grow the rect by `pad` px on every side, clipped to `bounds` if given."""
        x0, y0, x1, y1 = self.x0 - pad, self.y0 - pad, self.x1 + pad, self.y1 + pad
        if bounds is not None:
            x0 = max(bounds.x0, x0)
            y0 = max(bounds.y0, y0)
            x1 = min(bounds.x1, x1)
            y1 = min(bounds.y1, y1)
        return Rect(x0, y0, max(0, x1 - x0), max(0, y1 - y0))

    def offset(self, dx: int, dy: int) -> "Rect":
        return Rect(self.x + dx, self.y + dy, self.w, self.h)

    def clip(self, bounds: "Rect") -> "Rect":
        x0 = max(self.x0, bounds.x0)
        y0 = max(self.y0, bounds.y0)
        x1 = min(self.x1, bounds.x1)
        y1 = min(self.y1, bounds.y1)
        return Rect(x0, y0, max(0, x1 - x0), max(0, y1 - y0))

    def intersection(self, other: "Rect") -> "Rect":
        return self.clip(other)

    def intersects(self, other: "Rect") -> bool:
        return self.intersection(other).area > 0

    def contains(self, other: "Rect") -> bool:
        return (
            self.x0 <= other.x0
            and self.y0 <= other.y0
            and self.x1 >= other.x1
            and self.y1 >= other.y1
        )

    def iou(self, other: "Rect") -> float:
        inter = self.intersection(other).area
        if inter == 0:
            return 0.0
        union = self.area + other.area - inter
        return inter / union if union else 0.0

    def contains_ratio(self, other: "Rect") -> float:
        """Fraction of `other`'s area covered by self. Useful for near-duplicate merges."""
        if other.area == 0:
            return 0.0
        return self.intersection(other).area / other.area


def union_rect(rects: list[Rect]) -> Rect:
    if not rects:
        return Rect(0, 0, 0, 0)
    x0 = min(r.x0 for r in rects)
    y0 = min(r.y0 for r in rects)
    x1 = max(r.x1 for r in rects)
    y1 = max(r.y1 for r in rects)
    return Rect(x0, y0, x1 - x0, y1 - y0)


def merge_overlapping(rects: list[Rect], iou_threshold: float = 0.3, contain_threshold: float = 0.85) -> list[Rect]:
    """Greedily collapse near-duplicate/nested rects (e.g. double-line borders).

    Sorted by area descending; a rect is dropped if it is already covered by
    (IoU or containment) a previously kept, larger rect.
    """
    ordered = sorted(rects, key=lambda r: -r.area)
    kept: list[Rect] = []
    for r in ordered:
        dup = False
        for k in kept:
            if k.iou(r) >= iou_threshold or k.contains_ratio(r) >= contain_threshold:
                dup = True
                break
        if not dup:
            kept.append(r)
    return kept


def group_rows(rects: list[Rect], overlap_ratio: float = 0.5) -> list[list[Rect]]:
    """Cluster rects into reading-order rows by vertical (y) overlap, then sort
    each row left-to-right. Rows are returned top-to-bottom."""
    remaining = sorted(rects, key=lambda r: r.y0)
    rows: list[list[Rect]] = []
    for r in remaining:
        placed = False
        for row in rows:
            ref = row[0]
            top = max(ref.y0, r.y0)
            bot = min(ref.y1, r.y1)
            overlap = max(0, bot - top)
            min_h = min(ref.h, r.h) or 1
            if overlap / min_h >= overlap_ratio:
                row.append(r)
                placed = True
                break
        if not placed:
            rows.append([r])
    rows.sort(key=lambda row: min(r.y0 for r in row))
    for row in rows:
        row.sort(key=lambda r: r.x0)
    return rows


def split_bands(density: list[float], valley_threshold: float, min_gap: int, min_band: int) -> list[tuple[int, int]]:
    """Given a 1D density profile (already normalized 0..1), return contiguous
    "active" bands (start, end) separated by valleys (>= min_gap consecutive
    values below valley_threshold). Bands shorter than min_band are dropped."""
    n = len(density)
    is_valley = [d <= valley_threshold for d in density]

    # find runs of valley
    valley_runs: list[tuple[int, int]] = []
    i = 0
    while i < n:
        if is_valley[i]:
            j = i
            while j < n and is_valley[j]:
                j += 1
            valley_runs.append((i, j))
            i = j
        else:
            i += 1

    cut_points = [0, n]
    for (a, b) in valley_runs:
        if b - a >= min_gap:
            mid = (a + b) // 2
            cut_points.append(mid)
    cut_points = sorted(set(cut_points))

    bands: list[tuple[int, int]] = []
    for k in range(len(cut_points) - 1):
        a, b = cut_points[k], cut_points[k + 1]
        # trim leading/trailing valley from the band
        while a < b and is_valley[a]:
            a += 1
        while b > a and is_valley[b - 1]:
            b -= 1
        if b - a >= min_band:
            bands.append((a, b))
    return bands
