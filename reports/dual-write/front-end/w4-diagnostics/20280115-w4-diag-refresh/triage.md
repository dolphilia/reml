# W4 診断差分トリアージ（Run: `20280115-w4-diag-refresh`）

- 対象成果物: `reports/dual-write/front-end/w4-diagnostics/20280115-w4-diag-refresh/`
- 参照手順: `docs/plans/rust-migration/1-2-diagnostic-compatibility.md#1-2-4-差分分類`
- 集計: 21 ケース中 3 ケースで `diag_match=true`（`recover_missing_semicolon|tuple_comma|unclosed_block`）。残り 18 ケースは実装差分またはハーネス未整備によりゲート不可。

## 合格ケース
- `recover_missing_semicolon`
- `recover_missing_tuple_comma`
- `recover_unclosed_block`

## 未解決差分（分類とフォローアップ）

| Case | カテゴリ | 観測差分（主要ログ） | 分類 (1.2.4) | 対応先 |
| --- | --- | --- | --- | --- |
| recover_else_without_if | parser-recover | `summary.json` で Rust 側 `rust_diag_count=0`／OCaml 側 1（`diagnostics.ocaml.json`）。`diagnostics.rust.json` が空で recover 拡張未生成。 | 実装差分（Rust recover 欠落） | TODO: DIAG-RUST-01 |
| recover_lambda_body | parser-recover | Rust が 2 件の無名 parser error を出力（`diagnostics.rust.json`）し、OCaml は 1 件。`expected_tokens` が Rust 側で欠落。 | 実装差分（Rust recover 正規化不足） | TODO: DIAG-RUST-01 |
| type_condition_bool | type/effect | `parser-metrics.ocaml.err.log` が `parser.expected_summary_presence: total=0` を記録しゲート失敗。diag は一致済み。 | 実装差分（計測ハーネス） | TODO: DIAG-RUST-06 |
| type_condition_literal_bool | type/effect | OCaml は `diagnostics.ocaml.json` で `E7006` 相当を出力、Rust は 0 件（`summary.json`）。 | 実装差分（Rust type/effect） | TODO: DIAG-RUST-06 |
| effect_residual_leak | type/effect | OCaml CLI が `--emit-effects-metrics` 未実装で即エラー（`diagnostics.ocaml.json`）。Rust は recover 5 件のみ。 | 実装差分（ハーネス/OCaml CLI） | TODO: DIAG-RUST-06 |
| effect_stage_cli_override | type/effect | OCaml は 1 件の parser error、Rust は 6 件の `未定義のトークン`（`diagnostics.rust.json`）。Stage/Capability 診断へ到達せず。 | 実装差分（Rust parser/Stage） | TODO: DIAG-RUST-06 |
| ffi_async_dispatch | ffi/capability | OCaml `diagnostics.ocaml.json` が空ファイル、Rust は 42 件の parser error。CLI で Stage/Runtime フラグが効かず比較不能。 | 実装差分（CLI/ハーネス） | TODO: DIAG-RUST-06 |
| ffi_ownership_mismatch | ffi/capability | OCaml は 1 件の parser error、Rust は 64 件（`diagnostics.rust.json`）。`collect-iterator-audit-metrics.py` で `metrics_ok=false`。 | 実装差分（Rust parser/Capability） | TODO: DIAG-RUST-06 |
| ffi_stage_messagebox | ffi/capability | OCaml 1 件、Rust 64 件で `schema_ok=true` だが `metrics` 崩壊。Rust 側が Stage/Ffi 拡張を欠落。 | 実装差分（Rust parser/Capability） | TODO: DIAG-RUST-06 |
| stream_backpressure_hint | streaming | OCaml 出力が空（`diagnostics.ocaml.json` サイズ 0）、Rust は 5 件。`parser-metrics.rust.err.log` が `parser.expected_summary_presence < 1.0`。 | 実装差分（Streaming メトリクス） | TODO: DIAG-RUST-05 |
| stream_checkpoint_drift | streaming | OCaml 1 件、Rust 4 件。Rust 側 `parser.stream.*` 拡張が不足し `metrics_ok=false`。 | 実装差分（Streaming 拡張） | TODO: DIAG-RUST-05 |
| stream_pending_resume | streaming | OCaml の schema 検証が `diagnostics[0].expected` 欠落で失敗（`schema-validate.log`）、Rust は 11 件。 | 実装差分（ハーネス + Streaming recover） | TODO: DIAG-RUST-05 |
| cli_merge_warnings | cli-runconfig | OCaml CLI が `Error: no input file` を出力し JSON にならず（`diagnostics.ocaml.json`）、Rust も診断 0件。 | 実装差分（ハーネス/CLI 引数） | TODO: DIAG-RUST-07 |
| cli_packrat_switch | cli-runconfig | OCaml 出力 0 件、Rust は 2 件の parser errorで `diag_match=false`。`diagnostics.rust.json` が CLI フラグ未適用を示唆。 | 実装差分（Rust CLI RunConfig） | TODO: DIAG-RUST-07 |
| cli_trace_toggle | cli-runconfig | OCaml stdout に `[TRACE]` が混在し JSON 解析不能（`schema-validate.log`）。Rust は 4 件の parser error。 | 実装差分（ハーネス/ログ分離） | TODO: DIAG-RUST-07 |
| lsp_diagnostic_stream | lsp-rpc | OCaml は 1 件の parser error、Rust は 4 件 (`diagnostics.rust.json`)。LSP フラグ伝播不足で `metrics_ok=false`。 | 実装差分（Rust streaming + LSP） | TODO: DIAG-RUST-07 |
| lsp_hover_internal_error | lsp-rpc | OCaml `diagnostics.ocaml.json` が空、Rust は 1 件。`summary.json` で `schema_ok=true` だが CLI 側が LSP fixture を読めていない。 | 実装差分（ハーネス/LSP） | TODO: DIAG-RUST-07 |
| lsp_workspace_config | lsp-rpc | OCaml CLI が `unknown option '--config'` を報告（`diagnostics.ocaml.json`）。Rust 側は 2 件の parser error。 | 実装差分（OCaml CLI / config フラグ） | TODO: DIAG-RUST-07 |

- 各 TODO の詳細・緩和策は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` を参照。
- `p1-front-end-checklists.csv` の診断カテゴリは本トリアージ結果を反映し、Run ID と成果物パスを付記した。
