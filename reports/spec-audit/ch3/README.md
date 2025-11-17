# ch3 - Chapter 3 監査ログ

- 対象: `docs/spec/3-0-core-library-overview.md`〜`3-10-core-env.md`, `docs/spec/3-6-core-diagnostics-audit.md`, `docs/spec/3-8-core-runtime-capability.md`, `docs/guides/runtime-bridges.md`。
- 保存物: Rust Runtime/Adapter テスト結果、`collect-iterator-audit-metrics.py --section diagnostics|effects` の出力、`audit` JSON スナップショット。
- 手順: `cargo test --manifest-path compiler/rust/runtime/ffi/Cargo.toml`, `cargo test --manifest-path compiler/rust/adapter/Cargo.toml`, `python3 tooling/ci/collect-iterator-audit-metrics.py --section diagnostics --require-success` を実行し、標準出力を貼付する。
- 更新責任者: Rust Runtime WG（#rust-runtime）。
