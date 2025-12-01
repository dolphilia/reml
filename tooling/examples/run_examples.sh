#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

usage() {
  cat <<'EOF'
usage: tooling/examples/run_examples.sh --suite <name>

利用可能なスイート:
  core_io     - examples/core_io/ 以下の Reader/Writer サンプル
  core_path   - examples/core_path/ 以下のセキュリティサンプル
EOF
}

if [[ $# -ne 2 || "$1" != "--suite" ]]; then
  usage
  exit 1
fi

SUITE="$2"
declare -a FILES=()

case "$SUITE" in
  core_io)
    FILES=("core_io/file_copy.reml")
    ;;
  core_path)
    FILES=("core_path/security_check.reml")
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

for example in "${FILES[@]}"; do
  target="${ROOT}/examples/${example}"
  if [[ ! -f "${target}" ]]; then
    echo "例が見つかりません: ${target}" >&2
    exit 1
  fi

  echo "==> running ${example}"
  (
    cd "${ROOT}"
    cargo run --quiet --bin reml -- "${target}"
  )
done
