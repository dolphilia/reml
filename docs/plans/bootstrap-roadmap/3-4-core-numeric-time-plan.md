# 3.4 Core Numeric & Time 実装計画

## 目的
- 仕様 [3-4-core-numeric-time.md](../../spec/3-4-core-numeric-time.md) に従って `Core.Numeric`/`Core.Time` API を実装し、数値演算・統計・時間測定の統一モデルを提供する。
- `Core.Diagnostics` と監査メトリクス連携 (`MetricPoint`) を整備し、Chapter 3 他モジュール (Collections/Config) が利用できる形で公開する。
- 時間表現 (Timestamp/Duration) とロケール非依存フォーマットを確立し、IO/Runtime Capability との連携を確保する。

## スコープ
- **含む**: `Numeric`/`OrderedFloat` トレイト、統計ヘルパ、Histogram/Regression、`Timestamp`/`Duration`/`Timezone` API、フォーマット/パース、`MetricPoint` と監査連携。
- **含まない**: GPU/並列集計、分散メトリクス収集、リアルタイム OS 向け拡張 (Phase 4 以降)。
- **前提**: `Core.Collections`/`Core.Iter` が利用可能、`Core.Diagnostics`/`Core.Runtime` の基盤が整備済みであること。

## 作業ブレークダウン

### 1. API 整理とバックログ作成（44週目）
**担当領域**: 設計調整

1.1. 数値トレイト・統計 API・時間 API の公開一覧を作成し、既存実装との差分を分類する。  
実施ステップ:
- `docs/spec/3-4-core-numeric-time.md` を章ごとに読み込み、API 名・戻り値・効果タグを抽出して `docs/plans/bootstrap-roadmap/assets/core-numeric-time-api-diff.csv` に整理する。
- `rg "pub (struct|enum|trait|fn)" compiler/rust/runtime/src -g "*numeric*" -g "*time*"` で Rust 側エクスポートを列挙し、CSV に `Rust実装位置` と `状態 (PoC/Implemented/Missing)` 列を追加する。
- 差分を `docs/notes/core-numeric-time-gap-log.md` に記録し、優先度・担当見込み・参照ファイルを backlog として登録する。

