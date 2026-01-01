# ch2 - Chapter 2 監査ログ

- 対象: `docs/spec/2-0-parser-api-overview.md`〜`2-6-execution-strategy.md`, `docs/guides/compiler/core-parse-streaming.md`。
- 保存物: Streaming Runner JSON（`streaming/`）、Recover & Fix-it ログ（`recover/`）、`collect-iterator-audit-metrics.py --section parser` の結果。
- 手順: `cargo test --manifest-path compiler/rust/frontend/Cargo.toml streaming::tests -- --nocapture`、`compiler/rust/frontend/tests/streaming_runner.rs`、`scripts/validate-diagnostic-json.sh --mode streaming` を実行し、出力を保存する。
- 更新責任者: Parser API WG（#parser-api）。

## 2025-11-21 追記

- `streaming/streaming_use_nested-20251121-diagnostics.json`
- `streaming/streaming_effect_handler-20251121-diagnostics.json`

いずれも `CI_RUN_ID=rust-frontend-streaming-20251121.1` と `git_rev=3c92026356502383863dee228220ecdf02c24fd8` を含み、Chapter 1 側の同名ファイルとハッシュ一致することを確認済み。
