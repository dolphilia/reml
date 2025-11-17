# effect_handler トレース (Rust Frontend) - 2025-11-18

```
[000] TraceEvent::ExprEnter(kind = "handle", trace_id = "syntax:expr-handle")
[001] TraceEvent::ExprEnter(kind = "operation", trace_id = "syntax:expr-operation")
[002] TraceEvent::ExprLeave(kind = "operation", trace_id = "syntax:expr-operation")
[003] TraceEvent::ExprLeave(kind = "handle", trace_id = "syntax:expr-handle")
```

- `ExprParser` を `module_parser` から分離した新実装で採取。
- 収集コマンド: `cargo run --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/effect_handler.reml --trace-output reports/spec-audit/ch1/effect_handler-20251118-trace.md`
- 参照: `docs/plans/rust-migration/unified-porting-principles.md` §2 `diff-harness` 観測点。
