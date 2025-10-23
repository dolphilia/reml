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

    opam_prefix = os.environ.get("OPAM_SWITCH_PREFIX")
    if opam_prefix:
        for name in ("llvm-config", "llvm-config-18", "llvm-config-17"):
            candidate_path = os.path.join(opam_prefix, "bin", name)
            if os.path.exists(candidate_path):
                candidates.append(candidate_path)

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

    def ensure_component(name: str, extra_search: Optional[str] = None) -> None:
        token = f"-l{name}"
        if any(flag == "-cclib" and value == token for flag, value in pair_seen):
            return
        if libdir:
            candidate = os.path.join(libdir, f"lib{name}.a")
            shared = os.path.join(libdir, f"lib{name}.so")
            if os.path.exists(candidate) or os.path.exists(shared):
                _add_pair(output, pair_seen, "-cclib", token)
                return
        if extra_search:
            for directory in extra_search.split(os.pathsep):
                candidate = os.path.join(directory, f"lib{name}.a")
                shared = os.path.join(directory, f"lib{name}.so")
                if os.path.exists(candidate) or os.path.exists(shared):
                    _add_pair(output, pair_seen, "-ccopt", f"-L{directory}")
                    _add_pair(output, pair_seen, "-cclib", token)
                    return

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

    component_search = []
    if libdir:
        component_search.append(libdir)
    system = platform.system()
    if system == "Linux":
        component_search.extend(["/usr/lib/llvm-18/lib", "/usr/lib/llvm-17/lib"])
    elif system == "Darwin":
        component_search.extend(
            [
                "/opt/homebrew/opt/llvm@18/lib",
                "/usr/local/opt/llvm@18/lib",
            ]
        )
    search_path = os.pathsep.join(component_search) if component_search else None

    ensure_component("LLVMCore", search_path)
    ensure_component("LLVMBitWriter", search_path)
    ensure_component("LLVMSupport", search_path)

    system = platform.system()
    if system == "Darwin":
        _add_pair(output, pair_seen, "-cclib", "-lcurses")
    elif system == "Linux":
        _add_pair(output, pair_seen, "-cclib", "-ltinfo")
        _add_pair(output, pair_seen, "-cclib", "-lncursesw")

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
