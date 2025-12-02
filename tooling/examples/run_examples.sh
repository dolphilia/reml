#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

usage() {
  cat <<'EOF'
usage: tooling/examples/run_examples.sh --suite <name> [--with-audit] [--update-golden]

利用可能なスイート:
  core_io          - examples/core_io/ 以下の Reader/Writer サンプル
  core_path        - examples/core_path/ 以下のセキュリティサンプル
  core_diagnostics - examples/core_diagnostics/ 以下の監査パイプラインサンプル
EOF
}

SUITE=""
WITH_AUDIT=false
UPDATE_GOLDEN=false

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
    --update-golden)
      UPDATE_GOLDEN=true
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
  if [[ "${UPDATE_GOLDEN}" == true ]]; then
    diag_tmp="$(mktemp)"
    audit_tmp="$(mktemp)"
    (
      cd "${FRONTEND_DIR}"
      CMD=(cargo run --quiet --bin reml_frontend -- --output json)
      if [[ "${WITH_AUDIT}" == true ]]; then
        CMD+=(--emit-audit-log)
      fi
      CMD+=("${target}")
      "${CMD[@]}" >"${diag_tmp}" 2>"${audit_tmp}"
    )
    diag_path="${target%.reml}.expected.diagnostic.json"
    audit_path="${target%.reml}.expected.audit.jsonl"
    python3 - <<'PY' "${diag_tmp}" "${diag_path}"
import json
import pathlib
import sys

src = pathlib.Path(sys.argv[1])
dst = pathlib.Path(sys.argv[2])
data = json.loads(src.read_text())
dst.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n")
PY
    python3 - <<'PY' "${audit_tmp}" "${audit_path}"
import json
import pathlib
import sys

src = pathlib.Path(sys.argv[1])
dst = pathlib.Path(sys.argv[2])
lines = []
for raw in src.read_text().splitlines():
    raw = raw.strip()
    if not raw:
        continue
    try:
        json.loads(raw)
    except json.JSONDecodeError:
        continue
    lines.append(raw)
dst.write_text("\n".join(lines) + ("\n" if lines else ""))
if not lines:
    raise SystemExit("監査ログの JSON 行が見つかりません")
PY
    rm -f "${diag_tmp}" "${audit_tmp}"
    echo "    -> updated ${diag_path#${ROOT}/}"
    echo "    -> updated ${audit_path#${ROOT}/}"
  else
    (
      cd "${FRONTEND_DIR}"
      CMD=(cargo run --quiet --bin reml_frontend --)
      if [[ "${WITH_AUDIT}" == true ]]; then
        CMD+=(--emit-audit-log)
      fi
      CMD+=("${target}")
      "${CMD[@]}"
    )
  fi
done
