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
| N-1 | `median`/`mode`/`range` の実装 | `compiler/runtime/src/numeric/mod.rs` に API 追加、`Iter` 拡張を更新、`docs/spec/3-4-core-numeric-time.md` §1 の引用を README に反映 | `numeric::tests::median_mode_range`, `reports/spec-audit/ch3/numeric_basic-extended.md` |
| N-2 | `Decimal`/`BigInt`/`Ratio` 対応 | `Numeric` トレイトを `feature = "decimal"` 等で拡張、`ordered-float` 以外の比較実装を追加、`docs/plans/rust-migration/1-1-ast-and-ir-alignment.md` の Numeric 制約テーブルを更新 | `compiler/runtime/src/numeric/decimal.rs`, `tests/data/numeric/decimal_cases.json` |
| N-3 | `StatisticsError` の Diagnostics 連携強化 | `StatisticsError::with_tags` と `StatisticsTags` を実装し、`data.stats.*` メタデータを一括付与。`scripts/validate-diagnostic-json.sh --suite numeric` に `tests/data/numeric/decimal_cases.json` を追加してタグ付き診断を検証 | `compiler/runtime/src/numeric/error.rs`, `tests/data/numeric/decimal_cases.json` |
| N-4 | `Precision`/`NumericError`/丸め API | `compiler/runtime/src/numeric/precision.rs` を新設し、`Precision` 列挙と `with_precision`/`round_to`/`truncate_to` を実装。`NumericError` の `IntoDiagnostic`/`AuditMetadata` を定義し、`scripts/validate-diagnostic-json.sh --suite numeric` に `tests/data/numeric/precision/*.json` を追加する | `compiler/runtime/src/numeric/precision.rs`, `tests/data/numeric/precision/*.json`, `reports/spec-audit/ch3/numeric_precision-*.json` |
| N-5 | 多倍長型の Iter 連携 | `IterNumericExt` が `Decimal`/`BigRational` で `mean`/`variance` を計算できるよう、`Floating` 依存を見直して代替演算経路を実装。`numeric/effects.rs` で Decimal/Ratio の `effect {mem}` を記録し、`iter_numeric_props.rs` に Decimal ケースを追加 | `compiler/runtime/src/numeric/{mod.rs,effects.rs}`, `compiler/runtime/tests/iter_numeric_props.rs`, `docs/plans/bootstrap-roadmap/assets/core-numeric-time-effects-matrix.md` 更新 |
| N-6 | 金融 API (`currency_add` 等) | `compiler/runtime/src/numeric/finance.rs` を追加し、`currency_add`/`compound_interest`/`net_present_value` を `Decimal` ベースで実装。`CurrencyCode` 検証と `NumericErrorKind::UnsupportedCurrency` 診断を用意し、`reports/spec-audit/ch3/numeric_finance-*.json` を作成 | `compiler/runtime/src/numeric/finance.rs`, `tests/data/numeric/finance/*.json`, `reports/spec-audit/ch3/numeric_finance-*.json`, `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` KPI 行 |

### チェックリスト
- [x] `IterNumericExt` に `median`/`mode`/`range` を追加し、`core-numeric` feature でビルド。
- [x] `Decimal` 型（`rust_decimal` 予定）を `Cargo.toml` に追加し、`Numeric` を実装。
- [x] `scripts/validate-diagnostic-json.sh --suite numeric` が新規 JSON を含めて成功。
- [x] `Precision`/`NumericError`/丸め API を `numeric/precision.rs` に実装し、`tests/data/numeric/precision/*.json` をゴールデンに追加。
- [x] `IterNumericExt` の Decimal/Ratio 経路を実装し、`iter_numeric_props.rs` に Decimal ケースを追加。
- [x] `numeric/finance.rs` と `CurrencyCode` 検証を実装し、`numeric_finance-*.json` を `reports/spec-audit/ch3/` に収集。

---

## 2. 時刻フォーマット/タイムゾーンと IO 連携

