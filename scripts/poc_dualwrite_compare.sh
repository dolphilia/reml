#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
OCAML_DIR="${REPO_ROOT}/compiler/ocaml"
RUST_DIR="${REPO_ROOT}/compiler/rust/frontend"
REPORT_DIR="${REPO_ROOT}/reports/dual-write/front-end/poc"
RUN_ID="2025-11-28-logos-chumsky"
CASE_DIR="${REPORT_DIR}/${RUN_ID}"

mkdir -p "${CASE_DIR}"
: "${CARGO_HOME:=${HOME}/.cargo}"
: "${CARGO_TARGET_DIR:=${RUST_DIR}/target}"
export CARGO_HOME CARGO_TARGET_DIR
mkdir -p "${CARGO_HOME}" "${CARGO_TARGET_DIR}"

declare -a CASES=(
  "empty_uses::fn answer() = 42"
  "multiple_functions::fn log(x) = x\nfn log_twice(x) = log(log(x))"
  "addition::fn add(x, y) = x + y"
  "missing_paren::fn missing(x = x"
)

printf "==> 出力ディレクトリ: %s\n" "${CASE_DIR}"

for entry in "${CASES[@]}"; do
  name="${entry%%::*}"
  payload="${entry#*::}"
  input_path="${CASE_DIR}/${name}.reml"

  printf ">> ケース %s\n" "${name}"
  printf "%b\n" "${payload}" > "${input_path}"

  (cd "${OCAML_DIR}" && dune exec remlc -- --emit-ast "${input_path}") \
    > "${CASE_DIR}/${name}.ocaml.ast.txt" 2>/dev/null || true
  (cd "${OCAML_DIR}" && dune exec remlc -- --format json --json-mode compact "${input_path}") \
    > "${CASE_DIR}/${name}.ocaml.diagnostics.json" 2>&1 || true
  (cd "${RUST_DIR}" && cargo run --quiet --bin poc_frontend -- "${input_path}") \
    > "${CASE_DIR}/${name}.rust.json"

  python3 - "${CASE_DIR}" "${name}" <<'PY'
import json
import pathlib
import sys

case_dir = pathlib.Path(sys.argv[1])
name = sys.argv[2]

def read_text(path: pathlib.Path) -> str:
    if not path.exists():
        return ""
    return path.read_text(encoding="utf-8").strip()

ocaml_ast = read_text(case_dir / f"{name}.ocaml.ast.txt")
rust_data = json.loads(read_text(case_dir / f"{name}.rust.json") or "{}")
rust_ast = (rust_data.get("ast_render") or "").strip()

ocaml_diag_raw = read_text(case_dir / f"{name}.ocaml.diagnostics.json")
if ocaml_diag_raw:
    try:
        ocaml_diag_json = json.loads(ocaml_diag_raw)
        ocaml_diag_list = ocaml_diag_json.get("diagnostics", [])
    except json.JSONDecodeError:
        ocaml_diag_list = []
else:
    ocaml_diag_list = []

rust_diag_list = rust_data.get("diagnostics", [])

summary = {
    "case": name,
    "ast_match": ocaml_ast == rust_ast,
    "ocaml_ast": ocaml_ast,
    "rust_ast": rust_ast,
    "ocaml_diag_count": len(ocaml_diag_list),
    "rust_diag_count": len(rust_diag_list),
    "diag_match": len(ocaml_diag_list) == len(rust_diag_list),
    "ocaml_diag_messages": [diag.get("message") for diag in ocaml_diag_list],
    "rust_diag_messages": [diag.get("message") for diag in rust_diag_list],
}

(case_dir / f"{name}.summary.json").write_text(
    json.dumps(summary, ensure_ascii=False, indent=2), encoding="utf-8"
)
PY
done

python3 - "${CASE_DIR}" <<'PY'
import json
import pathlib
import sys

case_dir = pathlib.Path(sys.argv[1])
summaries = []
for path in sorted(case_dir.glob("*.summary.json")):
    summaries.append(json.loads(path.read_text(encoding="utf-8")))

lines = [
    "| case | ast_match | diag_match | ocaml_diag | rust_diag |",
    "| --- | --- | --- | --- | --- |",
]
for summary in summaries:
    lines.append(
        f"| {summary['case']} | {summary['ast_match']} | {summary['diag_match']} | "
        f"{summary['ocaml_diag_count']} | {summary['rust_diag_count']} |"
    )

(case_dir / "summary.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

printf "==> サマリ: %s\n" "${CASE_DIR}/summary.md"
