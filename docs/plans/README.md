# docs/plans 目次

Reml 実装ロードマップや運用計画を集約しています。

## ブートストラップ計画 (`docs/plans/bootstrap-roadmap/`)
- [README.md](bootstrap-roadmap/README.md) — 全体構成と Phase サマリ
- [SUMMARY.md](bootstrap-roadmap/SUMMARY.md) — マイルストーン一覧
- Phase 0 基本方針: [0-1-roadmap-principles.md](bootstrap-roadmap/0-1-roadmap-principles.md), [0-2-roadmap-structure.md](bootstrap-roadmap/0-2-roadmap-structure.md), [0-3-audit-and-metrics.md](bootstrap-roadmap/0-3-audit-and-metrics.md), [0-4-risk-handling.md](bootstrap-roadmap/0-4-risk-handling.md)
- Phase 1〜4 詳細: `bootstrap-roadmap/1-x` 〜 `bootstrap-roadmap/4-x`

## リポジトリ再編計画
- [repository-restructure-plan.md](repository-restructure-plan.md)

## Rust ツールチェーン更新計画 (`docs/plans/rust-toolchain-upgrade/`)
- [README.md](rust-toolchain-upgrade/README.md) — 目的・手順・復帰計画の一覧
- [0-0-overview.md](rust-toolchain-upgrade/0-0-overview.md) — 背景・目的・対象範囲
- [0-1-upgrade-plan.md](rust-toolchain-upgrade/0-1-upgrade-plan.md) — 更新作業のフェーズ計画
- [0-2-validation-plan.md](rust-toolchain-upgrade/0-2-validation-plan.md) — ビルド/検証手順と記録方針
- [0-3-rollback-plan.md](rust-toolchain-upgrade/0-3-rollback-plan.md) — ロールバック指針
- [0-4-return-to-task.md](rust-toolchain-upgrade/0-4-return-to-task.md) — docs-examples-audit への復帰手順

## ドキュメント Reml コード検証計画 (`docs/plans/docs-examples-audit/`)
- [README.md](docs-examples-audit/README.md) — 計画の概要と参照先
- [0-0-overview.md](docs-examples-audit/0-0-overview.md) — 背景・目的・成功条件
- [0-1-workflow.md](docs-examples-audit/0-1-workflow.md) — 抽出ルールと運用フロー
- [1-0-validation-plan.md](docs-examples-audit/1-0-validation-plan.md) — 検証手順とログ規約

## Rust 移植計画 (`docs/plans/rust-migration/`)
- [README.md](rust-migration/README.md) — Rust 版コンパイラ移行タスクのドキュメント集約
- [overview.md](rust-migration/overview.md) — 移植計画の背景と必要ドキュメント一覧
- **P0 ベースライン整備**
  - [0-0-roadmap.md](rust-migration/0-0-roadmap.md) — P0 マイルストーンと完了条件
  - [0-1-baseline-and-diff-assets.md](rust-migration/0-1-baseline-and-diff-assets.md) — OCaml 資産棚卸しと差分ハーネス設計
  - [0-2-windows-toolchain-audit.md](rust-migration/0-2-windows-toolchain-audit.md) — Windows ツールチェーン監査手順
  - [appendix/glossary-alignment.md](rust-migration/appendix/glossary-alignment.md) — Rust↔Reml 用語整合表
- [unified-porting-principles.md](rust-migration/unified-porting-principles.md) — 移植原則・OCaml→Rust 鉄則・プロジェクト指針を統合したガイド
- **P2 バックエンド統合**
  - [2-0-llvm-backend-plan.md](rust-migration/2-0-llvm-backend-plan.md) — LLVM バックエンド統合とターゲット別検証計画
  - [2-1-runtime-integration.md](rust-migration/2-1-runtime-integration.md) — ランタイム FFI・Capability 連携と監査手順
  - [2-2-adapter-layer-guidelines.md](rust-migration/2-2-adapter-layer-guidelines.md) — プラットフォーム差分アダプタ層の設計ガイドライン

