"""Orchestrates the full extraction pipeline: panel detection -> per-panel
classification (grid / grid-of-subpanels / special) -> cell/sprite detection
-> background removal & trimming -> export + metadata (+ optional debug
visualization).
"""

from __future__ import annotations

import os
from dataclasses import replace

import numpy as np

from config import PanelOverride, SpriteExtractorConfig
from classification.categories import default_category, find_override, is_excluded
from detection.cells import split_cell_sprites
from detection.grids import GridResult, bands_are_regular, cells_from_bands, detect_grid, strip_header, uniform_bands_within
from detection.panels import detect_panels, detect_subpanels
from detection.sprites import looks_like_text
from output.exporter import AssetExporter
from output.metadata import MetadataCollector
from output.naming import direction_frame_name, ensure_dir, flat_item_name, subcategory_item_name
from utils.debug import DebugRun
from utils.geometry import Rect
from utils.image import crop, estimate_background_color, foreground_mask, image_bounds, load_rgba, to_gray

PANEL_COLOR = (80, 200, 255)
GRID_COLOR = (120, 220, 120)
SUBPANEL_COLOR = (220, 160, 60)
CELL_COLOR = (200, 200, 80)
SPRITE_COLOR = (255, 90, 90)
TEXT_COLOR = (255, 140, 220)
EXCLUDED_COLOR = (150, 150, 150)


def _resolve_bg_color(rgba: np.ndarray, cfg: SpriteExtractorConfig) -> tuple[int, int, int]:
    if not isinstance(cfg.background.color, str):
        c = cfg.background.color
        return tuple(c) if not isinstance(c, (int, float)) else (int(c),) * 3
    return estimate_background_color(rgba)


