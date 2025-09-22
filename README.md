# Reml 仕様書

Reml (Readable & Expressive Meta Language) は、パーサーコンビネーターと静的保証を重視した言語設計プロジェクトです。このリポジトリには、言語仕様・標準パーサーAPI・運用ガイドを日本語でまとめています。

## リポジトリ構成

### はじめに

- [概要](0-1-overview.md)
- [プロジェクトの目的と指針](0-2-project-purpose.md)

### 言語コア仕様

- [1.1 構文仕様](1-1-syntax.md)
- [1.2 型システムと推論](1-2-types-Inference.md)
- [1.3 効果システムと安全性](1-3-effects-safety.md)
- [1.4 Unicode 文字モデル](1-4-test-unicode-model.md)

### 標準パーサーAPI仕様

- [2.1 パーサ型と入力モデル](2-1-parser-type.md)
- [2.2 コアコンビネーター](2-2-core-combinator.md)
- [2.3 字句レイヤユーティリティ](2-3-lexer.md)
- [2.4 演算子優先度ビルダー](2-4-op-builder.md)
- [2.5 エラー設計](2-5-error.md)
- [2.6 実行戦略](2-6-execution-strategy.md)
- [2.7 設定スキーマ API](2-7-config.md)
- [2.8 データモデリング API](2-8-data.md)
- [2.9 実行時基盤ドラフト](2-9-runtime.md)

### 標準ライブラリ仕様（Chapter 4 準備中）

- [4.0 標準ライブラリ仕様: 範囲定義メモ（フェーズ1）](4-0-standard-library-scope.md)
- [4.1 標準ライブラリ章 骨子（フェーズ2）](4-1-standard-library-outline.md)
- 4.2 Core Prelude & Iteration（ドラフト予定）
- 4.3 Core Collections（ドラフト予定）
- 4.4 Core Text & Unicode（ドラフト予定）
- 4.5 Core Numeric & Time（ドラフト予定）
- 4.6 Core IO & Path（ドラフト予定）
- 4.7 Core Diagnostics & Audit（ドラフト予定）
- 4.8 Core Config & Data（ドラフト予定）
- 4.9 Core Runtime & Capability Registry（ドラフト予定）
- 4.10 Core Async / FFI / Unsafe（将来拡張メモ予定）

### ガイド

- [LSP / IDE 連携ガイド](guides/lsp-integration.md)
- [設定 CLI ワークフロー](guides/config-cli.md)
- [DSL プラグイン & Capability ガイド](guides/DSL-plugin.md)
- [ランタイム連携ガイド](guides/runtime-bridges.md)
- [制約DSL・ポリシー運用ベストプラクティス](guides/constraint-dsl-best-practices.md)
- [Core.Parse ストリーミング運用メモ](guides/core-parse-streaming.md)
- [Core.Unsafe ポインタAPIドラフト](guides/core-unsafe-ptr-api-draft.md)
- [データモデル・リファレンス](guides/data-model-reference.md)
- [FFI ハンドブック](guides/reml-ffi-handbook.md)

### 付録

- [BNF 文法仕様](3-1-bnf.md)
- [LLVM 連携ノート](a-jit.md)
- [初期設計コンセプト](b-first-idea.md)

### 補助ドキュメント

- 現在整理中です。必要に応じて付録や各種ガイドを参照してください。
- [標準ライブラリ仕様: 範囲定義メモ（フェーズ1）](4-0-standard-library-scope.md)

## 編集時のメモ

- 仕様本文・コメントはすべて日本語で記述します（コード例は Reml 構文を使用）。
- セクション間の相互参照は相対リンクで統一し、名称変更時は関連文書も更新します。
- 例や疑似コードを追加する際は、言語仕様に合致することを確認してください。
