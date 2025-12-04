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

--suite collectors を指定した場合は以下を検証します:
  - reports/spec-audit/ch1/core_iter_collectors.json
  - reports/spec-audit/ch1/core_iter_collectors.audit.jsonl
--suite numeric を指定した場合は以下を検証します:
  - tests/data/numeric 配下の JSON/JSONL
  - tests/expected/numeric_*.json
--suite numeric_time を指定した場合は以下を検証します:
  - tests/expected/time_{now,sleep}.json
  - compiler/rust/runtime/tests/golden/numeric_time 配下の JSON/JSONL
--suite core_io を指定した場合は以下を検証します:
  - compiler/rust/runtime/tests/data/core_io
  - compiler/rust/runtime/tests/golden/core_io
  - tests/data/core_path
--suite audit を指定した場合は以下を検証します:
  - reports/audit/privacy 配下の JSON/JSONL
--section config を指定した場合は `schema_diff.*` キーの存在をチェックします。

PATH には JSON ファイルまたはディレクトリを指定できます。
--pattern, --effect-tag は複数指定できます。`--effect-tag trace` のように指定すると
`effects.*` に `trace` を含む診断のみを Python 検証対象とし、該当診断が無かったファイルは
info ログ付きでスキップします。
--require-privacy を指定すると、対象ファイルに `privacy.*` キーを含む監査/診断エントリが存在するか確認し、
欠落時にはエラーとします。
EOF
}

SUITE=""
GENERIC_JSON_SUITE=0
SECTION=""
REQUIRE_PRIVACY=0
declare -a PATTERNS=()
declare -a TARGET_ARGS=()
declare -a EFFECT_TAGS=()

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --suite)
      shift
      if [[ "$#" -eq 0 ]]; then
        echo "[validate-diagnostic-json] error: --suite オプションには値が必要です" >&2
        exit 1
      fi
      SUITE="$1"
      shift
      ;;
    --section)
      shift
      if [[ "$#" -eq 0 ]]; then
        echo "[validate-diagnostic-json] error: --section オプションには値が必要です" >&2
        exit 1
      fi
      SECTION="$1"
      shift
      ;;
    --pattern)
      shift
      if [[ "$#" -eq 0 ]]; then
        echo "[validate-diagnostic-json] error: --pattern オプションには値が必要です" >&2
        exit 1
      fi
      PATTERNS+=("$1")
      shift
      ;;
    --effect-tag)
      shift
      if [[ "$#" -eq 0 ]]; then
        echo "[validate-diagnostic-json] error: --effect-tag オプションには値が必要です" >&2
        exit 1
      fi
      EFFECT_TAGS+=("$1")
      shift
      ;;
    --require-privacy)
      REQUIRE_PRIVACY=1
      shift
      ;;
    --help|-h)
      print_usage
      exit 0
      ;;
    --)
      shift
      while [[ "$#" -gt 0 ]]; do
        TARGET_ARGS+=("$1")
        shift
      done
      break
      ;;
    *)
      TARGET_ARGS+=("$1")
      shift
      ;;
  esac
done

if [[ "$SUITE" != "numeric_time" && "$SUITE" != "core_io" ]]; then
  if ! command -v node >/dev/null 2>&1; then
    echo "[validate-diagnostic-json] error: Node.js が見つかりません" >&2
    exit 1
  fi

  if [[ ! -d "$NODE_PROJECT/node_modules" ]]; then
    echo "[validate-diagnostic-json] warning: $NODE_PROJECT/node_modules が存在しません。" >&2
    echo "  → 先に \`npm install\` (tooling/lsp/tests/client_compat) を実行してください。" >&2
    exit 1
  fi
fi

if [[ "$SUITE" == "numeric_time" || "$SUITE" == "core_io" ]] ; then
  GENERIC_JSON_SUITE=1
fi

