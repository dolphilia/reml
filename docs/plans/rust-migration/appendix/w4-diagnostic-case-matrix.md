# W4 診断ケースマトリクス

W4 では `scripts/poc_dualwrite_compare.sh --mode diag` と `scripts/dualwrite_summary_report.py --diag-table` を組み合わせて、カテゴリ別に代表ケースを回しながら OCaml/Rust フロントエンドの診断互換性を確認する。本マトリクスではカテゴリごとに最低 3 ケース（parser recover は 5 ケース）を登録し、入力種別・期待する拡張キー／メトリクス・準備状況を横断管理する。`Status` が Ready のケースは `docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt` から自動読み込みできる。

- *2027-11-12 更新*: `compiler/ocaml/tests/test_cli_diagnostics.ml`, `streaming_runner_tests.ml`, `test_cli_callconv_snapshot.ml`, `test_ffi_contract.ml`, `docs/plans/bootstrap-roadmap/2-5-proposals/DIAG-002-proposal.md` など W4 Step2 で指定された参照元を横断し、各ケースへ「どのテスト／計画書を根拠にするか」「CLI/LSP でどのフラグを使うか」を明記した。これにより `poc_dualwrite_compare.sh --mode diag` と LSP フィクスチャの両方で同一入力・同一監査キーを辿れる。同日実行した `reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/summary.md` では 21 ケースすべて `gating=false` となり、Rust 側診断に schema 情報が欠落している（`parser_audit=0.0`）こと、およびケース固有フラグが CLI へ伝播していないことが確認された。
- *2028-01-15 更新*: Run ID `20280115-w4-diag-refresh` を `scripts/poc_dualwrite_compare.sh --mode diag` で実行し、`reports/dual-write/front-end/w4-diagnostics/20280115-w4-diag-refresh/summary.md` を取得。Streaming 系は CLI フラグが適用され `runconfig.extensions.stream.enabled=true` になったが、`collect-iterator-audit-metrics.py` が `parser.stream_extension_field_coverage < 1.0` / `parser.expected_summary_presence < 1.0` を継続検出し、`reports/dual-write/front-end/w4-diagnostics/20280115-w4-diag-refresh/stream_pending_resume/parser-metrics.ocaml.err.log` と `reports/dual-write/front-end/w4-diagnostics/20280115-w4-diag-refresh/stream_backpressure_hint/parser-metrics.rust.err.log` に同指標が残った。Type/Effect 系は OCaml 側が `diagnostics.ocaml.json` に 1 件ずつ記録されるようになった一方、Rust 側 `diagnostics.rust.json` は空（`reports/dual-write/front-end/w4-diagnostics/20280115-w4-diag-refresh/type_condition_bool/diagnostics.rust.json`）のままで `summary.json` でも `rust_diag_count=0`。`reports/dual-write/front-end/w4-diagnostics/README.md` のケース表は `--diag-table` で更新済み。

## Parser Recover（5 cases）
| Case ID | Source（テスト/資料） | Input / CLI | Status | Metrics / Extensions | Notes |
| --- | --- | --- | --- | --- | --- |
| `recover_missing_semicolon` | `compiler/ocaml/tests/test_cli_diagnostics.ml`（recover 快復スナップショット） / `docs/plans/bootstrap-roadmap/2-5-review-log.md`（Step3 `parser_recover_tests.ml` 記録） | inline（`cases.txt`） / diag デフォルト（`--packrat --format json --json-mode compact`） | Ready（20271107-w4-new: diag✅ / metrics⚠️ `parser.stream_extension_field_coverage`） | `extensions.recover.expected_tokens`, `parser.expected_summary` | 2 行連続の `let` 文でセミコロンを欠落させ、同期トークン復旧を比較する |
| `recover_unclosed_block` | 同上 + `docs/plans/bootstrap-roadmap/2-5-proposals/ERR-002-proposal.md`（FixIt 要件） | inline（`cases.txt`） / diag デフォルト | Ready（20271107-w4-new: diag✅ / metrics⚠️） | `extensions.recover.sync_tokens`, `span_trace` | `{ ...` でブロックを閉じず EOF へ到達。Packrat `span_trace` と sync token の一致を確認 |
| `recover_else_without_if` | `compiler/ocaml/tests/golden/diagnostics/parser/expected-summary.json.golden`（`parse.expected`） | inline（`cases.txt`） / diag デフォルト | Ready（20271107-w4-new: Rust diag=0 → `TODO: DIAG-RUST-05`） | `parser.expected.{token,alternatives}` | 孤立 `else` で message_key/alternatives を比較。Rust Recover 欠落を追跡 |
| `recover_missing_tuple_comma` | `compiler/ocaml/tests/test_parser.ml`（tuple 期待値） | inline（`cases.txt`） / diag デフォルト | Ready（20271107-w4-new: diag✅ / metrics⚠️） | `extensions.recover.notes`, `diagnostic.v2.codes` | タプル内カンマ欠落で複数候補の列挙順を比較 |
| `recover_lambda_body` | `compiler/ocaml/tests/test_cli_diagnostics.ml`（lambda recover note） | inline（`cases.txt`） / diag デフォルト | Ready（20271107-w4-new: Rust diag=2 → `TODO: DIAG-RUST-05`） | `extensions.recover.has_fixits`, `parser.stream.backpressure` | ラムダ本文で `return` 欠落。`has_fixits` と backpressure 計測を同時確認 |

