#!/usr/bin/env python3
"""
Phase4 シナリオマトリクスへ Phase3 テスト資産を自動登録する補助スクリプト。

`docs/plans/bootstrap-roadmap/p1-test-migration-*.txt` の各行を読み取り、
`phase4-scenario-matrix.csv` と互換の列を生成する。既存 ID と重複する場合は
スキップし、`--write` を指定しない限り標準出力に CSV を流すだけの dry-run になる。
"""

from __future__ import annotations

import argparse
import csv
import pathlib
from typing import Dict, Iterable, List, Sequence, Tuple


REPO_ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_CASES_GLOB = "docs/plans/bootstrap-roadmap/p1-test-migration-*.txt"
DEFAULT_MATRIX = "docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv"

COLUMN_ORDER: Sequence[str] = (
    "scenario_id",
    "category",
    "spec_chapter",
    "spec_anchor",
    "variant",
    "priority",
    "input_path",
    "expected",
    "diagnostic_keys",
    "capability",
    "stage_requirement",
    "scenario_notes",
    "resolution",
    "resolution_notes",
    "spec_vs_impl_decision",
)

# case 種別ごとの既定マッピング
CASE_RULES: Dict[str, Tuple[str, str, str]] = {
    "lexer": ("Prelude", "chapter1.syntax", "docs/spec/1-1-syntax.md§A"),
    "parser": ("Prelude", "chapter1.syntax", "docs/spec/1-1-syntax.md§B"),
    "constraint": ("Prelude", "chapter1.types", "docs/spec/1-2-types-Inference.md§D"),
    "effect": ("Capability", "chapter1.effects", "docs/spec/1-3-effects-safety.md§I"),
    "diagnostic": ("CLI", "chapter3.diagnostics", "docs/spec/3-6-core-diagnostics-audit.md§9"),
    "ffi": ("Runtime", "chapter3.runtime", "docs/spec/3-8-core-runtime-capability.md§4"),
    "streaming": ("Runtime", "chapter2.streaming", "docs/guides/core-parse-streaming.md§2"),
}


def load_existing_ids(matrix_path: pathlib.Path) -> Dict[str, Dict[str, str]]:
    with matrix_path.open(newline="", encoding="utf-8") as fh:
        reader = csv.DictReader(fh)
        return {row["scenario_id"]: row for row in reader}


def iter_case_files(cases_glob: str) -> Iterable[pathlib.Path]:
    yield from sorted(REPO_ROOT.glob(cases_glob))


def infer_mapping(case_file: pathlib.Path) -> Tuple[str, str, str]:
    # ファイル名からケース種別を推測
    stem = case_file.stem.replace("p1-test-migration-", "").replace("-cases", "")
    return CASE_RULES.get(stem, ("Prelude", "chapter1.syntax", "docs/spec/1-1-syntax.md"))


def parse_case_lines(case_file: pathlib.Path) -> Iterable[Tuple[str, str]]:
    for line in case_file.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split("::")
        if len(parts) >= 3:
            scenario_id, _, input_path = parts[:3]
            yield scenario_id.strip(), input_path.strip()


def build_row(
    scenario_id: str,
    category: str,
    spec_chapter: str,
    spec_anchor: str,
    case_file: pathlib.Path,
    input_path: str,
) -> Dict[str, str]:
    notes = (
        f"{case_file.relative_to(REPO_ROOT)} から自動取り込み。"
        "Phase3 の型推論成果物を Phase4 マトリクスで追跡するための暫定行。"
    )
    return {
        "scenario_id": scenario_id,
        "category": category,
        "spec_chapter": spec_chapter,
        "spec_anchor": spec_anchor,
        "variant": "legacy",
        "priority": "medium",
        "input_path": input_path,
        "expected": "",
        "diagnostic_keys": "[]",
        "capability": "none",
        "stage_requirement": "not-required",
        "scenario_notes": notes,
        "resolution": "pending",
        "resolution_notes": "auto-import",
        "spec_vs_impl_decision": "pending",
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Phase4 シナリオマトリクス自動登録")
    parser.add_argument(
        "--cases-glob",
        default=DEFAULT_CASES_GLOB,
        help="読み込むケースファイルの glob パターン（repo ルート基準）",
    )
    parser.add_argument(
        "--matrix",
        default=DEFAULT_MATRIX,
        help="更新対象の CSV パス（repo ルート基準）",
    )
    parser.add_argument(
        "--write",
        action="store_true",
        help="dry-run ではなく CSV へ追記する",
    )
    args = parser.parse_args()

    matrix_path = (REPO_ROOT / args.matrix).resolve()
    existing = load_existing_ids(matrix_path)

    new_rows: List[Dict[str, str]] = []
    for case_file in iter_case_files(args.cases_glob):
        category, spec_chapter, spec_anchor = infer_mapping(case_file)
        for scenario_id, input_path in parse_case_lines(case_file):
            if scenario_id in existing:
                continue
            row = build_row(
                scenario_id=scenario_id,
                category=category,
                spec_chapter=spec_chapter,
                spec_anchor=spec_anchor,
                case_file=case_file,
                input_path=input_path,
            )
            new_rows.append(row)

    if not new_rows:
        print("追加対象のシナリオはありません。")
        return

    if args.write:
        with matrix_path.open("a", newline="", encoding="utf-8") as fh:
            writer = csv.DictWriter(fh, fieldnames=COLUMN_ORDER)
            for row in new_rows:
                writer.writerow(row)
        print(f"{len(new_rows)} 行を {matrix_path.relative_to(REPO_ROOT)} に追記しました。")
    else:
        import sys

        writer = csv.DictWriter(sys.stdout, fieldnames=COLUMN_ORDER)
        writer.writeheader()
        for row in new_rows:
            writer.writerow(row)


if __name__ == "__main__":
    main()
