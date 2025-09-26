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

### 標準ライブラリ仕様

- [3.1 プレリュードと反復制御](3-1-core-prelude-iteration.md)
- [3.2 コレクション](3-2-core-collections.md)
- [3.3 テキストと Unicode サポート](3-3-core-text-unicode.md)
- [3.4 数値演算と時間管理](3-4-core-numeric-time.md)
- [3.5 入出力とパス操作](3-5-core-io-path.md)
- [3.6 診断と監査](3-6-core-diagnostics-audit.md)
- [3.7 設定とデータ管理](3-7-core-config-data.md)
- [3.8 ランタイムと Capability レジストリ](3-8-core-runtime-capability.md)
- [3.9 非同期・FFI・アンセーフ](3-9-core-async-ffi-unsafe.md)
- [3.10 環境機能とプラットフォーム連携](3-10-core-env.md)
- [3.11 システム — システムコールインターフェースとプラットフォームバインディング](3-11-core-system.md)
- [3.12 プロセス — ネイティブプロセスとスレッド制御](3-12-core-process.md)
- [3.13 メモリ — 仮想メモリと共有領域](3-13-core-memory.md)
- [3.14 シグナル — プロセス間シグナルとハンドラ](3-14-core-signal.md)
- [3.15 ハードウェア — CPU とプラットフォーム情報取得](3-15-core-hardware.md)
- [3.16 リアルタイム — スケジューリングと高精度タイマー](3-16-core-realtime.md)

### エコシステム仕様（Chapter 4 ドラフト）

- [4.0 エコシステム取り込み計画](4-0-ecosystem-integration-plan.md)
- [4.1 パッケージマネージャと CLI](4-1-package-manager-cli.md)
- [4.2 レジストリと配布](4-2-registry-distribution.md)
- [4.3 開発ツールチェーン](4-3-developer-toolchain.md)
- [4.4 コミュニティとコンテンツ戦略](4-4-community-content.md)
- [4.5 ロードマップと指標管理](4-5-roadmap-metrics.md)
- [4.6 リスクとガバナンス](4-6-risk-governance.md)

### ガイド

#### 開発ワークフロー & ツールチェーン

- [Reml CLI ワークフローガイド](guides/cli-workflow.md)
- [設定 CLI ワークフロー](guides/config-cli.md)
- [CI/テスト戦略ガイド](guides/ci-strategy.md)
- [LSP / IDE 連携ガイド](guides/lsp-integration.md)
- [ポータビリティガイド](guides/portability.md)
- [クロスコンパイル実務ガイド](guides/cross-compilation.md)

#### DSL / プラグイン

- [DSL プラグイン & Capability ガイド](guides/DSL-plugin.md)
- [プラグイン開発ガイド](guides/plugin-authoring.md)
- [DSLファースト導入ガイド](guides/dsl-first-guide.md)
- [DSL ギャラリー整備ガイド](guides/dsl-gallery.md)
- [Conductor パターン実践ガイド](guides/conductor-pattern.md)
- [DSLパフォーマンスプレイブック](guides/dsl-performance-playbook.md)
- [制約DSL・ポリシー運用ベストプラクティス](guides/constraint-dsl-best-practices.md)

#### エコシステム & コミュニティ

- [AI 統合ガイド](guides/ai-integration.md)
- [パッケージ管理ドラフト](guides/package-management.md)
- [Reml マニフェスト記述ガイド](guides/manifest-authoring.md)
- [コミュニティ運営ハンドブック](guides/community-handbook.md)
- [データモデル・リファレンス](guides/data-model-reference.md)
- [初期設計コンセプト](guides/early-design-concepts.md)

#### ランタイム / システム

- [System Programming Primer for Reml](guides/system-programming-primer.md)
- [ランタイム連携ガイド](guides/runtime-bridges.md)
- [Core.Parse ストリーミング運用メモ](guides/core-parse-streaming.md)
- [Core.Unsafe ポインタAPIドラフト](guides/core-unsafe-ptr-api-draft.md)
- [FFI ハンドブック](guides/reml-ffi-handbook.md)
- [形式文法リファレンス (BNF)](guides/formal-grammar-bnf.md)
- [LLVM 連携ノート](guides/llvm-integration-notes.md)

### 調査・補助ドキュメント

- [Remlエコシステム分析報告書](reml-ecosystem-analysis.md)
- [A. JIT / バックエンド拡張ノート](notes/a-jit.md)
- [標準ライブラリ仕様: 範囲定義メモ（フェーズ1）](notes/core-library-scope.md)
- [標準ライブラリ章 骨子（フェーズ2）](notes/core-library-outline.md)
- [DSLプラグイン提供ロードマップ](notes/dsl-plugin-roadmap.md)
- [クロスコンパイル調査メモ](notes/cross-compilation-spec-intro.md)
- [クロスコンパイル仕様組み込み計画](notes/cross-compilation-spec-update-plan.md)
- [DSL統合TODO](todo-dsl-integration.md)

## ビルド & ターゲット指定例

Reml コンパイラ `remlc` は `RunConfig.extensions["target"]` に整形済みターゲット情報を渡す。クロスビルド時は以下のスニペットを基準として、`@cfg` と標準ライブラリのプラットフォーム抽象（[3-5](3-5-core-io-path.md)、[3-10](3-10-core-env.md)）を同期させる。事前に `reml target list` / `reml toolchain install <profile>` で必要なプロファイルと標準ライブラリを取得し、`guides/cross-compilation.md` を参照して整合性を確認する。

```bash
# Windows 用バイナリを Linux ホストで生成
remlc --target x86_64-pc-windows-msvc src/main.reml

# Apple Silicon 向けビルド
remlc --target aarch64-apple-darwin src/main.reml
```

ターゲット指定に合わせて `RunConfig.extensions["target"]` を初期化することで、`@cfg` の条件分岐や FFI 呼出規約（[3-9](3-9-core-async-ffi-unsafe.md)）が一貫した状態で評価される。CI/CD では `REML_TARGET_PROFILE`, `REML_TARGET_TRIPLE`, `REML_TARGET_CAPABILITIES`, `REML_TARGET_FEATURES`, `REML_STD_VERSION`, `REML_RUNTIME_REVISION` などの環境変数を設定し、`Core.Env.infer_target_from_env()` が期待通りに解決したか `Diagnostic.domain = Target` のメッセージで確認する。


## 編集時のメモ

- 仕様本文・コメントはすべて日本語で記述します（コード例は Reml 構文を使用）。
- セクション間の相互参照は相対リンクで統一し、名称変更時は関連文書も更新します。
- 例や疑似コードを追加する際は、言語仕様に合致することを確認してください。
