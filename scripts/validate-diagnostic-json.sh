#!/usr/bin/env bash
#
# validate-diagnostic-json.sh — Diagnostic JSON Schema バリデーション補助スクリプト
#
# 使用例:
#   ./scripts/validate-diagnostic-json.sh \
#     compiler/ocaml/tests/golden/diagnostics \
#     tmp/cli-output.json
#
# 依存: Node.js, tooling/lsp/tests/client_compat 内の `npm install`

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIAG_SCHEMA_PATH="$ROOT_DIR/tooling/json-schema/diagnostic-v2.schema.json"
AUDIT_SCHEMA_PATH="$ROOT_DIR/tooling/runtime/audit-schema.json"
NODE_PROJECT="$ROOT_DIR/tooling/lsp/tests/client_compat"

print_usage() {
  cat <<'EOF'
Usage: scripts/validate-diagnostic-json.sh [PATH...]

引数を省略した場合は以下のディレクトリを既定で検証します:
  - compiler/ocaml/tests/golden/diagnostics
  - compiler/ocaml/tests/golden/audit

PATH には JSON ファイルまたはディレクトリを指定できます。
EOF
}

if [[ "${1:-}" == "--help" ]]; then
  print_usage
  exit 0
fi

if ! command -v node >/dev/null 2>&1; then
  echo "[validate-diagnostic-json] error: Node.js が見つかりません" >&2
  exit 1
fi

if [[ ! -d "$NODE_PROJECT/node_modules" ]]; then
  echo "[validate-diagnostic-json] warning: $NODE_PROJECT/node_modules が存在しません。" >&2
  echo "  → 先に \`npm install\` (tooling/lsp/tests/client_compat) を実行してください。" >&2
  exit 1
fi

declare -a TARGETS=()
if [[ "$#" -eq 0 ]]; then
  TARGETS+=("$ROOT_DIR/compiler/ocaml/tests/golden/diagnostics")
  TARGETS+=("$ROOT_DIR/compiler/ocaml/tests/golden/audit")
else
  for arg in "$@"; do
    TARGETS+=("$arg")
  done
fi

expand_targets() {
  local target="$1"
  if [[ -d "$target" ]]; then
    find "$target" -type f \( -name "*.json" -o -name "*.jsonl" -o -name "*.jsonl.golden" -o -name "*.json.golden" \) | sort
  else
    echo "$target"
  fi
}

declare -a FILES=()
for target in "${TARGETS[@]}"; do
  while IFS= read -r path; do
    [[ -n "$path" ]] && FILES+=("$path")
  done < <(expand_targets "$target")
done

if [[ "${#FILES[@]}" -eq 0 ]]; then
  echo "[validate-diagnostic-json] warning: 対象ファイルが見つかりません" >&2
  exit 0
fi

declare -a DIAG_FILES=()
declare -a AUDIT_FILES=()

for path in "${FILES[@]}"; do
  normalized="${path//\\//}"
  if [[ "$normalized" == *"/audit/"* ]]; then
    AUDIT_FILES+=("$path")
  else
    DIAG_FILES+=("$path")
  fi
done

EXIT_CODE=0

if [[ "${#DIAG_FILES[@]}" -gt 0 ]]; then
  if ! node "$NODE_PROJECT/validate-diagnostic-json.mjs" "$DIAG_SCHEMA_PATH" "${DIAG_FILES[@]}"; then
    EXIT_CODE=1
  fi
fi

if [[ "${#AUDIT_FILES[@]}" -gt 0 ]]; then
  if [[ ! -f "$AUDIT_SCHEMA_PATH" ]]; then
    echo "[validate-diagnostic-json] warning: audit schema not found ($AUDIT_SCHEMA_PATH)。検証をスキップします。" >&2
  else
    if ! node "$NODE_PROJECT/validate-diagnostic-json.mjs" "$AUDIT_SCHEMA_PATH" "${AUDIT_FILES[@]}"; then
      EXIT_CODE=1
    fi
  fi
fi

if [[ "${#DIAG_FILES[@]}" -gt 0 ]]; then
  if ! python3 - "${DIAG_FILES[@]}" <<'PY'; then
import json
import pathlib
import sys

files = sys.argv[1:]
error = False


def parse_entries(content: str, file_name: str):
    text = content.strip()
    if not text:
        return []
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        entries = []
        for line_no, line in enumerate(content.splitlines(), start=1):
            stripped = line.strip()
            if not stripped:
                continue
            try:
                entries.append(json.loads(stripped))
            except json.JSONDecodeError as exc:
                raise RuntimeError(f"JSONL parse error {file_name}:{line_no}: {exc}") from exc
        return entries
    else:
        return data if isinstance(data, list) else [data]


class MissingRecovered(Exception):
    pass


def walk(node, location="root"):
    if isinstance(node, dict):
        if "parse_result" in node:
            pr = node["parse_result"]
            if not isinstance(pr, dict):
                raise MissingRecovered(f"{location}.parse_result はオブジェクトである必要があります")
            if "recovered" not in pr:
                raise MissingRecovered(f"{location}.parse_result.recovered が欠落しています")
            if not isinstance(pr["recovered"], bool):
                raise MissingRecovered(f"{location}.parse_result.recovered は boolean である必要があります")
        for key, value in node.items():
            walk(value, f"{location}.{key}")
    elif isinstance(node, list):
        for index, value in enumerate(node):
            walk(value, f"{location}[{index}]")


for path_str in files:
    path = pathlib.Path(path_str)
    if not path.exists():
        continue
    try:
        content = path.read_text(encoding="utf-8")
        entries = parse_entries(content, str(path))
    except Exception as exc:  # noqa: BLE001
        print(f"[validate-diagnostic-json] {exc}", file=sys.stderr)
        error = True
        continue

    for entry_index, entry in enumerate(entries):
        try:
            walk(entry, f"entry[{entry_index}]")
        except MissingRecovered as exc:
            print(
                f"[validate-diagnostic-json] parse_result.recovered check failed: {path}: {exc}",
                file=sys.stderr,
            )
            error = True
            break

if error:
    sys.exit(1)
PY
  then
    EXIT_CODE=1
  fi
fi

exit $EXIT_CODE
