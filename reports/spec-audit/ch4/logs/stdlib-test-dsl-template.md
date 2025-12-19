# Phase 4 stdlib Core.Test.Dsl ログ

- 生成時刻: 2025-12-19 19:48:30Z
- 対象: CH3-TEST-410 / CH3-TEST-411 / CH3-TEST-412

## 実行詳細

### CH3-TEST-410

- ファイル: `examples/practical/core_test/dsl/ast_matcher_basic.reml`
- 期待 Diagnostics: `[]`
- 実際 Diagnostics: `[]`
- Exit code: 0
- CLI: `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_test/dsl/ast_matcher_basic.reml`
- run_id: `afc1980e-4213-45eb-a5cd-d5e011d8f84e`

### CH3-TEST-411

- ファイル: `examples/practical/core_test/dsl/error_expectation_basic.reml`
- 期待 Diagnostics: `[]`
- 実際 Diagnostics: `[]`
- Exit code: 0
- CLI: `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_test/dsl/error_expectation_basic.reml`
- run_id: `930c829f-bd68-4e56-a4f7-a0b76717c8a0`

### CH3-TEST-412

- 入力: `examples/practical/core_test/dsl/golden/basic.input`
- 期待 AST: `expected/practical/core_test/dsl/golden/basic.ast`
- 期待 Error: `expected/practical/core_test/dsl/golden/basic.error`
- CLI: なし（ファイル配置を確認）
- 備考: `.input`/`.ast`/`.error` の 3 点セットが揃っていることを確認する。
