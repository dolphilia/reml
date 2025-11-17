# 2025-11-17 Syntax Samples

| サンプル | コマンド | 結果 | 備考 |
|----------|----------|------|------|
| use_nested.reml | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested.reml --trace-output reports/spec-audit/ch1/use_nested-YYYYMMDD-trace.md` | ✅ 診断 0 件 | `module`/`use`/ブロック/`match` を Rust Frontend が受理し、`TraceEvent::{ModuleHeaderAccepted,UseDeclAccepted}` を保存できるようになった（2025-11-17 修正）。 |
| use_nested_rustcap.reml | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested_rustcap.reml` | ✅ 診断 0 件（参照目的のみ） | Phase 2-7 までのフォールバック。2025-11-21 以降は監査ベースラインから除外し、`docs/spec/1-1-syntax/examples/README.md` で履歴としてのみ参照。 |
| effect_handler.reml | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/effect_handler.reml` | ⚠️ `構文エラー: 入力を解釈できません` | `effect` 宣言をパーサが受理できず、`rust-gap SYNTAX-003` を継続。 |

## 保存ルール（Phase 2-8 W37 追補）

- `use_nested.reml` / `effect_handler.reml` の診断結果は `reports/spec-audit/ch1/<sample>-YYYYMMDD-diagnostics.json` 形式で保存し、`YYYYMMDD` は CI 実行日、ファイル末尾に `git rev-parse HEAD` をコメントとして追記する（2025-11-17 実行分は追記待ち）。
- Rust Frontend で `use_nested.reml` を実行する際は `--trace-output reports/spec-audit/ch1/use_nested-YYYYMMDD-trace.md` を併用し、`TraceEvent::ModuleHeaderAccepted` / `TraceEvent::UseDeclAccepted` を記録する。
- `use_nested_rustcap.reml` は参考用途として維持しつつ、監査ベースラインは正準サンプル `use_nested.reml`（診断 0 件）で取得する。

## 2025-11-18 追加サンプル

| サンプル | コマンド | 結果 | 備考 |
|----------|----------|------|------|
| block_scope.reml | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/block_scope.reml --trace-output reports/spec-audit/ch1/block_scope-20251118-trace.md` | ✅ 診断 0 件 | `ExprParser` で `let`/`var` バインディングと `{ ... }` ブロックを処理。ログ: `reports/spec-audit/ch1/block_scope-20251118-diagnostics.json`。 |
| effect_handler.reml | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/effect_handler.reml --trace-output reports/spec-audit/ch1/effect_handler-20251118-trace.md` | ✅ 診断 0 件 | `perform`/`do`/`handle`/`operation` を Rust Frontend で受理。dual-write 結果は `reports/spec-audit/ch1/effect_handler-20251118-dualwrite.md` に保存。 |

## 2025-11-19 module_parser 再実装ログ

| 項目 | コマンド | 結果 | 備考 |
|------|----------|------|------|
| parser::module テスト | `cargo test --manifest-path compiler/rust/frontend/Cargo.toml parser::module -- --nocapture` | ✅ 緑化 (`CI_RUN_ID=rust-frontend-w37-20251119.1`) | ログは `reports/spec-audit/ch1/module_parser-20251119-parser-tests.md`。`TraceEvent::ModuleStageEntered` を記録し、`use_nested`/`block_scope`/`effect_handler` の 6 ケースを収集。 |
| dual-write 確認 | `scripts/poc_dualwrite_compare.sh use_nested` / `... effect_handler` | ✅ 差分 0 | `reports/spec-audit/ch1/module_parser-20251119-dualwrite.md` に結果を保存。`docs/plans/rust-migration/3-0-ci-and-dual-write-strategy.md` の CI ブロッカーへ module_parser チェックを追加。 |
| 監査チェックリスト更新 | N/A | ✅ `Closed` | `docs/notes/spec-integrity-audit-checklist.md` の `SYNTAX-002/module_parser` 行を `Closed (P2-8 W38)` に更新し、証跡リンクとして `module_parser-20251119-{parser-tests,dualwrite}.md` と 3 つの `*-20251119-diagnostics.json` を登録。 |

- 追加証跡: `reports/spec-audit/ch1/use_nested-20251119-{diagnostics.json,trace.md}`, `block_scope-20251119-{diagnostics.json,trace.md}`, `effect_handler-20251119-{diagnostics.json,trace.md}` を保存し、各ファイル末尾に `git rev-parse HEAD = f9e10ae676bca22ed8a41e96d79f667310274990` をコメントで追記。TraceEvent の `trace_id` は `syntax:module-stage::<stage>` / `syntax:module-decl::<kind>` を採用。 |

## 2025-11-21 Streaming 監査

| サンプル | コマンド | 結果 | 備考 |
|----------|----------|------|------|
| streaming_metrics.rs | `cargo test --manifest-path compiler/rust/frontend/Cargo.toml streaming_metrics -- --nocapture` | ✅ 緑化 (`CI_RUN_ID=rust-frontend-streaming-20251121.1`) | `reports/spec-audit/ch1/streaming_metrics-20251121-log.md` に streaming サンプルのテストログを保存。`module_header_acceptance` / `effect_handler_acceptance` / `bridge_signal_roundtrip` を追加し、`StreamFlowState::latest_bridge_signal()` の戻り値が単一段 `Option` であることを検証。 |
| streaming_use_nested.reml | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested.reml --stream chunk=4096` | ✅ 診断 0 件 | `reports/spec-audit/ch1/streaming_use_nested-20251121-diagnostics.json` を作成し、同名ファイルを `reports/spec-audit/ch2/streaming/` に複製。`git rev-parse HEAD = 3c92026356502383863dee228220ecdf02c24fd8` をコメントで追記。 |
| streaming_effect_handler.reml | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/effect_handler.reml --stream chunk=<len-1>` | ✅ 診断 0 件 | `reports/spec-audit/ch1/streaming_effect_handler-20251121-diagnostics.json` を保存し、`bridge_signal_roundtrip` と照合。`docs/notes/spec-integrity-audit-checklist.md#期待集合err-001` の `parser.expected_summary_presence = 1.0` を更新。 |

- Streaming ログは Chapter 2 側の `reports/spec-audit/ch2/streaming/` にも同名で複製し、`ERR-001` 監査と `collect-iterator-audit-metrics.py --section streaming` の参照点とする。

## 2025-11-22 Trace coverage

| サンプル | コマンド | 結果 | 備考 |
|----------|----------|------|------|
| effect_handler.reml | `scripts/poc_dualwrite_compare.sh effect_handler --trace --run-id rust-frontend-w39-20251122.1` | ✅ `Trace coverage >= 4`（handle / perform / resume / block） | `reports/spec-audit/ch1/trace-coverage-20251122.md` に `trace_ids` と診断 JSON の対応、`CI_RUN_ID=rust-frontend-w39-20251122.1`、`git rev-parse HEAD = f9e10ae676bca22ed8a41e96d79f667310274990` を保存。 |

- `trace-coverage-20251122.md` は `FrontendDiagnostic.extensions.trace_ids` と `syntax:expr::<kind>` の突き合わせログ。`docs/notes/spec-integrity-audit-checklist.md#rust-gap-トラッキング表` (`SYNTAX-003`) と `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md` の Trace/Diagnostics 拡張ステップから参照する。