*2027-11-12 実行結果*: `reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/` では 5 ケース中 3 ケース（missing_semicolon / missing_tuple_comma / unclosed_block）が `diag_match=✅` だが Rust 側 `parser_audit` は 0.0 のまま（例: `reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/recover_missing_semicolon/parser-metrics.rust.err.log`）。`recover_else_without_if` は `reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/recover_else_without_if/diagnostics.rust.json` が空、`recover_lambda_body` は Rust Recover が 2 件のままで `gating=false`（`DIAG-RUST-05` 継続）。

## Streaming Meta（3 cases）
| Case ID | Source（テスト/資料） | Input / CLI | Status | Metrics / Extensions | Notes |
| --- | --- | --- | --- | --- | --- |
| `stream_pending_resume` | `compiler/ocaml/tests/streaming_runner_tests.ml::test_pending_resume_flow` | inline（`cases.txt`） / `--streaming --stream-resume-hint diag-w4 --stream-flow-policy auto --stream-demand-min-bytes 4` | Ready（2027-11-09: cases 登録、stream metrics 取得待ち） | `parser.stream.outcome_consistency`, `resume_hint` | `diagnostics.w4.stream_pending_resume` 入力を diag モードから参照し、Pending→Resume 経路を比較 |
| `stream_backpressure_hint` | `streaming_runner_tests.ml::test_streaming_matches_batch` / `docs/guides/core-parse-streaming.md` §3 | inline（`cases.txt`） / `--streaming --stream-flow-policy auto --stream-flow-max-lag 8192` | Ready（2027-11-09: cases 登録、backpressure メトリクス計測待ち） | `parser.stream.backpressure_sync`, `stream_meta.backpressure_events` | Flow Controller Auto で backpressure イベントと `resume_lineage` を比較 |
| `stream_checkpoint_drift` | `streaming_runner_tests.ml::test_checkpoint_restore`（補助メモ） | inline（`cases.txt`） / `--streaming --stream-checkpoint diag-w4` | Ready（2027-11-09: cases 登録、checkpoint diff 解析待ち） | `parser.stream.demandhint_coverage`, `last_checkpoint` | checkpoint 位置ずれをトレースし、`stream_meta.*` を同期 |

*2027-11-12 実行結果*: diag ハーネスが `#flags` を CLI へ適用していないため、3 ケースすべて `--streaming` なしで実行され `schema-validate.log` が `diagnostics[0].expected` 欠落で失敗し、`parser.stream_extension_field_coverage` が 0.0（例: `reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/stream_pending_resume/schema-validate.log`）。`DIAG-RUST-05` にて `scripts/poc_dualwrite_compare.sh` 側で CLI フラグ注入を実装する。

## Type & Effect（3 cases）
| Case ID | Source（テスト/資料） | Input / CLI | Status | Metrics / Extensions | Notes |
| --- | --- | --- | --- | --- | --- |
| `type_condition_bool` | `compiler/ocaml/tests/test_type_inference.ml` / `docs/spec/1-2-types-Inference.md`（条件型診断） | inline（`cases.txt`） / `--experimental-effects --type-row-mode dual-write --emit-typeck-debug <tmp>` | Ready（20271107-w4-new: OCaml diag=0 → `TODO: DIAG-RUST-06`) | `effects.stage.*`, `diagnostic_code=TYPE_condition_bool` | `if x` の条件型 mismatch を比較。OCaml 側 JSON 欠落のため Rust recover 1 件との差分を追跡 |
| `type_condition_literal_bool` | `compiler/ocaml/tests/test_type_errors.ml`（literal guard） | inline（`cases.txt`、`if 1 then ...`） / 同上 CLI | Ready（2027-11-09: 追加。OCaml 側で型診断を取得できることを確認予定） | `diagnostic_code=TYPE_condition_bool`, `recover.has_fixits` | リテラル bool 条件を使い型診断を確実に取得 |
| `effect_residual_leak` | `compiler/ocaml/tests/test_effect_residual.ml` / `docs/plans/bootstrap-roadmap/2-5-proposals/EFFECT-002-proposal.md` | inline（`cases.txt`、`effect ConsoleLog` snippet） / `--experimental-effects --type-row-mode dual-write --emit-effects-metrics <tmp>` | Ready（2027-11-09: cases 登録、Rust/OCaml JSON 取得待ち） | `effects.residual`, `effect.required_capabilities` | 未処理 effect を発火し `effects.residual` と Capability registry を比較 |
| `effect_stage_cli_override` | `compiler/ocaml/tests/test_cli_callconv_snapshot.ml::callconv_windows_messagebox`（stage override） | inline（`cases.txt`、`@requires_capability(stage="beta")`) / `--effect-stage beta --experimental-effects` | Ready（2027-11-09: cases 登録、Stage override 解析待ち） | `effects.stage_trace`, `effect.stage.inputs.cli` | CLI オプション／属性で Stage override を発火し、`extensions.effects.stage_override` を検証 |

