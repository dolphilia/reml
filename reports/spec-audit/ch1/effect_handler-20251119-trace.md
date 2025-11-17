# effect_handler-20251119 Trace

- Sample: `docs/spec/1-1-syntax/examples/effect_handler.reml`
- CI_RUN_ID: `rust-frontend-w37-20251119.1`
- Commit: `f9e10ae676bca22ed8a41e96d79f667310274990`

| Seq | Event | Span | trace_id | 備考 |
| --- | --- | --- | --- | --- |
| 1 | `module_stage_entered(stage="Header")` | 0-30 | `syntax:module-stage::header` | `module demo.effect.logger` を確定。 |
| 2 | `module_stage_entered(stage="DeclList")` | 32-812 | `syntax:module-stage::decl-list` | UseList 無しで DeclList へ遷移。 |
| 3 | `module_decl_accepted(kind="effect")` | 60-210 | `syntax:module-decl::effect` | `effect log` の署名を受理。 |
| 4 | `module_decl_accepted(kind="handler")` | 212-602 | `syntax:module-decl::handler` | `operation log(args, resume)` の `resume` 注釈を検証。 |
| 5 | `module_decl_accepted(kind="function")` | 604-812 | `syntax:module-decl::function` | `perform`/`handle` を含む `main` を受理。 |

- Dual-write: `scripts/poc_dualwrite_compare.sh effect_handler --ci-run rust-frontend-w37-20251119.1` により差分 0。  
- 証跡: `reports/spec-audit/ch1/effect_handler-20251119-diagnostics.json`、`effect_handler-20251118-diagnostics.json`、`effect_handler-20251118-dualwrite.md`。  
