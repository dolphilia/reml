#!/usr/bin/env bash
# Unicode Character Database (UCD) ファイルの取得スクリプト
# ---------------------------------------------------------
# 指定した Unicode バージョンの UCD ファイルをダウンロードし、
# `third_party/unicode/<version>/` 以下に配置する。
#
# 取得対象:
#   - DerivedCoreProperties.txt
#   - UnicodeData.txt
#   - PropList.txt
#
# 使い方:
#   scripts/unicode/fetch-unicode-data.sh 15.1.0
#
# 実行には `curl` または `wget` が必要です。ネットワークが利用できない場合は、
# Unicode 公式サイト (https://www.unicode.org/Public/) から手動でダウンロードし、
# 同じ配置パスにファイルを設置してください。

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <unicode-version> [--overwrite]" >&2
  exit 1
fi

UNICODE_VERSION="$1"
OVERWRITE=false

if [[ "${2:-}" == "--overwrite" ]]; then
  OVERWRITE=true
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET_DIR="$ROOT_DIR/compiler/ocaml/third_party/unicode/$UNICODE_VERSION"
mkdir -p "$TARGET_DIR"

FILES=(
  "ucd/$UNICODE_VERSION/DerivedCoreProperties.txt"
  "ucd/$UNICODE_VERSION/UnicodeData.txt"
  "ucd/$UNICODE_VERSION/PropList.txt"
)

download() {
  local url="$1"
  local destination="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$destination"
  elif command -v wget >/dev/null 2>&1; then
    wget -q "$url" -O "$destination"
  else
    echo "error: curl もしくは wget が必要です。" >&2
    exit 1
  fi
}

for path in "${FILES[@]}"; do
  filename="$(basename "$path")"
  url="https://www.unicode.org/Public/$path"
  dest="$TARGET_DIR/$filename"

  if [[ -f "$dest" && "$OVERWRITE" == "false" ]]; then
    echo "skip: $dest は既に存在します（--overwrite で再取得）"
    continue
  fi

  echo "fetch: $url"
  download "$url" "$dest"
done

echo "done: files are stored in $TARGET_DIR"