*2027-11-12 実行結果*: OCaml 側 CLI が parser で終了しており `reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/type_condition_bool/diagnostics.ocaml.json` などが空配列、Rust 側は recover 1 件（`reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/type_condition_bool/diagnostics.rust.json`）のまま。`effect_*` ケースも CLI フラグ未適用のため lex error 群（5〜6 件）が記録され、`effects.*` メトリクスは計測不可。`DIAG-RUST-06` に沿って `--experimental-effects --type-row-mode dual-write --emit-typeck-debug <tmp>` を diag モードへ注入する。

## Capability Stage / FFI（3 cases）
| Case ID | Source（テスト/資料） | Input / CLI | Status | Metrics / Extensions | Notes |
| --- | --- | --- | --- | --- | --- |
| `ffi_stage_messagebox` | `compiler/ocaml/tests/test_cli_callconv_snapshot.ml::callconv_windows_messagebox` | file `examples/ffi/windows/messagebox.reml` / `--effect-stage beta --runtime-capabilities windows.messagebox` | Ready（2027-11-09: cases.txt へ参照を追加、Windows ラン実行待ち） | `bridge.stage.*`, `effect.stage.actual_capabilities` | Windows Capability チェック用。CI では stage override + runtime capability 指定を要求 |
| `ffi_ownership_mismatch` | `compiler/ocaml/tests/test_ffi_contract.ml::ownership_transfer` | file `examples/ffi/windows/ownership_transfer.reml` / `--effect-stage beta --runtime-capabilities windows.ffi` | Ready（2027-11-09: cases.txt 参照追加） | `bridge.audit_pass_rate`, `ffi.contract.*` | 所有権タグ不一致を CLI/LSP 同時比較。Rust 版デコーダ未実装のため `TODO` 継続 |
| `ffi_async_dispatch` | `compiler/ocaml/tests/test_ffi_contract.ml::ffi_dispatch_async`（W3 TODO） | file `examples/ffi/macos/ffi_dispatch_async.reml` / `--effect-stage beta --experimental-effects` | Ready（2027-11-09: cases.txt 参照追加） | `effects.impl_resolve.*`, `ffi.bridge.*` | macOS Dispatch 差分を W3 TODO（`W3-TYPECK-ffi-dispatch-async`）と共有 |

*2027-11-12 実行結果*: Rust 側は 42〜64 件の lex recover 診断を出力し（例: `reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/ffi_stage_messagebox/diagnostics.rust.json`）、OCaml 側は `--runtime-capabilities` 未指定のため parser で停止。`bridge.*`/`effect.stage.*` メトリクスはいずれも計測できず `metrics_ok=false`。CLI フラグ伝播と Capability 監査 JSON の移植が `DIAG-RUST-06` の前提。

## CLI Config / RunConfig（3 cases）
| Case ID | Source（テスト/資料） | Input / CLI | Status | Metrics / Extensions | Notes |
| --- | --- | --- | --- | --- | --- |
| `cli_packrat_switch` | `compiler/ocaml/tests/run_config_tests.ml::packrat_switch` / `docs/plans/bootstrap-roadmap/2-5-proposals/DIAG-002-proposal.md`（監査必須フィールド） | file `examples/cli/emit_suite.reml` / `--packrat` と `--no-packrat` を順番に指定 | Ready（2027-11-09: cases.txt 参照追加、`--packrat`/`--no-packrat` 比較を記録） | `parser.runconfig.switch_coverage`, `runconfig.packrat`, `audit.*` | Packrat 切替と `audit.timestamp`/`audit_id` の残存を同時検証 |
| `cli_trace_toggle` | `compiler/ocaml/tests/run_config_tests.ml::trace_options` | file `examples/cli/trace_sample.reml` / `--trace --no-merge-warnings` | Ready（2027-11-09: cases.txt 参照追加） | `parser.runconfig.extensions.lex`, `trace` | `--trace`/`--no-merge-warnings` の差分を取得。Rust 側は `poc_frontend` の `--recover-*` オプションで代替 |
| `cli_merge_warnings` | `compiler/ocaml/tests/test_cli_diagnostics.ml`（warning merge スナップショット） | file `examples/cli/type_error.reml` / `--no-merge-warnings --json-mode compact` | Ready（2027-11-09: cases.txt 参照追加） | `parser.runconfig.extensions.recover`, `diagnostic.audit.*` | `--no-merge-warnings` で recover note 出力数と監査必須フィールドを確認 |

