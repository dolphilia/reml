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

## 2025-12-24 docs-examples-audit フェーズ 3 復元
| JST 時刻 | コマンド | 結果 | 備考 |
|----------|----------|------|------|
| 07:14 | `for file in sec_b_1_1 sec_b_4-f sec_b_5-c sec_b_8_3_2 sec_c_4-a sec_c_4-b sec_c_4-c sec_c_4-d sec_c_4-e sec_c_7 sec_section-b sec_e_2; do compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics --allow-top-level-expr examples/docs-examples/spec/1-1-syntax/$file.reml; done` | ✅ 対象 12 件 diagnostics 0 件 | `conductor`/`unsafe`/`...`/トップレベル式の復元分を再検証。`sec_b_1_1` の `conductor` は `@dsl_export` 省略のまま通過。 |
| 07:30 | `for file in sec_b_3 sec_f sec_h_2-a sec_h_2-b; do compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics examples/docs-examples/spec/1-2-types-Inference/$file.reml > reports/spec-audit/ch1/1-2-types-Inference__${file}-20251224-diagnostics.json; done` | ✅ 対象 4 件 diagnostics 0 件 | `docs/spec/1-2-types-Inference.md` と `examples/docs-examples/spec/1-2-types-Inference/*.reml` を更新。`reports/spec-audit/ch1/docs-examples-fix-notes-20251224.md` に記録。 |
| 13:24 | 手動編集（`docs/spec/1-2-types-Inference.md` / `examples/docs-examples/spec/1-2-types-Inference/*.reml`） | ⏳ 未再検証 | `[T]` / `&mut State` の正準表記へ復元。再検証は `reports/spec-audit/ch1/docs-examples-fix-notes-20251224.md` に記載。 |
| 13:27 | `for file in sec_b_3 sec_f sec_h_2-a sec_h_2-b; do compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics examples/docs-examples/spec/1-2-types-Inference/$file.reml > reports/spec-audit/ch1/1-2-types-Inference__${file}-20251224-diagnostics.json; done` | ⚠️ 対象 4 件中 1 件 OK / 3 件 NG | `sec_b_3` / `sec_h_2-a` が `parser.syntax.expected_tokens`、`sec_f` が `parser.lexer.unknown_token` + `parser.syntax.expected_tokens`。在庫表を更新。 |
| 13:32 | `cargo build --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend` / `for file in sec_b_3 sec_f sec_h_2-a sec_h_2-b; do compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics examples/docs-examples/spec/1-2-types-Inference/$file.reml > reports/spec-audit/ch1/1-2-types-Inference__${file}-20251224-diagnostics.json; done` | ✅ 対象 4 件 diagnostics 0 件 | `reml_frontend` を再ビルド後に再検証し、`[T]` / `&mut` の受理が通過。 |
| 13:48 | `for file in sec_c-a sec_c-b sec_e sec_f sec_g sec_j_3 sec_j_4; do compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics examples/docs-examples/spec/1-3-effects-safety/$file.reml > reports/spec-audit/ch1/1-3-effects-safety__${file}-20251224-diagnostics.json; done` | ✅ 対象 7 件 diagnostics 0 件 | `sec_g` は `defer` 未対応のため `f.close()` を明示してフォールバック。 |
| 13:28 | `for file in sec_a sec_c sec_d sec_d_1 sec_clilsp sec_g; do compiler/rust/frontend/target/debug/reml_frontend examples/docs-examples/spec/2-1-parser-type/${file}.reml; done` | ✅ 対象 6 件 diagnostics 0 件 | `docs/spec/2-1-parser-type.md` とサンプルを整合更新。`reports/spec-audit/ch2/docs-examples-fix-notes-20251224.md` に記録。 |
| 15:10 | 手動整理（Backend/Runtime 追随ログ） | ✅ 記録 | docs-examples 由来の実装修正に着手。対象: `sec_b_3`/`sec_f`/`sec_h_2-a`（`[T]`/`&mut`/`Parser<T>` の型表記）、`sec_b_4-f`（`extern "C"` の `...` variadic）。詳細メモは `reports/spec-audit/ch1/docs-examples-fix-notes-20251224.md` に追記。 |

