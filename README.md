# Reml 仕様書

Reml (Readable & Expressive Meta Language) は、パーサーコンビネーターと静的保証を重視した言語設計プロジェクトです。このリポジトリには、言語仕様・標準パーサーAPI・標準ライブラリ・運用ガイドを日本語でまとめています。

## リポジトリ構成

### はじめに

- [概要](0-0-overview.md)
- [プロジェクトの目的と指針](0-1-project-purpose.md)
- [用語集](0-2-glossary.md)
- [コードスタイルガイド](0-3-code-style-guide.md)

### 言語コア仕様

- [1.0 言語コア仕様 概要](1-0-language-core-overview.md)
- [1.1 構文仕様](1-1-syntax.md)
- [1.2 型システムと推論](1-2-types-Inference.md)
- [1.3 効果システムと安全性](1-3-effects-safety.md)
- [1.4 Unicode 文字モデル](1-4-test-unicode-model.md)
- [1.5 形式文法（BNF）](1-5-formal-grammar-bnf.md)

### 標準パーサーAPI仕様

- [2.0 標準パーサーAPI 概要](2-0-parser-api-overview.md)
- [2.1 パーサ型と入力モデル](2-1-parser-type.md)
- [2.2 コアコンビネーター](2-2-core-combinator.md)
- [2.3 字句レイヤユーティリティ](2-3-lexer.md)
- [2.4 演算子優先度ビルダー](2-4-op-builder.md)
- [2.5 エラー設計](2-5-error.md)
- [2.6 実行戦略](2-6-execution-strategy.md)
- [2.7 ストリーミング実行](2-7-core-parse-streaming.md)：`DemandHint`/`FlowController` によるインクリメンタル実行契約を正式化

### 標準ライブラリ仕様

- [3.0 標準ライブラリ仕様 概要](3-0-core-library-overview.md)
- [3.1 プレリュードと反復制御](3-1-core-prelude-iteration.md)
- [3.2 コレクション](3-2-core-collections.md)
- [3.3 テキストと Unicode サポート](3-3-core-text-unicode.md)
- [3.4 数値演算と時間管理](3-4-core-numeric-time.md)
- [3.5 入出力とパス操作](3-5-core-io-path.md)
- [3.6 診断と監査](3-6-core-diagnostics-audit.md)
- [3.7 設定とデータ管理](3-7-core-config-data.md)：QualityReport スキーマと監査フローを仕様化
- [3.8 ランタイムと Capability レジストリ](3-8-core-runtime-capability.md)：Runtime Bridge 契約と Stage 管理を統合
- [3.9 非同期・FFI・アンセーフ](3-9-core-async-ffi-unsafe.md)：`Core.Ffi`/`Core.Unsafe.Ptr` API を正式仕様として収録
- [3.10 環境機能とプラットフォーム連携](3-10-core-env.md)

### 公式プラグイン仕様（Chapter 4 ドラフト）

- [4.0 公式プラグイン仕様 概要](4-0-official-plugins-overview.md)
- [4.1 システムコール & プラットフォームバインディング](4-1-system-plugin.md)
- [4.2 プロセスとスレッド制御](4-2-process-plugin.md)
- [4.3 仮想メモリと共有領域](4-3-memory-plugin.md)
- [4.4 プロセス間シグナル](4-4-signal-plugin.md)
- [4.5 ハードウェア情報取得](4-5-hardware-plugin.md)
- [4.6 スケジューリングと高精度タイマー](4-6-realtime-plugin.md)
- [4.7 Core.Parse.Plugin と DSL 拡張契約](4-7-core-parse-plugin.md)：DSL プラグイン登録と署名検証の公式契約

### エコシステム仕様（Chapter 5 ドラフト）

- [5.0 エコシステム仕様 概要](5-0-ecosystem-overview.md)
- [5.1 パッケージマネージャと CLI](5-1-package-manager-cli.md)
- [5.2 レジストリと配布](5-2-registry-distribution.md)
- [5.3 開発ツールチェーン](5-3-developer-toolchain.md)
- [5.4 コミュニティとコンテンツ戦略](5-4-community-content.md)
- [5.5 ロードマップと指標管理](5-5-roadmap-metrics.md)
- [5.6 リスクとガバナンス](5-6-risk-governance.md)

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
- [コレクション収集戦略ガイド](guides/collection-pipeline-guide.md)