1.2. 効果タグ (`effect {time}`, `{audit}`, `{unicode}`) と Capability 要件を整理し、検証用テストを計画する。  
実施ステップ:
- `docs/spec/3-6-core-diagnostics-audit.md` と `docs/spec/3-8-core-runtime-capability.md` の要求を抜粋し、API ごとの効果タグと `CapabilityStage` をまとめたマトリクス (`docs/plans/bootstrap-roadmap/assets/core-numeric-time-effects-matrix.md`) を作成する。
- `EffectSet` と `CollectorEffectMarkers` が追加タグを記録できるか確認し、必要な場合は `compiler/rust/runtime/src/prelude/collectors/mod.rs` の TODO を記載する。
- 効果タグ検証コマンド `python3 tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario effects_matrix` の設計メモを `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に追記する。

1.3. 依存モジュール (Collections/Diagnostics/IO) との連携ポイントを洗い出し、相互参照更新タスクを作る。  
実施ステップ:
- `Core.Collections`/`Core.Iter`/`Core.Diagnostics`/`Core.Runtime` の仕様と実装パスを整理し、`Numeric` API が利用する依存関係ダイアグラム (`docs/plans/bootstrap-roadmap/assets/core-numeric-time-dependency-map.drawio`) を作る。
- `MetricPoint` → `AuditSink`、`StatisticsError` → `Diagnostic`、`Timestamp` → `IO` の3経路について、参照元ドキュメント（README・Phase3計画・spec脚注）を洗い出し、更新タスクを backlog に分解する。
- 依存図を `README.md` Phase3 表へリンクさせ、進捗報告フォーマットを `docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md` と合わせる。

#### 1.3.1 依存関係ダイアグラムと参照
- `docs/plans/bootstrap-roadmap/assets/core-numeric-time-dependency-map.drawio` に `Core.Numeric`/`Core.Time` と `Core.Collections`/`Core.Iter`/`Core.Diagnostics`/`Core.Runtime`/`Core.IO`/`MetricPoint` の依存関係を図示し、API の実装予定ディレクトリと仕様セクションをまとめた。Phase3 `M4`（Numeric / IO & Path）が参照する仕様と Rust 実装パスを視覚的に把握できる。
- 主要モジュールの対応表:

| モジュール | 仕様参照 | Rust 実装/予定 | Numeric/Time での役割 |
| --- | --- | --- | --- |
| Core.Collections | `docs/spec/3-2-core-collections.md` | `compiler/rust/runtime/src/collections/` | List/Map/Vec を通じて統計 API の入力源・結果コンテナを提供 |
| Core.Iter | `docs/spec/3-1-core-prelude-iteration.md` | `compiler/rust/runtime/src/prelude/iter/` | `IterNumericExt`/`NumericCollector` の土台となり効果タグを共有 |
| Core.Diagnostics | `docs/spec/3-6-core-diagnostics-audit.md` | `compiler/rust/runtime/src/prelude/ensure.rs` + `diagnostics/metric_point.rs`（予定） | `StatisticsError`/`TimeError` を `Diagnostic`・`AuditEnvelope` へ変換 |
| Core.Runtime Capability | `docs/spec/3-8-core-runtime-capability.md` | `compiler/rust/runtime/src/registry.rs`, `stage.rs` | `Numeric`/`Time` API の `StageRequirement`・Capability 検証 (`time.*`, `metrics.emit`) |
| MetricPoint/AuditSink | `docs/spec/3-6-core-diagnostics-audit.md` §4 | `compiler/rust/runtime/src/diagnostics/metric_point.rs`（予定） | `effect {audit}` を打刻し `AuditEnvelope` へメトリクスを送出 |
| Core.IO / Env | `docs/spec/3-5-core-io-path.md` | `compiler/rust/runtime/src/io/` + adapter | `Timestamp`/`Duration`/`Timezone` が利用するクロック・ロケール情報 |

#### 1.3.2 MetricPoint → AuditSink 連携バックログ
- 仕様 `docs/spec/3-4-core-numeric-time.md` §4 と `docs/spec/3-6-core-diagnostics-audit.md` §4 を突き合わせ、`MetricPoint`/`IntoMetricValue`/`emit_metric` が `effect {audit}` を記録した上で `AuditSink` (`AuditEnvelope.metadata.metric_point.*`) に連携する経路を確立する必要がある。
- KPI と監査ログの観測位置は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の Phase3 指標へ `numeric.metrics.emit_success_rate` / `numeric.metrics.audit_ingest_latency_ns` を追加する形で追跡する。
- `README.md#core-numeric--time-進捗` と `docs/notes/core-numeric-time-gap-log.md`（2025-12-01「監査連携」行）にタスクを登録済み。対応内容: `MetricPoint` 実装、`collect-iterator-audit-metrics.py --section numeric_time --scenario emit_metric` の CI 化、Phase3 Self-Host (`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` M4 行) からの参照更新。

#### 1.3.3 StatisticsError → Diagnostic 連携バックログ
- `docs/spec/3-4-core-numeric-time.md` §2（統計・データ品質）と `docs/spec/3-7-core-config-data.md` §2 のサンプルでは `StatisticsError`/`NumericError` が `Diagnostic` に昇格し、`column`/`aggregation`/`audit_id` メタデータを必須とする。現状 Rust 実装には `StatisticsErrorKind` も `IntoDiagnostic` も存在せず、Config 章・Diagnostics 章とリンクしていない。
- 今後実装する `compiler/rust/runtime/src/numeric/error.rs` → `Core.Diagnostics` ブリッジでは、`EffectSet`/`CollectorEffectMarkers` の記録順序と `AuditEnvelope.metadata.numeric.*` のフィールド設計をまとめる必要がある。
- `docs/notes/core-numeric-time-gap-log.md`（2025-12-01「診断連携」行）でバックログを管理し、`README.md#core-numeric--time-進捗` および `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` M4 行から参照できるようにした。`scripts/validate-diagnostic-json.sh --suite numeric` の追加もこのタスクに含まれる。

