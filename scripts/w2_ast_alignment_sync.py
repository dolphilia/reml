#!/usr/bin/env python3
"""
W2 AST/IR 対応表タスク用に既存 dual-write 出力を
`reports/dual-write/front-end/w2-ast-alignment/` へ再配置するスクリプト。

既存の PoC 実行結果
(`reports/dual-write/front-end/poc/2025-11-07-w2-ast-inventory/`) を入力とし、
ケースごとの成果物ディレクトリと検証用 bundle JSON を生成する。
"""

from __future__ import annotations

import json
import shutil
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List

REPO_ROOT = Path(__file__).resolve().parents[1]
CASES_FILE = REPO_ROOT / "docs/plans/rust-migration/appendix/w2-dualwrite-cases.txt"
SOURCE_DIR = (
    REPO_ROOT / "reports/dual-write/front-end/poc/2025-11-07-w2-ast-inventory"
)
TARGET_DIR = REPO_ROOT / "reports/dual-write/front-end/w2-ast-alignment"


def load_json(path: Path):
    if not path.exists() or path.stat().st_size == 0:
        return {}
    text = path.read_text(encoding="utf-8")
    try:
        return json.loads(text)
    except json.JSONDecodeError as exc:
        raise ValueError(f"Failed to parse JSON: {path}") from exc


def write_json(path: Path, payload: Dict):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as fh:
        json.dump(payload, fh, ensure_ascii=False, indent=2)
        fh.write("\n")


@dataclass
class CaseEntry:
    name: str
    mode: str
    value: str


def parse_cases() -> List[CaseEntry]:
    entries: List[CaseEntry] = []
    for raw in CASES_FILE.read_text(encoding="utf-8").splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        parts = line.split("::")
        if len(parts) != 3:
            raise ValueError(f"Invalid case definition: {raw}")
        entries.append(CaseEntry(name=parts[0].strip(), mode=parts[1], value=parts[2]))
    return entries


def copy_file(src: Path, dest: Path):
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dest)


