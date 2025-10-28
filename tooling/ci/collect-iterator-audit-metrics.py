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
"""

from __future__ import annotations

import argparse
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
    Field("effect.stage.required", ("effect.stage.required",)),
    Field("effect.stage.actual", ("effect.stage.actual",)),
    Field("effect.capability", ("effect.capability",)),
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
    if isinstance(domain, str) and domain.strip().lower() == "parser":
        return True
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
        for index, diag in enumerate(iter_diagnostics(data)):
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
        for index, diag in enumerate(iter_diagnostics(data)):
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

    pass_fraction = (passed / total) if total > 0 else 0.0
    pass_rate = 1.0 if total > 0 and passed == total else 0.0
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
        "status": "success" if total > 0 and pass_rate == 1.0 else "error",
    }

    related_status: str
    if passed == 0:
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

    for path in paths:
        data = load_json(path)

        run_config = _as_dict(data.get("run_config"))
        if run_config:
            _mark_switch_from_dict(_as_dict(run_config.get("switches")))
            extensions_container = run_config.get("extensions")
            if isinstance(extensions_container, dict):
                _mark_extension_from_dict(extensions_container)

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

    return metrics


def collect_metrics(paths: List[Path]) -> Dict:
    total = 0
    passed = 0
    failures: List[Dict[str, object]] = []
    schema_versions: Set[str] = set()

    for path in paths:
        data = load_json(path)
        for index, diag in enumerate(iter_diagnostics(data)):
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


def collect_bridge_metrics(
    paths: List[Path], audit_paths: List[Path]
) -> Dict:
    total = 0
    passed = 0
    failures: List[Dict[str, object]] = []
    platform_summary: Dict[str, Dict[str, int]] = {}
    audit_sources: List[str] = []
    schema_versions: Set[str] = set()
    status_success = 0
    status_failure = 0

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
        for index, diag in enumerate(iter_diagnostics(data)):
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
            _tally_status(status_value)

            platform_key = (
                str(platform_value)
                if isinstance(platform_value, str) and platform_value
                else "<unknown>"
            )
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
                    }
                )

    pass_rate, pass_fraction = calculate_pass_rates(passed, total)

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
            _tally_status(status_value)

            platform_key = (
                str(platform_value)
                if isinstance(platform_value, str) and platform_value
                else "<unknown>"
            )
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
                }
            )

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
        for index, diag in enumerate(iter_diagnostics(data)):
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
        for index, diag in enumerate(iter_diagnostics(data)):
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
        choices=["all", "parser", "iterator", "ffi", "typeclass", "review"],
        default="all",
        help="Collect metrics for a specific section (default: all).",
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
    return parser.parse_args(argv)


def main(argv: Optional[List[str]] = None) -> int:
    args = parse_args(argv)
    if args.prune and not args.summary:
        sys.stderr.write("--prune を使用する場合は --summary でインデックスを指定してください。\n")
        return 2

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

    sources: List[Path]
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

    section_order = ["parser", "iterator", "typeclass", "ffi", "review"]
    if args.section == "all":
        sections = section_order
    else:
        sections = [args.section]

    parser_metrics: Optional[Dict[str, Any]] = None
    runconfig_metrics: List[Dict[str, Any]] = []
    orphan_runconfig_metrics: List[Dict[str, Any]] = []
    iterator_metrics: Optional[Dict[str, Any]] = None
    typeclass_metrics: Optional[Dict[str, Any]] = None
    bridge_metrics: Optional[Dict[str, Any]] = None
    review_metrics: Optional[Dict[str, Any]] = None
    diagnostic_presence_metric: Optional[Dict[str, Any]] = None

    if "parser" in sections:
        parser_metrics = collect_parser_metrics(sources)
        runconfig_metrics = collect_runconfig_metrics(sources)
        if parser_metrics and runconfig_metrics:
            related = parser_metrics.setdefault("related_metrics", [])
            related.extend(runconfig_metrics)
        elif runconfig_metrics:
            orphan_runconfig_metrics = runconfig_metrics.copy()
    if "iterator" in sections:
        iterator_metrics = collect_metrics(sources)
    if "typeclass" in sections:
        typeclass_metrics = collect_typeclass_metrics(sources, audit_paths)
    if "ffi" in sections:
        bridge_metrics = collect_bridge_metrics(sources, audit_paths)
    if "review" in sections:
        review_metrics = collect_review_metrics(
            review_diff_paths, review_coverage_paths, review_dashboard_paths
        )
    diagnostic_presence_metric = collect_diagnostic_audit_presence_metric(sources)

    diagnostics_summary = summarize_diagnostics(sources)

    metrics_list: List[Dict[str, Any]] = []
    if diagnostic_presence_metric:
        metrics_list.append(diagnostic_presence_metric)
    if parser_metrics:
        metrics_list.append(parser_metrics)
    elif orphan_runconfig_metrics:
        metrics_list.extend(orphan_runconfig_metrics)
    if iterator_metrics:
        metrics_list.append(iterator_metrics)
    if typeclass_metrics:
        metrics_list.append(typeclass_metrics)
    if bridge_metrics:
        metrics_list.append(bridge_metrics)
    if review_metrics:
        metrics_list.append(review_metrics)

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
    if iterator_metrics:
        combined["iterator"] = iterator_metrics
    if typeclass_metrics:
        combined["typeclass"] = typeclass_metrics
    if bridge_metrics:
        combined["ffi_bridge"] = bridge_metrics
    if review_metrics:
        combined["audit_review"] = review_metrics

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
            total = metric.get("total")
            pass_rate = metric.get("pass_rate")
            if isinstance(total, (int, float)) and total > 0:
                if not isinstance(pass_rate, (int, float)) or pass_rate < 1.0:
                    failure_reasons.append(f"{label} < 1.0")

        _enforce(diagnostic_presence_metric, "diagnostic.audit_presence_rate")
        _enforce(parser_metrics, "parser.expected_summary_presence")
        runconfig_targets = (
            runconfig_metrics if runconfig_metrics else orphan_runconfig_metrics
        )
        for metric in runconfig_targets:
            if isinstance(metric, dict):
                label = metric.get("metric") or "parser.runconfig"
                _enforce(metric, label)
        _enforce(iterator_metrics, "iterator.stage.audit_pass_rate")
        _enforce(typeclass_metrics, "typeclass.metadata_pass_rate")
        if isinstance(typeclass_metrics, dict):
            _enforce(typeclass_metrics.get("dictionary_metric"), "typeclass.dictionary_pass_rate")
        _enforce(bridge_metrics, "ffi_bridge.audit_pass_rate")
        if isinstance(parser_metrics, dict):
            total = parser_metrics.get("total")
            if isinstance(total, (int, float)) and total == 0:
                failure_reasons.append("parser.expected_summary_presence: total=0")
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

    # When --require-success is specified, fail if critical metrics did not pass.
    if getattr(args, "require_success", False) and failure_reasons:
        for reason in failure_reasons:
            sys.stderr.write(f"[collect-iterator-audit-metrics] {reason}\n")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
