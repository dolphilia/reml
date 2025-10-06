# compiler/ocaml ワークスペース

**現在のフェーズ**: Phase 1 完了 → Phase 2 準備中

Phase 1 ブートストラップ計画に基づき、OCaml 製 Reml コンパイラを構築するための作業領域です。対応するタスクは主に [`docs/plans/bootstrap-roadmap/1-x`](../../docs/plans/bootstrap-roadmap/) に定義されています。

## 📊 進捗状況

### ✅ Phase 1 完了（2025-10-06）
- **M1: Parser MVP** - 完全実装
- **パターンマッチ検証** - 35+ テストケース全て成功
- **テストインフラ** - 165+ テストケース

**詳細**: [Phase 1 完了報告書](docs/phase1-completion-report.md)

### 🚀 Phase 2 準備完了
- **M2: Typer MVP** - 開始準備完了
- **引き継ぎ**: [Phase 2 ハンドオーバー](docs/phase2-handover.md)
- **チェックリスト**: [Phase 2 開始前チェックリスト](docs/phase2-checklist.md)

## ディレクトリ
- `src/`: コンパイラ本体（パーサー、型推論、Core IR、LLVM 出力など）
- `tests/`: ゴールデン AST・型推論スナップショット・IR 検証などのテストコード
- `docs/`: 実装メモ、設計ノート、Phase 移行ドキュメント

## セットアップ

### 前提条件
- OCaml >= 4.14 (推奨: 5.2.1)
- Dune >= 3.0
- Menhir >= 20201216

### 詳細なセットアップ手順

**📖 [環境セットアップガイド](docs/environment-setup.md)** を参照してください。

macOS、Linux、Windows (WSL) での詳細な手順を提供しています。

### クイックスタート（macOS）

```bash
# opamのインストール
brew install opam
opam init --auto-setup --yes
eval $(opam env)

# OCaml 5.2.1のインストール
opam switch create 5.2.1
eval $(opam env --switch=5.2.1)

# 必要なパッケージをインストール
opam install dune menhir --yes
```

## ビルド方法

```bash
# プロジェクトルート (compiler/ocaml) で実行
dune build

# 実行可能ファイルのパス
./_build/default/src/remlc.exe
```

## 使用方法

```bash
# AST を出力
dune exec -- remlc --emit-ast <input.reml>

# 例
dune exec -- remlc --emit-ast ../../examples/language-impl-comparison/reml/pl0_combinator.reml
```

## テスト実行

### すべてのテストを実行

```bash
dune test
```

### 個別のテストを実行

```bash
# Lexer ユニットテスト
dune exec tests/test_lexer.exe

# Parser ユニットテスト
dune exec tests/test_parser.exe

# Golden テスト
dune exec tests/test_golden.exe
```

### テストの説明

- **test_lexer**: 字句解析の境界ケースと基本機能を検証
  - キーワード、識別子、リテラル（整数、浮動小数、文字、文字列）
  - 演算子、コメント（行コメント、入れ子ブロックコメント）
  - 複合トークン列

- **test_parser**: 構文解析の成功ケースを検証
  - モジュールヘッダ、use宣言
  - let/var/fn/type/extern 宣言（trait/impl/handler は TODO として失敗を期待）
  - 式（リテラル、二項演算、パイプ、関数呼び出し、if/match/while/for、unsafe など）
  - 未実装の構文（フィールドアクセス、`loop` など）は `todo` テストで明示
  - パターンマッチ、属性、基本的な効果宣言
  - エラーケース（構文エラーの検出）

- **test_golden**: サンプルファイルのAST出力をスナップショットと比較
  - `tests/simple.reml`: 基本的な宣言と式のゴールデンテスト
  - ゴールデンファイル (`tests/golden/*.golden`) が存在しない場合は失敗し、`tests/golden/_actual/` に最新出力を保存
  - 差分が出た場合も `_actual` ディレクトリへ出力するので、意図した変更ならゴールデンを更新する

### テスト対象ファイル

- `tests/simple.reml`: Phase 1 の基本機能テスト用サンプル

## 現在の実装状況 (M1 マイルストーン)

### ✅ 完了
- [x] AST 定義 (`src/ast.ml`)
- [x] トークン定義 (`src/token.ml`)
- [x] Lexer 実装 (`src/lexer.mll`)
  - Unicode XID 準拠識別子 (Phase 1: ASCII のみ)
  - 整数・浮動小数・文字・文字列リテラル
  - コメント処理 (行コメント、入れ子ブロックコメント)
  - エスケープシーケンス
- [x] Parser 実装 (`src/parser.mly`)
  - 基本的な式・宣言の構文解析
  - 演算子優先順位 (Menhir %left/%right)
  - Span 情報付与
- [x] Dune ビルドシステム
- [x] CLI エントリポイント (`src/main.ml`)
- [x] テストインフラ整備
  - Lexer ユニットテスト (`tests/test_lexer.ml`)
  - Parser ユニットテスト (`tests/test_parser.ml`)
  - ゴールデンテスト (`tests/test_golden.ml`)
  - Dune テストルール (`tests/dune`)
