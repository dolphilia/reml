# WS3: Lex Helpers（scannerless ヘルパ）計画

## 背景と狙い
調査メモ `docs/notes/core-parse-improvement-survey.md` では、DSL 実装で頻出する字句処理を `Core.Parse` 側で支援することを推奨している。
特に `symbol/lexeme/integer/stringLiteral` などは、サンプル側の自前実装を減らし、エラー表示と空白処理の一貫性を高める。

## 参照
- `docs/spec/2-3-lexer.md`（字句ヘルパ仕様）
- `docs/spec/2-2-core-combinator.md`（期待/診断の統一方針）
- `docs/plans/bootstrap-roadmap/4-1-core-parse-combinator-plan-v2.md`（autoWhitespace/Layout と Lex ブリッジ）

## 目標
- サンプル DSL の多くが `Core.Parse.Lex` の標準ヘルパだけで書ける
- 空白・コメント・レイアウト（将来）を `RunConfig` のプロファイルで切替できる
- 期待/ラベルが字句レイヤでも破綻しない（"expected identifier" 等）

## 進捗状況
- Step0: 完了（プリセット範囲と RunConfig 連携の方針を確定）
- Step1: 進行中（自前ヘルパとの対応表と RunConfig/期待ラベルの論点を整理済み、棚卸し継続）
- Step2: 未着手
- Step3: 未着手

## 提供するヘルパ
- `lexeme(p)`: `p` 成功後に空白/コメント（trivia）を処理
- `symbol(text)`: `lexeme(string(text))` の糖衣（固定文字列）
- `keyword(text)`: `symbol` + キーワード境界（識別子の一部にならない）
- `integer` / `float`: 数値リテラル（符号、桁区切り、指数等は段階導入）
- `stringLiteral`: エスケープを含む文字列（エラー位置の精度が重要）
- `identifier`: Unicode 安全性（`docs/spec/3-3-core-text-unicode.md`）を踏まえた識別子

## タスク分割
### Step 0: Lex の「提供範囲」を決める（scannerless のプリセットとして整理）
Lex ヘルパは “字句と構文の完全分離” を強制しない一方で、最低限の標準化がないとサンプルが自前実装だらけになる。
まず `Core.Parse.Lex` を「プリセット」としてどこまで持つかを決める。

- 参照すべき仕様
  - `docs/spec/2-3-lexer.md`（API 一覧、代表レシピ、`label`/`expect` の流儀）
  - `docs/spec/2-2-core-combinator.md`（前後空白インターフェイス、`lexeme/symbol` と `with_space`）
  - `docs/spec/2-5-error.md`（`Expectation::{Keyword,Class,Rule}` の使い分け）

#### 読み合わせ結果（現行仕様で既に握れている点）
- Lex 側は **空白/コメントを受け取る API**（`lexeme/symbol/keyword` 等）が核で、`with_space` / `autoWhitespace` が `RunConfig.extensions["lex"].profile/space_id/layout_profile` を検出して二重スキップを防ぐ（`docs/spec/2-2-core-combinator.md` B-1/B-2）。
- 期待集合は `Expectation::{Keyword,Class,Rule}` を使い分け、キーワード境界は `keyword(space, kw)` で `Expectation::Keyword` を優先、識別子は `label("identifier")` で `Rule/Class` を提示する流儀が `docs/spec/2-3-lexer.md` J/L-1 と `docs/spec/2-5-error.md` で明示されている。
- `ConfigTriviaProfile`（`docs/spec/2-3-lexer.md` G-1）と `LayoutProfile`（同 H-2）は **profile として RunConfig 共有可能**な設計になっており、`lex_pack` 例（同 L-4）が「プリセット化した空白・識別子・symbol」をエントリポイントで束ねる前提を与えている。
- 安全性は UAX #31/#29 に沿った `IdentifierProfile`（NFC/Bidi/Confusable 警告付き）が既定で、`RunConfig.extensions["lex"].identifier_profile` で ASCII 互換への切替も許容される（`docs/spec/2-3-lexer.md` D-1）。

