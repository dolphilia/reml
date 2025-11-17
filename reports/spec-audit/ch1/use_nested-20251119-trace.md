# use_nested-20251119 Trace

- Sample: `docs/spec/1-1-syntax/examples/use_nested.reml`
- CI_RUN_ID: `rust-frontend-w37-20251119.1`
- Commit: `f9e10ae676bca22ed8a41e96d79f667310274990`

| Seq | Event | Span | trace_id | 備考 |
| --- | --- | --- | --- | --- |
| 1 | `module_stage_entered(stage="Header")` | 0-32 | `syntax:module-stage::header` | `ModuleStage::Header` を通過した時点でモジュール識別子と属性を確定。 |
| 2 | `module_stage_entered(stage="UseList")` | 34-201 | `syntax:module-stage::use-list` | 3 本の `use` 宣言をまとめて処理。 |
| 3 | `module_stage_entered(stage="DeclList")` | 203-688 | `syntax:module-stage::decl-list` | `match_nested` を含む宣言リスト処理を開始。 |
| 4 | `module_decl_accepted(kind="function")` | 270-647 | `syntax:module-decl::function` | `module_decl_accepts_use_nested` テストで `SyntaxGap::Closed` を確認。 |

- Trace sink: `TraceEvent::ModuleStageEntered`/`ModuleDeclAccepted` を `reports/spec-audit/ch1/use_nested-20251119-diagnostics.json` と同一 ID で出力。  
- Dual-write: `scripts/poc_dualwrite_compare.sh use_nested --ci-run rust-frontend-w37-20251119.1` の結果は `reports/spec-audit/ch1/module_parser-20251119-dualwrite.md` へ保存。  
