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
  python3 - "${DIAG_FILES[@]}" <<'PY' || {
import json
import pathlib
import sys
from typing import List

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


def is_parser_diagnostic(diag: dict) -> bool:
    domain = diag.get("domain")
    if isinstance(domain, str) and domain.strip().lower() == "parser":
        return True
    code = diag.get("code")
    if isinstance(code, str) and code.startswith("parser."):
        return True
    codes = diag.get("codes")
    if isinstance(codes, list):
        for item in codes:
            if isinstance(item, str) and item.startswith("parser."):
                return True
    return False




def is_value_restriction_diagnostic(diag: dict) -> bool:
    code = diag.get("code")
    if isinstance(code, str) and code.startswith("type_inference.value_restriction_"):
        return True
    codes = diag.get("codes")
    if isinstance(codes, list):
        for item in codes:
            if isinstance(item, str) and item.startswith("type_inference.value_restriction_"):
                return True
    return False


def validate_value_restriction_fields(diag: dict) -> List[str]:
    errors: List[str] = []
    extensions = diag.get("extensions")
    if not isinstance(extensions, dict):
        errors.append("extensions(value_restriction missing)")
        return errors
    vr = extensions.get("value_restriction")
    if not isinstance(vr, dict):
        errors.append("extensions.value_restriction(not object)")
        return errors
    mode = vr.get("mode")
    status = vr.get("status")
    evidence = vr.get("evidence")
    if not isinstance(mode, str) or not mode.strip():
        errors.append("extensions.value_restriction.mode")
    if not isinstance(status, str) or not status.strip():
        errors.append("extensions.value_restriction.status")
    if not isinstance(evidence, list):
        errors.append("extensions.value_restriction.evidence")
    else:
        for idx, item in enumerate(evidence):
            if not isinstance(item, dict):
                errors.append(f"extensions.value_restriction.evidence[{idx}]")
                continue
            tag = item.get("tag")
            capability = item.get("capability")
            stage = item.get("stage")
            if not isinstance(tag, str) or not tag.strip():
                errors.append(f"extensions.value_restriction.evidence[{idx}].tag")
            if not isinstance(capability, str) or not capability.strip():
                errors.append(f"extensions.value_restriction.evidence[{idx}].capability")
            if not isinstance(stage, dict):
                errors.append(f"extensions.value_restriction.evidence[{idx}].stage")
            else:
                required = stage.get("required")
                actual = stage.get("actual")
                if not isinstance(required, str) or not required.strip():
                    errors.append(f"extensions.value_restriction.evidence[{idx}].stage.required")
                if not isinstance(actual, str) or not actual.strip():
                    errors.append(f"extensions.value_restriction.evidence[{idx}].stage.actual")
    audit_metadata = diag.get("audit_metadata")
    if not isinstance(audit_metadata, dict):
        errors.append("audit_metadata.value_restriction(mode/status missing)")
    else:
        meta_mode = audit_metadata.get("value_restriction.mode")
        meta_status = audit_metadata.get("value_restriction.status")
        if not isinstance(meta_mode, str) or not meta_mode.strip():
            errors.append("audit_metadata.value_restriction.mode")
        if not isinstance(meta_status, str) or not meta_status.strip():
            errors.append("audit_metadata.value_restriction.status")
    audit_block = diag.get("audit")
    metadata = None
    if isinstance(audit_block, dict):
        metadata = audit_block.get("metadata")
    if not isinstance(metadata, dict):
        errors.append("audit.metadata.value_restriction(mode/status missing)")
    else:
        env_mode = metadata.get("value_restriction.mode")
        env_status = metadata.get("value_restriction.status")
        if not isinstance(env_mode, str) or not env_mode.strip():
            errors.append("audit.metadata.value_restriction.mode")
        if not isinstance(env_status, str) or not env_status.strip():
            errors.append("audit.metadata.value_restriction.status")
    return errors

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
        if isinstance(entry, dict):
            diagnostics = entry.get("diagnostics")
            if isinstance(diagnostics, list):
                for diag_index, diag in enumerate(diagnostics):
                    if not isinstance(diag, dict):
                        continue
                    if is_parser_diagnostic(diag):
                        expected = diag.get("expected")
                        if not isinstance(expected, dict):
                            print(
                                "[validate-diagnostic-json] parser expected summary missing: "
                                f"{path}: diagnostics[{diag_index}].expected",
                                file=sys.stderr,
                            )
                            error = True
                            continue
                        alternatives = expected.get("alternatives")
                        if not isinstance(alternatives, list) or len(alternatives) == 0:
                            print(
                                "[validate-diagnostic-json] parser expected summary empty alternatives: "
                                f"{path}: diagnostics[{diag_index}].expected.alternatives",
                                file=sys.stderr,
                            )
                            error = True
                    if is_value_restriction_diagnostic(diag):
                        vr_missing = validate_value_restriction_fields(diag)
                        if vr_missing:
                            for field in vr_missing:
                                print(
                                    "[validate-diagnostic-json] value_restriction field missing: "
                                    f"{path}: diagnostics[{diag_index}].{field}",
                                    file=sys.stderr,
                                )
                            error = True
                    extensions = diag.get("extensions")
                    if isinstance(extensions, dict):
                        effects = extensions.get("effects")
                        if isinstance(effects, dict):
                            required = effects.get("required_capabilities")
                            if not isinstance(required, list):
                                print(
                                    "[validate-diagnostic-json] effects.required_capabilities missing or invalid: "
                                    f"{path}: diagnostics[{diag_index}].extensions.effects.required_capabilities",
                                    file=sys.stderr,
                                )
                                error = True
                            actual = effects.get("actual_capabilities")
                            if not isinstance(actual, list):
                                print(
                                    "[validate-diagnostic-json] effects.actual_capabilities missing or invalid: "
                                    f"{path}: diagnostics[{diag_index}].extensions.effects.actual_capabilities",
                                    file=sys.stderr,
                                )
                                error = True
                    audit_metadata = diag.get("audit_metadata")
                    if isinstance(audit_metadata, dict):
                        required = audit_metadata.get("effect.required_capabilities")
                        actual = audit_metadata.get("effect.actual_capabilities")
                        if not isinstance(required, list):
                            print(
                                "[validate-diagnostic-json] audit_metadata effect.required_capabilities missing or invalid: "
                                f"{path}: diagnostics[{diag_index}].audit_metadata",
                                file=sys.stderr,
                            )
                            error = True
                        if not isinstance(actual, list):
                            print(
                                "[validate-diagnostic-json] audit_metadata effect.actual_capabilities missing or invalid: "
                                f"{path}: diagnostics[{diag_index}].audit_metadata",
                                file=sys.stderr,
                            )
                            error = True
                    audit_block = diag.get("audit")
                    if isinstance(audit_block, dict):
                        metadata = audit_block.get("metadata")
                        if isinstance(metadata, dict):
                            required = metadata.get("effect.required_capabilities")
                            actual = metadata.get("effect.actual_capabilities")
                            if not isinstance(required, list):
                                print(
                                    "[validate-diagnostic-json] audit.metadata effect.required_capabilities missing or invalid: "
                                    f"{path}: diagnostics[{diag_index}].audit.metadata",
                                    file=sys.stderr,
                                )
                                error = True
                            if not isinstance(actual, list):
                                print(
                                    "[validate-diagnostic-json] audit.metadata effect.actual_capabilities missing or invalid: "
                                    f"{path}: diagnostics[{diag_index}].audit.metadata",
                                    file=sys.stderr,
                                )
                                error = True

if error:
    sys.exit(1)
PY
    EXIT_CODE=1
  }
fi

exit $EXIT_CODE
