#!/usr/bin/env python3
"""
Unicode XID テーブル自動生成スクリプト
=======================================

指定された Unicode データファイル（DerivedCoreProperties.txt /
UnicodeData.txt / PropList.txt）を解析し、Reml コンパイラで利用する
XID_Start / XID_Continue テーブルを生成する。

出力:
  - compiler/ocaml/src/lexer_tables/unicode_xid_tables.ml
  - compiler/ocaml/src/lexer_tables/unicode_xid_manifest.json

実行例:
  scripts/unicode/generate-xid-tables.py \
    --source-cache third_party/unicode/15.1.0 \
    --unicode-version 15.1.0

補足:
  - 入力ファイルが見つからない場合は ASCII プロファイルにフォールバックする。
  - SPDX ライセンス文字列などのメタデータは manifest に記録する。
"""

from __future__ import annotations

import argparse
import dataclasses
import datetime as dt
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Iterable, List, Optional, Sequence, Tuple


DERIVED_CORE_PROPERTIES = "DerivedCoreProperties.txt"
UNICODE_DATA = "UnicodeData.txt"
PROP_LIST = "PropList.txt"


@dataclasses.dataclass(frozen=True)
class Range:
    lo: int
    hi: int

    @classmethod
    def from_hex(cls, text: str) -> "Range":
        if ".." in text:
            start, end = text.split("..", 1)
        else:
            start = end = text
        return cls(int(start, 16), int(end, 16))

    def to_ocaml(self) -> str:
        return f"Range.make 0x{self.lo:04X} 0x{self.hi:04X}"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Unicode XID テーブル生成ツール")
    parser.add_argument(
        "--derived-core-properties",
        type=Path,
        help="DerivedCoreProperties.txt のパス（未指定時は --source-cache から推測）",
    )
    parser.add_argument(
        "--unicode-data",
        type=Path,
        help="UnicodeData.txt のパス（フォールバック用・チェック用）",
    )
    parser.add_argument(
        "--prop-list",
        type=Path,
        help="PropList.txt のパス（フォールバック用・チェック用）",
    )
    parser.add_argument(
        "--source-cache",
        type=Path,
        help="Unicode オリジナルファイルが格納されたディレクトリ",
    )
    parser.add_argument(
        "--unicode-version",
        type=str,
        default=None,
        help="生成対象の Unicode バージョン（manifest に記録）",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path("compiler/ocaml/src/lexer_tables"),
        help="生成結果を配置するディレクトリ",
    )
    parser.add_argument(
        "--manifest-name",
        type=str,
        default="unicode_xid_manifest.json",
        help="manifest ファイル名（out-dir 以下）",
    )
    parser.add_argument(
        "--force-ascii-fallback",
        action="store_true",
        help="入力ファイルが存在しても ASCII プロファイルで生成する",
    )
    return parser.parse_args()


def resolve_input(path: Optional[Path], cache: Optional[Path], filename: str) -> Optional[Path]:
    if path:
        return path if path.exists() else None
    if cache:
        candidate = cache / filename
        return candidate if candidate.exists() else None
    return None


def load_property_ranges(path: Path, property_name: str) -> List[Range]:
    pattern = re.compile(r"^\s*([0-9A-Fa-f\.]+)\s*;\s*([A-Za-z0-9_]+)")
    ranges: List[Range] = []
    with path.open("r", encoding="utf-8") as stream:
        for line in stream:
            match = pattern.match(line)
            if not match:
                continue
            raw_range, prop = match.groups()
            if prop != property_name:
                continue
            ranges.append(Range.from_hex(raw_range))
    return ranges


def merge_ranges(ranges: Sequence[Range]) -> List[Range]:
    if not ranges:
        return []
    sorted_ranges = sorted(ranges, key=lambda r: (r.lo, r.hi))
    merged: List[Range] = [sorted_ranges[0]]
    for current in sorted_ranges[1:]:
        prev = merged[-1]
        if current.lo <= prev.hi + 1:
            merged[-1] = Range(prev.lo, max(prev.hi, current.hi))
        else:
            merged.append(current)
    return merged


def ascii_fallback_ranges() -> Tuple[List[Range], List[Range]]:
    start = merge_ranges(
        [
            Range(ord("_"), ord("_")),
            Range(ord("A"), ord("Z")),
            Range(ord("a"), ord("z")),
        ]
    )
    cont = merge_ranges(
        start
        + [
            Range(ord("0"), ord("9")),
        ]
    )
    return start, cont


def build_ascii_mask(ranges: Sequence[Range], limit: int = 0x80) -> List[bool]:
    mask = [False] * limit
    for rng in ranges:
        lo = max(rng.lo, 0)
        hi = min(rng.hi, limit - 1)
        if hi < lo:
            continue
        for code_point in range(lo, hi + 1):
            mask[code_point] = True
    return mask


def format_bool_array(values: Sequence[bool]) -> str:
    result = []
    for index, flag in enumerate(values):
        literal = "true" if flag else "false"
        suffix = ";" if index < len(values) - 1 else ""
        comment = f"(* 0x{index:02X} *)"
        result.append(f"  {literal}{suffix} {comment}")
    return "\n".join(result)


