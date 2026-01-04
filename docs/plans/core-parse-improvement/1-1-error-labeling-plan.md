# WS2: Error Labeling（文脈・期待集合）計画

## 背景と狙い
調査メモ `docs/notes/parser/core-parse-improvement-survey.md` が推奨する通り、Megaparsec の `<?>` に相当する **ラベル付け**は、DSL 利用者にとって「どこを直せばよいか」を即座に理解できる診断へ直結する。

Reml 仕様でも `docs/spec/2-1-parser-type.md` に `label("x", p)` があり、`ParseError.context` へ文脈を保持する契約がある。
本ワークストリームは、これを「使いどころが明確な API/慣習」に落とし込む。

## 目標
- エラーの期待表示が `"expected expression"` のような **概念ラベル**中心になる
- Cut/Commit と組み合わせても、期待集合が「最も分かりやすい形」で出る

## 設計論点
- `label` は期待名（expected）を差し替えるだけでなく、`context` へ積むか（仕様準拠の確認が必要）
- `rule(name, p)` と `label(name, p)` の役割分担（安定 ID、観測/プロファイルでの識別、IDE 表示）
- 期待集合が「トークン」か「ラベル」か混在する場合の統合ルール

## 進捗（この計画の状態が分かるチェックリスト）

> WS1 と同様、各 Step の出口を明示し、仕様・サンプル・回帰の揃い具合を追えるようにする。

- [x] Step 0: 期待集合の「構造化モデル」を前提にする（現状把握）
  - [x] `Expectation` 型と表示サマリ（`ExpectationSummary`）の現行仕様を確認
  - [x] `context` の積み方（`then` 後段で追加、外側→内側順）と整形手順（B-7）を整理
  - [x] Rule/Token の縮約規則（B-6）と Cut との相互作用（B-5）を Step1 への論点として抽出
- [x] Step 1: 仕様・用語の整理（rule/label/context/expected の齟齬を潰す）
  - [x] `label` が期待差し替えと `context` への push を両立することを 2-2/2-5 に明記
  - [x] `rule` は ParserId/トレース用の安定名を提供し、期待集合は差し替えない旨を明記
  - [x] `context` の順序（外側→内側）と `then/andThen` での付与規則（B-4）を確認
- [x] Step 2: 推奨ラベル語彙（“揺れ” を減らす）を定義する
  - [x] 最小セット（expression/pattern/statement/identifier/number/string/type）を 2-2 に掲載
  - [x] 付与ポリシー（構文単位へ label、致命的トークンには expect=label+cut）を明文化
- [x] Step 3: サンプルと回帰（ラベルの効果を固定）
  - [x] サンプル: `examples/spec_core/chapter2/parser_core/core-parse-label-vs-token-*.reml` を追加し、`+` 右項欠落でラベル有無の期待集合差を比較
  - [x] 回帰: `expected/spec_core/chapter2/parser_core/core-parse-label-vs-token-*.diagnostic.json` に `Rule("expression")` を含む期待集合（with-label）とトークン中心（no-label）を固定（`CP-WS2-001`）

## タスク分割
### Step 0: 期待集合の「構造化モデル」を前提にする（現状把握）
ラベル運用を決める前に、期待集合が `Expectation::Rule` 等の型で保持されること（`docs/spec/2-5-error.md`）を踏まえ、どの情報を「ラベル」として出すべきかを整理する。

- 参照すべき仕様
  - `docs/spec/2-1-parser-type.md`（`label`、`ParseError.context`、`ParserId`）
  - `docs/spec/2-2-core-combinator.md`（`rule/label` の役割、`expect` 糖衣）
  - `docs/spec/2-5-error.md`（`Expectation::{Token,Rule,Class,...}` と B-6/B-7 の縮約・整形）

