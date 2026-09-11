"""Configuration schema, defaults, and YAML loading/validation.

Every detection threshold lives here (never scattered as magic numbers in
detection/processing code). A threshold may be a concrete number or the
string "auto", in which case a resolver derives it at run time from image/
region size (see `resolve_auto`). Manual overrides for panels that can't be
reliably auto-classified live under `detection.panel_overrides` /
`detection.exclude_panels` / `manual_regions`.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Union

import yaml

AutoOr = Union[str, float, int]  # "auto" or a concrete number


def is_auto(value: AutoOr) -> bool:
    return isinstance(value, str) and value.strip().lower() == "auto"


def resolve_auto(value: AutoOr, default: float) -> float:
    """Return `value` unless it is the sentinel "auto", in which case `default`."""
    return default if is_auto(value) else float(value)


@dataclass
class PanelDetectionConfig:
    canny_low: int = 15
    canny_high: int = 45
    dilate_kernel: int = 3
    min_area_fraction: float = 0.004
    max_area_fraction: float = 0.5
    min_width: int = 50
    min_height: int = 50
    merge_iou: float = 0.3
    merge_contain_ratio: float = 0.85
    row_overlap_ratio: float = 0.4
    body_inset_px: int = 5  # shrink a header-stripped body before sub-detection so the parent's own border isn't mistaken for a nested one
    subpanel_dilate_kernel: int = 1  # lower than `dilate_kernel` - adjacent sub-panels sit close together and a larger kernel bridges their borders into one contour


@dataclass
class HeaderStripConfig:
    """Locates and strips the title/subtitle text band at the top of a panel
    or sub-panel, leaving the "body" (grid or sprite content) below it."""

    valley_density_threshold: float = 0.35
    min_gap_px: AutoOr = "auto"  # auto = fraction of region height
    min_gap_fraction_of_height: float = 0.01
    max_header_fraction_of_height: float = 0.45
    density_percentile: float = 85
    min_body_band_fraction: float = 0.15  # a candidate "body start" band must span at least this fraction of the region height to be trusted over a stray header-text fragment


@dataclass
class GridDetectionConfig:
    valley_density_threshold: float = 0.35
    min_gap_px: AutoOr = "auto"
    min_gap_fraction: float = 0.006  # of body dimension, used when min_gap_px == auto
    min_band_px: AutoOr = "auto"
    min_band_fraction: float = 0.025
    regularity_tolerance: float = 0.5  # relative spread allowed among band sizes
    density_percentile: float = 85

    # Edge-line boundary detection (primary grid-line finder - see
    # detection/grids.py module docstring). Falls back to the density-valley
    # profile above only when this finds fewer than 2 bands on an axis.
    edge_canny_low: int = 15
    edge_canny_high: int = 45
    edge_peak_fraction: float = 0.55  # min fraction of the perpendicular extent an edge run must cover
    edge_peak_window: int = 2  # local-maximum window (px) for peak picking
    edge_peak_merge_px: int = 6  # merge peaks/candidates within this many px into one boundary
    edge_pitch_snap_tolerance: float = 0.35  # fraction of the fitted pitch a candidate may drift and still snap
    min_band_ratio_of_median: float = 0.4  # bands smaller than this ratio of the median band are merged away


@dataclass
class SubsplitConfig:
    """Splits a single grid cell into multiple sprites when its foreground
    content forms >=2 well separated horizontal clusters (e.g. CORPO BASE's
    two side-by-side sprites per labeled cell)."""

    enabled: bool = True
    dilate_kernel: int = 3
    min_gap_fraction_of_cell: float = 0.06
    min_cluster_fraction_of_cell: float = 0.1


@dataclass
class TextFilterConfig:
    """Heuristics used to recognize & exclude text (titles, subtitles,
    headers, legends, info-card copy) without OCR."""

    max_glyph_height_fraction: float = 0.11  # of panel/body height
    min_components_for_text: int = 4
    row_alignment_tolerance_px: int = 6
    max_component_aspect_for_text: float = 6.0
    min_row_fill_ratio: float = 0.35
    dominant_blob_area_fraction: float = 0.1  # fraction of components in a band aligned in a row


@dataclass
class BackgroundConfig:
    color: AutoOr = "auto"
    tolerance: int = 20
    flood_fill_tolerance: int = 18


@dataclass
class SpriteConfig:
    padding: int = 4
    min_area: AutoOr = "auto"
    min_area_fraction_of_cell: float = 0.01
    alpha_threshold: int = 10


@dataclass
class PanelOverride:
    """Manual guidance for one panel, matched by title substring (needs OCR)
    or by its 0-based reading-order index (always available, deterministic
    for a given detected layout)."""

    panel_index: int | None = None
    title_contains: str | None = None
    type: str = "auto"  # auto | grid | special | grid_of_subpanels | explicit
    category: str | None = None
    subpanel_names: list[str] = field(default_factory=list)
    subpanel_categories: list[str] = field(default_factory=list)
    rows: AutoOr = "auto"
    columns: AutoOr = "auto"
    cell_multi_sprite: bool | None = None
    subpanel_naming: str = "indexed"  # "indexed" -> {subcategory}_NNN.png sequential; "frame" -> frame_NNN[_a/_b].png per (row, col)
    column_group_names: list[str] = field(default_factory=list)  # e.g. [front, back, left, right]: groups a plain grid's columns into named sub-folders (frame_NNN[_a/_b].png each) instead of detecting bordered sub-panels - a more robust alternative when sub-panel borders are hard to isolate but the full fine-grained cell grid detects cleanly
    body_inset_left: int = 0  # extra left-edge crop (px) applied to the header-stripped body before grid detection - for content that sits inside the panel body but outside the actual sprite grid (e.g. CORPO BASE's row labels), which has no border of its own for auto-detection to key off
    subpanel_header_px: int = 0  # fixed fallback header height (px) applied per sub-panel in a grid_of_subpanels/column_group split, for a sub-header flush against its content with no whitespace gap for strip_header's valley search to find (e.g. ROUPAS' "TORSO"/"BRAÇOS"/"PERNAS" sub-labels) of
    single_name: str = "sprite"
    rect: dict | None = None  # explicit {x,y,width,height} for type=explicit


@dataclass
class ExcludePanel:
    panel_index: int | None = None
    title_contains: str | None = None
    reason: str = ""


@dataclass
class ManualRegion:
    """A fully explicit override region, bypassing detection entirely."""

    name: str
    category: str
    rect: dict  # {x, y, width, height}
    mode: str = "single"  # single | grid
    rows: AutoOr = "auto"
    columns: AutoOr = "auto"


@dataclass
class DetectionOverrides:
    panel_overrides: list[PanelOverride] = field(default_factory=list)
    exclude_panels: list[ExcludePanel] = field(default_factory=list)


@dataclass
class OcrConfig:
    enabled: bool = False  # optional assist only; structural detection is primary


@dataclass
class SpriteExtractorConfig:
    background: BackgroundConfig = field(default_factory=BackgroundConfig)
    panels: PanelDetectionConfig = field(default_factory=PanelDetectionConfig)
    header: HeaderStripConfig = field(default_factory=HeaderStripConfig)
    grid: GridDetectionConfig = field(default_factory=GridDetectionConfig)
    subsplit: SubsplitConfig = field(default_factory=SubsplitConfig)
    text_filter: TextFilterConfig = field(default_factory=TextFilterConfig)
    sprite: SpriteConfig = field(default_factory=SpriteConfig)
    ocr: OcrConfig = field(default_factory=OcrConfig)
    detection: DetectionOverrides = field(default_factory=DetectionOverrides)
    manual_regions: list[ManualRegion] = field(default_factory=list)
    layer_order: list[str] = field(
        default_factory=lambda: [
            "body",
            "hair_back",
            "eyes",
            "eyebrows",
            "mouth",
            "clothes",
            "shoes",
            "hair_front",
            "accessories",
            "effects",
        ]
    )
    padding: int = 4  # default CLI padding, overridable per-category via sprite.*


def _dataclass_from_dict(cls, data: dict):
    if data is None:
        return cls()
    kwargs = {}
    field_types = {f.name: f.type for f in cls.__dataclass_fields__.values()}
    for key, value in data.items():
        if key not in field_types:
            raise ValueError(f"Unknown config key '{key}' for {cls.__name__}")
        kwargs[key] = value
    return cls(**kwargs)


def _load_nested(cls, data: dict | None):
    return _dataclass_from_dict(cls, data or {})


def load_config(path: str | None) -> SpriteExtractorConfig:
    raw: dict[str, Any] = {}
    if path:
        with open(path, "r", encoding="utf-8") as fh:
            raw = yaml.safe_load(fh) or {}

    cfg = SpriteExtractorConfig()
    cfg.background = _load_nested(BackgroundConfig, raw.get("background"))
    cfg.panels = _load_nested(PanelDetectionConfig, raw.get("panels"))
    cfg.header = _load_nested(HeaderStripConfig, raw.get("header"))
    cfg.grid = _load_nested(GridDetectionConfig, raw.get("grid"))
    cfg.subsplit = _load_nested(SubsplitConfig, raw.get("subsplit"))
    cfg.text_filter = _load_nested(TextFilterConfig, raw.get("text_filter"))
    cfg.sprite = _load_nested(SpriteConfig, raw.get("sprite"))
    cfg.ocr = _load_nested(OcrConfig, raw.get("ocr"))

    det_raw = raw.get("detection") or {}
    overrides = [PanelOverride(**o) for o in det_raw.get("panel_overrides", [])]
    excludes = [ExcludePanel(**o) for o in det_raw.get("exclude_panels", [])]
    cfg.detection = DetectionOverrides(panel_overrides=overrides, exclude_panels=excludes)

    cfg.manual_regions = [ManualRegion(**m) for m in raw.get("manual_regions", [])]

    if "layer_order" in raw:
        cfg.layer_order = list(raw["layer_order"])
    if "padding" in raw:
        cfg.padding = int(raw["padding"])

    _validate(cfg)
    return cfg


def _validate(cfg: SpriteExtractorConfig) -> None:
    if cfg.panels.min_area_fraction < 0 or cfg.panels.max_area_fraction > 1:
        raise ValueError("panels.min_area_fraction/max_area_fraction must be within [0, 1]")
    if cfg.background.tolerance < 0:
        raise ValueError("background.tolerance must be >= 0")
    for mr in cfg.manual_regions:
        for k in ("x", "y", "width", "height"):
            if k not in mr.rect:
                raise ValueError(f"manual_regions entry '{mr.name}' missing rect.{k}")
