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
Usage: scripts/poc_dualwrite_compare.sh [--run-id <id>] [--cases <path>] [--mode <ast|typeck>]

Options:
  --run-id <id>     出力ディレクトリ名を上書き（既定: 2025-11-28-logos-chumsky）
  --cases <path>    ケース定義ファイル（format: name::inline::<src> | name::file::<path>）
  --mode <ast|typeck>
                     実行モード（typeck は型推論成果物を収集）
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
elif [[ "${MODE}" != "ast" ]]; then
  echo "Unsupported mode: ${MODE}" >&2
  exit 1
fi

declare -a CASE_ENTRIES=()

if [[ -n "${CASES_FILE}" ]]; then
  if [[ ! -f "${CASES_FILE}" ]]; then
    echo "ケース定義ファイルが見つかりません: ${CASES_FILE}" >&2
    exit 1
  fi
  while IFS= read -r line || [[ -n "$line" ]]; do
    trimmed="${line%%#*}"
    trimmed="$(printf '%s' "${trimmed}" | sed 's/[[:space:]]*$//')"
    if [[ -z "${trimmed}" ]]; then
      continue
    fi
    CASE_ENTRIES+=("${trimmed}")
  done < "${CASES_FILE}"
else
  CASE_ENTRIES+=(
    "empty_uses::inline::fn answer() = 42"
    "multiple_functions::inline::fn log(x) = x\nfn log_twice(x) = log(log(x))"
    "addition::inline::fn add(x, y) = x + y"
    "missing_paren::inline::fn missing(x = x"
  )
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

for entry in "${CASE_ENTRIES[@]}"; do
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
  if [[ "${MODE}" == "typeck" ]]; then
    mkdir -p "${typeck_dir}"
    ocaml_typed_json="${typeck_dir}/typed-ast.ocaml.json"
    ocaml_constraints_json="${typeck_dir}/constraints.ocaml.json"
    ocaml_typeck_debug_json="${typeck_dir}/typeck-debug.ocaml.json"
    rust_typed_json="${typeck_dir}/typed-ast.rust.json"
    rust_constraints_json="${typeck_dir}/constraints.rust.json"
    rust_typeck_debug_json="${typeck_dir}/typeck-debug.rust.json"
  fi
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
            if stripped.startswith("(") or stripped.startswith("case "):
                typed_exprs += 1
    if typed_functions == 0:
        typed_functions = estimate_functions_from_source(source_text)
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

def estimate_functions_from_source(source_text: str) -> int:
    count = 0
    tokens = source_text.replace("\n", " ").split()
    for token in tokens:
        if token == "fn":
            count += 1
    return count

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
        f"| {summary['case']} | {summary['source']} | {summary['ast_match']} | "
        f"{summary['diag_match']} | {typeck_match} | {summary['ocaml_diag_count']} | {summary['rust_diag_count']} | "
        f"{summary['ocaml_packrat_queries']}/{summary['ocaml_packrat_hits']} | "
        f"{summary['rust_packrat_queries']}/{summary['rust_packrat_hits']} |"
    )

(run_dir / "summary.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

printf "==> サマリ: %s\n" "${RUN_DIR}/summary.md"
