# WS1: Cut/Commit（バックトラック制御）計画

## 背景と狙い
調査メモ `docs/notes/parser/core-parse-improvement-survey.md` が強調する通り、パーサーコンビネーターの実用性（エラー位置・性能）には **バックトラック制御**が不可欠である。

- Parsec: `try` を明示した時だけバックトラック
- FastParse: `Cut` を頻繁に用い、分岐点を確定する

Reml は `docs/spec/2-1-parser-type.md` で `Reply{consumed, committed}` と `cut` の意味を定義しているため、これを **運用可能な API/慣習**へ落とし込む。

## 目標
- 代表的な「分岐が確定する地点」で `cut/commit` を適用でき、診断が自然になる
- `cut/commit` が性能（不要な分岐探索）にも寄与し、退行時は opt-in で切り戻せる

## 設計要点（仕様準拠の確認項目）
- `cut` は「以降の失敗を committed=true にする境界」として扱う（消費有無とは独立）
- `or` は `committed=true` または `consumed=true` の失敗で代替分岐を試さない
- 期待集合（expected set）は「最遠失敗」「cut 境界」を加味して統合する（詳細は `docs/spec/2-5-error.md`）

## タスク分割

## 進捗（この計画の状態が分かるチェックリスト）

> 更新方針: 「Step の完了」だけでなく、仕様・回帰・サンプルが揃っているかをサブ項目で追えるようにする。

- [x] Step 0: 現状の「Cut を置くべき場所」を棚卸しする
  - [x] 代表サンプル（PL/0 / JSON / TOML / YAML / Spec.Core）の観測点を整理
  - [x] Step1 へ渡す最小チェックリストを確定（JSON/YAML の境界例を含む）
- [x] Step 1: 仕様・ガイドの最小一貫化（Cut の意味と運用を固定）
  - [x] `docs/spec/2-2-core-combinator.md`（D 節）へ JSON/YAML 境界例（短縮版）を反映
  - [x] `docs/spec/2-1-parser-type.md` / `docs/spec/2-6-execution-strategy.md` を読み合わせし、`committed` が消費と独立であること・cut 通過後に期待集合を再初期化することを明示
  - [x] 追加追記は最小（2-1 に cut/committed 独立の注記、2-6 に期待再初期化＋ゼロ幅 cut の明記。2-2 は再追記不要）
- [x] Step 2: API 表面（糖衣）を「迷いが減る形」で整える
  - [x] 決定ログを `docs/notes/parser/core-parse-api-evolution.md` に記録（`commit(p)` は derived、`p.commit()` は追加しない等）
- [x] Step 3: サンプルと回帰（Cut の効果を “見える化” して固定）
  - [x] Cut 有り（現行）: `core-parse-cut-branch-mislead` / `core-parse-cut-unclosed-paren` をゴールデン化
  - [x] 比較対象（Cut 無し相当）: `*-no-cut` の入力と期待（誤誘導版）を追加
  - [x] Phase4 マトリクスへ比較対象リンクを反映（`CH2-PARSE-102/103` の scenario_notes / spec_anchor）
  - [x] `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` 側へ比較対象の参照を追記

### Step 0: 現状の「Cut を置くべき場所」を棚卸しする
Cut を入れる位置が曖昧だと、診断の改善も回帰の固定もできないため、まず「典型パターン」を確定する。

- 参照すべき既存仕様（読み合わせ対象）
  - `docs/spec/2-1-parser-type.md`（`Reply{consumed, committed}` と `cut` の意味、ミニ例）
  - `docs/spec/2-2-core-combinator.md`（A-3 使用指針、`cut_here()`、`expect` 糖衣）
  - `docs/spec/2-5-error.md`（B-5 `cut` の効果、B-2 最遠位置の優先規則）
  - `docs/spec/2-4-op-builder.md`（演算子消費後の `cut_here()` 相当）
  - `docs/spec/2-6-execution-strategy.md`（`cut_here()` 通過後の期待集合破棄）
