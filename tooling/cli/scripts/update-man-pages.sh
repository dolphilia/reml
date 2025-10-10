#!/usr/bin/env bash
# Reml CLI man ページ生成スクリプト（Markdown テンプレートとの同期用）
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"

TEMPLATE_PATH="${REPO_ROOT}/docs/guides/man/remlc-ocaml.1.md"
OUTPUT_DIR="${REPO_ROOT}/tooling/cli/man"
OUTPUT_PATH="${OUTPUT_DIR}/remlc-ocaml.1"

if [[ ! -f "${TEMPLATE_PATH}" ]]; then
  echo "テンプレートが見つかりません: ${TEMPLATE_PATH}" >&2
  exit 1
fi

PANDOC_BIN="${PANDOC:-pandoc}"
if ! command -v "${PANDOC_BIN}" >/dev/null 2>&1; then
  echo "pandoc が見つかりません。'brew install pandoc' などでインストールするか PANDOC 環境変数を設定してください。" >&2
  exit 1
fi

mkdir -p "${OUTPUT_DIR}"

TEMP_FILE="$(mktemp "${TMPDIR:-/tmp}/remlc-man.XXXXXX")"
cleanup() {
  rm -f "${TEMP_FILE}"
}
trap cleanup EXIT

"${PANDOC_BIN}" \
  --from=markdown \
  --to=man \
  --output="${TEMP_FILE}" \
  "${TEMPLATE_PATH}"

if [[ "${1:-}" == "--check" ]]; then
  if [[ ! -f "${OUTPUT_PATH}" ]]; then
    echo "生成済みの man ページが存在しません: ${OUTPUT_PATH}" >&2
    exit 1
  fi

  if diff -u "${OUTPUT_PATH}" "${TEMP_FILE}"; then
    echo "man ページはテンプレートと同期しています。"
    exit 0
  else
    echo "man ページがテンプレートと異なります。'$(basename "$0")' を実行して更新してください。" >&2
    exit 1
  fi
else
  mv "${TEMP_FILE}" "${OUTPUT_PATH}"
  echo "man ページを生成しました: ${OUTPUT_PATH}"
fi
