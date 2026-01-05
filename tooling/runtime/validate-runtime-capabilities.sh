#!/usr/bin/env bash

set -euo pipefail

CLI_STAGE=""
ENV_STAGE_OVERRIDE=""
OUTPUT_PATH=""
JSON_FILES=()

print_usage() {
    cat <<'EOF'
Usage:
  tooling/runtime/validate-runtime-capabilities.sh [options] <capabilities.json> [...]

Options:
  --cli-stage <STAGE>     Stage を CLI フラグが指定した場合の想定値として記録する
  --env-stage <STAGE>     Stage を環境変数が指定した場合の想定値として記録する
  --output <PATH>         検証サマリー JSON の出力先（既定: reports/runtime-capabilities-validation.json）
  -h, --help              このヘルプを表示

説明:
  指定した RuntimeCapability JSON を検証し、Stage 情報・オーバーライド内容を
  正規化して `stage_summary` と `stage_trace` を含むレポート JSON を出力します。
  スキーマ違反を検出した場合は非ゼロで終了します。
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --cli-stage)
            shift || { echo "error: --cli-stage requires an argument" >&2; exit 1; }
            CLI_STAGE="$1"
            shift
            ;;
        --env-stage)
            shift || { echo "error: --env-stage requires an argument" >&2; exit 1; }
            ENV_STAGE_OVERRIDE="$1"
            shift
            ;;
        --output)
            shift || { echo "error: --output requires an argument" >&2; exit 1; }
            OUTPUT_PATH="$1"
            shift
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        --)
            shift
            JSON_FILES+=("$@")
            break
            ;;
        -*)
            echo "error: unknown option: $1" >&2
            print_usage
            exit 1
            ;;
        *)
            JSON_FILES+=("$1")
            shift
            ;;
    esac
done

