#!/usr/bin/env python3
"""
LLVM のリンクフラグを動的に検出し、Dune 用の S 式として出力するスクリプト。

GitHub Actions (Linux/macOS/Windows) で `llvm-config` が指すライブラリ名が
異なるため、プラットフォームに応じた `-l` / `-L` フラグを生成して
`ocamlopt` のリンク失敗を防ぐ。
"""

from __future__ import annotations

import glob
import os
import platform
import shlex
import shutil
import subprocess
import sys
from typing import Iterable, List, Optional


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


def _add_pair(
    acc: List[str], seen: set[tuple[str, str]], flag: str, value: str
) -> None:
    key = (flag, value)
    if key in seen:
        return
    seen.add(key)
    acc.extend([flag, value])


def gather_flags() -> List[str]:
    """llvm-config とプラットフォーム情報からリンクフラグを収集する。"""
    output: List[str] = []
    pair_seen: set[tuple[str, str]] = set()

    llvm_config = _find_llvm_config()
    libs: List[str] = []
    system_libs: List[str] = []
    libdir: Optional[str] = None

    if llvm_config:
        libs = _run_command([llvm_config, "--libs", "core", "bitwriter"])
        system_libs = _run_command([llvm_config, "--system-libs"])
        try:
            libdir = subprocess.check_output(
                [llvm_config, "--libdir"], text=True
            ).strip()
        except (OSError, subprocess.CalledProcessError):
            libdir = None

    # フォールバック: llvm-config が見つからない場合
    if not libs and not system_libs:
        libs = ["-lLLVM-18"]

    if libdir:
        _add_pair(output, pair_seen, "-ccopt", f"-L{libdir}")

    def process_flag(flag: str) -> None:
        if flag.startswith("-l"):
            _add_pair(output, pair_seen, "-cclib", flag)
        elif flag.startswith("-L"):
            _add_pair(output, pair_seen, "-ccopt", flag)
        else:
            _add_pair(output, pair_seen, "-ccopt", flag)

    for flag in libs:
        process_flag(flag)

    for flag in system_libs:
        process_flag(flag)

    has_llvm_c = any(
        flag == "-cclib" and value == "-lLLVM-C" for flag, value in pair_seen
    )

    if not has_llvm_c:
        search_dirs: List[str] = []
        if libdir:
            search_dirs.append(libdir)
        system = platform.system()
        if system == "Darwin":
            search_dirs.extend(
                [
                    "/opt/homebrew/opt/llvm@18/lib",
                    "/usr/local/opt/llvm@18/lib",
                ]
            )
        elif system == "Linux":
            search_dirs.extend(["/usr/lib/llvm-18/lib", "/usr/lib/llvm-17/lib"])
        for directory in search_dirs:
            pattern = os.path.join(directory, "libLLVM-C.*")
            if glob.glob(pattern):
                _add_pair(output, pair_seen, "-ccopt", f"-L{directory}")
                _add_pair(output, pair_seen, "-cclib", "-lLLVM-C")
                has_llvm_c = True
                break

    system = platform.system()
    if system == "Darwin":
        _add_pair(output, pair_seen, "-cclib", "-lcurses")
    elif system == "Linux":
        _add_pair(output, pair_seen, "-cclib", "-ltinfo")

    return output


def emit_sexp(flags: List[str]) -> str:
    """Dune が読み込めるよう S 式へ変換する。"""
    return "(" + " ".join(flags) + ")\n"


def main() -> int:
    flags = gather_flags()
    sys.stdout.write(emit_sexp(flags))
    return 0


if __name__ == "__main__":
    sys.exit(main())
