import os
import tempfile

import pytest

from config import load_config


def test_load_config_defaults_when_no_path():
    cfg = load_config(None)
    assert cfg.padding == 4
    assert cfg.background.tolerance == 20
    assert "body" in cfg.layer_order


def test_load_config_reads_yaml_overrides():
    yaml_text = """
padding: 9
background:
  tolerance: 33
detection:
  panel_overrides:
    - panel_index: 0
      type: special
      category: base/reference
  exclude_panels:
    - panel_index: 5
      reason: "legend panel"
"""
    with tempfile.NamedTemporaryFile("w", suffix=".yaml", delete=False) as fh:
        fh.write(yaml_text)
        path = fh.name
    try:
        cfg = load_config(path)
        assert cfg.padding == 9
        assert cfg.background.tolerance == 33
        assert len(cfg.detection.panel_overrides) == 1
        assert cfg.detection.panel_overrides[0].category == "base/reference"
        assert cfg.detection.exclude_panels[0].panel_index == 5
    finally:
        os.unlink(path)


def test_load_config_rejects_unknown_key():
    yaml_text = "background:\n  not_a_real_field: 1\n"
    with tempfile.NamedTemporaryFile("w", suffix=".yaml", delete=False) as fh:
        fh.write(yaml_text)
        path = fh.name
    try:
        with pytest.raises(ValueError):
            load_config(path)
    finally:
        os.unlink(path)


def test_load_config_validates_manual_region_rect():
    yaml_text = """
manual_regions:
  - name: broken
    category: broken
    rect:
      x: 0
      y: 0
"""
    with tempfile.NamedTemporaryFile("w", suffix=".yaml", delete=False) as fh:
        fh.write(yaml_text)
        path = fh.name
    try:
        with pytest.raises(ValueError):
            load_config(path)
    finally:
        os.unlink(path)
