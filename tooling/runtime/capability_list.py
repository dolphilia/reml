#!/usr/bin/env python3
"""Capability Registry 一覧を JSON/Markdown へ変換しドキュメントへ反映するスクリプト。"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import subprocess
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_CLI = REPO_ROOT / "compiler/runtime/target/debug/reml_capability"
DEFAULT_SPEC = REPO_ROOT / "docs/spec/3-8-core-runtime-capability.md"
DEFAULT_README = REPO_ROOT / "docs/plans/bootstrap-roadmap/README.md"
DEFAULT_LOG = REPO_ROOT / "docs/notes/runtime/runtime-capability-stage-log.md"
GENERATOR = REPO_ROOT / "scripts/capability/generate_md.py"
DEFAULT_JSON_DIR = REPO_ROOT / "reports/spec-audit/ch3"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Capability Registry CLI を呼び出し、Markdown テーブルを更新します。",
    )
    parser.add_argument(
        "--cli",
        default=str(DEFAULT_CLI),
        help="`reml_capability` バイナリのパス",
    )
    parser.add_argument(
        "--spec",
        default=str(DEFAULT_SPEC),
        help="更新対象の仕様書パス（既定: docs/spec/3-8...）",
    )
    parser.add_argument(
        "--readme",
        default=str(DEFAULT_README),
        help="更新対象の README/計画書パス",
    )
    parser.add_argument(
        "--log",
        default=str(DEFAULT_LOG),
        help="更新履歴を追記するノートのパス",
    )
    parser.add_argument(
        "--json-output",
        help="CLI 出力を保存する JSON ファイル。未指定時は `reports/spec-audit/ch3/capability_list-<date>.json`",
    )
    return parser.parse_args()


def run_cli(cli_path: Path) -> dict:
    result = subprocess.run(
        [str(cli_path), "list", "--format", "json"],
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def ensure_generator() -> None:
    if not GENERATOR.exists():
        raise SystemExit(f"Markdown 生成スクリプトが見つかりません: {GENERATOR}")


def write_json(data: dict, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n")


def update_document(json_path: Path, target: Path) -> None:
    subprocess.run(
        [
            "python3",
            str(GENERATOR),
            "--json",
            str(json_path),
            "--output",
            str(target),
        ],
        check=True,
    )


def append_log(log_path: Path, cli: Path, json_path: Path, spec: Path, readme: Path) -> None:
    timestamp = dt.datetime.utcnow().strftime("%Y-%m-%d %H:%M:%S UTC")
    rel_cli = cli.resolve().relative_to(REPO_ROOT)
    rel_json = json_path.resolve().relative_to(REPO_ROOT)
    rel_spec = spec.resolve().relative_to(REPO_ROOT)
    rel_readme = readme.resolve().relative_to(REPO_ROOT)
    entry = (
        f"- {timestamp}: CLI `{rel_cli}` → JSON `{rel_json}`、"
        f"docs `{rel_spec}`, `{rel_readme}` を更新\n"
    )
    if log_path.exists():
        text = log_path.read_text()
    else:
        text = "# Runtime Capability Stage ログ\n"
    marker = "## Capability List Update"
    if marker not in text:
        text = text.rstrip() + f"\n\n{marker}\n\n"
    text = text.rstrip() + "\n" + entry
    log_path.write_text(text)


def main() -> None:
    args = parse_args()
    cli_path = Path(args.cli)
    ensure_generator()
    data = run_cli(cli_path)
    json_path = (
        Path(args.json_output)
        if args.json_output
        else DEFAULT_JSON_DIR
        / f"capability_list-{dt.datetime.utcnow().strftime('%Y%m%d')}.json"
    )
    write_json(data, json_path)
    spec_path = Path(args.spec)
    readme_path = Path(args.readme)
    update_document(json_path, spec_path)
    update_document(json_path, readme_path)
    append_log(Path(args.log), cli_path, json_path, spec_path, readme_path)


if __name__ == "__main__":
    main()
