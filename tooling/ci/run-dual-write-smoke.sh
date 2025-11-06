#!/usr/bin/env bash
# Dual-write front-end成果物のスモーク生成
#
# Usage:
#   tooling/ci/run-dual-write-smoke.sh [output_dir]
#
# 指定されたディレクトリ（デフォルト: reports/dual-write/front-end）配下に
# `ocaml/`・`rust/`・`diff/` を生成し、それぞれに検証用のサンプルファイルを
# 出力する。CI とローカル実行の双方で利用できるよう、副作用のない内容で
# 最低限の成果物を作成する。

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_ROOT="${1:-reports/dual-write/front-end}"
OUTPUT_DIR="${ROOT_DIR}/${OUTPUT_ROOT}"

# サブディレクトリ作成
mkdir -p "${OUTPUT_DIR}/ocaml" "${OUTPUT_DIR}/rust" "${OUTPUT_DIR}/diff"

# タイムスタンプを共通で使用
timestamp="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

# Python を利用して JSON を整形出力
python3 - "${OUTPUT_DIR}" "${timestamp}" <<'PYCODE'
from __future__ import annotations

import json
import sys
from pathlib import Path

output_dir = Path(sys.argv[1])
timestamp = sys.argv[2]

ocaml_payload = {
    "frontend": "ocaml",
    "sample": "dual-write-smoke",
    "generated_at": timestamp,
    "notes": ["baseline generated for dual-write smoke test"],
}

rust_payload = {
    "frontend": "rust",
    "sample": "dual-write-smoke",
    "generated_at": timestamp,
    "notes": ["candidate generated for dual-write smoke test"],
}

(output_dir / "ocaml" / "smoke.json").write_text(
    json.dumps(ocaml_payload, ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
)
(output_dir / "rust" / "smoke.json").write_text(
    json.dumps(rust_payload, ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
)

summary = {
    "cases": [
        {
            "id": "dual-write-smoke",
            "baseline": "ocaml/smoke.json",
            "candidate": "rust/smoke.json",
            "diff": "diff/smoke.diff",
        }
    ],
    "generated_at": timestamp,
}
(output_dir / "summary.json").write_text(
    json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
)
PYCODE

# 差分生成（diff が空の場合はメッセージを記録）
python3 - "${OUTPUT_DIR}" <<'PYCODE'
from __future__ import annotations

import difflib
from pathlib import Path
import sys

output_dir = Path(sys.argv[1])
baseline = (output_dir / "ocaml" / "smoke.json").read_text(encoding="utf-8").splitlines()
candidate = (output_dir / "rust" / "smoke.json").read_text(encoding="utf-8").splitlines()

diff_lines = list(
    difflib.unified_diff(
        baseline,
        candidate,
        fromfile="ocaml/smoke.json",
        tofile="rust/smoke.json",
        lineterm="",
    )
)

diff_path = output_dir / "diff" / "smoke.diff"
if diff_lines:
    diff_path.write_text("\n".join(diff_lines) + "\n", encoding="utf-8")
else:
    diff_path.write_text("No diff detected.\n", encoding="utf-8")
PYCODE

printf '[dual-write-smoke] artifacts generated under %s\n' "${OUTPUT_ROOT}"
