#!/usr/bin/env bash
set -euo pipefail

HEADER="compiler/runtime/native/include/reml_runtime.h"
OUT="compiler/runtime/ffi/bindings-bindgen.rs"

if ! command -v bindgen >/dev/null 2>&1; then
  echo "bindgen がインストールされていません（`cargo install bindgen` を実行してください）" >&2
  exit 1
fi

mkdir -p "$(dirname "$OUT")"

bindgen "$HEADER" \
  --allowlist-function mem_alloc \
  --allowlist-function mem_free \
  --allowlist-function inc_ref \
  --allowlist-function dec_ref \
  --allowlist-function panic \
  --allowlist-function print_i64 \
  --allowlist-function string_eq \
  --allowlist-function string_compare \
  --allowlist-function reml_ffi_bridge_record_status \
  --allowlist-function reml_ffi_acquire_borrowed_result \
  --allowlist-function reml_ffi_acquire_transferred_result \
  --allowlist-type reml_string_t \
  --allowlist-type reml_object_header_t \
  --output "$OUT" \
  -- \
  -I compiler/runtime/native/include

echo "bindgen 出力を $OUT に書き出しました。src/lib.rs と比較してください。"
