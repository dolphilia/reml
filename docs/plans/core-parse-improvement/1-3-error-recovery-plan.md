# WS4: Error Recovery（複数エラー・IDE 向け）計画

## 背景と狙い
調査メモ `docs/notes/core-parse-improvement-survey.md` は Chumsky の強力な回復（`recover_with`）を挙げ、IDE の解析エンジン向けには「失敗したら止まる」だけでは不足すると示唆している。

Reml の回帰計画（Phase4）でも、診断品質を継続監視するには「単発エラー」だけでなく **複数エラーの収集** が必要になる。

## 参照
- `docs/spec/2-5-error.md`（回復戦略と診断モデル）
- `docs/spec/2-7-core-parse-streaming.md`（ストリーミングでの再開と整合）
- `docs/spec/3-6-core-diagnostics-audit.md`（診断キー運用）

## 目標
- 代表的な DSL 入力で、1 回の実行で複数箇所のエラーを報告できる
- `cut` と矛盾しない回復戦略（「確定すべき境界」と「回復すべき境界」を分ける）を持つ
- 回復によっても Span/位置情報が破綻しない

## 回復戦略（採用する最小セット）
- `recover_with_default(value)`：失敗時に既定値を置いて続行（式の穴埋め）
- `recover_until(sync)`：同期トークン（`;` や `}` など）まで読み飛ばして継続
- `recover_with_insert(token)`：欠落トークンを補挿し、FixIt を添付して継続
- `recover_with_context(message)`：回復に関するヒントを診断へ追加

## タスク分割
### Step 0: 回復の “責務境界” を決める（停止/継続/厳格モード）
回復は強力だが、ビルド用途では「誤った AST で先へ進む」危険もある。
まず「どの場面で回復を許すか」を明文化する。

- 参照
  - `docs/spec/2-5-error.md`（`ParseError.secondaries`、FixIt、回復時の診断生成）
  - `docs/spec/2-2-core-combinator.md`（`recover(p, until, with)` の定義）
  - `docs/spec/3-6-core-diagnostics-audit.md`（診断キー/Severity 運用）
- 決めること
  - **IDE/LSP 向け**：回復を積極利用して複数エラーを収集
  - **ビルド/CI 向け**：`RunConfig` で「回復無効（fail-fast）」を選べる前提を維持

### Step 1: 仕様上の回復契約を “固定” する（cut との整合が中心）
- `recover` の契約について、少なくとも次を明文化できる状態にする
  - `recover` は「診断を残しつつ同期して継続」する（診断生成は `Err.pretty` 経路に乗る）
  - 同期点（`until`）の設計方針（例: `;` / `}` / 改行など）をガイド化する
  - `cut`（committed）を跨いだ失敗でも回復するか（優先度）
    - 例: 方針案A「committed でも回復は可能（分岐しないだけ）」
    - 方針案B「committed を越えた失敗は回復しない（fail-fast）」
  - どちらを採るかは、WS1（Cut）とセットで決定し、判断根拠を残す
- 仕様追記が必要な場合の対象
  - `docs/spec/2-5-error.md`: 回復による `secondaries` の扱い、FixIt の位置づけ
  - `docs/spec/2-2-core-combinator.md`: `recover` の推奨同期点パターン（短い表）

### Step 2: “回復の型” を最小セットに整理する（糖衣の設計）
実装側の都合ではなく、DSL 作者が頻繁に使う形に合わせて最小セットを定義する。

- 糖衣と `recover` への落とし込み
  - `recover_with_default(value)` → `recover(p, until=..., with=value)`
  - `recover_until(sync)` → `recover(p, until=sync, with=...)`（with は ErrorNode など）
  - `recover_with_insert(token)` → FixIt を付与しつつ同期（仕様上の FixIt と整合）
- 「回復した結果として何を返すか」を例で固定する
  - AST の穴（ErrorNode）
  - `Option<T>`（欠落を `None` で表現）
  - `Result<T, _>`（失敗を値に落とすのは最小限にする、などの指針）

### Step 3: サンプルと回帰（複数エラーを固定できる最小入力から始める）
- サンプル
  - `examples/spec_core/chapter2/parser_core/` に「複数エラーを含む入力」を追加
    - 例: `let x = ; let y = 1 + ;` のような “同期点がある” 例
  - 期待出力で固定する要素（初期の最低保証）
    - 2 件以上の診断が出ること
    - 最初の診断位置（Span）と主要メッセージが固定されること
    - 同期点（例: `;`）以降も解析が進むこと
- 回帰登録
  - 計画起点 ID: `CP-WS4-001`（複数診断の収集）
  - 期待出力の揺れ対策
    - 初期は「件数/最初の位置/代表キー」中心に固定し、詳細な期待集合は段階導入する

### Step 4: 他 WS との整合チェック（Cut/Label/Lex）
- WS1（Cut）: committed 失敗と回復の優先度が矛盾していないか
- WS2（Label）: 回復時にも `label` が期待集合へ残るか（期待がトークン列だけに崩れないか）
- WS3（Lex）: 同期点が字句ヘルパ（`symbol/keyword`）で自然に書けるか

## リスクと緩和
- 回復の導入で誤った AST が広がる  
  → `Diagnostic` を必ず添付し、IDE 表示用途は許容しても、ビルド用途は `RunConfig` で厳格モードを選べる設計を維持する
