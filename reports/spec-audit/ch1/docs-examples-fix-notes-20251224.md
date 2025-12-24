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

## 1-3-effects-safety サンプル修正（2025-12-24）
- `parser.syntax.expected_tokens`: `(+ )` の演算子セクション、`{ ...; ... }` 形式のセミコロン、`extern` 宣言末尾、`if ... { ... }` 形式が Rust Frontend で未受理。
- `parser.top_level_expr.disallowed`: `sec_e` の断片コードがトップレベル式扱いで失敗。
- `defer` 構文は Rust Frontend が未対応のため、`sec_g` は明示的な `close()` でフォールバック。

### 修正対象
- `examples/docs-examples/spec/1-3-effects-safety/sec_c-a.reml`: `fold(xs, 0, |acc, x| acc + x)` へ更新し、属性列に関数本体を付与。
- `examples/docs-examples/spec/1-3-effects-safety/sec_c-b.reml`: ブロックのセミコロンを除去し、`fold` をラムダへ更新。
- `examples/docs-examples/spec/1-3-effects-safety/sec_e.reml`: `sum` 関数に包んでトップレベル式を解消。
- `examples/docs-examples/spec/1-3-effects-safety/sec_f.reml`: `extern` 宣言にセミコロンを付与。
- `examples/docs-examples/spec/1-3-effects-safety/sec_g.reml`: `defer` をコメント化し `f.close()` を追加。
- `examples/docs-examples/spec/1-3-effects-safety/sec_j_3.reml`: `extern` 宣言にセミコロンを付与。
- `examples/docs-examples/spec/1-3-effects-safety/sec_j_4.reml`: `if ... then ... else ...` へ変更し、`?` を括弧付きで適用。

### 仕様整合メモ
- `docs/spec/1-3-effects-safety.md` の C / E / F / G / J.3 / J.4 のコードブロックを更新。
- `defer` 未対応に伴う実装ギャップ対応計画を `docs/plans/docs-examples-audit/1-3-impl-gap-plan-20251224.md` に追加。
- 再検証は `compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics <sample>` を使用し、対象 7 件は diagnostics 0 件を確認。

## 1-3-effects-safety `defer` 復元（2025-12-24）
- `examples/docs-examples/spec/1-3-effects-safety/sec_g.reml` を `defer f.close()` に復元。
- `docs/spec/1-3-effects-safety.md` の G 節コードブロックを正準例へ復元。
- `reml_frontend` を再ビルドし、diagnostics JSON を再生成。
  - `reports/spec-audit/ch1/1-3-effects-safety__sec_g-20251224-diagnostics.json` を更新（diagnostics 0 件）。

## MIR 型表記の確認サンプル（2025-12-24）
- `reports/spec-audit/ch1/mir-json-type-sample-20251224.json` を追加し、`&` / `&mut` / `[T]` の表記をそのまま MIR JSON に載せる形を固定。
- Backend 側は `parse_reml_type` が `&` / `&mut` / `[T]` を再帰的に解析するため、サンプルの文字列表記が維持されていれば `RemlType::Ref` / `RemlType::Slice` に展開できる。
- `reports/spec-audit/ch1/mir-input-type-sample-20251224.reml` から `reports/spec-audit/ch1/mir-output-type-sample-20251224.json` を生成し、`&` / `&mut` / `[T]` が `ty`/`return_type` に保持されることを確認。
- MIR 出力では `Int`/`Unit` が混在していたため、`compiler/rust/frontend/src/semantics/mir.rs` で `i64`/`()` へ正規化する変換を追加。
- `Str`/`Bytes` は 3-3 の型であり所有権表現と衝突するため、現時点の MIR では表記を維持する方針とする（`String` への寄せは行わない）。