#### 1.3.4 Timestamp → IO 連携バックログ
- 時間 API (`docs/spec/3-4-core-numeric-time.md` §3) は `Core.IO` (`docs/spec/3-5-core-io-path.md`) と `Core.Runtime` Capability (`time.now`, `time.sleep`, `timezone.resolve`) に依存する。Rust 実装では `compiler/rust/runtime/src/io/` の `Env` アダプタと `runtime/src/registry.rs` を跨ぐ Stage 検証が未定義で、`Timestamp`/`Duration`/`Timezone` が `effect {time}` や `CapabilityStage::{Exact,AtLeast}` を記録できていない。
- `docs/notes/core-numeric-time-gap-log.md`（2025-12-01「IO 連携」行）で `Core.Time` ↔ `Core.IO` の作業と `timezone`/`Env::platform()` 参照を追跡する。`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` の `M4: Numeric / IO & Path` 行へ依存図リンクを張り、IO 章と Time 章の更新が同期できるよう README と本計画書の両方で参照ポイントを明示した。
- `tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario clock_accuracy`（新設予定）と `reports/spec-audit/ch3/time_clock-*.json` を使い、`Timestamp → IO` 経路の KPI (`time.syscall.latency_ns`, `time.timezone.lookup_success_rate`) を `0-3-audit-and-metrics.md` に登録する。

> 進行ログ（Phase3 W44）  
> - `docs/plans/bootstrap-roadmap/assets/core-numeric-time-dependency-map.drawio` を追加し、`Core.Collections/Core.Iter/Core.Diagnostics/Core.Runtime/Core.IO` と `Core.Numeric/Core.Time` の依存関係・仕様参照・実装パスを一覧化した。README（`README.md#core-numeric--time-進捗`）と Phase3 Self-Host (`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` M4 行) から参照できるようリンク付け済み。  
> - `MetricPoint → AuditSink`、`StatisticsError → Diagnostic`、`Timestamp → IO` の 3 経路について `docs/notes/core-numeric-time-gap-log.md` に 2025-12-01 付けでバックログを登録し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` と README から追跡できるよう整理した。  
> - `README.md#core-numeric--time-進捗` セクションを新設し、本計画書 §1.3 の進捗と依存図を Phase3 全体のロードマップへ共有（`Core.Collections`/`Core.Text` セクションと同一フォーマット）した。

### 2. 数値トレイト・ユーティリティ実装（44-45週目）
**担当領域**: 基本演算

2.1. `Numeric`/`OrderedFloat` トレイトと基本関数 (`lerp`, `mean`, `variance`, `percentile` 等) を実装し、`Iter` ベースでテストする。  
実施ステップ:
- `compiler/rust/runtime/src/numeric/mod.rs` にトレイト定義とデフォルト実装を追加し、`Core.Iter` の `try_fold` を使うためのヘルパ (`IterNumericExt`) を設計する。
- `docs/spec/1-2-types-Inference.md` の `Numeric<T>` 制約と照合し、型推論テスト (`compiler/rust/frontend/tests/type_numeric.rs`) 追加計画を `docs/plans/rust-migration/1-1-ast-and-ir-alignment.md` にリンクする。
- `cargo test --manifest-path compiler/rust/runtime/Cargo.toml numeric_basic --features core-numeric` で `mean/variance/percentile` のゴールデンを検証し、結果を `reports/spec-audit/ch3/numeric_basic-*.json` に保存する。

> 進行ログ（Phase3 W44, 2.1）  
> - `core_numeric` feature を追加し、`compiler/rust/runtime/src/numeric/mod.rs` に `Numeric`/`OrderedFloat`/`Floating`/`IterNumericExt`・`lerp`/`mean`/`variance`/`percentile` を PoC 実装。`Iter.try_fold` ベースで Welford 法を採用し、`percentile` は nearest-rank + 線形補間を利用。  
> - `docs/plans/bootstrap-roadmap/assets/core-numeric-time-api-diff.csv` と `docs/notes/core-numeric-time-gap-log.md` を更新し、`Numeric` 制約テスト計画（`compiler/rust/frontend/tests/type_numeric.rs`）を `docs/plans/rust-migration/1-1-ast-and-ir-alignment.md` に追記。  
> - `cargo test --manifest-path compiler/rust/runtime/Cargo.toml --features core-numeric` を実行して `numeric::tests::*` を追加検証。`reports/spec-audit/ch3/numeric_basic-*` の自動化は別途スクリプト化する。