## 2025-12-26 docs-examples-audit フェーズ 3 再検証
| JST 時刻 | コマンド | 結果 | 備考 |
|----------|----------|------|------|
| 07:26 | `for file in examples/docs-examples/spec/2-1-parser-type/sec_*.reml; do compiler/rust/frontend/target/debug/reml_frontend "$file"; done` | ✅ 対象 7 件 diagnostics 0 件 | `sec_a` / `sec_c` / `sec_d` / `sec_clilsp` を復元済みのまま通過。`reports/spec-audit/ch2/docs-examples-fix-notes-20251224.md` に追記。 |
| 07:27 | `cargo test --manifest-path compiler/rust/frontend/Cargo.toml` | ❌ `lexer_unicode_identifier` 2 件失敗 | `unicode_identifier_error_matrix` が `UnknownToken`、`unicode_identifier_success_matrix` が未定義トークンで失敗。 |

## Rust ツールチェーン更新（stable 版の確定）
- `rust-toolchain.toml` を追加し、`channel = "stable"` と `components = ["rustfmt", "clippy"]` を設定。
- `rustup` を導入して `rustup update stable` を実行し、`rustc 1.92.0 (ded5c06cf 2025-12-08)` を stable として確定。

## Rust ツールチェーン更新（フェーズ 4: 再ビルドと検証）
| JST 時刻 | コマンド | 結果 | 備考 |
|----------|----------|------|------|
| 07:01 | `cargo build --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend` | ✅ 成功 | `time 0.3.30` の型推論エラー解消後にビルド通過。`core_prelude` 未定義など `reml_runtime` 側の警告は継続。 |
| 07:02 | `cargo build --manifest-path compiler/rust/runtime/Cargo.toml` | ✅ 成功 | `core_prelude` 未定義 (`unexpected_cfgs`) など警告 21 件。 |
| 07:02 | `cargo build --manifest-path compiler/rust/tooling/Cargo.toml` | ⏸️ 対象なし | `compiler/rust/tooling` が存在しないため実施不要。 |

## Rust ツールチェーン更新: 修正対応ログテンプレート
| JST 時刻 | 対象クレート | 症状/ログ要約 | 原因切り分け | 修正内容 | パッチ有無 | 結果 | 備考 |
|----------|--------------|----------------|--------------|----------|-----------|------|------|
| 06:55 | `compiler/rust/runtime` / `compiler/rust/runtime/ffi` | `time 0.3.30` で `error[E0282]` | `time` クレートが `rustc 1.92` で型推論失敗 | `time = "0.3.36"` に緩和し `cargo update -p time` で `0.3.44` へ更新 | `なし` | ✅ 成功 | `compiler/rust/runtime/Cargo.toml` / `compiler/rust/runtime/ffi/Cargo.toml` と各 `Cargo.lock` を更新。 |
| 06:58 | `compiler/rust/frontend` | `error[E0308]`/`error[E0609]` でビルド停止 | `OperationDecl` に `body` がなく、extern パラメータ型が `Param` と不一致 | `inspect_decl` の `effect` 分岐から `body` 参照を削除し、extern パラメータを `Param` へ統一 | `なし` | ✅ 成功 | `compiler/rust/frontend/src/parser/mod.rs` を修正。 |
| 00:00 | `compiler/rust/<crate>` |  |  |  | `なし` / `あり` |  |  |
| 10:30 | `compiler/rust/frontend` | `error[E0599]` でビルド停止 | `frontend` のみ再現、依存更新が要因 | `parser/lexer.rs` の型変換を修正 | `なし` | ✅ 成功 | `cargo build --manifest-path compiler/rust/frontend/Cargo.toml` |
