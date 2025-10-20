#!/usr/bin/env python3
"""
LLVM のリンクフラグを動的に検出し、Dune 用の S 式として出力するスクリプト。

GitHub Actions (Linux/macOS/Windows) で `llvm-config` が指すライブラリ名が
異なるため、プラットフォームに応じた `-l` / `-L` フラグを生成して
`ocamlopt` のリンク失敗を防ぐ。
"""

from __future__ import annotations

import os
import platform
import shlex
import shutil
import subprocess
import sys
from typing import Iterable, List


def _find_llvm_config() -> str | None:
    """適切な llvm-config バイナリを探索する。"""
    candidates: List[str] = []
    env_value = os.environ.get("LLVM_CONFIG")
    if env_value:
        candidates.append(env_value)
    candidates.extend(
        [
            "llvm-config",
            "llvm-config-18",
            "llvm-config-17",
        ]
    )
    for candidate in candidates:
        path = shutil.which(candidate)
        if path:
            return path
    return None


def _run_command(args: Iterable[str]) -> List[str]:
    """コマンドを実行してフラグをトークン化して返す。失敗時は空。"""
    try:
        output = subprocess.check_output(list(args), text=True)
    except (OSError, subprocess.CalledProcessError):
        return []
    return shlex.split(output)


def _ensure_flag(flags: List[str], flag: str) -> None:
    if flag not in flags:
        flags.append(flag)


def gather_flags() -> List[str]:
    """llvm-config とプラットフォーム情報からリンクフラグを収集する。"""
    flags: List[str] = []
    llvm_config = _find_llvm_config()

    if llvm_config:
        flags.extend(_run_command([llvm_config, "--libs", "core", "bitwriter"]))
        flags.extend(_run_command([llvm_config, "--system-libs"]))

    # 生成物が空の場合は LLVM 18 系のモノリシックライブラリをデフォルトとする。
    if not flags:
        flags.append("-lLLVM-18")

    # 追加で必要なライブラリを補完する。
    _ensure_flag(flags, "-lLLVM-C")

    system = platform.system()
    if system == "Darwin":
        if not any(f in ("-lcurses", "-lncurses", "-lncursesw") for f in flags):
            flags.append("-lcurses")
    elif system == "Linux":
        if not any(f in ("-ltinfo", "-lcurses", "-lncurses", "-lncursesw") for f in flags):
            flags.append("-ltinfo")

    # 重複を除去しつつ順序を維持する。
    deduped: List[str] = []
    for flag in flags:
        if flag not in deduped:
            deduped.append(flag)

    return deduped


def emit_sexp(flags: List[str]) -> str:
    """Dune が読み込めるよう S 式へ変換する。"""
    parts: List[str] = []
    for flag in flags:
        parts.append("-cclib")
        parts.append(flag)
    return "(" + " ".join(parts) + ")\n"


def main() -> int:
    flags = gather_flags()
    sys.stdout.write(emit_sexp(flags))
    return 0


if __name__ == "__main__":
    sys.exit(main())
