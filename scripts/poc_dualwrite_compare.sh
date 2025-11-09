#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
OCAML_DIR="${REPO_ROOT}/compiler/ocaml"
RUST_DIR="${REPO_ROOT}/compiler/rust/frontend"
REPORT_DIR="${REPO_ROOT}/reports/dual-write/front-end/poc"
COLLECT_METRICS_SCRIPT="${REPO_ROOT}/tooling/ci/collect-iterator-audit-metrics.py"
VALIDATE_DIAG_SCRIPT="${SCRIPT_DIR}/validate-diagnostic-json.sh"

RUN_ID="${DUALWRITE_RUN_ID:-2025-11-28-logos-chumsky}"
CASES_FILE="${DUALWRITE_CASES_FILE:-}"
MODE="ast"

usage() {
  cat <<'EOF'
Usage: scripts/poc_dualwrite_compare.sh [--run-id <id>] [--cases <path>] [--mode <ast|typeck|diag>]

Options:
  --run-id <id>     出力ディレクトリ名を上書き（既定: 2025-11-28-logos-chumsky）
  --cases <path>    ケース定義ファイル（format: name::inline::<src> | name::file::<path>）
  --mode <ast|typeck|diag>
                     実行モード（typeck は型推論成果物、diag は W4 診断互換向け成果物を収集）
  --help            このヘルプを表示

環境変数:
  DUALWRITE_RUN_ID       --run-id と同様
  DUALWRITE_CASES_FILE   --cases と同様
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --run-id)
      RUN_ID="$2"
      shift 2
      ;;
    --cases)
      CASES_FILE="$2"
      shift 2
      ;;
    --mode)
      MODE="$2"
      shift 2
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      exit 1
      ;;
esac
done

if [[ "${MODE}" == "typeck" ]]; then
  REPORT_DIR="${REPO_ROOT}/reports/dual-write/front-end/w3-type-inference"
elif [[ "${MODE}" == "diag" ]]; then
  REPORT_DIR="${REPO_ROOT}/reports/dual-write/front-end/w4-diagnostics"
elif [[ "${MODE}" != "ast" ]]; then
  echo "Unsupported mode: ${MODE}" >&2
  exit 1
fi

declare -a CASE_ENTRIES=()
declare -a CASE_TESTS_META=()
declare -a CASE_FLAGS_META=()
declare -a CASE_FLAGS_META_OCAML=()
declare -a CASE_FLAGS_META_RUST=()
declare -a CASE_LSP_META=()

if [[ -n "${CASES_FILE}" ]]; then
  if [[ ! -f "${CASES_FILE}" ]]; then
    echo "ケース定義ファイルが見つかりません: ${CASES_FILE}" >&2
    exit 1
  fi
  current_tests=""
  current_flags=""
  current_flags_ocaml=""
  current_flags_rust=""
  current_lsp=""
  while IFS= read -r line || [[ -n "$line" ]]; do
    line="$(printf '%s' "${line}" | sed 's/[[:space:]]*$//')"
    if [[ -z "${line}" ]]; then
      continue
    fi
    if [[ "${line}" =~ ^#[[:space:]]*(.*)$ ]]; then
      comment="${BASH_REMATCH[1]}"
      if [[ "${comment}" =~ ^([A-Za-z0-9._-]+)[[:space:]]*:(.*)$ ]]; then
        key="$(printf '%s' "${BASH_REMATCH[1]}" | tr '[:upper:]' '[:lower:]')"
        value="$(printf '%s' "${BASH_REMATCH[2]}" | sed 's/^[[:space:]]*//')"
        case "${key}" in
          tests)
            current_tests="${value}"
            ;;
          flags)
            current_flags="${value}"
            ;;
          flags.ocaml|flags_ocaml)
            current_flags_ocaml="${value}"
            ;;
          flags.rust|flags_rust)
            current_flags_rust="${value}"
            ;;
          lsp-fixture|lsp_fixture)
            current_lsp="${value}"
            ;;
        esac
      fi
      continue
    fi
    CASE_ENTRIES+=("${line}")
    CASE_TESTS_META+=("${current_tests:-}")
    CASE_FLAGS_META+=("${current_flags:-}")
    CASE_FLAGS_META_OCAML+=("${current_flags_ocaml:-}")
    CASE_FLAGS_META_RUST+=("${current_flags_rust:-}")
    CASE_LSP_META+=("${current_lsp:-}")
    current_tests=""
    current_flags=""
    current_flags_ocaml=""
    current_flags_rust=""
    current_lsp=""
  done < "${CASES_FILE}"
else
  CASE_ENTRIES+=(
    "empty_uses::inline::fn answer() = 42"
    "multiple_functions::inline::fn log(x) = x\nfn log_twice(x) = log(log(x))"
    "addition::inline::fn add(x, y) = x + y"
    "missing_paren::inline::fn missing(x = x"
  )
  for _ in "${CASE_ENTRIES[@]}"; do
    CASE_TESTS_META+=("")
    CASE_FLAGS_META+=("")
    CASE_FLAGS_META_OCAML+=("")
    CASE_FLAGS_META_RUST+=("")
    CASE_LSP_META+=("")
  done
fi

