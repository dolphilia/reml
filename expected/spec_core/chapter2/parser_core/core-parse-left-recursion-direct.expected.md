# core-parse-left-recursion-direct（CP-WS6-001）期待メモ

このファイルは、`examples/spec_core/chapter2/parser_core/core-parse-left-recursion-direct.reml` を用いて
**左再帰の自己呼出検出**（`E4001`）を回帰に固定するための期待メモである。

## 目的
- `RunConfig.left_recursion="off"` のときに左再帰が検出されることを確認する。
- 診断の **存在** と **位置** を固定し、メッセージ詳細は段階導入にする。

## 実行条件（暫定）
- `left_recursion="off"` を明示する（安全弁としての検出を有効化）。
- Packrat は有効（`packrat=true`）を推奨。

## 期待
- 診断キー: `E4001`（2-5 §D-6）
- 位置: `expr_left_recursion_direct` 定義の開始付近

## ログ保存（例）
- 出力: `reports/spec-audit/ch4/logs/spec_core-CP-WS6-001-<timestamp>.diagnostic.json`
- 参考: `summary.stats.parse_result.farthest_error_offset`

## 実行ログ（採取済み）
- `reports/spec-audit/ch4/logs/spec_core-CP-WS6-001-20251218T225541Z.diagnostic.json`

## TODO
- 実行用の CLI/ランナー手順を Phase4 側で確定し、`phase4-scenario-matrix.csv` に登録する。
