# 4.3 Developer Toolchain（ドラフト）

> 目的：IDE/LSP、フォーマッタ、リンター、デバッガー、プロファイラーなどの開発者ツールチェーン仕様を統合し、Reml 言語・標準ライブラリとの結節点を示す。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 草案（Draft） |
| 参照文書 | [guides/lsp-integration.md](guides/lsp-integration.md), [guides/config-cli.md](guides/config-cli.md), [reml-ecosystem-analysis.md](reml-ecosystem-analysis.md) §4.1, §4.2 |
| 関連章 | 3-6, 3-7, 3-8, 4-1 |

## 1. LSP 実装ロードマップ

- サーバー機能（補完、定義ジャンプ、ドキュメント、コードアクション）。
- `DslExportSignature` を活用した DSL 特化ヒント。
- 監査/トレースとの連携（3-6 §8.2）。

## 2. フォーマッタ & リンター

- `reml fmt` / `reml check` に対応するライブラリ API と CLI.
- スタイルガイド、設定ファイル形式（`reml-format.toml` 仮称）。
- 効果タグや DSL メタデータに基づく静的解析規則。

## 3. テストランナー

- `Core.Test`（計画中）および `reml test` の相互作用。
- 並列化戦略とEffect制約。
- スナップショットテストや DSL 専用マクロの扱い。

## 4. デバッガー

- DSL 境界を跨ぐステップ実行、逆方向トレース。
- `TraceSink`（3-8 §7）および `AuditEnvelope` を用いた履歴管理。
- CLI 連携 (`reml debug` コマンド案)。

## 5. プロファイラー

- `benchmark_dsl` との整合性、サンプリング/計測の要件。
- ホットスポット抽出、DSL パイプライン単位の計測。

## 6. AI 支援ツール

- `reml ai-*` コマンド（4-0, reml-ecosystem-analysis §4.3）に対応する API。
- LLM 連携時の安全ガードライン（guides/ai-integration.md 参照）。

## 7. 今後の作業

- 既存ガイドの統合（lsp, config-cli, dsl-performance など）。
- 参考実装（VS Code 拡張、Neovim プラグイン、CLI ツール）の執筆計画。
- テストベンチ / CI 戦略の取りまとめ。

> メモ: 本章はツールチェーンの包括的な仕様をまとめるための骨組みであり、個別節の詳細は今後のドラフトで充実させる。
