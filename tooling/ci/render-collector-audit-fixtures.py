#!/usr/bin/env python3
"""
core_iter_collectors のスナップショットを診断 JSON / 監査ログへ変換する補助スクリプト。

`compiler/frontend/tests/__snapshots__/core_iter_collectors.snap` をパースし、
`tooling/ci/collect-iterator-audit-metrics.py` で扱いやすい JSON 形式を生成する。
"""

from __future__ import annotations

import argparse
import json
import uuid
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Tuple


BOOL_EFFECT_KEYS = {
    "mem",
    "mut",
    "debug",
    "async_pending",
    "audit",
    "cell",
    "rc",
}
INT_EFFECT_KEYS = {
    "predicate_calls",
    "mem_bytes",
    "cell_mutations",
    "rc_ops",
}
SNAPSHOT_BASE_TIMESTAMP = datetime(2025, 12, 3, tzinfo=timezone.utc)
SNAPSHOT_PROGRAM_NAME = "collector.snapshot-fixture"
SNAPSHOT_COMMAND = "render-collector-audit-fixtures"
SNAPSHOT_PHASE = "spec-fixture"
SNAPSHOT_AUDIT_CHANNEL = "collector.snapshots"
SNAPSHOT_AUDIT_POLICY = "collector.spec.audit.v1"
SNAPSHOT_CHANGE_SET_POLICY = "collector.snapshot.change_set.v1"
SNAPSHOT_SCHEMA_VERSION = "collector.snapshot.v1"
SNAPSHOT_RUN_ID = str(
    uuid.uuid5(uuid.NAMESPACE_URL, "collector.snapshot.fixtures.run_id")
)


def _as_dict(value: Any) -> Optional[Dict[str, Any]]:
    return value if isinstance(value, dict) else None


def _as_bool(value: Any) -> bool:
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


def _as_int(value: Any) -> int:
    if isinstance(value, bool):
        return int(value)
    if isinstance(value, (int, float)):
        return int(value)
    if isinstance(value, str):
        stripped = value.strip()
        try:
            return int(stripped, 0)
        except ValueError:
            return 0
    return 0


def parse_snapshot_entries(path: Path) -> List[Tuple[str, Any]]:
    text = path.read_text(encoding="utf-8")
    entries: List[Tuple[str, Any]] = []
    current_name: Optional[str] = None
    buffer: List[str] = []
    depth = 0

    for raw_line in text.splitlines():
        line = raw_line.rstrip("\n")
        if current_name is None:
            stripped = line.strip()
            if not stripped:
                continue
            if ":" not in line or line.startswith(" "):
                continue
            name, rest = line.split(":", 1)
            current_name = name.strip()
            buffer = []
            rest = rest.strip()
            if rest:
                buffer.append(rest)
                depth = rest.count("{") - rest.count("}")
            else:
                depth = 0
            continue

        buffer.append(line)
        depth += line.count("{") - line.count("}")
        if depth <= 0:
            payload_text = "\n".join(buffer).strip()
            if not payload_text:
                raise ValueError(f"Empty payload for snapshot {current_name}")
            try:
                payload = json.loads(payload_text)
            except json.JSONDecodeError as exc:
                raise ValueError(
                    f"Failed to parse JSON block for {current_name}: {exc}"
                ) from exc
            entries.append((current_name, payload))
            current_name = None
            buffer = []
            depth = 0

    if current_name is not None:
        raise ValueError(f"Incomplete snapshot block for {current_name}")
    return entries


def _normalize_effects(raw: Any) -> Dict[str, Any]:
    result: Dict[str, Any] = {
        "mem": False,
        "mut": False,
        "debug": False,
        "async_pending": False,
        "audit": False,
        "cell": False,
        "cell_mutations": 0,
        "rc": False,
        "rc_ops": 0,
        "predicate_calls": 0,
        "mem_bytes": 0,
    }
    data = _as_dict(raw)
    if not data:
        return result
    for key, value in data.items():
        normalized_key = key
        if key == "mutating":
            normalized_key = "mut"
        if normalized_key in BOOL_EFFECT_KEYS:
            result[normalized_key] = _as_bool(value)
            continue
        if normalized_key in INT_EFFECT_KEYS:
            result[normalized_key] = _as_int(value)
            continue
        result[normalized_key] = value
    return result


def _normalize_markers(raw: Any) -> Dict[str, int]:
    result = {"finish": 0, "mem_reservation": 0, "reserve": 0}
    data = _as_dict(raw)
    if not data:
        return result
    for key in result:
        result[key] = _as_int(data.get(key))
    return result


def _stable_snapshot_timestamp(sequence: int) -> str:
    base = SNAPSHOT_BASE_TIMESTAMP + timedelta(seconds=sequence)
    return base.strftime("%Y-%m-%dT%H:%M:%SZ")


