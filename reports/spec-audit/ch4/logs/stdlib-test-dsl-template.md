# Phase 4 stdlib Core.Test.Dsl ログ

- 生成時刻: 2025-12-19 20:44:45Z
- 対象: CH3-TEST-410 / CH3-TEST-411 / CH3-TEST-412

## 実行詳細

### CH3-TEST-410

- ファイル: `examples/practical/core_test/dsl/ast_matcher_basic.reml`
- 期待 Diagnostics: `[]`
- 実際 Diagnostics: `[]`
- Exit code: 0
- CLI: `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_test/dsl/ast_matcher_basic.reml`
- run_id: `e4ae1dff-c7e1-4a18-926c-a989033d8320`

### CH3-TEST-411

- ファイル: `examples/practical/core_test/dsl/error_expectation_basic.reml`
- 期待 Diagnostics: `[]`
- 実際 Diagnostics: `[]`
- Exit code: 0
- CLI: `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_test/dsl/error_expectation_basic.reml`
- run_id: `172f60d5-9e35-45da-b5ed-59003551678d`

### CH3-TEST-412

- 入力: `examples/practical/core_test/dsl/golden/basic.input`
- 期待 AST: `expected/practical/core_test/dsl/golden/basic.ast`
- 期待 Error: `expected/practical/core_test/dsl/golden/basic.error`
- CLI: なし（ファイル配置を確認）
- 備考: `.input`/`.ast`/`.error` の 3 点セットが揃っていることを確認する。
