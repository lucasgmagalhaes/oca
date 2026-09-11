"""Resolves detection results (panel index, optional OCR'd title) against the
config's manual `panel_overrides` / `exclude_panels`, and supplies category
labels used for output folder naming.

Matching order: an explicit `title_contains` match (only possible when OCR is
enabled and available) wins; otherwise `panel_index` (the panel's 0-based
position in reading order, which is deterministic for a given detected
layout) is used. This lets a config author write overrides either way, and
keeps everything working with OCR fully disabled (the default).
"""

from __future__ import annotations

from config import ExcludePanel, PanelOverride, SpriteExtractorConfig


def find_override(cfg: SpriteExtractorConfig, panel_index: int, title: str | None) -> PanelOverride | None:
    for ov in cfg.detection.panel_overrides:
        if title and ov.title_contains and ov.title_contains.lower() in title.lower():
            return ov
    for ov in cfg.detection.panel_overrides:
        if ov.panel_index is not None and ov.panel_index == panel_index:
            return ov
    return None


def is_excluded(cfg: SpriteExtractorConfig, panel_index: int, title: str | None) -> tuple[bool, str]:
    for ex in cfg.detection.exclude_panels:
        if title and ex.title_contains and ex.title_contains.lower() in title.lower():
            return True, ex.reason
    for ex in cfg.detection.exclude_panels:
        if ex.panel_index is not None and ex.panel_index == panel_index:
            return True, ex.reason
    return False, ""


def default_category(panel_index: int) -> str:
    return f"panel_{panel_index:02d}"
