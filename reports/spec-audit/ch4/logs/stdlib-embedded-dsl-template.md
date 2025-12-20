# Phase 4 stdlib Embedded DSL Composability ログ

- 生成時刻: YYYY-MM-DD HH:MM:SSZ
- 対象: CH4-DSL-COMP-001 / CH4-DSL-COMP-002 / CH4-DSL-COMP-003

## 実行詳細

### CH4-DSL-COMP-001

- ファイル: `examples/practical/embedded_dsl/markdown_reml_basic.reml`
- 期待 Diagnostics: `[]`
- 実際 Diagnostics: `[]`
- Exit code: 0
- CLI: `compiler/rust/frontend/target/debug/reml_frontend --output json examples/practical/embedded_dsl/markdown_reml_basic.reml`
- run_id: `TBD`
- expected: `expected/practical/embedded_dsl/markdown_reml_basic.audit.jsonl`
- 備考: `dsl.id`/`dsl.embedding.span`/`dsl.embedding.mode` の JSON Lines を確認する。

### CH4-DSL-COMP-002

- ファイル: `examples/practical/embedded_dsl/markdown_reml_error.reml`
- 期待 Diagnostics: `["parser.unexpected_eof"]`
- 実際 Diagnostics: `[]`
- Exit code: 0
- CLI: `compiler/rust/frontend/target/debug/reml_frontend --output json examples/practical/embedded_dsl/markdown_reml_error.reml`
- run_id: `TBD`
- expected: `expected/practical/embedded_dsl/markdown_reml_error.diagnostic.json`
- 備考: `source_dsl` と `audit_metadata["dsl.id"]` の出力を確認する。

### CH4-DSL-COMP-003

- ファイル: `examples/practical/embedded_dsl/markdown_reml_parallel.reml`
- 期待 Diagnostics: `[]`
- 実際 Diagnostics: `[]`
- Exit code: 0
- CLI: `compiler/rust/frontend/target/debug/reml_frontend --output json examples/practical/embedded_dsl/markdown_reml_parallel.reml`
- run_id: `TBD`
- expected: `expected/practical/embedded_dsl/markdown_reml_parallel.audit.jsonl`
- 備考: `dsl.embedding.mode=ParallelSafe` が監査メタデータに含まれることを確認する。
