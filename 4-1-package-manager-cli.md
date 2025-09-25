# 4.1 Package Manager & CLI（ドラフト）

> 目的：Reml エコシステムの基盤となる公式 CLI (`reml`) およびパッケージマネージャーの仕様を定義し、Chapter 1-3 で規定された言語・標準API・ランタイム機能と統合する。現段階では 4-0 の統合計画に基づくアウトラインであり、今後の章立て作業の指針を提供する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 草案（Draft） |
| 参照文書 | [reml-ecosystem-analysis.md](reml-ecosystem-analysis.md), [4-0-ecosystem-integration-plan.md](4-0-ecosystem-integration-plan.md) |
| 主要関連章 | 1-1, 1-2, 1-3, 3-6, 3-7, 3-8 |

## 1. CLI 全体像

- `reml` コマンドの責務、サブコマンド一覧（`new`, `add`, `build`, `test`, `fmt`, `check`, `publish`, `registry` など）。
- 0-2 指針の性能・安全性原則との関連付け（構造化エラー、線形性能）。
- `CliDiagnosticEnvelope`（3-6 §9）との統合ポイント。

## 2. マニフェスト統合

- `reml.toml` のロード/検証（3-7 §1）を CLI がどのように利用するか。
- `dsl` セクションと `DslCapabilityProfile` の同期フロー（3-8 §7）。
- プロファイルキャッシュ、`reml manifest sync` 等の運用コマンド草案。

## 3. サブコマンド仕様（アウトライン）

### 3.1 `reml new`
- プロジェクトひな形生成、DSL エントリーポイントの初期配置。
- テンプレート選択（config/template/query 等）と `guides/dsl-gallery.md` の連携。

### 3.2 `reml add`
- 依存解決アルゴリズムの要件（Git/Tarball、将来の中央レジストリ）。
- 競合解決ポリシー、バージョン範囲の記法。

### 3.3 `reml build`
- ビルドステージ（解析→型付け→効果検査→コード生成）。
- `CliPhase`（3-6 §9）を使用した診断フェーズの追跡。
- パフォーマンス・セーフティチェック（0-2 指針）を CLI レベルで担保する方法。

### 3.4 `reml test`
- 標準テストランナー（計画中 `3-10-core-test-support.md`）との接続点。
- 並列・シリアル実行戦略、`effect` 制約の扱い。

### 3.5 `reml fmt` / `reml check`
- フォーマッタ・リンターの差分出力、`--check` モードの規定。
- `Diagnostic` と `AuditEnvelope` の使い分け。

### 3.6 `reml publish`
- 4.2 レジストリ仕様との整合性。
- 署名・検証・再現性（deterministic tarball）の要求事項。

## 4. 出力形式と UX

- ヒューマンリーダブル出力と JSON/NDJSON/LSP モードの仕様詳細。
- `--summary` / `--fail-on-warning` / `--fail-on-performance` 等の制御フラグ。
- カラーリング、ロケール対応、アクセシビリティ指針。

## 5. セキュリティ・監査要件

- コマンドごとの Capability 要求（3-8 の Registry API、SecurityCapability 参照）。
- 監査ログ最小要件（`run_id`, `command`, `phase`）。
- 署名付きマニフェスト/アーティファクトの検証フロー。

## 6. 今後の執筆タスク

- サブコマンドごとの正式仕様策定。
- CLI オプションリファレンス（短形式/長形式、環境変数対応）。
- エラーコード/終了コードテーブルの整備（3-6 §9 と統一）。
- 性能要件テスト（ベンチマーク）と検証手順を付録として追加。

> メモ: 本書はドラフト段階であり、追って詳細仕様と例示を追加する。初稿作成時は 4-0 の進捗サマリを参照し、章間整合性を確認すること。
