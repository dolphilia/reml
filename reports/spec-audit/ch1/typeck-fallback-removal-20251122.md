# Typeck Fallback Removal — 2025-11-22

- `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested.reml --emit-typeck-debug reports/spec-audit/ch1/use_nested-20251122-typeck.json --trace-output reports/spec-audit/ch1/use_nested-20251122-trace.md`  
  - `schema_version = "3.0.0-alpha"`、`stage_trace`、`used_impls` が `typeck` 出力へ追加されたことを確認 (`git rev-parse HEAD = 282c2e8d4a0c4fd4dbd20bd081eb1a7381301380`, `CI_RUN_ID=rust-frontend-w39-20251122.1`)。
- `cargo test --manifest-path compiler/rust/frontend/Cargo.toml typeck_hindley_milner::reports_ast_unavailable_when_module_is_absent -- --nocapture`  
  - `typeck.aborted.ast_unavailable` 診断が `typeck.infer_module(None, ..)` で発火し、本メモに `severity = error / domain = "type"` の確認結果を記録。
- `scripts/poc_dualwrite_compare.sh use_nested --mode typeck --run-id 20251122-w39-typeck --cases docs/plans/rust-migration/appendix/w3-typeck-dualwrite-plan.md`  
  - Rust 側の `typeck/typeck-debug.rust.json` に `stage_trace` と `used_impls` が保存され、`StageAuditPayload` が `schema.version = 3.0.0-alpha` を記録していることを確認。`docs/spec/3-6-core-diagnostics-audit.md` §1.2 へ反映済み。
- 付随更新: `compiler/rust/frontend/src/bin/reml_frontend.rs`・`scripts/poc_dualwrite_compare.sh`・`tooling/lsp/tests/client_compat/fixtures/*.json` が `reml_frontend` 呼び出しへ統一され、旧 `poc_frontend` CLI は完全に廃止された（参照: `docs-migrations.log` エントリ `DOCS-20251122-REM`）。
