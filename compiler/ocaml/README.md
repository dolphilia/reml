# compiler/ocaml ワークスペース

このディレクトリは Reml ブートストラップ計画 Phase 1〜3 の OCaml 実装をまとめる作業領域です。最新の指針は `docs/plans/bootstrap-roadmap/` を参照してください。

## フェーズ状況
- Phase 1 — Parser & Frontend（完了: 2025-10-06）: [docs/phase1-completion-report.md](docs/phase1-completion-report.md)
- Phase 2 — Typer MVP（完了: 2025-10-07）: [docs/phase2-completion-report.md](docs/phase2-completion-report.md)
- Phase 3 — Core IR & LLVM 生成（進行中: Week 10/16）: [docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md](../../docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md), [docs/phase3-handover.md](docs/phase3-handover.md)

## Phase 3 ダッシュボード
**更新日**: 2025-10-07（Week 10/16 後半）

- 完了:
  - `src/core_ir/ir.ml` で Core IR 型を整備（384行）
  - `src/core_ir/ir_printer.ml` を追加
  - `src/core_ir/desugar.ml` で糖衣削除パスの主要ケースと let 再束縛正規化を実装済み（638行）
  - `src/core_ir/cfg.ml` でベーシックブロック生成と CFG 構築アルゴリズムを実装（430行）
    - 制御フロー分岐点の検出（if式・match式対応）
    - ラベル自動生成とブロック接続
    - CFG整形性検証（到達不能ブロック検出・無限ループ検出・ラベル重複チェック）
  - `tests/test_core_ir.ml`（143行）と`tests/test_cfg.ml`（249行）を含む 118 件のテストが成功
  - **NEW**: `src/core_ir/const_fold.ml` 定数畳み込みパスの骨格を実装（460行、未完成）
    - 算術演算・比較演算・論理演算の畳み込み
    - 定数伝播と不動点反復
    - 条件分岐の静的評価
    - **状態**: ビルドエラー（Ast.literal型の不一致）
- 進行中: 定数畳み込みパスの修正（リテラル変換関数が必要）
- 次に着手: リテラル変換関数の実装、定数畳み込みテストの修正、死コード削除パスの実装
- ブロッカー: `Ast.literal` 型と評価済み値（int64/float）の変換処理
- 既知の課題:
  - `Ast.literal` は文字列+基数で保持されるため、`int64`/`float` への変換関数が必要
  - テストコードの `span` 構造が間違っている（`Ast.dummy_span` を使用すべき）
- 記録ルール: 週次で本節を更新し、詳細な議事録は `docs/phase3-handover.md` と `docs/technical-debt.md`、測定値は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に記録
- 詳細進捗: [docs/phase3-week10-progress.md](docs/phase3-week10-progress.md)

### 週次更新テンプレート
```text
Week NN（YYYY-MM-DD 更新）
- 完了: ...
- 進行中: ...
- 次に着手: ...
- ブロッカー: ...
```

## 参照ドキュメント
- 実装ガイド: [docs/plans/bootstrap-roadmap/IMPLEMENTATION-GUIDE.md](../../docs/plans/bootstrap-roadmap/IMPLEMENTATION-GUIDE.md)
- 現行計画: [docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md](../../docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md)
- 引き継ぎと統計: [docs/phase3-handover.md](docs/phase3-handover.md), [docs/technical-debt.md](docs/technical-debt.md)
- 仕様確認: [docs/spec/0-1-project-purpose.md](../../docs/spec/0-1-project-purpose.md), [docs/spec/1-1-syntax.md](../../docs/spec/1-1-syntax.md), [docs/spec/1-2-types-Inference.md](../../docs/spec/1-2-types-Inference.md)

## ディレクトリ概要
- `src/`: パーサー、型推論、Core IR、CLI などコンパイラ本体
- `tests/`: 字句解析・構文解析・型推論・IR のテストスイート
- `docs/`: フェーズ別報告書、環境セットアップ、技術的負債メモ

## 基本コマンド
### セットアップ
詳細手順は [docs/environment-setup.md](docs/environment-setup.md) を参照。

### ビルド
```bash
dune build
```
`opam` スイッチを指定する場合は `opam exec -- dune build` を利用します。

### CLI 例
```bash
dune exec -- remlc --emit-ast <input.reml>
dune exec -- remlc --emit-tast <input.reml>
```

### テスト
```bash
dune test
```
局所テストは `dune exec -- ./tests/test_parser.exe` など個別バイナリで実行できます。

## 過去フェーズのハイライト
- Phase 1（Parser & Frontend）: AST・Lexer・Parser と CLI 基盤を整備し、ゴールデンテストを含む 165 件以上のテストを構築済み。
- Phase 2（Typer MVP）: Hindley–Milner 型推論と型エラー診断を完成させ、`--emit-tast` CLI と 100+ テストを安定化。

## 備考
- 追加の TODO やリスクは `docs/technical-debt.md` と `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に追記してください。
- README の更新はフェーズ移行時と週次報告後に行い、変更日と担当者を記録することを推奨します。
