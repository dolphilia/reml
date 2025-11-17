# block_scope-20251119 Trace

- Sample: `docs/spec/1-1-syntax/examples/block_scope.reml`
- CI_RUN_ID: `rust-frontend-w37-20251119.1`
- Commit: `f9e10ae676bca22ed8a41e96d79f667310274990`

| Seq | Event | Span | trace_id | 備考 |
| --- | --- | --- | --- | --- |
| 1 | `module_stage_entered(stage="Header")` | 0-20 | `syntax:module-stage::header` | `module block.scope` を受理。 |
| 2 | `module_stage_entered(stage="DeclList")` | 22-278 | `syntax:module-stage::decl-list` | DeclList へ即遷移（UseList 無し）。 |
| 3 | `module_decl_accepted(kind="function")` | 60-260 | `syntax:module-decl::function` | `Block`/`Let`/`Var` を含む `scoped_assignments` を受理。 |

- `TraceEvent::ExprEnter(kind="block")` / `ExprLeave` は CLI `--trace-output` 側で出力され、`syntax:expr::block` のトレースと突き合わせ済み。  
- 監査ログ: `reports/spec-audit/ch1/block_scope-20251119-diagnostics.json`、`block_scope-20251118-diagnostics.json` の両方を保存し、履歴比較を行う。  