def _build_snapshot_change_set(
    snapshot_id: str, prelude: Dict[str, Any], timestamp: str, sequence: int
) -> Dict[str, Any]:
    return {
        "policy": SNAPSHOT_CHANGE_SET_POLICY,
        "origin": "spec-fixture",
        "source": {
            "command": SNAPSHOT_COMMAND,
            "args": ["--snapshot", snapshot_id],
            "workspace": ".",
        },
        "run_id": SNAPSHOT_RUN_ID,
        "items": [
            {
                "kind": "collector.snapshot",
                "snapshot_id": snapshot_id,
                "capability": prelude.get("capability"),
                "stage_actual": prelude.get("stage_actual"),
                "stage_required": prelude.get("stage_required"),
                "stage_mode": prelude.get("stage_mode"),
                "collector_kind": prelude.get("kind"),
                "sequence": sequence,
                "timestamp": timestamp,
            }
        ],
    }


def _apply_snapshot_audit_metadata(
    metadata: Dict[str, Any],
    prelude: Dict[str, Any],
    snapshot_id: str,
    sequence: int,
) -> Tuple[str, Dict[str, Any], str]:
    timestamp = _stable_snapshot_timestamp(sequence)
    metadata.setdefault("schema.version", SNAPSHOT_SCHEMA_VERSION)
    metadata.setdefault("event.domain", "core.collectors")
    metadata.setdefault("event.kind", "collector.snapshot")
    metadata.setdefault("event.category", "diagnostic")
    metadata.setdefault("audit.channel", SNAPSHOT_AUDIT_CHANNEL)
    metadata.setdefault("audit.policy.version", SNAPSHOT_AUDIT_POLICY)
    metadata["audit.timestamp"] = timestamp
    metadata["audit.sequence"] = metadata.get("audit.sequence") or sequence
    metadata.setdefault("cli.program", SNAPSHOT_PROGRAM_NAME)
    metadata.setdefault("cli.command", SNAPSHOT_COMMAND)
    metadata.setdefault("cli.phase", SNAPSHOT_PHASE)
    metadata.setdefault("cli.args", ["--snapshot", snapshot_id])
    metadata.setdefault("cli.run_id", SNAPSHOT_RUN_ID)

    change_set = _build_snapshot_change_set(snapshot_id, prelude, timestamp, sequence)
    stored_change_set = metadata.get("cli.change_set")
    if not isinstance(stored_change_set, dict) or not stored_change_set:
        metadata["cli.change_set"] = change_set
        stored_change_set = change_set

    audit_label = metadata.get("cli.audit_id")
    if not isinstance(audit_label, str) or not audit_label.strip():
        build_id = timestamp.replace("-", "").replace(":", "")
        audit_label = f"{SNAPSHOT_AUDIT_CHANNEL}/{build_id}#{sequence}"
        metadata["cli.audit_id"] = audit_label
    audit_uuid = uuid.uuid5(uuid.NAMESPACE_URL, audit_label)
    metadata["audit.id.uuid"] = str(audit_uuid)
    metadata["audit.id.label"] = audit_label

    return timestamp, stored_change_set, audit_label


def _build_metadata(
    prelude: Dict[str, Any], include_effect_stage: bool, snapshot_id: Optional[str] = None
) -> Dict[str, Any]:
    metadata: Dict[str, Any] = {}

    def _set(key: str, value: Any) -> None:
        if value is None:
            return
        metadata[key] = value

    _set("collector.kind", prelude.get("kind"))
    _set("collector.capability", prelude.get("capability"))
    _set("collector.stage.actual", prelude.get("stage_actual"))
    _set("collector.stage.required", prelude.get("stage_required"))
    _set("collector.stage.mode", prelude.get("stage_mode"))
    _set("collector.stage.source", prelude.get("source"))
    _set("collector.stage.mismatch", prelude.get("stage_mismatch"))

    effects = _as_dict(prelude.get("effects"))
    if effects:
        for key, value in effects.items():
            metadata[f"collector.effect.{key}"] = value

    markers = _as_dict(prelude.get("markers"))
    if markers:
        for key, value in markers.items():
            metadata[f"collector.effect.{key}"] = value

    if include_effect_stage:
        stage_required = prelude.get("stage_required")
        stage_actual = prelude.get("stage_actual")
        _set("effect.stage.required", stage_required)
        _set("effect.stage.actual", stage_actual)
        _set("effect.stage.mode", prelude.get("stage_mode"))
        _set("effect.stage.capability", prelude.get("capability"))
        _set("effect.stage.kind", prelude.get("kind"))
        _set("effect.stage.source", prelude.get("source"))
        missing: List[str] = []
        if stage_required is None:
            missing.append("effect.stage.required")
        if stage_actual is None:
            missing.append("effect.stage.actual")
        if missing:
            ident = prelude.get("snapshot_id") or snapshot_id or "(unknown)"
            joined = ", ".join(missing)
            raise ValueError(
                f"Missing mandatory effect stage fields ({joined}) for snapshot {ident}"
            )

    if "error_kind" in prelude:
        _set("collector.error.kind", prelude.get("error_kind"))
    if "error_key" in prelude:
        _set("collector.error.key", prelude.get("error_key"))

    return metadata