2.2. `HistogramBucket`/`HistogramBucketState` の実装と検証を行い、不正パラメータ時の `StatisticsError` 処理を整備する。  
実施ステップ:
- `docs/spec/3-7-core-config-data.md` の `ColumnStats` 記述からバケット検証ルールを抽出し、`docs/plans/bootstrap-roadmap/assets/histogram-error-matrix.md` に `StatisticsErrorKind` 対応表を作る。
- `compiler/rust/runtime/src/numeric/histogram.rs` を追加し、`Result<List<HistogramBucketState>, Diagnostic>` を返す実装と `IntoDiagnostic` 変換を定義する。
- `tests/data/numeric/histogram/*.json` に正常/異常ケースを揃え、`scripts/validate-diagnostic-json.sh --pattern numeric.histogram` を CI ゲートへ追加する。

> 進行ログ（Phase3 W45, 2.2）  
> - `docs/plans/bootstrap-roadmap/assets/histogram-error-matrix.md` を新設し、`H-01`〜`H-07` の検証ルールを `StatisticsErrorKind`/診断コードにマッピング。`Core.Config` §4.8 との整合を表で追跡できるようにした。  
> - `compiler/rust/runtime/src/numeric/error.rs` と `compiler/rust/runtime/src/numeric/histogram.rs` を追加し、`StatisticsError`→`GuardDiagnostic` 変換・`HistogramBucket`/`HistogramBucketState`・`histogram` 本体・単体テストを PoC 実装。`cargo test --manifest-path compiler/rust/runtime/Cargo.toml --features core-numeric` で検証済み。  
> - `tests/data/numeric/histogram/{success_basic,error_overlap}.json` を追加し、`scripts/validate-diagnostic-json.sh --pattern numeric.histogram` で `diagnostic-v2` スキーマとメタデータ拡張（`numeric.statistics.*`, `effects.*`）をバリデートできるようにした。  
> - `docs/plans/bootstrap-roadmap/assets/core-numeric-time-api-diff.csv` の `HistogramBucketState`/`histogram`/`StatisticsError*` 行を `PoC` へ更新し、`README.md#core-numeric--time-進捗` と `docs/notes/core-numeric-time-gap-log.md` に進行状況をリンクさせた。

2.3. 統計関数の数値安定性を確認し、再現性のあるベンチマークを追加する。  
実施ステップ:
- `docs/notes/core-numeric-stability.md` を作成し、`Kahan summation`・`Welford` 法・`Horvitz-Thompson` (回帰) の採用理由と代替案を明記する。
- `compiler/rust/runtime/benches/bench_numeric_statistics.rs` を `criterion` で実装し、`reports/benchmarks/numeric-phase3/*.json` に結果を記録して `docs/plans/rust-migration/3-2-benchmark-baseline.md` と同期する。
- `StatisticsErrorKind::NumericalInstability` を返した際の診断例を `docs/notes/core-numeric-time-gap-log.md` に残し、再現入力を `tests/data/numeric/instability/*.json` へ追加する。

> 進行ログ（Phase3 W45, 2.3）  
> - `docs/notes/core-numeric-stability.md` を新設し、`Numeric`/`Iter` における Kahan summation・Welford 法の採用理由、および将来の Horvitz-Thompson 適用方針を整理した。仕様 §2.3 の「数値的不安定性検出」要件を満たすための根拠・TODO を記録。  
> - `compiler/rust/runtime/benches/bench_numeric_statistics.rs` を追加し、`cargo bench --manifest-path compiler/rust/runtime/Cargo.toml --features core-numeric --bench bench_numeric_statistics -- --noplot` で実行。`reports/benchmarks/numeric-phase3/phase3-baseline-2025-12-04.json` に平均/分散/百分位の初回ベースラインを保存し、`docs/plans/rust-migration/3-2-benchmark-baseline.md` へスイート行を追記した。  
> - `tests/data/numeric/instability/histogram_non_finite.json` を登録し、`StatisticsErrorKind::NumericalInstability` (`rule = H-05`) を再現する診断サンプルを `docs/notes/core-numeric-time-gap-log.md` へリンク。JSON には `numeric.statistics.kind = numerical_instability` と `sample_value = NaN` を含み、監査メタデータの確認指針を記載した。

### 3. 統計・データ品質 API 拡充（45週目）
**担当領域**: コレクション連携