class Pipeline:
    def __init__(self, image_path: str, out_dir: str, cfg: SpriteExtractorConfig, mode: str, padding: int, debug: bool):
        self.image_path = image_path
        self.out_dir = out_dir
        self.cfg = cfg
        self.mode = mode
        self.padding = padding
        self.rgba = load_rgba(image_path)
        self.gray = to_gray(self.rgba)
        self.bg_color = _resolve_bg_color(self.rgba, cfg)
        self.bounds = image_bounds(self.rgba)

        self.exporter = AssetExporter(mode, padding, cfg.sprite.alpha_threshold, cfg.background)
        self.metadata = MetadataCollector(cfg.layer_order, mode, os.path.basename(image_path))
        self.debug = DebugRun(os.path.join(out_dir, "debug"), self.rgba, enabled=debug)

        # debug accumulators
        self._dbg_panels: list[tuple[Rect, tuple, str]] = []
        self._dbg_classified: list[tuple[Rect, tuple, str]] = []
        self._dbg_subunits: list[tuple[Rect, tuple, str]] = []
        self._dbg_cells: list[tuple[Rect, tuple, str]] = []
        self._dbg_text: list[tuple[Rect, tuple, str]] = []
        self._dbg_sprites: list[tuple[Rect, tuple, str]] = []

    # -- helpers -----------------------------------------------------
    def _mask(self, rect: Rect) -> np.ndarray:
        region = crop(self.rgba, rect)
        return foreground_mask(region, self.bg_color, self.cfg.background.tolerance)

    def _gray_region(self, rect: Rect) -> np.ndarray:
        return crop(self.gray, rect)

    def _resolve_grid(self, body: Rect, override: PanelOverride | None):
        mask = self._mask(body)
        grid = detect_grid(mask, self.cfg.grid, self._gray_region(body))
        if override is None:
            return grid
        rows, cols = grid.row_bands, grid.col_bands
        if not isinstance(override.rows, str):
            rows = uniform_bands_within(grid.row_bands, body.h, int(override.rows))
        if not isinstance(override.columns, str):
            cols = uniform_bands_within(grid.col_bands, body.w, int(override.columns))
        return GridResult(rows, cols)

    def _body_after_header(self, rect: Rect) -> Rect:
        mask = self._mask(rect)
        offset = strip_header(mask, self.cfg.header)
        body = Rect(rect.x0, rect.y0 + offset, rect.w, rect.h - offset)
        # Inset a touch so the parent panel/sub-panel's own border isn't
        # mistaken for part of a nested sub-panel or a grid boundary line
        # right at the body's edge (see detection/panels.py::detect_subpanels).
        m = self.cfg.panels.body_inset_px
        return body.expand(-m, self.bounds)

    def run(self) -> str:
        panels = detect_panels(self.gray, self.cfg.panels)
        self._dbg_panels = [(p, PANEL_COLOR, f"#{i}") for i, p in enumerate(panels)]
        self.debug.save_stage("detected_panels", self._dbg_panels)

        for idx, panel in enumerate(panels):
            excluded, reason = is_excluded(self.cfg, idx, None)
            if excluded:
                self._dbg_classified.append((panel, EXCLUDED_COLOR, f"#{idx} excluded"))
                continue
            override = find_override(self.cfg, idx, None)
            self._process_panel(idx, panel, override)

        self.debug.save_stage("classified_panels", self._dbg_classified)
        self.debug.save_stage("subunits", self._dbg_subunits)
        self.debug.save_stage("cells", self._dbg_cells)
        self.debug.save_stage("ignored_text", self._dbg_text)
        self.debug.save_stage("final_boxes", self._dbg_sprites)

        ensure_dir(self.out_dir)
        return self.metadata.write_aggregate(self.out_dir)

    # -- per-panel dispatch -------------------------------------------
    def _process_panel(self, idx: int, panel: Rect, override: PanelOverride | None) -> None:
        category = (override.category if override and override.category else default_category(idx))
        body = self._body_after_header(panel)
        if override and override.body_inset_left:
            body = Rect(body.x0 + override.body_inset_left, body.y0, body.w - override.body_inset_left, body.h)

        forced_type = override.type if override else "auto"

        body_gray = crop(self.gray, body)
        subpanel_cfg = replace(self.cfg.panels, dilate_kernel=self.cfg.panels.subpanel_dilate_kernel)
        subpanels = detect_subpanels(body_gray, body, subpanel_cfg)

        # Decide grid-of-subpanels vs special vs plain grid.
        non_text_subpanels = []
        text_subpanels = []
        for sp in subpanels:
            sp_mask = self._mask(sp)
            if looks_like_text(sp_mask, self.cfg.text_filter):
                text_subpanels.append(sp)
            else:
                non_text_subpanels.append(sp)

        use_subpanels = forced_type == "grid_of_subpanels" or (forced_type == "auto" and len(non_text_subpanels) >= 2)

        if forced_type == "explicit" and override and override.rect:
            r = override.rect
            rect = Rect(r["x"], r["y"], r["width"], r["height"])
            self._export_special(category, override.single_name if override else "sprite", rect, idx)
            self._dbg_classified.append((panel, SUBPANEL_COLOR, f"#{idx} {category} explicit"))
            return

        if use_subpanels:
            self._dbg_classified.append((panel, SUBPANEL_COLOR, f"#{idx} {category} grid_of_subpanels"))
            names = override.subpanel_names if override and override.subpanel_names else [
                f"group_{i+1}" for i in range(len(non_text_subpanels))
            ]
            cats = (
                override.subpanel_categories
                if override and override.subpanel_categories
                else [f"{category}/{n}" for n in names]
            )
            multi = override.cell_multi_sprite if override and override.cell_multi_sprite is not None else self.cfg.subsplit.enabled
            naming = override.subpanel_naming if override else "indexed"
            for i, sp in enumerate(non_text_subpanels):
                self._dbg_subunits.append((sp, SUBPANEL_COLOR, names[i] if i < len(names) else f"group_{i+1}"))
                sub_category = cats[i] if i < len(cats) else f"{category}/group_{i+1}"
                sp_body = self._body_after_header(sp)
                if override and override.subpanel_header_px:
                    extra = override.subpanel_header_px
                    sp_body = Rect(sp_body.x0, sp_body.y0 + extra, sp_body.w, sp_body.h - extra)
                grid = self._resolve_grid(sp_body, override)
                if not grid.row_bands or not grid.col_bands:
                    continue
                if naming == "frame":
                    self._process_frame_grid(sub_category, sp_body, grid.row_bands, grid.col_bands, multi)
                else:
                    self._process_grid(sub_category, sp_body, grid.row_bands, grid.col_bands, multi)
            for tsp in text_subpanels:
                self._dbg_text.append((tsp, TEXT_COLOR, "text"))
            return

        # not grid-of-subpanels: try a plain grid on body, excluding any
        # detected text subpanel regions (info-cards, legends) from the mask.
        body_mask = self._mask(body)
        for tsp in text_subpanels:
            local = tsp.offset(-body.x0, -body.y0).clip(Rect(0, 0, body.w, body.h))
            body_mask[local.y0 : local.y1, local.x0 : local.x1] = False
            self._dbg_text.append((tsp, TEXT_COLOR, "text"))
        for nsp in non_text_subpanels:
            # a lone (count==1) non-text nested box (rare) - still exclude
            # from grid detection to avoid corrupting projections; it will
            # not be separately exported in this branch.
            local = nsp.offset(-body.x0, -body.y0).clip(Rect(0, 0, body.w, body.h))
            body_mask[local.y0 : local.y1, local.x0 : local.x1] = False

        grid = detect_grid(body_mask, self.cfg.grid, self._gray_region(body))
        if override is not None:
            rows, cols = grid.row_bands, grid.col_bands
            if not isinstance(override.rows, str):
                rows = uniform_bands_within(grid.row_bands, body.h, int(override.rows))
            if not isinstance(override.columns, str):
                cols = uniform_bands_within(grid.col_bands, body.w, int(override.columns))
            grid = GridResult(rows, cols)
        is_real_grid = forced_type == "grid" or (
            forced_type == "auto"
            and grid.is_grid
            and len(grid.row_bands) * len(grid.col_bands) >= 2
            and bands_are_regular(grid.col_bands, self.cfg.grid.regularity_tolerance)
        )

        if is_real_grid and grid.row_bands and grid.col_bands:
            self._dbg_classified.append((panel, GRID_COLOR, f"#{idx} {category} grid"))
            multi = bool(override and override.cell_multi_sprite)
            if override and override.column_group_names:
                self._process_column_grouped_grid(category, body, grid.row_bands, grid.col_bands, override.column_group_names, multi)
            else:
                self._process_grid(category, body, grid.row_bands, grid.col_bands, multi)
        else:
            self._dbg_classified.append((panel, PANEL_COLOR, f"#{idx} {category} special"))
            single_name = override.single_name if override else "sprite"
            self._export_special(category, single_name, body, idx)

    def _process_column_grouped_grid(self, category: str, body: Rect, row_bands, col_bands, group_names: list[str], multi: bool) -> None:
        """Like `_process_frame_grid`, but for a plain (non-sub-panel) grid
        whose columns fold into named groups of equal size (e.g. CORPO
        BASE's 8 fine columns -> 4 directions x 2 sprites each). Frame index
        = row, sub-letter = position within the group."""
        n_groups = len(group_names)
        if n_groups == 0 or len(col_bands) % n_groups != 0:
            self._process_grid(category, body, row_bands, col_bands, multi)
            return
        group_size = len(col_bands) // n_groups
        cells = cells_from_bands(row_bands, col_bands, body)
        for row_idx, row in enumerate(cells):
            frame_index = row_idx + 1
            for col_idx, cell in enumerate(row):
                group_idx = col_idx // group_size
                sub_index = col_idx % group_size
                sub_category = f"{category}/{group_names[group_idx]}"
                self._dbg_cells.append((cell, CELL_COLOR, ""))
                cell_mask = self._mask(cell)
                if looks_like_text(cell_mask, self.cfg.text_filter):
                    self._dbg_text.append((cell, TEXT_COLOR, "text"))
                    continue
                subsplit_cfg = self.cfg.subsplit if multi else _disabled_subsplit(self.cfg)
                found = split_cell_sprites(cell_mask, cell, subsplit_cfg)
                found = [s for s in found if s.area >= self.cfg.sprite.min_area_fraction_of_cell * cell.area]
                if not found:
                    continue
                sprite_rect = found[0]
                fname = direction_frame_name(frame_index, sub_index, group_size)
                out_path = os.path.join(self.out_dir, sub_category, fname)
                meta = self.exporter.export(self.rgba, sprite_rect.expand(2, self.bounds), out_path)
                if meta is None:
                    continue
                self.metadata.write_sidecar(out_path, sub_category, meta)
                rel = os.path.relpath(out_path, self.out_dir)
                self.metadata.add(sub_category, rel, meta)
                self._dbg_sprites.append((sprite_rect, SPRITE_COLOR, group_names[group_idx]))

    def _process_frame_grid(self, category: str, body: Rect, row_bands, col_bands, multi: bool) -> None:
        """Like `_process_grid`, but names outputs `frame_NNN[_a/_b].png`:
        one animation frame per row, one letter suffix per column when a
        row has more than one sprite (e.g. CORPO BASE's 2 side-by-side
        sprites per labeled "Frame N (Idle)" row - real, grid-detected
        columns here, not a heuristic split)."""
        cells = cells_from_bands(row_bands, col_bands, body)
        for row_idx, row in enumerate(cells):
            frame_index = row_idx + 1
            col_sprites: list[Rect] = []
            for cell in row:
                self._dbg_cells.append((cell, CELL_COLOR, ""))
                cell_mask = self._mask(cell)
                subsplit_cfg = self.cfg.subsplit if multi else _disabled_subsplit(self.cfg)
                found = split_cell_sprites(cell_mask, cell, subsplit_cfg)
                found = [s for s in found if s.area >= self.cfg.sprite.min_area_fraction_of_cell * cell.area]
                col_sprites.append(found[0] if found else None)
            sub_count = sum(1 for s in col_sprites if s is not None)
            sub_index = 0
            for s in col_sprites:
                if s is None:
                    continue
                fname = direction_frame_name(frame_index, sub_index, sub_count)
                sub_index += 1
                out_path = os.path.join(self.out_dir, category, fname)
                meta = self.exporter.export(self.rgba, s.expand(2, self.bounds), out_path)
                if meta is None:
                    continue
                self.metadata.write_sidecar(out_path, category, meta)
                rel = os.path.relpath(out_path, self.out_dir)
                self.metadata.add(category, rel, meta)
                self._dbg_sprites.append((s, SPRITE_COLOR, category.split("/")[-1]))

    def _process_grid(self, category: str, body: Rect, row_bands, col_bands, multi: bool = False) -> None:
        cells = cells_from_bands(row_bands, col_bands, body)
        index = 0
        is_multi_sub = "/" in category
        for row in cells:
            for cell in row:
                self._dbg_cells.append((cell, CELL_COLOR, ""))
                cell_mask = self._mask(cell)
                if looks_like_text(cell_mask, self.cfg.text_filter):
                    self._dbg_text.append((cell, TEXT_COLOR, "text"))
                    continue
                subsplit_cfg = self.cfg.subsplit if multi else _disabled_subsplit(self.cfg)
                sprites = split_cell_sprites(cell_mask, cell, subsplit_cfg)
                sprites = [s for s in sprites if s.area >= self.cfg.sprite.min_area_fraction_of_cell * cell.area]
                if not sprites:
                    continue
                for sub_index, sprite_rect in enumerate(sprites):
                    index += 1
                    leaf = category.split("/")[-1]
                    fname = (
                        subcategory_item_name(leaf, index) if is_multi_sub else flat_item_name(leaf, index)
                    )
                    out_path = os.path.join(self.out_dir, category, fname)
                    meta = self.exporter.export(self.rgba, sprite_rect.expand(2, self.bounds), out_path)
                    if meta is None:
                        continue
                    self.metadata.write_sidecar(out_path, category, meta)
                    rel = os.path.relpath(out_path, self.out_dir)
                    self.metadata.add(category, rel, meta)
                    self._dbg_sprites.append((sprite_rect, SPRITE_COLOR, leaf))

    def _export_special(self, category: str, single_name: str, region: Rect, panel_idx: int) -> None:
        mask = self._mask(region)
        from detection.sprites import horizontal_clusters

        min_gap = max(2, int(region.w * self.cfg.subsplit.min_gap_fraction_of_cell * 2))
        clusters = horizontal_clusters(mask, self.cfg.subsplit.dilate_kernel, min_gap)
        min_area = self.cfg.sprite.min_area_fraction_of_cell * region.area * 4
        clusters = [c for c in clusters if c.area >= min_area]
        if not clusters:
            return
        for i, c in enumerate(clusters):
            rect = c.offset(region.x0, region.y0)
            fname = f"{single_name}.png" if len(clusters) == 1 else f"{single_name}_{i+1:03d}.png"
            out_path = os.path.join(self.out_dir, category, fname)
            meta = self.exporter.export(self.rgba, rect.expand(2, self.bounds), out_path)
            if meta is None:
                continue
            self.metadata.write_sidecar(out_path, category, meta)
            rel = os.path.relpath(out_path, self.out_dir)
            self.metadata.add(category, rel, meta)
            self._dbg_sprites.append((rect, SPRITE_COLOR, category))


def _disabled_subsplit(cfg: SpriteExtractorConfig):
    from dataclasses import replace

    return replace(cfg.subsplit, enabled=False)
