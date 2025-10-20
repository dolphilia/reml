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
from pathlib import Path
from typing import Dict, Iterable, List, Optional


# Required audit metadata keys.
REQUIRED_AUDIT_KEYS: List[str] = [
    "effect.stage.required",
    "effect.stage.actual",
    "effect.capability",
    "effect.stage.iterator.required",
    "effect.stage.iterator.actual",
    "effect.stage.iterator.kind",
    "effect.stage.iterator.capability",
    "effect.stage.iterator.source",
]

# Required fields under extensions.effects.
REQUIRED_EFFECT_STAGE_KEYS: List[str] = ["required", "actual"]
REQUIRED_EFFECT_ITERATOR_KEYS: List[str] = [
    "required",
    "actual",
    "kind",
    "capability",
    "source",
]

# FFI bridge metrics configuration.
BRIDGE_DIAG_PREFIX = "ffi.contract."
REQUIRED_BRIDGE_AUDIT_KEYS: List[str] = [
    "bridge.status",
    "bridge.target",
    "bridge.arch",
    "bridge.abi",
    "bridge.ownership",
    "bridge.extern_symbol",
    "bridge.return.ownership",
    "bridge.return.wrap",
    "bridge.return.release_handler",
    "bridge.return.rc_adjustment",
]
REQUIRED_BRIDGE_EXTENSION_KEYS: List[str] = [
    "bridge.target",
    "bridge.ownership",
    "bridge.abi",
    "bridge.return.ownership",
    "bridge.return.wrap",
    "bridge.return.release_handler",
    "bridge.return.rc_adjustment",
]

STATUS_OK_VALUES = {"ok", "success", "pass"}


def load_json(path: Path) -> Dict:
    with path.open("r", encoding="utf-8") as handle:
        try:
            return json.load(handle)
        except json.JSONDecodeError as exc:
            raise ValueError(f"Failed to parse JSON: {path}") from exc


def iter_diagnostics(data: Dict) -> Iterable[Dict]:
    diagnostics = data.get("diagnostics")
    if not isinstance(diagnostics, list):
        raise ValueError("diagnostics array is missing")
    for diag in diagnostics:
        if not isinstance(diag, dict):
            continue
        yield diag


def check_audit_fields(audit: Optional[Dict]) -> List[str]:
    if audit is None:
        return REQUIRED_AUDIT_KEYS.copy()
    missing: List[str] = []
    for key in REQUIRED_AUDIT_KEYS:
        if key not in audit or audit[key] in (None, "", []):
            missing.append(key)
    return missing


def _as_dict(value: Optional[object]) -> Optional[Dict]:
    if isinstance(value, dict):
        return value
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

    return missing


def _has_path(data: Optional[Dict], dotted_key: str) -> bool:
    if data is None:
        return False
    current: object = data
    for part in dotted_key.split("."):
        if not isinstance(current, dict) or part not in current:
            return False
        current = current[part]
    if current in (None, "", []):
        return False
    return True


def check_bridge_audit_fields(audit: Optional[Dict]) -> List[str]:
    missing: List[str] = []
    for key in REQUIRED_BRIDGE_AUDIT_KEYS:
        if not _has_path(audit, key):
            missing.append(key)
    return missing


def check_bridge_extension_fields(extensions: Optional[Dict]) -> List[str]:
    missing: List[str] = []
    for key in REQUIRED_BRIDGE_EXTENSION_KEYS:
        if not _has_path(extensions, key):
            missing.append(f"extensions.{key}")
    return missing


def extract_bridge_status(
    audit: Optional[Dict], extensions: Optional[Dict]
) -> Optional[str]:
    containers: List[Dict] = []
    if isinstance(audit, dict):
        containers.append(audit)
    if isinstance(extensions, dict):
        containers.append(extensions)
    for container in containers:
        bridge = container.get("bridge")
        if isinstance(bridge, dict):
            status = bridge.get("status")
            if status is not None:
                return str(status)
    return None


