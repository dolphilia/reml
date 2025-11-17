# ch1 - Chapter 1 監査ログ

- 対象: `docs/spec/1-1-syntax.md`〜`1-5-formal-grammar-bnf.md`, `docs/spec/1-2-types-Inference.md`, `docs/spec/1-3-effects-safety.md`。
- 保存物: `cargo run --bin poc_frontend --emit-*` によるサンプル JSON、Rust Frontend の `cargo test` 成果ログ、`syntax.effect_construct_acceptance` 計測結果。
- 手順: `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics --emit-typeck-debug --input <sample>` を実行し、標準出力と JSON を日付別フォルダへ保存。
- 更新責任者: Rust Parser WG（#rust-frontend-parser）。

## 2025-11-17 実行済みサンプル

- `docs/spec/1-1-syntax/examples/use_nested.reml` — 正準サンプル。`module`/`use` に加えて `fn ... { ... }` ブロック／`match` 構文まで Rust Frontend が受理し、`TraceEvent::{ModuleHeaderAccepted,UseDeclAccepted}` を `reports/spec-audit/ch1/use_nested-20251117-trace.md` に記録する（診断 0 件）。`use_nested_rustcap.reml` は参考用途のみ。
- `docs/spec/1-1-syntax/examples/use_nested_rustcap.reml` — Rust Frontend 制限を回避したフォールバック。ダミー関数→`use`→宣言の順で並べ、戻り値型を省略。診断 0 件で完了 (`reports/spec-audit/ch1/use_nested_rustcap-20251117-diagnostics.json`)。
- `docs/spec/1-1-syntax/examples/effect_handler.reml` — 効果構文の PoC。2025-11-18 の再実行で `ExprParser`／effect handler 実装が揃い、`reports/spec-audit/ch1/effect_handler-20251118-diagnostics.json` に診断 0 件の結果を保存。旧ログ `effect_handler-20251117-diagnostics.json` はギャップ再現用として保管。

`reports/spec-audit/summary.md` にコマンド・タイムスタンプを追記し、`docs/notes/spec-integrity-audit-checklist.md` で `rust-gap` 状態を更新する。

## 2025-11-18 追加サンプル

- `docs/spec/1-1-syntax/examples/block_scope.reml` — `let`/`var` によるブロックスコープと `return` を Rust Frontend が受理し、`reports/spec-audit/ch1/block_scope-20251118-diagnostics.json` に結果を保存。`BindingKind` と `TypeAnnot::Pending` の整合を確認。
- `docs/spec/1-1-syntax/examples/effect_handler.reml` — dual-write 比較を `reports/spec-audit/ch1/effect_handler-20251118-dualwrite.md` に整理し、トレースは `effect_handler-20251118-trace.md` に記録済み。