- 既存サンプルの現状確認（「どこで attempt に頼っているか」）
  - `examples/language-impl-samples/reml/pl0_combinator.reml`（括弧の `cut(expr)`、`expect_sym`）
  - `examples/language-impl-samples/reml/json_parser_combinator.reml`（`attempt` 多用の分岐）
  - `examples/language-impl-samples/reml/toml_parser.reml`（曖昧な字句分岐での `attempt`、`[`/`{` 開始枝の扱い）
  - `examples/language-impl-samples/reml/yaml_parser.reml`（構造 vs スカラー分岐、`lookahead` と Cut の必要性）
  - `examples/spec_core/chapter2/parser_core/core-parse-or-commit-ok.reml`（`commit(p)` 表面 API の現状）
- ここまでの成果（ドキュメント化）
  - 「Cut を置く場所チェックリスト（暫定）」を本計画内（本節末尾）に追加し、次の Step で仕様へ反映する判断材料にする

#### 棚卸し結果（現状のパターンとギャップ）
この節は「仕様が意図する Cut/attempt の役割」と「サンプルが実際に採っている書き方」を突き合わせ、**Cut を置くべき典型地点**を言語化する。
次の Step 1 では、本節の結論を `docs/spec/2-2-core-combinator.md` 等へ最小限で反映できる形へ整理する。

**仕様側で“既に明文化されている”典型**
- `cut` は `committed=true` を立て、`or` で右枝へ逃げない（`docs/spec/2-1-parser-type.md` / `docs/spec/2-2-core-combinator.md`）。
- `cut` を越えたら期待集合（expected set）を“再初期化”し、上位の曖昧な期待を引きずらない（`docs/spec/2-5-error.md` B-5、`docs/spec/2-6-execution-strategy.md`）。
- 括弧ペア（`between(open, p, close)`）は **開き側を消費した瞬間に cut**（`docs/spec/2-5-error.md` §D-1）。
- 演算子は **演算子を消費した時点で `cut_here()` 相当**を入れ、右項欠落を committed 失敗として報告（`docs/spec/2-4-op-builder.md` C-1）。

**サンプル側で観測された現状**
- PL/0（`examples/language-impl-samples/reml/pl0_combinator.reml`）
  - 括弧は `cut(expr)` を既に採用しており、**括弧内の失敗が別分岐へ逃げない**形になっている。
  - 文（`stmt`）の分岐は `attempt(while_stmt)` / `attempt(write_stmt)` で“戻れる”形を取っている一方、内部の `expect_kw` / `expect_sym` は `expect = label+cut`（2.2 C の糖衣）であるため、**分岐の粒度（attempt を置く場所）と cut の粒度が噛み合っていない可能性**がある。
    - 典型的には「分岐の入口（先頭キーワード等）は attempt で戻れる」「形が確定した後（`do`、`:=`、`)` など）は cut」で段階化する。
  - 観測ポイント（抜粋）

    ```reml
    // 括弧: open を読んだ瞬間に cut（括弧ペア定形に沿う）
    expect_sym("(").then(cut(expr)).then(expect_sym(")"))

    // stmt 分岐: 枝全体が attempt で包まれる（入口だけに寄せたい）
    choice([attempt(while_stmt), attempt(write_stmt), assign_stmt])
    ```
  - ギャップと推奨
    - `attempt(while_stmt)` のように枝全体を戻れる形にすると、`while <expr> do <block>` の途中失敗が「別の文」へ誤誘導しやすい。
    - **推奨**：入口（先頭キーワード）だけ `attempt` で戻れるようにし、形が確定する地点（`do`、`:=` 等）を `expect_*`（= `label+cut`）で固定する。