3.1. `quantiles`/`correlation`/`linear_regression` 等の高度統計を実装し、`Map`/`List` との連携をテストする。  
実施ステップ:
- `compiler/rust/runtime/src/numeric/statistics.rs` に高度統計 API を集約し、`Core.Collections` の `List`/`Map` ラッパを使った戻り値変換を実装する。
- `quantiles` の `points` 前処理で `effect {mem}` を記録するため `EffectSet::record_mem_bytes` を導入し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `numeric.quantiles.mem_bytes` KPI を追加する。
- `tests/expected/numeric_quantiles.json`・`numeric_regression.json` を作成し、`tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario statistics_accuracy` で自動検証する。

> 進行ログ（Phase3 W45, 3.1）  
> - `compiler/rust/runtime/src/numeric/statistics.rs` を追加し、`quantiles`/`correlation`/`linear_regression` を 1 パス実装。`LinearModel { slope, intercept, r_squared }` を導入し、`ordered-float` による `Map<QuantilePoint, f64>` を返す設計を採用した。  
> - ThreadLocal `compiler/rust/runtime/src/numeric/effects.rs` と `take_numeric_effects_snapshot()` を実装し、`quantiles` のポイント・サンプル確保時に `effect {mem}` / `mem_bytes` を記録できるようにした。  
> - `tests/expected/{numeric_quantiles.json,numeric_regression.json}` を作成して `statistics_accuracy` 入力を共有し、`numeric/statistics.rs` の単体テストで `OrderedFloat` キー／`StatisticsError` 分岐／効果計測を検証した。`docs/plans/bootstrap-roadmap/assets/core-numeric-time-api-diff.csv` と `0-3-audit-and-metrics.md` の KPI 行も同期済み。

3.2. `StatisticsError` → `Diagnostic` 変換を実装し、Config/Data 章で要求されるメッセージ整形を確認する。  
実施ステップ:
- `compiler/rust/runtime/src/numeric/error.rs` に `StatisticsErrorKind` 毎の `code`/`metadata`/`hints` を定義し、`IntoDiagnostic` 実装を `Core.Diagnostics` の `DiagnosticBuilder` と連携させる。
- `docs/spec/3-7-core-config-data.md` §2 の例や `reports/spec-audit/ch3/config-data-statistics.json` を参照し、`column`, `aggregation`, `audit_id` など必須キーを確認する。
- `scripts/validate-diagnostic-json.sh --suite numeric` を追加し、CLI/LSP/Runtime の JSON が同一 schema を満たすことをチェックする。

3.3. `rolling_average`/`z_score` 等の遅延計算が `Iter` と安全に連携することを確認する。  
実施ステップ:
- `compiler/rust/runtime/src/prelude/iter/collectors/mod.rs` に `NumericCollector` を追加し、`rolling_average` が `IterStage::Streaming` と互換であるか `StageRequirement` テストを整備する。
- `compiler/rust/runtime/tests/iter_numeric_props.rs` で `Iter` + 遅延計算の QuickCheck を実施し、リークや `effect {mem}` の過剰記録がないか検証する。
- `docs/plans/bootstrap-roadmap/checklists/core-iter-numeric.md` にテストケース・依存条件・責任者を記載し、`Core.Iter` 章との整合を維持する。

### 4. 時間・期間 API 実装（45-46週目）
**担当領域**: 時刻処理

4.1. `Timestamp`/`Duration` と基本操作 (`now`, `monotonic_now`, `duration_between`, `sleep`) を実装し、`effect {time}` の検証を行う。  
実施ステップ:
- `compiler/rust/runtime/src/time/mod.rs` に `Timestamp`/`Duration` 型と `SystemClockAdapter` を実装し、`std::time::{SystemTime, Instant}` からの変換パスを確立する。
- `EffectSet` に `record_time_call` を追加し、`now`/`monotonic_now`/`sleep` 呼び出し時のシステムコール遅延を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI (`time.syscall.latency_ns`) へ送る。
- `tests/expected/time_now.json`・`time_sleep.json` を作成し、`tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario clock_accuracy` を実行して再現性を計測する。

4.2. `TimeError`/`TimeFormat`/`Timezone` API を実装し、OS 依存情報を `Capability`/`Env` と連携するテストを作成する。  
実施ステップ:
- `compiler/rust/runtime/src/time/error.rs` に `TimeErrorKind` と `IntoDiagnostic` 実装を追加し、`Env::platform()` の情報を `metadata.time.platform` に記録する。
- タイムゾーン解決 (`timezone`, `local`) を `runtime/src/capabilities/timezone.rs` 経由にまとめ、`docs/notes/runtime-capability-stage-log.md` に OS ごとの差分を保存する。
- `tests/data/time/timezone_cases.json` を用意し、`python3 tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario timezone_lookup --tz-source tests/data/time/timezone_cases.json` で Linux/macOS/Windows の挙動を比較する。

