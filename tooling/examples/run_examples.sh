#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

usage() {
  cat <<'EOF'
usage: tooling/examples/run_examples.sh --suite <name> [--with-audit]

利用可能なスイート:
  core_io          - examples/core_io/ 以下の Reader/Writer サンプル
  core_path        - examples/core_path/ 以下のセキュリティサンプル
  core_diagnostics - examples/core_diagnostics/ 以下の監査パイプラインサンプル
EOF
}

SUITE=""
WITH_AUDIT=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --suite)
      SUITE="$2"
      shift 2
      ;;
    --with-audit)
      WITH_AUDIT=true
      shift
      ;;
    *)
      usage
      exit 1
      ;;
  esac
done

if [[ -z "${SUITE}" ]]; then
  usage
  exit 1
fi

declare -a FILES=()

case "$SUITE" in
  core_io)
    FILES=("core_io/file_copy.reml")
    ;;
  core_path)
    FILES=("core_path/security_check.reml")
    ;;
  core_diagnostics)
    FILES=("core_diagnostics/pipeline_success.reml" "core_diagnostics/pipeline_branch.reml")
    ;;
  *)
    echo "未対応のスイート: $SUITE" >&2
    usage
    exit 1
    ;;
esac

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo が見つかりません。Rust Toolchain をセットアップしてください。" >&2
  exit 1
fi

FRONTEND_DIR="${ROOT}/compiler/rust/frontend"

for example in "${FILES[@]}"; do
  target="${ROOT}/examples/${example}"
  if [[ ! -f "${target}" ]]; then
    echo "例が見つかりません: ${target}" >&2
    exit 1
  fi

  echo "==> running ${example}"
  (
    cd "${FRONTEND_DIR}"
    CMD=(cargo run --quiet --bin reml_frontend --)
    if [[ "${WITH_AUDIT}" == true ]]; then
      CMD+=(--emit-audit-log)
    fi
    CMD+=("${target}")
    "${CMD[@]}"
  )
done