#### プリセットとして提供する範囲（決定）
- **Core 基本セット（LexPreset::core）**: `whitespace/commentLine/commentBlock/skipMany` を前提にした `space` と、`lexeme`/`symbol`/`keyword`/`leading`/`trim`/`token` を **scannerless 前提の糖衣**として標準提供。`with_space`/`autoWhitespace` 経由で `space_id` を共有し、プリセットが無い箇所でも空白を自動注入する。
- **識別子・予約語（LexPreset::id）**: `identifier(DefaultId).label("identifier")`、`keyword`（境界判定つき）、`reserved(profile, set)` をプリセットに含める。`IdentifierProfile` の既定（NFC/Bidi 禁止・Confusable 警告）を安全側デフォルトとし、ASCII 互換は opt-in（`RunConfig.extensions["lex"].identifier_profile` 経由）。
- **数値・文字列（LexPreset::literal）**: `int10`/`intAuto`/`float`/`stringLit`/`stringRaw`/`stringMultiline`/`charLit` を **原文保持 + 後段で値化**する構成でプリセット化し、`label("number")` / `label("string")` を標準付与。オーバーフロー・不正値は 2-3 §E-1 の診断変換を推奨し、`Expectation::Class("number")` で統一。
- **トリビア・レイアウト（LexPreset::profiled）**: `ConfigTriviaProfile` ベースの `config_trivia/config_lexeme/config_symbol` を「設定 DSL 互換」のプリセットとして扱い、`RunConfig.extensions["lex"].profile`/`layout_profile` と連動させる。`LayoutProfile` は Phase 9 の opt-in として保持し、既定は無効（空白のみ）。
- **安全オプション（LexPreset::safety）**: `forbidBidiControls`/`requireNfc`/`warnConfusable` を **オンにしても破壊的にならない範囲**でプリセットに束ね、LSP/CLI が RunConfig から安全設定を復元できるよう `extensions["lex"]` にパラメータを保持する。

#### 境界と非プリセット（今回の計画で強制しないもの）
- 言語固有のトークン化（SQL の引用識別子、TOML 日付、シェルヒアドキュメント等）は **プリセット外**とし、必要なら Phase 9 以降の `profile` 拡張やサンプル側カスタムで扱う。
- Layout オフサイドのトークン生成は opt-in のまま保持し、WS3 で扱うのは **空白・コメント・識別子・数値・文字列の scannerless ヘルパ**に限定する。Layout の回帰は `CH2-PARSE-901/902` へ接続するが、Lex プリセットの必須要件には含めない。
- 実装非依存の範囲に留め、OCaml/Rust の実装差異（`lexer.mll` 由来メトリクス等）は計画書では固定しない。必要なら `docs/plans/bootstrap-roadmap/4-1-core-parse-combinator-plan-v2.md` でのブリッジ方針に委譲する。

#### TODO（Step1 以降への引き継ぎ）
- サンプルが自前定義している `lexeme/symbol/identifier/number/string` と本プリセットの対応表を作成し、`ConfigTriviaProfile` や `identifier_profile` で埋まる差分を洗い出す（Step1 の棚卸し）。
- `LexPreset::{core,id,literal,profiled,safety}` を `docs/spec/2-3-lexer.md` のチェックリストと対応付け、`RunConfig.extensions["lex"]` で共有するキー名を明示する（`space_id`/`profile`/`identifier_profile`/`layout_profile`/`safety`）。
- 期待表示の統一（`Keyword`/`Class`/`Rule` の使い分け）を `docs/spec/2-5-error.md` の B-6/B-7 に沿って確認し、プリセットが自動付与する `label` の一覧（identifier/number/string）を WS2 の推奨語彙に合わせる。

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
  - 将来の `profile`/`layout_profile`（Phase9 の拡張項目）との整合（既定フォールバックの扱い）

#### Step1 対応表（自前ヘルパ vs LexPreset とギャップ）

