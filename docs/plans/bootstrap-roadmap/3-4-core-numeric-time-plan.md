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
1.2. 効果タグ (`effect {time}`, `{audit}`, `{unicode}`) と Capability 要件を整理し、検証用テストを計画する。
1.3. 依存モジュール (Collections/Diagnostics/IO) との連携ポイントを洗い出し、相互参照更新タスクを作る。

### 2. 数値トレイト・ユーティリティ実装（44-45週目）
**担当領域**: 基本演算

2.1. `Numeric`/`OrderedFloat` トレイトと基本関数 (`lerp`, `mean`, `variance`, `percentile` 等) を実装し、`Iter` ベースでテストする。
2.2. `HistogramBucket`/`HistogramBucketState` の実装と検証を行い、不正パラメータ時の `StatisticsError` 処理を整備する。
2.3. 統計関数の数値安定性を確認し、再現性のあるベンチマークを追加する。

### 3. 統計・データ品質 API 拡充（45週目）
**担当領域**: コレクション連携

3.1. `quantiles`/`correlation`/`linear_regression` 等の高度統計を実装し、`Map`/`List` との連携をテストする。
3.2. `StatisticsError` → `Diagnostic` 変換を実装し、Config/Data 章で要求されるメッセージ整形を確認する。
3.3. `rolling_average`/`z_score` 等の遅延計算が `Iter` と安全に連携することを確認する。

### 4. 時間・期間 API 実装（45-46週目）
**担当領域**: 時刻処理

4.1. `Timestamp`/`Duration` と基本操作 (`now`, `monotonic_now`, `duration_between`, `sleep`) を実装し、`effect {time}` の検証を行う。
4.2. `TimeError`/`TimeFormat`/`Timezone` API を実装し、OS 依存情報を `Capability`/`Env` と連携するテストを作成する。
4.3. フォーマット (`format`)/パース (`parse`) を実装し、`Locale`/ICU 依存部分のエラーハンドリングを確認する。

### 5. メトリクス・監査統合（46週目）
**担当領域**: Diagnostics 連携

5.1. `MetricPoint`/`IntoMetricValue` を実装し、`emit_metric` が `AuditSink` と整合することを確認する。
5.2. `attach_audit` 等のヘルパで `AuditEnvelope` を取り扱うテストを整備し、監査ログ記録を `0-3-audit-and-metrics.md` に反映する。
5.3. CLI/ランタイム (3-8) との契約を確認し、Capability Stage 検証のフックを追加する。

### 6. ドキュメント・サンプル更新（46-47週目）
**担当領域**: 情報整備

6.1. 仕様書内サンプルの実行結果を確認し、`examples/` に統計・時間 API の例を追加する。
6.2. `3-0-phase3-self-host.md` へ Numeric/Time 実装状況を追記し、`README.md` の Phase 3 セクションを更新する。
6.3. `docs/guides/runtime-bridges.md`/`docs/guides/ai-integration.md` 等でメトリクス活用例を更新する。

### 7. テスト・ベンチマーク統合（47週目）
**担当領域**: 品質保証

7.1. 単体テストと QuickCheck スタイルのプロパティテストを導入し、統計結果と時間計算の妥当性を検証する。
7.2. ベンチマークスイート (集計/時間計測) を追加し、Rust 実装の Phase 2 ベースラインと比較して ±15% 以内であるかを確認する。OCaml 実装は設計上の参考としてのみ参照する。
7.3. CI に `--features core-numeric` 等の機能ゲートを追加し、測定結果をメトリクス文書へ記録する。

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
