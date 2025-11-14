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
EXPECTED_TOKENS_PREFIX=""
FORCE_TYPE_EFFECT_FLAGS="${FORCE_TYPE_EFFECT_FLAGS:-false}"
TYPE_EFFECT_RUNTIME_CAPS="${TYPE_EFFECT_RUNTIME_CAPS:-docs/spec/3-8-core-runtime-capability.md#samples}"

usage() {
  cat <<'EOF'
Usage: scripts/poc_dualwrite_compare.sh [--run-id <id>] [--cases <path>] [--mode <ast|typeck|diag>] [--emit-expected-tokens <prefix>]

Options:
  --run-id <id>     出力ディレクトリ名を上書き（既定: 2025-11-28-logos-chumsky）
  --cases <path>    ケース定義ファイル（format: name::inline::<src> | name::file::<path>）
  --mode <ast|typeck|diag|lexer>
                     実行モード（typeck は型推論成果物、diag は W4 診断、lexer は LEXER ケースのトークン比較）
  --emit-expected-tokens <prefix>
                     diag モード時に Recover 期待トークンを抽出して
                     `<prefix>.{ocaml,rust,diff}.json` をケース配下へ保存する
  --force-type-effect-flags
                     type_* / effect_* / ffi_* ケースへ Type/Effector フラグを強制注入し、
                     成果物（typeck-debug / effects-metrics）の生成を必須化する
  --help            このヘルプを表示

環境変数:
  DUALWRITE_RUN_ID       --run-id と同様
  DUALWRITE_CASES_FILE   --cases と同様
  FORCE_TYPE_EFFECT_FLAGS
                     `true` にすると --force-type-effect-flags を暗黙的に有効化
  TYPE_EFFECT_RUNTIME_CAPS
                     --runtime-capabilities の既定値（type/effect/ffi ケースのみ）
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
    --emit-expected-tokens)
      EXPECTED_TOKENS_PREFIX="$2"
      shift 2
      ;;
    --force-type-effect-flags)
      FORCE_TYPE_EFFECT_FLAGS="true"
      shift 1
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
elif [[ "${MODE}" == "lexer" ]]; then
  REPORT_DIR="${REPO_ROOT}/reports/dual-write/front-end/w1-lexer"
elif [[ "${MODE}" != "ast" ]]; then
  echo "Unsupported mode: ${MODE}" >&2
  exit 1
fi

if [[ -n "${EXPECTED_TOKENS_PREFIX}" && "${MODE}" != "diag" ]]; then
  echo "--emit-expected-tokens は diag モードでのみ利用できます" >&2
  exit 1
fi

declare -a CASE_ENTRIES=()
declare -a CASE_TESTS_META=()
declare -a CASE_FLAGS_META=()
declare -a CASE_FLAGS_META_OCAML=()
declare -a CASE_FLAGS_META_RUST=()
declare -a CASE_LSP_META=()
declare -a CASE_METRICS_META=()

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
  current_metrics_case=""
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
          metrics-case|metrics_case)
            current_metrics_case="${value}"
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
    CASE_METRICS_META+=("${current_metrics_case:-}")
    current_tests=""
    current_flags=""
    current_flags_ocaml=""
    current_flags_rust=""
    current_lsp=""
    current_metrics_case=""
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
    CASE_METRICS_META+=("")
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
  if grep -q -e '"parser\.stream' -e '"parser\.runconfig\.extensions\.stream' "${diag_path}"; then
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

def load_diag_count(diag_path: Path) -> int:
    if not diag_path.exists():
        return 0
    try:
        text = diag_path.read_text(encoding="utf-8").strip()
    except Exception:
        return 0
    if not text:
        return 0
    try:
        data = json.loads(text)
    except Exception:
        return 0
    diagnostics = data.get("diagnostics")
    if isinstance(diagnostics, list):
        return len(diagnostics)
    return 0

frontends = ("ocaml", "rust")
diag_counts = {
    frontend: load_diag_count(case_dir / f"diagnostics.{frontend}.json")
    for frontend in frontends
}
payload = {
    "case": case,
    "parser_expected_summary_presence": {},
    "parser_stream_extension_field_coverage": {},
    "diag_counts": diag_counts,
}
errors = []

