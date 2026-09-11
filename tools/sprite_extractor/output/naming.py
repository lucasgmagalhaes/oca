"""Output path/filename conventions per category, matching the spec's
directory layout: base/{front,back,left,right}/frame_NNN[_x].png, skin/,
eyes/, eyebrows/, mouths/, hair_back/, hair_front/, clothes/{torso,arms,
legs}/, shoes/, accessories/, effects/, plus base/reference/ for the single
"empty base" idle sprite (not covered by the direction grid).
"""

from __future__ import annotations

import os
import string

_SUFFIXES = list(string.ascii_lowercase)  # a, b, c... for multi-sprite cells


def direction_frame_name(frame_index: int, sub_index: int, sub_count: int) -> str:
    base = f"frame_{frame_index:03d}"
    if sub_count <= 1:
        return f"{base}.png"
    return f"{base}_{_SUFFIXES[sub_index]}.png"


def flat_item_name(category: str, index: int) -> str:
    return f"{category}_{index:03d}.png"


def subcategory_item_name(subcategory: str, index: int) -> str:
    return f"{subcategory}_{index:03d}.png"


def ensure_dir(path: str) -> str:
    os.makedirs(path, exist_ok=True)
    return path