4.3. フォーマット (`format`)/パース (`parse`) を実装し、`Locale`/ICU 依存部分のエラーハンドリングを確認する。  
実施ステップ:
- `compiler/rust/runtime/src/time/format.rs` に RFC3339/Unix/Custom をまとめ、`Core.Text` の `LocaleId` と共有する `docs/plans/bootstrap-roadmap/assets/time-format-locale-map.csv` を作る。
- `TimeFormat` エラーを `TimeError` に変換する `impl From` を定義し、`Diagnostic` へ昇格する際に `time.format.pattern` と `locale` を必須メタデータとして出力する。
- `tests/data/time/format/*.json` を整備し、`cargo test --manifest-path compiler/rust/runtime/Cargo.toml time_format_cases --features core-numeric` で ICU 互換性と `effect {unicode}` 計測を確認する。

### 5. メトリクス・監査統合（46週目）
**担当領域**: Diagnostics 連携

5.1. `MetricPoint`/`IntoMetricValue` を実装し、`emit_metric` が `AuditSink` と整合することを確認する。  
実施ステップ:
- `compiler/rust/runtime/src/diagnostics/metric_point.rs` を新設し、`MetricPoint` と `IntoMetricValue` を `Float`/`Int`/`Duration`/`Timestamp` 向けに実装する。
- `emit_metric` の JSON 形式を `docs/spec/3-6-core-diagnostics-audit.md` と照合し、`reports/spec-audit/ch3/metric_point-*.json` をゴールデンとして保存する。
- `python3 tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario emit_metric --metric-source tests/data/metrics/metric_point_cases.json` を追加し、`effect {audit}` 計測を自動化する。

5.2. `attach_audit` 等のヘルパで `AuditEnvelope` を取り扱うテストを整備し、監査ログ記録を `0-3-audit-and-metrics.md` に反映する。  
実施ステップ:
- `compiler/rust/runtime/src/diagnostics/audit_bridge.rs` に `attach_audit`/`with_metric_tags` を追加し、`AuditEnvelope.metadata.metric_point.*` を統一する。
- KPI `metrics.emit.success_rate` を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に登録し、`reports/audit/metric_point/*.jsonl` を自動生成する CI ジョブを設計する。
- `scripts/validate-diagnostic-json.sh --pattern metrics.emit` を追加し、CLI/LSP/Runtime の監査ログが一致するかを検証する。

5.3. CLI/ランタイム (3-8) との契約を確認し、Capability Stage 検証のフックを追加する。  
実施ステップ:
- `docs/spec/3-8-core-runtime-capability.md` に沿って `emit_metric` 呼び出し前に `RuntimeBridgeRegistry::verify_capability_stage("metrics.emit")` を行う設計を `docs/notes/runtime-metrics-capability.md` にまとめる。
- CLI (`remlc metrics emit`) と Runtime ブリッジに Stage mismatch 用の診断 (`effects.contract.stage_mismatch`) を追加し、`reports/dual-write/metrics-stage-mismatch.json` を作成する。
- `compiler/rust/runtime/tests/metrics_capability.rs` と `tooling/lsp/tests/metrics_stage.json` を追加し、ステージ検証が Linux/macOS/Windows で一致するかを CI で確認する。

### 6. ドキュメント・サンプル更新（46-47週目）
**担当領域**: 情報整備

6.1. 仕様書内サンプルの実行結果を確認し、`examples/` に統計・時間 API の例を追加する。  
実施ステップ:
- `docs/spec/3-4-core-numeric-time.md` のコード例を `examples/core-numeric/statistics.reml`・`examples/core-time/timezones.reml` として実装し、`README.md` の Examples 表へ追記する。
- `cargo run --example core_numeric_statistics` などの実行結果を `reports/examples/core-numeric/*.md` に保存し、仕様との出力差分を確認する。
- `examples/README.md` に `--features core-numeric` の必要性と実行手順を明記する。

