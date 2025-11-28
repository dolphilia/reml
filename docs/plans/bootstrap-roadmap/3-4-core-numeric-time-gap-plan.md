# 3.4 Core Numeric & Time フォローアップ計画

## 目的
- `docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md` で扱いきれていない未実装項目・リスクを整理し、Rust 実装を仕様どおりに完了させる。
- 数値トレイト・時間 API・メトリクス監査のギャップを明確な実装ステップに落とし込み、Phase3 W47 以降のタスク化を容易にする。

## 背景
- 現状の Rust 実装は `mean`/`variance`/`Timestamp`/`MetricPoint` など中核機能を提供しているが、仕様 §1 の `median`/`mode`/`range` や §3 の広範なロケール・タイムゾーン要件が未対応のまま残っている。
- `MetricPoint` は `core_time` feature に紐づいているため `core_numeric` 単独で利用できず、Plan §5 の「Numeric/Audit 連携」を阻害している。
- テスト/ベンチマーク計画（Plan §7）で挙げた property テストや `numeric_time` ゴールデンは未作成のままで、回帰検知の仕組みが不足している。

## スコープ
- 数値トレイトと統計 API の補完（`median`/`mode`/`range`、非プリミティブ型、Diagnostic 拡張）。
- 時刻フォーマット/タイムゾーンと IO/Capability の連携強化。
- メトリクス監査 API の feature 再編と Stage 検証の一般化。
- テスト・ベンチマーク・CI 成果物の整備。

---

## 1. 数値トレイト・統計 API 補完

| ID | 作業内容 | 実施ステップ | 成果物 |
| --- | --- | --- | --- |
| N-1 | `median`/`mode`/`range` の実装 | `compiler/rust/runtime/src/numeric/mod.rs` に API 追加、`Iter` 拡張を更新、`docs/spec/3-4-core-numeric-time.md` §1 の引用を README に反映 | `numeric::tests::median_mode_range`, `reports/spec-audit/ch3/numeric_basic-extended.md` |
| N-2 | `Decimal`/`BigInt`/`Ratio` 対応 | `Numeric` トレイトを `feature = "decimal"` 等で拡張、`ordered-float` 以外の比較実装を追加、`docs/plans/rust-migration/1-1-ast-and-ir-alignment.md` の Numeric 制約テーブルを更新 | `compiler/rust/runtime/src/numeric/decimal.rs`, `tests/data/numeric/decimal_cases.json` |
| N-3 | `StatisticsError` の Diagnostics 連携強化 | `StatisticsError::with_tags` と `StatisticsTags` を実装し、`data.stats.*` メタデータを一括付与。`scripts/validate-diagnostic-json.sh --suite numeric` に `tests/data/numeric/decimal_cases.json` を追加してタグ付き診断を検証 | `compiler/rust/runtime/src/numeric/error.rs`, `tests/data/numeric/decimal_cases.json` |

### チェックリスト
- [x] `IterNumericExt` に `median`/`mode`/`range` を追加し、`core-numeric` feature でビルド。
- [x] `Decimal` 型（`rust_decimal` 予定）を `Cargo.toml` に追加し、`Numeric` を実装。
- [x] `scripts/validate-diagnostic-json.sh --suite numeric` が新規 JSON を含めて成功。

---

## 2. 時刻フォーマット/タイムゾーンと IO 連携

