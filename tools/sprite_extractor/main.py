"""CLI entrypoint.

    python -m sprite_extractor --input sheet.png --output out/ \
        --config config.yaml --mode trimmed --padding 4 --debug
"""

from __future__ import annotations

import argparse
import os
import sys

from config import load_config
from pipeline import Pipeline


def build_arg_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="sprite_extractor", description="Extract sprite assets from a reference sheet image.")
    p.add_argument("--input", required=True, help="Path to the source sheet image (PNG).")
    p.add_argument("--output", required=True, help="Output directory for extracted assets.")
    p.add_argument("--config", default=None, help="Path to a YAML config (defaults to built-in defaults).")
    p.add_argument("--mode", choices=["trimmed", "canvas"], default="trimmed", help="Export mode.")
    p.add_argument("--padding", type=int, default=None, help="Padding (px) around each trimmed sprite bbox.")
    p.add_argument("--debug", action="store_true", help="Write staged debug visualizations to <output>/debug/.")
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_arg_parser().parse_args(argv)

    if not os.path.isfile(args.input):
        print(f"error: input image not found: {args.input}", file=sys.stderr)
        return 2

    cfg = load_config(args.config)
    padding = args.padding if args.padding is not None else cfg.padding

    pipeline = Pipeline(
        image_path=args.input,
        out_dir=args.output,
        cfg=cfg,
        mode=args.mode,
        padding=padding,
        debug=args.debug,
    )
    meta_path = pipeline.run()
    print(f"Done. Wrote {len(pipeline.metadata.assets)} assets. Aggregate metadata: {meta_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
