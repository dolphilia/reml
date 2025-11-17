# block_scope トレース (Rust Frontend) - 2025-11-18

```
[000] TraceEvent::ExprEnter(kind = "block", trace_id = "syntax:expr-block")
[001] TraceEvent::ExprEnter(kind = "let", trace_id = "syntax:expr-let")
[002] TraceEvent::ExprLeave(kind = "let", trace_id = "syntax:expr-let")
[003] TraceEvent::ExprEnter(kind = "var", trace_id = "syntax:expr-var")
[004] TraceEvent::ExprLeave(kind = "var", trace_id = "syntax:expr-var")
[005] TraceEvent::ExprLeave(kind = "block", trace_id = "syntax:expr-block")
```

- `ExprParser` の `TraceEvent` 拡張を確認するための最小セット。
- コマンド: `cargo run --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/block_scope.reml --trace-output reports/spec-audit/ch1/block_scope-20251118-trace.md`
