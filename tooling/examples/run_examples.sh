#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

SPEC_CORE_REQUIRED_SOURCE_DIRS=(
  "examples/spec_core/chapter1/control_flow"
  "examples/spec_core/chapter1/literals"
  "examples/spec_core/chapter1/lambda"
)

SPEC_CORE_REQUIRED_EXPECTED_DIRS=(
  "expected/spec_core/chapter1/control_flow"
  "expected/spec_core/chapter1/literals"
  "expected/spec_core/chapter1/lambda"
)

usage() {
  cat <<'EOF'
usage: tooling/examples/run_examples.sh --suite <name> [--with-audit] [--update-golden]

利用可能なスイート:
  spec_core       - docs/spec/1-x 由来の BNF サンプル（phase4-scenario-matrix 参照）
  practical       - docs/spec/3-x 由来の実務サンプル（phase4-scenario-matrix 参照）
  language_impl_samples - examples/language-impl-samples/ の Reml サンプル（phase4-scenario-matrix 参照）
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

if [[ "${SUITE}" == "spec_core" || "${SUITE}" == "practical" || "${SUITE}" == "language_impl_samples" ]]; then
  if [[ "${SUITE}" == "spec_core" ]]; then
    missing=()
    for dir in "${SPEC_CORE_REQUIRED_SOURCE_DIRS[@]}"; do
      if [[ ! -d "${ROOT}/${dir}" ]]; then
        missing+=("${dir}")
      fi
    done
    for dir in "${SPEC_CORE_REQUIRED_EXPECTED_DIRS[@]}"; do
      if [[ ! -d "${ROOT}/${dir}" ]]; then
        missing+=("${dir}")
      fi
    done
    if [[ ${#missing[@]} -gt 0 ]]; then
      {
        echo "spec_core スイートに必要なディレクトリが不足しています。"
        for dir in "${missing[@]}"; do
          echo "  - ${dir}"
        done
        echo "docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md の Phase4 Missing Examples 手順に従い整備してください。"
      } >&2
      exit 1
    fi
  fi
  if [[ "${WITH_AUDIT}" == true || "${UPDATE_GOLDEN}" == true ]]; then
    echo "${SUITE} スイートでは --with-audit / --update-golden オプションは未対応です。" >&2
    exit 1
  fi
  python3 "${ROOT}/tooling/examples/run_phase4_suite.py" \
    --suite "${SUITE}" \
    --root "${ROOT}"
  exit $?
fi

run_core_config_suite() {
  local frontend_dir="${ROOT}/compiler/frontend"
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

FRONTEND_DIR="${ROOT}/compiler/frontend"

# Stage mismatch を意図的に発生させるサンプルなど、非ゼロ終了を許容する
# 例はここで列挙する。
allows_failure() {
  local example="$1"
  case "$example" in
    core_diagnostics/pipeline_branch.reml)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

for example in "${FILES[@]}"; do
  target="${ROOT}/examples/${example}"
  if [[ ! -f "${target}" ]]; then
    echo "例が見つかりません: ${target}" >&2
    exit 1
  fi

  echo "==> running ${example}"
  allow_failure=false
  if allows_failure "${example}"; then
    allow_failure=true
  fi
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
        code=$?
        if [[ "${allow_failure}" == true ]]; then
          echo "    -> ${example} exited with status ${code} (allowed failure during --update-golden)" >&2
        else
          exit "${code}"
        fi
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
      if "${CMD[@]}"; then
        :
      else
        code=$?
        if [[ "${allow_failure}" == true ]]; then
          echo "    -> ${example} exited with status ${code} (allowed failure)" >&2
        else
          exit "${code}"
        fi
      fi
    )
  fi
done