| ID | 作業内容 | 実施ステップ | 成果物 |
| --- | --- | --- | --- |
| T-1 | ロケールテーブルの拡張 | `docs/plans/bootstrap-roadmap/assets/time-format-locale-map.csv` を仕様の全ロケールで更新し、`compiler/runtime/src/time/format.rs` の `TIME_LOCALE_TABLE` を自動生成スクリプトに置き換える | `tooling/scripts/update_time_locale_table.py`, `compiler/runtime/src/time/locale_table_data.rs`, `reports/spec-audit/ch3/time_format-locales.md` |
| T-2 | ICU 互換/フォールバック実装 | `TimeFormat::Custom` に ICU 互換パターンを導入（`time` 記法へのトランスレータ + fallback）し、`effect {unicode}` 記録を `LocaleId` と同期 | `compiler/runtime/src/time/format/icu.rs`, `tests/data/time/format/icu_cases.json`, `reports/spec-audit/ch3/time_format-locales.md` |
| T-3 | IANA タイムゾーン/Capability 連携 | 代表的な IANA 名を認識して `timezone(name)` に反映、`CapabilityRegistry::verify_capability_stage` のログと `collect-iterator-audit-metrics.py --scenario timezone_lookup` のカバレッジを拡張 | `compiler/runtime/src/time/timezone.rs`, `tests/data/time/timezone_iana.json`, `reports/spec-audit/ch3/time_timezone-iana.md` |
| T-4 | Core.IO との接続 | `compiler/runtime/src/io/env.rs` にタイムゾーン/ロケールのフェッチ API を追加し、`TimeError::with_capability_context` に `time.env.*` メタデータを渡す | `compiler/runtime/src/io/env.rs`, `reports/spec-audit/ch3/time_env-bridge.md` |

### チェックリスト
- [x] `TIME_LOCALE_TABLE` を CSV 由来の自動生成ファイルへ分離し、`python3 tooling/scripts/update_time_locale_table.py` で再現できるようにした。
- [x] `TimeFormat::Custom` が ICU パターン（yyyy/MM/dd など）をトランスレートし、`tests/data/time/format/icu_cases.json` でフォーマット/パースの双方を検証。
- [x] `collect-iterator-audit-metrics.py --section numeric_time --scenario timezone_lookup` に IANA 用 JSON（`tests/data/time/timezone_iana.json`）を追加で指定し、CI でケース数・プラットフォームリストを記録。
- [x] Core.IO の `time_env_snapshot()` を追加し、`TimeError` で `time.env.{timezone,locale}` を監査メタデータとして出力。
- [ ] `now()`/`sleep()` の KPI (`time.syscall.latency_ns`) を `0-3-audit-and-metrics.md` に追記。

---

## 3. メトリクス監査 API の再編

| ID | 作業内容 | 実施ステップ | 成果物 |
| --- | --- | --- | --- |
| M-1 | feature ゲートの整理 | `compiler/runtime/src/lib.rs` から `#[cfg(feature = "core_time")] pub mod diagnostics;` を外し、`metrics` feature を新設。`core_numeric` 単独で `MetricPoint` が利用できるようにする | `Cargo.toml` feature 行、`docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md` §5 更新 |
| M-2 | Stage 検証の共通化 | `CapabilityRegistry::verify_capability_stage` をラップする `MetricsStageGuard` を追加し、`emit_metric` 以外の API（例: `Iter::collect_numeric`）でも Stage mismatch 診断を出力 | `compiler/runtime/src/diagnostics/metric_point.rs`、`compiler/runtime/src/prelude/iter/mod.rs`、`tests/metrics_capability.rs` 更新 |
| M-3 | KPI/監査ログの一元管理 | `tooling/ci/collect-iterator-audit-metrics.py` に `numeric_time` の統合メトリクス出力 (`reports/metrics/numeric-time-latest.json`) を追加し、CI artefact を `docs/notes/core-numeric-time-ci-log.md` で追跡 | 新規 log ファイル、`README.md#core-numeric--time-進捗` へのリンク |

### チェックリスト
- [x] `cargo test --manifest-path compiler/runtime/Cargo.toml --features core_numeric`（`metrics` 有効化） が通過。
- [ ] `reports/audit/metric_point/*.audit.jsonl` が `effects.contract.stage_mismatch` を含む場合に CI で赤くなる。

> 進行ログ（Phase3 W48, M-1〜M-2）  
> - `compiler/runtime/Cargo.toml` に `metrics` feature を追加し、`core_numeric`/`core_time` から継承する形へ移行。`compiler/runtime/src/lib.rs` は `diagnostics` を `metrics`、`time` を `core_time` または `metrics` で公開する構成とし、`MetricPoint` が `core_time` なしでも利用可能になった。  
> - `compiler/runtime/src/diagnostics/stage_guard.rs` を新設し、`metrics.emit` Stage 検証を `MetricsStageGuard` へ統合。`emit_metric`・`metric_audit_metadata` はこのガードを介してメタデータを生成するよう更新した。  
> - `Iter::collect_numeric` へ `ensure_numeric_metrics_stage` を挿入し、Stage mismatch が `CollectError::capability_denied` で露出するようにした（`prelude/iter/mod.rs`、テスト `prelude::iter::tests::ensure_numeric_metrics_stage_reports_capability_error` を追加）。  
> - `cargo test --manifest-path compiler/runtime/Cargo.toml --features core_numeric` を実施し、`metrics` フィーチャー経路のリグレッションを確認。`core_time` 専用テスト (`time::tests::custom_format_rejects_unsupported_locale`) は既存のロケール仕様と齟齬があるため別途 TODO として継続。

