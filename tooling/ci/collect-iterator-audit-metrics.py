#!/usr/bin/env python3
"""
iterator.stage.audit_pass_rate collector.

The script scans diagnostics with code `typeclass.iterator.stage_mismatch` and
verifies that mandatory audit metadata and iterator extensions are provided.
It prints a JSON summary to stdout and can also write the result to a file.

Example:
    ./tooling/ci/collect-iterator-audit-metrics.py \
        --source compiler/ocaml/tests/golden/typeclass_iterator_stage_mismatch.json.golden \
        --output tooling/ci/iterator-audit-metrics.json

    # Windows 向けに bridge.platform をフィルタして収集
    ./tooling/ci/collect-iterator-audit-metrics.py \
        --source compiler/ocaml/tests/golden/diagnostics/ffi/unsupported-abi.json.golden \
        --audit-source compiler/ocaml/tests/golden/audit/cli-ffi-bridge-windows.jsonl.golden \
        --platform windows-msvc \
        --output tooling/ci/iterator-audit-metrics-windows.json \
        --require-success
"""

from __future__ import annotations

import argparse
import copy
import json
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Sequence, Set, Tuple

try:
    import tomllib  # Python 3.11+
except ModuleNotFoundError:  # pragma: no cover
    tomllib = None


# Required audit metadata keys (logical name, candidate paths, allow_empty, allow_null).
class RequiredField(Tuple[str, Tuple[str, ...], bool, bool]):
    __slots__ = ()

    @property
    def logical(self) -> str:
        return self[0]

    @property
    def candidates(self) -> Tuple[str, ...]:
        return self[1]

    @property
    def allow_empty(self) -> bool:
        return self[2]

    @property
    def allow_null(self) -> bool:
        return self[3]


def Field(
    logical: str,
    candidates: Sequence[str],
    *,
    allow_empty: bool = False,
    allow_null: bool = False,
) -> RequiredField:
    return RequiredField((logical, tuple(candidates), allow_empty, allow_null))


REQUIRED_AUDIT_FIELDS: List[RequiredField] = [
    Field("cli.audit_id", ("cli.audit_id", "audit_id")),
    Field("cli.change_set", ("cli.change_set", "change_set")),
    Field("schema.version", ("schema.version",)),
    Field("event.domain", ("event.domain",)),
    Field("event.kind", ("event.kind",)),
    Field("effect.stage.required", ("effect.stage.required",)),
    Field("effect.stage.actual", ("effect.stage.actual",)),
    Field("effect.capability", ("effect.capability",)),
    Field("capability.ids", ("capability.ids",)),
    Field(
        "effect.required_capabilities",
        ("effect.required_capabilities",),
        allow_empty=True,
    ),
    Field(
        "effect.stage.required_capabilities",
        ("effect.stage.required_capabilities",),
        allow_empty=True,
    ),
    Field(
        "effect.actual_capabilities",
        ("effect.actual_capabilities",),
        allow_empty=True,
    ),
    Field(
        "effect.stage.actual_capabilities",
        ("effect.stage.actual_capabilities",),
        allow_empty=True,
    ),
    Field(
        "effect.capability_descriptor",
        ("effect.capability_descriptor", "effect.capability_metadata"),
        allow_empty=True,
    ),
    Field("effect.handler_stack", ("effect.handler_stack",)),
    Field(
        "effect.unhandled_operations",
        ("effect.unhandled_operations",),
        allow_empty=True,
        allow_null=True,
    ),
    Field("effect.stage.iterator.required", ("effect.stage.iterator.required",)),
    Field("effect.stage.iterator.actual", ("effect.stage.iterator.actual",)),
    Field("effect.stage.iterator.kind", ("effect.stage.iterator.kind",)),
    Field(
        "effect.stage.iterator.capability",
        ("effect.stage.iterator.capability",),
    ),
    Field("effect.stage.iterator.source", ("effect.stage.iterator.source",)),
    Field("bridge.audit_pass_rate", ("bridge.audit_pass_rate",)),
]

# Required fields under extensions.effects / extensions.typeclass / extensions.parse.
REQUIRED_EFFECT_STAGE_KEYS: List[str] = ["required", "actual"]
REQUIRED_EFFECT_ITERATOR_KEYS: List[str] = [
    "required",
    "actual",
    "kind",
    "capability",
    "source",
]
REQUIRED_EFFECT_ADDITIONAL_KEYS: List[str] = [
    "residual",
    "handler_stack",
    "unhandled_operations",
    "capability_descriptor",
]
EFFECT_ADDITIONAL_KEY_ALIASES: Dict[str, Tuple[str, ...]] = {
    "residual": ("residual",),
    "handler_stack": ("handler_stack",),
    "unhandled_operations": ("unhandled_operations",),
    "capability_descriptor": ("capability_descriptor", "metadata"),
}
EFFECT_ALLOW_EMPTY_KEYS: Set[str] = {"unhandled_operations"}
REQUIRED_TYPECLASS_SCALAR_KEYS: Sequence[str] = (
    "trait",
    "constraint",
    "resolution_state",
)
REQUIRED_TYPECLASS_LIST_KEYS: Sequence[str] = (
    "type_args",
    "pending",
    "generalized_typevars",
    "candidates",
)
REQUIRED_TYPECLASS_OPTIONAL_KEYS: Sequence[str] = (
    "dictionary",
    "graph",
)
TYPECLASS_REQUIRED_AUDIT_KEYS: Sequence[str] = (
    "typeclass.trait",
    "typeclass.type_args",
    "typeclass.constraint",
    "typeclass.resolution_state",
    "typeclass.dictionary.kind",
    "typeclass.pending",
    "typeclass.generalized_typevars",
    "typeclass.candidates",
)
REQUIRED_PARSE_KEYS: List[str] = ["input_name", "stage_trace"]

# FFI bridge metrics configuration.
BRIDGE_DIAG_PREFIX = "ffi.contract."
REQUIRED_BRIDGE_AUDIT_KEYS: List[str] = [
    "audit_id",
    "change_set",
    "cli.audit_id",
    "cli.change_set",
    "schema.version",
    "bridge.audit_pass_rate",
    "bridge.status",
    "bridge.target",
    "bridge.arch",
    "bridge.abi",
    "bridge.ownership",
    "bridge.extern_symbol",
    "bridge.platform",
    "bridge.return.ownership",
    "bridge.return.status",
    "bridge.return.wrap",
    "bridge.return.release_handler",
    "bridge.return.rc_adjustment",
]
REQUIRED_BRIDGE_EXTENSION_KEYS: List[str] = [
    "bridge.target",
    "bridge.ownership",
    "bridge.abi",
    "bridge.platform",
    "bridge.audit_pass_rate",
    "bridge.return.ownership",
    "bridge.return.status",
    "bridge.return.wrap",
    "bridge.return.release_handler",
    "bridge.return.rc_adjustment",
]

BASIC_AUDIT_FIELDS: List[RequiredField] = [
    Field("cli.audit_id", ("cli.audit_id", "audit_id")),
    Field("cli.change_set", ("cli.change_set", "change_set")),
    Field("schema.version", ("schema.version",)),
]

DEFAULT_RETENTION_POLICY: Dict[str, int] = {
    "ci": 100,
    "local": 30,
    "tmp": 20,
    "default": 50,
}

TEXT_DEFAULT_METRICS_PATH = Path("reports/spec-audit/ch1/core_text_grapheme_stats.json")
TEXT_CACHE_HIT_TARGET = 0.8
TEXT_CASE_RULES: Dict[str, Dict[str, float]] = {
    "UC-01": {
        "min_cache_miss": 1,
        "max_cache_hits": 0,
    },
    "UC-02": {
        "min_hit_ratio": 0.7,
        "min_cache_hits": 1,
    },
    "UC-03": {
        "max_cache_miss": 0,
        "min_cache_hits": 1,
        "min_hit_ratio": 1.0,
    },
}


def calculate_pass_rates(passed: int, total: int) -> Tuple[Optional[float], Optional[float]]:
    if total <= 0:
        return None, None
    fraction = passed / total
    effective = 1.0 if passed == total else 0.0
    return effective, fraction


def _ensure_path_list(raw_paths: Optional[Sequence[str]]) -> List[Path]:
    if not raw_paths:
        return []
    result: List[Path] = []
    for item in raw_paths:
        if item is None:
            continue
        result.append(Path(item))
    return result

def load_json(path: Path) -> Dict:
    with path.open("r", encoding="utf-8") as handle:
        try:
            return json.load(handle)
        except json.JSONDecodeError as exc:
            raise ValueError(f"Failed to parse JSON: {path}") from exc


def load_audit_entries(path: Path) -> List[Dict[str, Any]]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ValueError(f"Failed to read audit log: {path}") from exc

    text = text.strip()
    if not text:
        return []

    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        entries: List[Dict[str, Any]] = []
        for line_no, line in enumerate(text.splitlines(), start=1):
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError as exc:
                raise ValueError(
                    f"Failed to parse JSON line ({path}:{line_no}): {exc}"
                ) from exc
            if isinstance(obj, dict):
                entries.append(obj)
        return entries

    if isinstance(data, list):
        return [entry for entry in data if isinstance(entry, dict)]
    if isinstance(data, dict):
        return [data]
    return []


def iter_diagnostics(data: Dict) -> Iterable[Dict]:
    diagnostics = data.get("diagnostics")
    if not isinstance(diagnostics, list):
        raise ValueError("diagnostics array is missing")
    for diag in diagnostics:
        if not isinstance(diag, dict):
            continue
        yield diag


def _lookup_in_container(container: Dict[str, Any], path: str) -> Tuple[bool, Optional[object]]:
    if path in container:
        return True, container[path]
    current: object = container
    for part in path.split("."):
        if isinstance(current, dict) and part in current:
            current = current[part]
        else:
            return False, None
    return True, current


def _check_required_field_set(
    audit: Optional[Dict],
    fields: Sequence[RequiredField],
) -> List[str]:
    if audit is None or not isinstance(audit, dict):
        return ["audit"] + [field.logical for field in fields]

    metadata = audit.get("metadata")
    containers: List[Dict[str, Any]] = [audit]
    if isinstance(metadata, dict):
        containers.append(metadata)

    missing: List[str] = []
    for field in fields:
        found = False
        for container in containers:
            if not isinstance(container, dict):
                continue
            for candidate in field.candidates:
                exists, value = _lookup_in_container(container, candidate)
                if not exists:
                    continue
                if value is None and not field.allow_null:
                    continue
                if value in ("", []) and not field.allow_empty:
                    continue
                found = True
                break
            if found:
                break
        if not found:
            missing.append(field.logical)
    return missing


def check_audit_fields(audit: Optional[Dict]) -> List[str]:
    return _check_required_field_set(audit, REQUIRED_AUDIT_FIELDS)


def check_basic_audit_fields(audit: Optional[Dict]) -> List[str]:
    return _check_required_field_set(audit, BASIC_AUDIT_FIELDS)


def _as_dict(value: Optional[object]) -> Optional[Dict]:
    if isinstance(value, dict):
        return value
    return None


def _coerce_bool(value: Optional[object]) -> bool:
    if isinstance(value, bool):
        return value
    if isinstance(value, (int, float)):
        return value != 0
    if isinstance(value, str):
        lowered = value.strip().lower()
        if lowered in {"true", "1", "yes", "on"}:
            return True
        if lowered in {"false", "0", "no", "off"}:
            return False
    return False


def _coerce_int_value(value: Optional[object]) -> Optional[int]:
    if isinstance(value, bool):
        return int(value)
    if isinstance(value, (int, float)):
        return int(value)
    if isinstance(value, str):
        stripped = value.strip()
        if not stripped:
            return None
        try:
            return int(float(stripped))
        except ValueError:
            return None
    return None


def _extract_stream_extension(
    run_config: Optional[Dict[str, Any]]
) -> Optional[Dict[str, Any]]:
    if not isinstance(run_config, dict):
        return None
    extensions = _as_dict(run_config.get("extensions"))
    if not extensions:
        return None
    return _as_dict(extensions.get("stream"))


def _is_streaming_enabled(entry: Dict[str, Any]) -> bool:
    run_config = _as_dict(entry.get("run_config"))
    if not run_config:
        diagnostics = entry.get("diagnostics")
        if isinstance(diagnostics, list):
            for diag in diagnostics:
                extensions = _as_dict(diag.get("extensions"))
                if not extensions:
                    continue
                run_config_ext = _as_dict(extensions.get("runconfig"))
                if run_config_ext:
                    run_config = run_config_ext
                    break
    stream_extension = _extract_stream_extension(run_config)
    if not stream_extension:
        return False
    return _coerce_bool(stream_extension.get("enabled"))


def _expected_alternative_labels(alternatives: object) -> List[str]:
    labels: List[str] = []
    if not isinstance(alternatives, list):
        return labels
    for entry in alternatives:
        label: Optional[str] = None
        if isinstance(entry, dict):
            for key in ("token", "label", "value", "text", "name"):
                candidate = entry.get(key)
                if isinstance(candidate, str) and candidate.strip():
                    label = candidate.strip()
                    break
        elif isinstance(entry, str) and entry.strip():
            label = entry.strip()
        if label:
            labels.append(label)
    return labels


def _streaming_expected_tokens_status(entry: Dict[str, Any]) -> Tuple[bool, Dict[str, Any]]:
    parser_diag_count = 0
    placeholder_only = False
    for diag in iter_diagnostics(entry):
        if not isinstance(diag, dict) or not is_parser_diagnostic(diag):
            continue
        parser_diag_count += 1
        expected = diag.get("expected")
        if not isinstance(expected, dict):
            continue
        labels = _expected_alternative_labels(expected.get("alternatives"))
        if not labels:
            continue
        if any(label not in STREAMING_PLACEHOLDER_LABELS for label in labels):
            return True, {"parser_diagnostics": parser_diag_count}
        placeholder_only = True
    failure: Dict[str, Any] = {"parser_diagnostics": parser_diag_count}
    if placeholder_only:
        failure["placeholder_only"] = True
        failure["reason"] = "placeholder_only"
    else:
        failure["reason"] = "missing_expected_tokens"
    return False, failure



def _as_string_list(value: Optional[object]) -> Optional[List[str]]:
    if isinstance(value, list):
        result: List[str] = []
        for item in value:
            if isinstance(item, str):
                result.append(item)
            elif item is not None:
                result.append(str(item))
        return result
    return None


def _value_present(value: Optional[object]) -> bool:
    if value is None:
        return False
    if isinstance(value, str):
        return bool(value.strip())
    if isinstance(value, (list, tuple, set, dict)):
        return len(value) > 0
    return True


def _diagnostic_has_code(diag: Dict[str, Any], target: str) -> bool:
    primary = primary_code_of(diag)
    if primary == target:
        return True
    codes_field = diag.get("codes")
    if isinstance(codes_field, list):
        for item in codes_field:
            if isinstance(item, str) and item == target:
                return True
    return False


def _diagnostic_metadata_lookup(diag: Dict[str, Any], key: str) -> Optional[object]:
    metadata = _as_dict(diag.get("audit_metadata"))
    if metadata and key in metadata:
        return metadata.get(key)
    audit = _as_dict(diag.get("audit"))
    if audit:
        meta = _as_dict(audit.get("metadata"))
        if meta and key in meta:
            return meta.get(key)
    return None


def _normalize_domain(value: Optional[object]) -> Optional[str]:
    if isinstance(value, str):
        lowered = value.strip().lower()
        return lowered if lowered else None
    return None


def _normalize_platform(value: Optional[object]) -> Optional[str]:
    if isinstance(value, str):
        lowered = value.strip().lower()
        if not lowered:
            return None
        if lowered in {"windows-msvc-x64", "x86_64-pc-windows-msvc", "win64"}:
            return "windows-msvc"
        if lowered in {"macos-arm64", "arm64-apple-darwin", "darwin-arm64"}:
            return "macos-arm64"
        return lowered
    return None


def _collect_diagnostic_codes(diags: Iterable[Any]) -> Set[str]:
    codes: Set[str] = set()
    for diag in diags:
        if not isinstance(diag, dict):
            continue
        primary = primary_code_of(diag)
        if isinstance(primary, str) and primary.strip():
            codes.add(primary.strip().lower())
        raw_codes = diag.get("codes")
        if isinstance(raw_codes, list):
            for item in raw_codes:
                if isinstance(item, str) and item.strip():
                    codes.add(item.strip().lower())
    return codes


IGNORED_BRIDGE_CODES: Set[str] = {
    "bridge.stage.backpressure",
    "effects.contract.stage_mismatch",
}

STREAMING_PLACEHOLDER_LABELS = {"<streaming-placeholder>", "解析継続トークン"}


def _filter_bridge_diagnostics(diags: Iterable[Any]) -> List[Dict[str, Any]]:
    filtered: List[Dict[str, Any]] = []
    for diag in diags:
        if not isinstance(diag, dict):
            continue
        primary = primary_code_of(diag)
        if isinstance(primary, str) and primary.strip().lower() in IGNORED_BRIDGE_CODES:
            continue
        diag_codes = _collect_diagnostic_codes([diag])
        if diag_codes.intersection(IGNORED_BRIDGE_CODES):
            continue
        filtered.append(diag)
    return filtered


def _extract_bridge_platforms(entry: object) -> Set[str]:
    platforms: Set[str] = set()

    def visit(node: object, path: Tuple[str, ...]) -> None:
        if isinstance(node, dict):
            for key, value in node.items():
                key_str = str(key)
                lowered = key_str.strip().lower()
                next_path = path + (lowered,)
                in_bridge_scope = "bridge" in lowered or any(
                    "bridge" in segment for segment in path
                )
                if isinstance(value, str):
                    normalized = None
                    if "bridge.platform" in lowered:
                        normalized = _normalize_platform(value)
                    elif lowered == "platform" and in_bridge_scope:
                        normalized = _normalize_platform(value)
                    elif lowered.endswith(".platform") and in_bridge_scope:
                        normalized = _normalize_platform(value)
                    if normalized:
                        platforms.add(normalized)
                visit(value, next_path)
        elif isinstance(node, list):
            for item in node:
                visit(item, path)

    visit(entry, tuple())
    return platforms


def _is_nonempty_string(value: Optional[object]) -> bool:
    return isinstance(value, str) and value.strip() != ""


def primary_code_of(diag: Dict[str, Any]) -> Optional[str]:
    code = diag.get("code")
    if isinstance(code, str) and code:
        return code
    codes = diag.get("codes")
    if isinstance(codes, list):
        for item in codes:
            if isinstance(item, str) and item:
                return item
    return None


def is_parser_diagnostic(diag: Dict[str, Any]) -> bool:
    domain = diag.get("domain")
    if isinstance(domain, str):
        lowered = domain.strip().lower()
        if lowered == "parser":
            return True
        if lowered != "":
            return False
    code = primary_code_of(diag)
    if isinstance(code, str) and code.startswith("parser."):
        return True
    codes = diag.get("codes")
    if isinstance(codes, list):
        for item in codes:
            if isinstance(item, str) and item.startswith("parser."):
                return True
    return False


def extract_schema_version(diag: Dict[str, Any]) -> Optional[str]:
    schema = diag.get("schema_version")
    if isinstance(schema, str) and schema:
        return schema
    extensions = diag.get("extensions")
    if isinstance(extensions, dict):
        nested = extensions.get("diagnostic.v2")
        if isinstance(nested, dict):
            schema = nested.get("schema_version")
            if isinstance(schema, str) and schema:
                return schema
    return None


def check_extension_fields(extensions: Optional[Dict]) -> List[str]:
    missing: List[str] = []
    effects = _as_dict(extensions.get("effects") if extensions else None)
    if effects is None:
        return ["extensions.effects"]

    stage = _as_dict(effects.get("stage"))
    iterator = _as_dict(effects.get("iterator"))

    if stage is None:
        missing.append("extensions.effects.stage")
    else:
        for key in REQUIRED_EFFECT_STAGE_KEYS:
            if key not in stage or stage[key] in (None, ""):
                missing.append(f"extensions.effects.stage.{key}")

    required_caps = effects.get("required_capabilities")
    if not isinstance(required_caps, list) or len(required_caps) == 0:
        missing.append("extensions.effects.required_capabilities")

    actual_caps = effects.get("actual_capabilities")
    if not isinstance(actual_caps, list) or len(actual_caps) == 0:
        missing.append("extensions.effects.actual_capabilities")
    else:
        for index, entry in enumerate(actual_caps):
            if not isinstance(entry, dict):
                missing.append("extensions.effects.actual_capabilities")
                break
            capability_name = entry.get("capability")
            if not isinstance(capability_name, str) or capability_name.strip() == "":
                missing.append(
                    f"extensions.effects.actual_capabilities[{index}].capability"
                )
                break

    if iterator is None:
        missing.append("extensions.effects.iterator")
    else:
        for key in REQUIRED_EFFECT_ITERATOR_KEYS:
            if key not in iterator or iterator[key] in (None, ""):
                missing.append(f"extensions.effects.iterator.{key}")

    capability = effects.get("capability")
    if capability in (None, ""):
        missing.append("extensions.effects.capability")

    for key in REQUIRED_EFFECT_ADDITIONAL_KEYS:
        aliases = EFFECT_ADDITIONAL_KEY_ALIASES.get(key, (key,))
        present = False
        for alias in aliases:
            if alias not in effects:
                continue
            value = effects.get(alias)
            if value in (None,) and key not in EFFECT_ALLOW_EMPTY_KEYS:
                continue
            if value in ("", []) and key not in EFFECT_ALLOW_EMPTY_KEYS:
                continue
            present = True
            break
        if not present:
            missing.append(f"extensions.effects.{key}")

    typeclass = _as_dict(extensions.get("typeclass") if extensions else None)
    if typeclass is None:
        missing.append("extensions.typeclass")
    else:
        missing.extend(_validate_typeclass_extension(typeclass, "extensions.typeclass"))

    parse_ext = _as_dict(extensions.get("parse") if extensions else None)
    if parse_ext is None:
        missing.append("extensions.parse")
    else:
        for key in REQUIRED_PARSE_KEYS:
            value = parse_ext.get(key)
            if value in (None, ""):
                missing.append(f"extensions.parse.{key}")

    return missing


def _has_path(data: Optional[Dict], dotted_key: str) -> bool:
    if data is None:
        return False
    if dotted_key in data:
        value = data.get(dotted_key)
        return value not in (None, "", [])
    current: object = data
    for part in dotted_key.split("."):
        if not isinstance(current, dict) or part not in current:
            return False
        current = current[part]
    if current in (None, "", []):
        return False
    return True


def _has_any_path(data: Optional[Dict], *paths: str) -> bool:
    for path in paths:
        if _has_path(data, path):
            return True
    return False


def _normalize_string_list(value: Any) -> Optional[List[str]]:
    if not isinstance(value, list) or len(value) == 0:
        return None
    result: List[str] = []
    for entry in value:
        if not isinstance(entry, str):
            return None
        normalized = entry.strip()
        if normalized == "":
            return None
        result.append(normalized)
    return result


def _normalize_capability_stage_entries(
    value: Any,
) -> Optional[List[Tuple[str, Optional[str]]]]:
    if not isinstance(value, list) or len(value) == 0:
        return None
    result: List[Tuple[str, Optional[str]]] = []
    for entry in value:
        if not isinstance(entry, dict):
            return None
        capability = entry.get("capability")
        if not isinstance(capability, str):
            return None
        capability_normalized = capability.strip()
        if capability_normalized == "":
            return None
        stage_value = entry.get("stage")
        if stage_value is None:
            stage_normalized: Optional[str] = None
        elif isinstance(stage_value, str):
            stripped = stage_value.strip()
            stage_normalized = stripped if stripped != "" else None
        else:
            stage_normalized = str(stage_value)
        result.append((capability_normalized, stage_normalized))
    return result


