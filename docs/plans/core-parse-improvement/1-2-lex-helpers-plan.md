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
### Step 1: 仕様の棚卸し
- `docs/spec/2-3-lexer.md` に既に存在する API と、サンプル側の自前実装を比較し、欠落ヘルパを列挙する
- 既存の `RunConfig.extensions["lex"]`（プロファイル）との接合点を明確化する

### Step 2: サンプル整備（自前定義の削減）
- `examples/language-impl-comparison/reml/basic_interpreter_combinator.reml` の自前 `lexeme/symbol` を、可能な範囲で `Core.Parse.Lex` へ寄せる（破壊的変更は避ける）
- 小さな DSL（例: mini JSON/INI）を追加し、`Lex` だけで書けることを示す

### Step 3: 回帰・診断
- 空白・コメント・改行の差で壊れやすい入力を回帰として固定する
- `label` を使い、字句レイヤ由来の失敗が「読みやすい期待」として出ることを確認する（WS2 と連動）

## リスクと緩和
- lexer 的処理を parser 側に寄せすぎると、柔軟性を損ねる  
  → `Core.Parse.Lex` は **プリセット**として提供し、強制しない（opt-in/差し替え可能）

