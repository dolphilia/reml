# compiler/ocaml ワークスペース

このディレクトリは Reml ブートストラップ計画 Phase 1〜3 の OCaml 実装を管理し、以降のセルフホスト化へ向けてランタイム統合を進める作業拠点です。フェーズ別の詳細計画は `docs/plans/bootstrap-roadmap/` を参照してください。

## 現在のステータス
- Phase 1 — Parser & Frontend（完了: 2025-10-06）: `compiler/ocaml/docs/phase1-completion-report.md`
- Phase 2 — Typer MVP（完了: 2025-10-07）: `compiler/ocaml/docs/phase2-completion-report.md`
- Phase 3 — Core IR & LLVM 生成（完了: 2025-10-09）: `compiler/ocaml/docs/phase3-m3-completion-report.md`
- 次のマイルストーン: `docs/plans/bootstrap-roadmap/1-5-runtime-integration.md`（Phase 1-5 ランタイム連携）

過去フェーズの週次レポートや統計は `compiler/ocaml/docs/` 配下の各完了報告・引き継ぎ資料に集約しています。

## 1-5 ランタイム連携のフォーカス
### 目的
- 最小ランタイム API（`mem_alloc`, `mem_free`, `inc_ref`, `dec_ref`, `panic`, `print_i64`）を実装し、生成した LLVM IR からリンク可能にする。
- `docs/guides/llvm-integration-notes.md` §5 に準拠した参照カウント (RC) モデルを Phase 1 の OCaml 実装に接続する。
- ランタイム品質の検証（リーク・ダングリング検出、Valgrind/ASan 連携）と CI 統合を行う。

### 成果物と出口条件
- `runtime/native/` に最小ランタイムのソース・ビルドスクリプト・テストを配置し、`make runtime` で静的ライブラリ `libreml_runtime.a` を生成できる。
- `compiler/ocaml/src/llvm_gen/` からランタイム関数を宣言・呼び出し、`--link-runtime` オプションでバイナリ生成まで通す。
- ランタイム API と RC モデルのテスト結果・計測値を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に記録し、`compiler/ocaml/docs/` に検証ノートを残す。

### 作業トラック（詳細は計画書 §1〜§8 を参照）
- **API 定義**: `runtime/reml_runtime.h` を作成し、関数シグネチャ・ヘッダ構成・型タグ規約を決める。
- **メモリアロケータ**: `runtime/mem_alloc.c`（malloc ベース、8 バイト境界調整、デバッグフック）を実装。
- **参照カウント**: `runtime/refcount.c` で RC 操作と型別デストラクタ呼び出しを整備。
- **パニックハンドラ**: `runtime/panic.c` で診断フォーマットと終了処理 (`exit(1)`) を実装。
- **ビルドシステム**: `runtime/Makefile`（`-O2`/`-Wall -Wextra`/`-g`）を用意し、プラットフォーム検出と依存関係を整理。
- **LLVM 連携**: `compiler/ocaml/src/llvm_gen/codegen.ml` と `abi.ml` でランタイムシンボル宣言・属性設定・リンクフラグを統合（`llvm_attr.ml` + C スタブで `sret` / `byval` の型付き属性を付与）。
- **テストと検証**: `runtime/native/tests/` と `compiler/ocaml/tests/codegen/` に単体/統合テストを追加し、Valgrind/ASan のジョブを CI に組み込む。
- **ドキュメントと CI**: `docs/guides/llvm-integration-notes.md` および `compiler/ocaml/docs/` を更新し、GitHub Actions でランタイムビルドと検証を自動化。

## 直近の準備チェックリスト
- `docs/plans/bootstrap-roadmap/1-5-runtime-integration.md` を精読し、各トラックのスコープと完了条件を確認する。
- `compiler/ocaml/src/llvm_gen/` で呼び出しているランタイム関数を洗い出し、必要なシグネチャが計画書と一致しているか確認する（特に `panic` の属性と `inc_ref`/`dec_ref` の呼び出し箇所）。
- `compiler/ocaml/docs/phase3-to-phase2-handover.md`・`compiler/ocaml/docs/technical-debt.md` の High 優先度項目（型マッピング TODO, CFG 線形化など）がランタイム統合のブロッカーにならないよう対応状況を見直す。
- `runtime/native/` の既存ファイル構成と CI スクリプト (`compiler/ocaml/scripts/verify_llvm_ir.sh` など) を確認し、ランタイム検証を追加する際の差分影響を把握する。
- 計測結果を追記するための記録先（`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`）とリスク登録先（`docs/plans/bootstrap-roadmap/0-4-risk-handling.md`）のフォーマットを再確認する。

## 関連ドキュメント
- **計画書**: `docs/plans/bootstrap-roadmap/IMPLEMENTATION-GUIDE.md`, `docs/plans/bootstrap-roadmap/1-0-phase1-bootstrap.md`, `docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md`, `docs/plans/bootstrap-roadmap/1-5-runtime-integration.md`
- **仕様・ガイド**: `docs/spec/0-1-project-purpose.md`, `docs/spec/1-1-syntax.md`, `docs/guides/llvm-integration-notes.md`, `docs/notes/llvm-spec-status-survey.md`
- **進捗記録**: `compiler/ocaml/docs/phase3-m3-completion-report.md`, `compiler/ocaml/docs/phase3-to-phase2-handover.md`, `compiler/ocaml/docs/technical-debt.md`, `compiler/ocaml/docs/phase3-remaining-tasks.md`

## ワークスペース概要
- `compiler/ocaml/src/`: パーサー、型推論、Core IR、LLVM 生成、CLI
- `compiler/ocaml/tests/`: 字句解析〜LLVM 検証までのテストスイート、ゴールデンファイル
- `runtime/native/`: ランタイム実装（Phase 1-5 で拡充予定）
- `compiler/ocaml/docs/`: フェーズ完了報告、技術的負債、環境設定メモ

### 基本コマンド
```bash
opam exec -- dune build       # ビルド
opam exec -- dune test        # テスト一式
opam exec -- dune exec -- remlc --emit-ir samples/basic.reml
```
ランタイム連携後は `--link-runtime` オプションおよび `runtime/Makefile` のビルド結果を CI で検証します。

## 過去フェーズの詳細
- Phase 1/2 の仕様・テスト整備: `compiler/ocaml/docs/phase1-completion-report.md`, `compiler/ocaml/docs/phase2-completion-report.md`
- Phase 3 の Core IR・LLVM 成果: `compiler/ocaml/docs/phase3-m3-completion-report.md`, `compiler/ocaml/docs/phase3-week10-11-completion.md`
- 残課題とフォローアップ: `compiler/ocaml/docs/phase3-remaining-tasks.md`, `compiler/ocaml/docs/technical-debt.md`

詳細な進捗ログや週次の統計は各報告書を参照してください。README では次フェーズへ進むための要約と着手ポイントのみを保持します。
