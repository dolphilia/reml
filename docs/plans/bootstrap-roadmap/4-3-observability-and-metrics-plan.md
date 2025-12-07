# 4.3 Phase 4 観測・メトリクス統合計画

## 目的
- Phase 4 M3 「観測メトリクス接続」を達成し、`.reml` 実行結果の性能・診断・監査データを `0-3-audit-and-metrics.md` の KPI と連動させる。
- `collect-iterator-audit-metrics.py --section practical` を整備し、`spec.chapter1.pass_rate` などの指標を自動集計できるようにする。
- `reports/spec-audit/ch4/` にダッシュボード（`spec-core-dashboard.md`, `practical-suite-index.md`, `perf-summary.md` 等）を追加し、Phase 5/6 のレビューで参照できる統一フォーマットを提供する。
- `scripts/validate-diagnostic-json.sh`・`scripts/validate-phase4-golden.sh` と連携し、診断や監査ログの逸脱を即時検知する。
- `.reml` テストの結果から「仕様どおり」「仕様の記述不足」「実装修正必要」を定量化し、複数表記・境界・意地悪ケースのカバレッジを Chapter 1〜3 毎に可視化する。

## スコープ
- **含む**: `tooling/ci/collect-iterator-audit-metrics.py` の拡張、`reports/spec-audit/ch4/` ダッシュボード生成スクリプト、`docs/spec/0-3-audit-and-metrics.md` の KPI 追記、`AuditEnvelope` へのメタデータフィールド追加方針の文書化。
- **含まない**: `.reml` 実行パイプラインの構築（`4-2` で管理）、Self-host Stage 測定（Phase 5 で対応）、OCaml 実装由来の診断互換性検証。
- **前提条件**: `4-1` で `phase4-scenario-matrix.csv` が揃い、`4-2` で `.reml` スイートが CI で実行できる状態。

## 成果物と出口条件
- `collect-iterator-audit-metrics.py` に `--section practical` オプションと `spec.chapter{1,2,3}.pass_rate`, `practical.pass_rate`, `practical.stage_mismatch`, `core_prelude.guard.failures` などのカウンタを追加する。
- `reports/spec-audit/ch4/spec-core-dashboard.md`, `reports/spec-audit/ch4/practical-suite-index.md`, `reports/spec-audit/ch4/perf-summary.md` を生成するスクリプト（`scripts/gen_phase4_dashboard.py` 等）を整備し、CI から自動更新できる。
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に Phase 4 KPI を追記し、測定手順・更新頻度を明記する。
- `.github/workflows/phase4-practical.yml`（または既存ワークフロー）から `collect-iterator-audit-metrics.py --section practical --require-success` を呼び出し、M3 exit レビューで緑化レポートを提示する。
- `spec_core_dashboard` には `variant=canonical|alternate|boundary|invalid` 別の Pass/Fail、`impl_fix`/`spec_fix` の件数、`resolution` 率を記載し、Chapter 1〜3 の網羅度を一目で確認できるようにする。

## 作業ブレークダウン

### 1. メトリクス設計とスキーマ定義（74週目前半）
- `docs/spec/0-1-project-purpose.md` と `0-3-audit-and-metrics.md` を参照し、性能（`parse_throughput`, `memory_peak_ratio`）と安全性（`stage_mismatch`, `diagnostic_regressions`）の指標を Phase 4 向けに再定義。
- `phase4-scenario-matrix.csv` から `spec.chapter`・`category` を読み取り、メトリクス集計に必要なキー（`scenario.id`, `input.hash`, `runtime.bridge`, `capability.stage`）を一覧化。
- `AuditEnvelope.metadata` に追加するフィールド（`scenario.id`, `spec.chapter`, `input.hash`, `runtime.bridge`, `capability.stage.required`, `capability.stage.actual`）を `docs/spec/3-6-core-diagnostics-audit.md` の脚注案としてまとめる。

