# Dual-write Report (/Users/dolphilia/github/kestrel/reports/dual-write/front-end/poc/2025-11-07-w2-ast-inventory)

- ケース数: 9
- AST 一致: 0 / 9
- 診断一致: 1 / 9
- Packrat 一致: 0 / 9

| case | AST | diagnostics | packrat | ocaml_diag | rust_diag | ocaml_packrat | rust_packrat |
| --- | --- | --- | --- | --- | --- | --- | --- |
| add_example | ❌ | ✅ | ❌ | 1 | 1 | 8/7 | 7/6 |
| effectful_sum | ❌ | ❌ | ❌ | 1 | 2 | 8/7 | 7/6 |
| emit_suite_cli | ❌ | ❌ | ❌ | 0 | 2 | 0/0 | 7/6 |
| ffi_callconv_sample | ❌ | ❌ | ❌ | 0 | 32 | 0/0 | 7/6 |
| pattern_examples | ❌ | ❌ | ❌ | 1 | 272 | 8/7 | 7/6 |
| simple_module | ❌ | ❌ | ❌ | 0 | 6 | 0/0 | 7/6 |
| trace_sample_cli | ❌ | ❌ | ❌ | 1 | 4 | 8/7 | 7/6 |
| type_error_cli | ❌ | ❌ | ❌ | 0 | 1 | 0/0 | 7/6 |
| unicode_identifiers | ❌ | ❌ | ❌ | 1 | 139 | 0/0 | 7/6 |

## 差分のあるケース
- `add_example`: AST=False, diag=True, packrat=False (ocaml_diag=1, rust_diag=1, Δdiag=0)
- `effectful_sum`: AST=False, diag=False, packrat=False (ocaml_diag=1, rust_diag=2, Δdiag=1)
- `emit_suite_cli`: AST=False, diag=False, packrat=False (ocaml_diag=0, rust_diag=2, Δdiag=2)
- `ffi_callconv_sample`: AST=False, diag=False, packrat=False (ocaml_diag=0, rust_diag=32, Δdiag=32)
- `pattern_examples`: AST=False, diag=False, packrat=False (ocaml_diag=1, rust_diag=272, Δdiag=271)
- `simple_module`: AST=False, diag=False, packrat=False (ocaml_diag=0, rust_diag=6, Δdiag=6)
- `trace_sample_cli`: AST=False, diag=False, packrat=False (ocaml_diag=1, rust_diag=4, Δdiag=3)
- `type_error_cli`: AST=False, diag=False, packrat=False (ocaml_diag=0, rust_diag=1, Δdiag=1)
- `unicode_identifiers`: AST=False, diag=False, packrat=False (ocaml_diag=1, rust_diag=139, Δdiag=138)
