#!/usr/bin/env python3
"""
監査ログのサマリーファイル (index.json) を生成する補助スクリプト。

典型的な利用例:

```
python3 tooling/ci/create-audit-index.py \
  --output reports/audit/index.json \
  --audit ci:linux-x86_64:tooling/ci/ffi-audit/linux/cli-callconv-unsupported.audit.jsonl:success:full:1.0 \
  --audit ci:stage:tooling/ci/ffi-audit/stage.audit.jsonl:success:full:1.0 \
  --skip-missing
```

- `--audit` は `profile:target:path[:status[:audit_level[:pass_rate]]]` 形式。
- `--skip-missing` を指定すると存在しないファイルは警告を出してスキップ。
- `build_id` や `timestamp` は GitHub Actions の環境変数を利用しつつ、ローカル実行にも対応。
"""

from __future__ import annotations

import argparse
import json
import os
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Sequence, Tuple


DEFAULT_STATUS = "unknown"
DEFAULT_AUDIT_LEVEL = "full"


@dataclass
class AuditSpec:
    profile: str
    target: str
    path: Path
    status: str = DEFAULT_STATUS
    audit_level: str = DEFAULT_AUDIT_LEVEL
    pass_rate: Optional[float] = None


def parse_args(argv: Optional[Sequence[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="監査ログ index.json 生成ツール")
    parser.add_argument(
        "--audit",
        action="append",
        dest="audits",
        help="profile:target:path[:status[:audit_level[:pass_rate]]]",
    )
    parser.add_argument(
        "--output",
        required=True,
        type=Path,
        help="生成する index.json のパス",
    )
    parser.add_argument(
        "--build-id",
        type=str,
        default=os.environ.get("GITHUB_RUN_ID", "local"),
        help="index.json に記録する build_id（既定: GITHUB_RUN_ID または local）",
    )
    parser.add_argument(
        "--commit",
        type=str,
        default=os.environ.get("GITHUB_SHA"),
        help="コミットハッシュ（省略可）",
    )
    parser.add_argument(
        "--timestamp",
        type=str,
        default=None,
        help="ISO8601 タイムスタンプ（指定しない場合は現在時刻）",
    )
    parser.add_argument(
        "--skip-missing",
        action="store_true",
        help="存在しないファイルは警告を出してスキップする",
    )
    parser.add_argument(
        "--pruned",
        type=str,
        nargs="*",
        default=[],
        help="pruned リストへ追記する build_id",
    )
    return parser.parse_args(argv)


def parse_audit_spec(text: str) -> AuditSpec:
    parts = text.split(":")
    if len(parts) < 3:
        raise ValueError(f"--audit の形式が不正です: {text}")

    profile = parts[0] or "ci"
    target = parts[1] or "<unknown>"
    raw_path = ":".join(parts[2:3]) if len(parts) >= 3 else parts[2]
    path = Path(raw_path)
    status = parts[3] if len(parts) >= 4 and parts[3] else DEFAULT_STATUS
    audit_level = parts[4] if len(parts) >= 5 and parts[4] else DEFAULT_AUDIT_LEVEL
    pass_rate = None
    if len(parts) >= 6 and parts[5]:
        try:
            pass_rate = float(parts[5])
        except ValueError:
            raise ValueError(f"--audit の pass_rate が不正です: {text}")

    return AuditSpec(
        profile=profile,
        target=target,
        path=path,
        status=status,
        audit_level=audit_level,
        pass_rate=pass_rate,
    )


def ensure_parent_dir(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)


def iso_timestamp(explicit: Optional[str] = None) -> str:
    if explicit:
        return explicit
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def build_entry(spec: AuditSpec, build_id: str, timestamp: str, commit: Optional[str]) -> Tuple[Optional[Dict], Optional[str]]:
    if not spec.path.exists():
        return None, f"[warning] 監査ログが見つかりません: {spec.path}"

    try:
        stat = spec.path.stat()
        size = stat.st_size
    except OSError as exc:
        return None, f"[warning] 監査ログの stat 取得に失敗しました: {spec.path} ({exc})"

    entry = {
        "build_id": build_id,
        "timestamp": timestamp,
        "profile": spec.profile,
        "audit_store": spec.profile,
        "target": spec.target,
        "audit_level": spec.audit_level,
        "path": str(spec.path),
        "status": spec.status,
        "pass_rate": spec.pass_rate,
        "size_bytes": str(size),
    }
    if commit:
        entry["commit"] = commit
    return entry, None


def aggregate_retained(entries: Iterable[Dict]) -> List[Dict]:
    summary: Dict[Tuple[str, str], Tuple[int, int]] = {}
    for entry in entries:
        profile = entry.get("profile") or "ci"
        target = entry.get("target") or "<unknown>"
        key = (str(profile), str(target))
        count, size = summary.get(key, (0, 0))
        try:
            size_value = int(entry.get("size_bytes") or 0)
        except (TypeError, ValueError):
            size_value = 0
        summary[key] = (count + 1, size + size_value)

    return [
        {"profile": profile, "target": target, "count": count, "size_bytes": size}
        for (profile, target), (count, size) in sorted(summary.items())
    ]


def main(argv: Optional[Sequence[str]] = None) -> int:
    args = parse_args(argv)

    timestamp = iso_timestamp(args.timestamp)
    specs: List[AuditSpec] = []
    warnings: List[str] = []

    for text in args.audits or []:
        try:
            specs.append(parse_audit_spec(text))
        except ValueError as exc:
            warnings.append(f"[error] {exc}")

    if warnings:
        for warning in warnings:
            print(warning)
        if any(w.startswith("[error]") for w in warnings):
            return 1

    entries: List[Dict] = []
    for spec in specs:
        entry, warning = build_entry(spec, args.build_id, timestamp, args.commit)
        if entry is None:
            if warning and not args.skip_missing:
                print(warning)
                return 1
            if warning:
                print(warning)
            continue
        entries.append(entry)

    index = {
        "entries": entries,
        "retained_entries": aggregate_retained(entries),
        "pruned": args.pruned,
    }

    ensure_parent_dir(args.output)
    with args.output.open("w", encoding="utf-8") as handle:
        json.dump(index, handle, indent=2, ensure_ascii=False)
        handle.write("\n")

    print(f"[info] 監査インデックスを生成しました: {args.output} (entries={len(entries)})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
