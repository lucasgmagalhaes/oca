"""Debug visualization: draws labeled boxes over the source image at each
detection stage and writes them to `<output>/debug/NN_name.png`."""

from __future__ import annotations

import os

import numpy as np
from PIL import Image, ImageDraw, ImageFont

from .geometry import Rect

_STAGE_COUNTER = {"n": 0}


class DebugRun:
    def __init__(self, out_dir: str, base_rgba: np.ndarray, enabled: bool = True):
        self.out_dir = out_dir
        self.enabled = enabled
        self.base = Image.fromarray(base_rgba).convert("RGB")
        self.stage = 0
        if enabled:
            os.makedirs(out_dir, exist_ok=True)
        try:
            self.font = ImageFont.load_default()
        except Exception:
            self.font = None

    def save_stage(self, name: str, boxes: list[tuple[Rect, tuple, str]]):
        """boxes: list of (rect, rgb_color, label)."""
        if not self.enabled:
            return
        self.stage += 1
        img = self.base.copy()
        draw = ImageDraw.Draw(img)
        for rect, color, label in boxes:
            draw.rectangle([rect.x0, rect.y0, max(rect.x1 - 1, rect.x0), max(rect.y1 - 1, rect.y0)], outline=color, width=2)
            if label:
                ty = max(0, rect.y0 - 12)
                draw.rectangle([rect.x0, ty, rect.x0 + 6 + 6 * len(label), ty + 11], fill=(0, 0, 0))
                draw.text((rect.x0 + 2, ty), label, fill=color, font=self.font)
        path = os.path.join(self.out_dir, f"{self.stage:02d}_{name}.png")
        img.save(path)
        return path
