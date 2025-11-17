# ch2 - Chapter 2 監査ログ

- 対象: `docs/spec/2-0-parser-api-overview.md`〜`2-6-execution-strategy.md`, `docs/guides/core-parse-streaming.md`。
- 保存物: Streaming Runner JSON（`streaming/`）、Recover & Fix-it ログ（`recover/`）、`collect-iterator-audit-metrics.py --section parser` の結果。
- 手順: `cargo test --manifest-path compiler/rust/frontend/Cargo.toml streaming::tests -- --nocapture`、`compiler/rust/frontend/tests/streaming_runner.rs`、`scripts/validate-diagnostic-json.sh --mode streaming` を実行し、出力を保存する。
- 更新責任者: Parser API WG（#parser-api）。
