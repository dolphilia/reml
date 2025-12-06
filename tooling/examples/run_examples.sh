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
  core_config      - examples/core_config/cli の Config CLI サンプル
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

run_core_config_suite() {
  local frontend_dir="${ROOT}/compiler/rust/frontend"
  local manifest_path="../../examples/core_config/cli/reml.toml"
  local schema_path="../../examples/core_config/cli/schema.json"
  local base_path="../../examples/core_config/cli/config_old.json"
  local target_path="../../examples/core_config/cli/config_new.json"
  local lint_expected="${ROOT}/examples/core_config/cli/lint.expected.json"
  local diff_expected="${ROOT}/examples/core_config/cli/diff.expected.json"

  if [[ "${UPDATE_GOLDEN}" == true ]]; then
    echo "==> updating remlc config lint golden"
    (
      cd "${frontend_dir}"
      cargo run --quiet --bin remlc -- \
        config lint \
        --manifest "${manifest_path}" \
        --schema "${schema_path}" \
        --format json
    ) >"${lint_expected}"

    echo "==> updating remlc config diff golden"
    (
      cd "${frontend_dir}"
      cargo run --quiet --bin remlc -- \
        config diff \
        "${base_path}" \
        "${target_path}" \
        --format json
    ) >"${diff_expected}"
    echo "    -> wrote ${lint_expected#${ROOT}/}"
    echo "    -> wrote ${diff_expected#${ROOT}/}"
  else
    echo "==> remlc config lint (human)"
    (
      cd "${frontend_dir}"
      cargo run --quiet --bin remlc -- \
        config lint \
        --manifest "${manifest_path}" \
        --schema "${schema_path}" \
        --format human
    )
    echo "==> remlc config diff (human)"
    (
      cd "${frontend_dir}"
      cargo run --quiet --bin remlc -- \
        config diff \
        "${base_path}" \
        "${target_path}" \
        --format human
    )
  fi
}

if [[ "${SUITE}" == "core_config" ]]; then
  run_core_config_suite
  exit 0
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
      if "${CMD[@]}" >"${diag_tmp}" 2>"${audit_tmp}"; then
        :
      else
        # 致命的診断を含むサンプル（例: pipeline_branch）は非ゼロ終了するため、ゴールデン更新時のみ警告を出して継続する
        code=$?
        echo "    -> ${example} exited with status ${code} (continuing --update-golden)" >&2
      fi
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
