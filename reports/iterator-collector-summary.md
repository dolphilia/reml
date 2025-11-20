# Iterator Collector Summary (2025-11-19)

- **Diagnostics source**: `compiler/rust/frontend/tests/core_iter_collectors.rs` + `collect-iterator-audit-metrics.py --module iter --section collectors`.
- **Monitored KPI**: `collector.effect.mem`, `collector.effect.mut`, `collector.effect.mem_reservation`, `collector.effect.reserve`, `collector.error.duplicate_key_rate`, `collector.error.invalid_encoding`, `iterator.stage.audit_pass_rate`.
- **Audit log**: `reports/spec-audit/ch0/collector-YYYYMMDD.json`（F2 snapshot set）までの `prelude.collector` 拡張の値を `reports/spec-audit/ch0/links.md#collector-f2` から参照。

### collect_list_baseline
- Stage: `stable` (`CollectorKind::List` / `IteratorStageProfile::stable`), `collector.stage.audit_pass_rate` target は `1.0`.
- Effects: `collector.effect.mem = false`, `collector.effect.mut = false`, `collector.effect.finish = 1`.
- Value: `[1, 2, 3]` (`List` の固定順序)。

### collect_vec_mem_reservation
- Stage: `beta`, capability `core.collector.vec`.
- Effects: `collector.effect.mem = true`, `collector.effect.mut = true`, `collector.effect.mem_reservation = 4`, `collector.effect.reserve = 2`.
- Purpose: `effect {mem}` を出す `VecCollector` の `reserve` 呼び出しを `reports/iterator-collector-summary.md#collect_vec_mem_reservation` で掬い、`collect-iterator-audit-metrics.py` の `collector.effect.mem_leak` KPI へ接続。

### collect_map_duplicate
- Error: `CollectError::DuplicateKey`, `collector.error.key = "\"dup\""`, `Diagnostic.extensions["prelude.collector.error_key"]` 経由でキー情報を保持。
- Stage: `beta`, capability `core.collector.map`, `collector.error.duplicate_key_rate = 1`（検証目的）。
- Metrics: `collector.effect.mem = false`, `collector.effect.mut = false`, `collector.error.duplicate_key_rate` と `collector.stage.mismatch_rate` が 0 であることを確認。

### collect_set_stage
- Stage: `stable` (`SetCollector` が `StageRequirement::Exact("stable")` を満たす)。
- Effects: `collector.effect.mem = false`, `collector.effect.mut = false`, `collector.effect.finish = 1`.
- Value: `[1, 3, 5]`（`BTreeSet` による昇順）。

### collect_string_invalid
- Error: `StringError::InvalidEncoding` → `CollectErrorKind::InvalidEncoding`、`message` は `invalid UTF-8 byte 0x28 at offset 1 (expected continuation byte in 0x80..=0xBF)`。
- Diagnostics detail: `extensions.detail = "byte=0x28; offset=1"`.
- Effects: `collector.effect.mem = true`, `collector.effect.mut = true`, `collector.effect.mem_reservation = 0`.
- KPI: `collector.error.invalid_encoding` は意図的な失敗として記録しつつ、正常系では `0` であることを `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` で追跡。
