#!/usr/bin/env python3
"""ラッパースクリプト — `compiler/ocaml/scripts/unicode/generate-xid-tables.py` を実行する"""

from __future__ import annotations

import runpy
import sys
from pathlib import Path


def main(argv: list[str]) -> int:
    script = (
        Path(__file__).resolve().parents[2]
        / "compiler"
        / "ocaml"
        / "scripts"
        / "unicode"
        / "generate-xid-tables.py"
    )
    runpy.run_path(str(script), run_name="__main__")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
