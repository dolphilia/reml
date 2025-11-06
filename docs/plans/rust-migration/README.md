# Rust 移植計画ドキュメント集約

このディレクトリは OCaml 実装から Rust 実装へ移行するための計画書と補助資料を管理する。Phase 2-6 で判明した Windows 対応課題を解消し、Phase 3 のセルフホスト移行前に Rust 版コンパイラの足場を整えることを目的とする。

## 位置づけ
- `docs/plans/bootstrap-roadmap/2-6-windows-support-migration-options.md` で Rust 採用を決定した後続計画のハブ
- Phase 2-8 仕様監査や Phase 3 Self-Host 計画と並行して進める移行タスク群の参照点
- `compiler/ocaml/` 資産と Rust 実装（`compiler/rust/` 予定）を比較する際のドキュメント基準

## ファイル構成
- [`overview.md`](overview.md): 移植計画の背景、フェーズ構成、必要ドキュメント一覧
- **P0 ベースライン整備**
  - [`0-0-roadmap.md`](0-0-roadmap.md): P0 の目的、マイルストーン、完了条件
  - [`0-1-baseline-and-diff-assets.md`](0-1-baseline-and-diff-assets.md): OCaml 資産棚卸しと差分ハーネス設計
  - [`0-2-windows-toolchain-audit.md`](0-2-windows-toolchain-audit.md): Windows ツールチェーン監査手順
  - [`appendix/glossary-alignment.md`](appendix/glossary-alignment.md): Rust↔Reml 用語対応表
- **P1 フロントエンド移植**
  - [`1-0-front-end-transition.md`](1-0-front-end-transition.md): パーサ/型推論移植の方針とマイルストーン
  - [`1-1-ast-and-ir-alignment.md`](1-1-ast-and-ir-alignment.md): AST/IR 対応表と検証手順
  - [`1-2-diagnostic-compatibility.md`](1-2-diagnostic-compatibility.md): 診断互換性のチェックリストと dual-write 運用
- **P2 バックエンド統合**
  - [`2-0-llvm-backend-plan.md`](2-0-llvm-backend-plan.md): LLVM バックエンド移植・TargetMachine 整合・IR 検証手順
  - [`2-1-runtime-integration.md`](2-1-runtime-integration.md): ランタイム FFI・Capability Registry・監査ログ統合計画
  - [`2-2-adapter-layer-guidelines.md`](2-2-adapter-layer-guidelines.md): プラットフォーム差分吸収アダプタ層の設計指針
- **統合原則**
  - [`unified-porting-principles.md`](unified-porting-principles.md): 移植全体で共有する設計原則と成功指標

今後のフェーズ（P1〜P4）の詳細計画は `docs/plans/bootstrap-roadmap/0-2-roadmap-structure.md` の命名規則に従い、同ディレクトリに追加する。

## 運用ルール
- 既存仕様との整合を保つため、更新時は `docs/spec/` および関連ガイドの該当箇所を確認する。
- 新規ドキュメントを作成した場合は、本 README に追記し、必要に応じて `docs/plans/README.md` からの導線を整備する。
- ファイルの大規模改訂や名称変更を行った際は `docs-migrations.log` に記録し、レビューコメントを `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に残す。
