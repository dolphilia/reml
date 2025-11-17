# 2.8 仕様完全性監査・最終調整計画

## 目的
- Phase 2 の最終段として、仕様書（Chapter 0〜3）と実装の乖離を徹底的に洗い出し、残りの仕様差分・記述漏れを解消する。
- Rust 版 Reml コンパイラ（`compiler/rust/`）を唯一のアクティブ実装として監査し、Phase 2 で積み上げた `docs/plans/rust-migration/` 系列の成果を Bootstrap Roadmap に再合流させる。
- Phase 3 以降のセルフホスト移行に耐えうるドキュメント品質と参照体制を確立し、外部公開に備えた監査ログ・仕様索引を仕上げる。

## スコープ
- **含む**: 仕様書の全文レビュー、差分リストの完結、索引・脚注・ガイド・ノートの整合、CI によるリンク/スキーマ検証、リスク登録の更新。
- **含まない**: 新機能や将来拡張の提案、新たな API 設計（必要であれば Phase 3 にタスク化）。
- **前提**:
  - Phase 2-5 で主要差分の補正案が承認済みであり、修正案のドラフトが揃っていること。
  - Phase 2-7 で診断・監査パイプラインの運用が安定していること（CI ゲート・LSP テスト完了）。[^phase27-handshake-2-8]
  - 技術的負債リストのうち Phase 2 内で解消できる項目が処理済みで、残項目が Phase 3 引き継ぎとして仕分け済みであること。
  - `docs/plans/rust-migration/overview.md` と `docs/plans/rust-migration/unified-porting-principles.md` に記録された Rust 実装の要件と成果が参照でき、Phase 2-8 で追加の移植作業を行う必要がないこと。

## 作業ディレクトリ
- `docs/spec/0-*` : 索引用資料、用語集、スタイルガイド
- `docs/spec/1-*`, `2-*`, `3-*` : 各章本文・付録
- `docs/guides/` : ガイド整合、AI 連携資料
- `docs/notes/` : 監査結果・TODO 記録
- `docs/plans/` : 既存計画書との相互参照
- `reports/` : 監査ログ・ダッシュボード・差分レポート
- `scripts/` : リンクチェック・スキーマ検証用ツール
- `compiler/rust/` : 仕様整合性を直接確認するための現行実装とテスト資産（Phase 3 以降の主対象）
- `compiler/ocaml/` : 参考資料として参照するのみで、CI や dual-write では利用しない（差分調査時に限定的に参照）

## 作業ブレークダウン

### 1. 監査準備とベースライン収集（36週目後半）
**担当領域**: 準備・計画

