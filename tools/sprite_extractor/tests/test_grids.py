import numpy as np

from config import GridDetectionConfig, HeaderStripConfig
from detection.grids import (
    bands_are_regular,
    detect_grid,
    strip_header,
    uniform_bands,
    uniform_bands_within,
)


def _mask_with_row_gap(height=100, width=60, gap_start=45, gap_end=55):
    mask = np.ones((height, width), dtype=bool)
    mask[gap_start:gap_end, :] = False
    return mask


def test_uniform_bands_splits_evenly():
    bands = uniform_bands(100, 4)
    assert bands == [(0, 25), (25, 50), (50, 75), (75, 100)]


def test_uniform_bands_handles_remainder():
    bands = uniform_bands(10, 3)
    assert len(bands) == 3
    assert bands[0][0] == 0
    assert bands[-1][1] == 10


def test_uniform_bands_zero_count():
    assert uniform_bands(100, 0) == []


def test_uniform_bands_within_respects_auto_span():
    # auto-detected content only spans [10, 90) of a 100px region
    auto = [(10, 50), (50, 90)]
    bands = uniform_bands_within(auto, 100, 4)
    assert bands[0][0] == 10
    assert bands[-1][1] == 90
    assert len(bands) == 4


def test_uniform_bands_within_falls_back_when_no_auto_bands():
    bands = uniform_bands_within([], 100, 5)
    assert bands == uniform_bands(100, 5)


def test_bands_are_regular_true_for_equal_bands():
    bands = [(0, 10), (10, 20), (20, 30)]
    assert bands_are_regular(bands, tolerance=0.1)


def test_bands_are_regular_false_for_uneven_bands():
    bands = [(0, 10), (10, 15), (15, 60)]
    assert not bands_are_regular(bands, tolerance=0.2)


def test_bands_are_regular_single_band_is_trivially_regular():
    assert bands_are_regular([(0, 10)], tolerance=0.01)


def test_detect_grid_finds_rows_and_cols_from_foreground_valleys():
    # 2 rows x 2 cols of solid content, separated by background gaps
    mask = np.zeros((40, 40), dtype=bool)
    mask[2:16, 2:16] = True
    mask[2:16, 24:38] = True
    mask[24:38, 2:16] = True
    mask[24:38, 24:38] = True
    cfg = GridDetectionConfig()
    result = detect_grid(mask, cfg)
    assert len(result.row_bands) == 2
    assert len(result.col_bands) == 2
    assert result.is_grid


def test_detect_grid_single_blob_is_not_a_grid():
    mask = np.zeros((40, 40), dtype=bool)
    mask[5:35, 5:35] = True
    cfg = GridDetectionConfig()
    result = detect_grid(mask, cfg)
    assert not (len(result.row_bands) >= 2 and len(result.col_bands) >= 2)


def test_strip_header_finds_gap_after_title_block():
    h, w = 120, 60
    mask = np.zeros((h, w), dtype=bool)
    mask[0:15, :] = True  # title+subtitle block
    mask[40:110, :] = True  # real grid body, a substantial band
    cfg = HeaderStripConfig()
    offset = strip_header(mask, cfg)
    # a couple of px of tolerance: the density profile is smoothed (bridges
    # small noise dips), which can shift the detected edge by a pixel or two
    assert 38 <= offset <= 42


def test_strip_header_returns_zero_when_no_gap_found():
    mask = np.ones((50, 50), dtype=bool)
    cfg = HeaderStripConfig()
    assert strip_header(mask, cfg) == 0


def test_strip_header_skips_short_fragment_before_real_body():
    # simulates a title that itself band-splits into a couple of short
    # fragments, followed by a real (tall) content band
    h, w = 200, 80
    mask = np.zeros((h, w), dtype=bool)
    mask[3:4, :] = True  # noise speck
    mask[14:30, :] = True  # header fragment (short)
    mask[60:150, :] = True  # real body (tall)
    cfg = HeaderStripConfig()
    offset = strip_header(mask, cfg)
    assert 58 <= offset <= 62
