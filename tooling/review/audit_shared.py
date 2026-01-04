from __future__ import annotations
import json
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, Iterable, List, Mapping, Optional, Tuple


@dataclass
class NormalizedAuditEntry:
    category: str
    code: Optional[str]
    severity: Optional[str]
    timestamp: Optional[datetime]
    audit_id: Optional[str]
    cli_audit_id: Optional[str]
    change_set: Optional[str]
    pass_rate: Optional[float]
    metadata: Dict[str, Any]
    extensions: Dict[str, Any]
    source: Path
    raw: Dict[str, Any]

    def identity(self) -> Tuple[str, Optional[str]]:
        return (self.category, self.code)


@dataclass
class AuditDiffSummary:
    path: Path
    schema_version: Optional[str]
    diagnostic_regressions: int
    diagnostic_new: int
    metadata_changed: int
    pass_rate_previous: Optional[float]
    pass_rate_current: Optional[float]
    pass_rate_delta: Optional[float]


@dataclass
class CoverageEntry:
    preset: str
    matched: int
    total: int

    @property
    def ratio(self) -> Optional[float]:
        if self.total == 0:
            return None
        return self.matched / self.total


def _read_json_lines(path: Path) -> List[Dict[str, Any]]:
    text = path.read_text(encoding="utf-8").strip()
    if not text:
        return []
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        result: List[Dict[str, Any]] = []
        for line in text.splitlines():
            line = line.strip()
            if not line:
                continue
            obj = json.loads(line)
            if isinstance(obj, dict):
                result.append(obj)
        return result
    else:
        if isinstance(data, list):
            return [item for item in data if isinstance(item, dict)]
        if isinstance(data, dict):
            return [data]
        return []


def _to_datetime(value: Optional[str]) -> Optional[datetime]:
    if not value or not isinstance(value, str):
        return None
    try:
        return datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None


def _ensure_dict(value: Any) -> Dict[str, Any]:
    if isinstance(value, dict):
        return value
    return {}


def _extract_pass_rate(container: Mapping[str, Any]) -> Optional[float]:
    candidates = (
        container.get("pass_rate"),
        container.get("bridge.audit_pass_rate"),
        container.get("audit_pass_rate"),
    )
    for candidate in candidates:
        if candidate is None:
            continue
        if isinstance(candidate, (int, float)):
            return float(candidate)
        if isinstance(candidate, str):
            try:
                return float(candidate)
            except ValueError:
                continue
    return None


def _extract_category(entry: Mapping[str, Any]) -> str:
    metadata = _ensure_dict(entry.get("metadata"))
    category = metadata.get("category")
    if isinstance(category, str) and category:
        return category
    category = entry.get("category")
    if isinstance(category, str) and category:
        return category
    code = entry.get("code")
    if isinstance(code, str) and code:
        return code
    return "<unknown>"


def _extract_code(entry: Mapping[str, Any]) -> Optional[str]:
    code = entry.get("code")
    if isinstance(code, str) and code:
        return code
    metadata = _ensure_dict(entry.get("metadata"))
    code = metadata.get("code")
    if isinstance(code, str) and code:
        return code
    return None


def _extract_severity(entry: Mapping[str, Any]) -> Optional[str]:
    severity = entry.get("severity")
    if isinstance(severity, str) and severity:
        return severity
    metadata = _ensure_dict(entry.get("metadata"))
    severity = metadata.get("severity")
    if isinstance(severity, str) and severity:
        return severity
    return None


def _extract_audit_id(entry: Mapping[str, Any]) -> Tuple[Optional[str], Optional[str]]:
    audit_id = entry.get("audit_id")
    cli_audit_id = None
    metadata = _ensure_dict(entry.get("metadata"))
    if isinstance(audit_id, str) and audit_id:
        pass
    else:
        raw = metadata.get("audit_id")
        if isinstance(raw, str) and raw:
            audit_id = raw
    cli = metadata.get("cli.audit_id")
    if isinstance(cli, str) and cli:
        cli_audit_id = cli
    else:
        extensions = _ensure_dict(entry.get("extensions"))
        cli = extensions.get("cli.audit_id")
        if isinstance(cli, str) and cli:
            cli_audit_id = cli
    return audit_id, cli_audit_id


def _extract_change_set(entry: Mapping[str, Any]) -> Optional[str]:
    metadata = _ensure_dict(entry.get("metadata"))
    change_set = metadata.get("cli.change_set") or metadata.get("change_set")
    if isinstance(change_set, str) and change_set:
        return change_set
    return None


