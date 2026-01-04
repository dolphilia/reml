# Core.Parse Input 不変条件チェックリスト（Phase4 用）

## 目的
`docs/spec/2-1-parser-type.md` が規定する `Input`（参照共有の不変ビュー / ゼロコピー）を、Rust 実装と Phase4 回帰運用へ落とし込むためのチェックリストです。

- 設計指針: `docs/spec/0-1-project-purpose.md`（10MB 線形、メモリ 2x 以内）
- 仕様根拠: `docs/spec/2-1-parser-type.md`（入力モデル `Input` / `mark/rewind` / `MemoKey`）
- Unicode/表示根拠: `docs/spec/3-3-core-text-unicode.md`（列=グラフェム、`Span` ハイライト整合）
- 計画の出典（WS5 Step0）: `docs/plans/core-parse-improvement/1-4-input-zero-copy-plan.md`

この文書は **監査（WS5 Step1）** と **回帰追加（WS5 Step3）** の実務メモとして使う。
チェックを満たさない場合は、修正方針と影響範囲（どのシナリオ/期待出力が揺れるか）を併記して記録する。

## 用語
- 「コピー」: 部分文字列（`String` / `Vec<u8>` / `Bytes`）の新規確保を伴うこと（参照共有/COW の参照増加はコピーではない）
- 「列」: `Core.Text` の **拡張書記素（grapheme cluster）** を 1 として数える（表示幅・コードポイント数ではない）
- 「ホットパス」: 字句化（lex）/分岐（`or`）/繰り返し（`many`）/バックトラック（`attempt`）で反復実行される経路

## チェックリスト

### 1) ゼロコピー（不変ビュー）
- [ ] `rest` は **オフセット更新**であり、同一バッファ参照（`Input.bytes`）のビューとして表現される
- [ ] ホットパスで部分文字列（`String`/`Vec<u8>` など）を新規生成しない（必要なら “外側” の API で遅延生成）
- [ ] `Input` の派生（ビューの生成/複製）が **O(1)**（入力長に比例した走査を含まない）
- [ ] `cp_index/g_index` は **必要時だけ**構築し、同一入力バッファに対してビュー間で共有される（`rest` ごとの再構築をしない）

### 2) `mark/rewind`（スナップショット）
- [ ] `mark()` は **O(1)**（`byte_off/line/column` + 参照共有の保持のみ）
- [ ] `rewind(mark)` は `bytes` を変えずに位置だけ戻す（`byte_off/line/column` と境界キャッシュが矛盾しない）
- [ ] `attempt`/`or`/`many` の組み合わせで、`mark/rewind` が入力長に比例する処理を含まない

### 3) Unicode 位置（行/列/Span）
- [ ] 行/列/Span が `docs/spec/2-1-parser-type.md` と一致する
- [ ] 列はグラフェム境界に整合し、結合文字・絵文字でも崩れない
- [ ] `Span` の `byte_*` と `line/col` が矛盾しない（同じ範囲を指す）
- [ ] 列の算出で **先頭からの都度スキャン**をしない（`g_index` 等のキャッシュ再利用が前提）

### 4) 診断/回復との整合
- [ ] `attempt` の巻き戻しで、入力ビューだけでなく診断状態（最遠エラー・期待集合・回復メタ）も破綻しない
- [ ] 回復（`recover`）に伴う入力スキップが `Span`・行/列へ正しく反映される

### 5) Packrat とメモリ上限
- [ ] `MemoKey` は `byte_off` 等の入力位置 ID を用い、部分文字列スライスをキーにしない
- [ ] `MemoVal`（`Reply`）が `Input` を保持しても、部分文字列の新規確保や巨大な派生オブジェクトを作らない
- [ ] 10MB 級入力でピークメモリが **入力サイズの 2 倍以内**という方針を破らない（計測は WS5 Step2 で段階導入）

## 違反兆候（監査の着眼点）
- `to_string` / `String` 生成 / `substring` 相当が Core.Parse の内側ループで頻発している
- `g_index/cp_index` の構築が `rest` のたびに走っている（ビュー単位の再構築）
- `mark/rewind` が行頭からの再走査など “位置再計算” を含む

