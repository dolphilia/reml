#!/usr/bin/env bash

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $(basename "$0") <capabilities.json> [...]" >&2
  exit 1
fi

python3 - "$@" <<'PY'
import json
import pathlib
import sys

def validate_capabilities(path: pathlib.Path) -> None:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise SystemExit(f"{path}: invalid JSON — {exc}") from exc

    if not isinstance(data, dict):
        raise SystemExit(f"{path}: root value must be an object")

    stage = data.get("stage")
    if stage is not None and not isinstance(stage, str):
        raise SystemExit(f"{path}: 'stage' must be a string when present")

    capabilities = data.get("capabilities", [])
    if isinstance(capabilities, list):
        for idx, item in enumerate(capabilities):
            if isinstance(item, str):
                continue
            if isinstance(item, dict):
                name = item.get("name")
                if not isinstance(name, str):
                    raise SystemExit(f"{path}: capabilities[{idx}].name must be a string")
                if "stage" in item and not isinstance(item["stage"], str):
                    raise SystemExit(f"{path}: capabilities[{idx}].stage must be a string")
                continue
            raise SystemExit(
                f"{path}: capabilities[{idx}] must be a string or object "
                "(with 'name' and optional 'stage')"
            )
    elif isinstance(capabilities, dict):
        for cap_name, value in capabilities.items():
            if isinstance(value, str):
                continue
            if isinstance(value, dict):
                stage_value = value.get("stage")
                if stage_value is not None and not isinstance(stage_value, str):
                    raise SystemExit(
                        f"{path}: capability '{cap_name}' stage must be a string"
                    )
                continue
            raise SystemExit(
                f"{path}: capability '{cap_name}' must map to a string or object"
            )
    else:
        raise SystemExit(f"{path}: 'capabilities' must be a list or an object")

    overrides = data.get("overrides", {})
    if not isinstance(overrides, dict):
        raise SystemExit(f"{path}: 'overrides' must be an object when present")

    for target, value in overrides.items():
        if isinstance(value, dict):
            stage_value = value.get("stage")
            if stage_value is not None and not isinstance(stage_value, str):
                raise SystemExit(
                    f"{path}: override '{target}' stage must be a string"
                )
            capabilities_value = value.get("capabilities", [])
            if not isinstance(capabilities_value, (list, dict)):
                raise SystemExit(
                    f"{path}: override '{target}' capabilities must be a list or object"
                )
        elif isinstance(value, list):
            for idx, item in enumerate(value):
                if not isinstance(item, str):
                    raise SystemExit(
                        f"{path}: override '{target}' list entries must be strings "
                        f"(invalid entry at index {idx})"
                    )
        elif isinstance(value, dict):
            continue
        else:
            raise SystemExit(
                f"{path}: override '{target}' must be a list or object"
            )

for arg in sys.argv[1:]:
    file_path = pathlib.Path(arg)
    if not file_path.exists():
        raise SystemExit(f"{file_path}: file not found")
    validate_capabilities(file_path)
    print(f"{file_path}: ok")
PY