| サンプル | 自前ヘルパ/挙動 | LexPreset 対応 | 不足/揺れ |
| --- | --- | --- | --- |
| `basic_interpreter_combinator.reml` | `sc = whitespace` のみ（コメントなし）。`lexeme/symbol/keyword` を薄いラッパーで再定義。`identifier` は `Lex.identifier()` を rule で包むのみ。`number` は `Lex.float().or(Lex.integer())` + `parseF64` で文字列失敗時に `Parse.fail("…解釈できません")`。 | `core`（lexeme/symbol/keyword）、`id`（identifier）、`literal`（int/float/string） | 期待ラベル未付与（`identifier/number/string` を `label` していない）。エラーメッセージが自由文で `Expectation` を失う。`RunConfig.extensions["lex"].space_id` 未共有。コメントスキップのプリセット化なし（`ConfigTriviaProfile` を使えば統一可能）。 |
| `sql_parser.reml` | `sc = whitespace + commentLine(\"--\") + commentBlock(/* */, nested=false)`。`lexeme/sym` 再定義。`keyword` を `string_case_insensitive` + `notFollowedBy(alnum)` で自前境界。`identifier` は `satisfy XID_start + takeWhile XID_continue` → 予約語手動チェック → `Parse.fail("予約語…")`。 | `core`（lexeme/symbol）、部分的に `id`（識別子） | `keyword` に大文字小文字無視の糖衣が無いため自前実装。識別子の予約語拒否を Lex 側 `reserved` で吸収したい。期待ラベルなし（`label("identifier")/label("string")/label("number")` 不足）。`RunConfig.extensions["lex"]` との連携なし（space/profile/identifier_profile 未共有）。 |
| `toml_parser.reml` | `sc = whitespace + commentLine(\"#\")`。`lexeme/sym` 再定義。キーは `stringLit` または英数字/`-`/`_` の手書きルール。文字列複数行は `Lex.string(\"\"\"\")` + `takeWhile` の簡易実装。 | `core`（lexeme/symbol）、`literal`（stringLit） | TOML 互換のトリビアは `ConfigTriviaProfile::toml_relaxed` で代替可能だが未使用。ベアキーの文字集合（`-` 許容）を `IdentifierProfile` で表現していない。複数行文字列は `stringMultiline` プリセット未活用。期待ラベルなし。`RunConfig` 共有なし。 |
| `yaml_parser.reml` | `lexeme` 未使用。`hspace`/`newline`/`comment`/インデント期待を手書き。`scalar_value` で `Lex.integer()/stringLit()` を直接使用。`Parse.fail_with_expectations` で一部構造化エラーを自前生成。 | `core`（whitespace/comment 相当の手組み）には該当するがプリセット未利用。Layout/indent は独自実装。 | Layout/オフサイドに近い処理を独自に実装し、`LayoutProfile`・`autoWhitespace` の橋渡しが無い。`lexeme/symbol/keyword` プリセットを使っておらず、`space_id` 共有も無し。期待ラベルは全般不足。 |

#### RunConfig 共有キーと期待ラベルの揺れ（サンプル横断での論点）
- いずれのサンプルも `RunConfig.extensions["lex"]` への `space_id`/`profile`/`identifier_profile`/`layout_profile`/`safety` の注入を行っておらず、CLI/LSP/回帰でトリビア設定を再構成できない。`lex_pack` 例（2-3 §L-4）を共通エントリにする必要がある。
- 期待ラベルは `identifier/number/string` など WS2 推奨語彙が未付与で、`Parse.fail` の自由文も残っている。`Expectation::{Keyword,Class,Rule}` を維持するため、プリセット側で `label` 付きヘルパ（`identifier().label("identifier")` など）を標準化し、サンプルは糖衣をそのまま利用する方針に寄せる。

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
- 新規サンプル
  - `examples/spec_core/chapter2/parser_core/` に「LexPack を使った最小 DSL」（識別子 + 数値 + `=` + `;`）を追加し、空白/コメント混在でも安定することを示す
- 回帰
  - 計画起点 ID: `CP-WS3-001`（空白・コメント・改行の差で壊れない）
  - 期待出力で固定する要素
    - whitespace/comment を含む入力でも同じ AST/同じ診断になる
    - lex 由来の失敗が `label` 付きの期待（例: `identifier`）として出る（WS2 と連動）

## リスクと緩和
- lexer 的処理を parser 側に寄せすぎると、柔軟性を損ねる  
  → `Core.Parse.Lex` は **プリセット**として提供し、強制しない（opt-in/差し替え可能）
