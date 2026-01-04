#!/usr/bin/env bash
# Audit JSON/JSONL schema validator for CI pipelines.

set -euo pipefail

SCHEMA_PATH="tooling/runtime/audit-schema.json"
SUMMARY_PATH=""
FAIL_SILENTLY=0
ALLOW_MISSING=0
declare -a INPUT_PATHS=()

usage() {
  cat <<'USAGE'
Usage: scripts/ci-validate-audit.sh [options]

Options:
  --schema <path>        JSON Schema file (default: tooling/runtime/audit-schema.json)
  --input <path>         Audit JSON/JSONL file to validate (repeatable)
  --summary <path>       Write Markdown summary to the specified file
  --allow-missing        Skip validation when the input file does not exist
  --fail-silently        Do not fail the script even if validation errors occur
  -h, --help             Show this help message
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --schema)
      shift
      SCHEMA_PATH="${1:-}"
      shift
      ;;
    --input)
      shift
      INPUT_PATHS+=("${1:-}")
      shift
      ;;
    --summary)
      shift
      SUMMARY_PATH="${1:-}"
      shift
      ;;
    --allow-missing)
      ALLOW_MISSING=1
      shift
      ;;
    --fail-silently)
      FAIL_SILENTLY=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ ${#INPUT_PATHS[@]} -eq 0 ]]; then
  echo "No --input file specified." >&2
  usage >&2
  exit 2
fi

if [[ ! -f "$SCHEMA_PATH" ]]; then
  echo "Schema file not found: $SCHEMA_PATH" >&2
  exit 2
fi

TOTAL_COUNT=0
SUCCESS_COUNT=0
FAIL_COUNT=0
declare -a ERROR_LINES=()

TEMP_JSON="$(mktemp)"
trap 'rm -f "$TEMP_JSON"' EXIT

validate_json() {
  local descriptor="$1"
  local description="$2"
  if output=$(npx --yes ajv-cli validate -s "$SCHEMA_PATH" -d "$TEMP_JSON" 2>&1); then
    ((SUCCESS_COUNT++))
  else
    ((FAIL_COUNT++))
    ERROR_LINES+=("SCHEMA_ERROR:${descriptor}:${description}:${output//$'\n'/ }")
    echo "SCHEMA_ERROR:${descriptor}:${description}:${output//$'\n'/ }" >&2
  fi
}

for input_path in "${INPUT_PATHS[@]}"; do
  if [[ ! -f "$input_path" ]]; then
    if (( ALLOW_MISSING )); then
      echo "SCHEMA_WARN: missing input ${input_path} (allowed)" >&2
      continue
    else
      ((FAIL_COUNT++))
      ERROR_LINES+=("SCHEMA_ERROR:${input_path}:missing:input file not found")
      echo "SCHEMA_ERROR:${input_path}:missing:input file not found" >&2
      continue
    fi
  fi

  extension="${input_path##*.}"
  if [[ "$extension" == "jsonl" ]]; then
    line_no=0
    while IFS= read -r line || [[ -n "$line" ]]; do
      ((line_no++))
      trimmed="${line#"${line%%[![:space:]]*}"}"
      if [[ -z "$trimmed" ]]; then
        continue
      fi
      printf '%s\n' "$line" > "$TEMP_JSON"
      ((TOTAL_COUNT++))
      validate_json "$input_path" "line ${line_no}"
    done < "$input_path"
  else
    cp "$input_path" "$TEMP_JSON"
    ((TOTAL_COUNT++))
    validate_json "$input_path" "document"
  fi
done

if [[ -n "$SUMMARY_PATH" ]]; then
  {
    echo "# 監査スキーマ検証レポート"
    echo ""
    echo "- 検証対象ファイル数: ${#INPUT_PATHS[@]}"
    echo "- 文書数: ${TOTAL_COUNT}"
    echo "- 成功: ${SUCCESS_COUNT}"
    echo "- 失敗: ${FAIL_COUNT}"
    if [[ ${#ERROR_LINES[@]} -gt 0 ]]; then
      echo ""
      echo "## エラー詳細"
      echo ""
      for entry in "${ERROR_LINES[@]}"; do
        echo "- ${entry}"
      done
    fi
  } > "$SUMMARY_PATH"
fi

if (( FAIL_COUNT > 0 )) && (( FAIL_SILENTLY == 0 )); then
  exit 1
fi

exit 0
