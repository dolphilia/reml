# WS5: Input/Zero-copy（入力抽象と性能）計画

## 背景と狙い
調査メモ `docs/notes/core-parse-improvement-survey.md` は、Nom の zero-copy 設計に触れ、`Text` の部分文字列生成コストが懸念される点を指摘している。
一方、Reml 仕様は既に `Input` を「参照共有の不変ビュー（ゼロコピー）」として定義している（`docs/spec/2-1-parser-type.md`）。

本ワークストリームは、仕様の前提を満たす入力モデルが **実装と運用に降りているか**を確認し、性能指針（`docs/spec/0-1-project-purpose.md`）に沿った形へ整備する。

## 目標
- `Input` の `rest` 生成がコピーではなく「オフセットの更新」であることが保証される
- 行/列/Span の算出が Unicode モデルに整合し、診断表示が崩れない
- 10MB 級入力でも解析が線形近似の特性を維持する（測定方法は段階導入）

## 検討項目
- `Input` が保持すべき最小情報（参照・オフセット・長さ・行頭インデックス等）
- `mark()/rewind()` のコスト（バックトラックと Packrat の鍵）
- `Core.Text` と `Core.Parse.Input` の境界（UTF-8、正規化、改行取り扱い）

## タスク分割
### Step 0: “仕様上の Input 不変条件” をチェックリスト化する
本 WS は「新しい入力モデルを発明する」のではなく、`docs/spec/2-1-parser-type.md` が前提としているゼロコピー特性を、実装・回帰・運用へ落とし込むための作業である。

- 参照
  - `docs/spec/2-1-parser-type.md`（`Input` は参照共有の不変ビュー、`mark/rewind`）
  - `docs/spec/0-1-project-purpose.md`（10MB 線形、メモリ 2x 以内）
  - `docs/spec/3-3-core-text-unicode.md`（列=グラフェム、表示幅）
- 産物
  - 「Input 不変条件チェックリスト（暫定）」を本計画内に作成（次 Step で監査に使う）
  - Phase4 側の運用チェックリストとして `docs/plans/bootstrap-roadmap/checklists/core-parse-input-invariants.md` に転記（実装者の作業導線を揃える）

#### Input 不変条件チェックリスト（暫定）
このチェックリストは、WS5 Step1 の「実装監査」および Step3 の「回帰（Unicode 位置）」で参照する。
とくに `Input` は **「見た目は不変」だが内部ではキャッシュを持ちうる**ため、ゼロコピー前提を破る実装（部分文字列生成・都度スキャン）が混入しやすい。

##### 0. 用語とスコープ
- `Input` は `docs/spec/2-1-parser-type.md` の入力モデル（`bytes` + `byte_off` + `line/column` + `cp_index/g_index`）
- 「コピー」とは、部分文字列（`String` / `Vec<u8>` / `Bytes` の新規確保）を伴うことを指す（参照共有/COW の “参照の増加” はコピーではない）
- 「列」は `Core.Text` の **拡張書記素（grapheme cluster）** を 1 として数える（表示幅やコードポイント数ではない）

##### 1. ゼロコピー（不変ビュー）不変条件
- `rest` は **オフセット更新**であり、同一バッファ参照（`Input.bytes`）のビューとして表現される
- `Input.bytes` から部分文字列を生成して返す API を、`Core.Parse` のホットパス（字句化/分岐/繰り返し）に持ち込まない
- `Input` の生成・移動・複製（ビューの派生）は **O(1)**（入力長に比例した走査をしない）
- `Input` のビュー同士で `cp_index/g_index` を共有できる（同一 `bytes` に対して “毎回” 新規構築しない）

##### 2. スナップショット（`mark/rewind`）不変条件
- `mark()` は **O(1)** でスナップショット化できる（`byte_off/line/column` と参照共有の保持のみ）
- `rewind(mark)` は **同じ `bytes` を指すビュー**へ戻る（`byte_off/line/column` と境界キャッシュの整合が取れる）
- `attempt`/`or`/`many` など「巻き戻しが頻発する組み合わせ」で、`mark/rewind` がボトルネックにならない（少なくとも入力長に比例する処理を含まない）

