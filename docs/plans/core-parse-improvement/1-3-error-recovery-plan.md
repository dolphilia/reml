# WS4: Error Recovery（複数エラー・IDE 向け）計画

## 背景と狙い
調査メモ `docs/notes/parser/core-parse-improvement-survey.md` は Chumsky の強力な回復（`recover_with`）を挙げ、IDE の解析エンジン向けには「失敗したら止まる」だけでは不足すると示唆している。

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
- Step0 の成果物（出口条件）
  - **運用プロファイル**（IDE/LSP と Build/CI）の最小方針が文章化されている
  - **RunConfig での切替契約**（どのキーを見て、どう振る舞いを切り替えるか）が明確になっている
  - **性能・安全弁**（無限回復/過剰スキップの抑止）を “仕様ではなく運用契約” として最低限定義している
  - Step1（cut との整合）へ持ち越す未決事項が列挙されている

- 決めること（暫定決定）
  - **回復は opt-in**：`recover`（および糖衣）を使った箇所だけ回復し、グローバルな暗黙回復は導入しない。
    - 目的：誤った AST が予期せず広がるリスクを抑え、DSL 作者が “回復境界” を明示できるようにする。
  - **IDE/LSP 向け（collect）**：回復を積極利用して複数エラーを収集する。
    - `RunConfig.extensions["recover"].mode = "collect"` を推奨。
    - `extensions["recover"].sync_tokens`（同期点）を明示し、復旧の再現性を保つ（CLI/LSP/ストリーミングで同一）。
  - **ビルド/CI 向け（fail-fast）**：回復は無効化でき、最初のエラーで停止できる前提を維持する。
    - `RunConfig.extensions["recover"].mode = "off"`（既定）を推奨。
    - “仕様上は recover を書いてあるが、実行時は回復しない” を許容し、AST の穴埋め（`with`）を発生させない。
  - **共通の最低保証**：回復した場合は必ず診断を残し、回復の事実が追跡できる。
    - 例：`Diagnostic.extensions["recover"]` に `{ recovered: true, sync: "...", inserted: "...", ... }` を保持（キー名は Step2 で確定）。

- 責務境界（どこまでを Core.Parse が担うか）
  - Core.Parse は「同期して継続できる最小プリミティブ（`recover`）」と「RunConfig による運用切替」を提供する。
  - “どの同期点が妥当か” は DSL/文法ごとの判断であり、`recover_until(";")` のように **呼び出し側が指定**する。
  - `merge_warnings` は “表示ノイズ抑制” のみを担い、監査ログ（3-6）や内部メタの損失は許さない。

- 性能・安全弁（運用契約）
  - 回復は「無制限に読み飛ばす」ことができるため、IDE/LSP プロファイルでは上限を必須にする。
    - 例：`extensions["recover"].max_diagnostics = 64`、`max_resync_bytes = 4096`、`max_recoveries = 128`（値はガイドで推奨し、実装は best-effort）。
  - 上限超過時は “それ以上回復しない” ことを優先し、パーサを停止（fail-fast へフォールバック）する。

- 未決事項（Step1 へ持ち越し）
  - committed（`cut`）を越えた失敗でも回復を許すか（優先度）
  - 回復診断の集約ルール（`merge_warnings` と `ParseError.secondaries` の関係）
  - `recover_with_insert` の FixIt 生成と、AST へ挿入する `ErrorNode` の正規形

### Step 1: 仕様上の回復契約を “固定” する（cut との整合が中心）
- `recover` の契約について、少なくとも次を明文化できる状態にする
  - `recover` は「診断を残しつつ同期して継続」する（診断生成は `Err.pretty` 経路に乗る）
  - `RunConfig.extensions["recover"].mode` により、回復の有効/無効を切り替えられる
    - `"collect"`: 回復して継続（IDE/LSP 向け）
    - `"off"`（既定）: `recover(...)` は `p` と同様に失敗を返す（Build/CI の fail-fast 維持）
  - `cut`（committed）を跨いだ失敗でも回復するか（優先度）を決め、仕様に固定する
    - **採用（方針案A）**: committed でも回復は可能（ただし分岐はしない）
      - 根拠: IDE/LSP では「分岐探索」よりも「同期して先へ進む」ことが重要であり、回復は `or` の代替枝選択ではない。
      - 注意: Build/CI は `mode="off"` で停止するため、誤った AST が次工程へ渡らない。
  - 回復の観測可能性
    - 回復が起きたら `ParseResult.recovered=true` を立て、`ParseResult.diagnostics` に診断が蓄積されること（複数回 recover で複数件になる）を最小保証として固定する
  - 同期点（`until`）の設計方針をガイド化する（Lex ヘルパと整合）
    - 例: 文末 `;`、ブロック終端 `}`、行末 `"\n"`、括弧閉じ `")"` など
    - 同期点は「安全に構造を再開できる位置」を優先し、トークン消費を最小化する

- Step1 の成果物（出口条件）
  - `docs/spec/2-2-core-combinator.md` と `docs/spec/2-5-error.md` に、`mode="collect"|"off"` と committed 超え回復（方針案A）の契約が反映されている
  - `docs/spec/2-5-error.md` で、回復時の `ParseResult.recovered` と診断蓄積（複数件）の最低保証が明記されている
  - 同期点ガイド（短い表 or 箇条書き）が仕様に追加され、WS3（Lex）で自然に書ける前提が置かれている
- 仕様追記が必要な場合の対象
  - `docs/spec/2-5-error.md`: 回復による `secondaries` の扱い、FixIt の位置づけ
  - `docs/spec/2-2-core-combinator.md`: `recover` の推奨同期点パターン（短い表）

