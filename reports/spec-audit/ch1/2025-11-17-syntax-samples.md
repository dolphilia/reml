# 2025-11-17 Syntax Samples

| サンプル | コマンド | 結果 | 備考 |
|----------|----------|------|------|
| use_nested.reml | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested.reml --trace-output reports/spec-audit/ch1/use_nested-YYYYMMDD-trace.md` | ⚠️ `構文エラー: 入力を解釈できません` | `module`/`use` は受理され `TraceEvent::*Accepted` を記録できたが、`fn ... { ... }`（ブロック構文）の受理が未実装なため `let` 行で停止。`rust-gap SYNTAX-002` を継続。 |
| use_nested_rustcap.reml | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested_rustcap.reml` | ✅ 診断 0 件 | ダミー関数→`use` の順に配置し、戻り値注釈を省略。Rust Frontend の現状で再現できる最小構成。 |
| effect_handler.reml | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/effect_handler.reml` | ⚠️ `構文エラー: 入力を解釈できません` | `effect` 宣言をパーサが受理できず、`rust-gap SYNTAX-003` を継続。 |

## 保存ルール（Phase 2-8 W37 追補）

- `use_nested.reml` / `effect_handler.reml` の診断結果は `reports/spec-audit/ch1/<sample>-YYYYMMDD-diagnostics.json` 形式で保存し、`YYYYMMDD` は CI 実行日、ファイル末尾に `git rev-parse HEAD` をコメントとして追記する（2025-11-17 実行分は追記待ち）。
- Rust Frontend で `use_nested.reml` を実行する際は `--trace-output reports/spec-audit/ch1/use_nested-YYYYMMDD-trace.md` を併用し、`TraceEvent::ModuleHeaderAccepted` / `TraceEvent::UseDeclAccepted` を記録する。
- `use_nested_rustcap.reml` はフォールバックが不要になったら削除予定のため、`docs/spec/1-1-syntax.md` 脚注と `reports/spec-audit/diffs/SYNTAX-002-ch1-rust-gap.md` のクローズ判定を同期させる。
