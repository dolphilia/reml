# W4 診断ケースマトリクス

W4 では `scripts/poc_dualwrite_compare.sh --mode diag` と `scripts/dualwrite_summary_report.py --diag-table` を組み合わせて、カテゴリ別に代表ケースを回しながら OCaml/Rust フロントエンドの診断互換性を確認する。本マトリクスではカテゴリごとに最低 3 ケース（parser recover は 5 ケース）を登録し、入力種別・期待する拡張キー／メトリクス・準備状況を横断管理する。`Status` が Ready のケースは `docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt` から自動読み込みできる。

## Parser Recover（5 cases）
| Case ID | Input | Status | Metrics / Extensions | Notes |
| --- | --- | --- | --- | --- |
| `recover_missing_semicolon` | inline (`cases.txt`) | Ready（20271107-w4-new: diag✅ / metrics⚠️ `parser.stream_extension_field_coverage`） | `extensions.recover.expected_tokens`, `parser.expected_summary` | 2 行連続の `let` 文でセミコロンを欠落させ、同期トークン復旧を比較する |
| `recover_unclosed_block` | inline (`cases.txt`) | Ready（20271107-w4-new: diag✅ / metrics⚠️） | `extensions.recover.sync_tokens`, `span_trace` | `{ ...` でブロックを閉じずに EOF へ到達させ、Packrat の巻き戻しログを確認 |
| `recover_else_without_if` | inline (`cases.txt`) | Ready（20271107-w4-new: Rust diag=0 → `TODO: DIAG-RUST-05`） | `parser.expected.{token,alternatives}` | 孤立した `else` を置き、エラープロフィールが同じになるか検証 |
| `recover_missing_tuple_comma` | inline (`cases.txt`) | Ready（20271107-w4-new: diag✅ / metrics⚠️） | `extensions.recover.notes`, `diagnostic.v2.codes` | タプル内でカンマを抜き、複数候補の列挙順を比較 |
| `recover_lambda_body` | inline (`cases.txt`) | Ready（20271107-w4-new: Rust diag=2 → `TODO: DIAG-RUST-05`） | `extensions.recover.has_fixits`, `parser.stream.backpressure` | ラムダ式の本文で `return` を欠落させ、Recover ヒントの `has_fixits` を観測 |

## Streaming Meta（3 cases）
| Case ID | Input | Status | Metrics / Extensions | Notes |
| --- | --- | --- | --- | --- |
| `stream_pending_resume` | inline (TBD) | Pending → TODO: DIAG-RUST-05 | `parser.stream.outcome_consistency`, `resume_hint` | Packrat Pending → Resume のケースを最小構成で作成予定 |
| `stream_backpressure_hint` | inline (TBD) | Pending → TODO: DIAG-RUST-05 | `parser.stream.backpressure_sync`, `stream_meta.backpressure_events` | `StreamMeta` の backpressure 計測を Rust 実装でも揃える |
| `stream_checkpoint_drift` | inline (TBD) | Pending → TODO: DIAG-RUST-05 | `parser.stream.demandhint_coverage`, `last_checkpoint` | `checkpoint` の位置がズレた場合の diff を採取（Recover case と同じ入力にする） |

## Type & Effect（3 cases）
| Case ID | Input | Status | Metrics / Extensions | Notes |
| --- | --- | --- | --- | --- |
| `type_condition_bool` | inline (`cases.txt`) | Ready（20271107-w4-new: OCaml diag=0 → `TODO: DIAG-RUST-06`) | `effects.stage.*`, `diagnostic_code=TYPE_xxx` | `if x` の条件型 mismatch を比較（`cases.txt` では inline 化済み。CLI 側 JSON が欠落しているため schema/log を要調査） |
| `effect_residual_leak` | inline (TBD) | Pending → TODO: DIAG-RUST-06 | `effects.residual`, `effect.required_capabilities` | 未処理の `ConsoleLog` effect を発生させ、残余判定の JSON を比較 |
| `effect_stage_cli_override` | inline (TBD) | Pending → TODO: DIAG-RUST-06 | `effects.stage_trace`, `effect.stage.inputs.cli` | `--effect-stage beta` 相当の入力で Stage 解決ログを確認 |

## Capability Stage / FFI（3 cases）
| Case ID | Input | Status | Metrics / Extensions | Notes |
| --- | --- | --- | --- | --- |
| `ffi_stage_messagebox` | file `examples/ffi/windows/messagebox.reml` | Pending → TODO: DIAG-RUST-06 | `bridge.stage.*`, `effect.stage.actual_capabilities` | Windows 固有 Capability を要求するケース。Rust FFI ルート移行後に Ready へ昇格 |
| `ffi_ownership_mismatch` | file `examples/ffi/windows/ownership_transfer.reml` | Pending → TODO: DIAG-RUST-06 | `bridge.audit_pass_rate`, `ffi.contract.*` | 所有権の解釈が異なる場合の診断を比較 |
| `ffi_async_dispatch` | file `examples/ffi/macos/ffi_dispatch_async.reml` | Pending → TODO: DIAG-RUST-06 | `effects.impl_resolve.*`, `ffi.bridge.*` | `ffi_dispatch_async` から派生する Stage/Effects 差分をトラッキング |

## CLI Config / RunConfig（3 cases）
| Case ID | Input | Status | Metrics / Extensions | Notes |
| --- | --- | --- | --- | --- |
| `cli_packrat_switch` | file `examples/cli/emit_suite.reml` | Pending → TODO: DIAG-RUST-07 | `parser.runconfig.switch_coverage`, `runconfig.packrat` | CLI スイッチが JSON に反映されるかを比較 |
| `cli_trace_toggle` | inline (TBD) | Pending → TODO: DIAG-RUST-07 | `parser.runconfig.extensions.lex`, `trace` | `--trace` 指定時の RunConfig 拡張を比較 |
| `cli_merge_warnings` | inline (TBD) | Pending → TODO: DIAG-RUST-07 | `parser.runconfig.extensions.recover` | `merge_warnings` オプションの差分を比較 |

## LSP RPC（3 cases）
| Case ID | Input | Status | Metrics / Extensions | Notes |
| --- | --- | --- | --- | --- |
| `lsp_hover_internal_error` | fixture sync with `tooling/lsp/tests/client_compat/fixtures/*.json` | Pending → TODO: DIAG-RUST-07 | `domain=lsp`, `audit_metadata.lsp.*` | LSP テストハーネスと cases.txt の同期が必要 |
| `lsp_diagnostic_stream` | fixture sync | Pending → TODO: DIAG-RUST-07 | `stream_meta.*`, `parser.stream.*` | LSP 経由でも Streaming メタの diff を採取 |
| `lsp_workspace_config` | fixture sync | Pending → TODO: DIAG-RUST-07 | `extensions.config.*`, `audit_metadata.cli.change_set` | `emit_suite_cli` 系の CLI 設定を LSP 出力と照合 |

### TODO リンク
- **DIAG-RUST-05** — ストリーミング復旧ケース（`stream_pending_resume` ほか）を作成し、`parser.stream.*` 系メトリクスが Rust/OCaml で比較できるようにする。
- **DIAG-RUST-06** — 効果・Capability Stage・FFI 関連の入力を整理し、`effects.*` / `bridge.*` 拡張の diff を確認できるようにする。
- **DIAG-RUST-07** — CLI RunConfig/LSP RPC 用のケースを `cases.txt` と LSP フィクスチャの双方に登録し、`parser.runconfig.*` と LSP ログが食い違わないようにする。