- [x] `Parser_driver` による Result ベースの診断出力と CLI 連携

### ✅ 完了（2025-10-06 更新）
- [x] **後置演算子の実装**
  - フィールドアクセス (`expr.field`)
  - タプルアクセス (`expr.0`)
  - インデックスアクセス (`expr[i]`)
  - 伝播演算子 (`expr?`)
- [x] **制御フロー構文の拡張**
  - `match` 式（複数アーム、ネストパターン、ガード条件対応）
  - `while` 式（基本ケース）
  - `for` 式（パターン分解対応）
  - `loop` 式
  - ブロック式 `{ ... }` の関数本体対応
- [x] **複雑ケーステスト追加**
  - ネストしたループ構文
  - パターン分解を伴う `for` ループ
  - 制御フロー専用テストセクション追加
- [x] **リテラルパターンの実装**
  - 整数、浮動小数、文字列、文字、真偽値のパターンマッチ対応
- [x] **match 式の複数アーム対応**
  - 複数アームの正しいパース
  - ネストした match 式
  - ガード条件 (`if`) 付きパターン

### ✅ 完了（2025-10-06 更新 - 代入文対応）
- [x] **代入文の左辺値拡張**
  - `LValue := Expr` の `LValue` を `ident` から `postfix_expr` に拡張
  - フィールドアクセス (`obj.field := value`)、インデックスアクセス (`arr[i] := value`)、タプルアクセス (`tuple.0 := value`) に対応
  - AST定義、パーサルール、AST Printerを更新し、仕様書 §D.2 `AssignStmt ::= LValue ":=" Expr` に準拠

### ✅ 完了（2025-10-06 更新 - パターンマッチの完全検証）
- [x] **パターンマッチの網羅的テスト実装**
  - ネストパターン（2層・3層）の完全検証: `Some(Some(x))`, `Ok(Some(value))`, `((a, b), (c, d))`
  - ガード条件の複雑ケース: 複数変数参照、ネストパターン+ガード
  - リテラルパターン（整数・文字列・文字・真偽値）の網羅的テスト
  - レコードパターン+コンストラクタ+rest の組み合わせ
  - 専用テストスイート `tests/test_pattern_matching.ml` (35+ テストケース) を追加
  - 実用例を含むサンプルファイル `tests/pattern_examples.reml` を追加
  - **Phase 1 で要求される全パターンマッチ機能の動作を確認済み**

### 🚧 既知の制限事項
- **レコードパターンの複数アーム制限**: `{ field: Constructor(x), other }` の形式（コンストラクタ+短縮形フィールド）を複数アームで使用すると、パーサが構文エラーを報告する既知の問題がある。回避策として、各フィールドを明示的に `field: pattern` の形式で記述するか、単一アームの match を使用する。Phase 2 で修正予定。

### 📝 Phase 2 への移行

**Phase 1 は完了しました。Phase 2（型推論実装）への移行準備が整っています。**

#### Phase 2 開始前に確認すべきドキュメント

1. **[Phase 1 完了報告書](docs/phase1-completion-report.md)**
   - Phase 1 の成果物と統計情報
   - テスト結果サマリー
   - 既知の制限事項

2. **[Phase 2 ハンドオーバー](docs/phase2-handover.md)**
   - Phase 2 の目標とタスク
   - 既存コードベースの構造
   - 実装する主要コンポーネント

3. **[Phase 2 開始前チェックリスト](docs/phase2-checklist.md)**
   - 環境確認（46 項目）
   - 仕様書の理解
   - 技術的準備

4. **[技術的負債リスト](docs/technical-debt.md)**
   - Phase 1 からの既知の問題
   - 優先度別の対応計画

#### Phase 2 で実装する主要機能

- **Typed AST 定義** (`src/typed_ast.ml`)
- **型推論エンジン** (`src/type_inference.ml`)
- **型エラーメッセージ** (`src/type_error.ml`)
- **型推論テストスイート** (`tests/test_type_inference.ml`)

## 技術詳細

設計ドキュメント: [docs/parser_design.md](docs/parser_design.md)

### AST 設計
- すべてのノードに `span: { start: int; end_: int }` を付与
- バイトオフセットで位置を記録 (行・列番号は診断時に計算)
- 仕様書 [1-1-syntax.md](../../docs/spec/1-1-syntax.md) に準拠

### 演算子優先順位
仕様書 §D.1 の固定優先順位表をMenhirの %left/%right で実装:
- 最高優先: 後置演算子 (関数呼び出し、フィールドアクセス、`?`)
- 最低優先: パイプ `|>` (左結合)

### Unicode 対応
- Phase 1: ASCII 識別子のみサポート (`[a-zA-Z_][a-zA-Z0-9_]*`)
- Phase 2 以降: Unicode XID 完全対応予定

### AST ダンプ
- `src/ast_printer.ml` で CLI とテスト向けの共通 AST 文字列表現を提供
- ゴールデンテストと `--emit-ast` の出力はこのプリンタを利用