def _validate_typeclass_extension(
    typeclass: Dict[str, Any], prefix: str
) -> List[str]:
    missing: List[str] = []
    for key in REQUIRED_TYPECLASS_SCALAR_KEYS:
        value = typeclass.get(key)
        if value is None or (isinstance(value, str) and value.strip() == ""):
            missing.append(f"{prefix}.{key}")

    for key in REQUIRED_TYPECLASS_LIST_KEYS:
        value = typeclass.get(key)
        if not isinstance(value, list):
            missing.append(f"{prefix}.{key}")

    dictionary = typeclass.get("dictionary")
    if not isinstance(dictionary, dict):
        missing.append(f"{prefix}.dictionary")
    else:
        for field in ("kind", "identifier", "repr"):
            if field not in dictionary:
                missing.append(f"{prefix}.dictionary.{field}")

    graph = typeclass.get("graph")
    if not isinstance(graph, dict):
        missing.append(f"{prefix}.graph")
    elif "export_dot" not in graph:
        missing.append(f"{prefix}.graph.export_dot")

    return missing


def _normalize_nonempty_string(value: Optional[object]) -> Optional[str]:
    if isinstance(value, str):
        stripped = value.strip()
        if stripped:
            return stripped
    return None


def _validate_dictionary_payload_dict(
    dictionary: Optional[Dict[str, Any]], prefix: str
) -> List[str]:
    if not isinstance(dictionary, dict):
        return [f"{prefix}.dictionary"]
    missing: List[str] = []
    kind = _normalize_nonempty_string(dictionary.get("kind"))
    if kind is None:
        missing.append(f"{prefix}.dictionary.kind")
    elif kind.lower() == "none":
        missing.append(f"{prefix}.dictionary.kind")
    identifier = _normalize_nonempty_string(dictionary.get("identifier"))
    if identifier is None:
        missing.append(f"{prefix}.dictionary.identifier")
    repr_value = dictionary.get("repr")
    if isinstance(repr_value, str):
        repr_value = repr_value.strip()
    if repr_value in (None, "", []):
        missing.append(f"{prefix}.dictionary.repr")
    return missing


def _lookup_metadata_value(container: Optional[Dict[str, Any]], dotted_key: str):
    if not isinstance(container, dict):
        return None
    if dotted_key in container:
        return container.get(dotted_key)
    current: object = container
    for part in dotted_key.split("."):
        if isinstance(current, dict) and part in current:
            current = current[part]
        else:
            return None
    return current


def _validate_dictionary_metadata(
    metadata: Optional[Dict[str, Any]], prefix: str
) -> List[str]:
    if not isinstance(metadata, dict):
        return [f"{prefix}.typeclass.dictionary"]
    missing: List[str] = []
    kind = _lookup_metadata_value(metadata, "typeclass.dictionary.kind")
    kind_str = _normalize_nonempty_string(kind)
    if kind_str is None or kind_str.lower() == "none":
        missing.append(f"{prefix}.typeclass.dictionary.kind")
    identifier = _lookup_metadata_value(metadata, "typeclass.dictionary.identifier")
    if _normalize_nonempty_string(identifier) is None:
        missing.append(f"{prefix}.typeclass.dictionary.identifier")
    repr_value = _lookup_metadata_value(metadata, "typeclass.dictionary.repr")
    if isinstance(repr_value, str):
        repr_value = repr_value.strip()
    if repr_value in (None, "", []):
        missing.append(f"{prefix}.typeclass.dictionary.repr")
    return missing


def check_dictionary_extension_payload(extensions: Optional[Dict]) -> List[str]:
    if not isinstance(extensions, dict):
        return ["extensions.typeclass"]
    typeclass = extensions.get("typeclass")
    if not isinstance(typeclass, dict):
        return ["extensions.typeclass"]
    return _validate_dictionary_payload_dict(
        typeclass.get("dictionary"), "extensions.typeclass"
    )


def check_dictionary_audit_payload(audit: Optional[Dict]) -> List[str]:
    if not isinstance(audit, dict):
        return ["audit.metadata"]
    metadata = audit.get("metadata")
    containers: List[Tuple[str, Optional[Dict[str, Any]]]] = []
    if isinstance(metadata, dict):
        containers.append(("audit.metadata", metadata))
    containers.append(("audit", audit))
    aggregated: List[str] = []
    for prefix, container in containers:
        issues = _validate_dictionary_metadata(container, prefix)
        if not issues:
            return []
        aggregated.extend(issues)
    return sorted(set(aggregated))


def check_bridge_audit_fields(audit: Optional[Dict]) -> List[str]:
    if audit is None:
        return ["audit"] + REQUIRED_BRIDGE_AUDIT_KEYS.copy()
    missing: List[str] = []
    for key in REQUIRED_BRIDGE_AUDIT_KEYS:
        if not _has_any_path(audit, key, f"metadata.{key}"):
            missing.append(key)
    return missing


def check_bridge_extension_fields(extensions: Optional[Dict]) -> List[str]:
    missing: List[str] = []
    for key in REQUIRED_BRIDGE_EXTENSION_KEYS:
        if not _has_path(extensions, key):
            missing.append(f"extensions.{key}")
    return missing


def check_typeclass_audit_fields(metadata: Optional[Dict]) -> List[str]:
    if not isinstance(metadata, dict):
        return list(TYPECLASS_REQUIRED_AUDIT_KEYS)
    missing: List[str] = []
    for key in TYPECLASS_REQUIRED_AUDIT_KEYS:
        if not _has_path(metadata, key):
            missing.append(key)
    return missing


def check_typeclass_extension_fields(extensions: Optional[Dict]) -> List[str]:
    if not isinstance(extensions, dict):
        return ["extensions.typeclass"]
    typeclass = extensions.get("typeclass")
    if not isinstance(typeclass, dict):
        return ["extensions.typeclass"]
    return _validate_typeclass_extension(typeclass, "extensions.typeclass")


def extract_bridge_status(
    audit: Optional[Dict], extensions: Optional[Dict]
) -> Optional[str]:
    containers: List[Dict] = []
    if isinstance(audit, dict):
        containers.append(audit)
        metadata = audit.get("metadata")
        if isinstance(metadata, dict):
            containers.append(metadata)
    if isinstance(extensions, dict):
        containers.append(extensions)
    for container in containers:
        bridge = container.get("bridge")
        if isinstance(bridge, dict):
            status = bridge.get("status")
            if status is not None:
                return str(status)
    return None


def extract_bridge_field(
    audit: Optional[Dict], extensions: Optional[Dict], key: str
) -> Optional[object]:
    containers: List[Dict] = []
    if isinstance(audit, dict):
        containers.append(audit)
        metadata = audit.get("metadata")
        if isinstance(metadata, dict):
            containers.append(metadata)
    if isinstance(extensions, dict):
        containers.append(extensions)
    for container in containers:
        bridge = container.get("bridge")
        if isinstance(bridge, dict) and key in bridge:
            return bridge.get(key)
    return None


def load_index(path: Path) -> Dict[str, Any]:
    return load_json(path)


def write_index(path: Path, data: Dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(data, handle, indent=2, ensure_ascii=False)
        handle.write("\n")


def load_retention_policy(config_path: Path) -> Dict[str, int]:
    policy = DEFAULT_RETENTION_POLICY.copy()
    if not config_path.is_file() or tomllib is None:
        return policy
    try:
        with config_path.open("rb") as handle:
            data = tomllib.load(handle)
    except Exception as exc:  # pragma: no cover - defensive
        sys.stderr.write(
            f"Failed to parse retention config ({config_path}): {exc}\n"
        )
        return policy

    retain = data.get("retain")
    if isinstance(retain, dict):
        for key, value in retain.items():
            if isinstance(value, int) and value >= 0:
                policy[str(key)] = value
    return policy


def _safe_get(data: Optional[Dict[str, Any]], *keys: str) -> Optional[object]:
    current: object = data
    for key in keys:
        if not isinstance(current, dict):
            return None
        current = current.get(key)
    return current


def _safe_int(
    data: Optional[Dict[str, Any]], *keys: str, default: int = 0
) -> int:
    value = _safe_get(data, *keys)
    if isinstance(value, bool):
        return int(value)
    if isinstance(value, (int, float)):
        return int(value)
    if isinstance(value, str) and value.strip():
        try:
            return int(float(value))
        except ValueError:
            return default
    return default


def _safe_float(
    data: Optional[Dict[str, Any]], *keys: str, default: Optional[float] = None
) -> Optional[float]:
    value = _safe_get(data, *keys)
    if isinstance(value, bool):
        return float(value)
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, str) and value.strip():
        try:
            return float(value)
        except ValueError:
            return default
    return default


def _load_json_with_failure(
    path: Path, failures: List[Dict[str, object]]
) -> Optional[Dict[str, Any]]:
    if not path.is_file():
        failures.append({"file": str(path), "reason": "not_found"})
        return None
    try:
        return load_json(path)
    except ValueError as exc:
        failures.append({"file": str(path), "reason": f"parse_error: {exc}"})
        return None


def _validate_audit_diff(data: Dict[str, Any]) -> List[str]:
    required_keys = ("schema_version", "base", "target", "diagnostic", "metadata", "pass_rate")
    errors: List[str] = []
    for key in required_keys:
        if key not in data:
            errors.append(f"missing_{key}")
    if not isinstance(data.get("diagnostic"), dict):
        errors.append("diagnostic_not_object")
    else:
        diagnostic = data["diagnostic"]
        for key in ("regressions", "new", "details"):
            if key not in diagnostic:
                errors.append(f"diagnostic_missing_{key}")
    if not isinstance(data.get("metadata"), dict):
        errors.append("metadata_not_object")
    else:
        metadata = data["metadata"]
        for key in ("changed", "details"):
            if key not in metadata:
                errors.append(f"metadata_missing_{key}")
    schema_version = data.get("schema_version")
    if not isinstance(schema_version, str) or not schema_version.startswith("audit-diff.v"):
        errors.append("invalid_schema_version")
    return errors


def _load_review_diff(
    path: Path, failures: List[Dict[str, object]]
) -> Optional[Dict[str, Any]]:
    data = _load_json_with_failure(path, failures)
    if data is None:
        return None
    errors = _validate_audit_diff(data)
    if errors:
        failures.append({"file": str(path), "reason": "audit_diff_schema", "details": errors})
    return data


def _load_review_coverage(
    path: Path, failures: List[Dict[str, object]]
) -> Optional[List[Dict[str, Any]]]:
    data = _load_json_with_failure(path, failures)
    if data is None:
        return None
    if isinstance(data, list):
        entries = [entry for entry in data if isinstance(entry, dict)]
        if entries:
            return entries
        return None
    if isinstance(data, dict):
        coverage = data.get("coverage")
        if isinstance(coverage, list):
            entries = [entry for entry in coverage if isinstance(entry, dict)]
            if entries:
                return entries
        return [data]
    failures.append({"file": str(path), "reason": "unsupported_format"})
    return None


def collect_review_metrics(
    diff_paths: List[Path], coverage_paths: List[Path], dashboard_paths: List[Path]
) -> Dict[str, Any]:
    failures: List[Dict[str, object]] = []

    diff_entries: List[Dict[str, Any]] = []
    total_regressions = 0
    total_metadata_changed = 0
    total_new = 0
    pass_rate_deltas: List[float] = []
    diff_sources: List[str] = []

    for path in diff_paths:
        data = _load_review_diff(path, failures)
        if data is None:
            continue
        diff_sources.append(str(path))
        diagnostic = data.get("diagnostic")
        metadata = data.get("metadata")
        pass_rate = data.get("pass_rate")
        regressions = _safe_int(diagnostic, "regressions", default=0)
        new = _safe_int(diagnostic, "new", default=0)
        metadata_changed = _safe_int(metadata, "changed", default=0)
        delta = _safe_float(pass_rate, "delta")
        if delta is not None:
            pass_rate_deltas.append(delta)
        total_regressions += regressions
        total_new += new
        total_metadata_changed += metadata_changed
        diff_entries.append(
            {
                "path": str(path),
                "regressions": regressions,
                "new": new,
                "metadata_changed": metadata_changed,
                "pass_rate": {
                    "previous": _safe_float(pass_rate, "previous"),
                    "current": _safe_float(pass_rate, "current"),
                    "delta": delta,
                },
                "base": _safe_get(data, "base", "path"),
                "target": _safe_get(data, "target", "path"),
            }
        )

    coverage_entries: List[Dict[str, Any]] = []
    coverage_matched = 0
    coverage_total = 0
    coverage_sources: List[str] = []

    for path in coverage_paths:
        raw_entries = _load_review_coverage(path, failures)
        if raw_entries is None:
            continue
        coverage_sources.append(str(path))
        for entry in raw_entries:
            preset = (
                entry.get("preset")
                or entry.get("name")
                or entry.get("id")
                or entry.get("query")
                or "<unknown>"
            )
            matched = _safe_int(entry, "matched", default=entry.get("hits", 0) or 0)
            total = _safe_int(entry, "total", default=entry.get("count", 0) or 0)
            ratio: Optional[float] = None
            if total > 0:
                ratio = matched / total
            coverage_matched += matched
            coverage_total += total
            coverage_entries.append(
                {
                    "preset": preset,
                    "matched": matched,
                    "total": total,
                    "ratio": ratio,
                }
            )

    dashboard_sources: List[str] = []
    missing_dashboards: List[str] = []
    dashboard_generated = 0
    for path in dashboard_paths:
        dashboard_sources.append(str(path))
        if path.is_file():
            dashboard_generated += 1
        else:
            missing_dashboards.append(str(path))

    audit_diff_regressions = total_regressions + total_metadata_changed
    coverage_ratio: Optional[float] = None
    if coverage_total > 0:
        coverage_ratio = coverage_matched / coverage_total

    review_metrics: Dict[str, Any] = {
        "metric": "audit_review.summary",
        "audit_diff": {
            "regressions": total_regressions,
            "metadata_changed": total_metadata_changed,
            "new": total_new,
            "pass_rate": {
                "delta": pass_rate_deltas[-1] if pass_rate_deltas else None,
                "min_delta": min(pass_rate_deltas) if pass_rate_deltas else None,
                "max_delta": max(pass_rate_deltas) if pass_rate_deltas else None,
            },
            "sources": diff_sources,
            "entries": diff_entries,
            "total_regressions": audit_diff_regressions,
        },
        "audit_query": {
            "coverage": coverage_ratio,
            "matched": coverage_matched,
            "total": coverage_total,
            "entries": coverage_entries,
            "sources": coverage_sources,
        },
        "audit_dashboard": {
            "generated": dashboard_generated,
            "sources": dashboard_sources,
            "missing": missing_dashboards,
        },
        "failures": failures,
    }

    return review_metrics


def _entry_profile(entry: Dict[str, Any]) -> str:
    for key in ("profile", "store", "audit_store"):
        value = entry.get(key)
        if isinstance(value, str) and value:
            return value
    return "ci"


def _entry_target(entry: Dict[str, Any]) -> str:
    for key in ("target", "platform", "triple"):
        value = entry.get(key)
        if isinstance(value, str) and value:
            return value
    return "<unknown>"


def prune_index_entries(
    entries: List[Dict[str, Any]], retention: Dict[str, int]
) -> Tuple[List[Dict[str, Any]], List[Dict[str, Any]]]:
    retain_default = retention.get("default", DEFAULT_RETENTION_POLICY["default"])
    kept_reversed: List[Dict[str, Any]] = []
    pruned: List[Dict[str, Any]] = []
    counts: Dict[Tuple[str, str], int] = defaultdict(int)

    for entry in reversed(entries):
        profile = _entry_profile(entry)
        target = _entry_target(entry)
        limit = retention.get(profile, retain_default)
        key = (profile, target)
        if limit <= 0:
            pruned.append(entry)
            continue
        if counts[key] < limit:
            counts[key] += 1
            kept_reversed.append(entry)
        else:
            pruned.append(entry)

    kept_reversed.reverse()
    pruned.reverse()
    return kept_reversed, pruned


def format_pass_rate(value: Optional[object]) -> str:
    if isinstance(value, (int, float)):
        return f"{float(value):.3f}"
    return "-"


def generate_summary_markdown(index_data: Dict[str, Any]) -> str:
    entries: List[Dict[str, Any]] = []
    raw_entries = index_data.get("entries")
    if isinstance(raw_entries, list):
        entries = [entry for entry in raw_entries if isinstance(entry, dict)]

    if not entries:
        return "# 監査ログサマリー\n\nエントリが存在しません。\n"

    groups: Dict[Tuple[str, str], List[Dict[str, Any]]] = defaultdict(list)
    for entry in entries:
        groups[_entry_profile(entry), _entry_target(entry)].append(entry)

    lines: List[str] = []
    lines.append("# 監査ログサマリー")
    lines.append("")
    lines.append(f"- 総エントリ数: {len(entries)}")
    pruned = index_data.get("pruned")
    if isinstance(pruned, list):
        lines.append(f"- 既存の削除済みビルド: {len(pruned)} 件")
    lines.append("")
    lines.append(
        "| プロファイル | ターゲット | 保持件数 | 最新ビルドID | 最新 pass_rate | 詳細度 | 出力パス |"
    )
    lines.append(
        "| --- | --- | ---: | --- | --- | --- | --- |"
    )

    for (profile, target), items in sorted(groups.items()):
        latest = items[-1]
        build_id = latest.get("build_id") or latest.get("id") or "-"
        audit_level = latest.get("audit_level") or latest.get("level") or "-"
        pass_rate = format_pass_rate(latest.get("pass_rate"))
        path = latest.get("path") or latest.get("artifact_path") or "-"
        lines.append(
            f"| {profile} | {target} | {len(items)} | {build_id} | {pass_rate} | {audit_level} | `{path}` |"
        )

    return "\n".join(lines) + "\n"


def summarize_diagnostics(paths: Sequence[Path]) -> Dict[str, Any]:
    summary = {
        "total": 0,
        "error": 0,
        "warning": 0,
        "info": 0,
        "hint": 0,
        "other": 0,
        "sources": [str(path) for path in paths],
        "info_fraction": 0.0,
        "hint_fraction": 0.0,
        "info_hint_ratio": 0.0,
        "parser_total": 0,
        "parser_expected": 0,
        "parser_expected_ratio": 0.0,
        "parser_expected_tokens_avg": 0.0,
    }

    severity_aliases = {
        "error": "error",
        "err": "error",
        "warning": "warning",
        "warn": "warning",
        "info": "info",
        "information": "info",
        "note": "info",
        "hint": "hint",
    }
    severity_numeric_aliases = {
        1: "error",
        2: "warning",
        3: "info",
        4: "hint",
    }

    parser_expected_tokens_sum = 0

    expected_metric_detail: Optional[Dict[str, Any]] = None
    expected_summary: Optional[Dict[str, Any]] = None

    for path in paths:
        data = load_json(path)
        for diag in iter_diagnostics(data):
            summary["total"] += 1
            severity = diag.get("severity")
            normalized = None
            if isinstance(severity, str):
                normalized = severity_aliases.get(severity.lower())
            elif isinstance(severity, (int, float)):
                normalized = severity_numeric_aliases.get(int(severity))
            if normalized and normalized in summary:
                summary[normalized] += 1
            else:
                summary["other"] += 1
            if is_parser_diagnostic(diag):
                summary["parser_total"] += 1
                expected = diag.get("expected")
                if isinstance(expected, dict):
                    alternatives = expected.get("alternatives")
                    if isinstance(alternatives, list) and alternatives:
                        summary["parser_expected"] += 1
                        parser_expected_tokens_sum += len(alternatives)
                # treat missing expected as zero; handled in metrics collector

    total = summary["total"]
    if total > 0:
        info = summary["info"]
        hint = summary["hint"]
        summary["info_fraction"] = info / total
        summary["hint_fraction"] = hint / total
        summary["info_hint_ratio"] = (info + hint) / total
    parser_total = summary["parser_total"]
    if parser_total > 0:
        summary["parser_expected_ratio"] = summary["parser_expected"] / parser_total
        if summary["parser_expected"] > 0 and parser_expected_tokens_sum > 0:
            summary["parser_expected_tokens_avg"] = (
                parser_expected_tokens_sum / summary["parser_expected"]
            )

    return summary


def collect_diagnostic_audit_presence_metric(paths: List[Path]) -> Dict[str, Any]:
    total = 0
    passed = 0
    failures: List[Dict[str, object]] = []
    required_keys = [field.logical for field in BASIC_AUDIT_FIELDS]

    for path in paths:
        data = load_json(path)
        try:
            diagnostics_iter = iter_diagnostics(data)
        except ValueError:
            continue
        for index, diag in enumerate(diagnostics_iter):
            diag_codes = _collect_diagnostic_codes([diag])
            if diag_codes.intersection(IGNORED_BRIDGE_CODES):
                continue
            total += 1
            audit_dict = _as_dict(diag.get("audit"))
            timestamp_value = diag.get("timestamp")

            missing_fields: List[str] = []
            missing_fields.extend(check_basic_audit_fields(audit_dict))
            if not isinstance(timestamp_value, str) or not timestamp_value.strip():
                missing_fields.append("timestamp")

            if missing_fields:
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "missing": sorted(set(missing_fields)),
                        "code": primary_code_of(diag) or "unknown",
                    }
                )
            else:
                passed += 1

    pass_rate, pass_fraction = calculate_pass_rates(passed, total)

    return {
        "metric": "diagnostic.audit_presence_rate",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "required_audit_keys": required_keys + ["timestamp"],
        "sources": [str(path) for path in paths],
        "failures": failures,
    }


def collect_parser_metrics(paths: List[Path]) -> Dict[str, Any]:
    total = 0
    passed = 0
    failures: List[Dict[str, object]] = []
    schema_versions: Set[str] = set()
    token_counts: List[int] = []

    for path in paths:
        data = load_json(path)
        try:
            diagnostics_iter = iter_diagnostics(data)
        except ValueError:
            continue
        for index, diag in enumerate(diagnostics_iter):
            if not is_parser_diagnostic(diag):
                continue
            total += 1
            schema = extract_schema_version(diag)
            if schema:
                schema_versions.add(schema)
            expected = diag.get("expected")
            missing_fields: List[str] = []
            has_expected = False
            if isinstance(expected, dict):
                alternatives = expected.get("alternatives")
                if isinstance(alternatives, list) and alternatives:
                    has_expected = True
                    token_counts.append(len(alternatives))
                else:
                    missing_fields.append("expected.alternatives")
            else:
                missing_fields.append("expected")
            if has_expected:
                passed += 1
            else:
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "code": primary_code_of(diag) or "unknown",
                        "missing": sorted(set(missing_fields)) or ["expected"],
                    }
                )

    if total == 0:
        pass_fraction = 1.0
        pass_rate = 1.0
    else:
        pass_fraction = passed / total
        pass_rate = 1.0 if passed == total else 0.0
    average_tokens = (sum(token_counts) / len(token_counts)) if token_counts else 0.0
    min_tokens = min(token_counts) if token_counts else 0
    max_tokens = max(token_counts) if token_counts else 0

    metric: Dict[str, Any] = {
        "metric": "parser.expected_summary_presence",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "average_expected_tokens": average_tokens,
        "min_expected_tokens": min_tokens,
        "max_expected_tokens": max_tokens,
        "sources": [str(path) for path in paths],
        "failures": failures,
        "schema_versions": sorted(schema_versions),
        "required_expected_keys": ["expected", "expected.alternatives"],
        "status": "success" if pass_rate == 1.0 else "error",
    }

    related_status: str
    if total == 0:
        related_status = "success"
    elif passed == 0:
        related_status = "error"
    elif average_tokens <= 0.0:
        related_status = "warning"
    else:
        related_status = "success"

    metric["related_metrics"] = [
        {
            "metric": "parser.expected_tokens_per_error",
            "total": total,
            "with_expected": passed,
            "average_tokens": average_tokens,
            "min_tokens": min_tokens,
            "max_tokens": max_tokens,
            "status": related_status,
        }
    ]

    return metric


