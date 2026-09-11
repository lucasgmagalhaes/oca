import numpy as np

from processing.background import apply_background_removal, flood_fill_background_alpha
from processing.trimming import bbox_from_alpha, trim


def _solid_rgb(h, w, color):
    arr = np.zeros((h, w, 3), dtype=np.uint8)
    arr[:, :, :] = color
    return arr


def test_flood_fill_clears_background_reachable_from_border():
    rgb = _solid_rgb(20, 20, (10, 10, 10))
    rgb[5:15, 5:15] = (200, 50, 50)  # a foreground square in the middle
    alpha = flood_fill_background_alpha(rgb, tolerance=10)
    # background corners cleared
    assert alpha[0, 0] == 0
    assert alpha[19, 19] == 0
    # foreground square preserved
    assert alpha[10, 10] == 255


def test_flood_fill_preserves_enclosed_background_colored_hole():
    # A ring of foreground color enclosing a background-colored center -
    # flood fill (not connected to the border) must NOT clear the center,
    # unlike a naive global color-key threshold would.
    rgb = _solid_rgb(21, 21, (10, 10, 10))
    rgb[5:16, 5:16] = (200, 50, 50)  # outer foreground square
    rgb[9:12, 9:12] = (10, 10, 10)  # enclosed "hole" matching background color
    alpha = flood_fill_background_alpha(rgb, tolerance=10)
    assert alpha[0, 0] == 0  # true background cleared
    assert alpha[10, 10] == 255  # enclosed hole preserved (not reachable from border)
    assert alpha[7, 7] == 255  # ring itself preserved


def test_apply_background_removal_combines_with_existing_alpha():
    rgb = _solid_rgb(10, 10, (10, 10, 10))
    rgb[2:8, 2:8] = (200, 50, 50)
    rgba = np.dstack([rgb, np.full((10, 10), 128, dtype=np.uint8)])
    out = apply_background_removal(rgba, tolerance=10)
    # background pixel: min(128, 0) = 0
    assert out[0, 0, 3] == 0
    # foreground pixel: min(128, 255) = 128 (pre-existing partial alpha kept)
    assert out[4, 4, 3] == 128


def test_trim_finds_tight_bbox_with_padding():
    rgba = np.zeros((30, 30, 4), dtype=np.uint8)
    rgba[10:20, 10:20, 3] = 255
    bbox = bbox_from_alpha(rgba, alpha_threshold=10)
    assert bbox.as_tuple() == (10, 10, 10, 10)

    result = trim(rgba, padding=2, alpha_threshold=10)
    assert result is not None
    cropped, padded_bbox = result
    assert padded_bbox.as_tuple() == (8, 8, 14, 14)
    assert cropped.shape[:2] == (14, 14)


def test_trim_returns_none_for_fully_transparent_region():
    rgba = np.zeros((10, 10, 4), dtype=np.uint8)
    assert trim(rgba, padding=2, alpha_threshold=10) is None
