#!/usr/bin/env bash
# 診断ゴールデン更新ユーティリティ (V2 対応)
#
# 使い方:
#   scripts/update-diagnostics-golden.sh [--no-test] [--pattern <glob>] [--diff]
#
# オプション:
#   --no-test     dune runtest を実行せずにゴールデンのみ更新
#   --pattern     dune runtest に渡すターゲットパターン（例: test_cli_diagnostics）
#   --diff        更新前に差分サマリを表示
#
# スクリプトは `compiler/ocaml/tests/golden/_actual/*.actual.json` を検出し、
# 対応する `*.json.golden` を上書きします。更新後は `_actual` ディレクトリをクリーンアップします。

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
GOLDEN_DIR="${REPO_ROOT}/compiler/ocaml/tests/golden"
ACTUAL_DIR="${GOLDEN_DIR}/_actual"
DIFF_SCRIPT="${REPO_ROOT}/tooling/ci/collect-diagnostic-diff.py"

run_tests=true
pattern=""
show_diff=false

print_usage() {
  sed -n '2,20p' "${BASH_SOURCE[0]}"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-test)
      run_tests=false
      shift
      ;;
    --pattern)
      shift || { echo "error: --pattern の直後に値を指定してください" >&2; exit 1; }
      pattern="$1"
      shift
      ;;
    --diff)
      show_diff=true
      shift
      ;;
    -h|--help)
      print_usage
      exit 0
      ;;
    *)
      echo "error: 不明なオプション: $1" >&2
      print_usage
      exit 1
      ;;
  esac
done

if [[ "${run_tests}" == true ]]; then
  if [[ -n "${pattern}" ]]; then
    echo "[INFO] dune runtest --no-buffer ${pattern}"
    (cd "${REPO_ROOT}" && dune runtest --no-buffer "${pattern}")
  else
    echo "[INFO] dune runtest --no-buffer"
    (cd "${REPO_ROOT}" && dune runtest --no-buffer)
  fi
fi

if [[ ! -d "${ACTUAL_DIR}" ]]; then
  echo "[INFO] 更新対象の実行結果 (_actual) が見つかりません: ${ACTUAL_DIR}"
  exit 0
fi

mapfile -t actual_files < <(find "${ACTUAL_DIR}" -type f -name '*.actual.json' | sort)

if [[ ${#actual_files[@]} -eq 0 ]]; then
  echo "[INFO] 更新対象となる .actual.json ファイルはありません。"
  exit 0
fi

if [[ ! -x "${DIFF_SCRIPT}" ]]; then
  echo "[WARN] 差分スクリプトが実行可能ではありません。: ${DIFF_SCRIPT}"
  chmod +x "${DIFF_SCRIPT}" 2>/dev/null || true
fi

if [[ "${show_diff}" == true && -f "${DIFF_SCRIPT}" ]]; then
  echo "[INFO] 更新前の差分サマリ:"
  python3 "${DIFF_SCRIPT}" \
    --baseline "${GOLDEN_DIR}" \
    --actual "${ACTUAL_DIR}" \
    --format markdown \
    || echo "[WARN] 差分集計に失敗しました"
fi

updated=0
for actual_path in "${actual_files[@]}"; do
  relative="${actual_path#${ACTUAL_DIR}/}"
  target_relative="${relative%.actual.json}.json.golden"
  destination="${GOLDEN_DIR}/${target_relative}"
  mkdir -p "$(dirname "${destination}")"
  cp "${actual_path}" "${destination}"
  printf '[UPDATE] %s\n' "${target_relative}"
  updated=$((updated + 1))
done

echo "[INFO] 更新したゴールデン数: ${updated}"

echo "[INFO] _actual ディレクトリをクリーンアップします"
find "${ACTUAL_DIR}" -type f -name '*.actual.json' -delete
find "${ACTUAL_DIR}" -type d -empty -delete

if [[ "${show_diff}" == true && -f "${DIFF_SCRIPT}" ]]; then
  echo "[INFO] 更新後の差分サマリ:"
  python3 "${DIFF_SCRIPT}" \
    --baseline "${GOLDEN_DIR}" \
    --actual "${ACTUAL_DIR}" \
    --format markdown \
    || echo "[WARN] 差分集計に失敗しました"
fi

echo "[DONE] 診断ゴールデンの更新が完了しました。"
