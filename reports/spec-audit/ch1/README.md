# ch1 - Chapter 1 監査ログ

- 対象: `docs/spec/1-1-syntax.md`〜`1-5-formal-grammar-bnf.md`, `docs/spec/1-2-types-Inference.md`, `docs/spec/1-3-effects-safety.md`。
- 保存物: `cargo run --bin poc_frontend --emit-*` によるサンプル JSON、Rust Frontend の `cargo test` 成果ログ、`syntax.effect_construct_acceptance` 計測結果。
- 手順: `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics --emit-typeck-debug --input <sample>` を実行し、標準出力と JSON を日付別フォルダへ保存。
- 更新責任者: Rust Parser WG（#rust-frontend-parser）。
