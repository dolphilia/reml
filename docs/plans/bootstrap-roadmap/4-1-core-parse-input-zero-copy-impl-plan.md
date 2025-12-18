# Phase4: Core.Parse Input/Zero-copy 実装計画

## 背景と目的
`Core.Parse` は仕様上すでに `Input` を「参照共有の不変ビュー（ゼロコピー）」として定義している（`docs/spec/2-1-parser-type.md`）。
一方で Phase4（spec_core 回帰）では、診断・回復・Packrat 等の改善が進むほど `Input` の派生（`rest`/`mark/rewind`）がホットパス化し、**部分文字列生成**や **Unicode 位置の都度走査**が混入すると性能と診断表示の両方が崩れうる。

本計画は、WS5（Input/Zero-copy）の決定事項を Phase4 実装へ接続し、性能指針（10MB 線形、メモリ 2x 以内）を守れる状態へ段階的に導くための作業導線を提供する。

- 出典（WS5）: `docs/plans/core-parse-improvement/1-4-input-zero-copy-plan.md`
- 設計指針: `docs/spec/0-1-project-purpose.md`

## スコープ
- 対象: Rust 実装の `Core.Parse` 入力モデル（`Input`/`Span`/`mark/rewind`/メモ化キー）
- 非対象: 新しい入力モデルの発明、API の全面置換（まずは仕様前提の「逸脱」をなくす）

## 仕様根拠
- `docs/spec/2-1-parser-type.md`（入力モデル `Input`、`mark/rewind`、`MemoKey`）
- `docs/spec/3-3-core-text-unicode.md`（列=グラフェム、`Span` ハイライト整合、`g_index/cp_index` 再利用）

## Step 0（完了条件の先出し）: Input 不変条件チェックリストを確定する
WS5 Step1 の「実装監査」へ進む前提として、仕様が要求する不変条件を **監査可能なチェック項目**へ落とす。

- 成果物:
  - `docs/plans/bootstrap-roadmap/checklists/core-parse-input-invariants.md`
- 完了判定:
  - `rest`/`mark/rewind`/Unicode 位置/診断整合/Packrat とメモリ上限について、最低限の “逸脱検知” ができる
  - WS5 計画（`docs/plans/core-parse-improvement/1-4-input-zero-copy-plan.md`）と矛盾しない

## 次のステップ（WS5 との対応）
- Step1: 実装監査（Rust/OCaml の現状点検）を行い、根拠を `docs/notes/core-parse-api-evolution.md` 等へ記録する
- Step2: 大入力向けの “オーダー異常検知” を回帰可能な指標へ落とす（絶対値ではなく増え方の監視）
- Step3: Phase4 シナリオへ接続（大入力 + Unicode 位置の固定）

## 監査結果（Step1 先行メモ）
初回の監査メモは次に記録した（WS5 Step1）。

- `docs/notes/core-parse-api-evolution.md` の `2025-12-18: WS5 Step1 Input/Zero-copy 実装監査メモ（Rust runtime）`

現状の主要論点（要是正）:
- `Input::new` が入力全体を複製しやすく、大入力でメモリ 2x 指針を破りやすい
- `advance`/`span_highlight` が都度走査に依存し、グラフェム境界キャッシュ（`g_index/cp_index`）の共有がない
- `remaining()` が UTF-8 境界前提で、誤用時に panic しうる（bytes 指定の `advance` と相性が悪い）

## 次のアクション（Phase4 側の具体）
- 入力二重確保の回避（`Arc<str>` 受け渡し or `Bytes` 受け渡しの導線）を先に入れて、WS5 Step3（大入力回帰）に備える
- `Input` の境界キャッシュ共有（または遅延位置計算）を検討し、WS5 Step2 の指標（増え方監視）と結び付ける

### Step2 で優先する観測（既存 `RunConfig.profile`）
WS5 Step2 の指標は、現行 Rust runtime が出力できる `ParserProfile`（追加カウンタ無し）を最小セットとして採用する。

- `packrat_hits` / `packrat_misses`（Packrat 有効時）
- `backtracks`（巻き戻し）
- `recoveries`（回復。`mode="off"|"collect"` を分ける）
- `left_recursion_guard_hits`（左再帰ガード。WS6 と切り分ける）
- `memo_entries`（メモ化テーブルのサイズ）

回帰（CP-WS5-001）では、入力サイズ（1KB/100KB/10MB）の増加に対して、これらが不自然に跳ね上がらないこと（オーダー異常がないこと）を “まず” 固定する。
