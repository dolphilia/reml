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

## 2025-11-18 (W37 後半) ExprParser / 効果構文
| JST 時刻 | コマンド | 結果 | 備考 |
|----------|----------|------|------|
| 09:35 | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/block_scope.reml --trace-output reports/spec-audit/ch1/block_scope-20251118-trace.md` | ✅ 診断 0 件 | `ExprParser` へブロック/`let`/`var` を導入。ログ: `reports/spec-audit/ch1/block_scope-20251118-diagnostics.json`。 |
| 10:12 | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/effect_handler.reml --trace-output reports/spec-audit/ch1/effect_handler-20251118-trace.md` | ✅ 診断 0 件 | `perform`/`handle`/`operation` が Rust Frontend で受理。dual-write: `reports/spec-audit/ch1/effect_handler-20251118-dualwrite.md`。`rust-gap SYNTAX-003` をクローズ。 |

## 2025-12-23 docs-examples-audit 一括検証
| JST 時刻 | コマンド | 結果 | 備考 |
|----------|----------|------|------|
| 17:00 | `compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics <sample>` | ⚠️ 438 件中 53 件 OK / 385 件 NG | `docs/plans/docs-examples-audit/1-1-spec-code-block-inventory.md` を更新し、`reports/spec-audit/ch0`〜`ch4` に `docs-examples-audit-20251223.md` と診断 JSON を保存。 |
| 17:08 | 手動編集（`docs/spec/0-3-code-style-guide.md` / `examples/docs-examples/spec/0-3-code-style-guide/*.reml`） | ✅ ch0 の NG 2 件を修正 | `reports/spec-audit/ch0/docs-examples-fix-notes-20251223.md` に修正メモを追記。再検証は未実施。 |
| 17:10 | `compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics examples/docs-examples/spec/0-3-code-style-guide/sec_3_4.reml` / `compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics examples/docs-examples/spec/0-3-code-style-guide/sec_3_5.reml` | ✅ diagnostics 0 件 | `docs/plans/docs-examples-audit/1-1-spec-code-block-inventory.md` の状態を `ok` に更新。 |
| 20:50 | `for file in sec_b_1_1 sec_b_4-c sec_b_4-e sec_b_4-f sec_b_5-c sec_b_6 sec_section-b sec_b_8_3_2 sec_b_8_5 sec_c_2 sec_c_4-a sec_c_4-b sec_c_4-c sec_c_4-d sec_c_4-e sec_c_6 sec_c_7 sec_e_2 sec_g; do compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics examples/docs-examples/spec/1-1-syntax/$file.reml; done` | ✅ 対象 19 件 diagnostics 0 件 | `docs/spec/1-1-syntax.md` と `examples/docs-examples/spec/1-1-syntax/*.reml` を整合更新。`reports/spec-audit/ch1/docs-examples-fix-notes-20251223.md` に修正メモを記録。 |
| 21:30 | `compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics examples/docs-examples/spec/1-1-syntax/<phase3>` | ✅ 対象 11 件 diagnostics 0 件 | `--allow-top-level-expr` が未提供で復元分はフォールバックへ戻し、`sec_b_8_3_2` の `conductor` 例のみ維持。`reports/spec-audit/ch1/docs-examples-fix-notes-20251223.md` に再検証メモを追記。 |

## Rust ツールチェーン更新（stable 版の確定）
- `rust-toolchain.toml` を追加し、`channel = "stable"` と `components = ["rustfmt", "clippy"]` を設定。
- `rustup` を導入して `rustup update stable` を実行し、`rustc 1.92.0 (ded5c06cf 2025-12-08)` を stable として確定。

## Rust ツールチェーン更新: 修正対応ログテンプレート
| JST 時刻 | 対象クレート | 症状/ログ要約 | 原因切り分け | 修正内容 | パッチ有無 | 結果 | 備考 |
|----------|--------------|----------------|--------------|----------|-----------|------|------|
| 00:00 | `compiler/rust/<crate>` |  |  |  | `なし` / `あり` |  |  |
| 10:30 | `compiler/rust/frontend` | `error[E0599]` でビルド停止 | `frontend` のみ再現、依存更新が要因 | `parser/lexer.rs` の型変換を修正 | `なし` | ✅ 成功 | `cargo build --manifest-path compiler/rust/frontend/Cargo.toml` |
