# WS2: Error Labeling（文脈・期待集合）計画（ドラフト）

## 背景と狙い
調査メモ `docs/notes/core-parse-improvement-survey.md` が推奨する通り、Megaparsec の `<?>` に相当する **ラベル付け**は、DSL 利用者にとって「どこを直せばよいか」を即座に理解できる診断へ直結する。

Reml 仕様でも `docs/spec/2-1-parser-type.md` に `label("x", p)` があり、`ParseError.context` へ文脈を保持する契約がある。
本ワークストリームは、これを「使いどころが明確な API/慣習」に落とし込む。

## 目標（ドラフト）
- エラーの期待表示が `"expected expression"` のような **概念ラベル**中心になる
- Cut/Commit と組み合わせても、期待集合が「最も分かりやすい形」で出る

## 設計論点（ドラフト）
- `label` は期待名（expected）を差し替えるだけでなく、`context` へ積むか（仕様準拠の確認が必要）
- `rule(name, p)` と `label(name, p)` の役割分担（安定 ID、観測/プロファイルでの識別、IDE 表示）
- 期待集合が「トークン」か「ラベル」か混在する場合の統合ルール

## タスク分割（ドラフト）
### Step 1: 仕様・用語の整理
- `label` / `rule` / `expected set` / `context` の用語を `docs/spec/2-1-parser-type.md` と `docs/spec/2-5-error.md` で読み合わせ、定義の齟齬をなくす
- `docs/spec/0-2-glossary.md` に追加が必要ならドラフト案を用意する（"label"/"cut" 等）

### Step 2: ラベリング指針（ガイドライン）
- 最小パターン:
  - 式: `label("expression", ...)`
  - 識別子: `label("identifier", ...)`
  - リテラル: `label("string literal", ...)` など
- 過剰ラベル（ノイズ）を避ける指針（例: 低レベル `char` にラベルを貼りすぎない）

### Step 3: サンプルと回帰
- ラベル付与の有無で期待表示が変わるサンプルを追加
- `cut` と併用した時に「親ルールではなく子ルールの期待」が出ることを固定

## 成果物（ドラフト）
- ドキュメント追記案（必要なら）:
  - `docs/spec/2-2-core-combinator.md`（ラベル付け指針）
  - `docs/spec/2-5-error.md`（期待集合・文脈表示）
- サンプル（候補）:
  - `examples/spec_core/chapter2/parser_core/`（ラベルの効果が見える小入力）

## リスクと緩和
- ラベル運用が不統一だと、期待が逆に分かりにくくなる  
  → `docs/spec/2-2-core-combinator.md` に「標準ラベル集合（推奨語彙）」を設ける案を検討

