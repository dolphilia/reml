#!/usr/bin/env python3

"""
WS5 Step2: 大入力（1KB/100KB/10MB）を生成し、Phase4 既存 JSON 出力をログ化する。

目的:
- CP-WS5-001（大入力オーダー異常）に向けて、入力サイズ別の増え方を観測できるログを残す。
- 10MB 本体は `expected/` にコミットせず、生成物は `reports/` 配下へ出力して追跡する。

出力（固定）:
- 生成した入力: reports/spec-audit/ch4/generated/ws5/CP-WS5-001/core-parse-large-input-order.<size>.reml
- 実行ログ(JSON): reports/spec-audit/ch4/logs/spec_core-CP-WS5-001-<size>-<timestamp>.diagnostic.json

前提:
- cargo / Rust toolchain がローカルに存在すること
"""

from __future__ import annotations

import argparse
import json
import math
import subprocess
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterable, Sequence


DEFAULT_SIZES = ("1kb", "100kb", "10mb")
ANCHOR_INPUT = Path(
    "examples/spec_core/chapter2/parser_core/core-parse-large-input-order.reml"
)
NOTES_PATH = Path(
    "expected/spec_core/chapter2/parser_core/core-parse-large-input-order.expected.md"
)

PADDING_MARKER = "WS5-LARGE-INPUT-PADDING"


@dataclass(frozen=True)
class SizeSpec:
    label: str
    target_bytes: int


def parse_size(label: str) -> SizeSpec:
    normalized = label.strip().lower()
    if normalized.endswith("kb"):
        n = int(normalized[:-2])
        return SizeSpec(label=normalized, target_bytes=n * 1024)
    if normalized.endswith("mb"):
        n = int(normalized[:-2])
        return SizeSpec(label=normalized, target_bytes=n * 1024 * 1024)
    raise ValueError(f"未知のサイズ指定です: {label} (例: 1kb, 100kb, 10mb)")


def utc_stamp() -> str:
    return datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def repo_root_from_script(script_path: Path) -> Path:
    # tooling/examples/gen_ws5_large_input.py -> repo root
    return script_path.resolve().parents[2]


def find_padding_line(lines: Sequence[str]) -> str:
    for idx, line in enumerate(lines):
        if PADDING_MARKER in line:
            # 期待: 次の行が padding 本体
            if idx + 1 >= len(lines):
                raise SystemExit(f"{PADDING_MARKER} の次行が見つかりません")
            padding = lines[idx + 1]
            if not padding.strip():
                raise SystemExit(f"{PADDING_MARKER} の次行が空行です")
            return padding
    raise SystemExit(f"{PADDING_MARKER} が見つかりません: {ANCHOR_INPUT}")


def generate_large_input(anchor_text: str, target_bytes: int) -> str:
    # UTF-8 バイト長で制御する（ファイルサイズの目安）。
    lines = anchor_text.splitlines(keepends=True)
    padding_line = find_padding_line(lines)
    padding_bytes = padding_line.encode("utf-8")
    if not padding_bytes:
        raise SystemExit("padding 行のバイト長が 0 です")

    current_bytes = anchor_text.encode("utf-8")
    if len(current_bytes) >= target_bytes:
        return anchor_text

    need = target_bytes - len(current_bytes)
    repeats = int(math.ceil(need / len(padding_bytes)))

    out_lines: list[str] = []
    inserted = False
    for line in lines:
        out_lines.append(line)
        if (not inserted) and (PADDING_MARKER in line):
            # marker の直後に padding 行を追加する（marker 自体は残す）
            out_lines.extend([padding_line] * repeats)
            inserted = True

    if not inserted:
        raise SystemExit(f"{PADDING_MARKER} が見つかりません（生成に失敗）")
    return "".join(out_lines)


def run_reml_frontend(
    root: Path, input_path: Path, extra_args: Sequence[str] | None = None
) -> str:
    manifest_path = root / "compiler" / "rust" / "frontend" / "Cargo.toml"
    extra_args = list(extra_args or [])
    cmd: Sequence[str] = (
        "cargo",
        "run",
        "--quiet",
        "--manifest-path",
        str(manifest_path),
        "--bin",
        "reml_frontend",
        "--",
        "--output",
        "json",
        *extra_args,
        str(input_path),
    )
    print(f"==> running CP-WS5-001: {input_path.relative_to(root).as_posix()}")
    completed = subprocess.run(
        cmd,
        cwd=root,
        capture_output=True,
        text=True,
    )
    if completed.returncode == 0:
        return completed.stdout
    # エラーでも JSON が得られる場合があるため stdout を優先して返す
    if completed.stdout.strip():
        return completed.stdout
    stderr = completed.stderr.strip()
    raise SystemExit(f"reml_frontend が失敗しました: exit={completed.returncode}\n{stderr}")


