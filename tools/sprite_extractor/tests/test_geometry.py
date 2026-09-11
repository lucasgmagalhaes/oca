from utils.geometry import Rect, group_rows, merge_overlapping, split_bands, union_rect


def test_rect_basic_properties():
    r = Rect(10, 20, 30, 40)
    assert r.x0 == 10 and r.y0 == 20
    assert r.x1 == 40 and r.y1 == 60
    assert r.area == 1200
    assert r.cx == 25 and r.cy == 40


def test_rect_expand_clips_to_bounds():
    bounds = Rect(0, 0, 100, 100)
    r = Rect(2, 2, 5, 5)
    expanded = r.expand(10, bounds)
    assert expanded.x0 == 0 and expanded.y0 == 0
    assert expanded.x1 == 17 and expanded.y1 == 17


def test_rect_expand_negative_shrinks():
    bounds = Rect(0, 0, 100, 100)
    r = Rect(10, 10, 20, 20)
    shrunk = r.expand(-3, bounds)
    assert shrunk == Rect(13, 13, 14, 14)


def test_rect_intersection_and_iou():
    a = Rect(0, 0, 10, 10)
    b = Rect(5, 5, 10, 10)
    inter = a.intersection(b)
    assert inter == Rect(5, 5, 5, 5)
    assert a.iou(b) == 25 / (100 + 100 - 25)


def test_rect_contains_and_contains_ratio():
    outer = Rect(0, 0, 100, 100)
    inner = Rect(10, 10, 20, 20)
    assert outer.contains(inner)
    assert not inner.contains(outer)
    assert outer.contains_ratio(inner) == 1.0


def test_union_rect():
    rects = [Rect(0, 0, 10, 10), Rect(20, 20, 10, 10)]
    u = union_rect(rects)
    assert u == Rect(0, 0, 30, 30)


def test_union_rect_empty():
    assert union_rect([]) == Rect(0, 0, 0, 0)


def test_merge_overlapping_drops_nested_duplicate():
    # A double-line border commonly produces two close, nested contours -
    # only the larger, outer one should survive.
    outer = Rect(0, 0, 100, 100)
    inner = Rect(2, 2, 96, 96)
    result = merge_overlapping([outer, inner], iou_threshold=0.3, contain_threshold=0.85)
    assert result == [outer]


def test_merge_overlapping_keeps_distinct_rects():
    a = Rect(0, 0, 50, 50)
    b = Rect(200, 200, 50, 50)
    result = merge_overlapping([a, b])
    assert set(result) == {a, b}


def test_group_rows_reading_order():
    # two rows of panels, scrambled input order
    row1_a = Rect(0, 0, 10, 10)
    row1_b = Rect(20, 1, 10, 10)
    row2_a = Rect(0, 50, 10, 10)
    rows = group_rows([row2_a, row1_b, row1_a])
    assert len(rows) == 2
    assert rows[0] == [row1_a, row1_b]
    assert rows[1] == [row2_a]


def test_split_bands_finds_two_bands_separated_by_valley():
    # a wide flat valley of zeros in the middle
    density = [1.0] * 10 + [0.0] * 5 + [1.0] * 10
    bands = split_bands(density, valley_threshold=0.1, min_gap=3, min_band=3)
    assert bands == [(0, 10), (15, 25)]


def test_split_bands_ignores_short_valleys_below_min_gap():
    # a 1-sample dip shouldn't split a band when min_gap=3
    density = [1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0]
    bands = split_bands(density, valley_threshold=0.1, min_gap=3, min_band=1)
    assert bands == [(0, 7)]


def test_split_bands_drops_bands_smaller_than_min_band():
    # a single active sample flanked by wide valleys - real content bands
    # are never this thin, so it should be dropped as noise.
    density = [0.0] * 10 + [1.0] + [0.0] * 10
    bands = split_bands(density, valley_threshold=0.1, min_gap=2, min_band=3)
    assert bands == []