## パターンマッチ強化計画 (`docs/plans/pattern-matching-improvement/`)
- [README.md](pattern-matching-improvement/README.md) — 強化計画の目次と位置づけ
- [0-0-overview.md](pattern-matching-improvement/0-0-overview.md) — 背景・目的・進行フェーズの骨子
- [1-0-active-patterns-plan.md](pattern-matching-improvement/1-0-active-patterns-plan.md) — Active Patterns 導入計画
- [1-1-pattern-surface-plan.md](pattern-matching-improvement/1-1-pattern-surface-plan.md) — Or/Slice/Range/Binding/Regex 拡張計画

## Core.Parse 強化計画 (`docs/plans/core-parse-improvement/`)
- [README.md](core-parse-improvement/README.md) — 強化計画の目次と位置づけ
- [0-0-overview.md](core-parse-improvement/0-0-overview.md) — 背景・目的・成功条件
- [0-1-workstream-tracking.md](core-parse-improvement/0-1-workstream-tracking.md) — 追跡ルールとワークストリーム分割

## 標準ライブラリ改善計画 (`docs/plans/stdlib-improvement/`)
- [README.md](stdlib-improvement/README.md) — 計画の目次と位置づけ
- [0-0-overview.md](stdlib-improvement/0-0-overview.md) — 背景・目的・成功条件
- [0-1-workstream-tracking.md](stdlib-improvement/0-1-workstream-tracking.md) — 追跡ルールとワークストリーム分割
- [1-0-core-test-plan.md](stdlib-improvement/1-0-core-test-plan.md) — `Core.Test` 計画
- [1-1-core-cli-plan.md](stdlib-improvement/1-1-core-cli-plan.md) — `Core.Cli` 計画
- [1-2-core-text-pretty-plan.md](stdlib-improvement/1-2-core-text-pretty-plan.md) — `Core.Text.Pretty` 計画
- [1-3-core-doc-plan.md](stdlib-improvement/1-3-core-doc-plan.md) — `Core.Doc` 計画
- [1-4-core-lsp-plan.md](stdlib-improvement/1-4-core-lsp-plan.md) — `Core.Lsp` 計画
- [2-0-bootstrap-integration.md](stdlib-improvement/2-0-bootstrap-integration.md) — ブートストラップ統合

## FFI 強化計画 (`docs/plans/ffi-improvement/`)
- [README.md](ffi-improvement/README.md) — FFI 強化計画の目次と位置づけ
- [0-0-overview.md](ffi-improvement/0-0-overview.md) — 背景・目的・段階整理
- [0-1-workstream-tracking.md](ffi-improvement/0-1-workstream-tracking.md) — ワークストリーム管理（暫定）
- [1-0-bindgen-plan.md](ffi-improvement/1-0-bindgen-plan.md) — `reml-bindgen` 設計・仕様化
- [1-1-ffi-dsl-plan.md](ffi-improvement/1-1-ffi-dsl-plan.md) — `Core.Ffi.Dsl` 設計・仕様化
- [1-2-build-integration-plan.md](ffi-improvement/1-2-build-integration-plan.md) — `reml build` 連携の設計・仕様化
- [1-3-wasm-component-model-plan.md](ffi-improvement/1-3-wasm-component-model-plan.md) — WASM Component Model 調査・方針

## Typeck 改善計画 (`docs/plans/typeck-improvement/`)
- [README.md](typeck-improvement/README.md) — 型検査改善計画の目次
- [1-0-type-decl-realization-plan.md](typeck-improvement/1-0-type-decl-realization-plan.md) — 型宣言実体化計画

---
計画書を更新する際は、関連する仕様書やガイドのリンクが最新構成になっているか確認し、`docs-migrations.log` に必要な記録を追加してください。
