#!/usr/bin/env python3
"""
collector.effect.* キーの一覧を収集する補助スクリプト。

`reports/spec-audit/ch1/core_iter_collectors.{json,audit.jsonl}` 等を走査し、
出現した `collector.effect.*` キーと件数を Markdown 形式で出力する。
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Dict, Iterable, Iterator, List, Sequence, Union

DEFAULT_SOURCES = [
    Path("reports/spec-audit/ch1/core_iter_collectors.json"),
    Path("reports/spec-audit/ch1/core_iter_collectors.audit.jsonl"),
]


def iter_nodes(value: object) -> Iterator[Dict[str, object]]:
    if isinstance(value, dict):
        yield value
        for child in value.values():
            yield from iter_nodes(child)
    elif isinstance(value, list):
        for item in value:
            yield from iter_nodes(item)


def load_documents(path: Path) -> Iterable[object]:
    if path.suffix == ".jsonl":
        with path.open("r", encoding="utf-8") as handle:
            for line in handle:
                line = line.strip()
                if not line:
                    continue
                yield json.loads(line)
        return
    with path.open("r", encoding="utf-8") as handle:
        yield json.load(handle)


def collect_keys(sources: Sequence[Path]) -> Counter[str]:
    counts: Counter[str] = Counter()
    for source in sources:
        for document in load_documents(source):
            for node in iter_nodes(document):
                for key in node.keys():
                    if key.startswith("collector.effect."):
                        counts[key] += 1
    return counts


def format_markdown(counts: Counter[str], sources: Sequence[Path], repo_root: Path) -> str:
    lines: List[str] = []
    lines.append("# collector.effect.* キー一覧")
    lines.append("")
    lines.append("参照元:")
    for source in sources:
        rel = source.relative_to(repo_root)
        lines.append(f"- `{rel}`")
    lines.append("")
    lines.append("| Key | 出現回数 |")
    lines.append("| --- | ---: |")
    for key in sorted(counts.keys()):
        lines.append(f"| `{key}` | {counts[key]} |")
    lines.append("")
    lines.append(
        "_Generated with `python3 scripts/tools/list_collector_keys.py --output docs/plans/bootstrap-roadmap/assets/collector-effect-keys.md`_"
    )
    lines.append("")
    return "\n".join(lines)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="collector.effect.* キーの一覧を取得する",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="Markdown 形式で書き出すパス。省略時は stdout へ出力。",
    )
    parser.add_argument(
        "sources",
        nargs="*",
        type=Path,
        help="解析対象の JSON / JSONL ファイル（省略時は core_iter_collectors を使用）",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    repo_root = Path(__file__).resolve().parents[2]
    sources: List[Path] = list(args.sources) if args.sources else DEFAULT_SOURCES.copy()
    sources = [source if source.is_absolute() else repo_root / source for source in sources]
    missing = [str(path) for path in sources if not path.exists()]
    if missing:
        missing_str = ", ".join(missing)
        raise FileNotFoundError(f"Source file(s) not found: {missing_str}")
    counts = collect_keys(sources)
    markdown = format_markdown(counts, sources, repo_root)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(markdown, encoding="utf-8")
    else:
        print(markdown)


if __name__ == "__main__":
    main()