def _merge_audit_block(
    existing: Optional[Dict[str, Any]], metadata: Dict[str, Any]
) -> Dict[str, Any]:
    if existing is None:
        return {"metadata": metadata}
    merged = dict(existing)
    nested = _as_dict(merged.get("metadata"))
    if nested:
        combined = dict(nested)
        combined.update(metadata)
        merged["metadata"] = combined
    else:
        merged["metadata"] = metadata
    return merged


def build_prelude_payload(name: str, payload: Dict[str, Any]) -> Dict[str, Any]:
    stage = _as_dict(payload.get("stage")) or {}
    required = _as_dict(stage.get("required")) or {}

    prelude = {
        "snapshot_id": name,
        "kind": payload.get("kind"),
        "capability": stage.get("capability"),
        "stage_actual": stage.get("actual"),
        "stage_required": required.get("stage"),
        "stage_mode": required.get("mode"),
        "stage_mismatch": payload.get("stage_mismatch", False),
        "source": stage.get("source"),
        "effects": _normalize_effects(payload.get("effects")),
        "markers": _normalize_markers(payload.get("markers")),
        "value": payload.get("value"),
    }
    metrics = payload.get("metrics")
    if metrics is not None:
        prelude["metrics"] = metrics
    return prelude


def build_diagnostic(
    sequence: int, name: str, payload: Dict[str, Any], include_effect_stage: bool
) -> Dict[str, Any]:
    extensions = _as_dict(payload.get("extensions"))
    prelude_dict = (
        dict(_as_dict(extensions.get("prelude.collector")) or {})
        if extensions
        else None
    )

    if prelude_dict is None:
        prelude_dict = build_prelude_payload(name, payload)
        diag: Dict[str, Any] = {
            "snapshot_id": name,
            "code": "core.prelude.collector_snapshot",
            "severity": "info",
            "message": f"Collector snapshot: {name}",
        }
    else:
        diag = dict(payload)
        diag["snapshot_id"] = name

    prelude_dict["snapshot_id"] = name
    prelude_dict["stage_mismatch"] = _as_bool(prelude_dict.get("stage_mismatch"))
    prelude_dict["effects"] = _normalize_effects(prelude_dict.get("effects"))
    prelude_dict["markers"] = _normalize_markers(prelude_dict.get("markers"))

    diag_extensions = _as_dict(diag.get("extensions")) or {}
    diag_extensions["prelude.collector"] = prelude_dict
    diag["extensions"] = diag_extensions
    diag["schema_version"] = diag.get("schema_version") or SNAPSHOT_SCHEMA_VERSION

    metadata = _build_metadata(prelude_dict, include_effect_stage, name)
    timestamp, change_set, audit_label = _apply_snapshot_audit_metadata(
        metadata, prelude_dict, name, sequence
    )
    diag.setdefault("timestamp", timestamp)
    diag["audit"] = _merge_audit_block(_as_dict(diag.get("audit")), metadata)
    diag["audit"].setdefault("change_set", change_set)
    diag["audit"].setdefault("cli.audit_id", audit_label)
    diag["audit"].setdefault("audit_id", audit_label)

    return diag


def build_audit_entries(
    diagnostics: Iterable[Dict[str, Any]], include_effect_stage: bool
) -> List[Dict[str, Any]]:
    entries: List[Dict[str, Any]] = []
    for diag in diagnostics:
        prelude = _as_dict(
            (_as_dict(diag.get("extensions")) or {}).get("prelude.collector")
        )
        if not prelude:
            continue
        metadata = _build_metadata(
            prelude,
            include_effect_stage,
            prelude.get("snapshot_id") or diag.get("snapshot_id"),
        )
        entries.append(
            {
                "case": prelude.get("snapshot_id") or diag.get("snapshot_id"),
                "metadata": metadata,
            }
        )
    return entries


def main() -> None:
    parser = argparse.ArgumentParser(
        description="core_iter_collectors.snap から診断 JSON / 監査ログを生成する"
    )
    parser.add_argument(
        "--snapshots",
        type=Path,
        required=True,
        help="core_iter_collectors.snap のパス",
    )
    parser.add_argument(
        "--output",
        type=Path,
        required=True,
        help="生成する診断 JSON の出力先",
    )
    parser.add_argument(
        "--audit-output",
        type=Path,
        help="監査ログ (JSON Lines) の出力先（オプション）",
    )
    parser.add_argument(
        "--with-stage",
        action="store_true",
        help="`effect.stage.*` メタデータを collector スナップショットへ付与して検証する",
    )
    args = parser.parse_args()

    entries = parse_snapshot_entries(args.snapshots)
    diagnostics = [
        build_diagnostic(index, name, payload, args.with_stage)
        for index, (name, payload) in enumerate(entries)
    ]

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8") as handle:
        json.dump({"diagnostics": diagnostics}, handle, indent=2, ensure_ascii=False)
        handle.write("\n")

    if args.audit_output:
        audit_entries = build_audit_entries(diagnostics, args.with_stage)
        args.audit_output.parent.mkdir(parents=True, exist_ok=True)
        with args.audit_output.open("w", encoding="utf-8") as handle:
            for entry in audit_entries:
                handle.write(json.dumps(entry, ensure_ascii=False))
                handle.write("\n")


if __name__ == "__main__":
    main()
