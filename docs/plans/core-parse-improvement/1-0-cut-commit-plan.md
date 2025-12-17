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
### Step 1: 仕様・ガイドの最小一貫化
- `docs/spec/2-1-parser-type.md` と `docs/spec/2-5-error.md` の `cut/committed` 記述を読み合わせ、曖昧な表現（例: consumed と committed の関係）がないか洗い出す
- `docs/spec/2-2-core-combinator.md` に、`cut` を「頻繁に使う」推奨パターン（if/let/match など）を追記する案を準備

### Step 2: API 表面の整備（命名・糖衣）
- `cut(p)` に加えて、読みやすい糖衣（例: `p.cut()`、`commit(p)`）の要否を整理する
- `cut` と `attempt`（または `try` 相当）の関係を「DSL 作者が迷わない」形で説明する

### Step 3: サンプルと回帰シナリオ
- Cut がない場合に「別分岐へ逃げて変なエラーになる」入力をサンプル化する
  - 例: `if` 式の条件部で失敗したのに、`if` 全体が別ルール扱いになる
- Cut を入れた場合に「期待が条件式に固定される」ことを期待出力で固定する

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

