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
SCHEMA_PATH="$ROOT_DIR/tooling/json-schema/diagnostic-v2.schema.json"
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

node "$NODE_PROJECT/validate-diagnostic-json.mjs" "$SCHEMA_PATH" "${FILES[@]}"
