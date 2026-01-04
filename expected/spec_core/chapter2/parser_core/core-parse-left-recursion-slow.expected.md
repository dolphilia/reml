# core-parse-left-recursion-slow（CP-WS6-002）期待メモ

このファイルは、`examples/spec_core/chapter2/parser_core/core-parse-left-recursion-slow.reml` を用いて
**左再帰ガード時の profile 指標**を回帰に固定するための期待メモである。

## 目的
- `left_recursion="on"` のときに `left_recursion_guard_hits` が記録されることを確認する。
- 絶対値ではなく **0 ではないこと**を初期条件として固定する。

## 実行条件（暫定）
- `left_recursion="on"` と `packrat=true` を明示する。
- `RunConfig.extensions["parse"].profile=true` を有効化する。

## 期待
- `ParseResult.profile.left_recursion_guard_hits > 0`
- `ParseResult.profile.memo_entries > 0`

## プロファイル出力（例）
- `expected/spec_core/chapter2/parser_core/core-parse-left-recursion-slow.profile.json`
- 追加ログ: `reports/spec-audit/ch4/logs/spec_core-CP-WS6-002-<timestamp>.diagnostic.json`

## 実行ログ（採取済み）
- `reports/spec-audit/ch4/logs/spec_core-CP-WS6-002-20251218T225547Z.diagnostic.json`
- `reports/spec-audit/ch4/logs/spec_core-CP-WS6-002-20251218T232113Z.diagnostic.json`

## 備考
- `--parse-driver-left-recursion-parser` を使い、left_recursion_guard を 1 回以上記録できることを確認。
  - 生成物: `expected/spec_core/chapter2/parser_core/core-parse-left-recursion-slow.profile.json`

## TODO
- しきい値は Phase4 の実測値に合わせて調整する。
- 大入力版（繰返し回数を増やした入力）を別ログとして残す。
