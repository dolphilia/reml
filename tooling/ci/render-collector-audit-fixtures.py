#!/usr/bin/env python3
"""
core_iter_collectors のスナップショットを診断 JSON / 監査ログへ変換する補助スクリプト。

`compiler/rust/frontend/tests/__snapshots__/core_iter_collectors.snap` をパースし、
`tooling/ci/collect-iterator-audit-metrics.py` で扱いやすい JSON 形式を生成する。
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Tuple


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


def _normalize_effects(raw: Any) -> Dict[str, bool]:
    result = {"mem": False, "mut": False, "debug": False, "async_pending": False}
    data = _as_dict(raw)
    if not data:
        return result
    for key in ("mem", "mut", "mutating", "debug", "async_pending"):
        if key not in data:
            continue
        value = data.get(key)
        normalized = _as_bool(value)
        if key == "mutating":
            result["mut"] = normalized
        elif key in result:
            result[key] = normalized
    return result


def _normalize_markers(raw: Any) -> Dict[str, int]:
    result = {"finish": 0, "mem_reservation": 0, "reserve": 0}
    data = _as_dict(raw)
    if not data:
        return result
    for key in result:
        result[key] = _as_int(data.get(key))
    return result


def _build_metadata(prelude: Dict[str, Any]) -> Dict[str, Any]:
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
    return prelude


def build_diagnostic(name: str, payload: Dict[str, Any]) -> Dict[str, Any]:
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
    diag["schema_version"] = diag.get("schema_version") or "collector.snapshot.v1"

    metadata = _build_metadata(prelude_dict)
    diag["audit"] = _merge_audit_block(_as_dict(diag.get("audit")), metadata)

    return diag


def build_audit_entries(diagnostics: Iterable[Dict[str, Any]]) -> List[Dict[str, Any]]:
    entries: List[Dict[str, Any]] = []
    for diag in diagnostics:
        prelude = _as_dict(
            (_as_dict(diag.get("extensions")) or {}).get("prelude.collector")
        )
        if not prelude:
            continue
        metadata = _build_metadata(prelude)
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
    args = parser.parse_args()

    entries = parse_snapshot_entries(args.snapshots)
    diagnostics = [build_diagnostic(name, payload) for name, payload in entries]

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8") as handle:
        json.dump({"diagnostics": diagnostics}, handle, indent=2, ensure_ascii=False)
        handle.write("\n")

    if args.audit_output:
        audit_entries = build_audit_entries(diagnostics)
        args.audit_output.parent.mkdir(parents=True, exist_ok=True)
        with args.audit_output.open("w", encoding="utf-8") as handle:
            for entry in audit_entries:
                handle.write(json.dumps(entry, ensure_ascii=False))
                handle.write("\n")


if __name__ == "__main__":
    main()
