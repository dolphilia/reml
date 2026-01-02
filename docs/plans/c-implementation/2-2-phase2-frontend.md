# フェーズ 2: フロントエンドの基礎 (Lexer & Parser)

このフェーズでは、Reml ソースコードを C 言語の有効な抽象構文木 (AST) に変換することを目指します。

## 2.1 字句解析 (Lexer)
- **ライブラリ**: `re2c` (Lexer Generator) + `utf8proc` (検証)。
- **仕様**: `docs/spec/1-5-formal-grammar-bnf.md`, `docs/spec/2-3-lexer.md`。
- **タスク**:
  1.  トークン型の定義 (`include/reml/lexer/token.h`)。
  2.  `lexer.re` (re2c ソースファイル) の実装。
      - `lexer.c` を生成するためのビルドルールを CMake に構築。
  3.  **Unicode 処理**:
      - `utf8proc` を使用して UTF-8 の妥当性を検証。
      - ソース位置の追跡 (バイト位置, 行, 列)。
  4.  **エラー処理**: `LexError` 構造体の作成。
- **成果物**: ファイル全体をトークン化できる `lex_next_token()` 関数。

## 2.2 AST 定義
- **戦略**: Tagged Unions (struct with `enum` kind)。
- **データ構造**: ノードリスト用の `utarray` / `utlist`。
- **タスク**:
  1.  `Expr`, `Stmt`, `Decl`, `Type` 構造体を `include/reml/ast/` に定義。
  2.  AST コンストラクタ/デストラクタの実装 (準備できていればカスタムアリーナ、なければ `malloc`/`free`)。
  3.  デバッグ用の `ASTPrinter` (実装には `yyjson` の JSON 形式か S 式テキストを使用)。

## 2.3 解析戦略 (Parsing)
- **アプローチ**: 既存の Rust/OCaml 実装に厳密に合わせるため、再帰下降法 (手書き) を採用。
- **パーサーの状態**:
  - 現在のトークン、先読みトークン。
  - エラーレポーターコンテキスト。
  - メモリアリーナ。
- **タスク**:
  1.  特定の解析関数の実装: `parse_expr`, `parse_stmt`, `parse_decl`.
  2.  演算子の優先順位処理 (Pratt Parsing または Precedence Climbing)。
  3.  **エラー回復**: パニックモード (`;` や `}` などの境界で同期)。
- **仕様**: `docs/spec/2-1-parser-type.md`。

## 2.4 設定とマニフェストの解析
- **ライブラリ**: `tomlc99`。
- **タスク**: `reml.toml` を解析してパッケージメタデータとビルド設定を抽出する。

## 2.5 検証
- **ユニットテスト**:
  - Lexer テスト: 入力文字列 -> 期待されるトークン列。
  - Parser テスト: 入力コード -> 期待される AST 構造 (AST Printer で検証)。
- **リファレンス**: 既存の `compiler/rust/tests` または `examples/spec_core` のテストケースをゴールデンスタンダードとして使用。

## チェックリスト
- [ ] `lexer.re` がコンパイルされ、基本的な構文を処理できる。
- [ ] `tokens` ダンプ CLI コマンドの実装 (`reml internal lex <file>`)。
- [ ] Core 仕様に対する AST 定義の完了。
- [ ] 基本的な式と宣言に対する `parser` の実装。
- [ ] `ast` ダンプ CLI コマンドの実装 (`reml internal parse <file>`)。
- [ ] `reml.toml` の解析が動作する。
