# 1.1 構文サンプルセット

Chapter 1 の仕様で参照している Reml コード例を `.reml` ファイルとして切り出し、`reml_frontend` で監査できるようにした。Phase 2-8 では次の方針でメンテナンスする。

- `use_nested.reml` / `effect_handler.reml` などは仕様本文と 1:1 で対応する**正準サンプル**。脚注から直接リンクし、`reports/spec-audit/ch1/` のログと突き合わせて状態を記録する。
- `*_rustcap.reml` は Phase 2-7 以前のフォールバックとして履歴保管のみを行う。Rust Frontend + Streaming ランナーで正準サンプルが受理できるため、監査ベースラインやチェックリストでは使用しない。
- 検証は `cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --emit-diagnostics <sample> --emit-typeck-debug reports/spec-audit/ch1/<sample>-YYYYMMDD-typeck.json` を基本とし、必要に応じて `--emit-ast` や `--emit-impl-registry` を追加する。`typeck` JSON に `schema_version = "3.0.0-alpha"`、`stage_trace`、`used_impls` が含まれていることを確認し、コマンドと結果を `reports/spec-audit/summary.md` に追記する。
- 新しいコード片を仕様に追加する際は、ここへも `.reml` を追加し、`docs/spec/0-3-code-style-guide.md` §8 のチェックリストを更新する。
- `module_parser` の再実装に関わるサンプル（`use_nested.reml`, `effect_handler.reml`, `block_scope.reml` など）は `cargo test --manifest-path compiler/frontend/Cargo.toml parser::module -- --nocapture` の結果と紐付け、`reports/spec-audit/ch1/module_parser-YYYYMMDD-parser-tests.md` / `module_parser-YYYYMMDD-dualwrite.md` に保存する。`docs/notes/process/spec-integrity-audit-checklist.md#rust-gap-トラッキング表` の `SYNTAX-002/module_parser` 行とログ名を一致させること。

## Streaming 経由での再検証（W38 追加）

- Streaming Runner で再検証する際は `cargo test --manifest-path compiler/frontend/Cargo.toml streaming_metrics -- --nocapture` を正規ルートとし、ログを `reports/spec-audit/ch1/streaming_metrics-YYYYMMDD-log.md` へ保存する。`CI_RUN_ID` と `git rev-parse HEAD` をヘッダに記載する。
- `use_nested.reml` / `effect_handler.reml` については Streaming 実行後の診断結果を `reports/spec-audit/ch1/streaming_use_nested-YYYYMMDD-diagnostics.json`、`streaming_effect_handler-YYYYMMDD-diagnostics.json` に分けて格納し、同名ファイルを `reports/spec-audit/ch2/streaming/` に複製する。サンプルごとに `mode = streaming`、`ci_run_id`、`git_rev` を JSON に含める。
- `docs/spec/1-1-syntax.md` の監査ノートと `docs/spec/0-3-code-style-guide.md` のチェックリストは Streaming 実行がグリーンであることを前提にし、フォールバック（`*_rustcap.reml`）の復帰条件は Streaming/Cargo 実行がいずれか失敗した場合のみとする。
- CLI でストリーミング経路を検証する場合は `cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --emit-diagnostics examples/docs-examples/spec/1-1-syntax/<sample>.reml --stream chunk=<bytes> --emit-typeck-debug reports/spec-audit/ch1/<sample>-YYYYMMDD-typeck.json` を用い、生成された `streaming_<sample>-YYYYMMDD-diagnostics.json` と `typeck` JSON を `reports/spec-audit/ch1/2025-11-17-syntax-samples.md#2025-11-21-streaming-監査` に追記する。

`rust-gap` の解消順序は `docs/notes/process/spec-integrity-audit-checklist.md#rust-gap-トラッキング表` に従う。

## 監査ログと Trace Coverage（W39 追加）

- `scripts/poc_dualwrite_compare.sh effect_handler --trace` を正規ルートとして `syntax:expr::<kind>` 系のトレースを取得し、`reports/spec-audit/ch1/effect_handler-YYYYMMDD-trace.md` / `block_scope-YYYYMMDD-trace.md` などに保存する。`--trace` を併用すると `FrontendDiagnostic.extensions.trace_ids` に紐付いたイベントだけを抽出でき、`reports/spec-audit/ch1/trace-coverage-YYYYMMDD.md` へ CLI コマンド／`CI_RUN_ID`／`git rev-parse HEAD` と合わせて記録する。
- `trace-coverage-YYYYMMDD.md` では `Trace coverage >= 4`（handle / perform / resume / block の 4 系統）を満たしているかを確認し、`docs/notes/process/spec-integrity-audit-checklist.md#rust-gap-トラッキング表` の `SYNTAX-003` 行へ証跡リンクを追加する。
- `trace_id` 命名規約は `docs/plans/rust-migration/unified-porting-principles.md` の観測基盤セクションに従い、`syntax:expr::<kind>` / `syntax:effect::<kind>` / `syntax:handler::<name>` / `syntax:operation::resume` を使う。既存ログを更新する際は `trace_ids` 配列と一緒に差分メモへもリンクする。
