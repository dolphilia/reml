# WS1: Cut/Commit（バックトラック制御）計画（ドラフト）

## 背景と狙い
調査メモ `docs/notes/core-parse-improvement-survey.md` が強調する通り、パーサーコンビネーターの実用性（エラー位置・性能）には **バックトラック制御**が不可欠である。

- Parsec: `try` を明示した時だけバックトラック
- FastParse: `Cut` を頻繁に用い、分岐点を確定する

Reml は `docs/spec/2-1-parser-type.md` で `Reply{consumed, committed}` と `cut` の意味を定義しているため、これを **運用可能な API/慣習**へ落とし込む。

## 目標（ドラフト）
- 代表的な「分岐が確定する地点」で `cut/commit` を適用でき、診断が自然になる
- `cut/commit` が性能（不要な分岐探索）にも寄与し、退行時は opt-in で切り戻せる

## 設計要点（仕様準拠の確認項目）
- `cut` は「以降の失敗を committed=true にする境界」として扱う（消費有無とは独立）
- `or` は `committed=true` または `consumed=true` の失敗で代替分岐を試さない
- 期待集合（expected set）は「最遠失敗」「cut 境界」を加味して統合する（詳細は `docs/spec/2-5-error.md`）

## タスク分割（ドラフト）
### Step 0: 現状の「Cut を置くべき場所」を棚卸しする
Cut を入れる位置が曖昧だと、診断の改善も回帰の固定もできないため、まず「典型パターン」を確定する。

- 参照すべき既存仕様（読み合わせ対象）
  - `docs/spec/2-2-core-combinator.md`（A-3 使用指針、`cut_here()`、`expect` 糖衣）
  - `docs/spec/2-5-error.md`（B-5 `cut` の効果、B-2 最遠位置の優先規則）
  - `docs/spec/2-4-op-builder.md`（演算子消費後の `cut_here()` 相当）
  - `docs/spec/2-6-execution-strategy.md`（`cut_here()` 通過後の期待集合破棄）
- 既存サンプルの現状確認（「どこで attempt に頼っているか」）
  - `examples/language-impl-comparison/reml/pl0_combinator.reml`（括弧の `cut(expr)`、`expect_sym`）
  - `examples/language-impl-comparison/reml/json_parser_combinator.reml`（`attempt` 多用の分岐）
- ここまでの成果（ドキュメント化）
  - 「Cut を置く場所チェックリスト（暫定）」を本計画内（本節末尾）に追加し、次の Step で仕様へ反映する判断材料にする

#### Cut を置く場所チェックリスト（暫定）
- **固定形が確定した直後**：`let <ident>`、`if <cond> then`、`match <expr> with` のように、ここまで通れば構文が確定する地点 → `cut_here()`
- **括弧・括弧に準ずるペアの内側**：`(` の後に `expr` が失敗したら別分岐へ逃がさない → `cut(expr)`（または `cut_here()` + `expr`）
- **演算子消費後**：`term + <rhs>` の `<rhs>` 不足は「別構文へ分岐」ではなく「この構文の不足」 → `cut_here()` 相当
- **“期待を絞りたい”地点**：上位の曖昧な期待集合を引きずらない（`docs/spec/2-5-error.md` の `cut`/期待集合縮約の節）
- **回復（recover）と混ぜる場合**：cut 境界は「回復しない」ではなく「分岐しない」を保証する、と解釈できるよう設計する（WS4 と整合が必要）

### Step 1: 仕様・ガイドの最小一貫化（Cut の意味と運用を固定）
- `docs/spec/2-1-parser-type.md` / `docs/spec/2-2-core-combinator.md` / `docs/spec/2-5-error.md` を読み合わせ、次の点が一意に読めるか確認する
  - `consumed` と `committed` の独立性（cut は consumed とは別ビット）
  - `or` の分岐可否（`Err(consumed=true ∨ committed=true)` なら右を試さない）
  - `cut` 後は期待集合を再初期化する（B-5）
- 不足があれば追記案を作る（追記対象候補）
  - `docs/spec/2-2-core-combinator.md`: 「Cut を置く場所チェックリスト」を短く整理して追記
  - `docs/spec/2-5-error.md`: cut を跨いだ期待集合の縮約例（括弧、演算子）を追記
- 仕様の言い回しを揃える（用語ブレ防止）
  - `cut` / `cut_here` / `commit` の用語を統一し、別名を導入する場合は「同義語」ではなく「糖衣」として扱う

### Step 2: API 表面（糖衣）を「迷いが減る形」で整える
「新しい API を増やす」こと自体が目的にならないよう、追加判断を明示する。

- 判断基準（採否の物差し）
  - `docs/spec/0-1-project-purpose.md`（分かりやすいエラーメッセージ、学習コスト）
  - `docs/spec/0-1-project-purpose.md`（実用に耐える性能、無駄なバックトラック削減）
- 追加候補の棚卸し
  - `commit(p)` / `p.cut()` のような **糖衣**を追加するか（仕様・標準ライブラリ・ガイドのどこに置くか）
  - 既存の `expect(name, p)`（= `label` + `cut`）と役割が重複しないか
- 決定の記録
  - 採否理由を `docs/notes/core-parse-api-evolution.md` に短く残し、後続 WS（Label/Recovery）と衝突しないようにする

### Step 3: サンプルと回帰（Cut の効果を “見える化” して固定）
Cut の効果は「期待集合」「エラー位置」「分岐の抑制」に現れるため、いずれも固定できるシナリオを作る。

- サンプルの追加（候補）
  - `examples/spec_core/chapter2/parser_core/` に、Cut なし/ありで差が出る最小例を追加
    - 例: `if` の条件式が壊れているのに `if` 全体が別文扱いになり、位置や期待がずれるケース
- 期待出力の固定（候補）
  - `expected/spec_core/chapter2/parser_core/` に stdout/diagnostic を固定し、差分が出たら退行として検出できるようにする
- シナリオ登録（計画起点 ID）
  - 本ディレクトリ側で `CP-WS1-001` を付与（Cut による分岐抑制）
  - bootstrap-roadmap へ転写する際に `CH2-PARSE-xxx` を割当（`docs/plans/core-parse-improvement/2-0-integration-with-regression.md`）
  - 転写後は `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` の該当チェックへリンクを追記する

## 成果物（ドラフト）
- ドキュメント追記案（必要なら）:
  - `docs/spec/2-2-core-combinator.md`
  - `docs/spec/2-5-error.md`
- サンプル（候補）:
  - `examples/spec_core/chapter2/parser_core/` に Cut の有無比較
- 回帰（候補）:
  - bootstrap-roadmap のシナリオマトリクスへ転写（`2-0-integration-with-regression.md` 参照）

## リスクと緩和
- Cut の多用で「回復の余地」が減る可能性がある  
  → WS4（Error Recovery）とセットで運用し、「確定すべき境界」と「回復すべき境界」を分ける
- Cut 導入で期待集合の統合ルールが複雑化する  
  → `docs/spec/2-5-error.md` の規則を先に固定し、実装は仕様に追随させる
