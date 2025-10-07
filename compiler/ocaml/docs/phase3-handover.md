# Phase 3 引き継ぎドキュメント

**作成日**: 2025-10-07
**Phase 2 完了日**: 2025-10-07
**Phase 3 開始予定**: 2025-10-07 以降

## Phase 2 の成果物

### 完了した実装

✅ **M2: Typer MVP** - 完全実装
- 型システム基盤（型表現、型環境、単一化）
- Typed AST の完全実装
- 型推論エンジン（Hindley-Milner、単相・let多相）
- 型エラーシステム（15種類、E7001-E7015）
- CLI統合（`--emit-tast` オプション）
- テストインフラ（103+ テストケース、全成功）

詳細は [phase2-completion-report.md](./phase2-completion-report.md) を参照。

## Phase 3 の目標

Phase 3 では以下のマイルストーンを達成します：

### M3: CodeGen MVP ✅ 目標

**計画書**: [1-3-core-ir-min-optimization.md](../../../docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md)

**主要タスク**:
1. Core IR データ構造の設計と実装
2. Typed AST → Core IR の変換（糖衣削除）
3. 最小最適化パス（定数畳み込み、死コード削除）
4. LLVM IR 生成への準備

**期限目安**: 開始後 12 週（Phase 1 開始から累計）

### M4: LLVM IR 生成 ✅ 目標

**計画書**: [1-4-llvm-codegen.md](../../../docs/plans/bootstrap-roadmap/1-4-llvm-codegen.md) 等

**主要タスク**:
1. Core IR → LLVM IR の降格
2. ランタイム連携（参照カウント、メモリ管理）
3. x86_64 Linux ターゲット対応
4. エンドツーエンドテスト

**期限目安**: 開始後 16 週（Phase 1 開始から累計）

## 前提条件の確認

### 開発環境

- [x] OCaml >= 4.14 (推奨: 5.2.1)
- [x] Dune >= 3.0
- [x] Menhir >= 20201216
- [x] LLVM 15+ (Phase 3 で必須)
- [x] opam パッケージマネージャ

### 既存成果物

- [x] AST 定義 (`src/ast.ml`)
- [x] Parser 実装 (`src/parser.mly`, `src/lexer.mll`)
- [x] Typed AST (`src/typed_ast.ml`)
- [x] 型推論エンジン (`src/type_inference.ml`)
- [x] 診断システム (`src/diagnostic.ml`, `src/type_error.ml`)
- [x] テストインフラ (`tests/`)
- [x] CI/CD パイプライン (`.github/workflows/`)

### 仕様書の準備状況

- [x] [1-1-syntax.md](../../../docs/spec/1-1-syntax.md) - 構文仕様（完了）
- [x] [1-2-types-Inference.md](../../../docs/spec/1-2-types-Inference.md) - 型システム仕様（完了）
- [x] [2-5-error.md](../../../docs/spec/2-5-error.md) - エラー仕様（Phase 2 で拡張済み）
- [ ] [3-6-core-diagnostics-audit.md](../../../docs/spec/3-6-core-diagnostics-audit.md) - 診断・監査（Phase 3 で参照）
- [ ] LLVM 連携ガイド - [guides/llvm-integration-notes.md](../../../docs/guides/llvm-integration-notes.md)

## 既存コードベースの構造

### ディレクトリ構成

```
compiler/ocaml/
├── src/                     # コンパイラ本体
│   ├── ast.ml              # AST 定義
│   ├── token.ml            # トークン定義
│   ├── lexer.mll           # 字句解析器
│   ├── parser.mly          # 構文解析器
│   ├── parser_driver.ml    # パーサドライバ
│   ├── diagnostic.ml       # 診断メッセージ
│   ├── ast_printer.ml      # AST プリンター
│   ├── types.ml            # 型表現とスキーム（Phase 2）
│   ├── type_env.ml         # 型環境（Phase 2）
│   ├── constraint.ml       # 型制約と単一化（Phase 2）
│   ├── typed_ast.ml        # 型付きAST（Phase 2）
│   ├── type_inference.ml   # 型推論エンジン（Phase 2）
│   ├── type_error.ml       # 型エラーと診断（Phase 2）
│   └── main.ml             # CLI エントリポイント
├── tests/                   # テストコード
│   ├── test_lexer.ml       # Lexer ユニットテスト
│   ├── test_parser.ml      # Parser ユニットテスト
│   ├── test_pattern_matching.ml  # パターンマッチ専用テスト
│   ├── test_golden.ml      # ゴールデンテスト
│   ├── test_types.ml       # 型システムユニットテスト（Phase 2）
│   ├── test_type_inference.ml    # 型推論テスト（Phase 2）
│   ├── test_type_errors.ml       # 型エラーテスト（Phase 2）
│   ├── test_let_polymorphism.ml  # let多相テスト（Phase 2）
│   ├── simple.reml         # 基本機能テストサンプル
│   ├── pattern_examples.reml     # パターンマッチ実用例
│   └── test_tast.reml      # CLI統合テスト用サンプル
└── docs/                    # 実装ドキュメント
    ├── parser_design.md
    ├── environment-setup.md
    ├── phase1-completion-report.md
    ├── phase2-handover.md
    ├── phase2-checklist.md
    ├── phase2-completion-report.md
    ├── phase3-handover.md (このファイル)
    └── technical-debt.md
```