def collect_runconfig_metrics(paths: List[Path]) -> List[Dict[str, Any]]:
    switches_state: Dict[str, bool] = {
        "packrat": False,
        "left_recursion": False,
        "trace": False,
        "merge_warnings": False,
    }
    switch_samples: Dict[str, Any] = {key: None for key in switches_state}

    extensions_state: Dict[str, bool] = {
        "lex": False,
        "recover": False,
        "stream": False,
    }
    extension_samples: Dict[str, Any] = {key: None for key in extensions_state}
    stream_field_state: Dict[str, bool] = {
        "enabled": False,
        "packrat_enabled": False,
        "checkpoint": False,
        "resume_hint": False,
        "demand_min_bytes": False,
        "demand_preferred_bytes": False,
        "chunk_size": False,
        "flow.policy": False,
        "flow.backpressure.max_lag_bytes": False,
        "flow.checkpoints_closed": False,
    }
    stream_field_samples: Dict[str, Any] = {
        key: None for key in stream_field_state
    }

    lex_total = 0
    lex_shared = 0
    lex_sources: Set[str] = set()
    lex_failures: List[Dict[str, Any]] = []
    lex_profile_sample: Optional[str] = None
    lex_space_sample: Optional[int] = None
    lex_profile_counts: Dict[str, int] = defaultdict(int)

    def _mark_switch_from_dict(container: Optional[Dict]) -> None:
        if not isinstance(container, dict):
            return
        for key in switches_state:
            if switches_state[key]:
                continue
            if key in container:
                switches_state[key] = True
                if switch_samples[key] is None:
                    switch_samples[key] = container.get(key)

    def _mark_extension_from_dict(container: Optional[Dict]) -> None:
        if not isinstance(container, dict):
            return
        for key in extensions_state:
            if extensions_state[key]:
                continue
            if key in container:
                value = container.get(key)
                if value is not None:
                    extensions_state[key] = True
                    if extension_samples[key] is None:
                        extension_samples[key] = value

    def _mark_stream_fields(entry: Optional[Dict]) -> None:
        if not isinstance(entry, dict):
            return

        def _lookup_path(container: Dict[str, Any], path: str) -> Tuple[bool, Any]:
            current: Any = container
            for part in path.split("."):
                if isinstance(current, dict) and part in current:
                    current = current[part]
                else:
                    return False, None
            return True, current

        for key in stream_field_state:
            if stream_field_state[key]:
                continue
            exists, value = _lookup_path(entry, key)
            if not exists:
                continue
            if value in (None, "", [], {}):
                continue
            stream_field_state[key] = True
            if stream_field_samples[key] is None:
                stream_field_samples[key] = value

    for path in paths:
        data = load_json(path)

        run_config = _as_dict(data.get("run_config"))
        if run_config:
            _mark_switch_from_dict(_as_dict(run_config.get("switches")))
            extensions_container = run_config.get("extensions")
            if isinstance(extensions_container, dict):
                _mark_extension_from_dict(extensions_container)
                _mark_stream_fields(extensions_container.get("stream"))

            lex_total += 1
            lex_sources.add(str(path))
            if not isinstance(extensions_container, dict):
                lex_failures.append(
                    {
                        "file": str(path),
                        "reasons": ["extensions_missing"],
                        "profile": None,
                        "profile_raw": None,
                        "space_id": None,
                    }
                )
            else:
                lex_entry = extensions_container.get("lex")
                if lex_entry is None:
                    lex_failures.append(
                        {
                            "file": str(path),
                            "reasons": ["lex_extension_missing"],
                            "profile": None,
                            "profile_raw": None,
                            "space_id": None,
                        }
                    )
                elif not isinstance(lex_entry, dict):
                    lex_failures.append(
                        {
                            "file": str(path),
                            "reasons": ["lex_extension_not_object"],
                            "profile": None,
                            "profile_raw": lex_entry,
                            "space_id": None,
                        }
                    )
                else:
                    profile_raw = lex_entry.get("identifier_profile")
                    if profile_raw is None:
                        profile_raw = lex_entry.get("profile")
                    normalized_profile = _normalize_nonempty_string(profile_raw)
                    canonical_profile: Optional[str] = None
                    if normalized_profile is not None:
                        canonical_profile = normalized_profile.lower()
                        lex_profile_counts[canonical_profile] += 1
                    space_id_value = lex_entry.get("space_id")
                    reasons: List[str] = []
                    shared = False

                    if normalized_profile is None:
                        reasons.append("profile_missing")
                    else:
                        diagnostics = data.get("diagnostics")
                        diag_seen = False
                        diag_shared = False
                        matched_space_id: Optional[int] = None
                        if isinstance(diagnostics, list):
                            for diag in diagnostics:
                                if not isinstance(diag, dict):
                                    continue
                                diag_seen = True
                                audit_meta = _as_dict(diag.get("audit_metadata"))
                                metadata_profile: Optional[str] = None
                                if audit_meta:
                                    exists, value = _lookup_in_container(
                                        audit_meta, "parser.runconfig.extensions.lex"
                                    )
                                    if exists:
                                        metadata_profile = _normalize_nonempty_string(value)
                                        if metadata_profile is None and isinstance(value, dict):
                                            metadata_profile = _normalize_nonempty_string(
                                                value.get("identifier_profile")
                                            )
                                            if metadata_profile is None:
                                                metadata_profile = _normalize_nonempty_string(
                                                    value.get("profile")
                                                )

                                extensions = _as_dict(diag.get("extensions"))
                                diag_profile: Optional[str] = None
                                diag_space_id: Optional[int] = None
                                if extensions:
                                    runconfig_ext = _as_dict(extensions.get("runconfig"))
                                    if runconfig_ext:
                                        diag_extensions = _as_dict(runconfig_ext.get("extensions"))
                                        if diag_extensions:
                                            lex_diag = _as_dict(diag_extensions.get("lex"))
                                            if lex_diag:
                                                raw_profile = lex_diag.get("identifier_profile")
                                                if raw_profile is None:
                                                    raw_profile = lex_diag.get("profile")
                                                diag_profile = _normalize_nonempty_string(
                                                    raw_profile
                                                )
                                                space_value = lex_diag.get("space_id")
                                                if isinstance(space_value, (int, float)) and not isinstance(
                                                    space_value, bool
                                                ):
                                                    diag_space_id = int(space_value)
                                                elif isinstance(space_value, str):
                                                    try:
                                                        diag_space_id = int(float(space_value))
                                                    except ValueError:
                                                        diag_space_id = None

                                if (
                                    metadata_profile is not None
                                    and diag_profile is not None
                                    and metadata_profile == normalized_profile
                                    and diag_profile == normalized_profile
                                ):
                                    diag_shared = True
                                    matched_space_id = diag_space_id
                                    break

                        if not diag_seen:
                            reasons.append("diagnostics_absent")
                        elif not diag_shared:
                            reasons.append("diagnostic_profile_mismatch")
                        else:
                            shared = True
                            if lex_profile_sample is None:
                                lex_profile_sample = normalized_profile
                            if (
                                lex_space_sample is None
                                and matched_space_id is not None
                                and not isinstance(matched_space_id, bool)
                            ):
                                lex_space_sample = matched_space_id

                    if shared:
                        lex_shared += 1
                    else:
                        if not reasons:
                            reasons.append("unknown")
                        lex_failures.append(
                            {
                                "file": str(path),
                                "reasons": reasons,
                                "profile": normalized_profile,
                                "profile_raw": profile_raw,
                                "space_id": space_id_value,
                            }
                        )

        diagnostics = data.get("diagnostics")
        if isinstance(diagnostics, list):
            for diag in diagnostics:
                if not isinstance(diag, dict):
                    continue
                metadata = _as_dict(diag.get("audit_metadata"))
                if isinstance(metadata, dict):
                    for key in switches_state:
                        if switches_state[key]:
                            continue
                        exists, value = _lookup_in_container(
                            metadata, f"parser.runconfig.{key}"
                        )
                        if exists:
                            switches_state[key] = True
                            if switch_samples[key] is None:
                                switch_samples[key] = value
                    for key in extensions_state:
                        if extensions_state[key]:
                            continue
                        exists, value = _lookup_in_container(
                            metadata, f"parser.runconfig.extensions.{key}"
                        )
                        if exists and value is not None:
                            extensions_state[key] = True
                            if extension_samples[key] is None:
                                extension_samples[key] = value
                extensions = _as_dict(diag.get("extensions"))
                if isinstance(extensions, dict):
                    runconfig_extension = _as_dict(extensions.get("runconfig"))
                    if runconfig_extension:
                        _mark_switch_from_dict(runconfig_extension)
                        sub_extensions = runconfig_extension.get("extensions")
                        if isinstance(sub_extensions, dict):
                            _mark_extension_from_dict(sub_extensions)
                            _mark_stream_fields(sub_extensions.get("stream"))

    metrics: List[Dict[str, Any]] = []

    total_switches = len(switches_state)
    covered_switches = sum(1 for value in switches_state.values() if value)
    missing_switches = sorted(
        key for key, present in switches_state.items() if not present
    )
    if total_switches > 0:
        switch_pass_fraction = covered_switches / total_switches
        switch_pass_rate = 1.0 if covered_switches == total_switches else 0.0
        metrics.append(
            {
                "metric": "parser.runconfig_switch_coverage",
                "total": total_switches,
                "passed": covered_switches,
                "failed": total_switches - covered_switches,
                "missing": missing_switches,
                "pass_rate": switch_pass_rate,
                "pass_fraction": switch_pass_fraction,
                "status": "success" if switch_pass_rate == 1.0 else "error",
                "sources": [str(path) for path in paths],
                "samples": {
                    key: value
                    for key, value in switch_samples.items()
                    if value is not None
                },
            }
        )

    total_extensions = len(extensions_state)
    covered_extensions = sum(1 for value in extensions_state.values() if value)
    missing_extensions = sorted(
        key for key, present in extensions_state.items() if not present
    )
    if total_extensions > 0:
        ext_pass_fraction = covered_extensions / total_extensions
        ext_pass_rate = 1.0 if covered_extensions == total_extensions else 0.0
        metrics.append(
            {
                "metric": "parser.runconfig_extension_pass_rate",
                "total": total_extensions,
                "passed": covered_extensions,
                "failed": total_extensions - covered_extensions,
                "missing": missing_extensions,
                "pass_rate": ext_pass_rate,
                "pass_fraction": ext_pass_fraction,
                "status": "success" if ext_pass_rate == 1.0 else "error",
                "sources": [str(path) for path in paths],
                "samples": {
                    key: value
                    for key, value in extension_samples.items()
                    if value is not None
                },
            }
        )

    total_stream_fields = len(stream_field_state)
    covered_stream_fields = sum(
        1 for value in stream_field_state.values() if value
    )
    missing_stream_fields = sorted(
        key for key, present in stream_field_state.items() if not present
    )
    if total_stream_fields > 0:
        stream_pass_fraction = (
            covered_stream_fields / total_stream_fields
        )
        stream_pass_rate = (
            1.0 if covered_stream_fields == total_stream_fields else 0.0
        )
        metrics.append(
            {
                "metric": "parser.stream_extension_field_coverage",
                "total": total_stream_fields,
                "passed": covered_stream_fields,
                "failed": total_stream_fields - covered_stream_fields,
                "missing": missing_stream_fields,
                "pass_rate": stream_pass_rate,
                "pass_fraction": stream_pass_fraction,
                "status": "success" if stream_pass_rate == 1.0 else "warning",
                "sources": [str(path) for path in paths],
                "samples": {
                    key: value
                    for key, value in stream_field_samples.items()
                    if value is not None
                },
            }
        )

    if lex_total > 0:
        lex_pass_fraction = lex_shared / lex_total
        lex_pass_rate = 1.0 if lex_shared == lex_total else 0.0
        samples: Dict[str, Any] = {}
        if lex_profile_sample is not None:
            samples["profile"] = lex_profile_sample
        if lex_space_sample is not None:
            samples["space_id"] = lex_space_sample
        metrics.append(
            {
                "metric": "lexer.shared_profile_pass_rate",
                "total": lex_total,
                "passed": lex_shared,
                "failed": lex_total - lex_shared,
                "pass_rate": lex_pass_rate,
                "pass_fraction": lex_pass_fraction,
                "status": "success" if lex_pass_rate == 1.0 else "error",
                "sources": sorted(lex_sources),
                "failures": lex_failures,
                "samples": samples,
            }
        )

    unicode_count = lex_profile_counts.get("unicode", 0)
    ascii_count = lex_profile_counts.get("ascii", 0)
    other_count = max(0, lex_total - unicode_count - ascii_count)
    profile_breakdown = {
        key: value
        for key, value in sorted(lex_profile_counts.items())
        if value > 0
    }
    unicode_fraction: Optional[float] = None
    pass_fraction: Optional[float] = None
    status = "pending"
    if lex_total > 0:
        unicode_fraction = unicode_count / lex_total
        pass_fraction = unicode_fraction
        status = "success" if unicode_fraction == 1.0 else "monitoring"

    metrics.append(
        {
            "metric": "lexer.identifier_profile_unicode",
            "total": lex_total,
            "unicode": unicode_count,
            "ascii": ascii_count,
            "other": other_count,
            "pass_rate": None,
            "pass_fraction": pass_fraction,
            "unicode_fraction": unicode_fraction,
            "status": status,
            "sources": sorted(lex_sources),
            "profile_counts": profile_breakdown,
            "cli_switch": "--lex-profile",
            "cli_values": ["ascii", "unicode"],
            "expected_pass_fraction": 1.0,
        }
    )

    return metrics


def _extract_resume_lineage(container: Optional[Dict[str, Any]]) -> Optional[List[str]]:
    entry = _as_dict(container)
    if not entry:
        return None
    lineage = entry.get("resume_lineage")
    if isinstance(lineage, list) and lineage:
        result: List[str] = []
        for item in lineage:
            if isinstance(item, str):
                result.append(item)
            else:
                result.append(str(item))
        return result
    return None


def _extract_stream_flow_descriptor(
    run_config: Optional[Dict[str, Any]]
) -> Optional[Dict[str, Any]]:
    config = _as_dict(run_config)
    if not config:
        return None
    extensions = _as_dict(config.get("extensions"))
    if not extensions:
        return None
    stream_ext = _as_dict(extensions.get("stream"))
    if not stream_ext:
        return None
    flow = stream_ext.get("flow")
    if isinstance(flow, dict):
        return flow

    descriptor: Dict[str, Any] = {}
    policy = stream_ext.get("flow_policy")
    if isinstance(policy, str) and policy.strip():
        descriptor["policy"] = policy
    backpressure: Dict[str, Any] = {}
    for source_key, target_key in (
        ("flow_max_lag_bytes", "max_lag_bytes"),
        ("flow_debounce_ms", "debounce_ms"),
        ("flow_throttle_ratio", "throttle_ratio"),
    ):
        value = stream_ext.get(source_key)
        if isinstance(value, (int, float)):
            backpressure[target_key] = (
                float(value) if target_key == "throttle_ratio" else int(value)
            )
        elif isinstance(value, str) and value.strip():
            try:
                parsed = float(value) if target_key == "throttle_ratio" else int(
                    float(value)
                )
                backpressure[target_key] = parsed
            except ValueError:
                continue
    if backpressure:
        descriptor["backpressure"] = backpressure
    return descriptor if descriptor else None


def _normalize_reason(value: Optional[str]) -> Optional[str]:
    if not isinstance(value, str):
        return None
    normalized = value.strip().lower()
    if not normalized:
        return None
    prefixes = (
        "pending.",
        "parser.stream.",
        "stream.",
        "resume.",
        "demand.",
    )
    for prefix in prefixes:
        if normalized.startswith(prefix):
            normalized = normalized[len(prefix) :]
    return normalized


def _extract_resume_hint_reason(container: Optional[Dict[str, Any]]) -> Optional[str]:
    meta = _as_dict(container)
    if not meta:
        return None
    hint = _as_dict(meta.get("resume_hint"))
    if not hint:
        return None
    reason = hint.get("reason")
    if isinstance(reason, str):
        return reason
    return None


def collect_streaming_metrics(
    paths: List[Path], platform_filters: Optional[Set[str]] = None
) -> Optional[Dict[str, Any]]:
    total = 0
    passed = 0
    sources: List[str] = []
    failures: List[Dict[str, Any]] = []
    flow_total = 0
    flow_auto = 0
    flow_policies: List[str] = []
    backpressure_checks = 0
    backpressure_synced = 0
    backpressure_failures: List[Dict[str, Any]] = []
    demandhint_total = 0
    demandhint_covered = 0
    demandhint_failures: List[Dict[str, Any]] = []
    platform_filters = platform_filters or set()
    platform_counts: Dict[str, int] = defaultdict(int)
    platform_skipped: List[Dict[str, Any]] = []
    backpressure_diag_total = 0
    backpressure_diag_covered = 0
    backpressure_diag_failures: List[Dict[str, Any]] = []
    stage_mismatch_covered = 0
    stage_mismatch_failures: List[Dict[str, Any]] = []
    expected_total = 0
    expected_passed = 0
    expected_failures: List[Dict[str, Any]] = []
    expected_sources: List[str] = []

    for path in paths:
        data = load_json(path)
        run_config = _as_dict(data.get("run_config"))
        streaming_result = _as_dict(data.get("parse_result"))
        baseline = _as_dict(data.get("baseline"))
        baseline_result = (
            _as_dict(baseline.get("parse_result")) if baseline else None
        )
        streaming_enabled = _is_streaming_enabled(data)
        if streaming_enabled:
            expected_total += 1
            expected_sources.append(str(path))
            expected_ok, expected_detail = _streaming_expected_tokens_status(data)
            if expected_ok:
                expected_passed += 1
            else:
                failure_entry = {"file": str(path)}
                failure_entry.update(expected_detail)
                expected_failures.append(failure_entry)

        if not streaming_result or not baseline_result:
            continue

        streaming_diag = data.get("diagnostics")
        if not isinstance(streaming_diag, list):
            streaming_diag = []
        diag_codes = _collect_diagnostic_codes(streaming_diag)
        baseline_diag = []
        if baseline:
            base_diag = baseline.get("diagnostics")
            if isinstance(base_diag, list):
                baseline_diag = base_diag

        streaming_meta = _as_dict(data.get("stream_meta"))
        baseline_meta = (
            _as_dict(baseline.get("stream_meta")) if baseline else None
        )
        continuation_meta = _as_dict(data.get("continuation_meta"))
        baseline_continuation_meta = (
            _as_dict(baseline.get("continuation_meta")) if baseline else None
        )
        flow_descriptor = _extract_stream_flow_descriptor(data.get("run_config"))
        entry_platforms: Set[str] = set()
        if run_config:
            entry_platforms.update(_extract_bridge_platforms(run_config))
        entry_platforms.update(_extract_bridge_platforms(data))
        if streaming_meta:
            entry_platforms.update(_extract_bridge_platforms(streaming_meta))
        if continuation_meta:
            entry_platforms.update(_extract_bridge_platforms(continuation_meta))
        if baseline:
            entry_platforms.update(_extract_bridge_platforms(baseline))
        if platform_filters:
            if not entry_platforms:
                platform_skipped.append(
                    {"file": str(path), "reason": "unspecified", "filters": sorted(platform_filters)}
                )
                continue
            if not entry_platforms.intersection(platform_filters):
                platform_skipped.append(
                    {
                        "file": str(path),
                        "reason": "filtered",
                        "platforms": sorted(entry_platforms),
                        "filters": sorted(platform_filters),
                    }
                )
                continue

        sources.append(str(path))
        total += 1
        for platform in entry_platforms:
            platform_counts[platform] += 1

        flow_policy_normalized: Optional[str] = None
        if flow_descriptor:
            raw_policy = flow_descriptor.get("policy")
            if isinstance(raw_policy, str) and raw_policy.strip():
                flow_policy_normalized = raw_policy.strip().lower()
                flow_total += 1
                flow_policies.append(flow_policy_normalized)
                if flow_policy_normalized == "auto":
                    flow_auto += 1

        parse_match = streaming_result == baseline_result
        filtered_streaming_diag = _filter_bridge_diagnostics(streaming_diag)
        filtered_baseline_diag = _filter_bridge_diagnostics(baseline_diag)
        diagnostics_match = filtered_streaming_diag == filtered_baseline_diag
        meta_match = True
        if baseline_meta is not None:
            meta_match = streaming_meta == baseline_meta

        if continuation_meta is not None:
            demandhint_total += 1
            resume_hint = _as_dict(continuation_meta.get("resume_hint"))
            min_bytes = resume_hint.get("min_bytes") if resume_hint else None
            preferred_bytes = (
                resume_hint.get("preferred_bytes") if resume_hint else None
            )
            if (
                isinstance(min_bytes, int)
                and isinstance(preferred_bytes, int)
                and preferred_bytes >= min_bytes
            ):
                demandhint_covered += 1
            else:
                demandhint_failures.append(
                    {
                        "file": str(path),
                        "resume_hint": resume_hint,
                    }
                )

        if flow_policy_normalized == "auto":
            backpressure_checks += 1
            resume_reason_raw = _extract_resume_hint_reason(continuation_meta)
            stream_reason_raw = None
            if streaming_meta:
                stream_reason_raw = streaming_meta.get("last_reason")
            resume_reason = _normalize_reason(resume_reason_raw)
            stream_reason = _normalize_reason(stream_reason_raw)
            if resume_reason == "backpressure" or stream_reason == "backpressure":
                backpressure_synced += 1
                backpressure_diag_total += 1
                if "bridge.stage.backpressure" in diag_codes:
                    backpressure_diag_covered += 1
                else:
                    backpressure_diag_failures.append(
                        {
                            "file": str(path),
                            "resume_reason": resume_reason_raw,
                            "stream_reason": stream_reason_raw,
                            "diagnostic_codes": sorted(diag_codes),
                        }
                    )
                if "effects.contract.stage_mismatch" in diag_codes:
                    stage_mismatch_covered += 1
                else:
                    stage_mismatch_failures.append(
                        {
                            "file": str(path),
                            "resume_reason": resume_reason_raw,
                            "stream_reason": stream_reason_raw,
                            "diagnostic_codes": sorted(diag_codes),
                        }
                    )
            else:
                backpressure_failures.append(
                    {
                        "file": str(path),
                        "resume_reason": resume_reason_raw,
                        "stream_reason": stream_reason_raw,
                    }
                )

        if parse_match and diagnostics_match and meta_match:
            passed += 1
            continue

        failures.append(
            {
                "file": str(path),
                "parse_result_match": parse_match,
                "diagnostics_match": diagnostics_match,
                "stream_meta_match": meta_match,
                "resume_lineage": _extract_resume_lineage(
                    continuation_meta
                )
                or _extract_resume_lineage(baseline_continuation_meta),
            }
        )

    if total == 0 and expected_total == 0:
        return None

    if total > 0:
        pass_fraction = passed / total
        status = "success" if pass_fraction == 1.0 else "error"
    else:
        pass_fraction = 1.0
        status = "success"
    related_metrics: List[Dict[str, Any]] = []
    if flow_total > 0:
        auto_fraction = flow_auto / flow_total
        flow_status = "success" if flow_auto == flow_total else "warning"
        related_metrics.append(
            {
                "metric": "parser.stream.flow.auto_coverage",
                "total": flow_total,
                "auto": flow_auto,
                "manual": flow_total - flow_auto,
                "pass_rate": 1.0
                if flow_auto == flow_total
                else auto_fraction,
                "pass_fraction": auto_fraction,
                "status": flow_status,
                "sources": sources,
                "samples": {"policies": flow_policies},
            }
        )
    if backpressure_checks > 0:
        sync_fraction = (
            backpressure_synced / backpressure_checks
            if backpressure_checks > 0
            else 0.0
        )
        sync_status = (
            "success" if backpressure_synced == backpressure_checks else "error"
        )
        related_metrics.append(
            {
                "metric": "parser.stream.backpressure_sync",
                "total": backpressure_checks,
                "passed": backpressure_synced,
                "failed": backpressure_checks - backpressure_synced,
                "pass_rate": 1.0
                if backpressure_synced == backpressure_checks
                else sync_fraction,
                "pass_fraction": sync_fraction,
                "status": sync_status,
                "sources": sources,
                "failures": backpressure_failures,
            }
        )
    if demandhint_total > 0:
        coverage_fraction = demandhint_covered / demandhint_total
        coverage_status = (
            "success" if demandhint_covered == demandhint_total else "error"
        )
        related_metrics.append(
            {
                "metric": "parser.stream.demandhint_coverage",
                "total": demandhint_total,
                "covered": demandhint_covered,
                "missing": demandhint_total - demandhint_covered,
                "pass_rate": 1.0
                if demandhint_covered == demandhint_total
                else coverage_fraction,
                "pass_fraction": coverage_fraction,
                "status": coverage_status,
                "sources": sources,
                "failures": demandhint_failures,
            }
        )

    if demandhint_total > 0:
        coverage_fraction = demandhint_covered / demandhint_total
        coverage_status = "success" if demandhint_covered == demandhint_total else "error"
        related_metrics.append(
            {
                "metric": "parser.stream.demandhint_coverage",
                "total": demandhint_total,
                "covered": demandhint_covered,
                "missing": demandhint_total - demandhint_covered,
                "pass_rate": 1.0
                if demandhint_covered == demandhint_total
                else coverage_fraction,
                "pass_fraction": coverage_fraction,
                "status": coverage_status,
                "sources": sources,
                "failures": demandhint_failures,
            }
        )

    if backpressure_diag_total > 0:
        diag_fraction = backpressure_diag_covered / backpressure_diag_total
        diag_status = (
            "success"
            if backpressure_diag_covered == backpressure_diag_total
            else "error"
        )
        related_metrics.append(
            {
                "metric": "parser.stream.bridge_backpressure_diagnostics",
                "total": backpressure_diag_total,
                "covered": backpressure_diag_covered,
                "missing": backpressure_diag_total - backpressure_diag_covered,
                "pass_rate": 1.0
                if backpressure_diag_covered == backpressure_diag_total
                else diag_fraction,
                "pass_fraction": diag_fraction,
                "status": diag_status,
                "sources": sources,
                "failures": backpressure_diag_failures,
            }
        )
        stage_fraction = (
            stage_mismatch_covered / backpressure_diag_total
            if backpressure_diag_total > 0
            else 0.0
        )
        stage_status = (
            "success" if stage_mismatch_covered == backpressure_diag_total else "warning"
        )
        related_metrics.append(
            {
                "metric": "parser.stream.bridge_stage_propagation",
                "total": backpressure_diag_total,
                "covered": stage_mismatch_covered,
                "missing": backpressure_diag_total - stage_mismatch_covered,
                "pass_rate": 1.0
                if stage_mismatch_covered == backpressure_diag_total
                else stage_fraction,
                "pass_fraction": stage_fraction,
                "status": stage_status,
                "sources": sources,
                "failures": stage_mismatch_failures,
            }
        )
    if expected_total > 0:
        expected_fraction = expected_passed / expected_total
        expected_status = (
            "success" if expected_passed == expected_total else "error"
        )
        expected_metric = {
            "metric": "ExpectedTokenCollector.streaming",
            "total": expected_total,
            "passed": expected_passed,
            "failed": expected_total - expected_passed,
            "pass_rate": 1.0
            if expected_passed == expected_total
            else expected_fraction,
            "pass_fraction": expected_fraction,
            "status": expected_status,
            "sources": expected_sources if expected_sources else sources,
            "failures": expected_failures,
        }
        related_metrics.append(expected_metric)
        expected_metric_detail = expected_metric
        result_expected_summary = {
            "total": expected_total,
            "passed": expected_passed,
            "failed": expected_total - expected_passed,
            "pass_rate": expected_metric["pass_rate"],
            "pass_fraction": expected_fraction,
            "status": expected_status,
        }
        if expected_failures:
            result_expected_summary["failures"] = expected_failures
        expected_summary = result_expected_summary

    result = {
        "metric": "parser.stream.outcome_consistency",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": 1.0 if pass_fraction == 1.0 else pass_fraction,
        "pass_fraction": pass_fraction,
        "status": status,
        "sources": sources,
        "failures": failures,
    }
    if platform_counts:
        result["platform_counts"] = dict(sorted(platform_counts.items()))
    if platform_filters:
        result["platform_filters"] = sorted(platform_filters)
        if platform_skipped:
            result["platform_skipped"] = platform_skipped
    if related_metrics:
        result["related_metrics"] = related_metrics
    if expected_metric_detail:
        result["expected_tokens_metric"] = expected_metric_detail
    if expected_summary:
        result["expected_tokens"] = expected_summary
    return result


