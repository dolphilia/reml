# Phase 4 stdlib Core.Parse.Cst ログ

- 生成時刻: YYYY-MM-DD HH:MM:SSZ
- 対象: CH2-PARSE-930

## 実行詳細

### CH2-PARSE-930

- ファイル: `examples/practical/core_parse/cst_lossless.reml`
- 期待 Diagnostics: `[]`
- 実際 Diagnostics: `[]`
- Exit code: 0
- CLI: `compiler/rust/frontend/target/debug/reml_frontend --output json examples/practical/core_parse/cst_lossless.reml`
- run_id: `TBD`
- stdout 先頭行:

```
{"command":"Check","phase":"Reporting","run_id":"TBD",...}
```

- expected: `expected/practical/core_parse/cst_lossless.stdout`
- 備考: `run_id` は比較対象から除外する。
