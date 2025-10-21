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
from typing import Any, Dict, Iterable, List, Optional, Set, Tuple

try:
    import tomllib  # Python 3.11+
except ModuleNotFoundError:  # pragma: no cover
    tomllib = None


# Required audit metadata keys.
REQUIRED_AUDIT_KEYS: List[str] = [
    "audit_id",
    "change_set",
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
    "audit_id",
    "change_set",
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
    "bridge.return.ownership",
    "bridge.return.status",
    "bridge.return.wrap",
    "bridge.return.release_handler",
    "bridge.return.rc_adjustment",
]

DEFAULT_RETENTION_POLICY: Dict[str, int] = {
    "ci": 100,
    "local": 30,
    "tmp": 20,
    "default": 50,
}

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


def check_audit_fields(audit: Optional[Dict]) -> List[str]:
    if audit is None or not isinstance(audit, dict):
        return ["audit"] + REQUIRED_AUDIT_KEYS.copy()
    metadata = audit.get("metadata") if isinstance(audit.get("metadata"), dict) else None
    missing: List[str] = []
    for key in REQUIRED_AUDIT_KEYS:
        has_key = False
        for container in filter(None, (audit, metadata)):
            value = container.get(key)
            if value not in (None, "", []):
                has_key = True
                break
        if not has_key:
            missing.append(key)
    return missing


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


def _has_any_path(data: Optional[Dict], *paths: str) -> bool:
    for path in paths:
        if _has_path(data, path):
            return True
    return False


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
                failures.append(
                    {
                        "file": str(path),
                        "index": index,
                        "code": code or "unknown",
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

    pass_rate = None
    if total > 0:
        pass_rate = passed / total

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
        "required_audit_keys": REQUIRED_BRIDGE_AUDIT_KEYS,
        "sources": [str(path) for path in paths],
        "audit_sources": audit_sources,
        "failures": failures,
        "platform_summary": platform_summary,
        "schema_versions": sorted(schema_versions),
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
    parser.add_argument(
        "--audit-source",
        action="append",
        dest="audit_sources",
        help="Path to audit JSON (repeatable).",
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

    iterator_metrics = collect_metrics(sources)
    bridge_metrics = collect_bridge_metrics(sources, audit_paths)

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
        "audit_sources": bridge_metrics.get("audit_sources"),
        "schema_versions": sorted(
            {
                *(iterator_metrics.get("schema_versions") or []),
                *(bridge_metrics.get("schema_versions") or []),
            }
        ),
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