if [[ ${#CASE_ENTRIES[@]} -eq 0 ]]; then
  echo "実行対象のケースがありません。" >&2
  exit 1
fi

RUN_DIR="${REPORT_DIR}/${RUN_ID}"
mkdir -p "${RUN_DIR}"

: "${CARGO_HOME:=${HOME}/.cargo}"
: "${CARGO_TARGET_DIR:=${RUST_DIR}/target}"
export CARGO_HOME CARGO_TARGET_DIR
mkdir -p "${CARGO_HOME}" "${CARGO_TARGET_DIR}"

printf "==> 出力ディレクトリ: %s\n" "${RUN_DIR}"

sanitize_name() {
  local value="$1"
  printf "%s" "${value}" | tr '[:upper:]' '[:lower:]' | tr -cs 'a-z0-9._-' '_'
}

run_in_dir() {
  local dir="$1"
  shift
  (cd "${dir}" && "$@")
}

needs_streaming_metrics() {
  local diag_path="$1"
  if [[ ! -s "${diag_path}" ]]; then
    return 1
  fi
  if grep -q '"parser\.stream' "${diag_path}"; then
    return 0
  fi
  return 1
}

is_streaming_case() {
  local case_name="$1"
  [[ "${case_name}" == stream_* ]]
}

enforce_streaming_metrics_gate() {
  local case_name="$1"
  local case_dir="$2"
  python3 - "${case_name}" "${case_dir}" <<'PY'
import json
import math
import sys
from pathlib import Path

case = sys.argv[1]
case_dir = Path(sys.argv[2])

if not case.startswith("stream_"):
    sys.exit(0)

def load_metric(path: Path, metric_name: str):
    if not path.exists():
        return None
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return None
    metrics = data.get("metrics")
    if not isinstance(metrics, list):
        return None
    for item in metrics:
        if isinstance(item, dict) and item.get("metric") == metric_name:
            return item
    return None

frontends = ("ocaml", "rust")
payload = {
    "case": case,
    "parser_expected_summary_presence": {},
    "parser_stream_extension_field_coverage": {},
}
errors = []

def check_metric(metric, label, frontend):
    if metric is None:
        errors.append(f"{frontend}: {label} missing")
        return
    pass_rate = metric.get("pass_rate")
    if not isinstance(pass_rate, (int, float)):
        errors.append(f"{frontend}: {label} has invalid pass_rate")
        return
    if not math.isclose(pass_rate, 1.0, rel_tol=1e-9, abs_tol=1e-9):
        errors.append(f"{frontend}: {label}={pass_rate}")

for frontend in frontends:
    parser_path = case_dir / f"parser-metrics.{frontend}.json"
    streaming_path = case_dir / f"streaming-metrics.{frontend}.json"
    parser_metric = load_metric(parser_path, "parser.expected_summary_presence")
    streaming_metric = load_metric(
        streaming_path, "parser.stream_extension_field_coverage"
    )
    payload["parser_expected_summary_presence"][frontend] = parser_metric
    payload["parser_stream_extension_field_coverage"][frontend] = streaming_metric
    check_metric(parser_metric, "parser.expected_summary_presence", frontend)
    check_metric(streaming_metric, "parser.stream_extension_field_coverage", frontend)

if errors:
    payload["errors"] = errors

out_path = case_dir / "parser_expected_summary.json"
out_path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")

if errors:
    for message in errors:
        print(f"[streaming-metrics-gate] {message}", file=sys.stderr)
    sys.exit(1)

sys.exit(0)
PY
}

option_requires_value() {
  case "$1" in
    --type-row-mode|--effect-stage|--effect-stage-runtime|--effect-stage-capability)
      return 0
      ;;
    --recover-expected-tokens|--recover-context|--recover-max-suggestions)
      return 0
      ;;
    --stream-resume-hint|--stream-flow-policy|--stream-flow-max-lag|--stream-demand-min-bytes|--stream-demand-preferred-bytes|--stream-checkpoint)
      return 0
      ;;
    --runtime-capabilities|--emit-typeck-debug|--emit-effects-metrics|--config|--left-recursion|--json-mode)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

resolve_placeholder_value() {
  local option="$1"
  local value="$2"
  local case_dir="$3"
  local typeck_dir="$4"
  local frontend="$5"
  local label="$frontend"
  case "${value}" in
    "<dir>")
      case "${option}" in
        --emit-typeck-debug)
          mkdir -p "${typeck_dir}"
          printf "%s/typeck-debug.%s.json" "${typeck_dir}" "${label}"
          ;;
        --emit-effects-metrics)
          mkdir -p "${case_dir}/effects"
          printf "%s/effects-metrics.%s.json" "${case_dir}/effects" "${label}"
          ;;
        *)
          printf "%s" "${case_dir}"
          ;;
      esac
      ;;
    *)
      printf "%s" "${value}"
      ;;
  esac
}

