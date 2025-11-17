# spec-audit 実行ログ

## 2025-11-17 (W36 後半) Rust Frontend ベースライン
| JST 時刻 | コマンド | 結果 | 備考 |
|----------|----------|------|------|
| 12:21 | `cargo test --manifest-path compiler/rust/frontend/Cargo.toml` | ✅ 成功（30 tests, streaming_metrics.rs 5件含む） | `StreamFlowState::latest_bridge_signal` を含む streaming 経路が通過。出力ログは `compiler/rust/frontend/target/debug/test-logs/`（ローカル）と `reports/spec-audit/ch2/README.md` の指示に従って後続で抜粋予定。 |
| 12:28 | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --help` | ✅ 成功 | CLI が `lex-profile` / `streaming` / `effect-stage` オプションを表示することを確認。`reports/spec-audit/ch0/links.md` から Chapter 0 の索引に反映予定。 |
| 12:39 | `python3 - <<'PY' ...` | ✅ Chapter 0 リンク存在チェック完了 | `docs/spec/0-0-overview.md`〜`docs/spec/README.md` のローカルリンクを一括検査。`../reports/diagnostic-format-regression.md` と `[参照](file.md)` の誤りを検出し `reports/spec-audit/ch0/links.md` に記録。 |
| 12:41 | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested.reml` | ⚠️ `構文エラー` | 先頭 `module`/`use` を Rust Frontend が受理できず失敗。ログは `reports/spec-audit/ch1/use_nested-20251117-diagnostics.json` (`rust-gap SYNTAX-002`)。 |
| 12:42 | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested_rustcap.reml` | ✅ 診断 0 件 | ダミー関数→`use` の順に並べたフォールバックで `use` ネスト構文を検証。ログ: `reports/spec-audit/ch1/use_nested_rustcap-20251117-diagnostics.json`。 |
| 12:43 | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/effect_handler.reml` | ⚠️ `構文エラー` | `effect` 宣言が未実装のため失敗。`reports/spec-audit/ch1/effect_handler-20251117-diagnostics.json` で `rust-gap SYNTAX-003` を継続。 |
| 15:58 | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested.reml --trace-output reports/spec-audit/ch1/use_nested-20251117-trace.md` | ✅ 診断 0 件 | ブロック／`match` 構文を Rust Frontend が受理できるようになり、正準サンプル `use_nested.reml` の `rust-gap SYNTAX-002` を解消。`use_nested-20251117-diagnostics.json` / `use_nested-20251117-trace.md` を更新。 |
| 15:33 | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested.reml --trace-output reports/spec-audit/ch1/use_nested-20251117-trace.md` | ⚠️ `構文エラー`（`let` 行） | `module`/`use` は受理され `TraceEvent::*Accepted` を保存できたが、ブロック構文が未実装のため `use_nested.reml` 本体は継続して失敗。 |

> 以降、各 Chapter レビュー完了後 24 時間以内に本表へ追記し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md#0.3.4a-phase-2-8-仕様監査スプリントrust-フォーカス` のスケジュールを満たすこと。
