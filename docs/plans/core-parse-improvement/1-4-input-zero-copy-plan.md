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

#### Input 不変条件チェックリスト（暫定）
- `rest` は **コピーではなく**オフセット更新（同一バッファ参照のスライス）
- `mark()/rewind()` は **O(1)** でスナップショット化できる
- 行/列/Span の計算は `Core.Text` と一致し、列はグラフェム境界に整合する
- `attempt` の巻き戻しが、入力ビューと診断状態の両方で破綻しない

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
