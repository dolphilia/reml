#!/usr/bin/env bash
# 診断 JSON のスキーマ検証ユーティリティ
#
# 使い方:
#   tooling/json-schema/validate-diagnostic-json.sh [--schema <schema>] [--pattern <text>] [--suite <name>] [--root <dir>] [paths...]
#
# オプション:
#   --schema   使用する JSON Schema（既定: tooling/json-schema/diagnostic-v2.schema.json）
#   --pattern  ファイルパスの部分一致フィルタ（複数指定可）
#   --suite    代表パターン名のエイリアス（内部的に --pattern と同等）
#   --root     検索対象ディレクトリ（複数指定可）
#   -h, --help ヘルプ表示
#
# paths を指定した場合はそのパスのみを検証対象にします。
# paths 未指定時は --root の指定がない限り tests/ expected/ reports/ を対象にします。

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

SCHEMA_PATH="${REPO_ROOT}/tooling/json-schema/diagnostic-v2.schema.json"
NODE_VALIDATOR="${REPO_ROOT}/tooling/lsp/tests/client_compat/validate-diagnostic-json.mjs"

declare -a PATTERNS=()
declare -a ROOTS=()
declare -a INPUTS=()

usage() {
  sed -n '2,25p' "${BASH_SOURCE[0]}"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --schema)
      shift || { echo "error: --schema の直後に値を指定してください" >&2; exit 1; }
      SCHEMA_PATH="$1"
      ;;
    --pattern)
      shift || { echo "error: --pattern の直後に値を指定してください" >&2; exit 1; }
      PATTERNS+=("$1")
      ;;
    --suite)
      shift || { echo "error: --suite の直後に値を指定してください" >&2; exit 1; }
      PATTERNS+=("$1")
      ;;
    --root)
      shift || { echo "error: --root の直後に値を指定してください" >&2; exit 1; }
      ROOTS+=("$1")
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      INPUTS+=("$1")
      ;;
  esac
  shift
done

if [[ ! -f "${SCHEMA_PATH}" ]]; then
  echo "error: schema が見つかりません: ${SCHEMA_PATH}" >&2
  exit 1
fi

if [[ ! -f "${NODE_VALIDATOR}" ]]; then
  echo "error: validator が見つかりません: ${NODE_VALIDATOR}" >&2
  exit 1
fi

declare -a TARGETS=()

if [[ ${#INPUTS[@]} -gt 0 ]]; then
  TARGETS=("${INPUTS[@]}")
else
  if [[ ${#ROOTS[@]} -eq 0 ]]; then
    ROOTS=("tests" "expected" "reports")
  fi
  for root in "${ROOTS[@]}"; do
    if [[ ! -e "${root}" ]]; then
      continue
    fi
    while IFS= read -r -d '' file; do
      TARGETS+=("${file}")
    done < <(find "${root}" -type f \( -name '*.json' -o -name '*.jsonl' \) -print0)
  done
fi

if [[ ${#PATTERNS[@]} -gt 0 ]]; then
  declare -a FILTERED=()
  for file in "${TARGETS[@]}"; do
    for pattern in "${PATTERNS[@]}"; do
      if [[ "${file}" == *"${pattern}"* ]]; then
        FILTERED+=("${file}")
        break
      fi
    done
  done
  TARGETS=("${FILTERED[@]}")
fi

if [[ ${#TARGETS[@]} -eq 0 ]]; then
  echo "info: 検証対象ファイルが見つかりません。"
  exit 0
fi

node "${NODE_VALIDATOR}" "${SCHEMA_PATH}" "${TARGETS[@]}"