### 2. ツール実装（74週目後半〜75週目前半）
- `tooling/ci/collect-iterator-audit-metrics.py` に `Phase4Metrics` クラスを追加し、`phase4-scenario-matrix.csv` の ID ベースで Pass/Fail を集計する。`--section practical` 実行時は 4-2 スイートのログディレクトリを読み取る。
- `scripts/gen_phase4_dashboard.py` を追加し、`reports/spec-audit/ch4/` に Markdown ダッシュボードを生成。`spec_core_dashboard` は Chapter 1〜3 の Pass/Fail 表、`practical_suite_index` はカテゴリ別 KPI とリンク、`perf_summary` は parse/memory の中央値を掲載。
- `scripts/validate-diagnostic-json.sh`・`scripts/validate-phase4-golden.sh` を更新し、`scenario.id` や `spec.chapter` が欠落していないか検証。欠落時は CI を失敗させる。
- `Phase4Metrics` では `variant`（`canonical`/`alternate`/`boundary`/`invalid`）および `resolution`（`ok`/`impl_fix`/`spec_fix`）を必須フィールドとし、Chapter 1 の全規則で 4 種類が揃っていない場合は `chapter1.variant_coverage` を 1 未満としてアラートを出す。

### 3. CI 統合とレポート配布（75週目）
- `.github/workflows/phase4-practical.yml` から `collect-iterator-audit-metrics.py --section practical --require-success --matrix docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` を呼び出す。
- CI アーティファクトとして `reports/spec-audit/ch4/*.md`, `metrics/phase4-practical.json`, `metrics/phase4-perf.json` を保存し、`reports/spec-audit/ch4/README.md` に再取得手順を記載。
- `docs/plans/bootstrap-roadmap/README.md` と `SUMMARY.md` の Phase 4 セクションへ、観測・メトリクス計画の概要を追記。
- `metrics/phase4-practical.json` には `spec.chapter1.variant_coverage`, `spec.chapter1.boundary_pass_rate`, `spec.chapter2.variant_coverage`, `spec.chapter3.variant_coverage`, `practical.impl_fix_ratio`, `practical.spec_fix_ratio` を追加し、仕様 vs 実装修正の傾向を可視化する。

### 4. KPI 追記と運用ルール（75週目後半）
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ `spec.chapter1.pass_rate`, `spec.chapter1.variant_coverage`, `spec.chapter1.boundary_pass_rate`, `spec.chapter2.pass_rate`, `spec.chapter2.variant_coverage`, `spec.chapter3.pass_rate`, `spec.chapter3.variant_coverage`, `practical.pass_rate`, `practical.stage_mismatch`, `practical.impl_fix_ratio`, `practical.spec_fix_ratio`, `core_prelude.guard.failures`, `phase4.perf.parse_throughput`, `phase4.perf.memory_peak_ratio` を追加。
- KPI 更新手順を `reports/spec-audit/ch4/README.md` と `docs/plans/bootstrap-roadmap/4-4-field-regression-and-readiness-plan.md` にリンクし、レビュー時の参照ポイントを統一。
- `docs-migrations.log` に新規メトリクスとダッシュボードのエントリを記録し、Phase 5 での Self-host 測定へ引き継ぐ。
- Chapter 1〜3 それぞれに `edge_case_required` フラグを設け、ギリギリテストが未達の場合は KPI を `yellow` 扱いにし、`4-4` レビューへ自動通知する。

## リスクとフォローアップ
- **メトリクス未収束**: Pass/Fail が安定しない場合は `phase4-scenario-matrix.csv` の `priority` 列で `critical` を明示し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` にリンク。CI の `--require-success` を暫定で緩和する場合はレビューで承認を得る。
- **ログ肥大化**: `reports/spec-audit/ch4/*.md` と JSON ログが肥大化する場合は、圧縮アーカイブ (`.tar.zst`) と最新サマリ（`.md`）を分離し、`0-4-risk-handling.md` のストレージ対策ガイドに従う。
- **仕様齟齬の検出**: `spec.chapterX.pass_rate < 0.9` になった場合は、`docs/spec/1-x` または `compiler/rust/*` のどちらで修正すべきかを `phase4-scenario-matrix.csv` の `resolution` 列で明示し、`4-4` のレビューへ引き継ぐ。
- **バリエーション欠落**: `chapter1.variant_coverage < 1.0` の場合は該当する `.reml` の追加を最優先で計画し、`phase4-scenario-matrix.csv` に `missing_variant` タグを立てて 1 週間以内に追加テストを作成する。