#### 読み合わせ結果（現行仕様で既に定義されている構造）
- 期待集合は構造化された列挙型（`Expectation::{Token, Keyword, Rule, Eof, Not, Class, Custom}`）として保持し、縮約・整形は `ExpectationSummary` で行う（`docs/spec/2-5-error.md` A, B-6, B-7）。
- 表示時は具体トークン > 文字クラス > ルール名の優先で `alternatives` を並べ、`Rule("expression")` があっても `Token(")")` があれば具体トークンを前面に出す（同 B-6/B-7）。
- `ParseError.context` は外側→内側順で `rule/label` 名を積み、`then/andThen` の後段失敗時に付与する（`docs/spec/2-5-error.md` A, B-4）。`expected_summary.context_note` と結合して「`+` の後に式」などの文脈文を作る（同 B-7）。
- Cut を通過すると期待集合を再初期化し、親の曖昧な期待を持ち越さない（`docs/spec/2-5-error.md` B-5）。Cut 以降の期待をどうラベルで置き換えるかが運用上の鍵になる。
- `rule(name, p)` は ParserId 付与（Packrat/trace 用）と診断文脈の両方で使われ、`label("x", p)` は期待名の差し替えとして記述されている（`docs/spec/2-1-parser-type.md` E, F）。`expect = label + cut` の糖衣が `docs/spec/2-2-core-combinator.md` C で定義済み。

#### ギャップと Step1 へ渡す論点（現状で未整理な箇所）
- `label` が期待集合の差し替えに限定されるのか、`context` への push も伴うのかを仕様上で明示する必要がある（`ParseError.context` には `rule/label` とあるが、2-1 では期待差し替えのみを強調している）。
- `Rule` と `Token/Keyword/Class` が混在した場合に「概念ラベル」がどの程度表示に残るか（B-6 で具体優先のため、ラベルをどこで入れるかが重要）。Cut 境界で期待を再初期化する際に、Rule ラベルを残す運用を決める必要がある。
- `rule(name, p)` の名前が期待表示に出るか否か、`label` との役割分担（安定 ID vs 期待名）を整理しないと、`expected_summary` と `context` の両方で揺れが生じる。
- `ExpectationSummary.message_key` / `locale_args` を利用する翻訳運用と、`Rule("expression")` 等のラベル出力の境界を決める必要がある（LSP/CLI の humanized フォールバックとの整合を確認する）。

### Step 1: 仕様・用語の整理（rule/label/context/expected の齟齬を潰す）
- 次の問いに「仕様上の答え」が一意になるよう読み合わせる
  - `label("x", p)` は **期待集合の差し替え**なのか、**文脈スタック（context）への push**も含むのか
  - `rule(name, p)` は **識別（ParserId/Packrat/trace）**が主なのか、期待表示にも出すのか
  - `ParseError.context` の順序（外側→内側）が、`Err.pretty` の表示方針と一致しているか
- 追記が必要な場合の対象
  - `docs/spec/2-5-error.md`: `context` と `expected_summary.context_note` の関係（B-7）
  - `docs/spec/2-2-core-combinator.md`: `rule` と `label` の役割分担を「運用指針」として追記
- 用語追加が必要なら `docs/spec/0-2-glossary.md` に追記案を用意する
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
- 仕様への反映方法
  - `docs/spec/2-2-core-combinator.md` に「推奨ラベル語彙（短い表）」を追加し、サンプルと対応付ける
  - `docs/spec/2-5-error.md` の B-6（縮約）で `Rule("expression")` の扱い例を追加する

### Step 3: サンプルと回帰（ラベルの効果を固定）
- サンプル
  - `examples/spec_core/chapter2/parser_core/` に「ラベルなし」と「ラベルあり」の差分が出る最小例を追加
  - Cut と併用する例（`label("expression", cut(expr))` 等）を含め、「親ルールの曖昧な期待」が残らないことを示す
- 回帰
  - 計画起点 ID: `CP-WS2-001`（期待集合が概念ラベル中心になる）
  - 期待出力では、少なくとも次を固定する
    - 最初のエラー位置（Span）
    - `expected_summary.alternatives` に `Rule("expression")` 等が含まれること（トークン列だけにならない）
- 診断キーとの関係
  - ラベルは「期待集合の内容」であり診断キー自体は変えない方針を基本とする（キー運用は `docs/spec/3-6-core-diagnostics-audit.md` を参照）

## 成果物
- ドキュメント追記（必要な場合）:
  - `docs/spec/2-2-core-combinator.md`（ラベル付け指針）
  - `docs/spec/2-5-error.md`（期待集合・文脈表示）
- サンプル:
  - `examples/spec_core/chapter2/parser_core/`（ラベルの効果が見える小入力）

## リスクと緩和
- ラベル運用が不統一だと、期待が逆に分かりにくくなる  
  → `docs/spec/2-2-core-combinator.md` に「標準ラベル集合（推奨語彙）」を設ける案を検討