declare -a TARGETS=()
if [[ "${#TARGET_ARGS[@]}" -eq 0 ]]; then
  if [[ "$SUITE" == "streaming" ]]; then
    TARGETS+=("$ROOT_DIR/compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden")
  elif [[ "$SUITE" == "collectors" ]]; then
    TARGETS+=("$ROOT_DIR/reports/spec-audit/ch1/core_iter_collectors.json")
    TARGETS+=("$ROOT_DIR/reports/spec-audit/ch1/core_iter_collectors.audit.jsonl")
  elif [[ "$SUITE" == "numeric" ]]; then
    TARGETS+=("$ROOT_DIR/tests/data/numeric")
    TARGETS+=("$ROOT_DIR/tests/expected/numeric_quantiles.json")
    TARGETS+=("$ROOT_DIR/tests/expected/numeric_regression.json")
  elif [[ "$SUITE" == "numeric_time" ]]; then
    GENERIC_JSON_SUITE=1
    TARGETS+=("$ROOT_DIR/tests/expected/time_now.json")
    TARGETS+=("$ROOT_DIR/tests/expected/time_sleep.json")
    TARGETS+=("$ROOT_DIR/compiler/rust/runtime/tests/golden/numeric_time")
  elif [[ "$SUITE" == "core_io" ]]; then
    GENERIC_JSON_SUITE=1
    TARGETS+=("$ROOT_DIR/compiler/rust/runtime/tests/data/core_io")
    TARGETS+=("$ROOT_DIR/compiler/rust/runtime/tests/golden/core_io")
    TARGETS+=("$ROOT_DIR/tests/data/core_path")
  elif [[ "$SUITE" == "audit" ]]; then
    TARGETS+=("$ROOT_DIR/reports/audit/privacy")
  elif [[ "$SECTION" == "config" ]]; then
    TARGETS+=("$ROOT_DIR/reports/spec-audit/ch3/config_diagnostics-20251203.json")
  else
    TARGETS+=("$ROOT_DIR/compiler/ocaml/tests/golden/diagnostics")
    TARGETS+=("$ROOT_DIR/compiler/ocaml/tests/golden/audit")
  fi
else
  for arg in "${TARGET_ARGS[@]}"; do
    TARGETS+=("$arg")
  done
fi

if [[ "${#TARGET_ARGS[@]}" -eq 0 && "${#PATTERNS[@]}" -gt 0 ]]; then
  for raw_pattern in "${PATTERNS[@]}"; do
    pattern_lower="$(printf '%s' "$raw_pattern" | tr '[:upper:]' '[:lower:]')"
    if [[ "$pattern_lower" == *"text.grapheme_stats"* ]]; then
      TARGETS+=("$ROOT_DIR/reports/spec-audit/ch1/text_grapheme_stats.audit.jsonl")
    elif [[ "$pattern_lower" == *"numeric.histogram"* ]]; then
      TARGETS+=("$ROOT_DIR/tests/data/numeric/histogram")
    elif [[ "$pattern_lower" == *"metrics.emit"* ]]; then
      TARGETS+=("$ROOT_DIR/reports/audit/metric_point")
      TARGETS+=("$ROOT_DIR/tests/data/metrics")
    fi
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

matches_patterns() {
  if [[ "${#PATTERNS[@]}" -eq 0 ]]; then
    return 0
  fi
  local path_lower
  path_lower="$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')"
  for raw in "${PATTERNS[@]}"; do
    local pattern_lower
    pattern_lower="$(printf '%s' "$raw" | tr '[:upper:]' '[:lower:]')"
    local normalized_pattern="${pattern_lower//./\\/}"
    normalized_pattern="${normalized_pattern//\\/}"
    if [[ "$path_lower" == *"$pattern_lower"* || "$path_lower" == *"$normalized_pattern"* ]]; then
      return 0
    fi
  done
  return 1
}

declare -a FILES=()
for target in "${TARGETS[@]}"; do
  while IFS= read -r path; do
    if [[ -n "$path" ]]; then
      if matches_patterns "$path"; then
        FILES+=("$path")
      fi
    fi
  done < <(expand_targets "$target")
done

validate_generic_json_files() {
  if [[ "$#" -eq 0 ]]; then
    echo "[validate-diagnostic-json] warning: 対象ファイルが見つかりません" >&2
    return 0
  fi
  python3 - "$@" <<'PY'
import json
import pathlib
import sys

def validate_json(path: pathlib.Path) -> bool:
    name = path.name.lower()
    if name.endswith(".jsonl") or name.endswith(".jsonl.golden"):
        ok = True
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if not line.strip():
                continue
            try:
                json.loads(line)
            except json.JSONDecodeError as exc:
                print(f"[validate-diagnostic-json] error: {path}:{line_no}: {exc}", file=sys.stderr)
                ok = False
        return ok
    try:
        json.loads(path.read_text(encoding="utf-8"))
        return True
    except json.JSONDecodeError as exc:
        print(f"[validate-diagnostic-json] error: {path}: {exc}", file=sys.stderr)
        return False

all_ok = True
for raw in sys.argv[1:]:
    path = pathlib.Path(raw)
    if not path.exists():
        print(f"[validate-diagnostic-json] warning: {path} が存在しません", file=sys.stderr)
        continue
    all_ok &= validate_json(path)
sys.exit(0 if all_ok else 1)
PY
}

