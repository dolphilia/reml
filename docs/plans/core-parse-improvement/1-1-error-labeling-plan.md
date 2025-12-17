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
### Step 0: 期待集合の「構造化モデル」を前提にする（現状把握）
ラベル運用を決める前に、期待集合が `Expectation::Rule` 等の型で保持されること（`docs/spec/2-5-error.md`）を踏まえ、どの情報を「ラベル」として出すべきかを整理する。

- 参照すべき仕様
  - `docs/spec/2-1-parser-type.md`（`label`、`ParseError.context`、`ParserId`）
  - `docs/spec/2-2-core-combinator.md`（`rule/label` の役割、`expect` 糖衣）
  - `docs/spec/2-5-error.md`（`Expectation::{Token,Rule,Class,...}` と B-6/B-7 の縮約・整形）

### Step 1: 仕様・用語の整理（rule/label/context/expected の齟齬を潰す）
- 次の問いに「仕様上の答え」が一意になるよう読み合わせる
  - `label("x", p)` は **期待集合の差し替え**なのか、**文脈スタック（context）への push**も含むのか
  - `rule(name, p)` は **識別（ParserId/Packrat/trace）**が主なのか、期待表示にも出すのか
  - `ParseError.context` の順序（外側→内側）が、`Err.pretty` の表示方針と一致しているか
- 追記が必要な場合の候補
  - `docs/spec/2-5-error.md`: `context` と `expected_summary.context_note` の関係（B-7）
  - `docs/spec/2-2-core-combinator.md`: `rule` と `label` の役割分担を「運用指針」として追記
- 用語追加が必要なら `docs/spec/0-2-glossary.md` にドラフトを用意する
  - 例: *Label（期待ラベル）* / *Rule（安定 ID を持つルール名）* / *Context（失敗に至る文脈）*

### Step 2: 推奨ラベル語彙（“揺れ” を減らす）を定義する
ラベルが揺れると回帰が不安定になるため、まず「推奨語彙セット」を決める。

- 推奨ラベルの最小セット案（暫定）
  - `expression`
  - `pattern`
  - `statement`
  - `identifier`
  - `number`
  - `string`
  - `type`
- 付与方針（ノイズ抑制）
  - 低レベル（`char` や `string("(")`）には原則ラベルを付けず、構文単位（atom/expression 等）へ付ける
  - `expect(name, p)`（= `label` + `cut`）を「欠落が致命的なトークン」（例: `)`、`then`）に使い、ラベルと cut を同時に安定させる
- 仕様への反映方法（候補）
  - `docs/spec/2-2-core-combinator.md` に「推奨ラベル語彙（短い表）」を追加し、サンプルと対応付ける
  - `docs/spec/2-5-error.md` の B-6（縮約）で `Rule("expression")` の扱い例を追加する

### Step 3: サンプルと回帰（ラベルの効果を固定）
- サンプル（候補）
  - `examples/spec_core/chapter2/parser_core/` に「ラベルなし」と「ラベルあり」の差分が出る最小例を追加
  - Cut と併用する例（`label("expression", cut(expr))` 等）を含め、「親ルールの曖昧な期待」が残らないことを示す
- 回帰（候補）
  - 計画起点 ID: `CP-WS2-001`（期待集合が概念ラベル中心になる）
  - 期待出力では、少なくとも次を固定する
    - 最初のエラー位置（Span）
    - `expected_summary.alternatives` に `Rule("expression")` 等が含まれること（トークン列だけにならない）
- 診断キーとの関係
  - ラベルは「期待集合の内容」であり診断キー自体は変えない方針を基本とする（キー運用は `docs/spec/3-6-core-diagnostics-audit.md` を参照）

## 成果物（ドラフト）
- ドキュメント追記案（必要なら）:
  - `docs/spec/2-2-core-combinator.md`（ラベル付け指針）
  - `docs/spec/2-5-error.md`（期待集合・文脈表示）
- サンプル（候補）:
  - `examples/spec_core/chapter2/parser_core/`（ラベルの効果が見える小入力）

## リスクと緩和
- ラベル運用が不統一だと、期待が逆に分かりにくくなる  
  → `docs/spec/2-2-core-combinator.md` に「標準ラベル集合（推奨語彙）」を設ける案を検討
