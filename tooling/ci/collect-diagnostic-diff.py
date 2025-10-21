#!/usr/bin/env python3
"""
診断ゴールデン差分収集スクリプト

Usage:
    tooling/ci/collect-diagnostic-diff.py \
        --baseline compiler/ocaml/tests/golden \
        --actual compiler/ocaml/tests/golden/_actual \
        --format markdown
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Sequence, Tuple


@dataclass
class DiffEntry:
    relative_path: str
    status: str
    added: List[str] = field(default_factory=list)
    removed: List[str] = field(default_factory=list)
    changed: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "file": self.relative_path,
            "status": self.status,
            "added": self.added,
            "removed": self.removed,
            "changed": self.changed,
        }


def load_json(path: Path) -> Any:
    try:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(handle)
    except FileNotFoundError:
        raise
    except json.JSONDecodeError as exc:
        raise ValueError(f"JSON の読み込みに失敗しました: {path}: {exc}") from exc


def iter_golden_files(root: Path) -> Iterable[Tuple[str, Path]]:
    for path in sorted(root.rglob("*.json.golden")):
        try:
            relative = path.relative_to(root)
        except ValueError:
            continue
        yield str(relative).replace("\\", "/"), path


def iter_actual_files(root: Path) -> Iterable[Tuple[str, Path]]:
    for path in sorted(root.rglob("*.actual.json")):
        try:
            relative = path.relative_to(root)
        except ValueError:
            continue
        yield str(relative).replace("\\", "/"), path


def diff_json(
    baseline: Any,
    current: Any,
    path: str = "",
    added: Optional[List[str]] = None,
    removed: Optional[List[str]] = None,
    changed: Optional[List[str]] = None,
) -> Tuple[List[str], List[str], List[str]]:
    added = added if added is not None else []
    removed = removed if removed is not None else []
    changed = changed if changed is not None else []

    if isinstance(baseline, dict) and isinstance(current, dict):
        baseline_keys = set(baseline.keys())
        current_keys = set(current.keys())

        for key in sorted(current_keys - baseline_keys):
            added.append(f"{path}.{key}" if path else key)
        for key in sorted(baseline_keys - current_keys):
            removed.append(f"{path}.{key}" if path else key)

        for key in sorted(baseline_keys & current_keys):
            child_path = f"{path}.{key}" if path else key
            diff_json(baseline[key], current[key], child_path, added, removed, changed)
        return added, removed, changed

    if isinstance(baseline, list) and isinstance(current, list):
        min_size = min(len(baseline), len(current))
        for index in range(min_size):
            child_path = f"{path}[{index}]"
            diff_json(baseline[index], current[index], child_path, added, removed, changed)
        if len(current) > len(baseline):
            for index in range(len(baseline), len(current)):
                added.append(f"{path}[{index}]")
        elif len(baseline) > len(current):
            for index in range(len(current), len(baseline)):
                removed.append(f"{path}[{index}]")
        return added, removed, changed

    if baseline != current:
        changed.append(path or ".")
    return added, removed, changed


def collect_diff_entries(baseline_dir: Path, actual_dir: Path) -> List[DiffEntry]:
    baseline_map = {relative: path for relative, path in iter_golden_files(baseline_dir)}
    actual_map = {relative: path for relative, path in iter_actual_files(actual_dir)}

    entries: List[DiffEntry] = []

    processed: set[str] = set()

    for relative, actual_path in actual_map.items():
        processed.add(relative)
        target_relative = relative.replace(".actual.json", ".json.golden")
        baseline_path = baseline_dir / target_relative

        if not baseline_path.is_file():
            entries.append(DiffEntry(relative_path=target_relative, status="new"))
            continue

        baseline_data = load_json(baseline_path)
        actual_data = load_json(actual_path)

        added, removed, changed = diff_json(baseline_data, actual_data)

        if not added and not removed and not changed:
            continue

        entries.append(
            DiffEntry(
                relative_path=target_relative,
                status="changed",
                added=added,
                removed=removed,
                changed=changed,
            )
        )

    for relative, baseline_path in baseline_map.items():
        actual_relative = relative.replace(".json.golden", ".actual.json")
        if actual_relative in processed:
            continue
        actual_path = actual_dir / actual_relative
        if not actual_path.exists():
            entries.append(DiffEntry(relative_path=relative, status="missing_actual"))

    return entries


def render_markdown(entries: Sequence[DiffEntry]) -> str:
    if not entries:
        return "診断ゴールデンに差分はありません。"

    lines: List[str] = []
    lines.append("## 診断ゴールデン差分サマリー\n")
    summary = summarize_entries(entries)
    lines.append(
        f"- 変更ファイル数: {summary['changed']} / 新規: {summary['new']} / 未収集: {summary['missing_actual']}"
    )
    lines.append("")

    for entry in entries:
        lines.append(f"### {entry.relative_path}")
        lines.append(f"- 状態: {entry.status}")
        if entry.added:
            lines.append(f"- 追加フィールド: {', '.join(entry.added)}")
        if entry.removed:
            lines.append(f"- 削除フィールド: {', '.join(entry.removed)}")
        if entry.changed:
            lines.append(f"- 値の変更: {', '.join(entry.changed)}")
        lines.append("")

    return "\n".join(lines).rstrip() + "\n"


def summarize_entries(entries: Sequence[DiffEntry]) -> Dict[str, int]:
    summary = {"changed": 0, "new": 0, "missing_actual": 0}
    for entry in entries:
        if entry.status in summary:
            summary[entry.status] += 1
    return summary


def write_output(content: str, destination: Optional[Path]) -> None:
    if destination is None:
        sys.stdout.write(content)
        if not content.endswith("\n"):
            sys.stdout.write("\n")
        return
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(content, encoding="utf-8")


def parse_args(argv: Optional[Sequence[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="診断ゴールデン差分を収集します。")
    parser.add_argument("--baseline", required=True, type=Path, help="既存ゴールデンのディレクトリ")
    parser.add_argument("--actual", required=True, type=Path, help="最新出力 (.actual.json) のディレクトリ")
    parser.add_argument("--output", type=Path, help="出力ファイルパス。省略時は標準出力")
    parser.add_argument(
        "--format",
        choices=("json", "markdown"),
        default="json",
        help="出力形式（json または markdown）",
    )
    return parser.parse_args(argv)


def main(argv: Optional[Sequence[str]] = None) -> int:
    args = parse_args(argv)
    baseline_dir: Path = args.baseline
    actual_dir: Path = args.actual

    if not baseline_dir.is_dir():
        sys.stderr.write(f"error: baseline ディレクトリが見つかりません: {baseline_dir}\n")
        return 2
    if not actual_dir.is_dir():
        sys.stderr.write(f"error: actual ディレクトリが見つかりません: {actual_dir}\n")
        return 2

    entries = collect_diff_entries(baseline_dir, actual_dir)

    if args.format == "json":
        content = json.dumps(
            {"entries": [entry.to_dict() for entry in entries], "summary": summarize_entries(entries)},
            indent=2,
            ensure_ascii=False,
        )
        write_output(content + "\n", args.output)
    else:
        content = render_markdown(entries)
        write_output(content, args.output)

    return 0


if __name__ == "__main__":
    sys.exit(main())
