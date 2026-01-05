#!/usr/bin/env python3
"""Capability Registry の JSON 出力を Markdown テーブルへ変換するユーティリティ。"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Dict, List


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Capability Registry の JSON を Markdown テーブルへ変換します。",
    )
    parser.add_argument(
        "--json",
        required=True,
        type=Path,
        help="`reml_capability list --format json` の結果を書き出した JSON ファイル",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="Markdown テーブルを書き込むドキュメントのパス。未指定時は標準出力へ出力します。",
    )
    parser.add_argument(
        "--start-marker",
        default="<!-- capability-table:start -->",
        help="ドキュメント内でテーブル挿入を開始するマーカー",
    )
    parser.add_argument(
        "--end-marker",
        default="<!-- capability-table:end -->",
        help="ドキュメント内でテーブル挿入を終了するマーカー",
    )
    return parser.parse_args()


def load_capabilities(json_path: Path) -> List[Dict[str, Any]]:
    payload = json.loads(json_path.read_text())
    capabilities = payload.get("capabilities")
    if not isinstance(capabilities, list):
        raise SystemExit(f"{json_path} に capabilities 配列が存在しません")
    return capabilities


def provider_label(entry: Dict[str, Any]) -> str:
    provider = entry.get("provider") or {}
    kind = provider.get("kind")
    if kind == "core":
        return "Core"
    if kind == "plugin":
        package = provider.get("package", "")
        version = provider.get("version")
        return f"Plugin `{package}`" + (f" @{version}" if version else "")
    if kind == "external_bridge":
        name = provider.get("name", "")
        version = provider.get("version")
        return f"Bridge `{name}`" + (f" @{version}" if version else "")
    if kind == "runtime_component":
        name = provider.get("name", "")
        return f"Runtime `{name}`"
    return kind or "Unknown"


def effect_scope_label(entry: Dict[str, Any]) -> str:
    scope = entry.get("effect_scope") or []
    if not scope:
        return "(none)"
    return "<br>".join(f"`{tag}`" for tag in scope)


def manifest_label(entry: Dict[str, Any]) -> str:
    manifest = entry.get("manifest_path")
    if not manifest:
        return "-"
    return f"`{manifest}`"


def generate_table(capabilities: List[Dict[str, Any]]) -> str:
    rows = [
        "| Capability | Stage | Effect Scope | Provider | Manifest Path |",
        "| --- | --- | --- | --- | --- |",
    ]
    for entry in capabilities:
        rows.append(
            "| `{id}` | `{stage}` | {effects} | {provider} | {manifest} |".format(
                id=entry.get("id", ""),
                stage=entry.get("stage", ""),
                effects=effect_scope_label(entry),
                provider=provider_label(entry),
                manifest=manifest_label(entry),
            )
        )
    return "\n".join(rows) + "\n"


def update_document(path: Path, table: str, start_marker: str, end_marker: str) -> None:
    text = path.read_text()
    start = text.find(start_marker)
    if start == -1:
        raise SystemExit(f"{path} に start marker `{start_marker}` が見つかりません")
    start += len(start_marker)
    end = text.find(end_marker, start)
    if end == -1:
        raise SystemExit(f"{path} に end marker `{end_marker}` が見つかりません")
    before = text[:start]
    after = text[end:]
    if not before.endswith("\n"):
        before += "\n"
    table_block = table.rstrip() + "\n"
    new_text = before + table_block + after
    path.write_text(new_text)


def main() -> None:
    args = parse_args()
    capabilities = load_capabilities(args.json)
    table = generate_table(capabilities)
    if args.output:
        update_document(args.output, table, args.start_marker, args.end_marker)
    else:
        print(table, end="")


if __name__ == "__main__":
    main()
