# ch1 - Chapter 1 監査ログ

- 対象: `docs/spec/1-1-syntax.md`〜`1-5-formal-grammar-bnf.md`, `docs/spec/1-2-types-Inference.md`, `docs/spec/1-3-effects-safety.md`。
- 保存物: `cargo run --bin poc_frontend --emit-*` によるサンプル JSON、Rust Frontend の `cargo test` 成果ログ、`syntax.effect_construct_acceptance` 計測結果。
- 手順: `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics --emit-typeck-debug --input <sample>` を実行し、標準出力と JSON を日付別フォルダへ保存。
- 更新責任者: Rust Parser WG（#rust-frontend-parser）。

## 2025-11-17 実行済みサンプル

- `docs/spec/1-1-syntax/examples/use_nested.reml` — 正準サンプル。`module`/`use` に加えて `fn ... { ... }` ブロック／`match` 構文まで Rust Frontend が受理し、`TraceEvent::{ModuleHeaderAccepted,UseDeclAccepted}` を `reports/spec-audit/ch1/use_nested-20251117-trace.md` に記録する（診断 0 件）。`use_nested_rustcap.reml` は参考用途のみ。
- `docs/spec/1-1-syntax/examples/use_nested_rustcap.reml` — Rust Frontend 制限を回避したフォールバック。ダミー関数→`use`→宣言の順で並べ、戻り値型を省略。診断 0 件で完了 (`reports/spec-audit/ch1/use_nested_rustcap-20251117-diagnostics.json`)。
- `docs/spec/1-1-syntax/examples/effect_handler.reml` — 効果構文の PoC。2025-11-18 の再実行で `ExprParser`／effect handler 実装が揃い、`reports/spec-audit/ch1/effect_handler-20251118-diagnostics.json` に診断 0 件の結果を保存。旧ログ `effect_handler-20251117-diagnostics.json` はギャップ再現用として保管。

`reports/spec-audit/summary.md` にコマンド・タイムスタンプを追記し、`docs/notes/process/spec-integrity-audit-checklist.md` で `rust-gap` 状態を更新する。

## 2025-11-18 追加サンプル

- `docs/spec/1-1-syntax/examples/block_scope.reml` — `let`/`var` によるブロックスコープと `return` を Rust Frontend が受理し、`reports/spec-audit/ch1/block_scope-20251118-diagnostics.json` に結果を保存。`BindingKind` と `TypeAnnot::Pending` の整合を確認。
- `docs/spec/1-1-syntax/examples/effect_handler.reml` — dual-write 比較を `reports/spec-audit/ch1/effect_handler-20251118-dualwrite.md` に整理し、トレースは `effect_handler-20251118-trace.md` に記録済み。

## 2025-11-21 Streaming 監査の追加物

- `reports/spec-audit/ch1/streaming_metrics-20251121-log.md` — `cargo test --manifest-path compiler/rust/frontend/Cargo.toml streaming_metrics -- --nocapture` のログ。`module_header_acceptance` / `effect_handler_acceptance` / `bridge_signal_roundtrip` の 3 テストを Streaming Runner で固定した証跡。
- `reports/spec-audit/ch1/streaming_use_nested-20251121-diagnostics.json` / `streaming_effect_handler-20251121-diagnostics.json` — Streaming 実行時の診断結果。`mode = streaming`、`ci_run_id = rust-frontend-streaming-20251121.1`、`git_rev` を JSON のメタ情報に含め、Chapter 2 側の `reports/spec-audit/ch2/streaming/` にも複製する。

## 2027-03-30 Unicode Diagnostics の指標

- `reports/spec-audit/ch1/unicode_diagnostics-20270330.json` — `cargo test --manifest-path compiler/rust/frontend/Cargo.toml lexer_unicode_identifier -- --nocapture` 由来の Unicode エラー診断サマリ。`diagnostic.extensions["unicode"]` と `AuditEnvelope.metadata["unicode.*"]` が揃っていること、および `unicode.display_width` が書き出されることを `scripts/validate-diagnostic-json.sh --pattern unicode.display_width reports/spec-audit/ch1/unicode_diagnostics-20270330.json` で検証する。
- KPI `unicode.diagnostic.display_span` / `unicode.display_width` の達成状況を記録し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ転記する。

## 2027-03-30 Core Text Samples

- `reports/spec-audit/ch1/core_text_examples-20270330.md` — `examples/core-text/text_unicode.reml` のゴールデン更新ログ (`text_unicode.tokens`/`text_unicode.grapheme_stats`/`text_unicode.stream_decode`) とコマンド履歴。
- サンプルは `cargo run --manifest-path compiler/rust/runtime/Cargo.toml --bin text_stream_decode -- --input tests/data/unicode/streaming/sample_input.txt --output examples/core-text/expected/text_unicode.stream_decode.golden` で再現できる。`docs/spec/3-3-core-text-unicode.md` §9 と `docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md` §5 の参照先として利用する。