if [[ ${#JSON_FILES[@]} -eq 0 ]]; then
    echo "error: at least one capabilities.json must be provided" >&2
    print_usage
    exit 1
fi

if [[ -z "$OUTPUT_PATH" ]]; then
    OUTPUT_PATH="reports/runtime-capabilities-validation.json"
fi

if [[ "$OUTPUT_PATH" != "-" ]]; then
    mkdir -p "$(dirname "$OUTPUT_PATH")"
fi

if [[ -z "$ENV_STAGE_OVERRIDE" ]]; then
    ENV_STAGE_OVERRIDE="${REMLC_EFFECT_STAGE:-}"
fi

SUMMARY_JSON="$(
    CLI_STAGE="$CLI_STAGE" \
    ENV_STAGE_OVERRIDE="$ENV_STAGE_OVERRIDE" \
    python3 - "${JSON_FILES[@]}" <<'PYCODE'
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, List, Optional, Tuple, Union

CLI_STAGE = os.environ.get("CLI_STAGE") or None
ENV_STAGE_OVERRIDE = os.environ.get("ENV_STAGE_OVERRIDE") or None

if len(sys.argv) <= 1:
    print("No capability JSON files were provided", file=sys.stderr)
    sys.exit(1)

StageValue = Optional[str]


def load_json(path: Path) -> Dict:
    try:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(handle)
    except FileNotFoundError:
        raise SystemExit(f"{path}: file not found")
    except json.JSONDecodeError as exc:
        raise SystemExit(f"{path}: invalid JSON — {exc}")


class ValidationError(Exception):
    pass


def ensure_dict(value: Union[Dict, None], path: str) -> Dict:
    if isinstance(value, dict):
        return value
    if value is None:
        return {}
    raise ValidationError(f"{path}: must be an object")


def ensure_list(value: Union[List, None], path: str) -> List:
    if isinstance(value, list):
        return value
    if value is None:
        return []
    raise ValidationError(f"{path}: must be an array")


def normalise_capabilities(raw, *, context: str) -> List[Dict[str, Optional[str]]]:
    result: List[Dict[str, Optional[str]]] = []
    if raw is None:
        return result

    if isinstance(raw, list):
        for idx, item in enumerate(raw):
            if isinstance(item, str):
                result.append({"name": item, "stage": None})
            elif isinstance(item, dict):
                name = item.get("name")
                if not isinstance(name, str):
                    raise ValidationError(
                        f"{context}: capabilities[{idx}].name must be a string"
                    )
                stage = item.get("stage")
                if stage is not None and not isinstance(stage, str):
                    raise ValidationError(
                        f"{context}: capabilities[{idx}].stage must be a string"
                    )
                result.append({"name": name, "stage": stage})
            else:
                raise ValidationError(
                    f"{context}: capabilities[{idx}] must be a string or object"
                )
        return result

    if isinstance(raw, dict):
        for name, value in raw.items():
            if isinstance(value, str):
                result.append({"name": name, "stage": value})
            elif isinstance(value, dict):
                stage = value.get("stage")
                if stage is not None and not isinstance(stage, str):
                    raise ValidationError(
                        f"{context}: capability '{name}' stage must be a string"
                    )
                result.append({"name": name, "stage": stage})
            else:
                raise ValidationError(
                    f"{context}: capability '{name}' must map to a string or object"
                )
        return result

    raise ValidationError(f"{context}: capabilities must be a list or object")


def validate_file(path: Path) -> Tuple[Dict, Dict]:
    data = load_json(path)
    if not isinstance(data, dict):
        raise ValidationError(f"{path}: root value must be an object")

    stage = data.get("stage")
    if stage is not None and not isinstance(stage, str):
        raise ValidationError(f"{path}: 'stage' must be a string when present")

    capabilities = normalise_capabilities(
        data.get("capabilities"),
        context=f"{path}: capabilities",
    )

    overrides_raw = data.get("overrides")
    if overrides_raw is None:
        overrides_raw = {}
    if not isinstance(overrides_raw, dict):
        raise ValidationError(f"{path}: 'overrides' must be an object when present")

    overrides: List[Dict] = []
    for target, override_value in overrides_raw.items():
        entry_stage: StageValue = None
        normalized_caps: List[Dict[str, Optional[str]]] = []

        if isinstance(override_value, dict):
            entry_stage_value = override_value.get("stage")
            if entry_stage_value is not None and not isinstance(entry_stage_value, str):
                raise ValidationError(
                    f"{path}: override '{target}' stage must be a string"
                )
            entry_stage = entry_stage_value
            normalized_caps = normalise_capabilities(
                override_value.get("capabilities"),
                context=f"{path}: overrides['{target}'].capabilities",
            )
        elif isinstance(override_value, list):
            entry_stage = None
            normalized_caps = normalise_capabilities(
                override_value,
                context=f"{path}: overrides['{target}']",
            )
        else:
            raise ValidationError(
                f"{path}: override '{target}' must be a list or object"
            )

        overrides.append(
            {
                "target": target,
                "stage": entry_stage,
                "capabilities": normalized_caps,
            }
        )

    meta = {
        "file": str(path),
        "stage": stage,
        "capabilities": capabilities,
        "overrides": overrides,
    }
    return data, meta


errors: List[str] = []
files_summary: List[Dict] = []

for arg in sys.argv[1:]:
    file_path = Path(arg)
    try:
        _, meta = validate_file(file_path)
        files_summary.append(meta)
        print(f"{file_path}: ok", file=sys.stderr)
    except ValidationError as exc:
        errors.append(str(exc))

if errors:
    for message in errors:
        print(message, file=sys.stderr)
    sys.exit(1)

timestamp = datetime.now(timezone.utc).isoformat()

runtime_candidates: List[Dict[str, Optional[str]]] = []
for entry in files_summary:
    base_stage = entry.get("stage")
    runtime_candidates.append(
        {
            "source": "capability_json#stage",
            "file": entry["file"],
            "target": "default",
            "stage": base_stage,
        }
    )
    for override in entry["overrides"]:
        runtime_candidates.append(
            {
                "source": "capability_json#override",
                "file": entry["file"],
                "target": override["target"],
                "stage": override.get("stage", base_stage),
            }
        )

stage_trace: List[Dict[str, Optional[str]]] = []

if CLI_STAGE:
    stage_trace.append(
        {"source": "cli_option", "stage": CLI_STAGE, "note": "--cli-stage argument"}
    )
else:
    stage_trace.append(
        {"source": "cli_option", "stage": None, "note": "not provided"}
    )

if ENV_STAGE_OVERRIDE:
    stage_trace.append(
        {
            "source": "env_var",
            "stage": ENV_STAGE_OVERRIDE,
            "note": "REMLC_EFFECT_STAGE",
        }
    )
else:
    stage_trace.append(
        {
            "source": "env_var",
            "stage": None,
            "note": "REMLC_EFFECT_STAGE not set",
        }
    )

for entry in files_summary:
    stage_trace.append(
        {
            "source": "capability_json",
            "stage": entry.get("stage"),
            "file": entry["file"],
        }
    )

for candidate in runtime_candidates:
    stage_trace.append(
        {
            "source": "runtime_candidate",
            "stage": candidate.get("stage"),
            "file": candidate.get("file"),
            "target": candidate.get("target"),
        }
    )

summary = {
    "timestamp": timestamp,
    "checked_files": [entry["file"] for entry in files_summary],
    "stage_summary": {
        "cli": {"stage": CLI_STAGE},
        "env": {"stage": ENV_STAGE_OVERRIDE, "variable": "REMLC_EFFECT_STAGE"},
        "json": files_summary,
        "runtime_candidates": runtime_candidates,
    },
    "stage_trace": stage_trace,
    "validation": {
        "status": "ok",
        "errors": [],
    },
}

print(json.dumps(summary, ensure_ascii=False, indent=2))
PYCODE
)"

if [[ "$OUTPUT_PATH" == "-" ]]; then
    printf '%s\n' "$SUMMARY_JSON"
else
    printf '%s\n' "$SUMMARY_JSON" >"$OUTPUT_PATH"
    echo "Stage summary written to $OUTPUT_PATH"
fi
