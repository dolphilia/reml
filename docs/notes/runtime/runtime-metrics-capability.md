# Runtime Metrics Capability メモ

`Core.Diagnostics` の `MetricPoint`/`emit_metric` と `Core.Runtime` Capability Registry の連携仕様をまとめる。Phase3 Bootstrap `3-4 Core Numeric & Time` 計画 §5.3 および `docs/spec/3-8-core-runtime-capability.md` の要求に対応する作業ログとしても使用する。

## 背景
- Phase3 §5.1〜5.2 で `MetricPoint` → `AuditSink` 連携と監査メタデータは整備済みだが、`CapabilityStage` の検証フックと `effects.contract.stage_mismatch` 診断が未実装だった。
- CLI (`remlc metrics emit`) と Runtime Bridge の双方で `metrics.emit` Capability を参照する想定のため、Stage 逸脱時に共通の診断/監査キーを残す必要がある。
- 監査 KPI (`numeric.metrics.emit_success_rate`) を `docs/guides/tooling/audit-metrics.md` でトラッキングする際、Stage mismatch の再現ログ（`reports/dual-write/metrics-stage-mismatch.json`）が必要になった。

## 実装概要
- `compiler/runtime/src/diagnostics/metric_point.rs`
  - `emit_metric` の冒頭で `CapabilityRegistry::verify_capability_stage("metrics.emit", StageRequirement::Exact(StageId::Stable), ["audit"])` を実行。
  - 失敗した場合は `stage_mismatch_diagnostic` を構築し、`code = "effects.contract.stage_mismatch"` / `domain = "runtime"` / `extensions["effects.contract.*"]` に Capability/Stage/Required effects を記録する。
  - 成功した場合でも監査メタデータの `effect.stage.actual` に Registry の戻り値 (`StageId`) を書き込み、`effect.required_effects = ["audit"]` を固定。
- `compiler/runtime/src/diagnostics/audit_bridge.rs`
  - `attach_audit`/`metric_audit_metadata` が StageRequirement/StageId/Required effects を受け取るよう更新。
  - `stage_requirement_label` で `exact`/`at_least` を文字列表現（例: `"stable"`, `"at_least beta"`）へ正規化。
- `compiler/runtime/src/registry.rs`
  - `CapabilityError` に `actual_stage: Option<StageId>` を保持させ、`with_actual_stage`/`actual_stage()` で診断へ舞い戻れるようにした。
  - 既存の Collector 等は `CapabilityError` の文字列表現を利用しており、追加フィールドによる挙動変更はなし。
- `compiler/runtime/tests/metrics_capability.rs`
  - `metrics.emit` が Stable 要件で成功すること、および Beta 要件を指定した時に `actual_stage = Stable` として `capability.stage.mismatch` が返ることを確認。

## 診断/監査サンプル
- `reports/dual-write/metrics-stage-mismatch.json`
  - 再現コマンド（`remlc metrics emit --required-stage beta ...`）と `effects.contract.stage_mismatch` 診断を 1 つにまとめたゴールデン。
  - `metric_point.*` / `effect.*` / `effects.contract.*` キーが揃っていることを確認するスモークとして `scripts/validate-diagnostic-json.sh --pattern metrics.emit` を流用。
- `tooling/lsp/tests/client_compat/fixtures/metrics-stage.json`
  - LSP 互換テストで Stage mismatch 診断がシリアライズ可能か検証するための Fixture。`client_compat.test.ts` にテストケースを追加。

## 検証手順
- Rust Runtime 単体
  - `cargo test --manifest-path compiler/runtime/Cargo.toml --features core_numeric,core_time metrics_capability stage_mismatch_produces_guard_diagnostic`
  - 期待結果: すべて PASS。`metrics_capability_reports_actual_stage_on_violation` で `StageId::Stable` が返ってくることを確認。
- CLI/LSP
  - `node tooling/lsp/tests/client_compat/scripts/validate-diagnostic-json.mjs metrics-stage.json`
  - `pnpm --dir tooling/lsp/tests/client_compat test -- metrics-stage`（既存 test suite に新ケースが含まれる）
- 監査/KPI
  - `python3 tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario emit_metric --metric-source tests/data/metrics/metric_point_cases.json`
  - Stage mismatch の検証は `reports/dual-write/metrics-stage-mismatch.json` を `docs/guides/tooling/audit-metrics.md` にリンクする形で進捗管理。

## 参考リンク
- `docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md` §5.3
- `docs/spec/3-6-core-diagnostics-audit.md`, `docs/spec/3-8-core-runtime-capability.md`
- `docs/notes/runtime/runtime-capability-stage-log.md`（metrics 行を追加済み）
