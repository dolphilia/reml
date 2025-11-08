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
| `stream_pending_resume` | inline (`cases.txt`、CLI で `--streaming --stream-resume-hint diag-w4`) | Ready（2027-11-09: cases 登録、steam metrics 取得待ち） | `parser.stream.outcome_consistency`, `resume_hint` | `diagnostics.w4.stream_pending_resume` 入力を `poc_dualwrite_compare.sh --mode diag` から参照 |
| `stream_backpressure_hint` | inline (`cases.txt`) | Ready（2027-11-09: cases 登録、backpressure メトリクス計測待ち） | `parser.stream.backpressure_sync`, `stream_meta.backpressure_events` | `--streaming --stream-flow-policy auto` を付与しバックプレッシャイベントの差分を観測 |
| `stream_checkpoint_drift` | inline (`cases.txt`) | Ready（2027-11-09: cases 登録、checkpoint diff 解析待ち） | `parser.stream.demandhint_coverage`, `last_checkpoint` | `stream_checkpoint` CLI オプション併用で checkpoint ずれをトレース |

## Type & Effect（3 cases）
| Case ID | Input | Status | Metrics / Extensions | Notes |
| --- | --- | --- | --- | --- |
| `type_condition_bool` | inline (`cases.txt`) | Ready（20271107-w4-new: OCaml diag=0 → `TODO: DIAG-RUST-06`) | `effects.stage.*`, `diagnostic_code=TYPE_xxx` | `if x` の条件型 mismatch を比較（`cases.txt` では inline 化済み。CLI 側 JSON が欠落しているため schema/log を要調査） |
| `type_condition_literal_bool` | inline (`cases.txt`、`if 1 then ...`) | Ready（2027-11-09: 追加。OCaml 側で型診断を取得できることを確認予定） | `diagnostic_code=TYPE_condition_bool`, `recover.has_fixits` | リテラル値を bool 条件に置く簡易ケース。Rust は typed parameter 非対応でも再現可能 |
| `effect_residual_leak` | inline (`cases.txt`、`effect ConsoleLog` snippet) | Ready（2027-11-09: cases 登録、Rust/OCaml JSON 取得待ち） | `effects.residual`, `effect.required_capabilities` | CLI で `--experimental-effects --type-row-mode dual-write` を指定し未処理 effect を発火 |
| `effect_stage_cli_override` | inline (`cases.txt`、`@requires_capability(stage="beta")`) | Ready（2027-11-09: cases 登録、Stage override 解析待ち） | `effects.stage_trace`, `effect.stage.inputs.cli` | `--effect-stage beta` 指定で CLI / `extensions.effects.stage_override` の差分を取得 |

## Capability Stage / FFI（3 cases）
| Case ID | Input | Status | Metrics / Extensions | Notes |
| --- | --- | --- | --- | --- |
| `ffi_stage_messagebox` | file `examples/ffi/windows/messagebox.reml` | Ready（2027-11-09: cases.txt へ参照を追加、Windows ラン実行待ち） | `bridge.stage.*`, `effect.stage.actual_capabilities` | Windows Capability チェック用。CI では `--effect-stage beta --runtime-capabilities` を要求 |
| `ffi_ownership_mismatch` | file `examples/ffi/windows/ownership_transfer.reml` | Ready（2027-11-09: cases.txt 参照追加） | `bridge.audit_pass_rate`, `ffi.contract.*` | 所有権タグ不一致を CLI/LSP 同時比較。Rust 版デコーダ未実装のため `TODO` を維持 |
| `ffi_async_dispatch` | file `examples/ffi/macos/ffi_dispatch_async.reml` | Ready（2027-11-09: cases.txt 参照追加） | `effects.impl_resolve.*`, `ffi.bridge.*` | macOS Dispatch `ffi_dispatch_async` の診断差分を W3 TODO（`W3-TYPECK-ffi-dispatch-async`）と共有 |

## CLI Config / RunConfig（3 cases）
| Case ID | Input | Status | Metrics / Extensions | Notes |
| --- | --- | --- | --- | --- |
| `cli_packrat_switch` | file `examples/cli/emit_suite.reml` | Ready（2027-11-09: cases.txt 参照追加、`--packrat`/`--no-packrat` 比較を記録） | `parser.runconfig.switch_coverage`, `runconfig.packrat` | `diag` モードで `--packrat` を切り替え、`parser.runconfig.packrat` の JSON 変化を確認 |
| `cli_trace_toggle` | file `examples/cli/trace_sample.reml` | Ready（2027-11-09: cases.txt 参照追加） | `parser.runconfig.extensions.lex`, `trace` | `--trace`/`--no-merge-warnings` の差分を取得。Rust 側は `poc_frontend` の `--recover-*` オプションで代替 |
| `cli_merge_warnings` | file `examples/cli/type_error.reml` | Ready（2027-11-09: cases.txt 参照追加） | `parser.runconfig.extensions.recover` | `--no-merge-warnings` を指定し recover note の出力数が一致するか確認 |

## LSP RPC（3 cases）
| Case ID | Input | Status | Metrics / Extensions | Notes |
| --- | --- | --- | --- | --- |
| `lsp_hover_internal_error` | file `examples/cli/type_error.reml`（fixture: `diagnostic-v2-hover-internal.json`） | Ready（2027-11-09: cases/fixture を紐付け） | `domain=lsp`, `audit_metadata.lsp.*` | `tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-hover-internal.json` を `cases.txt` と共通入力に設定 |
| `lsp_diagnostic_stream` | file `examples/cli/trace_sample.reml`（fixture: `diagnostic-v2-stream.json`） | Ready（2027-11-09: cases/fixture 連携） | `stream_meta.*`, `parser.stream.*` | LSP 側は `npm run ci` で該当フィクスチャを比較、CLI 側は `--streaming` を併用 |
| `lsp_workspace_config` | file `examples/cli/emit_suite.reml`（fixture: `diagnostic-v2-workspace-config.json`） | Ready（2027-11-09: cases/fixture 連携） | `extensions.config.*`, `audit_metadata.cli.change_set` | Workspaces 設定を CLI/LSP で共有し、`cli.change_set` の差分を抑止 |

> 実行時は `docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt` に記載した入力を `scripts/poc_dualwrite_compare.sh --mode diag` へ渡し、ストリーミング／効果系ケースでは CLI フラグ（`--streaming`, `--stream-resume-hint`, `--experimental-effects`, `--effect-stage beta` など）を明示的に付与する。LSP ケースは `npm run ci --prefix tooling/lsp/tests/client_compat` と同期し、同一入力ファイルを `cases.txt` で参照する。

### TODO リンク
- **DIAG-RUST-05** — ストリーミング復旧ケース（`stream_pending_resume` ほか）を作成し、`parser.stream.*` 系メトリクスが Rust/OCaml で比較できるようにする。
- **DIAG-RUST-06** — 効果・Capability Stage・FFI 関連の入力を整理し、`effects.*` / `bridge.*` 拡張の diff を確認できるようにする。
- **DIAG-RUST-07** — CLI RunConfig/LSP RPC 用のケースを `cases.txt` と LSP フィクスチャの双方に登録し、`parser.runconfig.*` と LSP ログが食い違わないようにする。
