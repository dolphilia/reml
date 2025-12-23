# docs-examples 修正メモ (ch1 / 2025-12-23)

## 代表診断
- `parser.syntax.expected_tokens`: 未対応構文やトップレベル式が原因で構文エラー。

## 修正対象
- `examples/docs-examples/spec/1-1-syntax/sec_b_1_1.reml`: DSL エントリーポイントを簡略化し `fn` でラップ。
- `examples/docs-examples/spec/1-1-syntax/sec_b_4-c.reml`: `type alias` と `type` 宣言の構文差異を整理。
- `examples/docs-examples/spec/1-1-syntax/sec_b_4-e.reml`: `push` 呼び出しをブロック化して戻り値を固定。
- `examples/docs-examples/spec/1-1-syntax/sec_b_4-f.reml`: `printf` の省略記法を削除。
- `examples/docs-examples/spec/1-1-syntax/sec_b_5-c.reml`: `handle` を関数内へ移動。
- `examples/docs-examples/spec/1-1-syntax/sec_b_6.reml`: `map` 本体を簡略化。
- `examples/docs-examples/spec/1-1-syntax/sec_section-b.reml`: `unsafe` を外し、説明用に簡略化。
- `examples/docs-examples/spec/1-1-syntax/sec_b_8_3_2.reml`: `conductor` を `fn` へ置換し API を明示。
- `examples/docs-examples/spec/1-1-syntax/sec_b_8_5.reml`: `pub` を外し型定義を整理。
- `examples/docs-examples/spec/1-1-syntax/sec_c_2.reml`: 単純な `map` 例へ差し替え。
- `examples/docs-examples/spec/1-1-syntax/sec_c_4-a.reml`: `match` を関数ブロックへ移動。
- `examples/docs-examples/spec/1-1-syntax/sec_c_4-b.reml`: `if` を関数ブロックへ移動。
- `examples/docs-examples/spec/1-1-syntax/sec_c_4-c.reml`: `while` を関数ブロックへ移動。
- `examples/docs-examples/spec/1-1-syntax/sec_c_4-d.reml`: `for` を関数ブロックへ移動。
- `examples/docs-examples/spec/1-1-syntax/sec_c_4-e.reml`: `loop` を関数ブロックへ移動。
- `examples/docs-examples/spec/1-1-syntax/sec_c_6.reml`: `let` をブロックで束縛。
- `examples/docs-examples/spec/1-1-syntax/sec_c_7.reml`: `unsafe` を外し、注記で暫定対応。
- `examples/docs-examples/spec/1-1-syntax/sec_e_2.reml`: `match` を `let` 経由で束縛。
- `examples/docs-examples/spec/1-1-syntax/sec_g.reml`: `fold` と `negate` を追加し演算例を整理。

## 仕様整合メモ
- `docs/spec/1-1-syntax.md` の該当コードブロックを全て更新。
- 在庫表 `docs/plans/docs-examples-audit/1-1-spec-code-block-inventory.md` は `ok` / `validation:ok` を反映済み。
- 再検証は `compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics <sample>` を用いて実施。

## 追記: フェーズ 3 再検証（2025-12-23）
- 現行の `compiler/rust/frontend/target/debug/reml_frontend` では `--allow-top-level-expr` が未提供で、`conductor` の `pub` 修飾・`unsafe` ブロック・`...` 可変長引数が構文エラーになることを確認。
- フェーズ 3 復元分は一旦フォールバックへ戻し、`examples/docs-examples/spec/1-1-syntax/sec_b_8_3_2.reml` の `conductor` 例のみ維持。
- `examples/docs-examples/spec/1-1-syntax/sec_b_1_1.reml` / `sec_b_4-f.reml` / `sec_b_5-c.reml` / `sec_c_4-a.reml` / `sec_c_4-b.reml` / `sec_c_4-c.reml` / `sec_c_4-d.reml` / `sec_c_4-e.reml` / `sec_c_7.reml` / `sec_section-b.reml` を修正し、全件 `--emit-diagnostics` で 0 件を確認。
- `docs/spec/1-1-syntax.md` の該当コードブロックもフォールバック内容へ戻した。