def collect_effect_contract_metrics(paths: List[Path]) -> Optional[Dict[str, Any]]:
    required_codes = [
        "effects.contract.stage_mismatch",
        "effects.contract.capability_missing",
        "effects.contract.ownership",
    ]
    total = 0
    passed = 0
    sources: List[str] = []
    failures: List[Dict[str, Any]] = []
    code_counts: Dict[str, int] = {code: 0 for code in required_codes}
    schema_versions: Set[str] = set()

    for path in paths:
        data = load_json(path)
        diagnostics = data.get("diagnostics")
        if not isinstance(diagnostics, list):
            continue
        diag_codes = _collect_diagnostic_codes(diagnostics)
        total += 1
        sources.append(str(path))
        for code in diag_codes:
            if code in code_counts:
                code_counts[code] += 1
        missing = [code for code in required_codes if code not in diag_codes]
        if missing:
            failures.append(
                {"file": str(path), "missing": missing, "diag_codes": sorted(diag_codes)}
            )
        else:
            passed += 1
        for diag in diagnostics:
            schema = extract_schema_version(diag)
            if schema:
                schema_versions.add(schema)

    if total == 0:
        return None

    pass_rate = passed / total if total else None
    return {
        "metric": "effects-contract",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_rate,
        "metrics_case": "effects-contract",
        "required_codes": required_codes,
        "code_counts": code_counts,
        "failures": failures,
        "sources": sources,
        "schema_versions": sorted(schema_versions),
    }


def collect_diag_metrics(
    paths: List[Path], metrics_case: Optional[str]
) -> Optional[Dict[str, Any]]:
    if not metrics_case:
        return None
    normalized = metrics_case.strip().lower()
    if normalized == "effects-contract":
        return collect_effect_contract_metrics(paths)
    return None


def _core_rule_metadata_missing(diag: Dict[str, Any]) -> List[str]:
    missing: List[str] = []
    extensions = _as_dict(diag.get("extensions"))
    parse_ext = _as_dict(extensions.get("parse")) if extensions else None
    if parse_ext is None:
        missing.append("extensions.parse")
    else:
        parser_id = _as_dict(parse_ext.get("parser_id"))
        if parser_id is None:
            missing.append("extensions.parse.parser_id")
        else:
            for key in ("namespace", "name", "origin", "fingerprint"):
                if not _is_nonempty_string(parser_id.get(key)):
                    missing.append(f"extensions.parse.parser_id.{key}")
            ordinal = parser_id.get("ordinal")
            if not isinstance(ordinal, (int, float)):
                missing.append("extensions.parse.parser_id.ordinal")

    audit_metadata = _as_dict(diag.get("audit_metadata"))
    if audit_metadata is None:
        missing.append("audit_metadata")
    else:
        for key in ("namespace", "name", "origin", "fingerprint"):
            value = audit_metadata.get(f"parser.core.rule.{key}")
            if not _is_nonempty_string(value):
                missing.append(f"audit_metadata.parser.core.rule.{key}")
        ordinal_meta = audit_metadata.get("parser.core.rule.ordinal")
        if not isinstance(ordinal_meta, (int, float)):
            missing.append("audit_metadata.parser.core.rule.ordinal")

    audit_block = _as_dict(diag.get("audit"))
    audit_meta = _as_dict(audit_block.get("metadata")) if audit_block else None
    if audit_meta is None:
        missing.append("audit.metadata")
    else:
        for key in ("namespace", "name", "origin", "fingerprint"):
            value = audit_meta.get(f"parser.core.rule.{key}")
            if not _is_nonempty_string(value):
                missing.append(f"audit.metadata.parser.core.rule.{key}")
        ordinal_meta = audit_meta.get("parser.core.rule.ordinal")
        if not isinstance(ordinal_meta, (int, float)):
            missing.append("audit.metadata.parser.core.rule.ordinal")

    return missing


def collect_core_parser_metrics(paths: List[Path]) -> List[Dict[str, Any]]:
    total = 0
    covered = 0
    failures: List[Dict[str, Any]] = []
    sources: Set[str] = set()

    packrat_queries = 0
    packrat_hits = 0
    packrat_sources: Set[str] = set()
    packrat_anomalies: List[Dict[str, Any]] = []

    for path in paths:
        data = load_json(path)
        diagnostics = data.get("diagnostics")
        if isinstance(diagnostics, list):
            for index, diag in enumerate(diagnostics):
                if not isinstance(diag, dict):
                    continue
                if not is_parser_diagnostic(diag):
                    continue
                sources.add(str(path))
                total += 1
                missing = _core_rule_metadata_missing(diag)
                if missing:
                    failures.append(
                        {
                            "file": str(path),
                            "index": index,
                            "missing": sorted(set(missing)),
                            "code": primary_code_of(diag) or "unknown",
                        }
                    )
                else:
                    covered += 1

        parse_result = _as_dict(data.get("parse_result"))
        if parse_result is not None:
            stats = _as_dict(parse_result.get("packrat_stats"))
            if stats is not None:
                queries = _safe_int(stats, "queries")
                hits = _safe_int(stats, "hits")
                if queries > 0 or hits > 0:
                    packrat_sources.add(str(path))
                packrat_queries += max(0, queries)
                packrat_hits += max(0, hits)
                if hits > queries:
                    packrat_anomalies.append(
                        {
                            "file": str(path),
                            "hits": hits,
                            "queries": queries,
                        }
                    )

    metrics: List[Dict[str, Any]] = []
    if total > 0:
        pass_fraction = covered / total
        pass_rate = 1.0 if covered == total else 0.0
        metrics.append(
            {
                "metric": "parser.core_comb_rule_coverage",
                "total": total,
                "covered": covered,
                "missed": total - covered,
                "pass_rate": pass_rate,
                "pass_fraction": pass_fraction,
                "status": "success" if pass_rate == 1.0 else "error",
                "sources": sorted(sources),
                "failures": failures,
            }
        )

    if packrat_sources:
        hit_ratio = (packrat_hits / packrat_queries) if packrat_queries > 0 else None
        status = "success" if hit_ratio is not None else "warning"
        metrics.append(
            {
                "metric": "parser.packrat_cache_hit_ratio",
                "queries": packrat_queries,
                "hits": packrat_hits,
                "misses": max(0, packrat_queries - packrat_hits),
                "hit_ratio": hit_ratio,
                "status": status,
                "sources": sorted(packrat_sources),
                "anomalies": packrat_anomalies,
            }
        )

    return metrics


def collect_metrics(paths: List[Path]) -> Dict:
    total = 0
    passed = 0
    failures: List[Dict[str, object]] = []
    schema_versions: Set[str] = set()

    for path in paths:
        data = load_json(path)
        try:
            diagnostics_iter = iter_diagnostics(data)
        except ValueError:
            continue
        for index, diag in enumerate(diagnostics_iter):
            code = primary_code_of(diag) or ""
            codes_field = diag.get("codes") if isinstance(diag.get("codes"), list) else []
            target_present = (
                code == "typeclass.iterator.stage_mismatch"
                or (isinstance(codes_field, list) and "typeclass.iterator.stage_mismatch" in codes_field)
            )
            if not target_present:
                continue
            total += 1
            schema = extract_schema_version(diag)
            if schema:
                schema_versions.add(schema)

            audit_missing = check_audit_fields(_as_dict(diag.get("audit")))
            extensions_missing = check_extension_fields(_as_dict(diag.get("extensions")))
            timestamp_value = diag.get("timestamp")
            timestamp_missing: List[str] = []
            if not isinstance(timestamp_value, str) or not timestamp_value.strip():
                timestamp_missing.append("timestamp")

            missing = audit_missing + extensions_missing + timestamp_missing
            if not missing:
                passed += 1
            else:
                sys.stderr.write(
                    "[collect-iterator-audit-metrics] "
                    f"{path}:{index} missing {', '.join(sorted(set(missing)))}\n"
                )
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "code": code or "unknown",
                        "missing": sorted(set(missing)),
                    }
                )

    pass_rate, pass_fraction = calculate_pass_rates(passed, total)

    return {
        "metric": "iterator.stage.audit_pass_rate",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "required_audit_keys": [field.logical for field in REQUIRED_AUDIT_FIELDS],
        "sources": [str(path) for path in paths],
        "failures": failures,
        "schema_versions": sorted(schema_versions),
    }


def collect_capability_array_metric(
    paths: Sequence[Path],
) -> Optional[Dict[str, Any]]:
    total = 0
    passed = 0
    failures: List[Dict[str, Any]] = []
    schema_versions: Set[str] = set()
    source_paths: Set[str] = set()

    for path in paths:
        data = load_json(path)
        try:
            diagnostics_iter = iter_diagnostics(data)
        except ValueError:
            continue
        for index, diag in enumerate(diagnostics_iter):
            code = primary_code_of(diag) or ""
            codes_field = (
                diag.get("codes") if isinstance(diag.get("codes"), list) else []
            )
            target_present = (
                code == "typeclass.iterator.stage_mismatch"
                or (
                    isinstance(codes_field, list)
                    and "typeclass.iterator.stage_mismatch" in codes_field
                )
            )
            if not target_present:
                continue
            total += 1
            source_paths.add(str(path))
            schema = extract_schema_version(diag)
            if schema:
                schema_versions.add(schema)

            extensions = _as_dict(diag.get("extensions"))
            effects = _as_dict(extensions.get("effects") if extensions else None)
            capability_ext = (
                _as_dict(extensions.get("capability")) if extensions else None
            )
            audit_metadata = _as_dict(diag.get("audit_metadata"))
            audit_envelope = _as_dict(
                _as_dict(diag.get("audit")).get("metadata")
                if isinstance(diag.get("audit"), dict)
                else None
            )

            expected_required = (
                _normalize_string_list(effects.get("required_capabilities"))
                if effects
                else None
            )
            expected_actual = (
                _normalize_capability_stage_entries(
                    effects.get("actual_capabilities")
                )
                if effects
                else None
            )

            mismatches: List[str] = []
            if not expected_required:
                mismatches.append("extensions.effects.required_capabilities")
            if expected_actual is None or len(expected_actual) == 0:
                mismatches.append("extensions.effects.actual_capabilities")

            if not mismatches:
                required_checks = [
                    (
                        "extensions.capability.ids",
                        capability_ext.get("ids") if capability_ext else None,
                    ),
                    (
                        "extensions.capability.required",
                        capability_ext.get("required") if capability_ext else None,
                    ),
                    (
                        "audit_metadata.effect.required_capabilities",
                        audit_metadata.get("effect.required_capabilities")
                        if audit_metadata
                        else None,
                    ),
                    (
                        "audit_metadata.effect.stage.required_capabilities",
                        audit_metadata.get("effect.stage.required_capabilities")
                        if audit_metadata
                        else None,
                    ),
                    (
                        "audit_metadata.capability.ids",
                        audit_metadata.get("capability.ids")
                        if audit_metadata
                        else None,
                    ),
                    (
                        "audit.metadata.effect.required_capabilities",
                        audit_envelope.get("effect.required_capabilities")
                        if audit_envelope
                        else None,
                    ),
                    (
                        "audit.metadata.effect.stage.required_capabilities",
                        audit_envelope.get("effect.stage.required_capabilities")
                        if audit_envelope
                        else None,
                    ),
                    (
                        "audit.metadata.capability.ids",
                        audit_envelope.get("capability.ids")
                        if audit_envelope
                        else None,
                    ),
                ]
                for label, value in required_checks:
                    candidate = _normalize_string_list(value)
                    if candidate is None or candidate != expected_required:
                        mismatches.append(label)

                actual_checks = [
                    (
                        "extensions.capability.actual",
                        capability_ext.get("actual") if capability_ext else None,
                    ),
                    (
                        "extensions.capability.detail",
                        capability_ext.get("detail") if capability_ext else None,
                    ),
                    (
                        "audit_metadata.effect.actual_capabilities",
                        audit_metadata.get("effect.actual_capabilities")
                        if audit_metadata
                        else None,
                    ),
                    (
                        "audit_metadata.effect.stage.actual_capabilities",
                        audit_metadata.get("effect.stage.actual_capabilities")
                        if audit_metadata
                        else None,
                    ),
                    (
                        "audit_metadata.effect.stage.capabilities",
                        audit_metadata.get("effect.stage.capabilities")
                        if audit_metadata
                        else None,
                    ),
                    (
                        "audit.metadata.effect.actual_capabilities",
                        audit_envelope.get("effect.actual_capabilities")
                        if audit_envelope
                        else None,
                    ),
                    (
                        "audit.metadata.effect.stage.actual_capabilities",
                        audit_envelope.get("effect.stage.actual_capabilities")
                        if audit_envelope
                        else None,
                    ),
                    (
                        "audit.metadata.effect.stage.capabilities",
                        audit_envelope.get("effect.stage.capabilities")
                        if audit_envelope
                        else None,
                    ),
                ]
                for label, value in actual_checks:
                    candidate = _normalize_capability_stage_entries(value)
                    if candidate is None or candidate != expected_actual:
                        mismatches.append(label)

            if mismatches:
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "code": code or "unknown",
                        "mismatch": sorted(set(mismatches)),
                    }
                )
            else:
                passed += 1

    if total == 0:
        return None


def collect_collector_effect_metrics(
    paths: Sequence[Path],
) -> Optional[Dict[str, Any]]:
    total = 0
    schema_versions: Set[str] = set()
    stage_counts: Dict[str, int] = defaultdict(int)
    stage_required_counts: Dict[str, int] = defaultdict(int)
    stage_mode_counts: Dict[str, int] = defaultdict(int)
    stage_source_counts: Dict[str, int] = defaultdict(int)
    capability_counts: Dict[str, int] = defaultdict(int)
    kind_counts: Dict[str, int] = defaultdict(int)
    effect_flags: Dict[str, int] = {
        name: 0 for name in ["mem", "mut", "debug", "async_pending", "audit"]
    }
    marker_totals: Dict[str, int] = {
        "mem_reservation": 0,
        "reserve": 0,
        "finish": 0,
        "mem_bytes": 0,
    }
    stage_mismatch = 0
    error_counts: Dict[str, int] = defaultdict(int)
    error_details: List[Dict[str, Any]] = []
    cases: List[Dict[str, Any]] = []

    def _coerce_int(value: Any) -> int:
        if isinstance(value, bool):
            return int(value)
        if isinstance(value, (int, float)):
            return int(value)
        if isinstance(value, str):
            stripped = value.strip()
            if stripped.isdigit():
                return int(stripped)
        return 0

    for path in paths:
        data = load_json(path)
        try:
            diagnostics_iter = list(iter_diagnostics(data))
        except ValueError:
            continue
        for index, diag in enumerate(diagnostics_iter):
            extensions = _as_dict(diag.get("extensions"))
            prelude = (
                _as_dict(extensions.get("prelude.collector"))
                if extensions
                else None
            )
            if not prelude:
                continue

            audit = _as_dict(diag.get("audit"))
            metadata = _as_dict(audit.get("metadata")) if audit else None

            total += 1
            schema = extract_schema_version(diag)
            if schema:
                schema_versions.add(schema)

            case_id = (
                diag.get("snapshot_id")
                or prelude.get("snapshot_id")
                or diag.get("case_id")
                or f"{path.name}#{index}"
            )

            actual_stage = prelude.get("stage_actual") or prelude.get("stage")
            required_stage = prelude.get("stage_required")
            stage_mode = prelude.get("stage_mode")
            kind = prelude.get("kind")
            capability = prelude.get("capability")
            stage_source = prelude.get("source")
            stage_counts[(actual_stage or "unknown")] += 1
            if required_stage:
                stage_required_counts[required_stage] += 1
            if stage_mode:
                stage_mode_counts[stage_mode] += 1
            if stage_source:
                stage_source_counts[stage_source] += 1
            if capability:
                capability_counts[capability] += 1
            if kind:
                kind_counts[kind] += 1

            mismatch_flag = bool(prelude.get("stage_mismatch"))
            if metadata:
                exists, meta_value = _lookup_in_container(
                    metadata, "collector.stage.mismatch"
                )
                if exists:
                    mismatch_flag = _coerce_bool(meta_value)
            if mismatch_flag:
                stage_mismatch += 1

            case_entry: Dict[str, Any] = {
                "id": case_id,
                "kind": kind,
                "capability": capability,
                "stage_actual": actual_stage,
                "stage_required": required_stage,
                "stage_mode": stage_mode,
                "stage_mismatch": mismatch_flag,
                "effects": {},
                "markers": {},
            }

            effects = _as_dict(prelude.get("effects"))
            for effect_name in effect_flags.keys():
                value: Any = None
                if metadata:
                    exists, meta_value = _lookup_in_container(
                        metadata, f"collector.effect.{effect_name}"
                    )
                    if exists:
                        value = meta_value
                if value is None and effects:
                    value = effects.get(effect_name)
                present = _coerce_bool(value)
                case_entry["effects"][effect_name] = present
                if present:
                    effect_flags[effect_name] += 1

            markers = _as_dict(prelude.get("markers"))
            for marker_name in marker_totals.keys():
                marker_value: Any = None
                if metadata:
                    exists, meta_value = _lookup_in_container(
                        metadata, f"collector.effect.{marker_name}"
                    )
                    if exists:
                        marker_value = meta_value
                if marker_value is None and markers:
                    marker_value = markers.get(marker_name)
                numeric_value = _coerce_int(marker_value)
                case_entry["markers"][marker_name] = numeric_value
                marker_totals[marker_name] += numeric_value

            error_kind = prelude.get("error_kind")
            if not error_kind and metadata:
                exists, meta_value = _lookup_in_container(
                    metadata, "collector.error.kind"
                )
                if exists and isinstance(meta_value, str):
                    error_kind = meta_value
            error_key = prelude.get("error_key")
            if not error_key and metadata:
                exists, meta_value = _lookup_in_container(
                    metadata, "collector.error.key"
                )
                if exists and isinstance(meta_value, str):
                    error_key = meta_value
            if error_kind:
                error_counts[error_kind] += 1
                error_details.append(
                    {
                        "case": case_id,
                        "kind": error_kind,
                        "key": error_key,
                    }
                )
                case_entry["error_kind"] = error_kind
                if error_key:
                    case_entry["error_key"] = error_key
            cases.append(case_entry)

    if total == 0:
        return None

    passed = total - stage_mismatch
    pass_rate, pass_fraction = calculate_pass_rates(passed, total)
    error_total = sum(error_counts.values())
    error_rate_total = (error_total / total) if total else None
    effect_rates = {
        key: (value / total) if total else None
        for key, value in effect_flags.items()
    }
    error_rate_by_kind: Dict[str, float] = {}
    if total:
        error_rate_by_kind = {
            key: value / total for key, value in error_counts.items()
        }
    error_rate_within_errors: Dict[str, float] = {}
    if error_total:
        error_rate_within_errors = {
            key: value / error_total for key, value in error_counts.items()
        }

    stage_summary: Dict[str, Any] = {
        "actual_counts": dict(stage_counts),
        "required_counts": dict(stage_required_counts),
        "mode_counts": dict(stage_mode_counts),
        "source_counts": dict(stage_source_counts),
        "capability_counts": dict(capability_counts),
        "kind_counts": dict(kind_counts),
        "mismatch": stage_mismatch,
        "mismatch_rate": (stage_mismatch / total) if total else None,
        "audit_pass_rate": pass_rate,
    }

    effects_summary: Dict[str, Any] = {
        "counts": dict(effect_flags),
        "rates": effect_rates,
        "markers": dict(marker_totals),
    }

    errors_summary: Dict[str, Any] = {
        "total": error_total,
        "rate_per_total": error_rate_total,
        "counts": dict(error_counts),
        "rate_per_total_by_kind": error_rate_by_kind,
        "rate_within_errors": error_rate_within_errors,
        "details": error_details,
    }

    return {
        "metric": "collector.effect.audit_snapshot",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "stage": stage_summary,
        "effects": effects_summary,
        "errors": errors_summary,
        "cases": cases,
        "schema_versions": sorted(schema_versions),
        "sources": [str(path) for path in paths],
        "status": "success" if pass_rate == 1.0 else "warning",
    }


