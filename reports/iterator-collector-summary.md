# Iterator Collector Summary (2025-11-20)

- **Diagnostics source**: `reports/spec-audit/ch1/core_iter_collectors.json`（`python3 tooling/ci/render-collector-audit-fixtures.py --snapshots compiler/rust/frontend/tests/__snapshots__/core_iter_collectors.snap --output reports/spec-audit/ch1/core_iter_collectors.json --audit-output reports/spec-audit/ch1/core_iter_collectors.audit.jsonl`）。
- **Metrics command**: `python3 tooling/ci/collect-iterator-audit-metrics.py --section collectors --module iter --case wbs-31b-f2 --source reports/spec-audit/ch1/core_iter_collectors.json --audit-source reports/spec-audit/ch1/core_iter_collectors.audit.jsonl --output reports/iterator-collector-metrics.json`。
- **Monitored KPI**: `collector.stage.audit_pass_rate`, `collector.effect.mem`, `collector.effect.mut`, `collector.effect.mem_reservation`, `collector.effect.mem_bytes`, `collector.effect.reserve`, `collector.error.duplicate_key`, `collector.error.invalid_encoding`。
- **Audit log**: `reports/spec-audit/ch1/core_iter_collectors.audit.jsonl`（JSON Lines）と `reports/spec-audit/ch0/links.md#collector-f2` の手順ログを参照。
- **Metrics artifact**: `reports/iterator-collector-metrics.json` に `collector.effect.audit_snapshot` の集計結果を保存。

## 集計結果（wbs-31b-f2）
- Stage: `collector.stage.audit_pass_rate = 1.0`（`stage_actual {stable:2, beta:5}`, mismatch 0）。
- Effects: `collector.effect.mem = 2/7`, `collector.effect.mut = 4/7`, `collector.effect.mem_reservation = 4`, `collector.effect.reserve = 2`, `collector.effect.finish = 4`.
- Errors: `collector.error.duplicate_key = 2`、`collector.error.invalid_encoding = 1`、`collector.error.rate_per_total = 0.4286`。
- Schema: `collector.snapshot.v1`。すべてのケースで `AuditEnvelope.metadata.collector.*` を記録。

### collect_list_baseline
- Stage: `stable` (`CollectorKind::List` / `IteratorStageProfile::stable`), `collector.stage.audit_pass_rate` target は `1.0`.
- Effects: `collector.effect.mem = true`, `collector.effect.mut = false`, `collector.effect.mem_bytes = 12`, `collector.effect.mem_reservation = 12`, `collector.effect.finish = 1`.
- Value: `[1, 2, 3]` (`List` の固定順序)。
- KPI: `list_as_vec_mem_bytes` を `collect-iterator-audit-metrics.py --section collectors --module iter --case wbs-31b-f2` の集計に追加し、`scripts/validate-diagnostic-json.sh --pattern collector.effect.mem_bytes reports/spec-audit/ch1/core_iter_collectors.json` で検証する。

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

## Iter Terminator 経由の Collector 監査（WBS 3.1c-H1, 2027-03-06）
- **目的**: `Iter::collect_*` が `Collector` 実装と同一の監査メタデータ（`collector.effect.*`, `collector.stage.*`, `Diagnostic.extensions["prelude.collector.*"]`）を出力することを確認し、`collect_with` ヘルパと `CollectOutcome::audit` により `reports/spec-audit/ch1/core_iter_terminators.json` → `reports/iterator-collector-metrics.json` のパイプを定着させる。
- **Run ID**: `2027-03-06-iter-terminators-h1`（`cargo test --manifest-path compiler/rust/frontend/Cargo.toml core_iter_terminators` → `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case terminators --source reports/spec-audit/ch1/core_iter_terminators.json --output reports/iterator-collector-metrics.json --require-success` → `scripts/validate-diagnostic-json.sh --pattern iterator.collect --pattern prelude.collector reports/spec-audit/ch1/core_iter_terminators.json`）。`reports/spec-audit/ch0/links.md#iter-terminators-h1` にコマンドと成果物を集約。
- **共通結果**: `iterator.stage.audit_pass_rate = 1.0`、`collector.effect.mem_reservation` / `collector.error.*` / `collector.stage.*` が Collector 直接呼びと一致、`AuditEnvelope.metadata["prelude.collector.kind"]` が List/Vec/String で漏れなく出力された。

### collect_list_pipeline-h1
- `Iter::from_list([1,2,3]).map(|x| x * 2).collect_list()` の snapshot。`collector.stage.actual = "stable"`、`collector.effect.mem = true` となり `collector.effect.mem_bytes` にコピーコストが記録されつつ、`prelude.collector.kind = "list"` が `Diagnostic.extensions` と `AuditEnvelope.metadata` の両方で一致。

### collect_vec_reserve-h1
- `Iter::range(0,4).collect_vec()` に `reserve` 呼び出しを挟み、`collector.effect.mem_reservation = 4` / `collector.effect.mem = true` / `collector.effect.mut = true` を `reports/spec-audit/ch1/core_iter_terminators.json#collect_vec_reserve` に記録。`Diagnostic.extensions["prelude.collector.mem_reservation_bytes"] = 4` が `collect_vec_mem_reservation` ケースと同一値であることを確認。

### collect_string_invalid-h1
- `Iter::from_list([0x61u8, 0x28, 0x80])` を `collect_string` で終端させ、`collector.error.invalid_encoding = 1` が `Collector` 直接呼びケースと一致することを確認。`Diagnostic.extensions["prelude.collector.error_key"] = "offset=1"`、`effect {text}` タグが `reports/diagnostic-format-regression.md#iterator.collect_string_invalid` と同期。
