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
  - [`appendix/parser-ocaml-inventory.md`](appendix/parser-ocaml-inventory.md): OCaml パーサ資産棚卸し（W1 Lexer/Parser スケルトン移植）
  - [`appendix/frontend-crate-evaluation.md`](appendix/frontend-crate-evaluation.md): Lexing/Parsing/診断向けクレート評価メモ（W1 雛形）
- **P1 フロントエンド移植**
  - [`1-0-front-end-transition.md`](1-0-front-end-transition.md): パーサ/型推論移植の方針とマイルストーン
  - [`1-1-ast-and-ir-alignment.md`](1-1-ast-and-ir-alignment.md): AST/IR 対応表と検証手順
  - [`1-2-diagnostic-compatibility.md`](1-2-diagnostic-compatibility.md): 診断互換性のチェックリストと dual-write 運用
  - [`p1-front-end-checklists.csv`](p1-front-end-checklists.csv): P1 チェックリストのスプレッドシート転記用データ
  - [`1-3-dual-write-runbook.md`](1-3-dual-write-runbook.md): dual-write 実行コマンド・切り分け・レポート命名規則
- **P2 バックエンド統合**
  - [`2-0-llvm-backend-plan.md`](2-0-llvm-backend-plan.md): LLVM バックエンド移植・TargetMachine 整合・IR 検証手順
  - [`2-1-runtime-integration.md`](2-1-runtime-integration.md): ランタイム FFI・Capability Registry・監査ログ統合計画
  - [`2-2-adapter-layer-guidelines.md`](2-2-adapter-layer-guidelines.md): プラットフォーム差分吸収アダプタ層の設計指針
- **P3 CI/監査統合**
  - [`3-0-ci-and-dual-write-strategy.md`](3-0-ci-and-dual-write-strategy.md): CI マトリクス拡張と dual-write 運用戦略
  - [`3-1-observability-alignment.md`](3-1-observability-alignment.md): 監査メトリクス・ログ・ダッシュボード整合計画
  - [`3-2-benchmark-baseline.md`](3-2-benchmark-baseline.md): ベンチマーク指標と性能ベースライン構築計画
- **P4 最適化とハンドオーバー**
  - [`4-0-risk-register.md`](4-0-risk-register.md): 最終最適化期間のリスク台帳とエスカレーション基準
  - [`4-1-communication-plan.md`](4-1-communication-plan.md): チーム連携・レビュー頻度・ハンドオーバー準備計画
  - [`4-2-documentation-sync.md`](4-2-documentation-sync.md): 仕様・ガイド・ノートの同期手順とチェックリスト
- **統合原則**
  - [`unified-porting-principles.md`](unified-porting-principles.md): 移植全体で共有する設計原則と成功指標

今後のフェーズ（P1〜P4）の詳細計画は `docs/plans/bootstrap-roadmap/0-2-roadmap-structure.md` の命名規則に従い、同ディレクトリに追加する。

## 運用ルール
- 既存仕様との整合を保つため、更新時は `docs/spec/` および関連ガイドの該当箇所を確認する。
- 新規ドキュメントを作成した場合は、本 README に追記し、必要に応じて `docs/plans/README.md` からの導線を整備する。
- ファイルの大規模改訂や名称変更を行った際は `docs-migrations.log` に記録し、レビューコメントを `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に残す。

### P1 dual-write 実行メモ（型推論）

- W3 以降の Typecheck dual-write では、CLI に `--emit typeck-debug <dir>` を指定して `Type_inference_effect` / `Constraint_solver` の JSON ログを出力する（Rust: `remlc --frontend rust --emit typed-ast --emit constraints --emit typeck-debug <dir>`、OCaml: `remlc --frontend ocaml --emit-constraints-json <path> --emit-typeck-debug <dir>`）。  
- `scripts/poc_dualwrite_compare.sh --mode typeck --dualwrite-root reports/dual-write/front-end/w3-type-inference --run-id <label>` を利用すると、`typed-ast.{ocaml,rust}.json`／`constraints.{ocaml,rust}.json`／`impl-registry.{ocaml,rust}.json`／`effects-metrics.{ocaml,rust}.json`／`typeck-debug.{ocaml,rust}.json` が一括生成される。`--dualwrite-root` を必ず指定し、ランごとの成果物を `reports/dual-write/front-end/w3-type-inference/` に集約する。  
- 成果物のディレクトリ構造・メトリクス検証手順は [`reports/dual-write/front-end/w3-type-inference/README.md`](../../reports/dual-write/front-end/w3-type-inference/README.md) を参照。`--typeck-debug` のフィールド仕様や `collect-iterator-audit-metrics.py --section effects` の必須キーも同 README と `appendix/w3-typeck-dualwrite-plan.md` に記載されている。
