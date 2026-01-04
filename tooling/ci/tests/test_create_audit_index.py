#!/usr/bin/env python3
"""create-audit-index.py 用の簡易テスト."""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path
import sys


MODULE_PATH = Path(__file__).resolve().parents[1] / "create-audit-index.py"
SPEC = importlib.util.spec_from_file_location("create_audit_index", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"create-audit-index.py を読み込めません: {MODULE_PATH}")
CREATE_AUDIT_INDEX = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = CREATE_AUDIT_INDEX  # dataclass 評価時に必要
SPEC.loader.exec_module(CREATE_AUDIT_INDEX)


class CreateAuditIndexTests(unittest.TestCase):
    def test_parse_audit_spec_includes_optional_fields(self) -> None:
        spec = CREATE_AUDIT_INDEX.parse_audit_spec(
            "ci:windows-msvc:reports/audit/windows.audit.jsonl:success:full:0.95"
        )
        self.assertEqual(spec.profile, "ci")
        self.assertEqual(spec.target, "windows-msvc")
        self.assertEqual(spec.status, "success")
        self.assertEqual(spec.audit_level, "full")
        self.assertAlmostEqual(spec.pass_rate or 0.0, 0.95, places=2)
        self.assertTrue(spec.path.match("reports/audit/windows.audit.jsonl"))

    def test_build_entry_embeds_size_and_commit(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir) / "sample.audit.jsonl"
            tmp_path.write_text('{"category":"ffi.bridge"}\n', encoding="utf-8")
            spec = CREATE_AUDIT_INDEX.AuditSpec(
                profile="ci",
                target="macos-arm64",
                path=tmp_path,
                status="success",
                audit_level="full",
                pass_rate=1.0,
            )
            entry, warning = CREATE_AUDIT_INDEX.build_entry(
                spec, build_id="test-build", timestamp="2025-11-07T00:00:00Z", commit="deadbeef"
            )
            self.assertIsNone(warning)
            self.assertIsNotNone(entry)
            assert entry is not None
            self.assertEqual(entry["path"], str(tmp_path))
            self.assertEqual(entry["size_bytes"], str(tmp_path.stat().st_size))
            self.assertEqual(entry["commit"], "deadbeef")
            self.assertEqual(entry["pass_rate"], 1.0)

    def test_aggregate_retained_sorts_profiles(self) -> None:
        entries = [
            {"profile": "ci", "target": "macos", "size_bytes": "10"},
            {"profile": "ci", "target": "windows", "size_bytes": "5"},
            {"profile": "ci", "target": "macos", "size_bytes": "2"},
        ]
        summary = CREATE_AUDIT_INDEX.aggregate_retained(entries)
        self.assertEqual(
            summary,
            [
                {"profile": "ci", "target": "macos", "count": 2, "size_bytes": 12},
                {"profile": "ci", "target": "windows", "count": 1, "size_bytes": 5},
            ],
        )


if __name__ == "__main__":
    unittest.main()