def extract_stats(envelope: dict) -> dict:
    summary = envelope.get("summary") or {}
    stats = (summary.get("stats") or {}).get("parse_result") or {}
    return {
        "farthest_error_offset": stats.get("farthest_error_offset"),
        "packrat_stats": stats.get("packrat_stats"),
        "packrat_snapshot": stats.get("packrat_snapshot"),
    }


def append_notes(root: Path, size: SizeSpec, log_path: Path, envelope: dict) -> None:
    notes_path = root / NOTES_PATH
    stamp = utc_stamp()
    stats = extract_stats(envelope)
    lines: list[str] = []
    lines.append("")
    lines.append(f"## 計測ログ（自動生成）: {size.label} / {stamp}")
    lines.append(f"- 入力: `reports/spec-audit/ch4/generated/ws5/CP-WS5-001/core-parse-large-input-order.{size.label}.reml`")
    lines.append(f"- ログ: `{log_path.relative_to(root).as_posix()}`")
    lines.append(f"- farthest_error_offset: `{stats.get('farthest_error_offset')}`")
    packrat = stats.get("packrat_stats") or {}
    if isinstance(packrat, dict):
        for key in ("queries", "hits", "entries", "approx_bytes", "evictions", "budget_drops", "pruned"):
            if key in packrat:
                lines.append(f"- packrat_stats.{key}: `{packrat.get(key)}`")
    notes_path.write_text(notes_path.read_text(encoding="utf-8") + "\n".join(lines) + "\n", encoding="utf-8")


def ensure_dirs(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--sizes",
        default=",".join(DEFAULT_SIZES),
        help="生成・実行するサイズ（カンマ区切り）例: 1kb,100kb,10mb",
    )
    parser.add_argument(
        "--root",
        default=None,
        type=Path,
        help="Repository root (auto-detected by default)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="入力生成のみ行い、reml_frontend の実行とログ保存を行わない",
    )
    parser.add_argument(
        "--update-notes",
        action="store_true",
        help=f"{NOTES_PATH} へ計測結果を追記する",
    )
    parser.add_argument(
        "--streaming-fallback",
        action="store_true",
        help="通常実行が失敗した場合に stream.enabled=true で再実行する",
    )
    parser.add_argument(
        "--stream-chunk-size",
        type=int,
        default=None,
        help="fallback 実行時に指定する chunk サイズ（bytes）。未指定なら 65536",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    root = args.root or repo_root_from_script(Path(__file__))
    anchor_path = root / ANCHOR_INPUT
    anchor_text = anchor_path.read_text(encoding="utf-8")

    sizes = [parse_size(part) for part in args.sizes.split(",") if part.strip()]
    if not sizes:
        raise SystemExit("--sizes が空です")

    generated_dir = root / "reports" / "spec-audit" / "ch4" / "generated" / "ws5" / "CP-WS5-001"
    ensure_dirs(generated_dir)

    logs_dir = root / "reports" / "spec-audit" / "ch4" / "logs"
    ensure_dirs(logs_dir)

    for size in sizes:
        print(f"==> generating {size.label} (target_bytes={size.target_bytes})")
        generated_text = generate_large_input(anchor_text, size.target_bytes)
        generated_path = generated_dir / f"core-parse-large-input-order.{size.label}.reml"
        generated_path.write_text(generated_text, encoding="utf-8")
        actual = len(generated_text.encode("utf-8"))
        print(
            f"    -> wrote {generated_path.relative_to(root).as_posix()} (bytes={actual})"
        )

        if args.dry_run:
            continue

        stamp = utc_stamp()
        log_path = logs_dir / f"spec_core-CP-WS5-001-{size.label}-{stamp}.diagnostic.json"
        stdout: str | None = None
        try:
            stdout = run_reml_frontend(root, generated_path)
        except SystemExit as primary_err:
            if args.streaming_fallback:
                chunk = args.stream_chunk_size or 65536
                fallback_args = [
                    "--streaming",
                    "--stream-demand-min-bytes",
                    str(chunk),
                    "--stream-demand-preferred-bytes",
                    str(chunk),
                ]
                print(
                    f"    -> primary run failed; retrying with streaming (chunk={chunk} bytes)"
                )
                stdout = run_reml_frontend(root, generated_path, fallback_args)
            else:
                raise primary_err

        # まずは raw を保存（解析できれば整形して保存）
        try:
            envelope = json.loads(stdout)
            log_path.write_text(
                json.dumps(envelope, indent=2, ensure_ascii=False) + "\n",
                encoding="utf-8",
            )
            print(f"    -> wrote {log_path.relative_to(root).as_posix()}")
            if args.update_notes:
                append_notes(root, size, log_path, envelope)
        except json.JSONDecodeError:
            log_path.write_text(stdout, encoding="utf-8")
            print(f"    -> wrote {log_path.relative_to(root).as_posix()} (raw)")


if __name__ == "__main__":
    main()
