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


def _add_flag(
    acc: List[str], seen: set[tuple[str, str]], kind: str, value: str
) -> None:
    key = (kind, value)
    if key in seen:
        return
    seen.add(key)
    acc.extend([kind, value])


def gather_flags() -> List[str]:
    """llvm-config とプラットフォーム情報からリンクフラグを収集する。"""
    output: List[str] = []
    seen: set[tuple[str, str]] = set()

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
        _add_flag(output, seen, "-ccopt", f"-L{libdir}")
        _add_flag(output, seen, "-cclib", f"-L{libdir}")

    def process_flag(flag: str) -> None:
        if flag.startswith("-l"):
            _add_flag(output, seen, "-cclib", flag)
        elif flag.startswith("-L"):
            _add_flag(output, seen, "-ccopt", flag)
            _add_flag(output, seen, "-cclib", flag)
        else:
            _add_flag(output, seen, "-ccopt", flag)

    for flag in libs:
        process_flag(flag)

    for flag in system_libs:
        process_flag(flag)

    # LLVM C API を使用するので libLLVM-C.* が存在する場合は必ずリンクする。
    need_llvm_c = True
    for kind, value in seen:
        if kind == "-cclib" and value == "-lLLVM-C":
            need_llvm_c = False
            break

    if need_llvm_c:
        has_llvm_c = False
        search_dirs: List[str] = []
        if libdir:
            search_dirs.append(libdir)
        if platform.system() == "Darwin":
            search_dirs.extend(
                [
                    "/opt/homebrew/opt/llvm@18/lib",
                    "/usr/local/opt/llvm@18/lib",
                ]
            )
        for directory in search_dirs:
            pattern = os.path.join(directory, "libLLVM-C.*")
            if glob.glob(pattern):
                has_llvm_c = True
                break
        if has_llvm_c:
            _add_flag(output, seen, "-cclib", "-lLLVM-C")

    system = platform.system()
    if system == "Darwin":
        _add_flag(output, seen, "-cclib", "-lcurses")
    elif system == "Linux":
        _add_flag(output, seen, "-cclib", "-ltinfo")

    return output


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