#### エコシステム & コミュニティ

- [AI 統合ガイド](guides/ai-integration.md)
- [パッケージ管理ドラフト](guides/package-management.md)
- [Reml マニフェスト記述ガイド](guides/manifest-authoring.md)
- [コミュニティ運営ハンドブック](guides/community-handbook.md)
- [データモデル運用ガイド](guides/data-model-reference.md)（仕様 3.7 の適用手順）
- [初期設計コンセプト](guides/early-design-concepts.md)

#### ランタイム / システム

- [System Programming Primer for Reml](guides/system-programming-primer.md)
- [ランタイム連携ガイド](guides/runtime-bridges.md)（仕様 3.8 §10 の運用補足）
- [Core.Parse ストリーミング運用ガイド](guides/core-parse-streaming.md)（仕様 2.7 の活用例）
- [Core.Unsafe.Ptr 運用ガイド](guides/core-unsafe-ptr-api-draft.md)（仕様 3.9 §3 の補足）
- [FFI ハンドブック](guides/reml-ffi-handbook.md)（仕様 3.9 §2 の補足）
- [LLVM 連携ノート](guides/llvm-integration-notes.md)

### 調査・補助ドキュメント

- [A. JIT / バックエンド拡張ノート](notes/a-jit.md)
- [標準ライブラリ仕様: 範囲定義メモ（フェーズ1）](notes/core-library-scope.md)
- [標準ライブラリ章 骨子（フェーズ2）](notes/core-library-outline.md)
- [DSLプラグイン提供ロードマップ](notes/dsl-plugin-roadmap.md)
- [クロスコンパイル調査メモ](notes/cross-compilation-spec-intro.md)
- [クロスコンパイル仕様組み込み計画](notes/cross-compilation-spec-update-plan.md)
- [関数型言語の辛さ調査メモ](notes/fp-language-pain-points.md)

### ブートストラップ計画書

- [Reml ブートストラップ計画 統合マップ](plans/bootstrap-roadmap/README.md)：Phase構成と依存関係、測定指標の集約
- [Reml ブートストラップ計画 エグゼクティブサマリ](plans/bootstrap-roadmap/SUMMARY.md)：期間・成果物・ターゲットの要約
- [Reml ブートストラップ計画 実装ガイド](plans/bootstrap-roadmap/IMPLEMENTATION-GUIDE.md)：進行手順とレビューフロー
- [0.1 ブートストラップ計画の基本原則](plans/bootstrap-roadmap/0-1-roadmap-principles.md)
- [0.2 ブートストラップ計画の構成](plans/bootstrap-roadmap/0-2-roadmap-structure.md)
- [1.0 Phase 1 — Bootstrap Implementation (OCaml)](plans/bootstrap-roadmap/1-0-phase1-bootstrap.md)
  - [1.1 Parser 実装詳細計画](plans/bootstrap-roadmap/1-1-parser-implementation.md)
  - [1.2 Typer 実装詳細計画](plans/bootstrap-roadmap/1-2-typer-implementation.md)
  - [1.3 Core IR と最小最適化計画](plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md)
  - [1.4 LLVM IR 生成とターゲット設定計画](plans/bootstrap-roadmap/1-4-llvm-targeting.md)
  - [1.5 ランタイム連携計画](plans/bootstrap-roadmap/1-5-runtime-integration.md)
  - [1.6 開発者体験整備計画](plans/bootstrap-roadmap/1-6-developer-experience.md)
  - [1.7 x86_64 Linux 検証インフラ計画](plans/bootstrap-roadmap/1-7-linux-validation-infra.md)
- [2.0 Phase 2 — 言語仕様の安定化](plans/bootstrap-roadmap/2-0-phase2-stabilization.md)
  - [2.1 型クラス実装戦略評価計画](plans/bootstrap-roadmap/2-1-typeclass-strategy.md)
  - [2.2 効果システム統合計画](plans/bootstrap-roadmap/2-2-effect-system-integration.md)
  - [2.3 FFI 契約拡張計画](plans/bootstrap-roadmap/2-3-ffi-contract-extension.md)
  - [2.4 診断・監査パイプライン強化計画](plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md)
  - [2.5 仕様差分補正計画](plans/bootstrap-roadmap/2-5-spec-drift-remediation.md)
  - [2.6 Windows x64 (MSVC ABI) 対応計画](plans/bootstrap-roadmap/2-6-windows-support.md)
