# WS4: Error Recovery（複数エラー・IDE 向け）計画（ドラフト）

## 背景と狙い
調査メモ `docs/notes/core-parse-improvement-survey.md` は Chumsky の強力な回復（`recover_with`）を挙げ、IDE の解析エンジン向けには「失敗したら止まる」だけでは不足すると示唆している。

Reml の回帰計画（Phase4）でも、診断品質を継続監視するには「単発エラー」だけでなく **複数エラーの収集** が必要になる。

## 参照
- `docs/spec/2-5-error.md`（回復戦略と診断モデル）
- `docs/spec/2-7-core-parse-streaming.md`（ストリーミングでの再開と整合）
- `docs/spec/3-6-core-diagnostics-audit.md`（診断キー運用）

## 目標（ドラフト）
- 代表的な DSL 入力で、1 回の実行で複数箇所のエラーを報告できる
- `cut` と矛盾しない回復戦略（「確定すべき境界」と「回復すべき境界」を分ける）を持つ
- 回復によっても Span/位置情報が破綻しない

## 回復戦略の候補（ドラフト）
- `recover_with_default(value)`：失敗時に既定値を置いて続行（式の穴埋め）
- `recover_until(predicate)`：同期トークン（`;` や `}`）まで読み飛ばす
- `recover_with_insert(token)`：欠落トークンの補挿（例: `)` が抜けた）
- `recover_with_context(message)`：回復時にヒントを追加

## タスク分割（ドラフト）
### Step 1: 仕様上の回復契約の確認
- `recover` 系 API の定義と、`cut` との優先度（cut を越えた失敗でも回復するのか）を整理する
- 「回復した結果として何を返すか」（AST の穴、`Option`、`Result`）を例で示す

### Step 2: サンプルの用意
- エラーを複数含む入力を用意し、「何個の診断が出るか」「どこで同期するか」を固定する
- DSL の作者向けに「回復を入れる場所の指針」をまとめる

### Step 3: 回帰への接続
- 回復シナリオは、期待出力が揺れやすい  
  → まずは「最低保証」（例: 2 件以上報告、最初のエラー位置は固定）から始め、詳細は段階導入する

## リスクと緩和
- 回復の導入で誤った AST が広がる  
  → `Diagnostic` を必ず添付し、IDE 表示用途は許容しても、ビルド用途は `RunConfig` で厳格モードを選べる設計を維持する