def collect_metrics(paths: List[Path]) -> Dict:
    total = 0
    passed = 0
    failures: List[Dict[str, object]] = []

    for path in paths:
        data = load_json(path)
        for index, diag in enumerate(iter_diagnostics(data)):
            code = diag.get("code")
            if code != "typeclass.iterator.stage_mismatch":
                continue
            total += 1
            audit_missing = check_audit_fields(
                _as_dict(diag.get("audit"))
            )
            extensions_missing = check_extension_fields(
                _as_dict(diag.get("extensions"))
            )

            missing = audit_missing + extensions_missing
            if not missing:
                passed += 1
            else:
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "code": code,
                        "missing": sorted(set(missing)),
                    }
                )

    pass_rate = None
    if total > 0:
        pass_rate = passed / total

    return {
        "metric": "iterator.stage.audit_pass_rate",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "required_audit_keys": REQUIRED_AUDIT_KEYS,
        "sources": [str(path) for path in paths],
        "failures": failures,
    }


def collect_bridge_metrics(paths: List[Path]) -> Dict:
    total = 0
    passed = 0
    failures: List[Dict[str, object]] = []

    for path in paths:
        data = load_json(path)
        for index, diag in enumerate(iter_diagnostics(data)):
            code = diag.get("code")
            if not isinstance(code, str) or not code.startswith(
                BRIDGE_DIAG_PREFIX
            ):
                continue
            total += 1
            audit_dict = _as_dict(diag.get("audit"))
            extensions_dict = _as_dict(diag.get("extensions"))

            audit_missing = check_bridge_audit_fields(audit_dict)
            extensions_missing = check_bridge_extension_fields(
                extensions_dict
            )

            issues: List[str] = []
            issues.extend(audit_missing)
            issues.extend(extensions_missing)

            status_value = extract_bridge_status(audit_dict, extensions_dict)
            status_issue: Optional[str] = None
            if status_value is None:
                status_issue = "bridge.status"
            else:
                normalized = status_value.lower()
                if normalized not in STATUS_OK_VALUES:
                    status_issue = f"bridge.status={status_value}"

            if status_issue:
                issues.append(status_issue)

            if not issues:
                passed += 1
            else:
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "code": code,
                        "missing": sorted(set(issues)),
                        "status": status_value,
                    }
                )

    pass_rate = None
    if total > 0:
        pass_rate = passed / total

    return {
        "metric": "ffi_bridge.audit_pass_rate",
        "total": total,
        "passed": passed,
        "failed": total - passed,
        "pass_rate": pass_rate,
        "required_audit_keys": REQUIRED_BRIDGE_AUDIT_KEYS,
        "sources": [str(path) for path in paths],
        "failures": failures,
    }


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
    return parser.parse_args(argv)


def main(argv: Optional[List[str]] = None) -> int:
    args = parse_args(argv)
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

    iterator_metrics = collect_metrics(sources)
    bridge_metrics = collect_bridge_metrics(sources)

    combined = {
        "metrics": [iterator_metrics, bridge_metrics],
        # 互換性のため従来の iterator メトリクスをトップレベルにも残す。
        "metric": iterator_metrics.get("metric"),
        "total": iterator_metrics.get("total"),
        "passed": iterator_metrics.get("passed"),
        "failed": iterator_metrics.get("failed"),
        "pass_rate": iterator_metrics.get("pass_rate"),
        "required_audit_keys": iterator_metrics.get("required_audit_keys"),
        "sources": iterator_metrics.get("sources"),
        "failures": iterator_metrics.get("failures"),
        "ffi_bridge": bridge_metrics,
    }

    json_output = json.dumps(combined, indent=2, ensure_ascii=False)

    print(json_output)

    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with args.output.open("w", encoding="utf-8") as handle:
            handle.write(json_output)
            handle.write("\n")

    # Even if pass_rate < 1 we keep exit code 0 so CI can decide how to react.
    return 0


if __name__ == "__main__":
    sys.exit(main())
