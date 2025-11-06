# 診断差分サマリ（Rust vs OCaml）

`2025-11-07-w2-ast-inventory` バッチの `*.summary.json` から、OCaml 版と Rust 版の診断件数に大きな差分が出ているケースを抜粋した。値は `rust_diag_count - ocaml_diag_count` を示している。

| case | OCaml diagnostics | Rust diagnostics | Δ件数 | 備考 |
| --- | --- | --- | --- | --- |
| ffi_callconv_sample | 0 | 32 | +32 | Rust 側が FFI 呼出しの引数/ABI について大量の `recoverable` 警告を出力。OCaml は成功パスのみ。 |
| pattern_examples | 1 | 272 | +271 | `pattern_examples.reml` で Rust 側が各パターンに対して granular な recover 診断を報告。Menhir ベースの OCaml は 1 件のみ。 |
| unicode_identifiers | 1 | 139 | +138 | Unicode 漢字を含んだサンプル。Rust lexer の recover が毎トークンで発火している。 |
| simple_module | 0 | 6 | +6 | module header の recover ハンドラ差分。 |
| emit_suite_cli | 0 | 2 | +2 | CLI emit suite の missing diagnostics。 |
| trace_sample_cli | 1 | 4 | +3 | streaming trace サンプルで Rust が 3 件追加報告。 |
| type_error_cli | 0 | 1 | +1 | Rust 側のみ recover あり。 |

**アクション**:
- `ffi_callconv_sample`, `pattern_examples`, `unicode_identifiers` の 3 ケースを `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に TODO 登録し、Rust 側診断ロジックの調整（Recover 集約 or ParserExpectation 差分吸収）をフォローアップする。
- `emit_suite_cli`, `simple_module`, `trace_sample_cli`, `type_error_cli` は CLI/lexer レベルの recover テーブル更新で解消できる見込み。追加の diff ログは `reports/dual-write/front-end/poc/2025-11-07-w2-ast-inventory/*.summary.json` を参照。