6.2. `3-0-phase3-self-host.md` へ Numeric/Time 実装状況を追記し、`README.md` の Phase 3 セクションを更新する。  
実施ステップ:
- `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` の進捗表へ `Core.Numeric`/`Core.Time` のスプリント別完了条件を追加する。
- `README.md` Phase3 ロードマップの bullet を更新し、本計画書へのリンクを設定する。
- ドキュメント更新履歴を `docs-migrations.log` に記録しておく。

6.3. `docs/guides/runtime-bridges.md`/`docs/guides/ai-integration.md` 等でメトリクス活用例を更新する。  
実施ステップ:
- `docs/guides/runtime-bridges.md` に `MetricPoint`/`emit_metric` を使ったブリッジ実装例を追加し、`docs/guides/ai-integration.md` には AI ワークロード監視のケーススタディを記載する。
- `docs/notes/dsl-plugin-roadmap.md` にメトリクス/時間 API を利用するプラグイン要件を追記し、Capability Stage との整合を脚注に記す。
- ガイド更新後に `scripts/check-links.sh docs/guides` を実行してリンク切れを検証する。

### 7. テスト・ベンチマーク統合（47週目）
**担当領域**: 品質保証

7.1. 単体テストと QuickCheck スタイルのプロパティテストを導入し、統計結果と時間計算の妥当性を検証する。  
実施ステップ:
- `compiler/rust/runtime/tests/numeric_props.rs`・`time_props.rs` を追加し、`proptest` で `mean`/`variance`/`duration_between` の不変条件を検証する。
- `compiler/rust/runtime/tests/golden/numeric_time/*.json` を作り、`scripts/validate-diagnostic-json.sh --suite numeric_time` で CLI/Runtime/LSP の差分を監視する。
- テストケース一覧を `docs/plans/bootstrap-roadmap/checklists/core-numeric-time-tests.md` にまとめ、責任者と再実行コマンドを明示する。

7.2. ベンチマークスイート (集計/時間計測) を追加し、Rust 実装の Phase 2 ベースラインと比較して ±15% 以内であるかを確認する。OCaml 実装は設計上の参考としてのみ参照する。  
実施ステップ:
- `compiler/rust/runtime/benches/numeric_iter.rs`・`time_clock.rs` を `criterion` で追加し、データ規模（1k/100k/10M）ごとのスコアを `reports/benchmarks/numeric-time/*.json` に記録する。
- OCaml ベースライン (`reports/benchmarks/ocaml/numeric_time_baseline.json`) との差分を `docs/plans/rust-migration/3-2-benchmark-baseline.md` に書き戻す。
- KPI `numeric.bench.latency_ms`・`time.clock.jitter_us` を更新し、閾値超過時のフォローアップ手順を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に記載する。

7.3. CI に `--features core-numeric` 等の機能ゲートを追加し、測定結果をメトリクス文書へ記録する。  
実施ステップ:
- `.github/workflows/phase3-rust.yml` に `cargo test --manifest-path compiler/rust/runtime/Cargo.toml --features core-numeric,core-time` を含むジョブを追加し、成果物 (`reports/ci/numeric-time/*`) をアップロードする。
- `tooling/ci/collect-iterator-audit-metrics.py` に `section=numeric_time` の収集フローを実装し、CI 実行時に `reports/metrics/numeric-time-latest.json` を生成する。
- CI 成果物へのリンクを `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` と `docs/plans/rust-migration/3-0-ci-and-dual-write-strategy.md` に追記し、監査ログの保存場所を `docs/notes/core-numeric-time-ci-log.md` として記録する。

## 成果物と検証
- `Core.Numeric`/`Core.Time` API が仕様通りに実装され、効果タグと診断連携が正しく機能していること。
- 統計・時間処理のベンチマークが基準値内であり、差分が文書化されていること。
- ドキュメントとサンプルが更新され、他章との相互参照が解決していること。

## リスクとフォローアップ
- 浮動小数点の精度問題が解決しない場合、`Decimal`/`BigInt` の専用最適化や外部ライブラリ活用をフォローアップに追加する。
- `sleep` など時間 API が環境依存で不安定な場合、Phase 3-8 (Runtime Capability) で補強する。
- 監査メトリクスの性能が不足する場合、非同期送信やバッチ化を Phase 4 の改善項目に記録する。

## 参考資料
- [3-4-core-numeric-time.md](../../spec/3-4-core-numeric-time.md)
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
- [3-7-core-config-data.md](../../spec/3-7-core-config-data.md)
- [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