def collect_vec_effect_metrics(
    paths: Sequence[Path],
) -> Optional[Dict[str, Any]]:
    total = 0
    passed = 0
    failures: List[Dict[str, Any]] = []
    schema_versions: Set[str] = set()

    for path in paths:
        data = load_json(path)
        try:
            diagnostics_iter = list(iter_diagnostics(data))
        except ValueError:
            continue
        for index, diag in enumerate(diagnostics_iter):
            extensions = _as_dict(diag.get("extensions"))
            prelude = (
                _as_dict(extensions.get("prelude.collector"))
                if extensions
                else None
            )
            if not prelude:
                continue
            if prelude.get("kind") != "vec":
                continue
            snapshot_id = prelude.get("snapshot_id")
            if snapshot_id not in {
                "collect_vec_mem_reservation",
                "collect_vec_mem_exhaustion",
            }:
                continue

            audit_entry = _as_dict(diag.get("audit"))
            metadata = (
                _as_dict(audit_entry.get("metadata")) if audit_entry else None
            )
            effects = _as_dict(prelude.get("effects"))

            total += 1
            schema = extract_schema_version(diag)
            if schema:
                schema_versions.add(schema)
            case_id = snapshot_id or f"{path.name}#{index}"

            mem_bytes_value: Optional[int] = None
            if metadata:
                exists, raw_mem_bytes = _lookup_in_container(
                    metadata, "collector.effect.mem_bytes"
                )
                if exists:
                    mem_bytes_value = _coerce_int_value(raw_mem_bytes)
            if mem_bytes_value is None and effects:
                mem_bytes_value = _coerce_int_value(effects.get("mem_bytes"))

            effect_mut = False
            if metadata:
                exists, raw_mut = _lookup_in_container(
                    metadata, "collector.effect.mut"
                )
                if exists:
                    effect_mut = _coerce_bool(raw_mut)
                elif effects:
                    effect_mut = _coerce_bool(effects.get("mut"))
            elif effects:
                effect_mut = _coerce_bool(effects.get("mut"))

            reasons: List[str] = []
            if not effect_mut:
                reasons.append("collector.effect.mut")
            if mem_bytes_value is None:
                reasons.append("collector.effect.mem_bytes.missing")
            elif mem_bytes_value <= 0:
                reasons.append("collector.effect.mem_bytes.non_positive")

            if reasons:
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "case": case_id,
                        "snapshot": snapshot_id,
                        "mut": effect_mut,
                        "mem_bytes": mem_bytes_value,
                        "reasons": sorted(set(reasons)),
                    }
                )
            else:
                passed += 1

    if total == 0:
        return None

    status = "success" if passed == total else "error"
    pass_rate, pass_fraction = calculate_pass_rates(passed, total)

    return {
        "metric": "vec.effect.mem_bytes",
        "scenario": "vec_mem_exhaustion",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "status": status,
        "failures": failures,
        "sources": [str(path) for path in paths],
        "schema_versions": sorted(schema_versions),
        "required_audit_keys": ["collector.effect.mut", "collector.effect.mem_bytes"],
    }


def collect_cell_ref_effect_metrics(paths: Sequence[Path]) -> Optional[Dict[str, Any]]:
    total = 0
    passed = 0
    cell_mutations_total = 0
    rc_ops_total = 0
    borrow_conflicts = 0
    schema_versions: Set[str] = set()
    failures: List[Dict[str, Any]] = []
    scenario_ids = {"collect_cell_ref_effects", "ref_internal_mutation"}

    for path in paths:
        data = load_json(path)
        try:
            diagnostics_iter = list(iter_diagnostics(data))
        except ValueError:
            continue
        for index, diag in enumerate(diagnostics_iter):
            extensions = _as_dict(diag.get("extensions"))
            prelude = (
                _as_dict(extensions.get("prelude.collector"))
                if extensions
                else None
            )
            if not prelude:
                continue
            snapshot_id = prelude.get("snapshot_id")
            if not snapshot_id or snapshot_id not in scenario_ids:
                continue

            total += 1
            schema = extract_schema_version(diag)
            if schema:
                schema_versions.add(schema)

            audit_entry = _as_dict(diag.get("audit"))
            metadata = _as_dict(audit_entry.get("metadata")) if audit_entry else None
            effects = _as_dict(prelude.get("effects"))
            markers = _as_dict(prelude.get("markers"))

            cell_value = False
            exists, raw_cell = (False, None)
            if metadata:
                exists, raw_cell = _lookup_in_container(
                    metadata, "collector.effect.cell"
                )
            if exists:
                cell_value = _coerce_bool(raw_cell)
            elif effects:
                cell_value = _coerce_bool(effects.get("cell"))
            elif markers:
                cell_value = _coerce_bool(markers.get("cell"))

            cell_count = 0
            if metadata:
                exists, raw_cell_count = _lookup_in_container(
                    metadata, "collector.metrics.cell_mutations_total"
                )
                if exists:
                    converted = _coerce_int_value(raw_cell_count)
                    if converted is not None:
                        cell_count = converted
            if cell_count == 0:
                cell_count = 1 if cell_value else 0
            cell_mutations_total += cell_count

            rc_value = False
            exists, raw_rc = (False, None)
            if metadata:
                exists, raw_rc = _lookup_in_container(metadata, "collector.effect.rc")
            if exists:
                rc_value = _coerce_bool(raw_rc)
            elif effects:
                rc_value = _coerce_bool(effects.get("rc"))
            elif markers:
                rc_value = _coerce_bool(markers.get("rc"))

            rc_ops_value = 0
            if metadata:
                exists, raw_rc_ops = _lookup_in_container(
                    metadata, "collector.effect.rc_ops"
                )
                if exists:
                    converted = _coerce_int_value(raw_rc_ops)
                    if converted is not None:
                        rc_ops_value = converted
            if rc_ops_value == 0 and markers:
                candidate = _coerce_int_value(markers.get("rc_ops"))
                if candidate is not None:
                    rc_ops_value = candidate
            rc_ops_total += rc_ops_value

            conflict_count = 0
            if metadata:
                exists, raw_conflict = _lookup_in_container(
                    metadata, "collector.error.borrow_conflict"
                )
                if exists:
                    converted = _coerce_int_value(raw_conflict)
                    if converted is not None:
                        conflict_count += converted
            error_kind = prelude.get("error_kind")
            if isinstance(error_kind, str) and error_kind == "borrow_conflict":
                conflict_count += 1
            borrow_conflicts += conflict_count

            reasons: List[str] = []
            if cell_count == 0:
                reasons.append("collector.effect.cell")
            if rc_ops_value <= 0:
                reasons.append("collector.effect.rc_ops")

            case_id = snapshot_id or f"{path.name}#{index}"
            if reasons:
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "case": case_id,
                        "snapshot": snapshot_id,
                        "cell": cell_value,
                        "rc_ops": rc_ops_value,
                        "reasons": sorted(set(reasons)),
                    }
                )
            else:
                passed += 1

    if total == 0:
        return None

    pass_rate, pass_fraction = calculate_pass_rates(passed, total)
    ref_rate = float(borrow_conflicts) / rc_ops_total if rc_ops_total else 0.0
    status = "success" if passed == total and cell_mutations_total > 0 else "warning"

    return {
        "metric": "collector.effect.cell_rc",
        "scenario": "ref_internal_mutation",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "status": status,
        "cell_mutations_total": cell_mutations_total,
        "rc_ops_total": rc_ops_total,
        "borrow_conflicts": borrow_conflicts,
        "ref_borrow_conflict_rate": ref_rate,
        "failures": failures,
        "sources": [str(path) for path in paths],
        "schema_versions": sorted(schema_versions),
        "required_audit_keys": ["collector.effect.cell", "collector.effect.rc"],
    }

def _change_set_contains_collections(change_set: Dict[str, Any]) -> bool:
    def _match(value: Optional[object]) -> bool:
        return isinstance(value, str) and "collections.diff" in value

    if _match(change_set.get("kind")) or _match(change_set.get("category")):
        return True
    metadata = _as_dict(change_set.get("metadata"))
    if metadata:
        for key, value in metadata.items():
            if isinstance(key, str) and key.startswith("collections.diff"):
                return True
            if _match(value):
                return True
    items = change_set.get("items")
    if isinstance(items, list):
        for item in items:
            if isinstance(item, dict) and _match(item.get("kind")):
                return True
    return False


def _validate_collections_diff_extensions(
    extensions: Optional[Dict[str, Any]],
    change_dict: Dict[str, Any],
) -> List[str]:
    if extensions is None:
        return ["extensions.collections.diff (missing)"]

    collections = _as_dict(change_dict.get("collections"))
    if not collections:
        return []

    reasons: List[str] = []

    def _expect(key: str, expected: Optional[object]) -> None:
        if expected is None:
            return
        exists, value = _lookup_in_container(extensions, key)
        if not exists:
            reasons.append(key)
        elif value != expected:
            reasons.append(f"{key}.mismatch")

    _expect("collections.diff.kind", collections.get("kind"))
    _expect("collections.diff.total", change_dict.get("total"))
    summary = _as_dict(collections.get("summary"))
    if summary:
        _expect("collections.diff.summary.total", summary.get("total"))
    metadata = _as_dict(collections.get("metadata"))
    if metadata:
        _expect(
            "collections.diff.metadata.stage", metadata.get("stage")
        )
    return reasons


def collect_collections_audit_bridge_metrics(
    paths: Sequence[Path], *, kinds: Optional[Set[str]] = None
) -> Tuple[Optional[Dict[str, Any]], Optional[Dict[str, Any]]]:
    if kinds is None:
        monitored_kinds: Optional[Set[str]] = None
    else:
        monitored_kinds = {kind.lower() for kind in kinds}

    schema_versions: Set[str] = set()
    bridge_total = 0
    bridge_passed = 0
    bridge_failures: List[Dict[str, Any]] = []

    effect_total = 0
    effect_passed = 0
    effect_failures: List[Dict[str, Any]] = []

    for path in paths:
        data = load_json(path)
        try:
            diagnostics_iter = list(iter_diagnostics(data))
        except ValueError:
            continue
        for index, diag in enumerate(diagnostics_iter):
            extensions = _as_dict(diag.get("extensions"))
            prelude = (
                _as_dict(extensions.get("prelude.collector"))
                if extensions
                else None
            )
            if not prelude:
                continue
            kind_value = prelude.get("kind")
            if monitored_kinds and str(kind_value).lower() not in monitored_kinds:
                continue
            audit_entry = _as_dict(diag.get("audit"))
            if not audit_entry:
                continue
            schema = extract_schema_version(diag)
            if schema:
                schema_versions.add(schema)
            case_id = (
                diag.get("snapshot_id")
                or prelude.get("snapshot_id")
                or f"{path.name}#{index}"
            )
            metadata = _as_dict(audit_entry.get("metadata"))
            change_set = (
                audit_entry.get("change_set")
                if isinstance(audit_entry, dict)
                else None
            )
            change_dict = change_set if isinstance(change_set, dict) else None

            bridge_total += 1
            bridge_reasons: List[str] = []
            has_collections = False
            change_total_value: Optional[int] = None

            if change_dict is None:
                bridge_reasons.append("change_set.missing")
            else:
                has_collections = _change_set_contains_collections(change_dict)
                if not has_collections:
                    bridge_reasons.append("change_set.kind")
                total_value = _coerce_int_value(change_dict.get("total"))
                if total_value is None:
                    bridge_reasons.append("change_set.total")
                else:
                    change_total_value = total_value
                items = change_dict.get("items")
                if not isinstance(items, list):
                    bridge_reasons.append("change_set.items")

                if has_collections:
                    bridge_reasons.extend(
                        _validate_collections_diff_extensions(extensions, change_dict)
                    )

            if bridge_reasons:
                bridge_failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "case": case_id,
                        "kind": kind_value,
                        "reasons": sorted(set(bridge_reasons)),
                    }
                )
            else:
                bridge_passed += 1

            effect_applicable = change_dict is not None and has_collections
            if not effect_applicable:
                continue

            effect_total += 1
            effect_reasons: List[str] = []
            expected_total = change_total_value
            effect_flag: Optional[bool] = None
            mem_bytes_value: Optional[int] = None

            if metadata:
                exists, value = _lookup_in_container(
                    metadata, "collector.effect.audit"
                )
                if exists:
                    effect_flag = _coerce_bool(value)
                exists, mem_value = _lookup_in_container(
                    metadata, "collector.effect.mem_bytes"
                )
                if exists:
                    mem_bytes_value = _coerce_int_value(mem_value)

            if expected_total is None and change_dict is not None:
                expected_total = _coerce_int_value(change_dict.get("total"))

            if effect_flag is None:
                effect_reasons.append("collector.effect.audit")
            if mem_bytes_value is None:
                effect_reasons.append("collector.effect.mem_bytes")

            expected_audit = (expected_total or 0) > 0
            if effect_flag is not None:
                if expected_audit and not effect_flag:
                    effect_reasons.append("collector.effect.audit.expected_true")
                if not expected_audit and effect_flag:
                    effect_reasons.append("collector.effect.audit.expected_false")
            if mem_bytes_value is not None:
                if expected_audit and mem_bytes_value <= 0:
                    effect_reasons.append("collector.effect.mem_bytes.positive")
                if not expected_audit and mem_bytes_value not in (0,):
                    effect_reasons.append("collector.effect.mem_bytes.zero")

            if effect_reasons:
                effect_failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "case": case_id,
                        "kind": kind_value,
                        "expected_total": expected_total,
                        "audit_flag": effect_flag,
                        "mem_bytes": mem_bytes_value,
                        "reasons": sorted(set(effect_reasons)),
                    }
                )
            else:
                effect_passed += 1

    def _build_metric(
        metric_name: str,
        total: int,
        passed: int,
        failures: List[Dict[str, Any]],
    ) -> Optional[Dict[str, Any]]:
        status = "success" if total > 0 and passed == total else "error"
        pass_rate: Optional[float]
        pass_fraction: Optional[float]
        if total > 0:
            pass_rate, pass_fraction = calculate_pass_rates(passed, total)
        else:
            pass_rate = None
            pass_fraction = None
            if not failures:
                failures.append(
                    {
                        "file": None,
                        "index": None,
                        "case": None,
                        "reasons": ["no_applicable_cases"],
                    }
                )
        if total == 0 and not failures:
            return None
        return {
            "metric": metric_name,
            "scenario": "map_set_persistent",
            "total": total,
            "passed": passed,
            "failed": total - passed,
            "pass_rate": pass_rate,
            "pass_fraction": pass_fraction,
            "failures": failures,
            "sources": [str(path) for path in paths],
            "schema_versions": sorted(schema_versions),
            "status": status,
        }

    bridge_metric = _build_metric(
        "collections.audit_bridge_pass_rate", bridge_total, bridge_passed, bridge_failures
    )
    if bridge_metric:
        bridge_metric["required_audit_keys"] = ["audit.change_set"]
        bridge_metric["kinds"] = sorted(monitored_kinds) if monitored_kinds else None

    effect_metric = _build_metric(
        "collector.effect.audit_presence", effect_total, effect_passed, effect_failures
    )
    if effect_metric:
        effect_metric["required_audit_keys"] = [
            "collector.effect.audit",
            "collector.effect.mem_bytes",
        ]

    return bridge_metric, effect_metric


def collect_text_grapheme_metrics(paths: Sequence[Path]) -> Optional[Dict[str, Any]]:
    sources: List[str] = []
    structural_failures: List[Dict[str, Any]] = []
    expectation_failures: List[Dict[str, Any]] = []
    cases_output: List[Dict[str, Any]] = []
    total_cases = 0
    total_bytes = 0
    total_evictions = 0
    generation_sum = 0.0
    summary_payload: Optional[Dict[str, Any]] = None
    ratio_hits = 0
    ratio_miss = 0

    for path in paths:
        data = _load_json_with_failure(path, structural_failures)
        if data is None:
            continue
        sources.append(str(path))
        if summary_payload is None:
            summary = data.get("summary")
            if isinstance(summary, dict):
                summary_payload = summary
        cases = data.get("cases")
        if not isinstance(cases, list):
            structural_failures.append({"file": str(path), "reason": "cases_missing"})
            continue
        for case in cases:
            if not isinstance(case, dict):
                continue
            case_id = case.get("case_id") or case.get("case") or "<unknown>"
            hits = max(0, _coerce_int_value(case.get("cache_hits")) or 0)
            miss = max(0, _coerce_int_value(case.get("cache_miss")) or 0)
            bytes_value = max(
                0,
                _coerce_int_value(case.get("actual_bytes"))
                or _coerce_int_value(case.get("target_bytes"))
                or 0,
            )
            evictions = max(
                0, _coerce_int_value(case.get("version_mismatch_evictions")) or 0
            )
            avg_generation = case.get("avg_generation")
            if isinstance(avg_generation, (int, float)):
                generation_sum += float(avg_generation)
            else:
                generation_sum += float(
                    _coerce_int_value(case.get("cache_generation")) or 0
                )

            total_cases += 1
            total_bytes += bytes_value
            total_evictions += evictions

            denominator = hits + miss
            ratio = (hits / denominator) if denominator > 0 else None
            case_rules = TEXT_CASE_RULES.get(case_id)
            case_violations: List[str] = []
            if case_rules:
                min_miss = case_rules.get("min_cache_miss")
                if min_miss is not None and miss < min_miss:
                    case_violations.append(f"cache_miss<{min_miss}")
                max_miss = case_rules.get("max_cache_miss")
                if max_miss is not None and miss > max_miss:
                    case_violations.append(f"cache_miss>{max_miss}")
                min_hits = case_rules.get("min_cache_hits")
                if min_hits is not None and hits < min_hits:
                    case_violations.append(f"cache_hits<{min_hits}")
                max_hits = case_rules.get("max_cache_hits")
                if max_hits is not None and hits > max_hits:
                    case_violations.append(f"cache_hits>{max_hits}")
                required_ratio = case_rules.get("min_hit_ratio")
                if required_ratio is not None:
                    ratio_hits += hits
                    ratio_miss += miss
                    if ratio is None or ratio < required_ratio:
                        case_violations.append(
                            f"cache_hit_ratio<{required_ratio}"
                        )
            if case_violations:
                expectation_failures.append(
                    {
                        "case": case_id,
                        "violations": case_violations,
                        "file": str(path),
                    }
                )
            cases_output.append(
                {
                    "case": case_id,
                    "cache_hits": hits,
                    "cache_miss": miss,
                    "cache_hit_ratio": ratio,
                    "bytes": bytes_value,
                    "version_mismatch_evictions": evictions,
                    "notes": case.get("notes"),
                    "expected": case_rules,
                    "violations": case_violations or None,
                }
            )

    if total_cases == 0 and not structural_failures:
        return None

    ratio_denominator = ratio_hits + ratio_miss
    cache_hit_ratio = (
        ratio_hits / ratio_denominator if ratio_denominator > 0 else None
    )

    status = "success"
    all_failures = structural_failures + expectation_failures
    if all_failures:
        status = "error"
    elif cache_hit_ratio is not None and cache_hit_ratio < TEXT_CACHE_HIT_TARGET:
        status = "error"
    elif cache_hit_ratio is None and ratio_denominator > 0:
        status = "warning"

    metric: Dict[str, Any] = {
        "metric": "text.grapheme.cache_hit",
        "status": status,
        "threshold": TEXT_CACHE_HIT_TARGET,
        "total_cases": total_cases,
        "cache_hit_ratio": cache_hit_ratio,
        "cache_hits_monitored": ratio_hits,
        "cache_miss_monitored": ratio_miss,
        "ratio_cases": ratio_denominator,
        "total_bytes": total_bytes,
        "version_mismatch_evictions": total_evictions,
        "avg_generation": (generation_sum / total_cases) if total_cases > 0 else None,
        "cases": cases_output,
        "sources": sources,
    }
    if summary_payload:
        metric["summary"] = summary_payload
    if all_failures:
        metric["failures"] = all_failures
    return metric


