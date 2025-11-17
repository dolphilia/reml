# 2025-11-17 Syntax Samples

| サンプル | コマンド | 結果 | 備考 |
|----------|----------|------|------|
| use_nested.reml | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested.reml --trace-output reports/spec-audit/ch1/use_nested-YYYYMMDD-trace.md` | ✅ 診断 0 件 | `module`/`use`/ブロック/`match` を Rust Frontend が受理し、`TraceEvent::{ModuleHeaderAccepted,UseDeclAccepted}` を保存できるようになった（2025-11-17 修正）。 |
| use_nested_rustcap.reml | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested_rustcap.reml` | ✅ 診断 0 件 | ダミー関数→`use` の順に配置し、戻り値注釈を省略。Rust Frontend の現状で再現できる最小構成。 |
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
| 監査チェックリスト更新 | N/A | ✅ `In Review` | `docs/notes/spec-integrity-audit-checklist.md` の `SYNTAX-002/module_parser` 行で `owner=Parser QA`、`evidence(log)=module_parser-20251119-parser-tests.md` を記録。 |