def format_ranges(name: str, ranges: Sequence[Range]) -> str:
    lines = []
    for index, rng in enumerate(ranges):
        suffix = ";" if index < len(ranges) - 1 else ""
        lines.append(f"  {rng.to_ocaml()}{suffix}")
    body = "\n".join(lines)
    return f"let {name} : Range.t array = [|\n{body}\n|]\n"


def format_ocaml_module(
    unicode_version: str,
    start_ranges: Sequence[Range],
    continue_ranges: Sequence[Range],
) -> str:
    ascii_start_mask = build_ascii_mask(start_ranges)
    ascii_continue_mask = build_ascii_mask(continue_ranges)

    ocaml = f"""\
(* ⚠️ 自動生成ファイル — scripts/unicode/generate-xid-tables.py *)
(* リポジトリに直接編集内容を加えないでください。                       *)

module Range = struct
  type t = {{ lo : int; hi : int }}

  let make lo hi = {{ lo; hi }}

  let contains {{ lo; hi }} cp = lo <= cp && cp <= hi
end

module Range_set = struct
  let contains ranges cp =
    let rec binary_search lo hi =
      if lo > hi then false
      else
        let mid = (lo + hi) / 2 in
        let range = Array.unsafe_get ranges mid in
        if Range.contains range cp then true
        else if cp < range.Range.lo then binary_search lo (mid - 1)
        else binary_search (mid + 1) hi
    in
    binary_search 0 (Array.length ranges - 1)
end

let unicode_version = "{unicode_version}"

{format_ranges("xid_start_ranges", start_ranges)}
{format_ranges("xid_continue_ranges", continue_ranges)}

let ascii_start_mask : bool array = [|
{format_bool_array(ascii_start_mask)}
|]

let ascii_continue_mask : bool array = [|
{format_bool_array(ascii_continue_mask)}
|]

let is_ascii code_point = code_point land 0xFFFFFF80 = 0

let is_xid_start code_point =
  if is_ascii code_point then Array.unsafe_get ascii_start_mask code_point
  else Range_set.contains xid_start_ranges code_point

let is_xid_continue code_point =
  if is_ascii code_point then Array.unsafe_get ascii_continue_mask code_point
  else Range_set.contains xid_continue_ranges code_point
"""
    return ocaml


def sha256_of_file(path: Path) -> Optional[str]:
    if not path.exists():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_file(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content.rstrip() + "\n", encoding="utf-8")


def write_manifest(path: Path, data: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as stream:
        json.dump(data, stream, ensure_ascii=False, indent=2, sort_keys=True)
        stream.write("\n")


def main() -> int:
    args = parse_args()

    derived_path = resolve_input(args.derived_core_properties, args.source_cache, DERIVED_CORE_PROPERTIES)
    unicode_data_path = resolve_input(args.unicode_data, args.source_cache, UNICODE_DATA)
    prop_list_path = resolve_input(args.prop_list, args.source_cache, PROP_LIST)

    use_ascii_only = args.force_ascii_fallback or derived_path is None
    if use_ascii_only:
        start_ranges, continue_ranges = ascii_fallback_ranges()
        unicode_version = args.unicode_version or "ASCII-FALLBACK"
    else:
        raw_start = load_property_ranges(derived_path, "XID_Start")
        raw_continue = load_property_ranges(derived_path, "XID_Continue")
        start_ranges = merge_ranges(raw_start)
        continue_ranges = merge_ranges(raw_continue)
        unicode_version = args.unicode_version or derived_path.parent.name

    ocaml_module_path = args.out_dir / "unicode_xid_tables.ml"
    manifest_path = args.out_dir / args.manifest_name

    write_file(ocaml_module_path, format_ocaml_module(unicode_version, start_ranges, continue_ranges))

    manifest = {
        "unicode_version": unicode_version,
        "generated_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "inputs": {
            "DerivedCoreProperties.txt": str(derived_path) if derived_path else None,
            "UnicodeData.txt": str(unicode_data_path) if unicode_data_path else None,
            "PropList.txt": str(prop_list_path) if prop_list_path else None,
        },
        "input_sha256": {
            "DerivedCoreProperties.txt": sha256_of_file(derived_path) if derived_path else None,
            "UnicodeData.txt": sha256_of_file(unicode_data_path) if unicode_data_path else None,
            "PropList.txt": sha256_of_file(prop_list_path) if prop_list_path else None,
        },
        "ranges": {
            "xid_start": len(start_ranges),
            "xid_continue": len(continue_ranges),
        },
        "ascii_fallback": use_ascii_only,
        "spdx_license": "Unicode-Derived-Core-Properties-1.0",
    }
    write_manifest(manifest_path, manifest)

    if derived_path is None:
        sys.stderr.write(
            "warning: DerivedCoreProperties.txt が見つからなかったため ASCII フォールバックを生成しました。\n"
        )
    else:
        sys.stderr.write(
            "info: Unicode XID テーブルを生成しました "
            f"(version={unicode_version}, start={len(start_ranges)}, continue={len(continue_ranges)})\n"
        )

    return 0


if __name__ == "__main__":
    sys.exit(main())
