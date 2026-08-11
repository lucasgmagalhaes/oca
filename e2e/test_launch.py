def test_main_window_appears(oca_window):
    assert oca_window.exists()
    assert oca_window.is_visible()