### 主要モジュールの概要

#### Typed AST モジュール (`src/typed_ast.ml`)

- **役割**: 型情報を含む抽象構文木
- **主要型**:
  - `typed_expr`: 型付き式ノード（推論された型を保持）
  - `typed_pattern`: 型付きパターンノード
  - `typed_decl`: 型付き宣言ノード
  - `typed_compilation_unit`: 型付き編纂単位

**Phase 3 での利用**:
- Core IR への変換元データ
- 型情報の参照元

#### Type Inference モジュール (`src/type_inference.ml`)

- **役割**: 型推論エンジン
- **主要関数**:
  ```ocaml
  val infer_expr : type_env -> Ast.expr -> (typed_expr * Types.ty * substitution, type_error) result
  val infer_compilation_unit : Ast.compilation_unit -> (typed_compilation_unit, type_error) result
  ```

**Phase 3 での利用**:
- コンパイルパイプラインの型チェックフェーズ
- Typed AST の生成

#### Diagnostic モジュール (`src/diagnostic.ml`)

- **役割**: エラー・警告メッセージの管理
- **主要型**:
  ```ocaml
  type t = {
    severity: severity;
    code: string;
    message: string;
    span: span_info;
    notes: note list;
    fixit: fixit list;
  }
  ```

**Phase 3 での拡張ポイント**:
- IR レベルのエラーメッセージ
- LLVM 生成時の診断

## ✅ Phase 3 Week 10-11 で完了した主要コンポーネント

### 1. Core IR 定義 ✅ 完了

実装ファイル: `src/core_ir/ir.ml` (384行)

**実装済み内容**:
- Core IR データ構造（Expr, Stmt, Block, Function）
- 基本ブロックとCFG（制御フローグラフ）
- IR レベルの型情報
- メタデータ（Span、効果、Capability）

**参考**:
- [1-3-core-ir-min-optimization.md](../../../docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md) §1
- [guides/llvm-integration-notes.md](../../../docs/guides/llvm-integration-notes.md)

### 2. 糖衣削除（Desugaring） ✅ 完了

実装ファイル: `src/core_ir/desugar.ml` (638行)

**実装済み機能**:
- パターンマッチの分解
- パイプ演算子の展開
- let 再束縛の正規化
- クロージャ変換

**インターフェース**:
```ocaml
val desugar_expr : Typed_ast.typed_expr -> Core_ir.expr
val desugar_compilation_unit : Typed_ast.typed_compilation_unit -> Core_ir.module_
```

**参考**: [1-3-core-ir-min-optimization.md](../../../docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md) §2

### 3. 最小最適化パス ✅ 完了

実装ファイル:
- `src/core_ir/const_fold.ml` (519行) - 定数畳み込み
- `src/core_ir/dce.ml` (377行) - 死コード削除
- `src/core_ir/cfg.ml` (430行) - CFG 構築
- `src/core_ir/pipeline.ml` (216行) - パイプライン統合

**実装済み機能**:
- 定数畳み込み（算術・比較・論理演算）
- 定数伝播と不動点反復
- 死コード削除（生存解析、未使用束縛削除、到達不能ブロック除去）
- 最適化パイプライン（O0/O1レベル、統計収集）

**インターフェース**:
```ocaml
(* const_fold.ml *)
val optimize_function : ?config:fold_config -> function_def -> function_def * fold_stats
val optimize_module : ?config:fold_config -> module_def -> module_def * fold_stats

(* dce.ml *)
val optimize_function : function_def -> function_def * dce_stats
val optimize_module : module_def -> module_def * dce_stats

(* pipeline.ml *)
val optimize_function : ?config:pipeline_config -> function_def -> function_def * pipeline_stats
val optimize_module : ?config:pipeline_config -> module_def -> module_def * pipeline_stats
```

**テスト**: 42件（const_fold: 26件、dce: 9件、pipeline: 7件）- 全て成功

**参考**: [1-3-core-ir-min-optimization.md](../../../docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md) §3-6

### 4. IR 検査ツールと出力 ✅ 部分完了

実装ファイル: `src/core_ir/ir_printer.ml` (348行)

**実装済み機能**:
- 人間可読な IR 出力フォーマット
- Span情報の保持

**未実装**:
- `--emit-core` CLI（Phase 3 後半で実装予定）
- 中間段階の保存・差分表示

