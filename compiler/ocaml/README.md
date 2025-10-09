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
**更新日**: 2025-10-09（Week 15/16）

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

### ✅ Week 13-14: LLVM IRビルダー実装（完了: 2025-10-09）

計画書: [docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md](../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) §4

**実装統計**:

- 総コード行数: 775行（codegen.ml 662行 + codegen.mli 113行）
- 実装ファイル: 2ファイル（codegen.ml/mli）
- ビルド状態: ✅ 成功（警告なし）
- テスト: 未実装（次ステップ）

**完了項目**:

1. **コードジェネレーションコンテキスト**
   - LLVM コンテキスト・モジュール・ビルダー統合
   - ターゲット設定適用（x86_64 Linux System V ABI）
   - 関数・変数・ブロックマッピング管理

2. **ランタイム関数宣言**
   - `mem_alloc`, `inc_ref`, `dec_ref`, `panic`の外部宣言
   - noreturn属性設定（panicのみ）

3. **式のコード生成**（9種類対応）
   - Literal（整数・浮動小数・Bool・Char・String・Unit）
   - Var（変数参照）
   - App（関数適用、LLVM 18 opaque pointer対応）
   - Let（let束縛）
   - If（条件分岐、φノード生成）
   - Primitive（17種類の演算：算術・比較・論理・ビット）
   - TupleAccess（タプル要素アクセス）

4. **終端命令生成**
   - TermReturn、TermJump、TermBranch、TermUnreachable

5. **文のコード生成**（6種類）
   - Assign、Return、Jump、Branch、Phi、ExprStmt

6. **関数・モジュール生成**
   - 関数宣言生成（System V calling convention）
   - グローバル変数生成骨格
   - 基本ブロック生成（2フェーズ：全ブロック作成→命令生成）
   - モジュール全体生成パイプライン

7. **LLVM IR出力**
   - テキスト形式（.ll）出力
   - ビットコード形式（.bc）出力

8. **ビルド設定**
   - dune設定更新（codegen追加、llvm.bitwriter依存追加）

9. **型インポートエラー修正（2025-10-09）**
   - `Ast.literal` コンストラクタへの統一（`Int`, `Float`, `Bool`, `Char`, `String`, `Unit`）
   - 複合リテラル（`Tuple`, `Array`, `Record`）のエラーハンドリング追加
   - LLVM 18 opaque pointer対応（`build_call`の型引数追加）
   - 未使用変数警告の解消（`_prefix`による明示化）

### ✅ Week 14-15: ABI・呼び出し規約の実装（完了: 2025-10-09）

計画書: [docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md](../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) §5

**実装統計**:

- 総コード行数: 約400行（abi.ml 約200行 + abi.mli + type_mapping拡張 + codegen統合）
- 実装ファイル: 3ファイル（abi.ml/mli、type_mapping.ml/mli拡張、codegen.ml統合）
- ビルド状態: ✅ 成功（警告のみ、エラーなし）
- テスト: 次ステップで実装予定

**完了項目**:

1. **ABIモジュール実装** (`src/llvm_gen/abi.ml`)
   - ABI分類型定義（`return_classification`, `argument_classification`）
   - System V ABI準拠の判定ロジック（16バイト閾値）
   - Windows x64対応の基盤整備（Phase 2で有効化予定）

2. **ABI判定関数**
   - `classify_struct_return`: 構造体戻り値のABI分類（DirectReturn / SretReturn）
   - `classify_struct_argument`: 構造体引数のABI分類（DirectArg / ByvalArg）
   - 型サイズ計算（`get_type_size`）と構造体型判定

3. **LLVM属性設定関数**
   - `add_sret_attr`: 大きい構造体戻り値にsret属性を付与
   - `add_byval_attr`: 大きい構造体引数にbyval属性を付与
   - LLVM 18 API制限により文字列属性として実装（Phase 2で型付き属性に拡張予定）

4. **codegen.mlへのABI統合**
   - 関数宣言生成時にABI判定とLLVM属性付与を実装
   - sret属性による引数インデックスオフセット処理（隠れた戻り値用ポインタ対応）
   - 各引数へのbyval属性適用ロジック

5. **type_mapping拡張**
   - `get_llcontext`関数を公開API化（abiモジュールから利用）
   - インターフェース（.mli）と実装（.ml）の両方に追加

6. **ビルド設定更新**
   - dune設定にabiモジュールを追加
   - LLVM 18バインディングとの互換性確認

**実装の特徴**:

- **System V ABI準拠**: x86_64 Linux向けに16バイト閾値で構造体のレジスタ/メモリ渡しを判定
- **拡張性**: ターゲット別ABI切り替え機構（Windows x64は8バイト閾値、Phase 2で有効化）
- **LLVM 18対応**: opaque pointer・文字列属性APIを使用（型付き属性はバインディング制限により延期）
- **型安全**: OCamlの型システムでABI分類を明示的に表現（variant型による分岐）

**技術的負債とフォローアップ**:

- **LLVM 18型付き属性**: llvm-ocamlバインディングで`create_type_attr`が未サポートのため、文字列属性として実装。Phase 2でバインディング更新または手動FFIで対応予定。
- **複雑な構造体レイアウト**: Phase 1はタプル・レコードのみ対応。ネスト構造体・ADTのABI判定はPhase 2で拡張。
- **テスト未実装**: ABI判定ロジックと属性設定の正常性を検証するユニットテストは次週実装予定。

**次のステップ**:

- Week 15-16: LLVM IR検証パイプライン（llvm-as, opt -verify, llc）、テストスイート整備
- Week 16: CLI統合（`--emit-ir`）、ドキュメント整備

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
