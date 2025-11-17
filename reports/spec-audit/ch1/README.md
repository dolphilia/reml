# ch1 - Chapter 1 監査ログ

- 対象: `docs/spec/1-1-syntax.md`〜`1-5-formal-grammar-bnf.md`, `docs/spec/1-2-types-Inference.md`, `docs/spec/1-3-effects-safety.md`。
- 保存物: `cargo run --bin poc_frontend --emit-*` によるサンプル JSON、Rust Frontend の `cargo test` 成果ログ、`syntax.effect_construct_acceptance` 計測結果。
- 手順: `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics --emit-typeck-debug --input <sample>` を実行し、標準出力と JSON を日付別フォルダへ保存。
- 更新責任者: Rust Parser WG（#rust-frontend-parser）。

## 2025-11-17 実行済みサンプル

- `docs/spec/1-1-syntax/examples/use_nested.reml` — 正準サンプル。`module`/`use` は受理され `TraceEvent::{ModuleHeaderAccepted,UseDeclAccepted}` が `reports/spec-audit/ch1/use_nested-20251117-trace.md` に出力されるが、ブロック本体（`fn ... { ... }`）の解析が未実装のため `reports/spec-audit/ch1/use_nested-20251117-diagnostics.json` では `let` 行で停止する（`rust-gap SYNTAX-002` 継続）。
- `docs/spec/1-1-syntax/examples/use_nested_rustcap.reml` — Rust Frontend 制限を回避したフォールバック。ダミー関数→`use`→宣言の順で並べ、戻り値型を省略。診断 0 件で完了 (`reports/spec-audit/ch1/use_nested_rustcap-20251117-diagnostics.json`)。
- `docs/spec/1-1-syntax/examples/effect_handler.reml` — 効果構文の PoC。`effect` 宣言で即時失敗し、`rust-gap SYNTAX-003` を継続。ログ: `reports/spec-audit/ch1/effect_handler-20251117-diagnostics.json`。

`reports/spec-audit/summary.md` にコマンド・タイムスタンプを追記し、`docs/notes/spec-integrity-audit-checklist.md` で `rust-gap` 状態を更新する。