def main() -> int:
    cases = parse_cases()
    if not SOURCE_DIR.is_dir():
        raise SystemExit(f"source directory missing: {SOURCE_DIR}")

    if TARGET_DIR.exists():
        shutil.rmtree(TARGET_DIR)
    TARGET_DIR.mkdir(parents=True)

    summary_rows: List[Dict] = []

    for entry in cases:
        case_dir = TARGET_DIR / entry.name
        case_dir.mkdir(parents=True, exist_ok=True)
        prefix = entry.name

        def src(name: str) -> Path:
            return SOURCE_DIR / f"{prefix}.{name}"

        # Required source files
        copy_file(SOURCE_DIR / f"{prefix}.reml", case_dir / "input.reml")
        ocaml_diag = load_json(src("ocaml.diagnostics.json"))
        if "diagnostics" not in ocaml_diag:
            ocaml_diag = {"diagnostics": []}
        write_json(case_dir / "diagnostics.ocaml.json", ocaml_diag)
        ocaml_parse = load_json(src("ocaml.parse-debug.json"))
        write_json(case_dir / "parse-debug.ocaml.json", ocaml_parse)
        rust_parse = load_json(src("rust.parse-debug.json"))
        write_json(case_dir / "parse-debug.rust.json", rust_parse)
        rust_payload = load_json(src("rust.json"))
        write_json(case_dir / "rust.payload.json", rust_payload)
        legacy_summary = load_json(src("summary.json"))
        write_json(case_dir / "summary.legacy.json", legacy_summary)

        source_info_path = SOURCE_DIR / f"{prefix}.source.txt"
        source_info = (
            source_info_path.read_text(encoding="utf-8").strip()
            if source_info_path.exists()
            else entry.value
        )

        ocaml_ast_text = (src("ocaml.ast.txt")).read_text(encoding="utf-8")
        write_json(
            case_dir / "ast.ocaml.json",
            {"format": "text", "content": ocaml_ast_text},
        )

        ocaml_tast_text = (src("ocaml.tast.txt")).read_text(encoding="utf-8")
        write_json(
            case_dir / "typed-ast.ocaml.json",
            {"format": "text", "content": ocaml_tast_text},
        )

        rust_payload = load_json(case_dir / "rust.payload.json")
        write_json(
            case_dir / "ast.rust.json",
            {
                "format": "render_string",
                "render": rust_payload.get("ast_render"),
                "has_ast": rust_payload.get("ast_render") is not None,
            },
        )
        write_json(
            case_dir / "typed-ast.rust.json",
            {
                "status": "unavailable",
                "reason": "Rust フロントエンド PoC は Typed AST 出力に未対応です。",
            },
        )

        rust_diag = {"diagnostics": rust_payload.get("diagnostics", [])}
        write_json(case_dir / "diagnostics.rust.json", rust_diag)

        bundle = {
            "case": entry.name,
            "source": source_info,
            "diagnostics": rust_payload.get("diagnostics", []),
            "baseline": {
                "input": ocaml_parse.get("input"),
                "run_config": ocaml_parse.get("parser_run_config"),
                "parse_result": ocaml_parse.get("parse_result"),
                "stream_meta": ocaml_parse.get("stream_meta"),
                "diagnostics": ocaml_diag.get("diagnostics", []),
            },
            "candidate": {
                "input": rust_parse.get("input"),
                "run_config": rust_parse.get("run_config"),
                "parse_result": rust_parse.get("parse_result"),
                "stream_meta": rust_parse.get("stream_meta"),
                "diagnostics": rust_payload.get("diagnostics", []),
            },
            "tokens": rust_payload.get("tokens", []),
            "ast_render": rust_payload.get("ast_render"),
        }
        write_json(case_dir / "dualwrite.bundle.json", bundle)

        summary_rows.append(
            {
                "case": entry.name,
                "source": source_info,
                "ast_match": legacy_summary.get("ast_match"),
                "diag_match": legacy_summary.get("diag_match"),
                "ocaml_diag": legacy_summary.get("ocaml_diag_count"),
                "rust_diag": legacy_summary.get("rust_diag_count"),
                "ocaml_packrat": legacy_summary.get("ocaml_packrat_queries"),
                "rust_packrat": legacy_summary.get("rust_packrat_queries"),
            }
        )

    # Copy legacy summaries for reference.
    for name in ("summary.md", "summary_report.md", "summary_report.json"):
        src_path = SOURCE_DIR / name
        if src_path.exists():
            copy_file(src_path, TARGET_DIR / name)

    # Write consolidated summary table for the new layout.
    lines = [
        "| case | source | ast_match | diag_match | ocaml_diag | rust_diag | ocaml_packrat | rust_packrat |",
        "| --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for row in summary_rows:
        lines.append(
            f"| {row['case']} | {row['source']} | {row['ast_match']} | {row['diag_match']} | "
            f"{row['ocaml_diag']} | {row['rust_diag']} | {row['ocaml_packrat']} | {row['rust_packrat']} |"
        )
    (TARGET_DIR / "summary.alignment.md").write_text(
        "\n".join(lines) + "\n", encoding="utf-8"
    )

    README = """# W2 AST/IR dual-write alignment

`w2-ast-alignment/` は W2 タスクで利用した dual-write 入力ケースを
ケース単位で整理した検証用データセットです。各サブディレクトリには

- `input.reml`: 実行に使用したソース
- `diagnostics.{ocaml,rust}.json`: CLI 出力から抽出した診断
- `parse-debug.{ocaml,rust}.json`: Packrat / span_trace / run_config 付きのデバッグ情報
- `ast.{ocaml,rust}.json`, `typed-ast.{ocaml,rust}.json`: AST/Typed AST の一次データ
- `dualwrite.bundle.json`: `collect-iterator-audit-metrics.py` に渡せる baseline/candidate まとめ

が含まれます。`metrics/` ディレクトリには `collect-iterator-audit-metrics.py`
の実行結果を配置してください。
"""
    (TARGET_DIR / "README.md").write_text(README, encoding="utf-8")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
