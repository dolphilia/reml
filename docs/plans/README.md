# docs/plans 目次

Reml 実装ロードマップや運用計画を集約しています。

## ブートストラップ計画 (`docs/plans/bootstrap-roadmap/`)
- [README.md](bootstrap-roadmap/README.md) — 全体構成と Phase サマリ
- [SUMMARY.md](bootstrap-roadmap/SUMMARY.md) — マイルストーン一覧
- Phase 0 基本方針: [0-1-roadmap-principles.md](bootstrap-roadmap/0-1-roadmap-principles.md), [0-2-roadmap-structure.md](bootstrap-roadmap/0-2-roadmap-structure.md), [0-3-audit-and-metrics.md](bootstrap-roadmap/0-3-audit-and-metrics.md), [0-4-risk-handling.md](bootstrap-roadmap/0-4-risk-handling.md)
- Phase 1〜4 詳細: `bootstrap-roadmap/1-x` 〜 `bootstrap-roadmap/4-x`

## リポジトリ再編計画
- [repository-restructure-plan.md](repository-restructure-plan.md)

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

## Core.Parse 強化計画（ドラフト）(`docs/plans/core-parse-improvement/`)
- [README.md](core-parse-improvement/README.md) — 強化計画の目次と位置づけ（ドラフト）
- [0-0-overview.md](core-parse-improvement/0-0-overview.md) — 背景・目的・成功条件
- [0-1-workstream-tracking.md](core-parse-improvement/0-1-workstream-tracking.md) — 追跡ルールとワークストリーム分割

---
計画書を更新する際は、関連する仕様書やガイドのリンクが最新構成になっているか確認し、`docs-migrations.log` に必要な記録を追加してください。
