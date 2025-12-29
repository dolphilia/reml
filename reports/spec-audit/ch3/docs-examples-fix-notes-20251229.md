# docs-examples 修正メモ（ch3 / 2025-12-29）

## 対象
- `docs/spec/3-11-core-test.md`
- `examples/docs-examples/spec/3-11-core-test/*.reml`（sec_2 / sec_2_1 / sec_4 / sec_5 / sec_7_1 / sec_7_2 / sec_7_3）

## 修正内容
- `test` ブロックと `test_parser` 呼び出しを `fn main` でラップし、トップレベル式診断を回避。
- `TestError` / `Bytes` / `Parser` の不足参照をサンプル内の型宣言で補完。
- `Core.Test.Dsl` の最小構文/Matcher 例をケース配列形式へ揃え、`AstMatcher` を明示化。
- `test_parser { case ... }` ブロック構文が未実装のため、仕様側に注記を追加し、実装ギャップ計画を作成。

## 検証
- `compiler/rust/frontend/target/debug/reml_frontend --output json` を各サンプルに対して実行し、診断 0 件を確認。

## 関連リンク
- `docs/plans/docs-examples-audit/1-2-spec-sample-fix-targets.md`
- `docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251229-1.md`
