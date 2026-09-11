import numpy as np

from config import TextFilterConfig
from detection.sprites import bbox_of_mask, connected_components, horizontal_clusters, looks_like_text


def test_bbox_of_mask_tight():
    mask = np.zeros((20, 20), dtype=bool)
    mask[5:10, 3:12] = True
    bbox = bbox_of_mask(mask)
    assert bbox.as_tuple() == (3, 5, 9, 5)


def test_bbox_of_mask_none_when_empty():
    assert bbox_of_mask(np.zeros((10, 10), dtype=bool)) is None


def test_connected_components_counts_disjoint_blobs():
    mask = np.zeros((20, 20), dtype=bool)
    mask[1:4, 1:4] = True
    mask[10:15, 10:15] = True
    comps = connected_components(mask)
    assert len(comps) == 2


def test_connected_components_dilate_merges_close_blobs():
    mask = np.zeros((20, 20), dtype=bool)
    mask[5, 5] = True
    mask[5, 7] = True  # 1px gap - merges under a 3px dilation
    comps = connected_components(mask, dilate_kernel=3)
    assert len(comps) == 1


def test_horizontal_clusters_splits_well_separated_groups():
    # two sprite-like blobs far apart horizontally within one cell
    mask = np.zeros((20, 60), dtype=bool)
    mask[5:15, 2:10] = True
    mask[5:15, 45:55] = True
    clusters = horizontal_clusters(mask, dilate_kernel=1, min_gap=5)
    assert len(clusters) == 2


def test_horizontal_clusters_keeps_close_content_together():
    mask = np.zeros((20, 60), dtype=bool)
    mask[5:15, 2:10] = True
    mask[5:15, 12:20] = True  # only 2px gap
    clusters = horizontal_clusters(mask, dilate_kernel=1, min_gap=10)
    assert len(clusters) == 1


def test_looks_like_text_true_for_many_small_aligned_components():
    # a row of small, similarly-sized glyph-like blobs, short relative to
    # a tall region (as real title/subtitle glyphs are relative to a panel)
    mask = np.zeros((120, 100), dtype=bool)
    for i in range(8):
        x = 5 + i * 12
        mask[10:16, x : x + 6] = True
    cfg = TextFilterConfig()
    assert looks_like_text(mask, cfg)


def test_looks_like_text_false_for_one_dominant_blob():
    mask = np.zeros((30, 100), dtype=bool)
    mask[2:28, 20:80] = True  # one big sprite-sized shape
    # a couple of small flecks shouldn't flip this to "text"
    mask[1, 1] = True
    mask[2, 3] = True
    mask[3, 1] = True
    cfg = TextFilterConfig()
    assert not looks_like_text(mask, cfg)


def test_looks_like_text_false_when_too_few_components():
    mask = np.zeros((30, 100), dtype=bool)
    mask[5:10, 5:10] = True
    mask[5:10, 20:25] = True
    cfg = TextFilterConfig()
    assert not looks_like_text(mask, cfg)