- JSON（`examples/language-impl-samples/reml/json_parser_combinator.reml`）
  - `json_value` が `attempt(json_array)` のように代替枝を広く attempt で包んでいるため、`[` や `{` のような **一意トークンを消費した後の失敗でも、別枝へ戻れてしまう**（＝誤誘導しやすい）構造になっている。
  - 配列内部も `sepBy(attempt(json_value), ",")` となっており、要素の途中失敗が「空失敗化」されて **`,` や `]` の期待へ自然に収束しない**可能性がある。
    - 典型的には `[` / `{` / `:` / `,` を消費した後に cut（または cut を含む `expect_*`）を置き、「ここからは配列/オブジェクトとして報告する」を固定する。
  - 観測ポイント（抜粋）

    ```reml
    // 配列: `[` を読んだ後でも value 全体が attempt され、誤誘導しうる
    between(sym("["), sepBy(attempt(json_value), sym(",")), sym("]"))

    // 値: array/string/number を attempt で包む（`[` や `"` は一意なのに戻れる）
    choice([..., attempt(json_number), attempt(json_string), attempt(json_array), json_object])
    ```
  - ギャップと推奨（WS1 観点の最優先）
    - `[` / `{` / `"` のような **一意トークンの枝は attempt しない**（入口の曖昧さが無い）。
    - `[` / `{` / `:` / `,` を消費した直後に `cut_here()`（または `expect_*`）を置き、期待集合を **配列/オブジェクト文脈に再初期化**させる（2.5 B-5）。
- TOML（`examples/language-impl-samples/reml/toml_parser.reml`）
  - `toml_value` では `attempt` が主に **先頭が重なりうる字句（日時/浮動小数点/整数/文字列等）**へ使われており、`[`/`{` 開始の構文は attempt されていない。
  - この形は「曖昧な枝だけ戻れる」「一意の開始記号を見たら戻れない」という直観に沿うため、**Cut の置き場の手がかり**として扱える。
  - 観測ポイント（抜粋）

    ```reml
    // 曖昧な字句（日時/数値/文字列）だけ attempt
    Parse.choice([
      Parse.attempt(string_value),
      Parse.attempt(datetime_value),
      Parse.attempt(float_value),
      Parse.attempt(integer_value),
      boolean_value,
      array_value,
      inline_table
    ])

    // 一意トークン: "[" / "[[" / "{" は attempt されていない
    sym("[").skipR(...).skipL(sym("]"))
    sym("[[").skipR(...).skipL(sym("]]"))
    sym("{").skipR(...).skipL(sym("}"))
    ```
  - ギャップと推奨
    - すでに「曖昧な入口だけ attempt」という良い形になっている。WS1 では TOML の形を **推奨パターンの参照例**として扱う。
- YAML（`examples/language-impl-samples/reml/yaml_parser.reml`）
  - `parse_value` が `attempt(parse_list)` / `attempt(parse_map)` を入口で使っており、`-` や `:` を見た後でも戻れる構造になりやすい。
  - YAML のようにスカラーと構造が“見た目で近い”言語では、`lookahead` による判定と組み合わせて「`- ` を見たらリスト」「`:`（キー終端）を見たらマップ」を確定し、確定後は cut で報告を固定するのが自然である。
  - 観測ポイント（抜粋）

    ```reml
    // 値: list/map を attempt で包む（構造確定後も戻れてしまう）
    Parse.choice([Parse.attempt(parse_list(indent)), Parse.attempt(parse_map(indent)), scalar_value])

    // map_entry: ":" を読んだ後でも value 側が attempt されうる
    Lex.string(":").skipR(hspace).skipR(Parse.choice([Parse.attempt(parse_value(...)), newline.skipR(parse_value(...))]))
    ```
  - ギャップと推奨（WS1 と WS2 の接合点）
    - `-` / `:` は YAML 文脈では「構造確定トークン」に近い。`lookahead` で構造を判定し、確定後は `cut_here()` を入れて「list/map としての失敗」へ固定するのが自然。
    - この整理は WS2（label）とも相性が良く、`expected_tokens` の notes に「`:` の後に値」等の文脈を残す導線になる。
- Spec.Core（`examples/spec_core/chapter2/parser_core/core-parse-or-commit-ok.reml`）
  - `commit(p)` という表面 API がサンプルに登場しているため、本 WS では `cut`/`cut_here`/`commit` の関係（糖衣か別物か）を棚卸しし、用語の揺れが Step 1 以降の議論を阻害しないようにする。
  - 観測ポイント（抜粋）

    ```reml
    Parse.or(
      Parse.then(Parse.expect_symbol("+"), Parse.expect_int()),
      Parse.commit(Parse.expect_int())
    )
    ```
  - ギャップと推奨
    - `commit` は「別名」ではなく「意図強調の糖衣」として扱い、Step1 で用語を固定する（2.2 C の `commit = cut`）。

#### Cut を置く場所チェックリスト（暫定 / Step1 へ渡す最小形）
- **曖昧な入口は attempt、確定後は cut**：共通接頭辞（先頭キーワード等）だけ戻れるようにし、枝全体を安易に `attempt` で包まない
- **固定形が確定した直後**：`let <ident>` など「ここまで通れば構文が確定」→ `cut_here()`
- **ペア構造は open 消費で確定**：`(` / `[` / `{` を消費したら、その内側の失敗を別枝へ逃がさない（`cut_here()` または `cut(p)`）
- **区切り記号の直後で確定**：`:` / `,` / `->` / `=>` を消費したら「次要素が必須」→ `cut_here()`
- **演算子消費後で確定**：`term + <rhs>` の `<rhs>` 欠落は「別構文」ではなく「この構文の不足」→ `cut_here()` 相当（2.4）
- **期待集合を絞りたい地点で確定**：上位の曖昧な期待集合を引きずらない（2.5 B-5 の “再初期化”）
- **lookahead は判定、cut は確定**：見た目が近い分岐（例: YAML 構造）では `lookahead` で分岐を決め、確定トークン消費直後に `cut_here()` を置く

**JSON/YAML の境界例（短縮 / トークンを明示）**

```reml
sym("[").then(cut_here()).then(elements).then(sym("]"))        // JSON: `[`
sym("{").then(cut_here()).then(members).then(sym("}"))         // JSON: `{`
key.then(Lex.string(":").then(cut_here())).then(value)         // JSON/YAML: `:`
sym(",").then(cut_here()).then(value)                          // JSON: `,`
lookahead(Lex.string("-")).then(Lex.string("-").then(cut_here())).then(value) // YAML: `-`
```

### Step 1: 仕様・ガイドの最小一貫化（Cut の意味と運用を固定）
- `docs/spec/2-1-parser-type.md` / `docs/spec/2-2-core-combinator.md` / `docs/spec/2-5-error.md` を読み合わせ、次の点が一意に読めるか確認する
  - `consumed` と `committed` の独立性（cut は consumed とは別ビット）
  - `or` の分岐可否（`Err(consumed=true ∨ committed=true)` なら右を試さない）
  - `cut` 後は期待集合を再初期化する（B-5）
- 不足があれば追記案を作る（追記対象）
  - `docs/spec/2-2-core-combinator.md`: 「Cut を置く場所チェックリスト」を短く整理して追記
  - `docs/spec/2-5-error.md`: cut を跨いだ期待集合の縮約例（括弧、演算子）を追記
- 仕様の言い回しを揃える（用語ブレ防止）
  - `cut` / `cut_here` / `commit` の用語を統一し、別名を導入する場合は「同義語」ではなく「糖衣」として扱う

### Step 2: API 表面（糖衣）を「迷いが減る形」で整える
「新しい API を増やす」こと自体が目的にならないよう、追加判断を明示する。

- 判断基準（採否の物差し）
  - `docs/spec/0-1-project-purpose.md`（分かりやすいエラーメッセージ、学習コスト）
  - `docs/spec/0-1-project-purpose.md`（実用に耐える性能、無駄なバックトラック削減）
- 追加検討項目の棚卸し
  - `commit(p)` / `p.cut()` のような **糖衣**を追加するか（仕様・標準ライブラリ・ガイドのどこに置くか）
  - 既存の `expect(name, p)`（= `label` + `cut`）と役割が重複しないか
- 決定（2025-12-17）
  - `commit(p)` は **コアには追加しない**（最小公理系を増やさない）。
  - `commit(p)` は **派生（derived）の関数糖衣**として `Core.Parse` に提供し、意味論は `cut(p)` と同一とする（= 別名/意図強調）。
    - 採用理由: `Parse.or(..., Parse.commit(...))` の形は「ここで分岐を打ち切る」意図が読み取りやすく、Spec.Core サンプル（`examples/spec_core/chapter2/parser_core/core-parse-or-commit-ok.reml`）とも整合する。
  - メソッド糖衣は `p.cut()` を正とし、`p.commit()` は追加しない（表面積と用語の揺れを増やさない）。
    - `cut_here()` はゼロ幅コミットとして継続し、`commit_here()` 等の別名は導入しない。
- 決定の記録
  - 採否理由を `docs/notes/parser/core-parse-api-evolution.md` に短く残し、後続 WS（Label/Recovery）と衝突しないようにする

### Step 3: サンプルと回帰（Cut の効果を “見える化” して固定）
Cut の効果は「期待集合」「エラー位置」「分岐の抑制」に現れるため、いずれも固定できるシナリオを作る。

- 既存の基準ケース（先に維持確認）
  - Phase4 シナリオ `CH2-PARSE-101`（`examples/spec_core/chapter2/parser_core/core-parse-or-commit-ok.reml`）を本 WS の基準として扱い、`cut/commit` を使った分岐抑制が退行していないことを確認する
- 追加するサンプル（本計画で新規に作る）
  - `examples/spec_core/chapter2/parser_core/core-parse-cut-branch-mislead.reml`
    - 目的: 演算子 `+` の右項欠落（`(1 +)`）で、Cut/Commit により「右項（式）が必要」という期待へ収束することを固定する（誤誘導防止）
  - `expected/spec_core/chapter2/parser_core/core-parse-cut-branch-mislead.diagnostic.json`
    - 目的: `parser.syntax.expected_tokens` が「式開始トークン集合」を提示することを固定する（`+` 消費後の committed 境界）
  - `examples/spec_core/chapter2/parser_core/core-parse-cut-unclosed-paren.reml`
    - 目的: 括弧閉じ忘れ（`(1 + 2`）で、`)` が自然に期待へ現れることを固定する（括弧ペアの cut 境界）
  - `expected/spec_core/chapter2/parser_core/core-parse-cut-unclosed-paren.diagnostic.json`
    - 目的: `parser.syntax.expected_tokens` の期待集合が `)` を含む形で安定することを固定する
- 比較対象（Cut 無し相当 / “誤誘導版”）
  - 目的: 現行の `CH2-PARSE-102/103` は Cut 導入後の期待集合を固定しているため、差分で「Cut が効いて良くなった」ことが示しにくい。
    そこで **Cut が無い（または Cut が効いていない）想定**の期待集合を、比較用ゴールデンとして併置する。
  - `examples/spec_core/chapter2/parser_core/core-parse-cut-branch-mislead-no-cut.reml`
    - 入力は同じ（`(1 +)`）だが、**Cut 無し想定では `)` を期待してしまう**などの誤誘導を起こしうることを示す
  - `expected/spec_core/chapter2/parser_core/core-parse-cut-branch-mislead-no-cut.diagnostic.json`
    - 期待: `parser.syntax.expected_tokens` の notes が「`)` が必要」側へ崩れる（比較対象）
  - `examples/spec_core/chapter2/parser_core/core-parse-cut-unclosed-paren-no-cut.reml`
    - 入力は同じ（`(1 + 2`）だが、**Cut 無し想定では `)` へ自然に収束しない**（広い期待集合へ巻き戻る）可能性を示す
  - `expected/spec_core/chapter2/parser_core/core-parse-cut-unclosed-paren-no-cut.diagnostic.json`
    - 期待: `parser.syntax.expected_tokens` の notes が「式開始トークン集合」等へ巻き戻る（比較対象）
- シナリオ登録（計画起点 ID → Phase4 反映）
  - 計画起点 ID: `CP-WS1-001`（Cut による分岐抑制を可視化）
  - `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に新規行を追加し、`CH2-PARSE-102` を割り当てる
  - `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に新規行を追加し、`CH2-PARSE-103` を割り当てる
  - 併せて `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` の PhaseF チェックへリンクを追記する

## 成果物
- ドキュメント追記（必要な場合）:
  - `docs/spec/2-2-core-combinator.md`
  - `docs/spec/2-5-error.md`
- サンプル:
  - `examples/spec_core/chapter2/parser_core/` に Cut の有無比較
- 回帰:
  - bootstrap-roadmap のシナリオマトリクスへ転写（`2-0-integration-with-regression.md` 参照）

## リスクと緩和
- Cut の多用で「回復の余地」が減る可能性がある  
  → WS4（Error Recovery）とセットで運用し、「確定すべき境界」と「回復すべき境界」を分ける
- Cut 導入で期待集合の統合ルールが複雑化する  
  → `docs/spec/2-5-error.md` の規則を先に固定し、実装は仕様に追随させる