**参考**: [1-3-core-ir-min-optimization.md](../../../docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md) §7

## Phase 3 Week 12-16 で実装する主要コンポーネント

### 5. LLVM IR 生成（次のステップ）

新規ファイル: `src/llvm_gen/codegen.ml`（想定）

**主要機能**:
- Core IR → LLVM IR の降格
- 関数・型の変換
- ランタイム連携（RC、panic）
- x86_64 Linux ターゲット設定

**インターフェース例**:
```ocaml
val codegen_module : Core_ir.module_ -> Llvm.llmodule
val emit_llvm_ir : Llvm.llmodule -> string -> unit
```

**参考**: [1-4-llvm-targeting.md](../../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md)

### 6. LLVM IR テストスイート（次のステップ）

新規ファイル: `tests/test_llvm_gen.ml`（想定）

**テストケース例**:
- LLVM IR の妥当性（`llvm-as`/`opt` 検証）
- エンドツーエンドテスト（ソース → 実行）
- ランタイム連携テスト

## 既知の問題と技術的負債

Phase 2 から引き継ぐ技術的負債は [technical-debt.md](./technical-debt.md) を参照。

### Phase 3 で対応が推奨される項目

1. **配列リテラルの型推論**
   - 優先度: 🟡 Medium
   - Phase 2 で延期した配列リテラル `[1, 2, 3]` の型推論
   - Phase 3 前半で対応すると Core IR 生成が容易

2. **性能測定の実施**
   - 優先度: 🟢 Low
   - 10MB ソースファイルの解析時間計測
   - Core IR 生成・最適化の性能プロファイリング

### Phase 3 以降に持ち越す項目

1. **型クラス（トレイト）の本格実装**
   - 優先度: 🟠 High
   - MVP では基本演算子のみサポート
   - Phase 3-4 で段階的に実装

2. **効果システム**
   - 優先度: 🟠 High
   - 代数的効果の型推論
   - Phase 3-4 で実装

## Phase 3 開始前のチェックリスト

### 環境確認

- [ ] OCaml 環境が正しく設定されているか (`opam env`)
- [ ] すべてのテストが成功するか (`dune test`)
- [ ] ビルドが通るか (`dune build`)
- [ ] LLVM 15+ がインストールされているか (`llvm-config --version`)

### 仕様書の理解

- [ ] [1-3-core-ir-min-optimization.md](../../../docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md) を読む
- [ ] [guides/llvm-integration-notes.md](../../../docs/guides/llvm-integration-notes.md) を読む
- [ ] Core IR の設計方針を理解する

### 計画書の確認

- [ ] [1-3-core-ir-min-optimization.md](../../../docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md) を読む
- [ ] M3 マイルストーンの達成条件を理解
- [ ] 作業ブレークダウンを確認

### ツールとリソース

- [ ] LLVM ツールチェーンの使い方を理解（`llvm-as`, `opt`, `llc`）
- [ ] IR の可視化方法を調査（Graphviz など）
- [ ] ベンチマーク用のサンプルコードを準備

## 推奨される Phase 3 の進め方

### Week 9-10: Core IR 設計と糖衣削除

1. Core IR データ構造の設計と実装
2. Typed AST → Core IR の変換
3. 糖衣削除パス（パターンマッチ、パイプ、クロージャ）

### Week 10-11: 最適化パス実装

1. CFG 構築
2. 定数畳み込み
3. 死コード削除（DCE）
4. 最適化パイプライン統合

### Week 11-12: LLVM IR 生成準備

1. IR Pretty Printer
2. IR 検査ツール
3. テストスイートの整備

### Week 13-16: LLVM IR 生成と統合

1. Core IR → LLVM IR の降格
2. ランタイム連携
3. x86_64 Linux ターゲット対応
4. エンドツーエンドテスト

## 連絡先とサポート

### ドキュメント

- Phase 2 完了報告: [phase2-completion-report.md](./phase2-completion-report.md)
- 技術的負債リスト: [technical-debt.md](./technical-debt.md)
- パーサ設計: [parser_design.md](./parser_design.md)

### 仕様書

- Core IR 計画: [1-3-core-ir-min-optimization.md](../../../docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md)
- LLVM 連携: [guides/llvm-integration-notes.md](../../../docs/guides/llvm-integration-notes.md)
- 診断・監査: [3-6-core-diagnostics-audit.md](../../../docs/spec/3-6-core-diagnostics-audit.md)

### 計画書

- Phase 3 計画: [1-3-core-ir-min-optimization.md](../../../docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md)
- 全体計画: [1-0-phase1-bootstrap.md](../../../docs/plans/bootstrap-roadmap/1-0-phase1-bootstrap.md)

---

**引き継ぎ完了**: 2025-10-07
**Phase 3 開始**: 準備完了