def collect_audit_capability_metric(paths: Sequence[Path]) -> Optional[Dict[str, Any]]:
    total = 0
    passed = 0
    failures: List[Dict[str, Any]] = []
    schema_versions: Set[str] = set()
    for path in paths:
        data = load_json(path)
        try:
            diagnostics_iter = list(iter_diagnostics(data))
        except ValueError:
            continue
        for index, diag in enumerate(diagnostics_iter):
            snapshot_id = diag.get("snapshot_id")
            if snapshot_id != "audit_cap":
                continue
            audit_entry = _as_dict(diag.get("audit"))
            extensions = _as_dict(diag.get("extensions"))
            metadata = _as_dict(audit_entry.get("metadata")) if audit_entry else None
            change_set = (
                audit_entry.get("change_set")
                if isinstance(audit_entry, dict)
                else None
            )
            change_dict = change_set if isinstance(change_set, dict) else None
            if metadata:
                schema = extract_schema_version(diag)
                if schema:
                    schema_versions.add(schema)
            reasons: List[str] = []
            capability_value = metadata.get("collector.capability") if metadata else None
            if not capability_value:
                reasons.append("collector.capability.missing")
            elif capability_value != "core.collections.audit":
                reasons.append("collector.capability.mismatch")
            effect_flag = None
            mem_bytes_value = None
            if metadata:
                exists, raw_effect = _lookup_in_container(
                    metadata, "collector.effect.audit"
                )
                if exists:
                    effect_flag = _coerce_bool(raw_effect)
                exists, raw_mem_bytes = _lookup_in_container(
                    metadata, "collector.effect.mem_bytes"
                )
                if exists:
                    mem_bytes_value = _coerce_int_value(raw_mem_bytes)
            if effect_flag is not True:
                reasons.append("collector.effect.audit.false")
            if mem_bytes_value is None or mem_bytes_value <= 0:
                reasons.append("collector.effect.mem_bytes.positive")

            has_collections = False
            change_total = None
            if change_dict is None:
                reasons.append("change_set.missing")
            else:
                has_collections = _change_set_contains_collections(change_dict)
                if not has_collections:
                    reasons.append("change_set.collections")
                change_total = _coerce_int_value(change_dict.get("total"))
                if change_total is None:
                    reasons.append("collections.change_set.total")
                elif change_total <= 0:
                    reasons.append("collections.change_set.total_positive")
                items = change_dict.get("items")
                if not isinstance(items, list):
                    reasons.append("change_set.items")
                if has_collections:
                    reasons.extend(
                        _validate_collections_diff_extensions(extensions, change_dict)
                    )
            if reasons:
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "case": snapshot_id,
                        "reasons": sorted(set(reasons)),
                        "collector.capability": capability_value,
                        "collector.effect.audit": effect_flag,
                        "collections.change_set.total": change_total,
                    }
                )
            else:
                passed += 1
            total += 1

    if total == 0:
        return None
    status = "success" if passed == total else "error"
    pass_rate, pass_fraction = calculate_pass_rates(passed, total)
    return {
        "metric": "collector.capability.audit_pass_rate",
        "scenario": "audit_cap",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "status": status,
        "failures": failures,
        "sources": [str(path) for path in paths],
        "schema_versions": sorted(schema_versions),
        "required_audit_keys": [
            "collector.capability",
            "collector.effect.audit",
            "collector.effect.mem_bytes",
            "collections.diff.kind",
            "collections.diff.total",
        ],
    }

    pass_rate, pass_fraction = calculate_pass_rates(passed, total)
    status = "success" if pass_rate == 1.0 else "error"

    return {
        "metric": "effect.capability_array_pass_rate",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "status": status,
        "failures": failures,
        "sources": sorted(source_paths),
        "schema_versions": sorted(schema_versions),
    }


def collect_value_restriction_violation_metric(
    paths: Sequence[Path],
) -> Dict[str, Any]:
    occurrences: List[Dict[str, Any]] = []
    schema_versions: Set[str] = set()

    for path in paths:
        data = load_json(path)
        diag_list = list(iter_diagnostics(data))
        for index, diag in enumerate(diag_list):
            if not _diagnostic_has_code(
                diag, "type_inference.value_restriction_violation"
            ):
                continue
            schema = extract_schema_version(diag)
            if schema:
                schema_versions.add(schema)
            occurrences.append(
                {
                    "file": str(path),
                    "index": index,
                    "code": primary_code_of(diag) or "unknown",
                    "message": diag.get("message"),
                }
            )

    violation_count = len(occurrences)
    total_checks = 1
    passed_checks = 1 if violation_count == 0 else 0
    pass_rate, pass_fraction = calculate_pass_rates(passed_checks, total_checks)
    status = "success" if passed_checks == total_checks else "error"

    return {
        "metric": "type_inference.value_restriction_violation",
        "total": total_checks,
        "passed": passed_checks,
        "failed": total_checks - passed_checks,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "status": status,
        "violation_count": violation_count,
        "violations": occurrences,
        "sources": [str(path) for path in paths],
        "schema_versions": sorted(schema_versions),
    }


def collect_value_restriction_legacy_metric(
    paths: Sequence[Path],
) -> Dict[str, Any]:
    occurrences: List[Dict[str, Any]] = []
    schema_versions: Set[str] = set()
    source_paths: Set[str] = set()

    for path in paths:
        data = load_json(path)
        diag_list = list(iter_diagnostics(data))
        for index, diag in enumerate(diag_list):
            if not _diagnostic_has_code(
                diag, "type_inference.value_restriction_legacy_usage"
            ):
                continue
            schema = extract_schema_version(diag)
            if schema:
                schema_versions.add(schema)
            source_paths.add(str(path))
            occurrences.append(
                {
                    "file": str(path),
                    "index": index,
                    "code": primary_code_of(diag) or "unknown",
                    "message": diag.get("message"),
                    "mode": "legacy",
                }
            )

    usage_count = len(occurrences)
    if usage_count > 0:
        pass_rate, pass_fraction = calculate_pass_rates(usage_count, usage_count)
    else:
        pass_rate, pass_fraction = (None, None)

    return {
        "metric": "type_inference.value_restriction_legacy_usage",
        "total": usage_count,
        "passed": usage_count,
        "failed": 0,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "status": "info",
        "usage_count": usage_count,
        "occurrences": occurrences,
        "sources": sorted(source_paths),
        "schema_versions": sorted(schema_versions),
    }


def collect_domain_metrics(paths: Sequence[Path]) -> Optional[Dict[str, Any]]:
    total = 0
    passed = 0
    failures: List[Dict[str, Any]] = []
    schema_versions: Set[str] = set()

    for path in paths:
        data = load_json(path)
        diag_list = list(iter_diagnostics(data))
        for index, diag in enumerate(diag_list):
            domain_value = _normalize_domain(diag.get("domain"))
            expected_kind = primary_code_of(diag)
            if domain_value is None and not expected_kind:
                continue
            total += 1
            schema = extract_schema_version(diag)
            if schema:
                schema_versions.add(schema)
            event_domain_value = _diagnostic_metadata_lookup(diag, "event.domain")
            event_kind_value = _diagnostic_metadata_lookup(diag, "event.kind")
            reasons: List[str] = []
            if domain_value:
                normalized_event = _normalize_domain(event_domain_value)
                if normalized_event != domain_value:
                    reasons.append("event.domain")
            if expected_kind:
                if not isinstance(event_kind_value, str) or not event_kind_value.strip():
                    reasons.append("event.kind.missing")
                elif event_kind_value != expected_kind:
                    reasons.append("event.kind.mismatch")
            if not reasons:
                passed += 1
            else:
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "reasons": reasons,
                        "domain": domain_value,
                        "expected_kind": expected_kind,
                        "event_domain": event_domain_value,
                        "event_kind": event_kind_value,
                    }
                )

    if total == 0:
        return None

    pass_rate, pass_fraction = calculate_pass_rates(passed, total)
    status = "success" if pass_rate == 1.0 else "error"
    return {
        "metric": "diagnostics.domain_coverage",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "status": status,
        "sources": [str(path) for path in paths],
        "failures": failures,
        "schema_versions": sorted(schema_versions),
    }


def collect_effect_stage_consistency(
    paths: Sequence[Path],
) -> Optional[Dict[str, Any]]:
    total = 0
    passed = 0
    failures: List[Dict[str, Any]] = []

    for path in paths:
        data = load_json(path)
        try:
            diagnostics_iter = iter_diagnostics(data)
        except ValueError:
            continue
        for index, diag in enumerate(diagnostics_iter):
            extensions = _as_dict(diag.get("extensions"))
            capability_ext = None
            if extensions:
                capability_ext = _as_dict(extensions.get("capability"))
            metadata = _as_dict(diag.get("audit_metadata"))
            audit_entry = _as_dict(diag.get("audit"))
            envelope_meta = _as_dict(audit_entry.get("metadata")) if audit_entry else None

            capability_present = capability_ext is not None
            metadata_present = metadata and "capability.ids" in metadata
            envelope_present = envelope_meta and "capability.ids" in envelope_meta

            if not (capability_present or metadata_present or envelope_present):
                continue

            total += 1

            ids_extension = (
                _as_string_list(capability_ext.get("ids")) if capability_ext else None
            )
            metadata_ids = (
                _as_string_list(metadata.get("capability.ids"))
                if metadata and "capability.ids" in metadata
                else None
            )
            envelope_ids = (
                _as_string_list(envelope_meta.get("capability.ids"))
                if envelope_meta and "capability.ids" in envelope_meta
                else None
            )

            expected_ids = None
            for candidate in (ids_extension, metadata_ids, envelope_ids):
                if candidate:
                    expected_ids = sorted(set(candidate))
                    break

            reasons: List[str] = []
            if not expected_ids:
                reasons.append("capability.ids.empty")
            if ids_extension and sorted(set(ids_extension)) != expected_ids:
                reasons.append("capability.extension.mismatch")
            if metadata_present:
                if not metadata_ids:
                    reasons.append("capability.audit_metadata.missing")
                elif sorted(set(metadata_ids)) != expected_ids:
                    reasons.append("capability.audit_metadata.mismatch")
            if envelope_present:
                if not envelope_ids:
                    reasons.append("capability.audit_envelope.missing")
                elif sorted(set(envelope_ids)) != expected_ids:
                    reasons.append("capability.audit_envelope.mismatch")

            if reasons:
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "reasons": reasons,
                        "extension_ids": ids_extension,
                        "audit_ids": metadata_ids,
                        "envelope_ids": envelope_ids,
                    }
                )
            else:
                passed += 1

    if total == 0:
        return None

    pass_rate, pass_fraction = calculate_pass_rates(passed, total)
    status = "success" if pass_rate == 1.0 else "error"
    return {
        "metric": "diagnostics.effect_stage_consistency",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "status": status,
        "sources": [str(path) for path in paths],
        "failures": failures,
    }


def _diagnostic_is_rust_frontend(metadata: Optional[Dict[str, Any]]) -> bool:
    if not metadata:
        return False
    candidates = []
    for key in ("namespace", "parser.core.rule.namespace", "parser.core.rule.origin"):
        value = metadata.get(key)
        if isinstance(value, str) and value.strip():
            candidates.append(value.strip().lower())
    return any("rust" in candidate for candidate in candidates)


def collect_effect_scope_audit_presence(
    paths: Sequence[Path],
) -> Optional[Dict[str, Any]]:
    required_extension_keys = [
        "effect.capabilities",
        "effect.required_capabilities",
        "effect.stage.required_capabilities",
        "effect.stage.actual_capabilities",
        "effect.stage.required",
        "effect.stage.actual",
    ]
    required_metadata_keys = [
        "capability.ids",
        "effect.required_capabilities",
        "effect.stage.required_capabilities",
        "effect.stage.actual_capabilities",
        "effect.stage.required",
        "effect.stage.actual",
        "bridge.stage.required_capabilities",
        "bridge.stage.actual_capabilities",
        "bridge.stage.capability",
    ]

    total = 0
    passed = 0
    failures: List[Dict[str, Any]] = []

    for path in paths:
        data = load_json(path)
        try:
            diagnostics_iter = iter_diagnostics(data)
        except ValueError:
            continue
        for index, diag in enumerate(diagnostics_iter):
            metadata = _as_dict(diag.get("audit_metadata"))
            if not _diagnostic_is_rust_frontend(metadata):
                continue
            extensions = _as_dict(diag.get("extensions"))
            if not extensions:
                continue
            # Only enforce when capability payload is expected.
            capability_payload = extensions.get("effect.capabilities")
            if capability_payload is None:
                continue
            if isinstance(capability_payload, list) and not capability_payload:
                continue
            if isinstance(capability_payload, str) and not capability_payload.strip():
                continue

            total += 1
            missing_keys: List[str] = []
            for key in required_extension_keys:
                if not _value_present(extensions.get(key)):
                    missing_keys.append(f"extensions.{key}")
            for key in required_metadata_keys:
                if not _value_present(_diagnostic_metadata_lookup(diag, key)):
                    missing_keys.append(f"audit.{key}")

            if missing_keys:
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "missing_keys": sorted(set(missing_keys)),
                    }
                )
            else:
                passed += 1

    if total == 0:
        return None

    pass_rate, pass_fraction = calculate_pass_rates(passed, total)
    status = "success" if pass_rate == 1.0 else "error"
    return {
        "metric": "effect_scope.audit_presence",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "status": status,
        "sources": [str(path) for path in paths],
        "failures": failures,
        "required_keys": {
            "extensions": required_extension_keys,
            "audit_metadata": required_metadata_keys,
        },
    }


def collect_effect_stage_extension_presence(
    paths: Sequence[Path],
) -> Optional[Dict[str, Any]]:
    required_fields = [
        "required",
        "actual",
        "required_capabilities",
        "actual_capabilities",
    ]
    total = 0
    passed = 0
    failures: List[Dict[str, Any]] = []

    for path in paths:
        data = load_json(path)
        try:
            diagnostics_iter = iter_diagnostics(data)
        except ValueError:
            continue
        for index, diag in enumerate(diagnostics_iter):
            metadata = _as_dict(diag.get("audit_metadata"))
            if not _diagnostic_is_rust_frontend(metadata):
                continue
            extensions = _as_dict(diag.get("extensions"))
            effects_ext = _as_dict(extensions.get("effects")) if extensions else None
            if not effects_ext:
                continue
            capabilities_payload = effects_ext.get("capabilities")
            if capabilities_payload is None:
                continue
            if isinstance(capabilities_payload, list) and not capabilities_payload:
                continue
            stage_ext = _as_dict(effects_ext.get("stage"))
            total += 1
            missing: List[str] = []
            if stage_ext is None:
                missing.append("stage")
            else:
                for field in required_fields:
                    value = stage_ext.get(field)
                    if not _value_present(value):
                        missing.append(field)
            if missing:
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "missing_fields": sorted(set(missing)),
                    }
                )
            else:
                passed += 1

    if total == 0:
        return None

    pass_rate, pass_fraction = calculate_pass_rates(passed, total)
    status = "success" if pass_rate == 1.0 else "error"
    return {
        "metric": "effect_stage.audit_presence",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "status": status,
        "sources": [str(path) for path in paths],
        "failures": failures,
        "required_fields": required_fields,
    }


def collect_bridge_stage_extension_presence(
    paths: Sequence[Path],
) -> Optional[Dict[str, Any]]:
    required_fields = [
        "required_capabilities",
        "actual_capabilities",
        "required",
        "actual",
        "capability",
    ]
    total = 0
    passed = 0
    failures: List[Dict[str, Any]] = []

    for path in paths:
        data = load_json(path)
        try:
            diagnostics_iter = iter_diagnostics(data)
        except ValueError:
            continue
        for index, diag in enumerate(diagnostics_iter):
            metadata = _as_dict(diag.get("audit_metadata"))
            if not _diagnostic_is_rust_frontend(metadata):
                continue
            extensions = _as_dict(diag.get("extensions"))
            bridge_ext = _as_dict(extensions.get("bridge")) if extensions else None
            if not bridge_ext:
                continue
            stage_ext = _as_dict(bridge_ext.get("stage"))
            if stage_ext is None:
                continue
            total += 1
            missing: List[str] = []
            for field in required_fields:
                value = stage_ext.get(field)
                if not _value_present(value):
                    missing.append(field)
            if missing:
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "missing_fields": sorted(set(missing)),
                    }
                )
            else:
                passed += 1

    if total == 0:
        return None

    pass_rate, pass_fraction = calculate_pass_rates(passed, total)
    status = "success" if pass_rate == 1.0 else "error"
    return {
        "metric": "bridge_stage.audit_presence",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "status": status,
        "sources": [str(path) for path in paths],
        "failures": failures,
        "required_fields": required_fields,
    }


def _type_row_mode_is_dual(value: Any) -> bool:
    if isinstance(value, str):
        normalized = value.strip().lower()
        return normalized in {"dual", "dual-write", "dualwrite", "ty-dual-write"}
    return False


def collect_typeck_debug_metric(paths: Sequence[Path]) -> Optional[Dict[str, Any]]:
    total = 0
    passed = 0
    failures: List[Dict[str, Any]] = []
    sources: Set[str] = set()

    for diag_path in paths:
        case_dir = diag_path.parent
        typeck_dir = case_dir / "typeck"
        if not typeck_dir.is_dir():
            continue
        rust_debug = typeck_dir / "typeck-debug.rust.json"
        ocaml_debug = typeck_dir / "typeck-debug.ocaml.json"
        requirements = typeck_dir / "requirements.json"
        if not requirements.exists():
            continue

        total += 1
        sources.add(str(typeck_dir))
        missing: List[str] = []
        try:
            rust_payload = load_json(rust_debug)
        except Exception:
            rust_payload = None
        try:
            ocaml_payload = load_json(ocaml_debug) if ocaml_debug.exists() else None
        except Exception:
            ocaml_payload = None

        if rust_payload is None:
            missing.append("typeck-debug.rust.json")
        if ocaml_payload is None:
            missing.append("typeck-debug.ocaml.json")
        rust_mode = (
            rust_payload.get("type_row_mode") if isinstance(rust_payload, dict) else None
        )
        ocaml_mode = (
            ocaml_payload.get("type_row_mode")
            if isinstance(ocaml_payload, dict)
            else None
        )
        if rust_mode is not None and not _type_row_mode_is_dual(rust_mode):
            missing.append("type_row_mode.rust")
        if ocaml_payload is not None and not _type_row_mode_is_dual(ocaml_mode):
            missing.append("type_row_mode.ocaml")

        if missing:
            failures.append(
                {
                    "case": case_dir.name,
                    "typeck_dir": str(typeck_dir),
                    "missing": sorted(set(missing)),
                }
            )
        else:
            passed += 1

    if total == 0:
        return None

    pass_rate, pass_fraction = calculate_pass_rates(passed, total)
    status = "success" if pass_rate == 1.0 else "error"
    return {
        "metric": "typeck_debug_match",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "status": status,
        "sources": sorted(sources),
        "failures": failures,
    }


def collect_effect_syntax_metrics(
    paths: Sequence[Path],
) -> Optional[Dict[str, Any]]:
    total_expected = 0
    accepted = 0
    poison = 0
    failures: List[Dict[str, Any]] = []
    used_paths: List[str] = []

    for path in paths:
        data = load_json(path)
        effect_section = _as_dict(data.get("effect_syntax"))
        if not effect_section:
            continue
        constructs = effect_section.get("constructs")
        if not isinstance(constructs, list):
            continue
        used_paths.append(str(path))
        for entry in constructs:
            if not isinstance(entry, dict):
                continue
            expectation_raw = entry.get("expectation")
            expectation = (
                expectation_raw.lower()
                if isinstance(expectation_raw, str)
                else "accept"
            )
            if expectation not in ("accept", "reject"):
                expectation = "accept"
            diagnostics = _as_string_list(entry.get("diagnostics")) or []
            status_raw = entry.get("status")
            status = (
                status_raw.lower() if isinstance(status_raw, str) else "ok"
            )
            name = entry.get("name")
            if expectation == "accept":
                total_expected += 1
                if diagnostics or status not in ("ok", "success"):
                    poison += 1
                    failures.append(
                        {
                            "name": name,
                            "status": status,
                            "diagnostics": diagnostics,
                            "source": str(path),
                        }
                    )
                else:
                    accepted += 1

    if total_expected == 0:
        return None

    pass_rate, pass_fraction = calculate_pass_rates(accepted, total_expected)
    status = "success" if pass_rate == 1.0 else "error"
    poison_rate = poison / total_expected if total_expected > 0 else 0.0
    poison_status = "success" if poison == 0 else "error"

    sources = sorted(set(used_paths))
    metric: Dict[str, Any] = {
        "metric": "syntax.effect_construct_acceptance",
        "total": total_expected,
        "passed": accepted,
        "failed": total_expected - accepted,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "status": status,
        "sources": sources,
    }
    if failures:
        metric["failures"] = failures

    poison_metric: Dict[str, Any] = {
        "metric": "effects.syntax_poison_rate",
        "value": poison_rate,
        "total": total_expected,
        "failed": poison,
        "status": poison_status,
        "sources": sources,
    }
    if failures and poison > 0:
        poison_metric["failures"] = failures

    metric["related_metrics"] = [poison_metric]
    return metric


