#!/usr/bin/env python3
"""
監査ログインデックスと付随ファイルの健全性を検証するユーティリティ。

機能:
  * `reports/audit/index.json` に記録された各エントリの必須フィールド検証
  * 監査ログファイル（JSON / JSON Lines / gzipped JSON Lines）のパースと
    拡張メタデータ（bridge/effect/typeclass/parse）の欠落チェック
  * `retained_entries` 要約の再計算および不整合検出
  * 履歴ディレクトリ内の `.jsonl.gz` が正しい JSON を含むか確認

戻り値:
  0 : 全検証成功
  1 : いずれかの検証が失敗
"""

from __future__ import annotations

import argparse
import gzip
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Sequence, Tuple


COMMON_REQUIRED_KEYS: Sequence[str] = ("cli.audit_id", "cli.change_set", "schema.version")

BRIDGE_REQUIRED_KEYS: Sequence[str] = (
    "bridge.status",
    "bridge.target",
    "bridge.arch",
    "bridge.abi",
    "bridge.ownership",
    "bridge.extern_symbol",
    "bridge.platform",
    "bridge.return.ownership",
    "bridge.return.status",
    "bridge.return.wrap",
    "bridge.return.release_handler",
    "bridge.return.rc_adjustment",
)

BRIDGE_SHOULD_KEYS: Sequence[str] = ("bridge.audit_pass_rate", "bridge.expected_abi", "bridge.callconv")

EFFECT_REQUIRED_KEYS: Sequence[str] = (
    "effect.stage.required",
    "effect.stage.actual",
    "effect.capability",
)

EFFECT_ITERATOR_REQUIRED_KEYS: Sequence[str] = (
    "effect.stage.iterator.required",
    "effect.stage.iterator.actual",
    "effect.stage.iterator.kind",
    "effect.stage.iterator.capability",
    "effect.stage.iterator.source",
)

EFFECT_SHOULD_KEYS: Sequence[str] = (
    "effect.residual",
    "effect.handler_stack",
    "effect.unhandled_operations",
    "effect.capability_descriptor",
)

TYPECLASS_REQUIRED_KEYS: Sequence[str] = ("typeclass.constraint", "typeclass.resolution_state")
TYPECLASS_SHOULD_KEYS: Sequence[str] = (
    "typeclass.dictionary",
    "typeclass.candidates",
    "typeclass.pending",
    "typeclass.generalized_typevars",
    "typeclass.graph.export_dot",
)

PARSE_REQUIRED_KEYS: Sequence[str] = ("parse.input_name", "parse.stage_trace")


@dataclass
class Issue:
    severity: str  # "error" or "warning"
    message: str


