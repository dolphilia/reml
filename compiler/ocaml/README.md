# compiler/ocaml ワークスペース（Phase 1）

Phase 1 ブートストラップ計画に基づき、OCaml 製 Reml コンパイラを構築するための作業領域です。対応するタスクは主に [`docs/plans/bootstrap-roadmap/1-x`](../../docs/plans/bootstrap-roadmap/) に定義されています。

## ディレクトリ
- `src/`: コンパイラ本体（パーサー、型推論、Core IR、LLVM 出力など）
- `tests/`: ゴールデン AST・型推論スナップショット・IR 検証などのテストコード
- `docs/`: 実装メモ、設計ノート、調査結果

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

### 🚧 進行中
- [ ] エラー回復戦略の実装
- [ ] 診断モデル ([2-5-error.md](../../docs/spec/2-5-error.md) 準拠)
- [ ] 完全な構文要素のカバレッジ (match, while, for, 等)
- [ ] パターンマッチの完全実装

### 📝 TODO (Phase 1 後半)
- [ ] `1-2-typer-implementation.md` で求められる Typed AST/型推論テストのひな型作成
- [ ] `1-3`〜`1-5` のタスクに合わせた Core IR/LLVM/ランタイム連携の stub 追加
- [ ] 計測フック（`1-6`, `1-7`）の連携手順を記録

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
