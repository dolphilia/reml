# docs-examples 修正メモ（ch3 / 2025-12-29）

## 対象
- `docs/spec/3-11-core-test.md`
- `examples/docs-examples/spec/3-11-core-test/*.reml`（sec_2 / sec_2_1 / sec_4 / sec_5 / sec_7_1 / sec_7_2 / sec_7_3）

## 修正内容
- `test` ブロックと `test_parser` 呼び出しを `fn main` でラップし、トップレベル式診断を回避。
- `TestError` / `Bytes` / `Parser` の不足参照をサンプル内の型宣言で補完。
- `Core.Test.Dsl` の最小構文/Matcher 例をケース配列形式へ揃え、`AstMatcher` を明示化。
- `test_parser(parser) { case ... }` のブロック構文を 7.1/7.3 の正準例へ復元し、仕様とサンプルを同期。

## 検証
- `compiler/rust/frontend/target/debug/reml_frontend --output json` を各サンプルに対して実行し、診断 0 件を確認。
- `compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics` で `sec_7_1` / `sec_7_3` を再検証し、診断 0 件を確認。

## 関連リンク
- `docs/plans/docs-examples-audit/1-2-spec-sample-fix-targets.md`
- `docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251229-1.md`

## 追加対象（Core.Dsl.Object / 3.2）
- `docs/spec/3-16-core-dsl-paradigm-kits.md`
- `examples/docs-examples/spec/3-16-core-dsl-paradigm-kits/sec_3_2.reml`

## 追加修正内容（Core.Dsl.Object / 3.2）
- 最小 API の関数宣言を `Object.call` / `Object.lookup` / `Object.class_builder` / `Object.prototype_builder` 形式へ復元。

## 追加検証（Core.Dsl.Object / 3.2）
- `compiler/rust/frontend/target/debug/reml_frontend --output json examples/docs-examples/spec/3-16-core-dsl-paradigm-kits/sec_3_2.reml` を実行し、診断 0 件を確認。

## 追加対象（Core.Collections / 3.2）
- `docs/spec/3-2-core-collections.md`
- `examples/docs-examples/spec/3-2-core-collections/*.reml`（sec_2_1 / sec_2_2 / sec_3_1 / sec_3_2 / sec_3_3 / sec_7 / sec_8）

## 追加修正内容（Core.Collections / 3.2）
- `Box` / `PersistentMap` / `PersistentSet` / `Borrow` / `BorrowMut` / `Path` / `Record` / `Table` の不足型宣言を補完。
- 仕様書のコードブロックをサンプル差分と同期。

## 追加検証（Core.Collections / 3.2）
- `for f in examples/docs-examples/spec/3-2-core-collections/*.reml; do cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json "$f"; done` を実行し、診断 0 件を確認。
