# Core Numeric & Time 効果タグ・Capability マトリクス

## 背景
- 仕様 [3-4 Core Numeric & Time](../../spec/3-4-core-numeric-time.md) は `effect {time}` / `{audit}` / `{unicode}` を明示しており、監査と Capability Registry 側のステージ契約（[3-8 Core Runtime & Capability](../../spec/3-8-core-runtime-capability.md) §1.2）を同時に満たす必要がある。
- 監査ログは [3-6 Core Diagnostics & Audit](../../spec/3-6-core-diagnostics-audit.md) の `AuditEnvelope`/`effects` 拡張を経由して集約されるため、API ごとの効果タグと Stage 情報をマッピングした基準票を用意しておく。

## 効果タグ／Capability 整合マトリクス
| APIカテゴリ | 代表 API | 効果タグ | 必須 Capability / Stage 要件 | 検証ポイント | 仕様根拠 |
| --- | --- | --- | --- | --- | --- |
| クロック取得 | `now`, `monotonic_now`, `sleep` | `effect {time}` | `CapabilityId = "time"`, `StageRequirement::AtLeast(Beta)`（`CapabilityDescriptor::resolve("core.time.")` の既定に従う） | `CapabilityRegistry::verify_capability_stage("time", ..)` の結果を `effects.contract.stage_*` と `AuditEnvelope.metadata["time.stage.*"]` に転写する。`TimeErrorKind::SystemClockUnavailable` の診断経路が 3-6 §1 のフォーマットに揃っているかを `collect-iterator-audit-metrics.py --section numeric_time --scenario effects_matrix` で確認する。 | [3-4](../../spec/3-4-core-numeric-time.md) §3, [3-8](../../spec/3-8-core-runtime-capability.md) §1.2 |
| タイムゾーン解決 | `local`, `timezone`, `convert_timezone` | `effect {time}` (`convert_timezone` は `@pure`) | `CapabilityId = "time"`、`StageRequirement::AtLeast(Beta)`。`Core.Env` 側のタイムゾーン情報取得は `CapabilityRegistry` を経由している必要がある。 | `TimeErrorKind::InvalidTimezone` → `Diagnostic` 変換時に `effect.stage.required/actual` と `AuditEnvelope.metadata["time.tz.provider"]` を必須キーにする。 `effects_matrix` シナリオで `timezone_cases.json` を入力し、`stage_mismatch_count = 0` を保証する。 | [3-4](../../spec/3-4-core-numeric-time.md) §3.2, [3-6](../../spec/3-6-core-diagnostics-audit.md) §1.1, [3-8](../../spec/3-8-core-runtime-capability.md) §1.2 |
| 時刻フォーマット/パース | `format`, `parse` (`TimeFormat::{Rfc3339,Unix,Custom}`) | `effect {unicode}` + `effect {time}`（`Timestamp` 生成箇所） | `CapabilityId = "unicode"`（`StageRequirement::AtLeast(Beta)`）、`CapabilityId = "time"`（Timestamp 化の Stage 下限）。`ICU` 依存分は `RuntimeBridgeDescriptor` 連携を想定。 | `EffectSet::mark_unicode()` と `collector.effect.unicode` が `TimeFormat::Custom` を通じて記録されるか確認する。`tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario effects_matrix --check unicode` で `effect.unicode=true` を要求する。 | [3-4](../../spec/3-4-core-numeric-time.md) §3.1, [3-6](../../spec/3-6-core-diagnostics-audit.md) §1 (`TimeError`→`Diagnostic`), [3-8](../../spec/3-8-core-runtime-capability.md) §1.2 |
| メトリクス送信 | `metric_point`, `attach_audit`, `emit_metric` | `effect {audit}` (`emit_metric` のみ) | `CapabilityId = "audit"`, `StageRequirement::Exact(Stable)`（3-6 §1.1 で `AuditEnvelope` が常時必須のため）。`AuditSink` は `RuntimeBridge`/`CapabilityRegistry` で stage を検証する。 | `emit_metric` 呼び出しごとに `effects.contract.stage_mismatch = 0`、`AuditEnvelope.metadata["metrics.emit.*"]` が揃うことを `effects_matrix` シナリオでチェック。`MetricPoint` のタイムスタンプは `effect {time}` → `effect.stage.actual=time@beta` を監査に残す。 | [3-4](../../spec/3-4-core-numeric-time.md) §4, [3-6](../../spec/3-6-core-diagnostics-audit.md) §1, [3-8](../../spec/3-8-core-runtime-capability.md) §1.2 |

## 監査・CI 設計
- `python3 tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario effects_matrix --matrix docs/plans/bootstrap-roadmap/assets/core-numeric-time-effects-matrix.md --output reports/spec-audit/ch3/core_numeric_time_effects.json --require-success` を追加し、各行の `capability`, `stage_required`, `stage_actual`, `effect.*` を突き合わせる。
- 収集結果は `numeric_time.effect_matrix_pass_rate`（`docs/guides/tooling/audit-metrics.md` へ追記）として扱い、1.0 未満の場合は CI を失敗させる。
- `reports/spec-audit/ch3/core_numeric_time_effects.jsonl` を監査ログとして保存し、`AuditEnvelope.metadata["numeric_time.api"]` を `代表 API` 列と同期させる。

## 既知のギャップ
- ✅ `EffectSet`/`CollectorEffectMarkers` へ `effect {time}` フラグと `time_calls` カウンタを追加済み（`compiler/runtime/src/prelude/iter/mod.rs`, `compiler/runtime/src/prelude/collectors/mod.rs`）。`collector.effect.time` / `collector.effect.time_calls` が診断・監査ログに出力され、`collect-iterator-audit-metrics.py` の `collector.effect.audit_snapshot` 経由で参照できる。
- `AuditSink` から `CapabilityRegistry` への Stage 情報転送は現状コレクション系のみで使用しているため、`emit_metric` 経路に `required_effects = {"audit"}` を連携するフックを別途実装する必要がある。