##### 3. Unicode 位置（行/列/Span）不変条件
- 行/列/Span は `docs/spec/2-1-parser-type.md` と `docs/spec/3-3-core-text-unicode.md` の前提に一致する
  - 行: `LF` 正規化（`CRLF` 等の取り扱いは `Core.Text` へ寄せ、パーサ側で独自に二重実装しない）
  - 列: グラフェム境界に整合（結合文字・絵文字でも列が崩れない）
- `Span` は（少なくとも）`byte_start/byte_end` と `line/col` の両方が矛盾しない（同じ範囲を指す）
- `column` の更新に必要なグラフェム境界情報は、**都度 “先頭から” 走査して求めない**（`g_index` 等のキャッシュ利用を前提とする）

##### 4. 診断・回復と入力ビューの整合
- `attempt` の巻き戻しが、入力ビューだけでなく **診断状態**（最遠エラー/期待集合/回復メタ）とも整合する
  - 「右枝へ進んだが、失敗後に左枝の期待が残る」等の破綻が起きない
- 回復（WS4）が有効な場合でも、同期に伴う入力スキップが `Span`・行/列へ正しく反映される

##### 5. Packrat（メモ化）とメモリ上限
- `MemoKey` は `byte_off` など **入力位置の ID** を用い、`Input` のスライス（部分文字列）をキーにしない
- `MemoVal`（`Reply`）が `Input` を保持しても、追加の部分文字列確保や “巨大な派生オブジェクト” を伴わない
- 10MB 級入力で、ピークメモリが **入力サイズの 2 倍以内**というプロジェクト指針を破る設計を持ち込まない

##### 6. 監査時の「違反兆候」（簡易）
- `to_string` / `String` 生成 / `substring` 相当が `Core.Parse` の内側ループで頻発している
- `g_index/cp_index` の構築が `rest` のたびに走っている（ビュー単位の再構築）
- `mark/rewind` が `Input` の再計算（例: 行頭からの再走査）を含む

### Step 1: 実装監査（ゼロコピーが破れていないかを点検する）
ドキュメントだけで終わらせず、現行実装（Rust/OCaml）を “監査” して根拠を残す。

- 監査対象
  - Rust: `compiler/rust/runtime/src/parse/`（Input/Span/Combinator）
  - OCaml: `compiler/ocaml/docs/` 配下の入力モデルメモ（存在する場合）
- 点検観点
  - 部分文字列生成（`substring` 相当）がホットパスに入っていないか
  - UTF-8 → 列（グラフェム）変換が都度走っていないか（境界キャッシュの有無）
  - Packrat のキーが「入力ビュー」を保持して過剰にメモリを抱え込まないか
- 記録方法
  - 結果を `docs/notes/core-parse-api-evolution.md` に “監査メモ” として短く追記し、どの不変条件が満たされている/いないかを明記する

### Step 2: パフォーマンス指標の定義（回帰可能な形へ落とす）
実測の数値は環境差があるため、初期は「退行検出できる形（オーダー異常）」を優先する。

- 指標
  - 入力サイズ別の処理時間（1KB/100KB/10MB）
  - backtrack 回数・packrat hit/miss（Phase10 の profile と整合）
  - 最大メモ化エントリ数（入力サイズに対して線形に増えるか）
- “測り方” の決定
  - ベンチマーク数値の絶対値ではなく、**大域的な増え方**が異常でないことを確認する
  - 観測フック（`RunConfig.profile`）の利用を前提にし、デフォルト OFF を維持する

### Step 3: 回帰への接続（大入力と Unicode 位置をセットで固定）
- 回帰登録
  - 計画起点 ID: `CP-WS5-001`（大入力でのオーダー異常がない）
  - 計画起点 ID: `CP-WS5-002`（Unicode を含む入力で列/Span が崩れない）
- サンプル
  - `examples/spec_core/chapter2/parser_core/` に、Unicode 混在（全角・結合文字・絵文字）で位置が揃うことを確認できる入力例を追加する
- 注意
  - 性能回帰は CI 差の影響が強いため、Phase10 の観測結果（hit/miss/backtracks）と併用し、まずは “異常検知” を固定する

## リスクと緩和
- ゼロコピーを優先しすぎると API が使いにくくなる  
  → 「表面 API は簡潔」「内部はスライス」の分離を維持し、ユーザーが `Input` を意識しなくてよい設計に寄せる