*2027-11-12 実行結果*: `cli_packrat_switch` と `cli_merge_warnings` は OCaml 側診断が 0 件（`reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/cli_packrat_switch/diagnostics.ocaml.json` 等が空）で `parser.runconfig_switch_coverage` が測定できず、Rust 側は 1〜4 件の recover 診断を出力。`cli_trace_toggle` も `parser_audit=0.0` のまま。CLI 追加フラグを diag ハーネスへ引き渡すタスクを `DIAG-RUST-07` に統合した。

## LSP RPC（3 cases）
| Case ID | Source（テスト/資料） | Input / CLI | Status | Metrics / Extensions | Notes |
| --- | --- | --- | --- | --- | --- |
| `lsp_hover_internal_error` | `tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-hover-internal.json` / `docs/spec/3-6-core-diagnostics-audit.md`（LSP 監査） | file `examples/cli/type_error.reml` / CLI: `--format json --emit-parse-debug <tmp>` / LSP: `npm run ci --prefix tooling/lsp/tests/client_compat` | Ready（2027-11-09: cases/fixture を紐付け） | `domain=lsp`, `audit_metadata.lsp.*` | LSP フィクスチャと CLI 入力を共有し、監査キー `audit.channel=lsp` を比較 |
| `lsp_diagnostic_stream` | `tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-stream.json` / `streaming_runner_tests` | file `examples/cli/trace_sample.reml` / CLI: `--streaming --stream-resume-hint diag-w4` / LSP: `npm run ci ...` | Ready（2027-11-09: cases/fixture 連携） | `stream_meta.*`, `parser.stream.*` | LSP 側 stream フィールドと CLI メトリクスを同期 |
| `lsp_workspace_config` | `tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-workspace-config.json` / `docs/plans/bootstrap-roadmap/2-5-proposals/DIAG-002-proposal.md` | file `examples/cli/emit_suite.reml` / CLI: `--config workspace.json`（cases 付属） | Ready（2027-11-09: cases/fixture 連携） | `extensions.config.*`, `audit_metadata.cli.change_set`, `audit.timestamp` | Workspaces 設定を CLI/LSP で共有し、`cli.change_set` と監査必須フィールドの差分を抑止 |

*2027-11-12 実行結果*: CLI 側は `w4-diagnostic-cases.txt` のフラグ未適用で LSP ケースと同一設定にならず、OCaml 診断が 0〜1 件、Rust 側は 1〜4 件の recover 診断に留まる。`npm run ci --prefix tooling/lsp/tests/client_compat` のログは baseline 通過済みだが、dual-write ランでは CLI/LSP の入出力が同期していないため `reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/lsp_*` ディレクトリには diff が生成されていない。`DIAG-RUST-07` のハーネス修正後に再実行する。
> 実行時は `docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt` に記載した入力を `scripts/poc_dualwrite_compare.sh --mode diag` へ渡し、ストリーミング／効果系ケースでは CLI フラグ（`--streaming`, `--stream-resume-hint`, `--experimental-effects`, `--effect-stage beta` など）を明示的に付与する。LSP ケースは `npm run ci --prefix tooling/lsp/tests/client_compat` と同期し、同一入力ファイルを `cases.txt` で参照する。

### TODO リンク
- **DIAG-RUST-05** — ストリーミング復旧ケース（`stream_pending_resume` ほか）を作成し、`parser.stream.*` 系メトリクスが Rust/OCaml で比較できるようにする。
- **DIAG-RUST-06** — 効果・Capability Stage・FFI 関連の入力を整理し、`effects.*` / `bridge.*` 拡張の diff を確認できるようにする。
- **DIAG-RUST-07** — CLI RunConfig/LSP RPC 用のケースを `cases.txt` と LSP フィクスチャの双方に登録し、`parser.runconfig.*` と LSP ログが食い違わないようにする。