if [[ "$GENERIC_JSON_SUITE" -eq 1 ]]; then
  if validate_generic_json_files "${FILES[@]}"; then
    exit 0
  else
    exit 1
  fi
fi

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

if [[ "${#DIAG_FILES[@]}" -gt 0 ]]; then
  KEEP_TMP="$(mktemp)"
  SKIP_TMP="$(mktemp)"
  if ! python3 - "$KEEP_TMP" "$SKIP_TMP" "${DIAG_FILES[@]}" <<'PY'
import json
import pathlib
import sys

keep_path = pathlib.Path(sys.argv[1])
skip_path = pathlib.Path(sys.argv[2])
files = sys.argv[3:]

keep: list[str] = []
skipped: list[str] = []

for raw in files:
    path = pathlib.Path(raw)
    try:
        text = path.read_text(encoding="utf-8")
    except Exception:
        keep.append(str(path))
        continue
    text = text.strip()
    if not text:
        keep.append(str(path))
        continue
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        keep.append(str(path))
        continue
    if isinstance(data, dict) and "diagnostics" in data:
        keep.append(str(path))
    else:
        skipped.append(str(path))

keep_path.write_text("\n".join(keep), encoding="utf-8")
skip_path.write_text("\n".join(skipped), encoding="utf-8")
PY
  then
    rm -f "$KEEP_TMP" "$SKIP_TMP"
    exit 1
  fi
  DIAG_FILES=()
  if [[ -s "$KEEP_TMP" ]]; then
    while IFS= read -r line; do
      [[ -n "$line" ]] && DIAG_FILES+=("$line")
    done < "$KEEP_TMP"
  fi
  if [[ -s "$SKIP_TMP" ]]; then
    while IFS= read -r line; do
      [[ -n "$line" ]] && echo "[validate-diagnostic-json] info: skip non-diagnostic file: $line" >&2
    done < "$SKIP_TMP"
  fi
  rm -f "$KEEP_TMP" "$SKIP_TMP"