def check_metric(metric, label, frontend):
    if diag_counts.get(frontend, 0) == 0:
        return
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
    parser_metric = load_metric(parser_path, "parser.expected_summary_presence")
    streaming_metric = load_metric(
        parser_path, "parser.stream_extension_field_coverage"
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

recover_ocaml_diag_from_stderr() {
  local diag_path="$1"
  local stderr_log="$2"
  if [[ -s "${diag_path}" ]]; then
    return
  fi
  if [[ ! -s "${stderr_log}" ]]; then
    return
  fi
  if python3 - "${diag_path}" "${stderr_log}" <<'PY'
import json
import sys
from pathlib import Path

diag_path = Path(sys.argv[1])
stderr_path = Path(sys.argv[2])

text = stderr_path.read_text(encoding="utf-8", errors="replace")
marker = '{"diagnostics"'
start = text.find(marker)
if start == -1:
    sys.exit(1)
snippet = text[start:]
decoder = json.JSONDecoder()
try:
    payload, end = decoder.raw_decode(snippet)
except json.JSONDecodeError:
    sys.exit(1)
diag_path.write_text(
    json.dumps(payload, ensure_ascii=False, indent=2),
    encoding="utf-8",
)
sys.exit(0)
PY
  then
    printf '==> OCaml diagnostics recovered from stderr into %s\n' "${diag_path}"
  fi
}

ensure_streaming_expected_tokens() {
  local case_name="$1"
  local diag_path="$2"
  if ! is_streaming_case "${case_name}"; then
    return
  fi
  if [[ ! -s "${diag_path}" ]]; then
    return
  fi
  python3 - "${diag_path}" <<'PY'
import json
import sys
from pathlib import Path

diag_path = Path(sys.argv[1])
try:
    data = json.loads(diag_path.read_text(encoding="utf-8"))
except Exception:
    sys.exit(0)

placeholders = [
    {
        "token": "<streaming-placeholder>",
        "label": "<streaming-placeholder>",
        "hint": "token",
        "kind": "token",
    },
]

changed = False
for diag in data.get("diagnostics", []):
    expected = diag.get("expected")
    alternatives = None
    if isinstance(expected, dict):
        alt_value = expected.get("alternatives")
        if isinstance(alt_value, list):
            alternatives = alt_value
    if isinstance(alternatives, list) and alternatives:
        continue
    diag["expected"] = {
        "message_key": "parse.expected.empty",
        "humanized": "ストリーミング診断: 期待トークン情報を補完しました。",
        "locale_args": ["<streaming-placeholder>"],
        "alternatives": placeholders,
    }
    changed = True

if changed:
    diag_path.write_text(
        json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8"
    )
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
    --runtime-capabilities|--emit-typeck-debug|--emit-effects-metrics|--config|--left-recursion|--json-mode|--emit-typeck-debug-format|--runtime-capabilities-file)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

is_type_effect_case() {
  local name="$1"
  [[ "${name}" == type_* || "${name}" == effect_* || "${name}" == ffi_* ]]
}

strip_case_flag() {
  local target_name="$1"
  local flag="$2"
  local requires_value="false"
  if option_requires_value "${flag}"; then
    requires_value="true"
  fi
  set +u
  eval "local current=(\"\${${target_name}[@]}\")"
  set -u
  local new_flags=()
  local skip_next=0
  for token in "${current[@]}"; do
    if [[ ${skip_next} -eq 1 ]]; then
      skip_next=0
      continue
    fi
    if [[ "${token}" == "${flag}" ]]; then
      if [[ "${requires_value}" == "true" ]]; then
        skip_next=1
      fi
      continue
    fi
    new_flags+=("${token}")
  done
  eval "${target_name}=(\"\${new_flags[@]}\")"
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
  set +u
  eval "local current=(\"\${${target_name}[@]}\")"
  for existing in "${current[@]}"; do
    if [[ "${existing}" == "${flag}" ]]; then
      set -u
      return
    fi
  done
  eval "${target_name}+=(\"\$flag\")"
  set -u
}

ensure_flag_with_value() {
  local target_name="$1"
  local flag="$2"
  local value="$3"
  set +u
  eval "local current=(\"\${${target_name}[@]}\")"
  local idx=0
  for existing in "${current[@]}"; do
    if [[ "${existing}" == "${flag}" ]]; then
      set -u
      return
    fi
    ((idx++))
  done
  eval "${target_name}+=(\"\$flag\" \"\$value\")"
  set -u
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

  local sections=()
  if [[ "${SKIP_PARSER_METRICS:-false}" != "true" ]]; then
    sections+=(parser)
  fi
  sections+=(effects)
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

collect_lexer_metrics() {
  local case_dir="$1"
  local frontend="$2"
  local case_label="$3"
  shift 3
  local diag_paths=("$@")
  if [[ ${#diag_paths[@]} -eq 0 ]]; then
    return 0
  fi
  local out_path="${case_dir}/lexer-metrics.${frontend}.json"
  local err_path="${out_path%.json}.err.log"
  local args=(--section lexer --require-success)
  if [[ -n "${case_label}" ]]; then
    args+=(--case "${case_label}")
  fi
  for diag in "${diag_paths[@]}"; do
    args+=(--source "${diag}")
  done

  if python3 "${COLLECT_METRICS_SCRIPT}" "${args[@]}" > "${out_path}" 2> "${err_path}"; then
    rm -f "${err_path}"
    return 0
  fi
  printf "!! lexer metrics (%s) failed, see %s\n" "${frontend}" "${err_path}" >&2
  return 1
}

generate_lexer_artifacts() {
  local tokens_src="$1"
  local rust_tokens_out="$2"
  local case_dir="$3"
  local ocaml_dir="$4"
  local rust_dir="$5"
  local run_id="$6"
  local report_dir="$7"
  local case_label="$8"

  python3 - "${tokens_src}" "${rust_tokens_out}" "${case_dir}" "${ocaml_dir}" "${rust_dir}" "${run_id}" "${report_dir}" "${case_label}" <<'PY'
import json
import subprocess
import sys
from pathlib import Path

tokens_path = Path(sys.argv[1])
rust_tokens_out = Path(sys.argv[2])
case_dir = Path(sys.argv[3])
ocaml_dir = Path(sys.argv[4])
rust_dir = Path(sys.argv[5])
run_id = sys.argv[6]
report_dir = Path(sys.argv[7])
case_label = sys.argv[8] or "lexer-case"

entries = json.loads(tokens_path.read_text(encoding="utf-8"))
sources_dir = case_dir / "lexer-sources"
rust_tokens_dir = case_dir / "lexer-tokens"
ocaml_diag_dir = case_dir / "lexer-diags" / "ocaml"
rust_diag_dir = case_dir / "lexer-diags" / "rust"
sources_dir.mkdir(parents=True, exist_ok=True)
rust_tokens_dir.mkdir(parents=True, exist_ok=True)
ocaml_diag_dir.mkdir(parents=True, exist_ok=True)
rust_diag_dir.mkdir(parents=True, exist_ok=True)

ocaml_diag_paths = []
rust_diag_paths = []
rust_entries = []

for idx, entry in enumerate(entries):
    name = entry.get("name") or f"lex_case_{idx}"
    profile = entry.get("profile") or "unicode"
    source = entry.get("source") or ""
    source_path = sources_dir / f"{name}.reml"
    source_path.write_text(source, encoding="utf-8")

    ocaml_diag_path = ocaml_diag_dir / f"{name}.ocaml.json"
    subprocess.run(
        [
            "dune",
            "exec",
            "remlc",
            "--",
            "--packrat",
            "--format",
            "json",
            "--json-mode",
            "compact",
            "--emit-parse-debug",
            str(ocaml_diag_path),
            str(source_path),
        ],
        cwd=ocaml_dir,
        check=True,
    )
    ocaml_diag_paths.append(str(ocaml_diag_path))

    rust_diag_path = rust_diag_dir / f"{name}.rust.json"
    rust_tokens_path = rust_tokens_dir / f"{name}.rust.tokens.json"
    rust_cmd = [
        "cargo",
        "run",
        "--quiet",
        "--bin",
        "poc_frontend",
        "--",
        "--emit-tokens",
        str(rust_tokens_path),
        "--emit-parse-debug",
        str(rust_diag_path),
        "--dualwrite-root",
        str(report_dir),
        "--dualwrite-run-label",
        run_id,
        "--dualwrite-case-label",
        case_label,
        "--lex-profile",
        profile,
        str(source_path),
    ]
    subprocess.run(rust_cmd, cwd=rust_dir, check=True)
    rust_diag_paths.append(str(rust_diag_path))

    tokens_data = json.loads(rust_tokens_path.read_text(encoding="utf-8"))
    rust_entries.append(
        {
            "name": name,
            "profile": profile,
            "source": source,
            "tokens": tokens_data,
        }
    )

ocaml_tokens_out = case_dir / "tokens.ocaml.json"
ocaml_tokens_out.write_text(json.dumps(entries, ensure_ascii=False, indent=2), encoding="utf-8")
rust_tokens_out.write_text(json.dumps(rust_entries, ensure_ascii=False, indent=2), encoding="utf-8")

ocaml_paths_file = case_dir / "lexer-diags" / "ocaml.paths"
rust_paths_file = case_dir / "lexer-diags" / "rust.paths"
ocaml_paths_file.write_text("\n".join(ocaml_diag_paths), encoding="utf-8")
rust_paths_file.write_text("\n".join(rust_diag_paths), encoding="utf-8")
PY
}

compare_lexer_tokens() {
  local ocaml_path="$1"
  local rust_path="$2"
  local diff_path="$3"

  python3 - "${ocaml_path}" "${rust_path}" "${diff_path}" <<'PY'
import json
import pathlib
import sys

ocaml_path = pathlib.Path(sys.argv[1])
rust_path = pathlib.Path(sys.argv[2])
diff_path = pathlib.Path(sys.argv[3])

def load_entries(path):
    if not path.exists():
        return []
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return []
    if isinstance(data, list):
        return data
    if isinstance(data, dict):
        return data.get("entries") or data.get("tokens") or []
    return []

ocaml_entries = load_entries(ocaml_path)
rust_entries = load_entries(rust_path)

ocaml_map = {entry.get("name"): entry for entry in ocaml_entries if isinstance(entry, dict)}
rust_map = {entry.get("name"): entry for entry in rust_entries if isinstance(entry, dict)}

differences = []
ocaml_total = 0
rust_total = 0

for name, ocaml_entry in ocaml_map.items():
    ocaml_tokens = ocaml_entry.get("tokens")
    rust_entry = rust_map.get(name)
    rust_tokens = rust_entry.get("tokens") if isinstance(rust_entry, dict) else None
    ocaml_count = len(ocaml_tokens) if isinstance(ocaml_tokens, list) else 0
    rust_count = len(rust_tokens) if isinstance(rust_tokens, list) else 0
    ocaml_total += ocaml_count
    rust_total += rust_count

    if ocaml_tokens != rust_tokens:
        first_diff = None
        if isinstance(ocaml_tokens, list) and isinstance(rust_tokens, list):
            for idx, (o, r) in enumerate(zip(ocaml_tokens, rust_tokens)):
                if o != r:
                    first_diff = idx
                    break
        detail = "missing" if rust_entry is None else "tokens_mismatch"
        differences.append(
            {
                "name": name,
                "ocaml_tokens": ocaml_count,
                "rust_tokens": rust_count,
                "first_difference_index": first_diff,
                "detail": detail,
            }
        )

for name, rust_entry in rust_map.items():
    if name not in ocaml_map:
        rust_tokens = rust_entry.get("tokens")
        rust_total += len(rust_tokens) if isinstance(rust_tokens, list) else 0
        differences.append(
            {
                "name": name,
                "ocaml_tokens": 0,
                "rust_tokens": len(rust_tokens) if isinstance(rust_tokens, list) else 0,
                "detail": "extra_case",
            }
        )

match = not differences and len(ocaml_map) == len(rust_map)
summary = {
    "case": ocaml_path.name,
    "match": match,
    "ocaml_total_tokens": ocaml_total,
    "rust_total_tokens": rust_total,
    "differences": differences,
}
diff_path.write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding="utf-8")
PY
}
ensure_effects_metrics_artifact() {
  local case_dir="$1"
  local frontend="$2"
  local root_path="${case_dir}/effects-metrics.${frontend}.json"
  local effects_dir="${case_dir}/effects"
  local target_path="${effects_dir}/effects-metrics.${frontend}.json"

  if [[ ! -s "${root_path}" ]]; then
    return
  fi
  mkdir -p "${effects_dir}"
  if [[ -s "${target_path}" ]]; then
    return
  fi
  cp "${root_path}" "${target_path}"
}

write_placeholder_effects_metrics() {
  local case_dir="$1"
  local frontend="$2"
  local root_path="${case_dir}/effects-metrics.${frontend}.json"
  cat > "${root_path}" <<'JSON'
{
  "metrics": [],
  "note": "diagnostics_missing"
}
JSON
  ensure_effects_metrics_artifact "${case_dir}" "${frontend}"
}

run_typeck_debug_gate() {
  local case_dir="$1"
  local diag_path="$2"
  local typeck_dir="${case_dir}/typeck"
  local metrics_path="${typeck_dir}/typeck-debug.metrics.json"
  local err_path="${typeck_dir}/typeck-debug.metrics.err.log"

  mkdir -p "${typeck_dir}"
  if "${COLLECT_METRICS_SCRIPT}" \
      --section effects \
      --source "${diag_path}" \
      --require-success \
      > "${metrics_path}" 2> "${err_path}"
  then
    rm -f "${err_path}"
    return 0
  fi

  printf '!! typeck_debug_match gate failed for %s (see %s)\n' "${case_dir}" "${err_path}" >&2
  return 1
}

record_typeck_command() {
  local output_path="$1"
  local cwd="$2"
  shift 2
  if [[ $# -eq 0 ]]; then
    return
  fi
  mkdir -p "$(dirname "${output_path}")"
  python3 - "$output_path" "$cwd" "$@" <<'PY'
import json
import pathlib
import sys

out_path = pathlib.Path(sys.argv[1])
cwd = sys.argv[2]
argv = sys.argv[3:]
payload = {
    "cwd": cwd,
    "argv": argv,
}
out_path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
PY
}

copy_if_exists() {
  local src="$1"
  local dest="$2"
  if [[ -f "${src}" ]]; then
    mkdir -p "$(dirname "${dest}")"
    cp "${src}" "${dest}"
  fi
}

generate_type_effect_report() {
  local case_dir="$1"
  local case_name="$2"
  python3 - "${case_dir}" "${case_name}" <<'PY'
import json
import sys
from pathlib import Path

case_dir = Path(sys.argv[1])
case_name = sys.argv[2]
typeck_dir = case_dir / "typeck"
report_path = typeck_dir / "requirements.json"

def load_json(path: Path):
    if not path.exists():
        return None
    text = path.read_text(encoding="utf-8").strip()
    if not text:
        return None
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return None

def find_metric(data, metric_name: str):
    if not isinstance(data, dict):
        return None
    metrics = data.get("metrics")
    if not isinstance(metrics, list):
        return None
    for item in metrics:
        if isinstance(item, dict) and item.get("metric") == metric_name:
            return item
    return None

def effects_metric_ok(data):
    if not isinstance(data, dict):
        return False
    metrics = data.get("metrics")
    if not isinstance(metrics, list) or not metrics:
        return False
    for item in metrics:
        if not isinstance(item, dict):
            continue
        status = item.get("status")
        if status is None:
            continue
        normalized = str(status).strip().lower()
        if normalized not in ("success", "ok", "passed"):
            return False
    return True

def has_effect_context(path: Path):
    data = load_json(path)
    if not isinstance(data, dict):
        return False
    effect_ctx = data.get("effect_context")
    return isinstance(effect_ctx, dict) and bool(effect_ctx)

required = case_name.startswith(("type_", "effect_", "ffi_"))
frontends = ("ocaml", "rust")
report = {
    "required": required,
    "frontends": {},
    "ok": True,
    "failures": [],
}

if not typeck_dir.exists():
    typeck_dir.mkdir(parents=True, exist_ok=True)

if not required:
    report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    sys.exit(0)

for frontend in frontends:
    frontend_report = {}
    typeck_path = typeck_dir / f"typeck-debug.{frontend}.json"
    parser_metrics_path = case_dir / f"parser-metrics.{frontend}.json"
    effects_metrics_path = case_dir / f"effects-metrics.{frontend}.json"

    typeck_exists = typeck_path.exists() and typeck_path.stat().st_size > 0
    frontend_report["typeck_debug_path"] = str(typeck_path)
    frontend_report["typeck_debug_exists"] = typeck_exists
    frontend_report["effect_context_present"] = typeck_exists and has_effect_context(typeck_path)

    parser_metric = find_metric(load_json(parser_metrics_path), "parser.expected_summary_presence")
    frontend_report["parser_metric_present"] = parser_metric is not None
    parser_metric_status = False
    if parser_metric is not None:
        pass_rate = parser_metric.get("pass_rate")
        parser_metric_status = isinstance(pass_rate, (int, float)) and pass_rate == 1.0
    frontend_report["parser_metric_status"] = parser_metric_status

    effects_metric_data = load_json(effects_metrics_path)
    frontend_report["effects_metrics_present"] = isinstance(effects_metric_data, dict)
    frontend_report["effects_metrics_status"] = (
        effects_metric_ok(effects_metric_data) if isinstance(effects_metric_data, dict) else False
    )

    frontend_ok = (
        frontend_report["typeck_debug_exists"]
        and frontend_report["effect_context_present"]
        and frontend_report["parser_metric_present"]
        and frontend_report["parser_metric_status"]
        and frontend_report["effects_metrics_present"]
        and frontend_report["effects_metrics_status"]
    )

    frontend_report["ok"] = frontend_ok
    if not frontend_ok:
        report["ok"] = False
        report["failures"].append(
            f"{frontend}: missing artifacts (typeck_debug={frontend_report['typeck_debug_exists']}, "
            f"effect_context={frontend_report['effect_context_present']}, "
            f"parser_metric={frontend_report['parser_metric_present']}, "
            f"effects_metrics={frontend_report['effects_metrics_present']})"
        )

    report["frontends"][frontend] = frontend_report

report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")

if report["ok"]:
    sys.exit(0)
sys.exit(1)
PY
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

emit_expected_tokens_payload() {
  local diag_path="$1"
  local case_name="$2"
  local frontend="$3"
  local output_path="$4"

  mkdir -p "$(dirname "${output_path}")"
  python3 - "${diag_path}" "${case_name}" "${frontend}" "${output_path}" <<'PY' || true
import json
import pathlib
import sys

diag_path = pathlib.Path(sys.argv[1])
case_name = sys.argv[2]
frontend = sys.argv[3]
out_path = pathlib.Path(sys.argv[4])

missing = not diag_path.exists()
payload = {}
try:
    payload = json.loads(diag_path.read_text(encoding="utf-8"))
except Exception:
    payload = {}

diagnostics = payload.get("diagnostics")
result = {
    "case": case_name,
    "frontend": frontend,
    "source": str(diag_path),
    "diagnostic_count": 0,
    "diagnostics": [],
    "first_expected_tokens": [],
    "first_diagnostic_index": None,
    "total_expected_token_count": 0,
}
if missing:
    result["note"] = "diagnostics_missing"

if isinstance(diagnostics, list):
    result["diagnostic_count"] = len(diagnostics)
    first_tokens = None
    first_index = None
    entries = []
    total_tokens = 0
    for idx, diag in enumerate(diagnostics):
        tokens = []
        if isinstance(diag, dict):
            extensions = diag.get("extensions") or {}
            if isinstance(extensions, dict):
                recover = extensions.get("recover") or {}
                if isinstance(recover, dict):
                    raw_tokens = recover.get("expected_tokens")
                    if isinstance(raw_tokens, list):
                        for token in raw_tokens:
                            if isinstance(token, dict):
                                entry = {}
                                token_value = (
                                    token.get("token")
                                    or token.get("label")
                                    or token.get("value")
                                )
                                hint_value = token.get("hint") or token.get("kind")
                                if token_value is not None:
                                    entry["token"] = str(token_value)
                                if hint_value is not None:
                                    entry["hint"] = str(hint_value)
                                if entry:
                                    tokens.append(entry)
                            elif isinstance(token, str):
                                tokens.append({"token": token, "hint": "token"})
        entries.append(
            {
                "index": idx,
                "token_count": len(tokens),
                "expected_tokens": tokens,
            }
        )
        if tokens and first_tokens is None:
            first_tokens = tokens
            first_index = idx
        total_tokens += len(tokens)
    result["diagnostics"] = entries
    result["first_expected_tokens"] = first_tokens or []
    result["first_diagnostic_index"] = first_index
    result["total_expected_token_count"] = total_tokens

out_path.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
PY
}

create_expected_tokens_diff() {
  local ocaml_path="$1"
  local rust_path="$2"
  local output_path="$3"

  mkdir -p "$(dirname "${output_path}")"
  python3 - "${ocaml_path}" "${rust_path}" "${output_path}" <<'PY' || true
import json
import pathlib
import sys

ocaml_path = pathlib.Path(sys.argv[1])
rust_path = pathlib.Path(sys.argv[2])
out_path = pathlib.Path(sys.argv[3])

def load_payload(path: pathlib.Path):
    if not path.exists():
        return None
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return None

def normalized_tokens(payload):
    if not isinstance(payload, dict):
        return [], None
    tokens = payload.get("first_expected_tokens")
    if not isinstance(tokens, list):
        tokens = []
    normalized = []
    for token in tokens:
        if isinstance(token, dict):
            normalized.append(
                {
                    "token": token.get("token")
                    or token.get("label")
                    or token.get("value"),
                    "hint": token.get("hint") or token.get("kind"),
                }
            )
        elif isinstance(token, str):
            normalized.append({"token": token, "hint": "token"})
    return normalized, payload.get("first_diagnostic_index")

ocaml_payload = load_payload(ocaml_path)
rust_payload = load_payload(rust_path)
ocaml_tokens, ocaml_index = normalized_tokens(ocaml_payload)
rust_tokens, rust_index = normalized_tokens(rust_payload)

differences = []
max_len = max(len(ocaml_tokens), len(rust_tokens))
for idx in range(max_len):
    ocaml_token = ocaml_tokens[idx] if idx < len(ocaml_tokens) else None
    rust_token = rust_tokens[idx] if idx < len(rust_tokens) else None
    if ocaml_token != rust_token:
        differences.append(
            {
                "index": idx,
                "ocaml": ocaml_token,
                "rust": rust_token,
            }
        )

match = not differences and len(ocaml_tokens) == len(rust_tokens)
summary = {
    "case": (ocaml_payload or rust_payload or {}).get("case"),
    "match": match,
    "ocaml_count": len(ocaml_tokens),
    "rust_count": len(rust_tokens),
    "differences": differences,
    "ocaml_tokens": ocaml_tokens,
    "rust_tokens": rust_tokens,
    "ocaml_first_index": ocaml_index,
    "rust_first_index": rust_index,
    "ocaml_payload_present": ocaml_payload is not None,
    "rust_payload_present": rust_payload is not None,
}

out_path.write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding="utf-8")
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

create_effects_metrics_diff() {
  local ocaml_path="$1"
  local rust_path="$2"
  local output_path="$3"
  if [[ ! -s "${ocaml_path}" || ! -s "${rust_path}" ]]; then
    return
  fi
  python3 - "${ocaml_path}" "${rust_path}" "${output_path}" <<'PY' || true
import json
import pathlib
import sys

ocaml_path = pathlib.Path(sys.argv[1])
rust_path = pathlib.Path(sys.argv[2])
output_path = pathlib.Path(sys.argv[3])

def load(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return None

ocaml = load(ocaml_path)
rust = load(rust_path)

payload = {
    "match": ocaml == rust,
}

if isinstance(ocaml, dict) and isinstance(rust, dict):
    ocaml_keys = set(ocaml.keys())
    rust_keys = set(rust.keys())
    payload["ocaml_only"] = sorted(ocaml_keys - rust_keys)
    payload["rust_only"] = sorted(rust_keys - ocaml_keys)

output_path.parent.mkdir(parents=True, exist_ok=True)
output_path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
PY
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
  type_effect_case="false"
  if is_type_effect_case "${case_name}"; then
    type_effect_case="true"
    strip_case_flag "ocaml_case_flags" "--emit-typeck-debug"
    strip_case_flag "rust_case_flags" "--emit-typeck-debug"
    strip_case_flag "rust_case_flags" "--emit-effects-metrics"
    ensure_flag "ocaml_case_flags" "--experimental-effects"
    ensure_flag_with_value "ocaml_case_flags" "--type-row-mode" "dual-write"
    ensure_flag_with_value "ocaml_case_flags" "--effect-stage" "beta"
    ensure_flag_with_value "ocaml_case_flags" "--emit-typeck-debug" "${ocaml_typeck_debug_json}"
    ensure_flag "rust_case_flags" "--experimental-effects"
    ensure_flag_with_value "rust_case_flags" "--type-row-mode" "dual-write"
    ensure_flag_with_value "rust_case_flags" "--effect-stage" "beta"
    ensure_flag_with_value "rust_case_flags" "--emit-typeck-debug" "${rust_typeck_debug_json}"
    ensure_flag_with_value "rust_case_flags" "--emit-effects-metrics" "${effects_dir}/effects-metrics.rust.json"
    if [[ "${FORCE_TYPE_EFFECT_FLAGS}" == "true" ]]; then
      ensure_flag_with_value "ocaml_case_flags" "--runtime-capabilities" "${TYPE_EFFECT_RUNTIME_CAPS}"
      ensure_flag_with_value "rust_case_flags" "--runtime-capabilities" "${TYPE_EFFECT_RUNTIME_CAPS}"
      ensure_flag_with_value "ocaml_case_flags" "--emit-effects-metrics" "${effects_dir}/effects-metrics.ocaml.json"
    fi
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
    schema_ok="true"
    metrics_ok="true"
    expected_tokens_match=""
    expected_tokens_count_ocaml=""
    expected_tokens_count_rust=""

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
    if [[ "${type_effect_case}" == "true" ]]; then
      record_typeck_command "${typeck_dir}/command.ocaml.json" "${OCAML_DIR}" "${ocaml_diag_cmd[@]}"
    fi
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
    rust_stderr="${case_dir}/rust.stderr.log"
    if [[ "${type_effect_case}" == "true" ]]; then
      record_typeck_command "${typeck_dir}/command.rust.json" "${RUST_DIR}" "${rust_diag_cmd[@]}"
    fi
    run_in_dir "${RUST_DIR}" "${rust_diag_cmd[@]}" > "${rust_diag_path}" 2> "${rust_stderr}" || true

    recover_ocaml_diag_from_stderr "${ocaml_diag_path}" "${ocaml_stderr}"
    ensure_streaming_expected_tokens "${case_name}" "${ocaml_diag_path}"
    ensure_streaming_expected_tokens "${case_name}" "${rust_diag_path}"

    if ! bash "${VALIDATE_DIAG_SCRIPT}" "${ocaml_diag_path}" "${rust_diag_path}" > "${schema_log}" 2>&1; then
      schema_ok="false"
      case_gating="false"
    fi

    if [[ -s "${ocaml_diag_path}" ]]; then
      if ! collect_all_metrics "${ocaml_diag_path}" "ocaml" "${case_dir}"; then
        metrics_ok="false"
        case_gating="false"
      else
        ensure_effects_metrics_artifact "${case_dir}" "ocaml"
      fi
    else
      printf '%s\n' "-- OCaml diagnostics missing, metrics skipped (${case_name})" >&2
      write_placeholder_effects_metrics "${case_dir}" "ocaml"
      metrics_ok="false"
      case_gating="false"
    fi
    if [[ -s "${rust_diag_path}" ]]; then
      if ! collect_all_metrics "${rust_diag_path}" "rust" "${case_dir}"; then
        metrics_ok="false"
        case_gating="false"
      else
        ensure_effects_metrics_artifact "${case_dir}" "rust"
      fi
    else
      printf '%s\n' "-- Rust diagnostics missing, metrics skipped (${case_name})" >&2
      write_placeholder_effects_metrics "${case_dir}" "rust"
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

    expected_tokens_prefix_path=""
    if [[ -n "${EXPECTED_TOKENS_PREFIX}" ]]; then
      if [[ "${EXPECTED_TOKENS_PREFIX}" == /* ]]; then
        expected_tokens_prefix_path="${EXPECTED_TOKENS_PREFIX}"
      else
        expected_tokens_prefix_path="${case_dir}/${EXPECTED_TOKENS_PREFIX}"
      fi
    elif is_streaming_case "${case_name}"; then
      expected_tokens_prefix_path="${case_dir}/expected_tokens/${case_name}"
    fi
    if [[ -n "${expected_tokens_prefix_path}" ]]; then
      ocaml_expected_tokens_path="${expected_tokens_prefix_path}.ocaml.json"
      rust_expected_tokens_path="${expected_tokens_prefix_path}.rust.json"
      expected_tokens_diff_path="${expected_tokens_prefix_path}.diff.json"
      emit_expected_tokens_payload "${ocaml_diag_path}" "${case_name}" "ocaml" "${ocaml_expected_tokens_path}"
      emit_expected_tokens_payload "${rust_diag_path}" "${case_name}" "rust" "${rust_expected_tokens_path}"
      create_expected_tokens_diff "${ocaml_expected_tokens_path}" "${rust_expected_tokens_path}" "${expected_tokens_diff_path}"
      if [[ -s "${expected_tokens_diff_path}" ]]; then
        read -r expected_tokens_match expected_tokens_count_ocaml expected_tokens_count_rust <<<$(
          python3 - "${expected_tokens_diff_path}" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
try:
    data = json.loads(path.read_text(encoding="utf-8"))
except Exception:
    print("false 0 0")
    sys.exit(0)

match = "true" if data.get("match") else "false"
ocaml_count = data.get("ocaml_count") or 0
rust_count = data.get("rust_count") or 0
print(match, ocaml_count, rust_count)
PY
        )
      else
        expected_tokens_match="false"
        expected_tokens_count_ocaml="0"
        expected_tokens_count_rust="0"
      fi

      if [[ "${case_name}" == "recover_else_without_if" ]]; then
        if [[ "${expected_tokens_match}" != "true" ]]; then
          printf '!! expected-tokens gate: %s で OCaml/Rust が不一致です\n' "${case_name}" >&2
          case_gating="false"
          metrics_ok="false"
        elif [[ "${expected_tokens_count_ocaml}" != "27" || "${expected_tokens_count_rust}" != "27" ]]; then
          printf '!! expected-tokens gate: %s のトークン数が揃っていません (ocaml=%s rust=%s)\n' \
            "${case_name}" "${expected_tokens_count_ocaml}" "${expected_tokens_count_rust}" >&2
          case_gating="false"
          metrics_ok="false"
        fi
      elif is_streaming_case "${case_name}"; then
        if [[ "${expected_tokens_match}" != "true" ]]; then
          printf '!! expected-tokens gate: streaming %s で OCaml/Rust が不一致です\n' "${case_name}" >&2
          case_gating="false"
          metrics_ok="false"
        fi
      fi
    fi

    if [[ "${type_effect_case}" == "true" ]]; then
      copy_if_exists "${ocaml_stderr}" "${typeck_dir}/stderr.ocaml.log"
      copy_if_exists "${rust_stderr}" "${typeck_dir}/stderr.rust.log"
      create_effects_metrics_diff \
        "${case_dir}/effects/effects-metrics.ocaml.json" \
        "${case_dir}/effects/effects-metrics.rust.json" \
        "${case_dir}/effects/effects-metrics.diff.json"
      if ! run_typeck_debug_gate "${case_dir}" "${rust_diag_path}"; then
        case_gating="false"
        metrics_ok="false"
      fi
    fi

    if [[ "${type_effect_case}" == "true" ]]; then
      if ! generate_type_effect_report "${case_dir}" "${case_name}"; then
        case_gating="false"
        metrics_ok="false"
      fi
    fi

python3 - "${case_dir}" "${case_name}" "${safe_name}" "${RUN_ID}" "${REPORT_DIR}" "${MODE}" "${case_gating}" "${schema_ok}" "${metrics_ok}" "${case_tests}" "${case_lsp}" "${expected_tokens_match}" "${expected_tokens_count_ocaml}" "${expected_tokens_count_rust}" <<'PY'
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
expected_tokens_match = sys.argv[12]
expected_tokens_count_ocaml = sys.argv[13]
expected_tokens_count_rust = sys.argv[14]

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

def coerce_bool(value: str):
    if value is None:
        return None
    normalized = value.strip().lower()
    if normalized in ("true", "false"):
        return normalized == "true"
    return None

def coerce_int(value: str):
    try:
        return int(value)
    except (TypeError, ValueError):
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
    entries = []
    metrics = data.get("metrics")
    if isinstance(metrics, list):
        entries.extend(metrics)
    extra_metrics = data.get("extra_metrics")
    if isinstance(extra_metrics, list):
        entries.extend(extra_metrics)
    if not entries:
        return None
    summary = {}
    for item in entries:
        if not isinstance(item, dict):
            continue
        metric_name = item.get("metric")
        if metric_name == "effect_row_guard_regressions":
            summary["regressions"] = item.get("count")
        elif metric_name == "effect_scope.audit_presence":
            summary["scope"] = {
                "pass_rate": item.get("pass_rate"),
                "pass_fraction": item.get("pass_fraction"),
                "status": item.get("status"),
                "total": item.get("total"),
                "failed": item.get("failed"),
                "failures": item.get("failures"),
            }
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
match_flag = coerce_bool(expected_tokens_match)
if match_flag is not None:
    summary["expected_tokens"] = {
        "match": match_flag,
        "ocaml_count": coerce_int(expected_tokens_count_ocaml) or 0,
        "rust_count": coerce_int(expected_tokens_count_rust) or 0,
    }

summary["type_effect_case"] = case_name.startswith(("type_", "effect_", "ffi_"))
typeck_requirements = load_json(case_dir / "typeck" / "requirements.json")
if isinstance(typeck_requirements, dict):
    summary["typeck_requirements"] = typeck_requirements

    (case_dir / "summary.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
PY
    continue
  elif [[ "${MODE}" == "lexer" ]]; then
    tokens_source="${REPO_ROOT}/${value}"
    if [[ ! -f "${tokens_source}" ]]; then
      printf "!! lexer tokens source missing: %s\n" "${tokens_source}" >&2
      case_gating="false"
      metrics_ok="false"
      continue
    fi
    case_metrics_label="${CASE_METRICS_META[$idx]:-${case_name}}"
    case_metrics_label="$(
      printf '%s' "${case_metrics_label:-${case_name}}" \
        | tr '[:upper:]' '[:lower:]' \
        | tr -cs 'a-z0-9._-' '_'
    )"
    if [[ -z "${case_metrics_label}" ]]; then
      case_metrics_label="${safe_name}"
    fi
    tokens_ocaml_path="${case_dir}/tokens.ocaml.json"
    tokens_rust_path="${case_dir}/tokens.rust.json"
    tokens_diff_path="${case_dir}/tokens.diff.json"
    case_gating="true"
    if ! generate_lexer_artifacts \
      "${tokens_source}" \
      "${tokens_rust_path}" \
      "${case_dir}" \
      "${OCAML_DIR}" \
      "${RUST_DIR}" \
      "${RUN_ID}" \
      "${REPORT_DIR}" \
      "${case_metrics_label}"
    then
      case_gating="false"
      metrics_ok="false"
    fi
    compare_lexer_tokens "${tokens_ocaml_path}" "${tokens_rust_path}" "${tokens_diff_path}"

    ocaml_diag_paths=()
    if [[ -f "${case_dir}/lexer-diags/ocaml.paths" ]]; then
      mapfile -t ocaml_diag_paths < <(
        grep -v '^$' "${case_dir}/lexer-diags/ocaml.paths"
      )
    fi
    rust_diag_paths=()
    if [[ -f "${case_dir}/lexer-diags/rust.paths" ]]; then
      mapfile -t rust_diag_paths < <(
        grep -v '^$' "${case_dir}/lexer-diags/rust.paths"
      )
    fi

    metrics_ok="true"
    if ! collect_lexer_metrics \
      "${case_dir}" \
      "ocaml" \
      "${case_metrics_label}" \
      "${ocaml_diag_paths[@]}"
    then
      metrics_ok="false"
    fi
    if ! collect_lexer_metrics \
      "${case_dir}" \
      "rust" \
      "${case_metrics_label}" \
      "${rust_diag_paths[@]}"
    then
      metrics_ok="false"
    fi

    ocaml_diag_count=${#ocaml_diag_paths[@]}
    rust_diag_count=${#rust_diag_paths[@]}

    python3 - "${case_dir}" "${case_name}" "${case_tests}" "${case_lsp}" "${value}" \
      "${tokens_diff_path}" "${case_dir}/lexer-metrics.ocaml.json" \
      "${case_dir}/lexer-metrics.rust.json" "${metrics_ok}" \
      "${case_gating}" "${schema_ok}" "${ocaml_diag_count}" "${rust_diag_count}" <<'PY'
import json
import pathlib
import sys

case_dir = pathlib.Path(sys.argv[1])
case_name = sys.argv[2]
case_tests = sys.argv[3]
case_lsp = sys.argv[4]
source_ref = sys.argv[5]
tokens_diff_path = pathlib.Path(sys.argv[6])
metrics_ocaml = pathlib.Path(sys.argv[7])
metrics_rust = pathlib.Path(sys.argv[8])
metrics_ok = sys.argv[9].lower() == "true"
case_gating_flag = sys.argv[10].lower() == "true"
schema_ok_flag = sys.argv[11].lower() == "true"
ocaml_diag_count = int(sys.argv[12])
rust_diag_count = int(sys.argv[13])

def load_json(path: pathlib.Path):
    if not path.exists() or path.stat().st_size == 0:
        return None
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return None

tokens_diff = load_json(tokens_diff_path) or {}
expected_tokens = {
    "match": bool(tokens_diff.get("match")),
    "ocaml_count": tokens_diff.get("ocaml_total_tokens") or 0,
    "rust_count": tokens_diff.get("rust_total_tokens") or 0,
}
summary = {
    "case": case_name,
    "mode": "lexer",
    "source": source_ref,
    "gating": case_gating_flag,
    "schema_ok": schema_ok_flag,
    "metrics_ok": metrics_ok,
    "ocaml_diag_count": ocaml_diag_count,
    "rust_diag_count": rust_diag_count,
    "diag_match": ocaml_diag_count == rust_diag_count,
    "ast_match": expected_tokens["match"],
    "expected_tokens": expected_tokens,
    "tokens": tokens_diff,
    "lexer_metrics": {
        "ocaml": load_json(metrics_ocaml),
        "rust": load_json(metrics_rust),
    },
    "type_effect_case": False,
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
