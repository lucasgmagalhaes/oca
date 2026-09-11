import os
import sys

# Internal modules use flat absolute imports (`from config import ...`,
# `from detection.panels import ...`) rather than package-relative ones, so
# this package's own directory must be on sys.path - `python -m
# sprite_extractor` otherwise only has its *parent* directory on sys.path.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from main import main  # noqa: E402

if __name__ == "__main__":
    raise SystemExit(main())
