# docs-examples 修正メモ (ch1 / 2025-12-24)

## 代表診断
- `parser.syntax.expected_tokens`: 型注釈に `[T]` を使ったサンプルや本体省略の宣言で構文エラー。
- `parser.lexer.unknown_token`: `&mut` の参照トークンが未定義。

## 修正対象
- `examples/docs-examples/spec/1-2-types-Inference/sec_b_3.reml`: `[T]` を `List<T>` へ置換し、`...` を `zero()` に置換。
- `examples/docs-examples/spec/1-2-types-Inference/sec_f.reml`: `&mut` を除去し `fn(State) -> Reply<T>` へ簡略化、`then` を `then_` へ変更、宣言に簡易ボディを追加、`many` の戻り型を `List<A>` へ変更。
- `examples/docs-examples/spec/1-2-types-Inference/sec_h_2-a.reml`: `[T]` を `List<T>` へ置換し、`fold` の名前付き引数を位置引数へ変更。
- `examples/docs-examples/spec/1-2-types-Inference/sec_h_2-b.reml`: 呼出側の例を `List::from` へ更新。

## 仕様整合メモ
- `docs/spec/1-2-types-Inference.md` の B.3 / F / H.2 のコードブロックを更新。
- 在庫表 `docs/plans/docs-examples-audit/1-1-spec-code-block-inventory.md` を `ok` / `validation:ok` へ更新。
- 再検証は `compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics <sample>` を使用し、3 件とも diagnostics 0 件を確認。

## フェーズ3 復元対応（2025-12-24）
- `examples/docs-examples/spec/1-2-types-Inference/sec_b_3.reml` / `sec_h_2-a.reml` を `[T]` へ復元。
- `examples/docs-examples/spec/1-2-types-Inference/sec_h_2-b.reml` を `sum([1, 2, 3])` へ復元。
- `examples/docs-examples/spec/1-2-types-Inference/sec_f.reml` を `&mut State` 付きの `Parser<T>` 定義へ復元。
- `docs/spec/1-2-types-Inference.md` の B.3 / F / H.2 を正準例へ差し戻し。
- 再検証を実施し、診断 JSON を再生成。
  - `sec_b_3`: `parser.syntax.expected_tokens`（診断 1 件）。
  - `sec_f`: `parser.lexer.unknown_token` / `parser.syntax.expected_tokens`（診断 2 件）。
  - `sec_h_2-a`: `parser.syntax.expected_tokens`（診断 1 件）。
  - `sec_h_2-b`: diagnostics 0 件。
- `reml_frontend` 再ビルド後に再検証し、対象 4 件は diagnostics 0 件へ改善。
