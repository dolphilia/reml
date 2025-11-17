# effect_handler トレース (Rust Frontend) - 2025-11-18

```
[000] TraceEvent::ExprEnter(kind = "handle", trace_id = "syntax:expr::handle")
[001] TraceEvent::ExprEnter(kind = "perform", trace_id = "syntax:expr::perform")
[002] TraceEvent::EffectEnter(trace_id = "syntax:effect::perform", label = "Console.ask")
[003] TraceEvent::OperationResume(trace_id = "syntax:operation::resume", label = "resume")
[004] TraceEvent::EffectExit(trace_id = "syntax:effect::perform", label = "Console.ask")
[005] TraceEvent::ExprLeave(kind = "perform", trace_id = "syntax:expr::perform")
[006] TraceEvent::ExprLeave(kind = "handle", trace_id = "syntax:expr::handle")
```

- `ExprParser` を `module_parser` から分離した新実装で採取。
- 収集コマンド: `cargo run --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/effect_handler.reml --trace-output reports/spec-audit/ch1/effect_handler-20251118-trace.md`
- 参照: `docs/plans/rust-migration/unified-porting-principles.md` §2 `diff-harness` 観測点。