1.1. **差分リスト統合**
- Phase 2-5 で作成した差分リストと Phase 2-7 の更新結果を統合し、章・カテゴリ別に並べ替える。
- `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の「差分分類」を最新版に更新し、本計画書へ脚注として参照を追加。

1.2. **レビューチーム編成**
- Chapter 0〜3 の担当者を再割当し、レビューウィンドウ（36-38週）を設定。
- `0-3-audit-and-metrics.md` に監査スケジュールと責任者を記録。

1.3. **検証ツール整備**
- `scripts/validate-diagnostic-json.sh`, `scripts/ci-detect-regression.sh`, `scripts/ci-validate-audit.sh` 等を監査モードで再実行し、ベースライン成果物を `reports/audit/phase2-final/` に集約。
- `docs/notes/` に監査 TODO ノート（`docs/notes/spec-integrity-audit-checklist.md`）を新設し、レビュー項目を列挙。

**成果物**: 統合差分リスト、監査スケジュール、検証ベースライン

### 2. Chapter 0〜1 監査（37週目前半）
**担当領域**: 基本方針と言語コア

2.1. **索引・用語集整合**
- [0-0-overview.md](../../spec/0-0-overview.md), [0-2-glossary.md](../../spec/0-2-glossary.md) を最新仕様と照合し、Term/Definition を更新。
- `docs/spec/0-3-code-style-guide.md` のコード例を Reml スタイルで再確認し、誤記修正。

2.2. **言語コア仕様の整合**
- Chapter 1 (1-1〜1-5) の全文レビューを行い、Phase 2 実装で導入した効果タグ・型クラス辞書・Unicode 対応を再検証。
- 擬似コード・BNF の更新漏れをチェックし、`docs/spec/1-5-formal-grammar-bnf.md` を最新に更新。

2.3. **サンプル検証**
- Rust 版 Reml CLI (`compiler/rust/` ビルド成果) により Chapter 1 のサンプルコード全件をパース/型推論し、結果を `reports/spec-audit/ch1/` に保存。
- エラー発生時は差分リストに追記し、修正案を `docs/notes/spec-integrity-audit-checklist.md` に記録。OCaml 実装での再現確認は任意かつリファレンス使用のみに留める。

**成果物**: 更新済み索引・用語集、Chapter 0〜1 修正案、サンプル検証ログ

### 3. Chapter 2 監査（37週目後半）
**担当領域**: パーサー API

3.1. **API 記述の最終確認**
- `Parser<T>` の型引数・エラー戦略記述を実装コード (`compiler/ocaml/src/parser/`) と照合。
- `docs/guides/core-parse-streaming.md` の内容と Chapter 2 の記述を同時更新。

3.2. **例外・診断との整合**
- `docs/spec/2-5-error.md`, `2-6-execution-strategy.md` と Phase 2-7 で整備した診断 API を突き合わせ、用語とメタデータの整合を確認。
- エラーコード一覧を `docs/spec/3-6-core-diagnostics-audit.md` と同期し、参照表を付録に追加。

3.3. **リンク・脚注検証**
- Chapter 2 からのリンク（ガイド・ノート・計画書）を抽出し、リンク切れを修正。
- BNF と API サンプルの脚注を更新し、`reports/spec-audit/ch2/` に差分レポートを保存。

**成果物**: Chapter 2 修正案、リンク検証レポート、更新済みガイド

### 4. Chapter 3 監査（38週目前半）
**担当領域**: 標準ライブラリ

4.1. **ライブラリ API 整合**
- [3-0-core-library-overview.md](../../spec/3-0-core-library-overview.md)〜[3-10-core-env.md](../../spec/3-10-core-env.md) を精査し、Phase 2 で導入された診断・Capability 情報と乖離がないか確認。
- FFI/Async/Runtime 章で Stage/Ownership テーブルを更新し、`tooling/runtime/audit-schema.json` と一致させる。

4.2. **サンプルコード・図表更新**
- ライブラリ章のコード断片を Reml CLI で再検証し、結果を `reports/spec-audit/ch3/` に記録。
- 図表・フローチャートの差分がある場合は `docs/spec/assets/` を更新。

4.3. **ガイド・ノートとの同期**
- `docs/guides/plugin-authoring.md`, `docs/guides/runtime-bridges.md` などを章更新内容に合わせて調整。
- `docs/notes/dsl-plugin-roadmap.md`, `docs/notes/core-library-outline.md` に監査結果とフォローアップ TODO を記載。

**成果物**: Chapter 3 修正案、更新済み図表、ガイド整合記録（Rust 実装で再現確認済み）

### 5. 修正反映とクロスチェック（38週目後半）
**担当領域**: 最終更新

5.1. **修正案の適用**
- 各章の修正案をマージし、Git 管理の差分を `reports/spec-audit/diffs/` に保存。
- 大規模修正は PR 単位でレビューし、承認ログを `docs/notes/spec-integrity-audit-checklist.md` に記録。

5.2. **リンク・スキーマ検証**
- `scripts/ci-detect-regression.sh` にリンクチェックと JSON Schema 検証を統合し、`spec-audit` モードで実行。
- Rust 実装のテスト (`cargo test -p compiler` 等) と同じステージで実行されるよう CI 手順を同期させ、結果を `reports/spec-audit/summary.md` にまとめる。CI での自動実行手順は `docs/plans/bootstrap-roadmap/IMPLEMENTATION-GUIDE.md` に追記。

5.3. **用語・索引最終更新**
- `docs/README.md`, `README.md` の目次・リンクを更新し、`docs/plans/repository-restructure-plan.md` の進捗を反映。
- 用語集・索引に新旧用語の対応表を追加し、Phase 3 で参照するための脚注を整備。

**成果物**: 更新済み仕様書、検証レポート、索引・リンクの最終版

### 6. リスク登録と Phase 3 引き継ぎ（39週目）
**担当領域**: 記録整備

6.1. **残課題の整理**
- 解決できなかった差分・仕様不明点を `0-4-risk-handling.md` に登録し、優先度を設定。
- Phase 3 で扱うべき TODO を `docs/notes/spec-integrity-audit-checklist.md` に残す。

6.2. **メトリクス更新**
- `0-3-audit-and-metrics.md` に監査件数・修正件数・未解決件数を記録。
- `reports/audit/dashboard/` に Phase 2 の最終スナップショットを保存し、Phase 3 の比較ベースとする。

6.3. **ハンドオーバー**
- Phase 3 リーダー向けに `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` から参照できるハンドオーバー節を作成し、Rust 実装で達成済みの監査項目を一覧化する。
- 仕様更新履歴を `docs/notes/spec-update-log.md`（新設）にまとめ、外部公開時の変更点追跡と Rust 実装への反映状況を容易にする。

**成果物**: リスク登録、メトリクス更新、Phase 3 向けハンドオーバー資料

## 成果物と検証
- 仕様書 Chapter 0〜3 の差分が解消され、CI/手動検証でリンク切れ・スキーマ不整合がゼロであること。
- Rust 実装で Chapter 0〜3 のサンプルと監査ツールがすべて実行され、結果が `reports/spec-audit/*` に保存されていること。
- 監査レポート (`reports/spec-audit/summary.md`) と差分ログが公開され、レビュー履歴が残っていること。
- 用語集・索引が最新状態で、Phase 3 計画書から参照できること。

## リスクとフォローアップ
- 監査範囲の広さによるスケジュール遅延: 優先順位付けを徹底し、Phase 3 に移送する基準を明示。
- 記述更新によるガイド・ノートへの波及: クロスリンク管理を `docs/plans/repository-restructure-plan.md` で追跡し、一括更新スクリプトの導入を検討。
- 外部公開向けチェック未整備: Phase 3 での公開を見据え、ライセンス・記法・翻訳関連の TODO を `docs/notes/spec-integrity-audit-checklist.md` に残す。

## 参考資料
- [2-5-spec-drift-remediation.md](2-5-spec-drift-remediation.md)
- [2-7-deferred-remediation.md](2-7-deferred-remediation.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)
- [docs/spec/0-0-overview.md](../../spec/0-0-overview.md)
- [docs/spec/3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
- [docs/notes/repository-restructure-plan.md](../notes/repository-restructure-plan.md)
- [docs/plans/rust-migration/overview.md](../rust-migration/overview.md)
- [docs/plans/rust-migration/unified-porting-principles.md](../rust-migration/unified-porting-principles.md)

---

### Rust 実装集中への補足
- Phase 2-8 の監査完了をもって、dual-write や OCaml 実装ベースの回帰テストは停止する。必要な場合のみ `compiler/ocaml/` を参照し、差分や履歴を確認する。
- Rust 実装で未着手の Chapter 3 機能は 2-8 の差分リストに `rust-gap` ラベルを付け、3-0 以降のタスクへ直接引き継ぐ。
- 3-x 以降の成果物（Prelude/Collections/Diagnostics 等）を Rust 実装に合わせて更新する際は、2-8 で整理した脚注・索引・監査ロジックを共通の基盤として利用し、Phase 2 で確立した測定・リンク検証スクリプトを維持する。

[^phase27-handshake-2-8]: Phase 2-7 診断パイプライン残課題・技術的負債整理計画の最終成果。`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §5、`docs/plans/bootstrap-roadmap/2-7-to-2-8-handover.md`、`reports/audit/dashboard/diagnostics.md` に記録された監査ベースラインと差分ログを参照する。
