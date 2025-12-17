# WS3: Lex Helpers（scannerless ヘルパ）計画（ドラフト）

## 背景と狙い
調査メモ `docs/notes/core-parse-improvement-survey.md` では、DSL 実装で頻出する字句処理を `Core.Parse` 側で支援することを推奨している。
特に `symbol/lexeme/integer/stringLiteral` などは、サンプル側の自前実装を減らし、エラー表示と空白処理の一貫性を高める。

## 参照
- `docs/spec/2-3-lexer.md`（字句ヘルパ仕様）
- `docs/spec/2-2-core-combinator.md`（期待/診断の統一方針）
- `docs/plans/bootstrap-roadmap/4-1-core-parse-combinator-plan-v2.md`（autoWhitespace/Layout と Lex ブリッジ）

## 目標（ドラフト）
- サンプル DSL の多くが `Core.Parse.Lex` の標準ヘルパだけで書ける
- 空白・コメント・レイアウト（将来）を `RunConfig` のプロファイルで切替できる
- 期待/ラベルが字句レイヤでも破綻しない（"expected identifier" 等）

## 提供するヘルパ候補（ドラフト）
- `lexeme(p)`: `p` 成功後に空白/コメント（trivia）を処理
- `symbol(text)`: `lexeme(string(text))` の糖衣（固定文字列）
- `keyword(text)`: `symbol` + キーワード境界（識別子の一部にならない）
- `integer` / `float`: 数値リテラル（符号、桁区切り、指数等は段階導入）
- `stringLiteral`: エスケープを含む文字列（エラー位置の精度が重要）
- `identifier`: Unicode 安全性（`docs/spec/3-3-core-text-unicode.md`）を踏まえた識別子

## タスク分割（ドラフト）
### Step 0: Lex の「提供範囲」を決める（scannerless のプリセットとして整理）
Lex ヘルパは “字句と構文の完全分離” を強制しない一方で、最低限の標準化がないとサンプルが自前実装だらけになる。
まず `Core.Parse.Lex` を「プリセット」としてどこまで持つかを決める。

- 参照すべき仕様
  - `docs/spec/2-3-lexer.md`（API 一覧、代表レシピ、`label`/`expect` の流儀）
  - `docs/spec/2-2-core-combinator.md`（前後空白インターフェイス、`lexeme/symbol` と `with_space`）
  - `docs/spec/2-5-error.md`（`Expectation::{Keyword,Class,Rule}` の使い分け）

### Step 1: 仕様の棚卸し（既存 API と “足りないもの” を表にする）
- `docs/spec/2-3-lexer.md` の API と、既存サンプルの「自前定義」を突き合わせる
  - 対象例（差分の出やすいもの）
    - `examples/language-impl-comparison/reml/basic_interpreter_combinator.reml`
    - `examples/language-impl-comparison/reml/sql_parser.reml`
    - `examples/language-impl-comparison/reml/toml_parser.reml`
    - `examples/language-impl-comparison/reml/yaml_parser.reml`
- 産物（ドキュメント化）
  - 「サンプルが自前実装しているヘルパ一覧」→「Lex 既存 API」→「不足/過剰/重複」の対応表（本計画へ追記、または `docs/notes/core-parse-api-evolution.md` に記録）
- `RunConfig.extensions["lex"]` との接合点を明確化する
  - `space`/`space_id` の共有（`docs/spec/2-2-core-combinator.md` B-1）
  - 将来の `profile`/`layout_profile`（Phase9 ドラフト）との整合（既定フォールバックの扱い）

### Step 2: ヘルパ群を “利用者が迷わない形” に整備する（仕様とサンプルの一致）
ここでは実装追加そのものではなく、計画として「何を揃えると実用になるか」を具体化する。

- 最低限揃えるヘルパ（優先順の目安）
  1) `lexeme` / `symbol` / `keyword`（空白・コメント処理の共有）
  2) `identifier`（予約語衝突と Unicode 安全性）
  3) `number`（`int`/`float` とオーバーフロー診断の指針）
  4) `stringLiteral`（エスケープと失敗位置）
- エラー品質の統一ルール
  - `identifier` は `label("identifier")` を標準にする（`docs/spec/2-3-lexer.md` の「代表的なレシピ: 識別子/キーワード」）
  - `number/string` も同様に `label("number")` / `label("string")` を標準にし、WS2 の推奨語彙に合わせる
  - キーワード境界は `Expectation::Keyword` を優先（`Rule` に頼りすぎない）

### Step 3: サンプル整備（自前定義の削減）と回帰への接続
- サンプル整備の方針
  - 自前 `lexeme/symbol` は、可能な範囲で `Core.Parse.Lex` の `lexeme/symbol/keyword` へ寄せる
  - ただし、既存サンプルの “教材としての意図” を壊さないよう、破壊的変更は避ける（差分が大きい場合は新規サンプルを追加する）
- 新規サンプル（候補）
  - `examples/spec_core/chapter2/parser_core/` に「LexPack を使った最小 DSL」（識別子 + 数値 + `=` + `;`）を追加し、空白/コメント混在でも安定することを示す
- 回帰（候補）
  - 計画起点 ID: `CP-WS3-001`（空白・コメント・改行の差で壊れない）
  - 期待出力で固定する要素
    - whitespace/comment を含む入力でも同じ AST/同じ診断になる
    - lex 由来の失敗が `label` 付きの期待（例: `identifier`）として出る（WS2 と連動）

## リスクと緩和
- lexer 的処理を parser 側に寄せすぎると、柔軟性を損ねる  
  → `Core.Parse.Lex` は **プリセット**として提供し、強制しない（opt-in/差し替え可能）
