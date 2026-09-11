"""Contextual small-asset filtering. Deliberately NOT a single flat
`area < N` threshold - small/thin real assets (eyes, eyebrows, particles,
thin accessory straps) must survive, so the minimum viable area is always
computed relative to the region it was found in (a cell or a panel body),
never as one global magic number."""

from __future__ import annotations

from config import SpriteConfig
from utils.geometry import Rect


def min_area_for(cell_or_region: Rect, cfg: SpriteConfig) -> float:
    if not isinstance(cfg.min_area, str):
        return float(cfg.min_area)
    return cfg.min_area_fraction_of_cell * cell_or_region.area


def is_plausible_sprite(bbox: Rect, region: Rect, cfg: SpriteConfig) -> bool:
    if bbox.area <= 0:
        return False
    return bbox.area >= min_area_for(region, cfg)