def collect_effect_row_metrics(
    paths: Sequence[Path],
) -> Tuple[
    Optional[Dict[str, Any]],
    Optional[Dict[str, Any]],
    Optional[Dict[str, Any]],
]:
    stage_total = 0
    stage_passed = 0
    stage_failures: List[Dict[str, Any]] = []
    stage_sources: Set[str] = set()

    type_total = 0
    type_passed = 0
    type_failures: List[Dict[str, Any]] = []
    type_sources: Set[str] = set()

    guard_count = 0
    guard_occurrences: List[Dict[str, Any]] = []
    guard_sources: Set[str] = set()

    for path in paths:
        data = load_json(path)
        try:
            diagnostics_iter = iter_diagnostics(data)
        except ValueError:
            continue
        for index, diag in enumerate(diagnostics_iter):
            codes = _as_string_list(diag.get("codes"))
            if codes and any(
                isinstance(code, str)
                and code.strip() == "effects.type_row.integration_blocked"
                for code in codes
            ):
                guard_count += 1
                guard_sources.add(str(path))
                guard_occurrences.append(
                    {
                        "file": str(path),
                        "index": index,
                        "message": diag.get("message"),
                    }
                )

            extensions = _as_dict(diag.get("extensions"))
            declared_ext = (
                _normalize_string_list(
                    extensions.get("effect.type_row.declared") if extensions else None
                )
            )
            residual_ext = (
                _normalize_string_list(
                    extensions.get("effect.type_row.residual") if extensions else None
                )
            )
            canonical_ext = (
                _normalize_string_list(
                    extensions.get("effect.type_row.canonical") if extensions else None
                )
            )

            if (
                declared_ext is None
                and residual_ext is None
                and canonical_ext is None
            ):
                continue

            type_sources.add(str(path))
            type_total += 1
            type_reasons: List[str] = []
            if declared_ext is None:
                type_reasons.append("extensions.declared.missing")
                declared_norm: List[str] = []
            else:
                declared_norm = [value.lower() for value in declared_ext]
            if residual_ext is None:
                type_reasons.append("extensions.residual.missing")
                residual_norm: List[str] = []
            else:
                residual_norm = [value.lower() for value in residual_ext]
            if canonical_ext is None:
                type_reasons.append("extensions.canonical.missing")
                canonical_norm: List[str] = []
            else:
                canonical_norm = [value.lower() for value in canonical_ext]

            if not type_reasons:
                expected_canonical = sorted(set(declared_norm + residual_norm))
                if sorted(set(canonical_norm)) != expected_canonical:
                    type_reasons.append("canonical.mismatch")

            if type_reasons:
                type_failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "reasons": type_reasons,
                        "declared": declared_ext,
                        "residual": residual_ext,
                        "canonical": canonical_ext,
                    }
                )
            else:
                type_passed += 1

            metadata = _as_dict(diag.get("audit_metadata"))
            audit_entry = _as_dict(diag.get("audit"))
            envelope_meta = (
                _as_dict(audit_entry.get("metadata")) if audit_entry else None
            )

            stage_total += 1
            stage_sources.add(str(path))
            stage_reasons: List[str] = []

            declared_diag = declared_norm
            residual_diag = residual_norm
            canonical_diag = sorted(set(canonical_norm))

            metadata_declared = (
                _normalize_string_list(
                    metadata.get("effect.type_row.declared") if metadata else None
                )
            )
            metadata_residual = (
                _normalize_string_list(
                    metadata.get("effect.type_row.residual") if metadata else None
                )
            )
            metadata_canonical = (
                _normalize_string_list(
                    metadata.get("effect.type_row.canonical") if metadata else None
                )
            )

            envelope_declared = (
                _normalize_string_list(
                    envelope_meta.get("effect.type_row.declared")
                    if envelope_meta
                    else None
                )
            )
            envelope_residual = (
                _normalize_string_list(
                    envelope_meta.get("effect.type_row.residual")
                    if envelope_meta
                    else None
                )
            )
            envelope_canonical = (
                _normalize_string_list(
                    envelope_meta.get("effect.type_row.canonical")
                    if envelope_meta
                    else None
                )
            )

            def _normalize_canonical(value: Optional[List[str]]) -> Optional[List[str]]:
                if value is None:
                    return None
                return sorted(set(entry.lower() for entry in value))

            def _normalize_linear(value: Optional[List[str]]) -> Optional[List[str]]:
                if value is None:
                    return None
                return [entry.lower() for entry in value]

            metadata_declared_norm = _normalize_linear(metadata_declared)
            metadata_residual_norm = _normalize_linear(metadata_residual)
            metadata_canonical_norm = _normalize_canonical(metadata_canonical)

            envelope_declared_norm = _normalize_linear(envelope_declared)
            envelope_residual_norm = _normalize_linear(envelope_residual)
            envelope_canonical_norm = _normalize_canonical(envelope_canonical)

            if metadata_declared_norm is None:
                stage_reasons.append("audit_metadata.declared.missing")
            elif metadata_declared_norm != declared_diag:
                stage_reasons.append("audit_metadata.declared.mismatch")
            if metadata_residual_norm is None:
                stage_reasons.append("audit_metadata.residual.missing")
            elif metadata_residual_norm != residual_diag:
                stage_reasons.append("audit_metadata.residual.mismatch")
            if metadata_canonical_norm is None:
                stage_reasons.append("audit_metadata.canonical.missing")
            elif metadata_canonical_norm != canonical_diag:
                stage_reasons.append("audit_metadata.canonical.mismatch")

            if envelope_declared_norm is None:
                stage_reasons.append("audit_envelope.declared.missing")
            elif envelope_declared_norm != declared_diag:
                stage_reasons.append("audit_envelope.declared.mismatch")
            if envelope_residual_norm is None:
                stage_reasons.append("audit_envelope.residual.missing")
            elif envelope_residual_norm != residual_diag:
                stage_reasons.append("audit_envelope.residual.mismatch")
            if envelope_canonical_norm is None:
                stage_reasons.append("audit_envelope.canonical.missing")
            elif envelope_canonical_norm != canonical_diag:
                stage_reasons.append("audit_envelope.canonical.mismatch")

            if stage_reasons:
                stage_failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "reasons": stage_reasons,
                        "declared": declared_ext,
                        "residual": residual_ext,
                        "canonical": canonical_ext,
                        "audit_metadata_declared": metadata_declared,
                        "audit_metadata_residual": metadata_residual,
                        "audit_metadata_canonical": metadata_canonical,
                        "audit_envelope_declared": envelope_declared,
                        "audit_envelope_residual": envelope_residual,
                        "audit_envelope_canonical": envelope_canonical,
                    }
                )
            else:
                stage_passed += 1

    stage_metric: Optional[Dict[str, Any]]
    if stage_total == 0:
        stage_metric = None
    else:
        stage_pass_rate, stage_fraction = calculate_pass_rates(
            stage_passed, stage_total
        )
        stage_metric = {
            "metric": "diagnostics.effect_row_stage_consistency",
            "total": stage_total,
            "passed": stage_passed,
            "failed": stage_total - stage_passed,
            "pass_rate": stage_pass_rate,
            "pass_fraction": stage_fraction,
            "status": "success" if stage_pass_rate == 1.0 else "error",
            "sources": sorted(stage_sources),
        }
        if stage_failures:
            stage_metric["failures"] = stage_failures

    type_metric: Optional[Dict[str, Any]]
    if type_total == 0:
        type_metric = None
    else:
        type_pass_rate, type_fraction = calculate_pass_rates(type_passed, type_total)
        type_metric = {
            "metric": "type_effect_row_equivalence",
            "total": type_total,
            "passed": type_passed,
            "failed": type_total - type_passed,
            "pass_rate": type_pass_rate,
            "pass_fraction": type_fraction,
            "status": "success" if type_pass_rate == 1.0 else "error",
            "sources": sorted(type_sources),
        }
        if type_failures:
            type_metric["failures"] = type_failures

    guard_metric: Optional[Dict[str, Any]]
    if guard_count == 0:
        guard_metric = {
            "metric": "effect_row_guard_regressions",
            "count": 0,
            "status": "success",
        }
    else:
        guard_metric = {
            "metric": "effect_row_guard_regressions",
            "count": guard_count,
            "status": "error",
            "sources": sorted(guard_sources),
            "occurrences": guard_occurrences,
        }
    return stage_metric, type_metric, guard_metric


def collect_plugin_bundle_metrics(paths: Sequence[Path]) -> Optional[Dict[str, Any]]:
    total = 0
    passed = 0
    failures: List[Dict[str, Any]] = []

    for path in paths:
        data = load_json(path)
        try:
            diagnostics_iter = iter_diagnostics(data)
        except ValueError:
            continue
        for index, diag in enumerate(diagnostics_iter):
            domain = _normalize_domain(diag.get("domain"))
            extensions = _as_dict(diag.get("extensions"))
            plugin_ext = None
            if extensions:
                plugin_ext = _as_dict(extensions.get("plugin"))
            if domain != "plugin" and plugin_ext is None:
                continue
            total += 1
            bundle_extension = (
                plugin_ext.get("bundle_id") if plugin_ext and "bundle_id" in plugin_ext else None
            )
            metadata_bundle = _diagnostic_metadata_lookup(diag, "plugin.bundle_id")
            reasons: List[str] = []
            if not isinstance(bundle_extension, str) or not bundle_extension:
                reasons.append("extension.bundle_id.missing")
            if not isinstance(metadata_bundle, str) or not metadata_bundle:
                reasons.append("audit.bundle_id.missing")
            elif isinstance(bundle_extension, str) and bundle_extension != metadata_bundle:
                reasons.append("bundle_id.mismatch")

            signature_status = _diagnostic_metadata_lookup(diag, "plugin.signature.status")
            if plugin_ext and "signature" in plugin_ext:
                signature = plugin_ext["signature"]
                if isinstance(signature, dict) and "status" in signature:
                    status_value = signature["status"]
                    if status_value != signature_status:
                        reasons.append("signature.status.mismatch")

            if reasons:
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "reasons": reasons,
                        "bundle_extension": bundle_extension,
                        "bundle_audit": metadata_bundle,
                        "signature_status": signature_status,
                    }
                )
            else:
                passed += 1

    if total == 0:
        return None

    pass_rate, pass_fraction = calculate_pass_rates(passed, total)
    status = "success" if pass_rate == 1.0 else "warning"
    return {
        "metric": "diagnostics.plugin_bundle_ratio",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "status": status,
        "sources": [str(path) for path in paths],
        "failures": failures,
    }


def collect_bridge_metrics(
    paths: List[Path],
    audit_paths: List[Path],
    platform_filters: Optional[Set[str]] = None,
) -> Dict:
    total = 0
    passed = 0
    failures: List[Dict[str, object]] = []
    platform_summary: Dict[str, Dict[str, int]] = {}
    audit_sources: List[str] = []
    schema_versions: Set[str] = set()
    status_success = 0
    status_failure = 0
    normalized_filters: Set[str] = set()
    platform_hits: Dict[str, bool] = {}

    if platform_filters:
        for item in platform_filters:
            normalized = _normalize_platform(item)
            if normalized:
                normalized_filters.add(normalized)
        platform_hits = {key: False for key in normalized_filters}

    def _tally_status(value: Optional[object]) -> None:
        nonlocal status_success, status_failure
        if isinstance(value, str):
            lowered = value.lower()
            if lowered in {"ok", "success", "passed", "pass"}:
                status_success += 1
            elif lowered:
                status_failure += 1

    for path in paths:
        data = load_json(path)
        try:
            diagnostics_iter = iter_diagnostics(data)
        except ValueError:
            continue
        for index, diag in enumerate(diagnostics_iter):
            code = primary_code_of(diag)
            codes_field = diag.get("codes")
            has_bridge_code = False
            if isinstance(code, str) and code.startswith(BRIDGE_DIAG_PREFIX):
                has_bridge_code = True
            elif isinstance(codes_field, list):
                has_bridge_code = any(
                    isinstance(item, str) and item.startswith(BRIDGE_DIAG_PREFIX)
                    for item in codes_field
                )
            if not has_bridge_code:
                continue
            total += 1
            schema = extract_schema_version(diag)
            if schema:
                schema_versions.add(schema)
            audit_dict = _as_dict(diag.get("audit"))
            extensions_dict = _as_dict(diag.get("extensions"))
            status_value = extract_bridge_status(audit_dict, extensions_dict)
            platform_value = extract_bridge_field(
                audit_dict, extensions_dict, "platform"
            )
            platform_normalized = _normalize_platform(platform_value)
            if normalized_filters and platform_normalized not in normalized_filters:
                continue
            if platform_normalized in platform_hits:
                platform_hits[platform_normalized] = True
            _tally_status(status_value)

            if platform_normalized:
                platform_key = platform_normalized
            elif isinstance(platform_value, str) and platform_value.strip():
                platform_key = platform_value.strip()
            else:
                platform_key = "<unknown>"
            platform_record = platform_summary.setdefault(
                platform_key, {"total": 0, "ok": 0, "failed": 0}
            )
            platform_record["total"] += 1

            audit_missing = check_bridge_audit_fields(audit_dict)
            extensions_missing = check_bridge_extension_fields(
                extensions_dict
            )

            issues: List[str] = []
            issues.extend(audit_missing)
            issues.extend(extensions_missing)
            timestamp_value = diag.get("timestamp")
            if not isinstance(timestamp_value, str) or not timestamp_value.strip():
                issues.append("timestamp")

            if not issues:
                passed += 1
                platform_record["ok"] += 1
            else:
                platform_record["failed"] += 1
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "code": code or "unknown",
                        "missing": sorted(set(issues)),
                        "status": status_value,
                        "platform": platform_value,
                        "platform_normalized": platform_normalized,
                    }
                )

    for path in audit_paths:
        audit_sources.append(str(path))
        entries = load_audit_entries(path)
        if not entries:
            total += 1
            missing_record = platform_summary.setdefault(
                "<missing>", {"total": 0, "ok": 0, "failed": 0}
            )
            missing_record["total"] += 1
            missing_record["failed"] += 1
            failures.append(
                {
                    "file": str(path),
                    "index": None,
                    "code": "ffi.audit.empty",
                    "missing": ["audit_entries"],
                    "status": None,
                    "platform": None,
                    "platform_normalized": None,
                }
            )
            continue

        valid_entries = 0
        for index, entry in enumerate(entries):
            category = entry.get("category") if isinstance(entry, dict) else None
            if isinstance(category, str) and not category.startswith("ffi.bridge"):
                continue

            metadata = {}
            if isinstance(entry, dict):
                if isinstance(entry.get("metadata"), dict):
                    metadata = entry["metadata"]
                else:
                    metadata = entry
            audit_dict = metadata if isinstance(metadata, dict) else None
            extensions_dict = None
            if isinstance(entry, dict) and isinstance(entry.get("extensions"), dict):
                extensions_dict = entry["extensions"]

            audit_missing = check_bridge_audit_fields(audit_dict)
            extensions_missing = check_bridge_extension_fields(
                extensions_dict
            )

            issues: List[str] = []
            issues.extend(audit_missing)
            issues.extend(extensions_missing)

            status_value = extract_bridge_status(audit_dict, extensions_dict)
            platform_value = extract_bridge_field(
                audit_dict, extensions_dict, "platform"
            )
            platform_normalized = _normalize_platform(platform_value)
            if normalized_filters and platform_normalized not in normalized_filters:
                continue
            if platform_normalized in platform_hits:
                platform_hits[platform_normalized] = True
            _tally_status(status_value)

            if platform_normalized:
                platform_key = platform_normalized
            elif isinstance(platform_value, str) and platform_value.strip():
                platform_key = platform_value.strip()
            else:
                platform_key = "<unknown>"
            platform_record = platform_summary.setdefault(
                platform_key, {"total": 0, "ok": 0, "failed": 0}
            )

            total += 1
            platform_record["total"] += 1
            valid_entries += 1

            if not issues:
                passed += 1
                platform_record["ok"] += 1
            else:
                platform_record["failed"] += 1
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "code": category if isinstance(category, str) else "ffi.audit",
                        "missing": sorted(set(issues)),
                        "status": status_value,
                        "platform": platform_value,
                        "platform_normalized": platform_normalized,
                    }
                )

        if valid_entries == 0:
            total += 1
            missing_record = platform_summary.setdefault(
                "<missing>", {"total": 0, "ok": 0, "failed": 0}
            )
            missing_record["total"] += 1
            missing_record["failed"] += 1
            failures.append(
                {
                    "file": str(path),
                    "index": None,
                    "code": "ffi.audit.missing_bridge",
                    "missing": ["bridge"],
                    "status": None,
                    "platform": None,
                    "platform_normalized": None,
                }
            )

    if normalized_filters:
        for platform in normalized_filters:
            platform_summary.setdefault(platform, {"total": 0, "ok": 0, "failed": 0})
            if not platform_hits.get(platform, False):
                total += 1
                record = platform_summary.setdefault(
                    platform, {"total": 0, "ok": 0, "failed": 0}
                )
                record["total"] += 1
                record["failed"] += 1
                failures.append(
                    {
                        "file": None,
                        "index": None,
                        "code": "ffi.audit.platform_missing",
                        "missing": ["platform"],
                        "status": None,
                        "platform": platform,
                        "platform_normalized": platform,
                    }
                )

    pass_rate, pass_fraction = calculate_pass_rates(passed, total)

    return {
        "metric": "ffi_bridge.audit_pass_rate",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "required_audit_keys": REQUIRED_BRIDGE_AUDIT_KEYS,
        "sources": [str(path) for path in paths],
        "audit_sources": audit_sources,
        "failures": failures,
        "platform_summary": platform_summary,
        "schema_versions": sorted(schema_versions),
        "platform_filter": sorted(normalized_filters) if normalized_filters else None,
        "status_summary": {
            "success": status_success,
            "failure": status_failure,
            "platforms": platform_summary,
        },
    }


def _collect_typeclass_metadata_metric(
    paths: List[Path], audit_paths: List[Path]
) -> Dict:
    total = 0
    passed = 0
    failures: List[Dict[str, Any]] = []
    schema_versions: Set[str] = set()
    audit_sources: List[str] = []

    for path in paths:
        data = load_json(path)
        try:
            diagnostics_iter = iter_diagnostics(data)
        except ValueError:
            continue
        for index, diag in enumerate(diagnostics_iter):
            code = primary_code_of(diag)
            codes_field = diag.get("codes")
            has_typeclass_code = False
            if isinstance(code, str) and code.startswith("typeclass."):
                has_typeclass_code = True
            elif isinstance(codes_field, list):
                has_typeclass_code = any(
                    isinstance(item, str) and item.startswith("typeclass.")
                    for item in codes_field
                )
            if not has_typeclass_code:
                continue

            total += 1
            schema = extract_schema_version(diag)
            if schema:
                schema_versions.add(schema)

            audit_dict = _as_dict(diag.get("audit"))
            extensions_dict = _as_dict(diag.get("extensions"))

            issues: List[str] = []
            issues.extend(check_typeclass_audit_fields(audit_dict))
            issues.extend(check_typeclass_extension_fields(extensions_dict))

            timestamp_value = diag.get("timestamp")
            if not isinstance(timestamp_value, str) or not timestamp_value.strip():
                issues.append("timestamp")

            if issues:
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "code": code or "unknown",
                        "missing": sorted(set(issues)),
                    }
                )
            else:
                passed += 1

    for path in audit_paths:
        audit_sources.append(str(path))
        entries = load_audit_entries(path)
        if not entries:
            continue

        for index, entry in enumerate(entries):
            category = entry.get("category")
            if not (isinstance(category, str) and category.startswith("typeclass.")):
                continue

            total += 1
            metadata = entry.get("metadata")
            metadata_dict = metadata if isinstance(metadata, dict) else None

            issues = check_typeclass_audit_fields(metadata_dict)
            if issues:
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "code": category,
                        "missing": sorted(set(issues)),
                    }
                )
            else:
                passed += 1

    pass_rate, pass_fraction = calculate_pass_rates(passed, total)

    return {
        "metric": "typeclass.metadata_pass_rate",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "required_audit_keys": list(TYPECLASS_REQUIRED_AUDIT_KEYS),
        "sources": [str(path) for path in paths],
        "audit_sources": audit_sources,
        "failures": failures,
        "schema_versions": sorted(schema_versions),
    }


def collect_typeclass_dictionary_metric(
    paths: List[Path], audit_paths: List[Path]
) -> Dict:
    total = 0
    passed = 0
    failures: List[Dict[str, Any]] = []
    audit_sources: List[str] = []
    schema_versions: Set[str] = set()

    for path in paths:
        data = load_json(path)
        try:
            diagnostics_iter = iter_diagnostics(data)
        except ValueError:
            continue
        for index, diag in enumerate(diagnostics_iter):
            code = primary_code_of(diag) or ""
            codes_field = diag.get("codes")
            has_typeclass_code = False
            if isinstance(code, str) and code.startswith("typeclass."):
                has_typeclass_code = True
            elif isinstance(codes_field, list):
                has_typeclass_code = any(
                    isinstance(item, str) and item.startswith("typeclass.")
                    for item in codes_field
                )
            if not has_typeclass_code:
                continue

            total += 1
            schema = extract_schema_version(diag)
            if schema:
                schema_versions.add(schema)

            extensions_dict = _as_dict(diag.get("extensions"))
            audit_dict = _as_dict(diag.get("audit"))

            issues: List[str] = []
            issues.extend(check_dictionary_extension_payload(extensions_dict))
            issues.extend(check_dictionary_audit_payload(audit_dict))

            timestamp_value = diag.get("timestamp")
            if not isinstance(timestamp_value, str) or not timestamp_value.strip():
                issues.append("timestamp")

            if issues:
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "code": code or "unknown",
                        "missing": sorted(set(issues)),
                    }
                )
            else:
                passed += 1

    for path in audit_paths:
        audit_sources.append(str(path))
        entries = load_audit_entries(path)
        if not entries:
            continue
        for index, entry in enumerate(entries):
            category = entry.get("category") if isinstance(entry, dict) else None
            if not (isinstance(category, str) and category.startswith("typeclass.")):
                continue
            metadata = entry.get("metadata") if isinstance(entry, dict) else None
            metadata_dict = metadata if isinstance(metadata, dict) else None

            issues = _validate_dictionary_metadata(metadata_dict, "metadata")
            if not issues:
                schema_value = _lookup_metadata_value(metadata_dict, "schema.version")
                schema_str = _normalize_nonempty_string(schema_value)
                if schema_str:
                    schema_versions.add(schema_str)

            total += 1
            if issues:
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "code": category,
                        "missing": sorted(set(issues)),
                    }
                )
            else:
                passed += 1

    pass_rate, pass_fraction = calculate_pass_rates(passed, total)

    required_keys = [
        "extensions.typeclass.dictionary.kind",
        "extensions.typeclass.dictionary.identifier",
        "extensions.typeclass.dictionary.repr",
        "typeclass.dictionary.kind",
        "typeclass.dictionary.identifier",
        "typeclass.dictionary.repr",
    ]

    return {
        "metric": "typeclass.dictionary_pass_rate",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "pass_fraction": pass_fraction,
        "required_audit_keys": required_keys,
        "sources": [str(path) for path in paths],
        "audit_sources": audit_sources,
        "failures": failures,
        "schema_versions": sorted(schema_versions),
    }


def collect_typeclass_metrics(paths: List[Path], audit_paths: List[Path]) -> Dict:
    metadata_metric = _collect_typeclass_metadata_metric(paths, audit_paths)
    dictionary_metric = collect_typeclass_dictionary_metric(paths, audit_paths)

    combined = dict(metadata_metric)
    related_metrics: List[Dict[str, Any]] = []
    if dictionary_metric:
        related_metrics.append(dictionary_metric)
        combined["dictionary_metric"] = dictionary_metric
        combined["dictionary_pass_rate"] = dictionary_metric.get("pass_rate")
        combined["dictionary_pass_fraction"] = dictionary_metric.get("pass_fraction")
        combined_audit_sources = set(metadata_metric.get("audit_sources") or [])
        combined_audit_sources.update(dictionary_metric.get("audit_sources") or [])
        combined["audit_sources"] = sorted(combined_audit_sources)
        combined_schema_versions = set(metadata_metric.get("schema_versions") or [])
        combined_schema_versions.update(dictionary_metric.get("schema_versions") or [])
        combined["schema_versions"] = sorted(combined_schema_versions)
    else:
        combined["dictionary_metric"] = None

    if related_metrics:
        combined["related_metrics"] = related_metrics

    return combined


