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

### 2.1.1 字句仕様の分解 (最小セット)
`docs/spec/2-3-lexer.md` に沿って、以下のカテゴリごとに対応範囲とテストを明確化する。

- **空白・改行・コメント**: Unicode White_Space / `NL` の正規化、行・列更新。
- **識別子/キーワード**: UAX #31 準拠 (XID_Start/XID_Continue + `_`)、境界判定。
- **数値リテラル**: 10/2/8/16 進、`_` 区切り、`0x` などのプレフィックス。
- **文字列/文字リテラル**: エスケープ、raw、複数行、Unicode スカラ値検証。
- **演算子/区切り記号**: `docs/spec/1-5-formal-grammar-bnf.md` の終端記号を網羅。

### 2.1.2 診断とエラーハンドリング
- **期待集合 + 最遠位置 + ラベル** を保持する構造 (`LexError` に追加)。
- 数値リテラルの範囲エラーは `parser.number.overflow` / `parser.number.invalid` に変換する前提で情報を保持。
- エラーから `Diagnostic` への変換仕様は Parser 側と共有。

## 2.2 AST 定義
- **戦略**: Tagged Unions (struct with `enum` kind)。
- **データ構造**: ノードリスト用の `utarray` / `utlist`。
- **タスク**:
  1.  `Expr`, `Stmt`, `Decl`, `Type` 構造体を `include/reml/ast/` に定義。
  2.  AST コンストラクタ/デストラクタの実装 (準備できていればカスタムアリーナ、なければ `malloc`/`free`)。
  3.  デバッグ用の `ASTPrinter` (実装には `yyjson` の JSON 形式か S 式テキストを使用)。

### 2.2.1 AST 最小スコープ
`docs/spec/1-5-formal-grammar-bnf.md` のフェーズ 2 必須要素として、最低限以下を含める。

- **トップレベル**: `CompilationUnit`, `ModuleHeader`, `UseDecl`, `PubDecl`, `Attrs`, `Attribute`。
- **式/文**: `Expr`, `Stmt`, `Block`, `ReturnStmt`, `AssignStmt`。
- **リテラル/識別子**: `Literal`, `Ident`, `StringLiteral`, `IntLiteral`, `FloatLiteral`。
- **パターン**: `Pattern` (変数束縛/タプル/レコード/ワイルドカード/リテラルパターン)。

#### 2.2.1.1 Pattern 最小構成
- 変数束縛: `Ident`
- ワイルドカード: `_`
- リテラルパターン: `IntLiteral`, `FloatLiteral`, `StringLiteral`, `CharLiteral`, `BoolLiteral`
- タプルパターン: `("(" Pattern "," Pattern { "," Pattern } [","] ")")`
- レコードパターン: `"{" FieldPattern { "," FieldPattern } [","] "}"`
  - `FieldPattern`: `Ident [":" Pattern]` (省略時はフィールド名を束縛)

#### 2.2.1.2 Literal 最小構成
- 数値: `IntLiteral`, `FloatLiteral`
- 文字列: `StringLiteral` (raw/multiline の区別は token に保持)
- 文字: `CharLiteral`
- 真偽値: `BoolLiteral` (`true`/`false`)

#### 2.2.1.3 Primary 最小構成
- `Ident` / `Literal`
- 括弧式: `"(" Expr ")"`
- ブロック: `Block`
- 条件: `IfExpr`
- 分岐: `MatchExpr`

### 2.2.2 Span 付与方針
全ノードに `Span` を持たせるか、`Spanned<T>` ラッパーで表現するかを決定し、
Parser/Diagnostics で共有する。

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

### 2.3.1 エントリポイントと優先順位
- エントリポイントは `CompilationUnit` 固定とし、`docs/spec/1-5-formal-grammar-bnf.md` に合わせる。
- 演算子の優先順位テーブルを計画書内に固定し、Rust 実装との差分を監視する。

#### 2.3.1.1 演算子優先順位テーブル (Rust 実装準拠)
`compiler/rust/frontend/src/parser/mod.rs` の式パーサーに合わせる。

| 優先度 (高 -> 低) | 非終端 | 演算子/構文 | 結合性 |
| --- | --- | --- | --- |
| 6 | PostfixExpr | `.`, `( )`, `?` | 左結合 |
| 5 | UnaryExpr | `-`, `!`, `async`, `await`, `rec` | 右結合 |
| 4 | MulExpr | `*`, `/`, `%` | 左結合 |
| 3 | AddExpr | `+`, `-` | 左結合 |
| 2 | RangeExpr | `..` | 左結合 |
| 1 | CmpExpr | `<`, `<=`, `>`, `>=`, `==`, `!=` | 左結合 |
| 0 | PipeExpr | `|>` | 左結合 |

補足:
- `CallExpr` は `PostfixExpr` の `("(" Args? ")")` で表現する。
- 現行 Rust 版は `^` (Pow) と `&&` / `||` を構文として扱っていないため、C 実装では **保留** とする。

#### 2.3.1.2 仕様との差分 (RangeExpr)
- `docs/spec/1-5-formal-grammar-bnf.md` の式文法には `..` の `RangeExpr` が存在しない。
- Rust 実装は `AddExpr` と `CmpExpr` の間に `RangeExpr (..)` を追加している。
- `..` はパターン文法では `RangePattern` として仕様化されているが、式側は未定義。

対応方針:
- C 実装は **仕様優先** とし、`RangeExpr` は導入しない。
- Rust 実装準拠を選ぶ場合は、仕様側に `RangeExpr` の追加を提案する。

### 2.3.2 Parser 診断
- Lexer と同様に **期待集合 + 最遠位置 + ラベル** を保持するエラー構造を採用。
- `Span` を全 AST へ伝搬し、Diagnostics の位置情報を統一。

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