def parse_args(argv: Optional[Sequence[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="監査ログインデックスの検証ツール")
    parser.add_argument(
        "--index",
        required=True,
        type=Path,
        help="検証対象となる audit index JSON (例: reports/audit/index.json)",
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path("."),
        help="監査ファイルの探索ベースディレクトリ（既定: カレントディレクトリ）",
    )
    parser.add_argument(
        "--history-dir",
        type=Path,
        default=Path("reports/audit/history"),
        help="履歴ファイル (jsonl.gz) を格納したディレクトリ。存在しない場合はスキップ。",
    )
    parser.add_argument(
        "--max-files",
        type=int,
        default=1000,
        help="各監査ファイル内で検査する最大イベント数（既定: 1000）",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="\"should\" レベルの欠落もエラーとして扱う",
    )
    return parser.parse_args(argv)


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        try:
            return json.load(handle)
        except json.JSONDecodeError as exc:
            raise ValueError(f"JSON の解析に失敗しました: {path} ({exc})") from exc


def load_audit_entries(path: Path, limit: int) -> List[Dict[str, Any]]:
    def _parse_stream(stream: Iterable[str]) -> List[Dict[str, Any]]:
        entries: List[Dict[str, Any]] = []
        for line_no, raw in enumerate(stream, start=1):
            if limit and len(entries) >= limit:
                break
            line = raw.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError as exc:  # pragma: no cover
                raise ValueError(f"JSON Lines (行 {line_no}) の解析に失敗しました: {path}: {exc}") from exc
            if isinstance(obj, dict):
                entries.append(obj)
        return entries

    suffix = "".join(path.suffixes)
    if suffix.endswith(".gz"):
        with gzip.open(path, "rt", encoding="utf-8") as gz:
            return _parse_stream(gz)

    if path.suffix.lower() == ".jsonl":
        with path.open("r", encoding="utf-8") as handle:
            return _parse_stream(handle)

    if path.suffix.lower() == ".json":
        data = load_json(path)
        if isinstance(data, list):
            return [entry for entry in data if isinstance(entry, dict)][:limit or None]
        if isinstance(data, dict):
            return [data]
        raise ValueError(f"監査ファイルが JSON オブジェクト/配列ではありません: {path}")

    # フォールバック: JSON Lines として扱う
    with path.open("r", encoding="utf-8") as handle:
        return _parse_stream(handle)


def is_empty(value: Any) -> bool:
    if value is None:
        return True
    if isinstance(value, str):
        return value.strip() == ""
    if isinstance(value, (list, dict)):
        return len(value) == 0
    return False


def parse_int(value: Any) -> Optional[int]:
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        return int(value)
    if isinstance(value, str):
        try:
            return int(value, 10)
        except ValueError:
            return None
    return None


def expand_metadata(metadata: Dict[str, Any]) -> Dict[str, Any]:
    expanded: Dict[str, Any] = {}
    for key, value in metadata.items():
        parts = key.split(".")
        current: Dict[str, Any] = expanded
        for index, part in enumerate(parts):
            is_last = index == len(parts) - 1
            if is_last:
                existing = current.get(part)
                if isinstance(existing, dict) and isinstance(value, dict):
                    # マージ（既存 dict を上書き）.
                    merged = existing.copy()
                    merged.update(value)
                    current[part] = merged
                else:
                    current[part] = value
            else:
                next_value = current.get(part)
                if not isinstance(next_value, dict):
                    next_value = {}
                    current[part] = next_value
                current = next_value
    return expanded


def get_nested(original: Dict[str, Any], expanded: Dict[str, Any], dotted_key: str) -> Any:
    if dotted_key in original:
        return original[dotted_key]
    current: Any = expanded
    for part in dotted_key.split("."):
        if isinstance(current, dict) and part in current:
            current = current[part]
        else:
            return None
    return current


def collect_summary(entries: List[Dict[str, Any]]) -> Dict[Tuple[str, str], Tuple[int, int]]:
    summary: Dict[Tuple[str, str], Tuple[int, int]] = {}
    for entry in entries:
        if not isinstance(entry, dict):
            continue
        profile = str(entry.get("profile") or entry.get("audit_store") or "ci")
        target = str(entry.get("target") or "<unknown>")
        key = (profile, target)
        count, size = summary.get(key, (0, 0))
        size_value = parse_int(entry.get("size_bytes"))
        summary[key] = (count + 1, size + (size_value or 0))
    return summary


def collect_recorded_summary(retained: Any) -> Dict[Tuple[str, str], Tuple[int, int]]:
    summary: Dict[Tuple[str, str], Tuple[int, int]] = {}
    if not isinstance(retained, list):
        return summary
    for item in retained:
        if not isinstance(item, dict):
            continue
        profile = str(item.get("profile") or "ci")
        target = str(item.get("target") or "<unknown>")
        count = parse_int(item.get("count")) or 0
        size = parse_int(item.get("size_bytes")) or 0
        summary[(profile, target)] = (count, size)
    return summary


def compare_summary(
    expected: Dict[Tuple[str, str], Tuple[int, int]],
    recorded: Dict[Tuple[str, str], Tuple[int, int]],
) -> List[Issue]:
    issues: List[Issue] = []
    keys = set(expected) | set(recorded)
    for key in sorted(keys):
        exp = expected.get(key, (0, 0))
        rec = recorded.get(key, (0, 0))
        if exp != rec:
            issues.append(
                Issue(
                    "error",
                    f"retained_entries 不整合: {key} expected={exp} recorded={rec}",
                )
            )
    return issues


def check_metadata(
    metadata: Dict[str, Any],
    *,
    strict: bool,
) -> List[Issue]:
    issues: List[Issue] = []

    expanded = expand_metadata(metadata)

    def lookup(key: str) -> Any:
        return get_nested(metadata, expanded, key)

    missing_common = [key for key in COMMON_REQUIRED_KEYS if is_empty(lookup(key))]
    if missing_common:
        issues.append(
            Issue(
                "error",
                f"必須メタデータが不足しています: {', '.join(sorted(missing_common))}",
            )
        )

    def guard_group(keys: Sequence[str]) -> bool:
        return any(lookup(key) is not None for key in keys)

    def ensure_keys(
        group_name: str,
        required: Sequence[str],
        should: Sequence[str],
        keys_for_presence: Sequence[str],
    ) -> None:
        if not guard_group(keys_for_presence):
            return
        missing_required = [key for key in required if is_empty(lookup(key))]
        if missing_required:
            issues.append(
                Issue(
                    "error",
                    f"{group_name} メタデータの必須キーが不足: {', '.join(sorted(missing_required))}",
                )
            )
        missing_should = [key for key in should if is_empty(lookup(key))]
        if missing_should:
            issues.append(
                Issue(
                    "error" if strict else "warning",
                    f"{group_name} メタデータの推奨キーが不足: {', '.join(sorted(missing_should))}",
                )
            )

    ensure_keys(
        "bridge",
        BRIDGE_REQUIRED_KEYS,
        BRIDGE_SHOULD_KEYS,
        tuple(set(BRIDGE_REQUIRED_KEYS) | set(BRIDGE_SHOULD_KEYS)),
    )
    ensure_keys(
        "effect",
        tuple(set(EFFECT_REQUIRED_KEYS) | set(EFFECT_ITERATOR_REQUIRED_KEYS)),
        EFFECT_SHOULD_KEYS,
        tuple(set(EFFECT_REQUIRED_KEYS) | set(EFFECT_ITERATOR_REQUIRED_KEYS) | set(EFFECT_SHOULD_KEYS)),
    )
    ensure_keys(
        "typeclass",
        TYPECLASS_REQUIRED_KEYS,
        TYPECLASS_SHOULD_KEYS,
        tuple(set(TYPECLASS_REQUIRED_KEYS) | set(TYPECLASS_SHOULD_KEYS)),
    )
    ensure_keys(
        "parse",
        PARSE_REQUIRED_KEYS,
        (),
        PARSE_REQUIRED_KEYS,
    )
    return issues


def verify_audit_file(
    entry_path: Path,
    limit: int,
    *,
    strict: bool,
) -> List[Issue]:
    issues: List[Issue] = []
    try:
        events = load_audit_entries(entry_path, limit)
    except ValueError as exc:
        return [Issue("error", str(exc))]

    if not events:
        return [Issue("warning", f"監査エントリが存在しません: {entry_path}")]

    for index, event in enumerate(events):
        if not isinstance(event, dict):
            issues.append(Issue("error", f"{entry_path}:{index} が辞書型ではありません"))
            continue
        metadata = event.get("metadata")
        if not isinstance(metadata, dict):
            issues.append(Issue("error", f"{entry_path}:{index} metadata がオブジェクトではありません"))
            continue
        event_issues = check_metadata(metadata, strict=strict)
        for item in event_issues:
            issues.append(
                Issue(
                    item.severity,
                    f"{entry_path}:{index} {item.message}",
                )
            )
    return issues


def verify_history(history_dir: Path) -> List[Issue]:
    if not history_dir.exists():
        return []
    issues: List[Issue] = []
    for path in sorted(history_dir.glob("*.jsonl.gz")):
        try:
            with gzip.open(path, "rt", encoding="utf-8") as handle:
                for line_no, line in enumerate(handle, start=1):
                    text = line.strip()
                    if not text:
                        continue
                    try:
                        json.loads(text)
                    except json.JSONDecodeError as exc:
                        issues.append(
                            Issue(
                                "error",
                                f"履歴ファイル {path}:{line_no} が JSON として無効です ({exc})",
                            )
                        )
                        break
        except OSError as exc:
            issues.append(Issue("error", f"履歴ファイルの読み込みに失敗しました: {path} ({exc})"))
    return issues


def main(argv: Optional[Sequence[str]] = None) -> int:
    args = parse_args(argv)
    index_path = args.index

    if not index_path.is_file():
        sys.stderr.write(f"index ファイルが見つかりません: {index_path}\n")
        return 1

    try:
        index_data = load_json(index_path)
    except ValueError as exc:
        sys.stderr.write(str(exc) + "\n")
        return 1

    entries = index_data.get("entries")
    if not isinstance(entries, list):
        sys.stderr.write("index JSON に entries 配列が存在しません\n")
        return 1

    issues: List[Issue] = []
    root = args.root.resolve()

    for entry in entries:
        if not isinstance(entry, dict):
            issues.append(Issue("error", "entries 内にオブジェクト以外の項目があります"))
            continue
        path_value = entry.get("path")
        if not isinstance(path_value, str) or not path_value:
            issues.append(Issue("error", f"エントリの path が無効です: {entry}"))
            continue

        entry_path = Path(path_value)
        if not entry_path.is_absolute():
            entry_path = (root / entry_path).resolve()

        if not entry_path.is_file():
            issues.append(Issue("error", f"監査ファイルが存在しません: {entry_path}"))
            continue

        try:
            file_size = entry_path.stat().st_size
        except OSError as exc:
            issues.append(Issue("error", f"監査ファイルの stat に失敗しました: {entry_path} ({exc})"))
            continue

        recorded_size = parse_int(entry.get("size_bytes"))
        if recorded_size is not None and recorded_size != file_size:
            issues.append(
                Issue(
                    "warning",
                    f"size_bytes の記録と実際のファイルサイズが一致しません: {entry_path} (recorded={recorded_size} actual={file_size})",
                )
            )

        issues.extend(verify_audit_file(entry_path, args.max_files, strict=args.strict))

    expected_summary = collect_summary([entry for entry in entries if isinstance(entry, dict)])
    recorded_summary = collect_recorded_summary(index_data.get("retained_entries"))
    issues.extend(compare_summary(expected_summary, recorded_summary))

    issues.extend(verify_history((args.root / args.history_dir).resolve()))

    errors = [issue for issue in issues if issue.severity == "error"]
    warnings = [issue for issue in issues if issue.severity == "warning"]

    for issue in issues:
        stream = sys.stderr if issue.severity == "error" else sys.stdout
        stream.write(f"[{issue.severity}] {issue.message}\n")

    if not issues:
        print("監査メタデータ検証: 問題は検出されませんでした。")
    else:
        print(
            f"監査メタデータ検証: errors={len(errors)} warnings={len(warnings)}",
            file=sys.stdout,
        )

    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
