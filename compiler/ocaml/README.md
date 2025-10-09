# compiler/ocaml ワークスペース

このディレクトリは Reml ブートストラップ計画 Phase 1〜3 の OCaml 実装をまとめる作業領域です。最新の指針は `docs/plans/bootstrap-roadmap/` を参照してください。

## フェーズ状況
- Phase 1 — Parser & Frontend（完了: 2025-10-06）: [docs/phase1-completion-report.md](docs/phase1-completion-report.md)
- Phase 2 — Typer MVP（完了: 2025-10-07）: [docs/phase2-completion-report.md](docs/phase2-completion-report.md)
- Phase 3 — Core IR & LLVM 生成（進行中: Week 11/16）
  - ✅ Week 9-11: 最適化パス完了（定数畳み込み・DCE・パイプライン統合）
  - 📍 Week 12-16: LLVM IR 生成（次フェーズ）
  - 完了報告: [docs/phase3-week10-11-completion.md](docs/phase3-week10-11-completion.md)
  - 引き継ぎ: [docs/phase3-handover.md](docs/phase3-handover.md)

## Phase 3 ダッシュボード
**更新日**: 2025-10-07（Week 11/16）

### ✅ Week 9-11: Core IR 最適化パス（完了）

完了報告書: [docs/phase3-week10-11-completion.md](docs/phase3-week10-11-completion.md)

**実装統計**:
- 総コード行数: 5,642行（Core IR関連全実装）
- 実装ファイル: 7ファイル（ir.ml, ir_printer.ml, desugar.ml, cfg.ml, const_fold.ml, dce.ml, pipeline.ml）
- テスト: 42/42 成功（回帰なし）

**主要コンポーネント**:

1. **定数畳み込み（Constant Folding）**
   - `src/core_ir/const_fold.ml`（519行）
   - リテラル変換（Base2/8/10/16対応）
   - 算術演算・比較演算・論理演算の畳み込み
   - 条件分岐の静的評価（`if true then A` → `A`）
   - 定数伝播と不動点反復
   - テスト: 26/26 成功

2. **死コード削除（Dead Code Elimination）**
   - `src/core_ir/dce.ml`（377行）
   - 生存解析（liveness analysis）
   - 未使用束縛の削除
   - 到達不能ブロックの除去
   - 副作用保護
   - テスト: 9/9 成功

3. **最適化パイプライン統合**
   - `src/core_ir/pipeline.ml`（216行）
   - 不動点反復フレームワーク（ConstFold → DCE → ConstFold ...）
   - 最適化レベル設定（O0/O1）
   - 統計収集とレポート
   - テスト: 7/7 成功

4. **Core IR基盤**（Week 9完了）
   - `src/core_ir/ir.ml`: Core IR型定義（384行）
   - `src/core_ir/ir_printer.ml`: Pretty Printer（348行）
   - `src/core_ir/desugar.ml`: 糖衣削除（638行）
   - `src/core_ir/cfg.ml`: CFG構築（430行）

### ✅ Week 12-13: LLVM IR 型マッピング（完了: 2025-10-09）

計画書: [docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md](../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md)

**実装統計**:

- 総コード行数: 540行（LLVM型マッピング関連全実装）
- 実装ファイル: 4ファイル（type_mapping.ml/mli, target_config.ml/mli）
- テスト: 35/35 成功（回帰なし）

**完了項目**:

1. **LLVM 18 バインディング統合**
   - opam パッケージ: `llvm.18-static` インストール済み
   - opaque pointer 対応（LLVM 18 の新仕様に準拠）
   - dune-project バージョン制約: `>= 15.0 & < 20`
   - ビルド検証完了（警告なし）

2. **型マッピング実装**
   - `src/llvm_gen/type_mapping.ml` — Reml型からLLVM型への変換（280行）
   - プリミティブ型マッピング（Bool→i1, i64→i64, String→FAT pointer等）
   - 複合型マッピング（タプル、レコード、配列、関数型）
   - FAT pointer構造（`{ptr, i64}`）の実装
   - Tagged union（ADT）構造（`{i32, payload}`）の実装

3. **ターゲット設定実装**
   - `src/llvm_gen/target_config.ml` — DataLayoutとターゲット設定（180行）
   - DataLayout設定（x86_64 Linux: `e-m:e-p:64:64-f64:64:64-v128:128:128-a:0:64`）
   - ターゲットトリプル: `x86_64-unknown-linux-gnu`

4. **テストとドキュメント**
   - テストスイート（`tests/test_llvm_type_mapping.ml`, 35件）- 全成功
   - 技術文書（`docs/llvm-type-mapping.md`）
   - 環境構築ガイド更新（`docs/environment-setup.md`）- LLVM 18 対応手順を追記

📍 **次のステップ（Week 13-14）**:

- Week 13: LLVM IRビルダー実装（モジュール・関数・基本ブロック生成）
- Week 14-15: ABI・呼び出し規約の実装
- Week 16: LLVM IR検証パイプライン

### 記録ルール
- 週次で本セクションを更新
- 詳細な議事録: [docs/phase3-handover.md](docs/phase3-handover.md), [docs/technical-debt.md](docs/technical-debt.md)
- 測定値: [docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md](../../docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md)

### 週次更新テンプレート
```text
Week NN（YYYY-MM-DD 更新）
- 完了: ...
- 進行中: ...
- 次に着手: ...
- ブロッカー: ...
```

## 参照ドキュメント

### Phase 3 関連
- 現行計画: [docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md](../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) （Week 12-16）
- 完了済み: [docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md](../../docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md) （Week 9-11）
- 完了報告: [docs/phase3-week10-11-completion.md](docs/phase3-week10-11-completion.md)
- 引き継ぎと統計: [docs/phase3-handover.md](docs/phase3-handover.md), [docs/technical-debt.md](docs/technical-debt.md)
- 測定値とメトリクス: [docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md](../../docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md)

### 実装ガイド
- 実装ガイド: [docs/plans/bootstrap-roadmap/IMPLEMENTATION-GUIDE.md](../../docs/plans/bootstrap-roadmap/IMPLEMENTATION-GUIDE.md)
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
