# Phase 2 引き継ぎドキュメント

**作成日**: 2025-10-06
**Phase 1 完了日**: 2025-10-06
**Phase 2 開始予定**: 2025-10-06

## Phase 1 の成果物

### 完了した実装

✅ **M1: Parser MVP** - 完全実装
- AST 定義とパーサ実装
- パターンマッチの網羅的検証
- 診断メッセージシステム
- テストインフラ（165+ テストケース）

詳細は [phase1-completion-report.md](./phase1-completion-report.md) を参照。

## Phase 2 の目標

Phase 2 では以下のマイルストーンを達成します：

### M2: Typer MVP ✅ 目標

**計画書**: [1-2-typer-implementation.md](../../../docs/plans/bootstrap-roadmap/1-2-typer-implementation.md)

**主要タスク**:
1. Typed AST の設計
2. Hindley-Milner 型推論の実装
3. 型エラーメッセージの整備
4. 型推論スナップショットテスト

**期限目安**: 開始後 8 週（Phase 1 開始から累計）

## 前提条件の確認

### 開発環境

- [x] OCaml >= 4.14 (推奨: 5.2.1)
- [x] Dune >= 3.0
- [x] Menhir >= 20201216
- [x] opam パッケージマネージャ

### 既存成果物

- [x] AST 定義 (`src/ast.ml`)
- [x] Parser 実装 (`src/parser.mly`, `src/lexer.mll`)
- [x] テストインフラ (`tests/`)
- [x] CI/CD パイプライン (`.github/workflows/`)

### 仕様書の準備状況

- [x] [1-1-syntax.md](../../../docs/spec/1-1-syntax.md) - 構文仕様（完了）
- [x] [1-2-types-Inference.md](../../../docs/spec/1-2-types-Inference.md) - 型システム仕様（Phase 2 で参照）
- [x] [2-5-error.md](../../../docs/spec/2-5-error.md) - エラー仕様（Phase 2 で拡張）

## 既存コードベースの構造

### ディレクトリ構成

```
compiler/ocaml/
├── src/                  # コンパイラ本体
│   ├── ast.ml           # AST 定義（Phase 2 で Typed AST 追加予定）
│   ├── token.ml         # トークン定義
│   ├── lexer.mll        # 字句解析器
│   ├── parser.mly       # 構文解析器
│   ├── parser_driver.ml # パーサドライバ
│   ├── diagnostic.ml    # 診断メッセージ
│   ├── ast_printer.ml   # AST プリンター
│   └── main.ml          # CLI エントリポイント
├── tests/               # テストコード
│   ├── test_lexer.ml
│   ├── test_parser.ml
│   ├── test_pattern_matching.ml
│   ├── test_golden.ml
│   ├── simple.reml
│   └── pattern_examples.reml
└── docs/                # 実装ドキュメント
    ├── parser_design.md
    ├── environment-setup.md
    ├── phase1-completion-report.md
    └── phase2-handover.md (このファイル)
```

### 主要モジュールの概要

#### AST モジュール (`src/ast.ml`)

- **役割**: 抽象構文木の定義
- **主要型**:
  - `expr`: 式ノード
  - `pattern`: パターンノード
  - `decl`: 宣言ノード
  - `stmt`: 文ノード
  - `type_annot`: 型注釈
  - `span`: 位置情報

**Phase 2 での拡張ポイント**:
- Typed AST の追加（`typed_expr`, `typed_pattern` など）
- 型情報の格納
- 型変数の管理

#### Parser Driver (`src/parser_driver.ml`)

- **役割**: パーサのエントリポイント
- **インターフェース**:
  ```ocaml
  val parse_string : string -> (Ast.compilation_unit, Diagnostic.t) Result.t
  val parse_file : string -> (Ast.compilation_unit, Diagnostic.t) Result.t
  ```

**Phase 2 での拡張ポイント**:
- 型チェックパスの追加
- Typed AST の返却

#### Diagnostic モジュール (`src/diagnostic.ml`)

- **役割**: エラー・警告メッセージの管理
- **主要型**:
  ```ocaml
  type t = {
    severity: severity;
    message: string;
    span: span_info;
  }
  ```

**Phase 2 での拡張ポイント**:
- 型エラー専用メッセージの追加
- 期待される型と実際の型の表示
- 型不一致の詳細情報

## Phase 2 で実装する主要コンポーネント

### 1. Typed AST 定義

新規ファイル: `src/typed_ast.ml`

**含むべき内容**:
- 型情報を含む式ノード
- 型環境 (type environment)
- 型変数の管理

**参考**:
- [1-2-types-Inference.md](../../../docs/spec/1-2-types-Inference.md) の型定義
- Hindley-Milner 型システムの標準実装

