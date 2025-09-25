# Reml 仕様書

Reml (Readable & Expressive Meta Language) は、パーサーコンビネーターと静的保証を重視した言語設計プロジェクトです。このリポジトリには、言語仕様・標準パーサーAPI・標準ライブラリ・運用ガイドを日本語でまとめています。

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

### 標準ライブラリ仕様（Chapter 3 正式仕様）

- [3.1 Core Prelude & Iteration](3-1-core-prelude-iteration.md)
- [3.2 Core Collections](3-2-core-collections.md)
- [3.3 Core Text & Unicode](3-3-core-text-unicode.md)
- [3.4 Core Numeric & Time](3-4-core-numeric-time.md)
- [3.5 Core IO & Path](3-5-core-io-path.md)
- [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md)
- [3.7 Core Config & Data](3-7-core-config-data.md)
- [3.8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md)
- [3.9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md)
- [3.10 Core Env & Platform Bridge](3-10-core-env.md)

### ガイド

- [LSP / IDE 連携ガイド](guides/lsp-integration.md)
- [設定 CLI ワークフロー](guides/config-cli.md)
- [DSL プラグイン & Capability ガイド](guides/DSL-plugin.md)
- [DSLファースト導入ガイド](guides/dsl-first-guide.md)
- [Conductor パターン実践ガイド](guides/conductor-pattern.md)
- [DSLパフォーマンスプレイブック](guides/dsl-performance-playbook.md)
- [ランタイム連携ガイド](guides/runtime-bridges.md)
- [パッケージ管理ドラフト](guides/package-management.md)
- [制約DSL・ポリシー運用ベストプラクティス](guides/constraint-dsl-best-practices.md)
- [Core.Parse ストリーミング運用メモ](guides/core-parse-streaming.md)
- [Core.Unsafe ポインタAPIドラフト](guides/core-unsafe-ptr-api-draft.md)
- [データモデル・リファレンス](guides/data-model-reference.md)
- [FFI ハンドブック](guides/reml-ffi-handbook.md)
- [形式文法リファレンス (BNF)](guides/formal-grammar-bnf.md)
- [LLVM 連携ノート](guides/llvm-integration-notes.md)
- [初期設計コンセプト](guides/early-design-concepts.md)

### 補助ドキュメント

- 現在整理中です。必要に応じて付録や各種ガイドを参照してください。
- [標準ライブラリ仕様: 範囲定義メモ（フェーズ1）](notes/core-library-scope.md)
- [標準ライブラリ章 骨子（フェーズ2）](notes/core-library-outline.md)
- [DSLプラグイン提供ロードマップ](notes/dsl-plugin-roadmap.md)
- [DSL統合TODO](todo-dsl-integration.md)

## ビルド & ターゲット指定例

Reml コンパイラ `remlc` は `RunConfig.extensions["target"]` に整形済みターゲット情報を渡す。クロスビルド時は以下のスニペットを基準として、`@cfg` と標準ライブラリのプラットフォーム抽象（[3-5](3-5-core-io-path.md)、[3-10](3-10-core-env.md)）を同期させる。

```bash
# Windows 用バイナリを Linux ホストで生成
remlc --target x86_64-pc-windows-msvc src/main.reml

# Apple Silicon 向けビルド
remlc --target aarch64-apple-darwin src/main.reml
```

ターゲット指定に合わせて `RunConfig.extensions["target"]` を初期化することで、`@cfg` の条件分岐や FFI 呼出規約（[3-9](3-9-core-async-ffi-unsafe.md)）が一貫した状態で評価される。CI/CD では `REML_TARGET` と `REML_FEATURES` 環境変数を設定し、`Core.Env.infer_target_from_env()` が期待通りに解決したか診断メッセージで確認する。


## 編集時のメモ

- 仕様本文・コメントはすべて日本語で記述します（コード例は Reml 構文を使用）。
- セクション間の相互参照は相対リンクで統一し、名称変更時は関連文書も更新します。
- 例や疑似コードを追加する際は、言語仕様に合致することを確認してください。