- [3.0 Phase 3 — Self-Host Transition](plans/bootstrap-roadmap/3-0-phase3-self-host.md)
  - [3.1 Reml Parser 再実装計画](plans/bootstrap-roadmap/3-1-reml-parser-port.md)
  - [3.2 Reml TypeChecker 再実装計画](plans/bootstrap-roadmap/3-2-reml-typechecker-port.md)
  - [3.3 クロスコンパイル機能実装計画](plans/bootstrap-roadmap/3-3-cross-compilation.md)
  - [3.4 中間 IR と CodeGen 再実装計画](plans/bootstrap-roadmap/3-4-intermediate-ir-and-codegen.md)
  - [3.5 ランタイムと Capability 統合計画](plans/bootstrap-roadmap/3-5-runtime-capability-integration.md)
  - [3.6 メモリ管理戦略評価計画](plans/bootstrap-roadmap/3-6-memory-management-evaluation.md)
  - [3.7 セルフホストビルドパイプライン計画](plans/bootstrap-roadmap/3-7-self-host-build-pipeline.md)
  - [3.8 ドキュメント・仕様フィードバック計画](plans/bootstrap-roadmap/3-8-doc-spec-feedback.md)
- [4.0 Phase 4 — 移行完了と運用体制](plans/bootstrap-roadmap/4-0-phase4-migration.md)
  - [4.1 マルチターゲット互換性検証計画](plans/bootstrap-roadmap/4-1-multitarget-compatibility-verification.md)
  - [4.2 マルチターゲットリリースパイプライン計画](plans/bootstrap-roadmap/4-2-multitarget-release-pipeline.md)
  - [4.3 ドキュメント更新計画](plans/bootstrap-roadmap/4-3-documentation-updates.md)
  - [4.4 エコシステム移行計画](plans/bootstrap-roadmap/4-4-ecosystem-migration.md)
  - [4.5 後方互換チェックリスト実施計画](plans/bootstrap-roadmap/4-5-backward-compat-checklist.md)
  - [4.6 サポートポリシー策定計画](plans/bootstrap-roadmap/4-6-support-policy.md)
- [0.3 測定・監査・レビュー記録](plans/bootstrap-roadmap/0-3-audit-and-metrics.md)
- [0.4 リスク管理とフォローアップ](plans/bootstrap-roadmap/0-4-risk-handling.md)

### サンプル実装

- [代数的効果サンプルセット](samples/algebraic-effects/README.md)
- [言語実装比較ミニ言語集](samples/language-impl-comparison/README.md)

## 最近の仕様統合（ガイド→章）
- `guides/core-parse-streaming.md` の API 定義を [2-7-core-parse-streaming.md](2-7-core-parse-streaming.md) へ統合し、ガイドは運用事例に特化。
- `guides/core-unsafe-ptr-api-draft.md` と `guides/reml-ffi-handbook.md` の型/契約を [3-9-core-async-ffi-unsafe.md](3-9-core-async-ffi-unsafe.md) に吸収。
- `guides/data-model-reference.md` の QualityReport 仕様を [3-7-core-config-data.md](3-7-core-config-data.md) §4 に組み込み。
- `guides/runtime-bridges.md` の Stage/ホットリロード契約を [3-8-core-runtime-capability.md](3-8-core-runtime-capability.md) §10 に昇格。
- `guides/DSL-plugin.md` のプラグイン契約を [4-7-core-parse-plugin.md](4-7-core-parse-plugin.md) に集約し、ガイドはベストプラクティスを補足。

## 編集時のメモ

- 仕様本文・コメントはすべて日本語で記述します（コード例は Reml 構文を使用）。
- セクション間の相互参照は相対リンクで統一し、名称変更時は関連文書も更新します。
- 例や疑似コードを追加する際は、言語仕様に合致することを確認してください。