| ID | 作業内容 | 実施ステップ | 成果物 |
| --- | --- | --- | --- |
| T-1 | ロケールテーブルの拡張 | `docs/plans/bootstrap-roadmap/assets/time-format-locale-map.csv` を仕様の全ロケールで更新し、`compiler/rust/runtime/src/time/format.rs` の `TIME_LOCALE_TABLE` を自動生成スクリプトに置き換える | `tooling/scripts/update_time_locale_table.py`, `reports/spec-audit/ch3/time_format-locales.md` |
| T-2 | ICU 互換/フォールバック実装 | `TimeFormat::Custom` に ICU 互換パターンを導入（`icu_datetime` or 既存 `time` パーサ` +` fallback`）、`effect {unicode}` 記録を `LocaleId` と同期 | `compiler/rust/runtime/src/time/format/icu.rs`, `tests/data/time/format/icu_cases.json` |
| T-3 | IANA タイムゾーン/Capability 連携 | `time` crate の `tzdb` 機能または `chrono-tz` を利用して IANA 名を解決し、`CapabilityRegistry::verify_capability_stage` を `core.time.timezone.lookup` で詳細化。`docs/notes/runtime-capability-stage-log.md` に Stage 要件を追記 | `compiler/rust/runtime/src/time/timezone.rs` 更新、`tests/data/time/timezone_iana.json` |
| T-4 | Core.IO との接続 | `compiler/rust/runtime/src/io/env.rs` にタイムゾーン/ロケールのフェッチ API を追加し、`TimeError::with_capability_context` で `Env` 情報 (`time.platform.*`) を提供 | `reports/spec-audit/ch3/time_env-bridge.md` |

### チェックリスト
- [ ] `collect-iterator-audit-metrics.py --section numeric_time --scenario timezone_lookup` が IANA ケースを含む新 JSON を処理。
- [ ] `now()`/`sleep()` の KPI (`time.syscall.latency_ns`) を `0-3-audit-and-metrics.md` に追記。

---

## 3. メトリクス監査 API の再編

| ID | 作業内容 | 実施ステップ | 成果物 |
| --- | --- | --- | --- |
| M-1 | feature ゲートの整理 | `compiler/rust/runtime/src/lib.rs` から `#[cfg(feature = "core_time")] pub mod diagnostics;` を外し、`metrics` feature を新設。`core_numeric` 単独で `MetricPoint` が利用できるようにする | `Cargo.toml` feature 行、`docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md` §5 更新 |
| M-2 | Stage 検証の共通化 | `CapabilityRegistry::verify_capability_stage` をラップする `MetricsStageGuard` を追加し、`emit_metric` 以外の API（例: `Iter::collect_numeric`）でも Stage mismatch 診断を出力 | `compiler/rust/runtime/src/diagnostics/metric_point.rs`、`tests/metrics_capability.rs` 更新 |
| M-3 | KPI/監査ログの一元管理 | `tooling/ci/collect-iterator-audit-metrics.py` に `numeric_time` の統合メトリクス出力 (`reports/metrics/numeric-time-latest.json`) を追加し、CI artefact を `docs/notes/core-numeric-time-ci-log.md` で追跡 | 新規 log ファイル、`README.md#core-numeric--time-進捗` へのリンク |

### チェックリスト
- [ ] `cargo test --manifest-path compiler/rust/runtime/Cargo.toml --features metrics` が通過。
- [ ] `reports/audit/metric_point/*.audit.jsonl` が `effects.contract.stage_mismatch` を含む場合に CI で赤くなる。

---

## 4. テスト・ベンチマーク・CI

| ID | 作業内容 | 実施ステップ | 成果物 |
| --- | --- | --- | --- |
| Q-1 | プロパティテスト拡充 | `compiler/rust/runtime/tests/time_props.rs` を追加し、`duration_between`/`convert_timezone` の不変条件を `proptest` で検証。`numeric_props` にも `median` ケースを追加 | `proptest` 依存を `Cargo.toml` に追加、`reports/spec-audit/ch3/time_props-log.md` |
| Q-2 | ベンチマーク整備 | `compiler/rust/runtime/benches/time_clock.rs` を作成し、`Instant`/`SystemTime` 呼び出しのジッターを測定。`reports/benchmarks/numeric-time/phase3-bench-YYYYMMDD.json` を生成 | `docs/plans/rust-migration/3-2-benchmark-baseline.md` へ数値を追記 |
| Q-3 | ゴールデン/CI 成果物 | `compiler/rust/runtime/tests/golden/numeric_time/*.json` を追加し、`scripts/validate-diagnostic-json.sh --suite numeric_time` を CI に統合。`phase3-rust.yml` で `core-numeric,core-time,metrics` を同時実行 | `.github/workflows/phase3-rust.yml` 更新、`reports/ci/numeric-time/latest.md` |

### チェックリスト
- [ ] `scripts/validate-diagnostic-json.sh --suite numeric_time` が `tests/expected/time_{now,sleep}.json` と新ゴールデンを検証。
- [ ] `cargo bench --features core-time --bench time_clock` の結果を `reports/benchmarks/numeric-time/` に保存。

---

## 5. スケジュール提案

| 週 | 主タスク | 依存 |
| --- | --- | --- |
| W47 | N-1, N-3, M-1 | 既存 `core_numeric` |
| W48 | T-1, T-2, T-3 | W47 の `metrics` 再編 |
| W49 | M-2, M-3, T-4 | Capability/Env 更新 |
| W50 | Q-1, Q-2, Q-3 | 数値/時間 API 完備 |

完了後は `docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md` と README の進捗表を更新し、`docs-migrations.log` にフォローアップ計画の適用を記録する。