fi

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
  declare -a PY_EFFECT_ARGS=()
  if [[ "${#EFFECT_TAGS[@]}" -gt 0 ]]; then
    for tag in "${EFFECT_TAGS[@]}"; do
      PY_EFFECT_ARGS+=("--effect-tag")
      PY_EFFECT_ARGS+=("$tag")
    done
  fi
  PY_EFFECT_ARGS+=("--")
  PY_EFFECT_ARGS+=("${DIAG_FILES[@]}")
  python3 - "${PY_EFFECT_ARGS[@]}" <<'PY' || {
import json
import pathlib
import sys
from typing import List, Optional, Sequence

raw_args = sys.argv[1:]
effect_tags: list[str] = []
files: list[str] = []
arg_iter = iter(raw_args)
for token in arg_iter:
    if token == "--effect-tag":
        try:
            effect_tags.append(next(arg_iter).strip().lower())
        except StopIteration as exc:  # noqa: PERF203
            raise RuntimeError("--effect-tag needs an argument") from exc
    elif token == "--":
        files.extend(list(arg_iter))
        break
    else:
        files.append(token)

error = False
effect_tags = [tag for tag in effect_tags if tag]
effect_tag_set = set(effect_tags)


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


def value_contains_tag(value, tag_set: set[str]) -> bool:
    if not tag_set:
        return True
    if isinstance(value, str):
        lowered = value.strip().lower()
        if not lowered:
            return False
        if lowered in tag_set:
            return True
        if "." in lowered:
            suffix = lowered.split(".")[-1]
            if suffix in tag_set:
                return True
        return False
    if isinstance(value, list):
        return any(value_contains_tag(item, tag_set) for item in value)
    if isinstance(value, dict):
        return any(value_contains_tag(item, tag_set) for item in value.values())
    return False


def diag_has_effect_tag(diag: dict, tag_set: set[str]) -> bool:
    if not tag_set:
        return True

    def fetch_related(source: Optional[dict]):
        if not isinstance(source, dict):
            return []
        related = []
        for key, value in source.items():
            if not isinstance(key, str):
                continue
            lowered = key.strip().lower()
            if lowered.startswith("effect") or lowered.startswith("effects"):
                related.append(value)
        return related

    candidates: list[object] = []
    extensions = diag.get("extensions")
    if isinstance(extensions, dict):
        effects_entry = extensions.get("effects")
        if isinstance(effects_entry, dict):
            candidates.append(effects_entry)
        candidates.extend(fetch_related(extensions))
    audit_metadata = diag.get("audit_metadata")
    candidates.extend(fetch_related(audit_metadata))
    audit_block = diag.get("audit")
    if isinstance(audit_block, dict):
        metadata = audit_block.get("metadata")
        candidates.extend(fetch_related(metadata))
    return any(value_contains_tag(candidate, tag_set) for candidate in candidates)


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


def _is_nonempty_string(value: object) -> bool:
    return isinstance(value, str) and value.strip() != ""


def _ensure_metadata_strings(container: Optional[dict], prefix: str, keys: Sequence[str]) -> List[str]:
    if not isinstance(container, dict):
        return [f"{prefix} (not object)"]
    missing: List[str] = []
    for key in keys:
        value = container.get(key)
        if not _is_nonempty_string(value):
            missing.append(f"{prefix}.{key}")
    return missing


def validate_core_parser_fields(diag: dict) -> List[str]:
    if not is_parser_diagnostic(diag):
        return []

    errors: List[str] = []
    extensions = diag.get("extensions")
    if not isinstance(extensions, dict):
        errors.append("extensions (not object)")
        return errors

    parse_ext = extensions.get("parse")
    if not isinstance(parse_ext, dict):
        errors.append("extensions.parse (not object)")
    else:
        parser_id = parse_ext.get("parser_id")
        if not isinstance(parser_id, dict):
            errors.append("extensions.parse.parser_id (not object)")
        else:
            for key in ("namespace", "name", "origin", "fingerprint"):
                value = parser_id.get(key)
                if not _is_nonempty_string(value):
                    errors.append(f"extensions.parse.parser_id.{key}")
            ordinal = parser_id.get("ordinal")
            if not isinstance(ordinal, int):
                errors.append("extensions.parse.parser_id.ordinal")

    audit_metadata = diag.get("audit_metadata")
    if not isinstance(audit_metadata, dict):
        errors.append("audit_metadata (not object)")
    else:
        parser_meta = audit_metadata.get("parser.core.rule")
        if isinstance(parser_meta, dict):
            errors.extend(
                _ensure_metadata_strings(
                    parser_meta,
                    "audit_metadata.parser.core.rule",
                    ("namespace", "name", "origin", "fingerprint"),
                )
            )
            ordinal_meta = parser_meta.get("ordinal")
            if not isinstance(ordinal_meta, int):
                errors.append("audit_metadata.parser.core.rule.ordinal")
        else:
            errors.append("audit_metadata.parser.core.rule (not object)")

    audit_block = diag.get("audit")
    if not isinstance(audit_block, dict):
        errors.append("audit (not object)")
    else:
        metadata = audit_block.get("metadata")
        if not isinstance(metadata, dict):
            errors.append("audit.metadata (not object)")
        else:
            parser_meta = metadata.get("parser.core.rule")
            if isinstance(parser_meta, dict):
                errors.extend(
                    _ensure_metadata_strings(
                        parser_meta,
                        "audit.metadata.parser.core.rule",
                        ("namespace", "name", "origin", "fingerprint"),
                    )
                )
                ordinal_meta = parser_meta.get("ordinal")
                if not isinstance(ordinal_meta, int):
                    errors.append("audit.metadata.parser.core.rule.ordinal")
            else:
                errors.append("audit.metadata.parser.core.rule (not object)")

    return errors

def validate_collections_diff_extensions(diag: dict) -> List[str]:
    errors: List[str] = []
    audit_block = diag.get("audit")
    if not isinstance(audit_block, dict):
        return errors
    change_set = audit_block.get("change_set")
    if not isinstance(change_set, dict):
        return errors
    collections = change_set.get("collections")
    if not isinstance(collections, dict):
        return errors
    extensions = diag.get("extensions")
    if not isinstance(extensions, dict):
        errors.append("collections.diff")
        return errors

    def expect(key: str, expected: Optional[object]) -> None:
        if expected is None:
            return
        actual = extensions.get(key)
        if actual != expected:
            errors.append(f"{key}.mismatch")

    expect("collections.diff.kind", collections.get("kind"))
    expect("collections.diff.total", change_set.get("total"))
    summary = collections.get("summary")
    if isinstance(summary, dict):
        expect("collections.diff.summary.total", summary.get("total"))
    metadata = collections.get("metadata")
    if isinstance(metadata, dict):
        expect("collections.diff.metadata.stage", metadata.get("stage"))
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

    file_contains_relevant_diag = False
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
                    if effect_tag_set and not diag_has_effect_tag(diag, effect_tag_set):
                        continue
                    file_contains_relevant_diag = True
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
                            continue
                        core_missing = validate_core_parser_fields(diag)
                        if core_missing:
                            for field in core_missing:
                                print(
                                    "[validate-diagnostic-json] parser core metadata missing: "
                                    f"{path}: diagnostics[{diag_index}].{field}",
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
                    coll_ext_errors = validate_collections_diff_extensions(diag)
                    if coll_ext_errors:
                        for field in coll_ext_errors:
                            print(
                                "[validate-diagnostic-json] collections diff extension missing or mismatched: "
                                f"{path}: diagnostics[{diag_index}].extensions.{field}",
                                file=sys.stderr,
                            )
                        error = True
        stream_meta = entry.get("stream_meta")
        if stream_meta is not None:
            if not isinstance(stream_meta, dict):
                print(
                    f"[validate-diagnostic-json] stream_meta must be an object: {path}",
                    file=sys.stderr,
                )
                error = True
            else:
                for key in ("bytes_consumed", "chunks_consumed", "await_count", "resume_count"):
                    value = stream_meta.get(key)
                    if not isinstance(value, int):
                        print(
                            "[validate-diagnostic-json] stream_meta field missing or invalid: "
                            f"{path}: stream_meta.{key}",
                            file=sys.stderr,
                        )
                        error = True
                last_reason = stream_meta.get("last_reason")
                if last_reason is not None and not isinstance(last_reason, str):
                    print(
                        f"[validate-diagnostic-json] stream_meta.last_reason must be string when present: {path}",
                        file=sys.stderr,
                    )
                    error = True

    if effect_tag_set and not file_contains_relevant_diag:
        print(
            "[validate-diagnostic-json] info: no diagnostics with requested effect tags "
            f"{sorted(effect_tag_set)} in {path}",
            file=sys.stderr,
        )

if error:
    sys.exit(1)
PY
    EXIT_CODE=1
  }
  if [[ "$SUITE" == "streaming" ]]; then
    declare -a PY_STREAM_ARGS=()
    if [[ "${#EFFECT_TAGS[@]}" -gt 0 ]]; then
      for tag in "${EFFECT_TAGS[@]}"; do
        PY_STREAM_ARGS+=("--effect-tag")
        PY_STREAM_ARGS+=("$tag")
      done
    fi
    PY_STREAM_ARGS+=("--")
    PY_STREAM_ARGS+=("${DIAG_FILES[@]}")
    python3 - "${PY_STREAM_ARGS[@]}" <<'PY' || {
import json
import pathlib
import sys

raw_args = sys.argv[1:]
effect_tags: list[str] = []
files: list[str] = []
arg_iter = iter(raw_args)
for token in arg_iter:
    if token == "--effect-tag":
        try:
            effect_tags.append(next(arg_iter).strip().lower())
        except StopIteration as exc:  # noqa: PERF203
            raise RuntimeError("--effect-tag needs an argument") from exc
    elif token == "--":
        files.extend(list(arg_iter))
        break
    else:
        files.append(token)

error = False
effect_tags = [tag for tag in effect_tags if tag]
effect_tag_set = set(effect_tags)

required_pending_keys = [
    "parser.stream.pending.resume_hint",
    "parser.stream.pending.last_reason",
    "parser.stream.pending.expected_tokens",
    "parser.stream.pending.last_checkpoint",
]

required_error_keys = [
    "parser.stream.error.resume_hint",
    "parser.stream.error.last_reason",
    "parser.stream.error.expected_tokens",
    "parser.stream.error.last_checkpoint",
    "parser.stream.error.diagnostic",
]


def as_dict(value):
    return value if isinstance(value, dict) else None


def ensure_resume_hint(meta: dict, key: str, path: str) -> bool:
    value = meta.get(key)
    hint = as_dict(value)
    if hint is None:
        print(
            f"[validate-diagnostic-json] {path}: metadata.{key} が不足しています",
            file=sys.stderr,
        )
        return False
    min_bytes = hint.get("min_bytes")
    preferred_bytes = hint.get("preferred_bytes")
    if not isinstance(min_bytes, int) or not isinstance(preferred_bytes, int):
        print(
            f"[validate-diagnostic-json] {path}: metadata.{key} に min_bytes / preferred_bytes が含まれていません",
            file=sys.stderr,
        )
        return False
    if preferred_bytes < min_bytes:
        print(
            f"[validate-diagnostic-json] {path}: metadata.{key} preferred_bytes < min_bytes",
            file=sys.stderr,
        )
        return False
    return True


def diag_matches_tags(entry: dict) -> bool:
    if not effect_tag_set:
        return True
    diag = entry.get("diagnostics")
    if isinstance(diag, list):
        for item in diag:
            if isinstance(item, dict):
                extensions = item.get("extensions")
                if isinstance(extensions, dict):
                    effects = extensions.get("effects")
                    if isinstance(effects, dict):
                        for field in ("before", "handled", "residual"):
                            values = effects.get(field)
                            if isinstance(values, list):
                                for candidate in values:
                                    if isinstance(candidate, str):
                                        lowered = candidate.strip().lower()
                                        if lowered in effect_tag_set:
                                            return True
                                        if "." in lowered and lowered.split(".")[-1] in effect_tag_set:
                                            return True
    return False


for raw_path in files:
    path = str(raw_path)
    if "stream" not in path.lower():
        continue
    text = pathlib.Path(path).read_text().strip()
    if not text:
        continue
    try:
        data = json.loads(text)
    except json.JSONDecodeError as exc:
        print(f"[validate-diagnostic-json] {path}: JSON parse error: {exc}", file=sys.stderr)
        error = True
        continue
    entries = data if isinstance(data, list) else [data]
    matched_effect_diag = False
    for entry in entries:
        if effect_tag_set and not diag_matches_tags(entry):
            continue
        matched_effect_diag = True
        events = entry.get("audit_events")
        if not isinstance(events, list):
            print(f"[validate-diagnostic-json] {path}: audit_events が配列ではありません", file=sys.stderr)
            error = True
            continue
        categories = {}
        for event in events:
            if isinstance(event, dict):
                categories[event.get("category")] = event
        pending = categories.get("parser.stream.pending")
        if pending is None:
            print(f"[validate-diagnostic-json] {path}: parser.stream.pending イベントがありません", file=sys.stderr)
            error = True
            continue
        pending_meta = as_dict(pending.get("metadata"))
        if pending_meta is None:
            print(f"[validate-diagnostic-json] {path}: parser.stream.pending metadata がありません", file=sys.stderr)
            error = True
            continue
        for key in required_pending_keys:
            if key not in pending_meta:
                print(f"[validate-diagnostic-json] {path}: metadata.{key} が不足しています", file=sys.stderr)
                error = True
        if not ensure_resume_hint(pending_meta, "parser.stream.pending.resume_hint", path):
            error = True
        pending_expected = pending_meta.get("parser.stream.pending.expected_tokens")
        if not isinstance(pending_expected, list):
            print(f"[validate-diagnostic-json] {path}: metadata.parser.stream.pending.expected_tokens は配列である必要があります", file=sys.stderr)
            error = True
        last_checkpoint_pending = pending_meta.get("parser.stream.pending.last_checkpoint")
        if last_checkpoint_pending is not None and not isinstance(last_checkpoint_pending, dict):
            print(f"[validate-diagnostic-json] {path}: metadata.parser.stream.pending.last_checkpoint は null またはオブジェクトである必要があります", file=sys.stderr)
            error = True

        error_event = categories.get("parser.stream.error")
        if error_event is None:
            print(f"[validate-diagnostic-json] {path}: parser.stream.error イベントがありません", file=sys.stderr)
            error = True
            continue
        error_meta = as_dict(error_event.get("metadata"))
        if error_meta is None:
            print(f"[validate-diagnostic-json] {path}: parser.stream.error metadata がありません", file=sys.stderr)
            error = True
            continue
        for key in required_error_keys:
            if key not in error_meta:
                print(f"[validate-diagnostic-json] {path}: metadata.{key} が不足しています", file=sys.stderr)
                error = True
        if not ensure_resume_hint(error_meta, "parser.stream.error.resume_hint", path):
            error = True
        error_expected = error_meta.get("parser.stream.error.expected_tokens")
        if not isinstance(error_expected, list):
            print(f"[validate-diagnostic-json] {path}: metadata.parser.stream.error.expected_tokens は配列である必要があります", file=sys.stderr)
            error = True
        last_checkpoint_error = error_meta.get("parser.stream.error.last_checkpoint")
        if last_checkpoint_error is not None and not isinstance(last_checkpoint_error, dict):
            print(f"[validate-diagnostic-json] {path}: metadata.parser.stream.error.last_checkpoint は null またはオブジェクトである必要があります", file=sys.stderr)
            error = True
        diagnostic_meta = error_meta.get("parser.stream.error.diagnostic")
        if not isinstance(diagnostic_meta, dict):
            print(f"[validate-diagnostic-json] {path}: metadata.parser.stream.error.diagnostic はオブジェクトである必要があります", file=sys.stderr)
            error = True
    if effect_tag_set and not matched_effect_diag:
        print(
            f"[validate-diagnostic-json] info: streaming filter skipped {path} "
            f"(no diagnostics with effect tags {sorted(effect_tag_set)})",
            file=sys.stderr,
        )

if error:
    sys.exit(1)
PY
      EXIT_CODE=1
    }
  fi
fi

if [[ "$REQUIRE_PRIVACY" -eq 1 ]]; then
  declare -a PRIVACY_TARGETS=()
  for file_path in "${DIAG_FILES[@]}"; do
    PRIVACY_TARGETS+=("$file_path")
  done
  for file_path in "${AUDIT_FILES[@]}"; do
    PRIVACY_TARGETS+=("$file_path")
  done
  if [[ "${#PRIVACY_TARGETS[@]}" -eq 0 ]]; then
    echo "[validate-diagnostic-json] error: --require-privacy が指定されましたが検証対象ファイルがありません" >&2
    EXIT_CODE=1
  else
    if ! python3 - "${PRIVACY_TARGETS[@]}" <<'PY'; then
import json
import pathlib
import sys
from typing import List

paths: List[pathlib.Path] = [pathlib.Path(item) for item in sys.argv[1:] if item]

def parse_entries(content: str, file_name: str) -> List[object]:
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
                raise RuntimeError(f"{file_name}:{line_no}: JSON parse error: {exc}") from exc
        return entries
    else:
        if isinstance(data, list):
            return list(data)
        return [data]


def contains_privacy_flag(container) -> bool:
    if not isinstance(container, dict):
        return False
    for key, value in container.items():
        if not isinstance(key, str):
            continue
        if not key.startswith("privacy."):
            continue
        if isinstance(value, bool):
            if value:
                return True
        elif isinstance(value, str):
            lowered = value.strip().lower()
            if lowered in {"1", "true", "yes", "required"}:
                return True
        else:
            return True
    return False


def diag_has_privacy(diag: object) -> bool:
    if not isinstance(diag, dict):
        return False
    if contains_privacy_flag(diag.get("audit_metadata")):
        return True
    audit_block = diag.get("audit")
    if isinstance(audit_block, dict) and contains_privacy_flag(audit_block.get("metadata")):
        return True
    if contains_privacy_flag(diag.get("extensions")):
        return True
    return False


def audit_event_has_privacy(event: object) -> bool:
    if not isinstance(event, dict):
        return False
    envelope = event.get("envelope")
    if isinstance(envelope, dict) and contains_privacy_flag(envelope.get("metadata")):
        return True
    if contains_privacy_flag(event.get("extensions")):
        return True
    return False


def entry_has_privacy(entry: object) -> bool:
    if isinstance(entry, dict) and "diagnostics" in entry:
        diagnostics = entry.get("diagnostics")
        if isinstance(diagnostics, list) and diagnostics:
            return any(diag_has_privacy(item) for item in diagnostics if isinstance(item, dict))
    if diag_has_privacy(entry):
        return True
    if audit_event_has_privacy(entry):
        return True
    return False


missing: List[str] = []
for path in paths:
    if not path.exists():
        continue
    text = path.read_text(encoding="utf-8")
    try:
        entries = parse_entries(text, str(path))
    except Exception as exc:  # noqa: BLE001
        print(f"[validate-diagnostic-json] {exc}", file=sys.stderr)
        missing.append(str(path))
        continue
    if not entries:
        missing.append(str(path))
        continue
    if not any(entry_has_privacy(entry) for entry in entries):
        missing.append(str(path))

if missing:
    for name in missing:
        print(f"[validate-diagnostic-json] privacy metadata missing: {name}", file=sys.stderr)
    sys.exit(1)
PY
      EXIT_CODE=1
    fi
  fi
fi

if [[ "$SUITE" == "collectors" ]]; then
  if [[ "${#AUDIT_FILES[@]}" -eq 0 ]]; then
    echo "[validate-diagnostic-json] error: collectors スイート向けの監査ファイルが見つかりません" >&2
    EXIT_CODE=1
  else
    if ! python3 - "${AUDIT_FILES[@]}" <<'PY'; then
import json
import pathlib
import sys

paths = [pathlib.Path(item) for item in sys.argv[1:] if item]
found_cell = False
found_rc = False

for path in paths:
    if not path.exists():
        continue
    text = path.read_text(encoding="utf-8")
    for line in text.splitlines():
        entry = line.strip()
        if not entry:
            continue
        try:
            data = json.loads(entry)
        except json.JSONDecodeError:
            continue
        metadata = data.get("metadata")
        if isinstance(metadata, dict):
            if "collector.effect.cell" in metadata:
                found_cell = True
            if "collector.effect.rc" in metadata:
                found_rc = True
        if found_cell and found_rc:
            break
    if found_cell and found_rc:
        break

if not found_cell:
    print("[validate-diagnostic-json] collectors audit に collector.effect.cell がありません", file=sys.stderr)
if not found_rc:
    print("[validate-diagnostic-json] collectors audit に collector.effect.rc がありません", file=sys.stderr)
if not found_cell or not found_rc:
    sys.exit(1)
PY
      EXIT_CODE=1
    fi
  fi
fi

if [[ "$SECTION" == "config" ]]; then
  if [[ "${#FILES[@]}" -eq 0 ]]; then
    echo "[validate-diagnostic-json] error: --section config で対象ファイルがありません" >&2
    EXIT_CODE=1
  else
    if ! python3 - "${FILES[@]}" <<'PY'; then
import json
import pathlib
import sys

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
        if isinstance(data, dict):
            diagnostics = data.get("diagnostics")
            if isinstance(diagnostics, list):
                return diagnostics
            return [data]
        return data if isinstance(data, list) else [data]


def contains_schema_diff(node) -> bool:
    if isinstance(node, dict):
        for key, value in node.items():
            if isinstance(key, str) and key.startswith("schema_diff"):
                return True
            if contains_schema_diff(value):
                return True
        return False
    if isinstance(node, list):
        return any(contains_schema_diff(item) for item in node)
    return False


def has_config_metadata(entry) -> bool:
    if not isinstance(entry, dict):
        return False
    extensions = entry.get("extensions")
    config_extension = (
        isinstance(extensions, dict) and isinstance(extensions.get("config"), dict)
    )
    audit_metadata = entry.get("audit_metadata")
    audit_has_config = False
    if isinstance(audit_metadata, dict):
        audit_has_config = any(
            isinstance(key, str) and key.startswith("config.")
            for key in audit_metadata.keys()
        )
    return config_extension and audit_has_config


paths = [pathlib.Path(item) for item in sys.argv[1:] if item]

for path in paths:
    if not path.exists():
        continue
    entries = parse_entries(path.read_text(encoding="utf-8"), str(path))
    name = path.name.lower()
    if "config_diagnostics" in name:
        if not any(has_config_metadata(entry) for entry in entries):
            raise RuntimeError(f"{path}: config.* metadata が不足しています")
    else:
        if not any(contains_schema_diff(entry) for entry in entries):
            raise RuntimeError(f"{path}: schema_diff metadata が見つかりません")
PY
      EXIT_CODE=1
    fi
  fi
fi

exit $EXIT_CODE