def parse_args(argv: Optional[List[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Collect iterator stage audit metrics."
    )
    parser.add_argument(
        "--source",
        action="append",
        dest="sources",
        help="Path to diagnostic JSON (repeatable).",
    )
    parser.add_argument(
        "--text-source",
        action="append",
        dest="text_sources",
        help="Path to Core.Text metrics JSON (repeatable).省略時は reports/spec-audit/ch1/core_text_grapheme_stats.json を参照します。",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="Destination for collected metrics (JSON).",
    )
    parser.add_argument(
        "--audit-source",
        action="append",
        dest="audit_sources",
        help="Path to audit JSON (repeatable).",
    )
    parser.add_argument(
        "--section",
        choices=[
            "all",
            "parser",
            "lexer",
            "streaming",
            "iterator",
            "collectors",
            "effects",
            "diag",
            "type_inference",
            "ffi",
            "typeclass",
            "text",
            "review",
        ],
        default="all",
        help="Collect metrics for a specific section (default: all).",
    )
    parser.add_argument(
        "--module",
        action="append",
        dest="modules",
        help="監査対象のモジュール（例: iter, collector）をメタデータとして記録（繰り返し指定可）。",
    )
    parser.add_argument(
        "--case",
        help="Associate the collected metrics with a named case (metadata-only).",
    )
    parser.add_argument(
        "--metrics-case",
        help="diag セクションで検証するメトリクスケース名 (例: effects-contract)。",
    )
    parser.add_argument(
        "--platform",
        action="append",
        dest="platforms",
        help="bridge.platform を指定して各種メトリクスをフィルタ（繰り返し指定可）。",
    )
    parser.add_argument(
        "--summary",
        type=Path,
        help="Generate Markdown summary from the specified audit index JSON.",
    )
    parser.add_argument(
        "--prune",
        action="store_true",
        help="Prune the index passed via --summary according to retention policy.",
    )
    parser.add_argument(
        "--retention-config",
        type=Path,
        default=Path("tooling/ci/audit-retention.toml"),
        help="Retention policy TOML (default: tooling/ci/audit-retention.toml).",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show prune result without writing changes.",
    )
    parser.add_argument(
        "--review-diff",
        action="append",
        dest="review_diff",
        help="Path to review diff summary JSON (diff.json).",
    )
    parser.add_argument(
        "--review-coverage",
        action="append",
        dest="review_coverage",
        help="Path to review coverage report JSON.",
    )
    parser.add_argument(
        "--review-dashboard",
        action="append",
        dest="review_dashboard",
        help="Path to generated dashboard artifact (HTML/Markdown).",
    )
    parser.add_argument(
        "--scenario",
        action="append",
        dest="scenarios",
        choices=[
            "map_set_persistent",
            "vec_mem_exhaustion",
            "ref_internal_mutation",
            "audit_cap",
            "grapheme_stats",
        ],
        help="Scenario-specific validation (repeatable).",
    )
    parser.add_argument(
        "--ci-duration-seconds",
        type=float,
        default=None,
        help="Overall CI job duration in seconds (optional).",
    )
    parser.add_argument(
        "--stage-duration-seconds",
        type=float,
        default=None,
        help="Iterator audit stage duration in seconds (optional).",
    )
    parser.add_argument(
        "--append-from",
        action="append",
        dest="append_from",
        help="追加のメトリクス JSON を結合して出力（繰り返し指定可）。",
    )
    parser.add_argument(
        "--require-success",
        action="store_true",
        help="主要メトリクスが失敗した場合に非ゼロ終了コードを返す。",
    )
    parser.add_argument(
        "--require-cell",
        action="store_true",
        help="ref_internal_mutation シナリオの cell/ref KPI が success であることを保証する。",
    )
    return parser.parse_args(argv)


def main(argv: Optional[List[str]] = None) -> int:
    args = parse_args(argv)
    if args.prune and not args.summary:
        sys.stderr.write("--prune を使用する場合は --summary でインデックスを指定してください。\n")
        return 2

    scenario_filters: Set[str] = set()
    if getattr(args, "scenarios", None):
        for scenario in args.scenarios:
            if not scenario:
                continue
            scenario_filters.add(scenario)

    section_order = [
        "parser",
        "lexer",
        "streaming",
        "iterator",
        "collectors",
        "effects",
        "type_inference",
        "typeclass",
        "ffi",
        "text",
        "review",
    ]
    if args.section == "all":
        sections = section_order
    else:
        sections = [args.section]

    diagnostic_sections = {
        "parser",
        "lexer",
        "streaming",
        "iterator",
        "collectors",
        "effects",
        "diag",
        "type_inference",
        "ffi",
        "typeclass",
    }
    scenario_diagnostic_requirements = {
        "map_set_persistent",
        "vec_mem_exhaustion",
        "ref_internal_mutation",
        "audit_cap",
    }
    needs_diagnostic_sources = bool(
        set(sections) & diagnostic_sections
    ) or bool(scenario_filters & scenario_diagnostic_requirements)

    module_filters: Optional[List[str]] = None
    if getattr(args, "modules", None):
        normalized: Set[str] = set()
        for value in args.modules:
            if not value:
                continue
            normalized_value = value.strip().lower()
            if normalized_value:
                normalized.add(normalized_value)
        if normalized:
            module_filters = sorted(normalized)

    if args.summary:
        index_path = Path(args.summary)
        if not index_path.is_file():
            sys.stderr.write(f"Index file not found: {index_path}\n")
            return 2
        index_data = load_index(index_path)

        if args.prune:
            raw_entries = index_data.get("entries")
            if isinstance(raw_entries, list):
                normalized_entries = [
                    entry for entry in raw_entries if isinstance(entry, dict)
                ]
            else:
                normalized_entries = []
            retention_policy = load_retention_policy(args.retention_config)
            kept_entries, pruned_entries = prune_index_entries(
                normalized_entries, retention_policy
            )
            if pruned_entries:
                index_data["entries"] = kept_entries
                existing_pruned = index_data.get("pruned")
                pruned_log: List[str] = (
                    [item for item in existing_pruned if isinstance(item, str)]
                    if isinstance(existing_pruned, list)
                    else []
                )
                seen_ids = set(pruned_log)
                for entry in pruned_entries:
                    identifier = entry.get("build_id") or entry.get("id")
                    if isinstance(identifier, str) and identifier and identifier not in seen_ids:
                        pruned_log.append(identifier)
                        seen_ids.add(identifier)
                index_data["pruned"] = pruned_log
                if args.dry_run:
                    sys.stderr.write(
                        f"[dry-run] Would prune {len(pruned_entries)} entries from {index_path}\n"
                    )
                else:
                    write_index(index_path, index_data)
                    sys.stderr.write(
                        f"Pruned {len(pruned_entries)} entries from {index_path}\n"
                    )
            else:
                sys.stderr.write(
                    f"No entries pruned for {index_path}\n"
                )

        summary_text = generate_summary_markdown(index_data)
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(summary_text, encoding="utf-8")
        else:
            print(summary_text, end="")
        return 0

    platform_filters: Set[str] = set()
    if args.platforms:
        for item in args.platforms:
            if item is None:
                continue
            normalized = _normalize_platform(item)
            if normalized:
                platform_filters.add(normalized)
            elif isinstance(item, str):
                stripped = item.strip().lower()
                if stripped:
                    platform_filters.add(stripped)

    text_source_paths: List[Path] = _ensure_path_list(
        getattr(args, "text_sources", None)
    )
    if not text_source_paths and TEXT_DEFAULT_METRICS_PATH.is_file():
        text_source_paths = [TEXT_DEFAULT_METRICS_PATH]

    sources: List[Path] = []
    if needs_diagnostic_sources:
        if args.sources:
            sources = [Path(src) for src in args.sources]
        else:
            default_paths = [
                Path(
                    "compiler/ocaml/tests/golden/"
                    "typeclass_iterator_stage_mismatch.json.golden"
                ),
                Path(
                    "compiler/ocaml/tests/golden/diagnostics/ffi/"
                    "unsupported-abi.json.golden"
                ),
                Path(
                    "compiler/ocaml/tests/golden/diagnostics/effects/"
                    "syntax-constructs.json.golden"
                ),
            ]
            sources = [path for path in default_paths if path.is_file()]

        missing_paths = [str(path) for path in sources if not path.is_file()]
        if missing_paths:
            sys.stderr.write(
                "Missing input files: " + ", ".join(missing_paths) + "\n"
            )
            return 2
        if not sources:
            sys.stderr.write(
                "No default diagnostic sources found. "
                "Specify --source explicitly.\n"
            )
            return 2
    elif args.sources:
        sources = [Path(src) for src in args.sources]
        missing_paths = [str(path) for path in sources if not path.is_file()]
        if missing_paths:
            sys.stderr.write(
                "Missing input files: " + ", ".join(missing_paths) + "\n"
            )
            return 2

    audit_paths: List[Path] = []
    if args.audit_sources:
        audit_paths = [Path(src) for src in args.audit_sources]
        missing_audit = [str(path) for path in audit_paths if not path.is_file()]
        if missing_audit:
            sys.stderr.write(
                "Missing audit files: " + ", ".join(missing_audit) + "\n"
            )
            return 2

    review_diff_paths = _ensure_path_list(getattr(args, "review_diff", None))
    review_coverage_paths = _ensure_path_list(getattr(args, "review_coverage", None))
    review_dashboard_paths = _ensure_path_list(getattr(args, "review_dashboard", None))
    append_metrics: List[Dict[str, Any]] = []
    append_sources: List[str] = []
    if getattr(args, "append_from", None):
        for raw_path in args.append_from:
            path = Path(raw_path)
            if not path.is_file():
                sys.stderr.write(f"append-from ファイルが見つかりません: {path}\n")
                return 2
            try:
                data = load_json(path)
            except ValueError as exc:
                sys.stderr.write(f"append-from の読み込みに失敗しました: {exc}\n")
                return 2
            if isinstance(data, dict):
                append_metrics.append(data)
                append_sources.append(str(path))
            else:
                sys.stderr.write(f"append-from ファイルが dict ではありません: {path}\n")
                return 2

    runconfig_metrics: Optional[List[Dict[str, Any]]] = None
    lexer_metrics: List[Dict[str, Any]] = []
    if "parser" in sections or "lexer" in sections:
        runconfig_metrics = collect_runconfig_metrics(sources)

    parser_metrics: Optional[Dict[str, Any]] = None
    core_parser_metrics: List[Dict[str, Any]] = []
    orphan_parser_related_metrics: List[Dict[str, Any]] = []
    iterator_metrics: Optional[Dict[str, Any]] = None
    collector_metrics: Optional[Dict[str, Any]] = None
    collections_audit_metric: Optional[Dict[str, Any]] = None
    collector_effect_audit_metric: Optional[Dict[str, Any]] = None
    domain_metrics: Optional[Dict[str, Any]] = None
    effect_consistency_metric: Optional[Dict[str, Any]] = None
    capability_array_metric: Optional[Dict[str, Any]] = None
    plugin_bundle_metric: Optional[Dict[str, Any]] = None
    effects_metric: Optional[Dict[str, Any]] = None
    effect_stage_metric: Optional[Dict[str, Any]] = None
    bridge_stage_metric: Optional[Dict[str, Any]] = None
    typeck_debug_metric: Optional[Dict[str, Any]] = None
    type_inference_metric: Optional[Dict[str, Any]] = None
    value_restriction_legacy_metric: Optional[Dict[str, Any]] = None
    typeclass_metrics: Optional[Dict[str, Any]] = None
    bridge_metrics: Optional[Dict[str, Any]] = None
    review_metrics: Optional[Dict[str, Any]] = None
    diagnostic_presence_metric: Optional[Dict[str, Any]] = None
    streaming_metric: Optional[Dict[str, Any]] = None
    diag_metric: Optional[Dict[str, Any]] = None

    if "parser" in sections:
        parser_metrics = collect_parser_metrics(sources)
        core_parser_metrics = collect_core_parser_metrics(sources)
        runconfig_results = runconfig_metrics or []
        if parser_metrics:
            related = parser_metrics.setdefault("related_metrics", [])
            related.extend(runconfig_results)
            related.extend(core_parser_metrics)
        else:
            orphan_parser_related_metrics.extend(runconfig_results)
            orphan_parser_related_metrics.extend(core_parser_metrics)
    if "lexer" in sections:
        runconfig_results = runconfig_metrics or []
        lexer_metrics = [
            metric
            for metric in runconfig_results
            if isinstance(metric, dict)
            and str(metric.get("metric", "")).startswith("lexer.")
        ]
    if "streaming" in sections:
        streaming_metric = collect_streaming_metrics(sources, platform_filters)
    if "iterator" in sections:
        iterator_metrics = collect_metrics(sources)
        domain_metrics = collect_domain_metrics(sources)
        effect_consistency_metric = collect_effect_stage_consistency(sources)
        capability_array_metric = collect_capability_array_metric(sources)
        plugin_bundle_metric = collect_plugin_bundle_metrics(sources)
        if iterator_metrics:
            related = iterator_metrics.setdefault("related_metrics", [])
            for metric in (
                domain_metrics,
                effect_consistency_metric,
                capability_array_metric,
                plugin_bundle_metric,
            ):
                if metric:
                    related.append(metric)
    if "collectors" in sections or "collector" in sections:
        collector_metrics = collect_collector_effect_metrics(sources)
    if "map_set_persistent" in scenario_filters:
        (
            collections_audit_metric,
            collector_effect_audit_metric,
        ) = collect_collections_audit_bridge_metrics(sources, kinds={"map", "set"})
        for metric in (collections_audit_metric, collector_effect_audit_metric):
            if metric:
                append_metrics.append(metric)
    if "vec_mem_exhaustion" in scenario_filters:
        vec_metric = collect_vec_effect_metrics(sources)
        if vec_metric:
            append_metrics.append(vec_metric)
    if "ref_internal_mutation" in scenario_filters:
        cell_ref_metric = collect_cell_ref_effect_metrics(sources)
        if cell_ref_metric:
            append_metrics.append(cell_ref_metric)
    if "audit_cap" in scenario_filters:
        audit_cap_metric = collect_audit_capability_metric(sources)
        if audit_cap_metric:
            append_metrics.append(audit_cap_metric)

    text_metric: Optional[Dict[str, Any]] = None
    text_section_requested = "text" in sections or "grapheme_stats" in scenario_filters
    if text_section_requested:
        if not text_source_paths:
            sys.stderr.write(
                "Text metrics source not found. 指定ファイルを --text-source で渡すか "
                f"{TEXT_DEFAULT_METRICS_PATH} を生成してください。\n"
            )
            return 2
        missing_text_sources = [
            str(path) for path in text_source_paths if not path.is_file()
        ]
        if missing_text_sources:
            sys.stderr.write(
                "Missing text metrics files: " + ", ".join(missing_text_sources) + "\n"
            )
            return 2
        text_metric = collect_text_grapheme_metrics(text_source_paths)
        if text_metric is None:
            sys.stderr.write(
                "Text metrics file did not contain any cases. 再生成してください。\n"
            )
            return 2
        if "grapheme_stats" in scenario_filters:
            scenario_metric = copy.deepcopy(text_metric)
            scenario_metric["scenario"] = "grapheme_stats"
            append_metrics.append(scenario_metric)
    if "effects" in sections:
        effects_metric = collect_effect_syntax_metrics(sources)
        (
            effect_row_stage_metric,
            type_effect_row_metric,
            guard_metric,
        ) = collect_effect_row_metrics(sources)
        effect_scope_metric = collect_effect_scope_audit_presence(sources)
        effect_stage_metric = collect_effect_stage_extension_presence(sources)
        bridge_stage_metric = collect_bridge_stage_extension_presence(sources)
        typeck_debug_metric = collect_typeck_debug_metric(sources)
        related_metrics_target: List[Dict[str, Any]] = []
        if effects_metric:
            related_metrics_target = effects_metric.setdefault("related_metrics", [])
        for metric in (
            effect_row_stage_metric,
            type_effect_row_metric,
            effect_scope_metric,
            effect_stage_metric,
            bridge_stage_metric,
            typeck_debug_metric,
        ):
            if not metric:
                continue
            append_metrics.append(metric)
            if effects_metric:
                related_metrics_target.append(metric)
        if guard_metric:
            append_metrics.append(guard_metric)
    if "type_inference" in sections:
        type_inference_metric = collect_value_restriction_violation_metric(sources)
        value_restriction_legacy_metric = collect_value_restriction_legacy_metric(
            sources
        )
        if type_inference_metric and value_restriction_legacy_metric:
            related = type_inference_metric.setdefault("related_metrics", [])
            related.append(value_restriction_legacy_metric)
    if "typeclass" in sections:
        typeclass_metrics = collect_typeclass_metrics(sources, audit_paths)
    if "ffi" in sections:
        bridge_metrics = collect_bridge_metrics(sources, audit_paths, platform_filters)
    if "review" in sections:
        review_metrics = collect_review_metrics(
            review_diff_paths, review_coverage_paths, review_dashboard_paths
        )
    if "diag" in sections:
        diag_metric = collect_diag_metrics(sources, args.metrics_case)
    if sections == ['effects']:
        diagnostic_presence_metric = None
    else:
        diagnostic_presence_metric = collect_diagnostic_audit_presence_metric(sources)

    if sections == ['effects']:
        diagnostics_summary = None
    else:
        diagnostics_summary = summarize_diagnostics(sources)

    metrics_list: List[Dict[str, Any]] = []
    if diagnostic_presence_metric:
        metrics_list.append(diagnostic_presence_metric)
    if parser_metrics:
        metrics_list.append(parser_metrics)
    if streaming_metric:
        metrics_list.append(streaming_metric)
    elif orphan_parser_related_metrics:
        metrics_list.extend(orphan_parser_related_metrics)
    if capability_array_metric and iterator_metrics is None:
        metrics_list.append(capability_array_metric)
    if iterator_metrics:
        metrics_list.append(iterator_metrics)
    if collector_metrics:
        metrics_list.append(collector_metrics)
    if effects_metric:
        metrics_list.append(effects_metric)
    if type_inference_metric:
        metrics_list.append(type_inference_metric)
    elif value_restriction_legacy_metric:
        metrics_list.append(value_restriction_legacy_metric)
    if typeclass_metrics:
        metrics_list.append(typeclass_metrics)
    if bridge_metrics:
        metrics_list.append(bridge_metrics)
    if review_metrics:
        metrics_list.append(review_metrics)
    if diag_metric:
        metrics_list.append(diag_metric)
    if text_metric and "text" in sections:
        metrics_list.append(text_metric)

    if lexer_metrics and "parser" not in sections:
        metrics_list.extend(lexer_metrics)

    all_metrics: List[Dict[str, Any]] = []
    for metric in metrics_list:
        all_metrics.append(metric)
        related = metric.get("related_metrics") if isinstance(metric, dict) else None
        if isinstance(related, list):
            all_metrics.extend(
                related_metric
                for related_metric in related
                if isinstance(related_metric, dict)
            )
    all_metrics.extend(append_metrics)

    combined: Dict[str, Any] = {"metrics": all_metrics}
    if append_metrics:
        combined["extra_metrics"] = append_metrics
    if append_sources:
        combined["extra_metrics_sources"] = append_sources
    if module_filters:
        combined["modules"] = module_filters
    if scenario_filters:
        combined["scenarios"] = sorted(scenario_filters)
    if platform_filters:
        combined["platform_filters"] = sorted(platform_filters)
    if getattr(args, "case", None):
        combined["case"] = args.case

    primary_metrics: Optional[Dict[str, Any]] = iterator_metrics
    if primary_metrics is None and metrics_list:
        primary_metrics = metrics_list[0]

    if primary_metrics:
        combined.update(
            {
                "metric": primary_metrics.get("metric"),
                "total": primary_metrics.get("total"),
                "passed": primary_metrics.get("passed"),
                "failed": primary_metrics.get("failed"),
                "pass_rate": primary_metrics.get("pass_rate"),
                "pass_fraction": primary_metrics.get("pass_fraction"),
                "required_audit_keys": primary_metrics.get("required_audit_keys"),
                "sources": primary_metrics.get("sources"),
                "failures": primary_metrics.get("failures"),
            }
        )

    if diagnostic_presence_metric:
        combined["diagnostic_audit"] = diagnostic_presence_metric
    if parser_metrics:
        combined["parser"] = parser_metrics
    if diag_metric:
        combined["diagnostics"] = diag_metric
    if lexer_metrics:
        combined["lexer"] = {"metrics": lexer_metrics}
    if iterator_metrics:
        combined["iterator"] = iterator_metrics
    if collector_metrics:
        combined["collector"] = collector_metrics
    if streaming_metric:
        combined["streaming"] = streaming_metric
    if effects_metric:
        combined["effects"] = effects_metric
    if type_inference_metric:
        combined["type_inference"] = type_inference_metric
    if value_restriction_legacy_metric:
        combined["type_inference_legacy"] = value_restriction_legacy_metric
    if typeclass_metrics:
        combined["typeclass"] = typeclass_metrics
    if bridge_metrics:
        combined["ffi_bridge"] = bridge_metrics
    if review_metrics:
        combined["audit_review"] = review_metrics
    if text_metric and "text" in sections:
        combined["text"] = text_metric

    combined_audit_sources: List[str] = []
    for metrics in (iterator_metrics, typeclass_metrics, bridge_metrics):
        if metrics:
            combined_audit_sources.extend(metrics.get("audit_sources") or [])
    if append_sources:
        combined_audit_sources.extend(append_sources)
    if combined_audit_sources:
        combined["audit_sources"] = sorted({*combined_audit_sources})

    schema_versions: Set[str] = set()
    for metrics in all_metrics:
        schema_versions.update(metrics.get("schema_versions") or [])
    combined["schema_versions"] = sorted(schema_versions)
    if diagnostics_summary:
        combined["diagnostics"] = diagnostics_summary

    ci_info: Dict[str, Any] = {}
    duration_bucket: Dict[str, Any] = {}
    if getattr(args, "ci_duration_seconds", None) is not None:
        ci_info["duration_seconds"] = args.ci_duration_seconds
        duration_bucket["total_seconds"] = args.ci_duration_seconds
    if getattr(args, "stage_duration_seconds", None) is not None:
        ci_info["stage_duration_seconds"] = args.stage_duration_seconds
        duration_bucket["stage_seconds"] = args.stage_duration_seconds
    if duration_bucket:
        ci_info["duration"] = duration_bucket
    if ci_info:
        combined["ci"] = ci_info

    failure_reasons: List[str] = []
    if getattr(args, "require_success", False):
        def _enforce(metric: Optional[Dict[str, Any]], label: str) -> None:
            if not isinstance(metric, dict):
                return
            metric_name = metric.get("metric")
            if metric_name == "lexer.identifier_profile_unicode":
                return
            total = metric.get("total")
            pass_rate = metric.get("pass_rate")
            if isinstance(total, (int, float)) and total > 0:
                if not isinstance(pass_rate, (int, float)) or pass_rate < 1.0:
                    failure_reasons.append(f"{label} < 1.0")

        _enforce(diagnostic_presence_metric, "diagnostic.audit_presence_rate")
        _enforce(parser_metrics, "parser.expected_summary_presence")
        runconfig_targets = (
            runconfig_metrics if runconfig_metrics else orphan_parser_related_metrics
        )
        for metric in runconfig_targets:
            if isinstance(metric, dict):
                label = metric.get("metric") or "parser.runconfig"
                _enforce(metric, label)
        _enforce(streaming_metric, "parser.stream.outcome_consistency")
        if streaming_metric:
            for related_metric in streaming_metric.get("related_metrics") or []:
                if (
                    isinstance(related_metric, dict)
                    and related_metric.get("metric")
                    == "ExpectedTokenCollector.streaming"
                ):
                    _enforce(
                        related_metric, "ExpectedTokenCollector.streaming"
                    )
        _enforce(iterator_metrics, "iterator.stage.audit_pass_rate")
        _enforce(capability_array_metric, "effect.capability_array_pass_rate")
        _enforce(effect_stage_metric, "effect_stage.audit_presence")
        _enforce(bridge_stage_metric, "bridge_stage.audit_presence")
        _enforce(effects_metric, "syntax.effect_construct_acceptance")
        _enforce(typeck_debug_metric, "typeck_debug_match")
        _enforce(type_inference_metric, "type_inference.value_restriction_violation")
        _enforce(typeclass_metrics, "typeclass.metadata_pass_rate")
        if isinstance(typeclass_metrics, dict):
            _enforce(typeclass_metrics.get("dictionary_metric"), "typeclass.dictionary_pass_rate")
        _enforce(bridge_metrics, "ffi_bridge.audit_pass_rate")
        _enforce(collections_audit_metric, "collections.audit_bridge_pass_rate")
        _enforce(
            collector_effect_audit_metric, "collector.effect.audit_presence"
        )
        _enforce(diag_metric, "effects-contract")
        for metric in append_metrics:
            if not isinstance(metric, dict):
                continue
            status = metric.get("status")
            metric_name = str(metric.get("metric", "extra_metric"))
            if isinstance(status, str):
                status_norm = status.strip().lower()
                if status_norm not in ("success", "ok", "passed"):
                    failure_reasons.append(f"{metric_name}: status={status}")
            else:
                failure_reasons.append(f"{metric_name}: status=<missing>")
        if failure_reasons:
            combined.setdefault("enforcement", {})
            combined["enforcement"]["failures"] = failure_reasons
            combined["enforcement"]["require_success"] = True
        else:
            combined.setdefault("enforcement", {})
            combined["enforcement"]["require_success"] = True

    json_output = json.dumps(combined, indent=2, ensure_ascii=False)

    print(json_output)

    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with args.output.open("w", encoding="utf-8") as handle:
            handle.write(json_output)
            handle.write("\n")

    if getattr(args, "require_cell", False):
        ref_metric: Optional[Dict[str, Any]] = next(
            (
                metric
                for metric in append_metrics
                if isinstance(metric, dict)
                and metric.get("scenario") == "ref_internal_mutation"
            ),
            None,
        )
        if ref_metric is None:
            sys.stderr.write(
                "[collect-iterator-audit-metrics] ref_internal_mutation metric is missing\n"
            )
            return 1
        status = str(ref_metric.get("status", "")).strip().lower()

        if status not in ("success", "ok", "passed"):
            sys.stderr.write(
                f"[collect-iterator-audit-metrics] ref_internal_mutation: status={ref_metric.get('status')}\n"
            )
            return 1

    # When --require-success is specified, fail if critical metrics did not pass.
    if getattr(args, "require_success", False) and failure_reasons:
        for reason in failure_reasons:
            sys.stderr.write(f"[collect-iterator-audit-metrics] {reason}\n")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