### Step 2: “回復の型” を最小セットに整理する（糖衣の設計）
実装側の都合ではなく、DSL 作者が頻繁に使う形に合わせて最小セットを定義する。

- Step2 の成果物（出口条件）
  - `recover_with_default/recover_until/recover_with_insert/recover_with_context` の **糖衣 API** が仕様に記載されている
  - `recover` 実行時に付与される `Diagnostic.extensions["recover"]` の **最低限スキーマ**（action/sync/insert/context 等）が確定している
  - DSL 作者が選ぶ「回復後の戻り値方針」（ErrorNode/Option/Result）について、**推奨と禁則**が短くまとまっている

- 糖衣（最小セット）
  - `recover_with_default(value)`：
    - 典型: 欠落オペランドを `ErrorNode` で穴埋め、あるいは `None` を返して継続したい
    - 仕様上は `recover(...)` の薄いラッパであり、回復メタ（`action="default"`）を付与する
  - `recover_until(sync)`：
    - 典型: 文末 `;` や `}` 等まで飛ばして “次の構造” から再開したい
    - `sync` は **DSL が決める**（Step1 の同期点指針に従う）
  - `recover_with_insert(token)`：
    - 典型: `")"` / `"}"` / `";"` の欠落を補挿して継続したい（IDE の QuickFix）
    - `FixIt::InsertToken(token)`（または同等）を付与し、`action="insert"` と `inserted=token` を `extensions["recover"]` に残す
  - `recover_with_context(message)`：
    - 典型: 「ここは expression を期待していた」のような回復ヒントを診断へ添付したい
    - `Diagnostic.notes` または `extensions["recover"].context` として保持する（`notes=true` 運用と整合）

- 回復時メタデータ（最小スキーマ）
  - `Diagnostic.extensions["recover"]` は最低限次のキーを持つ（詳細は仕様で確定）
    - `mode`: `"collect"`（回復実行時）
    - `action`: `"default" | "skip" | "insert" | "context"`
    - `sync`: `Option<Str>`（同期点の表示用）
    - `inserted`: `Option<Str>`（補挿トークン）
    - `context`: `Option<Str>`（回復ヒント）

- 回復後の戻り値方針（推奨）
  - **推奨 1: ErrorNode**（AST の穴）
    - すべての主要 AST ノードに `Error(ErrorNode)` を用意し、`recover_with_default(Expr.Error(...))` の形で継続する
    - IDE は `ErrorNode.span` と `expected` を使って赤波線・FixIt を提示できる
  - **推奨 2: Option**（欠落を `None` で表現）
    - 例: optional な `else`、省略可能な注釈などは `recover_with_default(None)` が自然
    - 禁則: `None` を返したのに診断を残さない（回復と silent success を混同しない）
  - **非推奨: Result**（失敗を値に落とす）
    - `Result<T, _>` を戻り値にすると、成功/失敗が二重化しやすく、回復境界が不透明になる
    - 例外: “部分構文を意図的に緩く受理する” DSL で、失敗もデータとして扱う場合のみ採用

### Step 3: サンプルと回帰（複数エラーを固定できる最小入力から始める）
- サンプル（文末 `;` 同期の最小ケース）
  - 入力: `let = 1; let x = ; let y = 3;`
    - 1 件目: 識別子欠落（`let` 直後）
    - 2 件目: 値欠落（`=` 直後）
    - 3 件目: 正常
  - サンプル実装（`.reml`）:
    - `examples/spec_core/chapter2/parser_core/core-parse-recover-multiple-errors-semicolon.reml`
  - 期待出力（診断 JSON）:
    - `expected/spec_core/chapter2/parser_core/core-parse-recover-multiple-errors-semicolon.diagnostic.json`
  - 同期点方針:
    - `until = symbol(";")` を **消費**する同期点として使い、同じ位置での再回復ループを避ける（2-5 §E-1）
  - RunConfig 方針:
    - `extensions["recover"].mode="collect"`
    - `extensions["recover"].sync_tokens=[";"]`

- 期待出力で固定する要素（初期の最低保証）
  - 2 件以上の診断が出ること（回復 2 回）
  - 診断コード `core.parse.recover.branch` が 2 件出ること（回復の蓄積）
  - 先頭の診断が「1 件目の欠落（識別子）」を指していること（メッセージ/notes で固定）

- 回帰登録（計画起点 ID）
  - `CP-WS4-001`（複数診断の収集 / `;` 同期）
  - Phase4 のシナリオ ID（`CH2-PARSE-202`）として `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に転写し、回帰監視対象へ追加する
  - 期待出力の揺れ対策（初期段階の固定方針）
    - `expected/**.diagnostic.json` は **診断件数** と **コード列**（順序を含む）を主に固定し、メッセージは短いテンプレートに留める
    - `extensions["recover"]`（`action` / `sync` / `inserted` / `context`）の詳細固定は Step2/Step3 の次段（Implementation 追随）で段階導入する

### Step 4: 他 WS との整合チェック（Cut/Label/Lex）
- WS1（Cut）: committed 失敗と回復の優先度が矛盾していないか
- WS2（Label）: 回復時にも `label` が期待集合へ残るか（期待がトークン列だけに崩れないか）
- WS3（Lex）: 同期点が字句ヘルパ（`symbol/keyword`）で自然に書けるか

## リスクと緩和
- 回復の導入で誤った AST が広がる  
  → `Diagnostic` を必ず添付し、IDE 表示用途は許容しても、ビルド用途は `RunConfig` で厳格モードを選べる設計を維持する
