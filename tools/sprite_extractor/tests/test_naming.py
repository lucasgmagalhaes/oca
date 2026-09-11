from output.naming import direction_frame_name, flat_item_name, subcategory_item_name


def test_direction_frame_name_single_sprite():
    assert direction_frame_name(1, 0, 1) == "frame_001.png"


def test_direction_frame_name_multi_sprite_uses_letter_suffix():
    assert direction_frame_name(2, 0, 2) == "frame_002_a.png"
    assert direction_frame_name(2, 1, 2) == "frame_002_b.png"


def test_flat_item_name_zero_pads():
    assert flat_item_name("eyes", 7) == "eyes_007.png"
    assert flat_item_name("eyes", 123) == "eyes_123.png"


def test_subcategory_item_name():
    assert subcategory_item_name("torso", 3) == "torso_003.png"