### 2. 型推論エンジン

新規ファイル: `src/type_inference.ml`

**主要機能**:
- 型変数の生成
- 単一化 (Unification)
- 一般化 (Generalization)
- インスタンス化 (Instantiation)

**インターフェース例**:
```ocaml
val infer_expr : type_env -> Ast.expr -> (Typed_ast.typed_expr * ty, type_error) Result.t
val infer_decl : type_env -> Ast.decl -> (Typed_ast.typed_decl * type_env, type_error) Result.t
```

### 3. 型エラーメッセージ

新規ファイル: `src/type_error.ml`

**主要型**:
```ocaml
type type_error =
  | UnificationFailure of { expected: ty; actual: ty; span: span }
  | UnboundVariable of { name: string; span: span }
  | OccursCheck of { var: type_var; ty: ty; span: span }
  | ArityMismatch of { expected: int; actual: int; span: span }
```

### 4. テストスイート

新規ファイル: `tests/test_type_inference.ml`

**テストケース例**:
- 基本型推論（整数、文字列、ブール値）
- let 多相の推論
- 関数型の推論
- 型エラーケース（型不一致、未定義変数）

## 既知の問題と技術的負債

Phase 1 から引き継ぐ既知の問題は [technical-debt.md](./technical-debt.md) を参照。

### 必須対応（Phase 2 で修正）

1. **レコードパターンの複数アーム制限**
   - 問題: コンストラクタ+短縮形フィールドの組み合わせ
   - 優先度: 中
   - 影響範囲: 限定的

2. **Handler 宣言のパース問題**
   - 問題: TODO テストが予期せず成功
   - 優先度: 低
   - 影響範囲: handler 構文の一部

### 任意対応（Phase 2 以降）

1. **Unicode XID 対応**
   - 問題: ASCII 識別子のみサポート
   - 優先度: 低
   - 仕様書では要求されているが、MVP では不要

2. **性能測定**
   - 10MB ソースの解析時間計測
   - メモリプロファイリング

## Phase 2 開始前のチェックリスト

### 環境確認

- [ ] OCaml 環境が正しく設定されているか (`opam env`)
- [ ] すべてのテストが成功するか (`dune test`)
- [ ] ビルドが通るか (`dune build`)

### 仕様書の理解

- [ ] [1-2-types-Inference.md](../../../docs/spec/1-2-types-Inference.md) を読む
- [ ] Hindley-Milner 型推論の基礎を理解
- [ ] [2-5-error.md](../../../docs/spec/2-5-error.md) の型エラー仕様を確認

### 計画書の確認

- [ ] [1-2-typer-implementation.md](../../../docs/plans/bootstrap-roadmap/1-2-typer-implementation.md) を読む
- [ ] M2 マイルストーンの達成条件を理解
- [ ] 作業ブレークダウンを確認

### ツールとリソース

- [ ] 型推論の参考実装を調査（OCaml、SML、Haskell）
- [ ] 型エラーメッセージのベストプラクティスを調査
- [ ] Dune でのテスト作成方法を理解

## 推奨される Phase 2 の進め方

### Week 1-2: Typed AST 設計

1. `src/typed_ast.ml` の設計と実装
2. 型環境の定義
3. 型変数の管理機構

### Week 3-4: 型推論コアの実装

1. Unification アルゴリズム
2. 基本的な型推論（リテラル、変数、関数適用）
3. let 多相の実装

### Week 5-6: エラーハンドリングとテスト

1. 型エラーメッセージの実装
2. 型推論テストスイートの作成
3. スナップショットテストの整備

### Week 7-8: 統合と品質保証

1. Parser との統合
2. CLI への型チェック機能追加
3. CI/CD への型テスト追加
4. ドキュメント整備

## 連絡先とサポート

### ドキュメント

- Phase 1 完了報告: [phase1-completion-report.md](./phase1-completion-report.md)
- 技術的負債リスト: [technical-debt.md](./technical-debt.md)
- パーサ設計: [parser_design.md](./parser_design.md)

### 仕様書

- 型システム: [1-2-types-Inference.md](../../../docs/spec/1-2-types-Inference.md)
- エラー仕様: [2-5-error.md](../../../docs/spec/2-5-error.md)

### 計画書

- Phase 2 計画: [1-2-typer-implementation.md](../../../docs/plans/bootstrap-roadmap/1-2-typer-implementation.md)
- 全体計画: [1-0-phase1-bootstrap.md](../../../docs/plans/bootstrap-roadmap/1-0-phase1-bootstrap.md)

---

**引き継ぎ完了**: 2025-10-06
**Phase 2 開始**: 準備完了