emit_case_flags() {
  local raw="$1"
  local case_dir="$2"
  local typeck_dir="$3"
  local frontend="$4"
  if [[ -z "${raw}" ]]; then
    return
  fi
  local tokens=()
  read -r -a tokens <<<"${raw}"
  local idx=0
  while [[ ${idx} -lt ${#tokens[@]} ]]; do
    local tok="${tokens[$idx]}"
    if [[ "${tok}" != -* ]]; then
      ((idx++))
      continue
    fi
    printf '%s\0' "${tok}"
    if option_requires_value "${tok}"; then
      ((idx++))
      if [[ ${idx} -ge ${#tokens[@]} ]]; then
        break
      fi
      local next_value="${tokens[$idx]}"
      printf '%s\0' "$(resolve_placeholder_value "${tok}" "${next_value}" "${case_dir}" "${typeck_dir}" "${frontend}")"
    fi
    ((idx++))
  done
}

append_case_flags() {
  local frontend="$1"
  local raw="$2"
  local case_dir="$3"
  local typeck_dir="$4"
  local flag
  if [[ -z "${raw}" ]]; then
    return
  fi
  while IFS= read -r -d '' flag; do
    if [[ "${frontend}" == "ocaml" ]]; then
      ocaml_case_flags+=("${flag}")
    else
      rust_case_flags+=("${flag}")
    fi
  done < <(emit_case_flags "${raw}" "${case_dir}" "${typeck_dir}" "${frontend}")
}

ensure_flag() {
  local target_name="$1"
  local flag="$2"
  eval "local current=(\"\${${target_name}[@]}\")"
  for existing in "${current[@]}"; do
    if [[ "${existing}" == "${flag}" ]]; then
      return
    fi
  done
  eval "${target_name}+=(\"\$flag\")"
}

ensure_flag_with_value() {
  local target_name="$1"
  local flag="$2"
  local value="$3"
  eval "local current=(\"\${${target_name}[@]}\")"
  local idx=0
  for existing in "${current[@]}"; do
    if [[ "${existing}" == "${flag}" ]]; then
      return
    fi
    ((idx++))
  done
  eval "${target_name}+=(\"\$flag\" \"\$value\")"
}

declare -a diag_ocaml_flags=()
declare -a diag_rust_flags=()
if [[ "${MODE}" == "diag" ]]; then
  diag_ocaml_flags+=(--left-recursion off)
fi

collect_all_metrics() {
  local diag_path="$1"
  local frontend="$2"
  local case_dir="$3"
  local status=0
  local section

  if [[ ! -s "$diag_path" ]]; then
    printf "!! %s diagnostics missing, skip metrics (%s)\n" "$frontend" "$diag_path" >&2
    return 1
  fi

  local sections=(parser effects)
  if needs_streaming_metrics "$diag_path"; then
    sections+=(streaming)
  else
    printf '%s\n' "-- streaming metrics skipped for ${frontend} (no parser.stream.* extensions)" >&2
  fi
  for section in "${sections[@]}"; do
    local out_path="${case_dir}/${section}-metrics.${frontend}.json"
    local err_path="${out_path%.json}.err.log"
    if python3 "${COLLECT_METRICS_SCRIPT}" \
      --section "${section}" \
      --source "${diag_path}" \
      --require-success \
      > "${out_path}" 2> "${err_path}"
    then
      rm -f "${err_path}"
    else
      printf "!! metrics (%s:%s) failed, see %s\n" "$frontend" "$section" "$err_path" >&2
      status=1
    fi
  done
  return $status
}

create_diag_diff() {
  local ocaml_path="$1"
  local rust_path="$2"
  local output_path="$3"
  if [[ ! -s "$ocaml_path" || ! -s "$rust_path" ]]; then
    return
  fi
  python3 - "$ocaml_path" "$rust_path" "$output_path" <<'PY' || true
import json
import pathlib
import sys
from difflib import unified_diff

ocaml_path = pathlib.Path(sys.argv[1])
rust_path = pathlib.Path(sys.argv[2])
out_path = pathlib.Path(sys.argv[3])

def load_sorted(path: pathlib.Path) -> str:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:  # pragma: no cover
        return f"<<failed to load {path}: {exc}>>"
    return json.dumps(data, ensure_ascii=False, indent=2, sort_keys=True)

ocaml_text = load_sorted(ocaml_path)
rust_text = load_sorted(rust_path)
diff_lines = list(
    unified_diff(
        ocaml_text.splitlines(),
        rust_text.splitlines(),
        fromfile="ocaml",
        tofile="rust",
        lineterm="",
    )
)

payload = {
    "ocaml_sorted": ocaml_text,
    "rust_sorted": rust_text,
    "diff": diff_lines,
    "delta": len(diff_lines),
}
out_path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
PY
}

collect_effects_metrics() {
  local diag_path="$1"
  local output_path="$2"
  local label="$3"

  if [[ ! -s "${diag_path}" ]]; then
    printf "!! %s diagnostics not found, skip effects metrics (%s)\n" "${label}" "${diag_path}" >&2
    return
  fi
  local err_path="${output_path%.json}.err.log"
  if python3 "${COLLECT_METRICS_SCRIPT}" \
    --section effects \
    --source "${diag_path}" \
    --require-success \
    > "${output_path}" 2> "${err_path}"
  then
    rm -f "${err_path}"
  else
    printf "!! collect-iterator-audit-metrics failed for %s (see %s)\n" "${label}" "${err_path}" >&2
  fi
}

validate_diagnostics_schema() {
  local ocaml_path="$1"
  local rust_path="$2"
  local log_path="$3"

  if [[ ! -s "${ocaml_path}" || ! -s "${rust_path}" ]]; then
    printf "!! Diagnostics validation skipped (missing files: %s / %s)\n" "${ocaml_path}" "${rust_path}" >&2
    return
  fi
  if bash "${VALIDATE_DIAG_SCRIPT}" "${ocaml_path}" "${rust_path}" > "${log_path}" 2>&1; then
    :
  else
    printf "!! Diagnostic schema validation failed (see %s)\n" "${log_path}" >&2
  fi
}

for idx in "${!CASE_ENTRIES[@]}"; do
  entry="${CASE_ENTRIES[$idx]}"
  case_name="${entry%%::*}"
  rest="${entry#*::}"
  if [[ "${case_name}" == "${entry}" ]]; then
    echo "無効なケース定義: ${entry}" >&2
    continue
  fi
  mode="${rest%%::*}"
  if [[ "${mode}" == "${rest}" ]]; then
    echo "無効なケース定義: ${entry}" >&2
    continue
  fi
  value="${rest#*::}"
  if [[ -z "${case_name}" || -z "${mode}" || -z "${value}" ]]; then
    echo "無効なケース定義: ${entry}" >&2
    continue
  fi

  safe_name="$(sanitize_name "${case_name}")"
  case_dir="${RUN_DIR}/${safe_name}"
  mkdir -p "${case_dir}"
  input_path="${case_dir}/input.reml"
  source_info="${case_dir}/source.txt"
  ocaml_ast_path="${case_dir}/ocaml.ast.txt"
  ocaml_tast_path="${case_dir}/ocaml.tast.txt"
  ocaml_diag_path="${case_dir}/ocaml.diagnostics.json"
  ocaml_parse_debug_path="${case_dir}/ocaml.parse-debug.json"
  rust_json_path="${case_dir}/rust.json"
  rust_parse_debug_path="${case_dir}/rust.parse-debug.json"
  typeck_dir="${case_dir}/typeck"
  effects_dir="${case_dir}/effects"
  mkdir -p "${typeck_dir}"
  mkdir -p "${effects_dir}"
  ocaml_typed_json="${typeck_dir}/typed-ast.ocaml.json"
  ocaml_constraints_json="${typeck_dir}/constraints.ocaml.json"
  ocaml_typeck_debug_json="${typeck_dir}/typeck-debug.ocaml.json"
  rust_typed_json="${typeck_dir}/typed-ast.rust.json"
  rust_constraints_json="${typeck_dir}/constraints.rust.json"
  rust_typeck_debug_json="${typeck_dir}/typeck-debug.rust.json"
  typeck_ocaml_flags=()
  if [[ "${MODE}" == "typeck" ]]; then
    typeck_ocaml_flags+=(
      --emit-typed-ast "${ocaml_typed_json}"
      --emit-constraints "${ocaml_constraints_json}"
      --emit-typeck-debug "${ocaml_typeck_debug_json}"
    )
  fi
  typeck_rust_flags=()
  if [[ "${MODE}" == "typeck" ]]; then
    typeck_rust_flags+=(
      --emit-typed-ast "${rust_typed_json}"
      --emit-constraints "${rust_constraints_json}"
      --emit-typeck-debug "${rust_typeck_debug_json}"
    )
  fi

  case "${mode}" in
    inline)
      printf "%b\n" "${value}" > "${input_path}"
      printf "inline\n" > "${source_info}"
      ;;
    file)
      src_path="${REPO_ROOT}/${value}"
      if [[ ! -f "${src_path}" ]]; then
        echo "ケース入力ファイルが見つかりません: ${value}" >&2
        continue
      fi
      cp "${src_path}" "${input_path}"
      printf "%s\n" "${value}" > "${source_info}"
      ;;
    *)
      echo "未知のケースモード (${mode}) : ${entry}" >&2
      continue
      ;;
  esac

  printf ">> ケース %s (safe=%s)\n" "${case_name}" "${safe_name}"
  case_tests="${CASE_TESTS_META[$idx]}"
  case_flags_raw="${CASE_FLAGS_META[$idx]}"
  case_flags_raw_ocaml="${CASE_FLAGS_META_OCAML[$idx]}"
  case_flags_raw_rust="${CASE_FLAGS_META_RUST[$idx]}"
  case_lsp="${CASE_LSP_META[$idx]}"
  declare -a ocaml_case_flags=()
  declare -a rust_case_flags=()
  append_case_flags "ocaml" "${case_flags_raw}" "${case_dir}" "${typeck_dir}"
  append_case_flags "rust" "${case_flags_raw}" "${case_dir}" "${typeck_dir}"
  append_case_flags "ocaml" "${case_flags_raw_ocaml}" "${case_dir}" "${typeck_dir}"
  append_case_flags "rust" "${case_flags_raw_rust}" "${case_dir}" "${typeck_dir}"
  if [[ "${case_name}" == type_* || "${case_name}" == effect_* || "${case_name}" == ffi_* ]]; then
    ensure_flag "ocaml_case_flags" "--experimental-effects"
    ensure_flag_with_value "ocaml_case_flags" "--type-row-mode" "dual-write"
    ensure_flag_with_value "ocaml_case_flags" "--effect-stage" "beta"
    ensure_flag_with_value "ocaml_case_flags" "--emit-typeck-debug" "${ocaml_typeck_debug_json}"
    ensure_flag "rust_case_flags" "--experimental-effects"
    ensure_flag_with_value "rust_case_flags" "--type-row-mode" "dual-write"
    ensure_flag_with_value "rust_case_flags" "--effect-stage" "beta"
    ensure_flag_with_value "rust_case_flags" "--emit-typeck-debug" "${rust_typeck_debug_json}"
    ensure_flag_with_value "rust_case_flags" "--emit-effects-metrics" "${effects_dir}/effects-metrics.rust.json"
  fi

  if [[ "${MODE}" == "diag" ]]; then
    ocaml_parse_debug_path="${case_dir}/ocaml.parse-debug.json"
    rust_parse_debug_path="${case_dir}/rust.parse-debug.json"
    ocaml_diag_path="${case_dir}/diagnostics.ocaml.json"
    rust_diag_path="${case_dir}/diagnostics.rust.json"
    schema_log="${case_dir}/schema-validate.log"
    diag_diff_path="${case_dir}/diagnostics.diff.json"
    case_gating="true"
    schema_ok="true"
    metrics_ok="true"

    ocaml_diag_cmd=(
      dune exec remlc --
      --packrat
      --format json
      --json-mode compact
      --emit-parse-debug "${ocaml_parse_debug_path}"
    )
    if [[ ${#diag_ocaml_flags[@]} -gt 0 ]]; then
      ocaml_diag_cmd+=("${diag_ocaml_flags[@]}")
    fi
    if [[ ${#ocaml_case_flags[@]} -gt 0 ]]; then
      ocaml_diag_cmd+=("${ocaml_case_flags[@]}")
    fi
    ocaml_diag_cmd+=("${input_path}")
    ocaml_stderr="${case_dir}/ocaml.stderr.log"
    run_in_dir "${OCAML_DIR}" "${ocaml_diag_cmd[@]}" > "${ocaml_diag_path}" 2> "${ocaml_stderr}" || true

    rust_diag_cmd=(
      cargo run --quiet --bin poc_frontend --
      --emit-parse-debug "${rust_parse_debug_path}"
      --dualwrite-root "${REPORT_DIR}"
      --dualwrite-run-label "${RUN_ID}"
      --dualwrite-case-label "${safe_name}"
    )
    if [[ ${#diag_rust_flags[@]} -gt 0 ]]; then
      rust_diag_cmd+=("${diag_rust_flags[@]}")
    fi
    if [[ ${#rust_case_flags[@]} -gt 0 ]]; then
      rust_diag_cmd+=("${rust_case_flags[@]}")
    fi
    rust_diag_cmd+=("${input_path}")
    run_in_dir "${RUST_DIR}" "${rust_diag_cmd[@]}" > "${rust_diag_path}" || true

    if ! bash "${VALIDATE_DIAG_SCRIPT}" "${ocaml_diag_path}" "${rust_diag_path}" > "${schema_log}" 2>&1; then
      schema_ok="false"
      case_gating="false"
    fi

    if ! collect_all_metrics "${ocaml_diag_path}" "ocaml" "${case_dir}"; then
      metrics_ok="false"
      case_gating="false"
    fi
    if ! collect_all_metrics "${rust_diag_path}" "rust" "${case_dir}"; then
      metrics_ok="false"
      case_gating="false"
    fi

    create_diag_diff "${ocaml_diag_path}" "${rust_diag_path}" "${diag_diff_path}"

    if is_streaming_case "${case_name}"; then
      if ! enforce_streaming_metrics_gate "${case_name}" "${case_dir}"; then
        metrics_ok="false"
        case_gating="false"
      fi
    fi

python3 - "${case_dir}" "${case_name}" "${safe_name}" "${RUN_ID}" "${REPORT_DIR}" "${MODE}" "${case_gating}" "${schema_ok}" "${metrics_ok}" "${case_tests}" "${case_lsp}" <<'PY'
import json
import pathlib
import sys

case_dir = pathlib.Path(sys.argv[1])
case_name = sys.argv[2]
safe_name = sys.argv[3]
run_id = sys.argv[4]
report_root = pathlib.Path(sys.argv[5])
mode = sys.argv[6]
gating_flag = sys.argv[7].lower() == "true"
schema_ok = sys.argv[8].lower() == "true"
metrics_ok = sys.argv[9].lower() == "true"
case_tests = sys.argv[10]
case_lsp = sys.argv[11]

def read_text(path: pathlib.Path) -> str:
    if not path.exists():
        return ""
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return path.read_bytes().decode("utf-8", errors="replace")

def load_json(path: pathlib.Path):
    if not path.exists():
        return None
    text = read_text(path).strip()
    if not text:
        return None
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return None

def summarize_parser_metrics(data):
    if not isinstance(data, dict):
        return None
    metrics = data.get("metrics")
    if not isinstance(metrics, list):
        return None
    summary = {}
    for item in metrics:
        if not isinstance(item, dict):
            continue
        name = item.get("metric")
        if name == "diagnostic.audit_presence_rate":
            summary["audit_pass_fraction"] = item.get("pass_fraction")
            summary["audit_pass_rate"] = item.get("pass_rate")
        if name == "parser.expected_summary_presence":
            summary["expected_pass_fraction"] = item.get("pass_fraction")
    return summary or None

def summarize_streaming_metrics(data):
    if not isinstance(data, dict):
        return None
    metrics = data.get("metrics")
    if not isinstance(metrics, list):
        return None
    summary = {}
    for item in metrics:
        if not isinstance(item, dict):
            continue
        name = item.get("metric")
        if name == "diagnostic.audit_presence_rate":
            summary["audit_pass_fraction"] = item.get("pass_fraction")
        if name == "parser.stream.outcome_consistency":
            summary["outcome_pass_rate"] = item.get("pass_rate")
    return summary or None

def summarize_effects_metrics(data):
    if not isinstance(data, dict):
        return None
    metrics = data.get("metrics")
    if not isinstance(metrics, list):
        return None
    summary = {}
    for item in metrics:
        if not isinstance(item, dict):
            continue
        if item.get("metric") == "effect_row_guard_regressions":
            summary["regressions"] = item.get("count")
    return summary or None

ocaml_diag = load_json(case_dir / "diagnostics.ocaml.json") or {}
rust_diag = load_json(case_dir / "diagnostics.rust.json") or {}
ocaml_diagnostics = ocaml_diag.get("diagnostics") or []
rust_diagnostics = rust_diag.get("diagnostics") or []

parser_metrics_ocaml = load_json(case_dir / "parser-metrics.ocaml.json")
parser_metrics_rust = load_json(case_dir / "parser-metrics.rust.json")
effects_metrics_ocaml = load_json(case_dir / "effects-metrics.ocaml.json")
effects_metrics_rust = load_json(case_dir / "effects-metrics.rust.json")
streaming_metrics_ocaml = load_json(case_dir / "streaming-metrics.ocaml.json")
streaming_metrics_rust = load_json(case_dir / "streaming-metrics.rust.json")

summary = {
    "case": case_name,
    "mode": mode,
    "source": read_text(case_dir / "source.txt").strip() or "inline",
    "ocaml_diag_count": len(ocaml_diagnostics),
    "rust_diag_count": len(rust_diagnostics),
    "diag_match": len(ocaml_diagnostics) == len(rust_diagnostics),
    "gating": gating_flag,
    "schema_ok": schema_ok,
    "metrics_ok": metrics_ok,
    "diag": {
        "parser": {
            "ocaml": summarize_parser_metrics(parser_metrics_ocaml),
            "rust": summarize_parser_metrics(parser_metrics_rust),
        },
        "effects": {
            "ocaml": summarize_effects_metrics(effects_metrics_ocaml),
            "rust": summarize_effects_metrics(effects_metrics_rust),
        },
        "streaming": {
            "ocaml": summarize_streaming_metrics(streaming_metrics_ocaml),
            "rust": summarize_streaming_metrics(streaming_metrics_rust),
        },
    },
}
if case_tests:
    summary["tests"] = case_tests
if case_lsp:
    summary["lsp_fixture"] = case_lsp

(case_dir / "summary.json").write_text(
    json.dumps(summary, ensure_ascii=False, indent=2),
    encoding="utf-8",
)
PY
    continue
  fi

  (
    cd "${OCAML_DIR}"
    dune exec remlc -- --emit-ast "${input_path}"
  ) > "${ocaml_ast_path}" 2>&1 || true

  (
    cd "${OCAML_DIR}"
    dune exec remlc -- --emit-tast "${input_path}"
  ) > "${ocaml_tast_path}" 2>&1 || true

  (
    cd "${OCAML_DIR}"
    dune exec remlc -- \
      --packrat \
      --format json \
      --json-mode compact \
      --emit-parse-debug "${ocaml_parse_debug_path}" \
      "${typeck_ocaml_flags[@]}" \
      "${ocaml_case_flags[@]}" \
      "${input_path}"
  ) > "${ocaml_diag_path}" 2>&1 || true

  (
    cd "${RUST_DIR}"
    cargo run --quiet --bin poc_frontend -- \
      --emit-parse-debug "${rust_parse_debug_path}" \
      --dualwrite-root "${REPORT_DIR}" \
      --dualwrite-run-label "${RUN_ID}" \
      --dualwrite-case-label "${safe_name}" \
      "${typeck_rust_flags[@]}" \
      "${rust_case_flags[@]}" \
      "${input_path}"
  ) > "${rust_json_path}" || true

  if [[ "${MODE}" == "typeck" ]]; then
    collect_effects_metrics "${ocaml_diag_path}" "${typeck_dir}/effects-metrics.ocaml.json" "OCaml"
    collect_effects_metrics "${rust_json_path}" "${typeck_dir}/effects-metrics.rust.json" "Rust"
    validate_diagnostics_schema "${ocaml_diag_path}" "${rust_json_path}" "${typeck_dir}/diagnostic-validate.log"
  fi

python3 - "${case_dir}" "${case_name}" "${safe_name}" "${RUN_ID}" "${REPORT_DIR}" "${MODE}" <<'PY'
import json
import pathlib
import sys
import re

case_dir = pathlib.Path(sys.argv[1])
case_name = sys.argv[2]
safe_name = sys.argv[3]
run_id = sys.argv[4]
report_root = pathlib.Path(sys.argv[5])
mode = sys.argv[6]
typeck_dir = case_dir / "typeck"

def read_text(path: pathlib.Path) -> str:
    if not path.exists():
        return ""
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return path.read_bytes().decode("utf-8", errors="replace")

def load_json(path: pathlib.Path):
    if not path.exists():
        return None
    text = read_text(path).strip()
    if not text:
        return None
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return None

def derive_ocaml_typeck_metrics(tast_text: str, summary_json, source_text: str):
    typed_functions = 0
    typed_exprs = 0
    functions = []
    fallback_functions = []
    has_summary = isinstance(summary_json, dict)
    if has_summary:
        fn_list = summary_json.get("function_summaries")
        if isinstance(fn_list, list):
            typed_functions = len(fn_list)
            functions = fn_list
    if not has_summary and tast_text:
        for line in tast_text.splitlines():
            stripped = line.strip()
            if not stripped:
                continue
            if stripped.startswith("fn "):
                typed_functions += 1
                name = stripped[3:].split("(")[0].strip()
                fallback_functions.append(
                    {
                        "name": name,
                        "param_types": [],
                        "return_type": "Unknown",
                        "source": "typed_ast_text",
                    }
                )
            if stripped.startswith("(") or stripped.startswith("case "):
                typed_exprs += 1
    if not functions and fallback_functions:
        functions = fallback_functions
    if typed_functions == 0:
        guessed_names = guess_function_names_from_source(source_text)
        typed_functions = len(guessed_names)
        if not functions and guessed_names:
            functions = [
                {
                    "name": name,
                    "param_types": [],
                    "return_type": "Unknown",
                    "source": "source_scan",
                }
                for name in guessed_names
            ]
    if typed_exprs == 0 and typed_functions > 0:
        typed_exprs = typed_functions

    constraint_total = typed_exprs if has_summary else 0
    constraint_breakdown = {"ocaml_stub": typed_exprs} if has_summary else {}

    metrics = {
        "typed_functions": typed_functions,
        "typed_exprs": typed_exprs,
        "constraints_total": constraint_total,
        "constraint_breakdown": constraint_breakdown,
        "unresolved_identifiers": 0,
        "call_sites": 0,
        "binary_expressions": 0,
    }
    return {"metrics": metrics, "typed_functions": functions}

def diff_typeck_metrics(ocaml, rust):
    result = {"match": False, "fields": {}}
    if not ocaml or not rust:
        return result
    o_metrics = ocaml.get("metrics") or {}
    r_metrics = rust.get("metrics") or {}
    fields = {}
    for key in sorted(set(o_metrics.keys()) | set(r_metrics.keys())):
        o_val = o_metrics.get(key)
        r_val = r_metrics.get(key)
        delta = None
        if isinstance(o_val, (int, float)) and isinstance(r_val, (int, float)):
            delta = (r_val or 0) - (o_val or 0)
        fields[key] = {"ocaml": o_val, "rust": r_val, "delta": delta}
    result["fields"] = fields
    result["match"] = all(
        (item["delta"] is None or item["delta"] == 0) for item in fields.values()
    )
    return result

def packrat_numbers(stats):
    if isinstance(stats, dict):
        return int(stats.get("queries", 0) or 0), int(stats.get("hits", 0) or 0)
    return 0, 0

def guess_function_names_from_source(source_text: str):
    if not source_text:
        return []
    pattern = re.compile(r"\bfn\s+([A-Za-z0-9_]+)")
    return pattern.findall(source_text)

def build_impl_registry_payload(frontend: str, case: str, run: str, typed_functions):
    typed_functions = typed_functions or []
    entries = []
    for index, fn in enumerate(typed_functions):
        if isinstance(fn, dict):
            name = fn.get("name") or f"{frontend}_fn_{index}"
            entries.append(
                {
                    "index": index,
                    "impl_name": name,
                    "trait_path": fn.get("trait_path") or fn.get("trait"),
                    "impl_type": fn.get("return_type"),
                    "param_types": fn.get("param_types"),
                    "origin": fn.get("source") or "typed_functions",
                }
            )
        elif isinstance(fn, str):
            entries.append(
                {
                    "index": index,
                    "impl_name": fn,
                    "trait_path": None,
                    "impl_type": None,
                    "param_types": None,
                    "origin": "typed_functions",
                }
            )
    return {
        "schema_version": "w3-typeck-impl-registry/0.1",
        "frontend": frontend,
        "case": case,
        "run_id": run,
        "entries": entries,
    }

def ensure_impl_registry_snapshot(path: pathlib.Path, payload: dict):
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and path.stat().st_size > 0:
        return
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")

source_info = read_text(case_dir / "source.txt").strip()
source_code = read_text(case_dir / "input.reml")
ocaml_ast = read_text(case_dir / "ocaml.ast.txt").strip()
ocaml_tast = read_text(case_dir / "ocaml.tast.txt").strip()
ocaml_diag_json = load_json(case_dir / "ocaml.diagnostics.json") or {}
ocaml_parse_debug = load_json(case_dir / "ocaml.parse-debug.json") or {}
rust_json = load_json(case_dir / "rust.json") or {}

ocaml_diagnostics = ocaml_diag_json.get("diagnostics", [])
ocaml_parse_result = ocaml_parse_debug.get("parse_result") or ocaml_diag_json.get("parse_result") or {}
ocaml_stream_meta = ocaml_parse_debug.get("stream_meta")

ocaml_queries, ocaml_hits = packrat_numbers(ocaml_parse_result.get("packrat_stats"))
rust_diagnostics = rust_json.get("diagnostics", [])
rust_parse_result = rust_json.get("parse_result") or {}
rust_queries, rust_hits = packrat_numbers(rust_parse_result.get("packrat_stats"))
rust_span_trace_len = len(rust_parse_result.get("span_trace") or [])

summary = {
    "case": case_name,
    "source": source_info or "inline",
    "ast_match": (ocaml_ast == rust_json.get("ast_render")),
    "ocaml_ast": ocaml_ast,
    "rust_ast": rust_json.get("ast_render", ""),
    "ocaml_diag_count": len(ocaml_diagnostics),
    "rust_diag_count": len(rust_diagnostics),
    "diag_match": len(ocaml_diagnostics) == len(rust_diagnostics),
    "ocaml_diag_messages": [diag.get("message") for diag in ocaml_diagnostics],
    "rust_diag_messages": [diag.get("message") for diag in rust_diagnostics],
    "ocaml_packrat_queries": ocaml_queries,
    "ocaml_packrat_hits": ocaml_hits,
    "ocaml_span_trace_len": len(ocaml_parse_result.get("span_trace") or []),
    "ocaml_stream_meta": ocaml_stream_meta,
    "rust_packrat_queries": rust_queries,
    "rust_packrat_hits": rust_hits,
    "rust_span_trace_len": rust_span_trace_len,
    "ocaml_tast_lines": ocaml_tast.count("\\n") + 1 if ocaml_tast else 0,
    "ocaml_tast_available": bool(ocaml_tast),
}

if mode == "typeck":
    ocaml_summary_json = load_json(typeck_dir / "typed-ast.ocaml.json")
    ocaml_metrics = derive_ocaml_typeck_metrics(ocaml_tast, ocaml_summary_json, source_code)
    rust_metrics = load_json(typeck_dir / "metrics.json")
    typeck_diff = diff_typeck_metrics(ocaml_metrics, rust_metrics)
    summary["typeck_metrics"] = {
        "ocaml": ocaml_metrics.get("metrics") if ocaml_metrics else None,
        "rust": rust_metrics.get("metrics") if rust_metrics else None,
        "match": typeck_diff.get("match", False),
    }
    (case_dir / "typeck.diff.json").write_text(
        json.dumps(typeck_diff, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    ocaml_impl_payload = build_impl_registry_payload(
        "ocaml",
        case_name,
        run_id,
        ocaml_metrics.get("typed_functions") if ocaml_metrics else None,
    )
    rust_impl_payload = build_impl_registry_payload(
        "rust",
        case_name,
        run_id,
        rust_metrics.get("typed_functions") if isinstance(rust_metrics, dict) else None,
    )
    ensure_impl_registry_snapshot(typeck_dir / "impl-registry.ocaml.json", ocaml_impl_payload)
    ensure_impl_registry_snapshot(typeck_dir / "impl-registry.rust.json", rust_impl_payload)

(case_dir / "summary.json").write_text(
    json.dumps(summary, ensure_ascii=False, indent=2),
    encoding="utf-8",
)
PY
done

python3 - "${RUN_DIR}" "${MODE}" <<'PY'
import json
import pathlib
import sys

run_dir = pathlib.Path(sys.argv[1])
mode = sys.argv[2]
summaries = []
for path in sorted(run_dir.glob("*/summary.json")):
    summaries.append(json.loads(path.read_text(encoding="utf-8")))

if mode == "diag":
    lines = [
        "| case | source | gating | schema | metrics | diag_match | ocaml_diag | rust_diag |",
        "| --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for summary in summaries:
        lines.append(
            "| {case} | {source} | {gating} | {schema} | {metrics} | {diag_match} | {ocaml} | {rust} |".format(
                case=summary.get("case"),
                source=summary.get("source"),
                gating="✅" if summary.get("gating") else "❌",
                schema="✅" if summary.get("schema_ok") else "❌",
                metrics="✅" if summary.get("metrics_ok") else "❌",
                diag_match="✅" if summary.get("diag_match") else "❌",
                ocaml=summary.get("ocaml_diag_count"),
                rust=summary.get("rust_diag_count"),
            )
        )
    content = "\n".join(lines) + "\n"
else:
    lines = [
        "| case | source | ast_match | diag_match | typeck_match | ocaml_diag | rust_diag | ocaml_packrat (q/h) | rust_packrat (q/h) |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for summary in summaries:
        typeck_match = (
            summary.get("typeck_metrics", {}).get("match")
            if isinstance(summary.get("typeck_metrics"), dict)
            else None
        )
        lines.append(
            f"| {summary['case']} | {summary['source']} | {summary.get('ast_match')} | "
            f"{summary.get('diag_match')} | {typeck_match} | {summary.get('ocaml_diag_count')} | {summary.get('rust_diag_count')} | "
            f"{summary.get('ocaml_packrat_queries')}/{summary.get('ocaml_packrat_hits')} | "
            f"{summary.get('rust_packrat_queries')}/{summary.get('rust_packrat_hits')} |"
        )
    content = "\n".join(lines) + "\n"

(run_dir / "summary.md").write_text(content, encoding="utf-8")
PY

printf "==> サマリ: %s\n" "${RUN_DIR}/summary.md"
