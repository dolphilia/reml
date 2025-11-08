| case | gating | schema | metrics | diag_match | parser_audit (ocaml/rust) | parser_expected (ocaml/rust) | stream_outcome (ocaml/rust) | effects_regressions (ocaml/rust) | diag_counts (ocaml/rust) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| cli_merge_warnings | ❌ | ✅ | ❌ | ❌ | - / 0.000 | - / 0.000 | - / - | - / 0 | 0 / 1 |
| cli_packrat_switch | ❌ | ✅ | ❌ | ❌ | - / 0.000 | - / 0.000 | - / - | - / 0 | 0 / 2 |
| cli_trace_toggle | ❌ | ✅ | ❌ | ❌ | 1.000 / 0.000 | 1.000 / 0.000 | - / - | 0 / 0 | 1 / 4 |
| effect_residual_leak | ❌ | ✅ | ❌ | ❌ | 1.000 / 0.000 | 1.000 / 0.000 | - / - | 0 / 0 | 1 / 5 |
| effect_stage_cli_override | ❌ | ✅ | ❌ | ❌ | 1.000 / 0.000 | 1.000 / 0.000 | - / - | 0 / 0 | 1 / 6 |
| ffi_async_dispatch | ❌ | ✅ | ❌ | ❌ | - / 0.000 | - / 0.000 | - / - | - / 0 | 0 / 42 |
| ffi_ownership_mismatch | ❌ | ✅ | ❌ | ❌ | 1.000 / 0.000 | 1.000 / 0.000 | - / - | 0 / 0 | 1 / 64 |
| ffi_stage_messagebox | ❌ | ✅ | ❌ | ❌ | 1.000 / 0.000 | 1.000 / 0.000 | - / - | 0 / 0 | 1 / 64 |
| lsp_diagnostic_stream | ❌ | ✅ | ❌ | ❌ | 1.000 / 0.000 | 1.000 / 0.000 | - / - | 0 / 0 | 1 / 4 |
| lsp_hover_internal_error | ❌ | ✅ | ❌ | ❌ | - / 0.000 | - / 0.000 | - / - | - / 0 | 0 / 1 |
| lsp_workspace_config | ❌ | ✅ | ❌ | ❌ | - / 0.000 | - / 0.000 | - / - | - / 0 | 0 / 2 |
| recover_else_without_if | ❌ | ✅ | ❌ | ❌ | 1.000 / - | 1.000 / 0.000 | - / - | 0 / 0 | 1 / 0 |
| recover_lambda_body | ❌ | ✅ | ❌ | ❌ | 1.000 / 0.000 | 1.000 / 0.000 | - / - | 0 / 0 | 1 / 2 |
| recover_missing_semicolon | ❌ | ✅ | ❌ | ✅ | 1.000 / 0.000 | 1.000 / 0.000 | - / - | 0 / 0 | 1 / 1 |
| recover_missing_tuple_comma | ❌ | ✅ | ❌ | ✅ | 1.000 / 0.000 | 1.000 / 0.000 | - / - | 0 / 0 | 1 / 1 |
| recover_unclosed_block | ❌ | ✅ | ❌ | ✅ | 1.000 / 0.000 | 1.000 / 0.000 | - / - | 0 / 0 | 1 / 1 |
| stream_backpressure_hint | ❌ | ✅ | ❌ | ❌ | - / 0.000 | - / 0.000 | - / - | - / 0 | 0 / 5 |
| stream_checkpoint_drift | ❌ | ✅ | ❌ | ❌ | 1.000 / 0.000 | 1.000 / 0.000 | - / - | 0 / 0 | 1 / 4 |
| stream_pending_resume | ❌ | ❌ | ❌ | ❌ | 1.000 / 0.000 | 0.000 / 0.000 | - / - | 0 / 0 | 1 / 11 |
| type_condition_bool | ❌ | ✅ | ❌ | ❌ | - / 0.000 | - / 0.000 | - / - | - / 0 | 0 / 1 |
| type_condition_literal_bool | ❌ | ✅ | ❌ | ✅ | - / - | - / 0.000 | - / - | - / 0 | 0 / 0 |