---

## 4. テスト・ベンチマーク・CI

| ID | 作業内容 | 実施ステップ | 成果物 |
| --- | --- | --- | --- |
| Q-1 | プロパティテスト拡充 | `compiler/runtime/tests/time_props.rs` を追加し、`duration_between`/`convert_timezone` の不変条件を `proptest` で検証。`numeric_props` にも `median` ケースを追加 | `proptest` 依存を `Cargo.toml` に追加、`reports/spec-audit/ch3/time_props-log.md` |
| Q-2 | ベンチマーク整備 | `compiler/runtime/benches/time_clock.rs` を作成し、`Instant`/`SystemTime` 呼び出しのジッターを測定。`reports/benchmarks/numeric-time/phase3-bench-YYYYMMDD.json` を生成 | `docs/plans/rust-migration/3-2-benchmark-baseline.md` へ数値を追記 |
| Q-3 | ゴールデン/CI 成果物 | `compiler/runtime/tests/golden/numeric_time/*.json` を追加し、`scripts/validate-diagnostic-json.sh --suite numeric_time` を CI に統合。`phase3-rust.yml` で `core-numeric,core-time,metrics` を同時実行 | `.github/workflows/phase3-rust.yml` 更新、`reports/ci/numeric-time/latest.md` |

### チェックリスト
- [x] `scripts/validate-diagnostic-json.sh --suite numeric_time` が `tests/expected/time_{now,sleep}.json` と新ゴールデンを検証。
- [x] `cargo bench --features core-time --bench time_clock` の結果を `reports/benchmarks/numeric-time/` に保存。

> 実施ログ（Phase3 W50, §4）
> - `compiler/runtime/tests/time_props.rs` に `proptest` ベースの `duration_between`/`convert_timezone` 不変条件テストを追加し、`core-time` feature でのみビルドされるようにした。`timezone_offset_minutes_strategy` 経由で IANA/UTC 文字列を生成し、`duration_between` の対称性と `convert_timezone` の逆変換を検証【F:../../compiler/runtime/tests/time_props.rs†L1-L94】。
> - `compiler/runtime/tests/iter_numeric_props.rs` のヘルパに `manual_lower_median` を追加し、`median` が lower median 仕様を満たすかシード/長さを変えて検証する `median_matches_manual_lower_median` を実装した【F:../../compiler/runtime/tests/iter_numeric_props.rs†L1-L120】。
> - `compiler/runtime/benches/time_clock.rs` を追加し、`time_now_latency`/`time_monotonic_now_latency`/`duration_between_*` を Criterion で測定。結果を `reports/benchmarks/numeric-time/phase3-bench-20250107.json` に保存して Phase3 KPI の基準値とした【F:../../compiler/runtime/benches/time_clock.rs†L1-L64】【F:../../reports/benchmarks/numeric-time/phase3-bench-20250107.json†L1-L24】。
> - `compiler/runtime/tests/golden/numeric_time/clock_accuracy.{json,audit.jsonl}` を追加し、`scripts/validate-diagnostic-json.sh --suite numeric_time` で `tests/expected/time_{now,sleep}.json` と併せて JSON パース検証できるようスイートを拡張した。CI 依存のない Generic JSON チェックとして Python バリデータを挿入している【F:../../compiler/runtime/tests/golden/numeric_time/clock_accuracy.json†L1-L41】【F:../../compiler/runtime/tests/golden/numeric_time/clock_accuracy.audit.jsonl†L1-L3】【F:../../scripts/validate-diagnostic-json.sh†L1-L220】。

---

## 5. スケジュール提案

| 週 | 主タスク | 依存 |
| --- | --- | --- |
| W47 | N-1, N-3, M-1 | 既存 `core_numeric` |
| W48 | T-1, T-2, T-3 | W47 の `metrics` 再編 |
| W49 | M-2, M-3, T-4 | Capability/Env 更新 |
| W50 | Q-1, Q-2, Q-3 | 数値/時間 API 完備 |

完了後は `docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md` と README の進捗表を更新し、`docs-migrations.log` にフォローアップ計画の適用を記録する。
