# Rust 移植計画概要

Phase 2-6 の Windows 対応停滞を受け、OCaml 実装から Rust 実装へ移行するための工程と成果物を定義する。移植の原則は `unified-porting-principles.md` に統合されており、本概要は同ガイドに沿って必要ドキュメントと実務タスクを整理する。

## 背景と目的
- `docs/plans/bootstrap-roadmap/2-6-windows-support-migration-options.md` にて移植先言語として Rust を選定済み
- Phase 2-8 仕様監査 (`docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md`) を阻害しないよう、Rust 版を並行準備しつつ OCaml 実装との整合を保つ
- Phase 3 のセルフホスト移行を見据え、Rust 実装の計画・リスク・CI 基盤を段階的に整備する

## 必要ドキュメント一覧（統合原則に基づく）

統合ガイドで定義したフェーズ（P0〜P4）に対応する計画書・設計書を以下に整理する。新規ドキュメントは名称のみ先行定義し、作成順序を本一覧で管理する。

| フェーズ | ドキュメント（予定ファイル名） | 目的 | 主要参照元 / 連携先 |
| --- | --- | --- | --- |
| P0 ベースライン整備 | `0-0-roadmap.md` | 全体ロードマップ・マイルストーン定義・依存関係整理 | `docs/plans/bootstrap-roadmap/0-2-roadmap-structure.md`, `docs/plans/bootstrap-roadmap/2-0-phase2-stabilization.md` |
|  | `0-1-baseline-and-diff-assets.md` | 現行 OCaml 資産棚卸し、ゴールデン/ベンチ基準、差分テストハーネス設計 | `compiler/ocaml/`, `reports/diagnostic-format-regression.md`, `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` |
|  | `0-2-windows-toolchain-audit.md` | Windows 向け環境診断・`rustup`/MSVC 設定・自動化手順 | `docs/plans/bootstrap-roadmap/2-6-windows-support.md`, `tooling/toolchains/` |
|  | `appendix/glossary-alignment.md` | 用語・略語整合（Rust 特有用語と仕様用語の対応表） | `docs/spec/0-2-glossary.md`, `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` |
| P1 フロントエンド移植 | `1-0-front-end-transition.md` | パーサ/型推論移植、テスト移行計画、dual-write 戦略 | `compiler/ocaml/src/parser_*`, `docs/spec/1-1-syntax.md`, `docs/spec/1-2-types-Inference.md` |
|  | `1-1-ast-and-ir-alignment.md` | OCaml↔Rust AST/IR 対応表・検証手順 | `compiler/ocaml/docs/parser_design.md`, `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` |
|  | `1-2-diagnostic-compatibility.md` | 診断・監査出力の互換計画、差分検証フロー | `reports/diagnostic-format-regression.md`, `tooling/ci/collect-iterator-audit-metrics.py`, `docs/spec/3-6-core-diagnostics-audit.md` |
| P2 バックエンド統合 | `2-0-llvm-backend-plan.md` | LLVM バックエンド実装、`TargetMachine`/`DataLayout` 整合、MSVC/GNU 対応 | `docs/guides/llvm-integration-notes.md`, `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` |
|  | `2-1-runtime-integration.md` | Rust 実装と既存ランタイムの橋渡し、FFI/ABI 契約、unsafe ポリシー | `docs/spec/3-8-core-runtime-capability.md`, `docs/spec/3-9-core-async-ffi-unsafe.md`, `runtime/native/` |
|  | `2-2-adapter-layer-guidelines.md` | FS/ネット/時刻/乱数などアダプタ層設計とプラットフォーム差分吸収方針 | `language-runtime` 設計メモ, `tooling/` |
| P3 CI/監査統合 | `3-0-ci-and-dual-write-strategy.md` | CI マトリクス、dual-write 運用、差分ハーネス自動化 | `docs/plans/bootstrap-roadmap/IMPLEMENTATION-GUIDE.md`, `.github/workflows/`, `tooling/ci/` |
|  | `3-1-observability-alignment.md` | 監査メトリクス・ログ・トレース連携、`collect-iterator-audit-metrics.py` 対応 | `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`, `reports/audit/dashboard/` |
|  | `3-2-benchmark-baseline.md` | Rust 版ベンチマーク定義、性能比較、許容回帰線 | `compiler/ocaml/benchmarks/`, `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` |
| P4 最適化とハンドオーバー | `4-0-risk-register.md` | 移植固有リスク・緩和策・エスカレーション経路 | `compiler/ocaml/docs/technical-debt.md`, `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` |
|  | `4-1-communication-plan.md` | チーム連携・レビュー体制・Phase 3/4 ハンドオーバー方針 | `docs/plans/bootstrap-roadmap/2-7-to-2-8-handover.md`, `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` |
|  | `4-2-documentation-sync.md` | 仕様・ガイド・ノート更新、脚注整合、`docs-migrations.log` 管理 | `docs/spec/`, `docs/guides/`, `docs/notes/` |


## 次のステップ
1. `unified-porting-principles.md` を参照して移植方針を確定し、その方針を `0-0-roadmap.md` へ反映する
2. Rust 実装の着手対象（フロントエンド、バックエンド、CI）の優先順位を `docs/plans/bootstrap-roadmap/2-6-windows-support-migration-options.md` の評価軸に基づいて確定する
3. 各計画書で使用する測定項目・マイルストーンを `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` と同期させ、Phase 3 へのハンドオーバー方針を `4-1-communication-plan.md` に追記する
4. P4 最適化とハンドオーバー向けのドキュメント（`4-0-risk-register.md` / `4-1-communication-plan.md` / `4-2-documentation-sync.md`）を参照し、最終調整タスクのリスク・連携・文書整合を明文化する

## 関連する既存タスクとの依存
- Phase 2-6 の未完了タスクは Rust 移植計画へ移管し、`docs/plans/bootstrap-roadmap/2-6-windows-support.md` の更新時に脚注で参照する。移管後は `unified-porting-principles.md` のチェックリストに沿って進捗を評価する
- Phase 2-8 の仕様監査では Rust 計画書の差分検討結果を確認対象とし、仕様更新後は即時に `glossary-alignment.md` を更新する
- 技術的負債 (`compiler/ocaml/docs/technical-debt.md`) に記載された Windows 関連項目の解消状況を追跡し、Rust 移植のリスク評価に反映する
