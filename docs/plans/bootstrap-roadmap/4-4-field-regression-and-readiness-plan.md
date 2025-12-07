# 4.4 Phase 4 フィールドデータ・レグレッション管理計画

## 目的
- Phase 4 M4 「Phase 5 ハンドオーバー判定」に向け、シナリオ網羅率とレグレッション対応状況を可視化し、`phase4-readiness.md` で Self-host チームへ引き継ぐ。
- `.reml` 実行で得たフィールドデータ（Capability 監査、診断ログ、性能測定）を整理し、`impl_fix` / `spec_fix` を即時に切り分けて `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` と連携する。
- `docs/notes/dsl-plugin-roadmap.md`, `docs/guides/plugin-authoring.md`, `docs/guides/runtime-bridges.md` に掲載されている DSL/Plugin 例を Phase 4 スイートへ取り込み、実運用シナリオでの再現性を検証する。
- `phase4-readiness.md` と `reports/spec-audit/ch4/practical-bundle-*.md` を整備し、Phase 5/6 が即時に参照できる判断材料を提供する。

## スコープ
- **含む**: `phase4-readiness.md` のテンプレート策定、`examples/practical/` ミニプロジェクトの実行ログ整備、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` との連携ルール、`docs/plans/bootstrap-roadmap/README.md`/`SUMMARY.md` 更新、週次レビュー手順の文書化。
- **含まない**: 実装コード修正の詳細（該当チームへエスカレーション）、セルフホスト Stage の運用（Phase 5 管轄）、正式リリース（Phase 6 管轄）。
- **前提条件**: `4-1` でシナリオマトリクスが承認済み、`4-2` で実行パイプラインが稼働、`4-3` でメトリクスが収集できる状態。

## 成果物と出口条件
- `docs/plans/bootstrap-roadmap/phase4-readiness.md`（新規）を作成し、シナリオ網羅率、Open Issue、既知制約、次フェーズ要求を記録する。
- `examples/practical/` にミニプロジェクト（`bundle_config`, `bundle_runtime_bridge`, `bundle_cli`, など）を作成し、`reports/spec-audit/ch4/practical-bundle-YYYYMMDD.md` で実行記録を保持する。
- レグレッション報告フロー（`PRACTICAL-###` チケット、`impl_fix`/`spec_fix` 判定、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` とのリンク）を文書化し、週次レビューで実施できる。
- Phase 4 exit 時点で `phase4-scenario-matrix.csv` の網羅率が 85% 以上、`collect-iterator-audit-metrics.py --section practical --require-success` が緑化している、`phase4-readiness.md` が承認されている。

## 作業ブレークダウン

### 1. フィールドシナリオ整備（75週目）
- `docs/notes/dsl-plugin-roadmap.md` で紹介している DSL/Plugin 例を `.reml` ミニプロジェクトに落とし込み、`examples/practical/plugins/` に配置する。Capability 署名や Stage ルールは `docs/spec/3-8-core-runtime-capability.md` と整合させる。
- `examples/practical/bundle_runtime_bridge` では `docs/guides/runtime-bridges.md` の `FlowController` サンプルを実行し、`reports/spec-audit/ch4/practical-bundle-runtime-bridge.md` に結果と監査ログを添付。
- `examples/practical/bundle_cli` では `docs/spec/3-10-core-env.md` の CLI サンプルを `.reml` 化し、`phase4-scenario-matrix.csv` に `category=cli` で登録。

### 2. レグレッション管理フロー（75〜76週目前半）
- `.reml` 実行の Pass/Fail を `phase4-scenario-matrix.csv` の `resolution` へ反映し、`impl_fix` / `spec_fix` / `ok` の判定基準を `phase4-readiness.md` に記述。
- Fail ケースは `PRACTICAL-###`（`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` のサブタスク）として登録し、`docs/notes/phase4-practical-test-backlog.md`（新規）に TODO を残す。
- `docs/plans/rust-migration/overview.md` および `docs/plans/rust-migration/p1-test-migration-*.txt` から関連する既存課題を洗い出し、`impl_fix` と `spec_fix` の優先度を調整。

### 3. `phase4-readiness.md` 作成（76週目前半）
- テンプレート項目: `実行スイート状況`, `シナリオ網羅率`, `診断・監査メトリクス`, `性能傾向`, `既知リスク`, `Phase 5 への要求`, `フォールバック計画`。
- `collect-iterator-audit-metrics.py` の JSON 出力と `reports/spec-audit/ch4/perf-summary.md` を引用し、数値の更新日と CI ジョブ名を明記。
- Phase 5 チームが必要とする `Self-host インプット`（例: `spec-core` の Pass/Fail、`practical` の Stage 依存）を `phase4-readiness.md` の専用セクションにまとめる。

### 4. レビューと承認プロセス（76週目後半〜77週目）
- 週次レビューで `phase4-readiness.md` を更新し、`impl_fix` / `spec_fix` の進捗を確認。`docs/plans/bootstrap-roadmap/README.md` と `SUMMARY.md` の Phase 4 セクションを新たな 4-x 計画へリンク。
- M4 exit 判定では、`phase4-readiness.md` に署名（PM/Tech Lead）が行われ、`docs/plans/bootstrap-roadmap/6-0-phase6-migration.md` に Phase 4 成果のリンクを追加する（別タスク）。
- 承認後、`phase4-readiness.md` を Phase 5 キックオフの必須資料として `reports/spec-audit/ch4/` に保存し、`docs-migrations.log` に記録。

## リスクとフォローアップ
- **シナリオ網羅率未達**: 85% 未満の場合は `phase4-readiness.md` に残課題を明記し、`docs/plans/bootstrap-roadmap/0-4-risk-handling.md` のエスカレーション基準に従って Phase 5 着手可否を判断。
- **レグレッション処理遅延**: `impl_fix` が 2 週以上滞留した場合は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ優先度 `critical` で登録し、Phase 3 の該当章または Rust 実装チームへ通知。
- **フィールドデータ不足**: 現場シナリオ（CLI/Plugin/Capability）が不足する場合は `docs/guides/ai-integration.md` や `examples/pipeline_*` から再利用し、`phase4-scenario-matrix.csv` の `source` 列に根拠を残す。
- **ハンドオーバーギャップ**: `phase4-readiness.md` で Phase 5 が必要とするログやシナリオが欠落している場合は、`4-2`・`4-3` の成果物を再実行し、`reports/spec-audit/ch4/` に追記。`0-2-roadmap-structure.md` の「相互参照維持」を満たすようリンクを確認する。
