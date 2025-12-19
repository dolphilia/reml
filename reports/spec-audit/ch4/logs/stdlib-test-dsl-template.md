# Phase 4 stdlib Core.Test.Dsl ログ（テンプレート）

- 生成時刻: YYYY-MM-DD HH:MM:SSZ
- 対象: CH3-TEST-410 / CH3-TEST-411 / CH3-TEST-412

## 実行詳細

### CH3-TEST-410

- ファイル: `examples/practical/core_test/dsl/ast_matcher_basic.reml`
- 期待 Diagnostics: `[]`
- 実際 Diagnostics: `[]`
- Exit code: 0
- CLI: `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_test/dsl/ast_matcher_basic.reml`
- run_id: `<run_id>`

### CH3-TEST-411

- ファイル: `examples/practical/core_test/dsl/error_expectation_basic.reml`
- 期待 Diagnostics: `["parser.unexpected_eof"]`
- 実際 Diagnostics: `["parser.unexpected_eof"]`
- Exit code: 0
- CLI: `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_test/dsl/error_expectation_basic.reml`
- run_id: `<run_id>`

### CH3-TEST-412

- 入力: `examples/practical/core_test/dsl/golden/basic.input`
- 期待 AST: `expected/practical/core_test/dsl/golden/basic.ast`
- 期待 Error: `expected/practical/core_test/dsl/golden/basic.error`
- CLI: `<golden_case 実行コマンド>`
- 備考: `.input`/`.ast`/`.error` の 3 点セットが揃っていることを確認する。