def _normalize_entry(raw: Dict[str, Any], source: Path) -> NormalizedAuditEntry:
    metadata = _ensure_dict(raw.get("metadata"))
    extensions = _ensure_dict(raw.get("extensions"))
    category = _extract_category(raw)
    code = _extract_code(raw)
    severity = _extract_severity(raw)

    timestamp_value = raw.get("timestamp") or metadata.get("timestamp")
    timestamp = _to_datetime(timestamp_value if isinstance(timestamp_value, str) else None)

    audit_id, cli_audit_id = _extract_audit_id(raw)
    change_set = _extract_change_set(raw)
    pass_rate = _extract_pass_rate(metadata)
    if pass_rate is None:
        pass_rate = _extract_pass_rate(extensions.get("bridge", {}))

    return NormalizedAuditEntry(
        category=category,
        code=code,
        severity=severity,
        timestamp=timestamp,
        audit_id=audit_id,
        cli_audit_id=cli_audit_id,
        change_set=change_set,
        pass_rate=pass_rate,
        metadata=metadata,
        extensions=extensions,
        source=source,
        raw=raw,
    )


def load_entries(path: Path) -> List[NormalizedAuditEntry]:
    records = _read_json_lines(path)
    result: List[NormalizedAuditEntry] = []
    for record in records:
        result.append(_normalize_entry(record, path))
    return result


def index_by_category(entries: Iterable[NormalizedAuditEntry]) -> Dict[str, List[NormalizedAuditEntry]]:
    index: Dict[str, List[NormalizedAuditEntry]] = {}
    for entry in entries:
        index.setdefault(entry.category, []).append(entry)
    return index


def flatten_metadata(entry: NormalizedAuditEntry, *, include_extensions: bool = True) -> Dict[str, str]:
    flattened: Dict[str, str] = {}

    def walk(prefix: str, value: Any) -> None:
        if isinstance(value, dict):
            for key, child in value.items():
                if not isinstance(key, str):
                    continue
                walk(f"{prefix}.{key}" if prefix else key, child)
        elif isinstance(value, list):
            flattened[prefix] = json.dumps(value, sort_keys=True, ensure_ascii=False)
        else:
            flattened[prefix] = "" if value is None else str(value)

    walk("", entry.metadata)
    if include_extensions:
        walk("extensions", entry.extensions)
    return flattened


def load_diff_manifest(path: Path) -> AuditDiffSummary:
    data = json.loads(path.read_text(encoding="utf-8"))
    diagnostic = data.get("diagnostic") if isinstance(data, dict) else {}
    metadata = data.get("metadata") if isinstance(data, dict) else {}
    pass_rate = data.get("pass_rate") if isinstance(data, dict) else {}
    return AuditDiffSummary(
        path=path,
        schema_version=data.get("schema_version") if isinstance(data, dict) else None,
        diagnostic_regressions=int(diagnostic.get("regressions") or 0),
        diagnostic_new=int(diagnostic.get("new") or 0),
        metadata_changed=int(metadata.get("changed") or 0),
        pass_rate_previous=(
            float(pass_rate.get("previous"))
            if isinstance(pass_rate, dict) and pass_rate.get("previous") is not None
            else None
        ),
        pass_rate_current=(
            float(pass_rate.get("current"))
            if isinstance(pass_rate, dict) and pass_rate.get("current") is not None
            else None
        ),
        pass_rate_delta=(
            float(pass_rate.get("delta"))
            if isinstance(pass_rate, dict) and pass_rate.get("delta") is not None
            else None
        ),
    )


def load_coverage_report(path: Path) -> List[CoverageEntry]:
    data = json.loads(path.read_text(encoding="utf-8"))
    result: List[CoverageEntry] = []
    if isinstance(data, list):
        iterable = data
    elif isinstance(data, dict) and isinstance(data.get("coverage"), list):
        iterable = data["coverage"]
    elif isinstance(data, dict):
        iterable = [data]
    else:
        iterable = []
    for entry in iterable:
        if not isinstance(entry, dict):
            continue
        preset = entry.get("preset") or entry.get("name") or entry.get("id") or "<unknown>"
        matched = int(entry.get("matched") or entry.get("hits") or 0)
        total = int(entry.get("total") or entry.get("count") or 0)
        result.append(CoverageEntry(preset=preset, matched=matched, total=total))
    return result
