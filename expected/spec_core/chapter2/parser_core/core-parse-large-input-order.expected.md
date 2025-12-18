# core-parse-large-input-order（CP-WS5-001）計測メモ

このファイルは、`examples/spec_core/chapter2/parser_core/core-parse-large-input-order.reml` を基に
10MB 級入力を生成して回帰用の観測結果を残すためのメモである。

## 目的
- WS5 Step2 で定義した「オーダー異常検知」を、Phase4 の回帰運用へ接続する（CP-WS5-001）。
- 絶対値ではなく **入力サイズに対する増え方**を確認する。

## 生成方針（暫定）
- ベース: `examples/spec_core/chapter2/parser_core/core-parse-large-input-order.reml`
- パディング領域: `WS5-LARGE-INPUT-PADDING` コメントを複製してファイルサイズを調整する
  - 目安: 1KB / 100KB / 10MB の 3 サイズを作る
- 末尾は **構文エラー**（未閉じ `(`）のままにし、EOF 近傍まで入力を走査させる

## 生成・実行（自動化スクリプト）
手作業ではなく、`tooling/examples/` のスクリプトで生成→実行→ログ保存まで行う。

- スクリプト: `tooling/examples/gen_ws5_large_input.py`
- 出力（生成入力）:
  - `reports/spec-audit/ch4/generated/ws5/CP-WS5-001/core-parse-large-input-order.1kb.reml`
  - `reports/spec-audit/ch4/generated/ws5/CP-WS5-001/core-parse-large-input-order.100kb.reml`
  - `reports/spec-audit/ch4/generated/ws5/CP-WS5-001/core-parse-large-input-order.10mb.reml`
- 出力（実行ログ）:
  - `reports/spec-audit/ch4/logs/spec_core-CP-WS5-001-<size>-<timestamp>.diagnostic.json`

実行例:

```sh
python3 tooling/examples/gen_ws5_large_input.py --sizes 1kb,100kb,10mb
```

メモ追記（任意）:

```sh
python3 tooling/examples/gen_ws5_large_input.py --sizes 1kb,100kb,10mb --update-notes
```

## 観測対象（既存の JSON から取得）
`reml_frontend --output json <generated.reml>` の JSON から、次を記録する。

- `summary.stats.parse_result.packrat_stats`（hits/queries/entries/approx_bytes 等）
- `summary.stats.parse_result.farthest_error_offset`
- `diagnostics[].location`（大入力でも line/column が破綻していないか）

## 合否（暫定）
- `packrat_stats` / `farthest_error_offset` が入力サイズ増加に対して不自然に跳ね上がらない
  - 例: 100KB→10MB で `entries` が入力サイズ比以上に爆発していない
- Unicode 混在の有無は CP-WS5-002（別シナリオ）で固定する

## TODO
- 生成と計測を自動化する（Phase4 の `tooling/examples/` へ投入候補）
- 記録フォーマット（Markdown or JSON）を決め、`reports/spec-audit/ch4/logs/` に保存する運用へ寄せる
