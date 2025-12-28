# docs/spec/2-1-parser-type.md サンプル修正メモ（2025-12-24）

## 対象
- `examples/docs-examples/spec/2-1-parser-type/sec_a.reml`
- `examples/docs-examples/spec/2-1-parser-type/sec_c.reml`
- `examples/docs-examples/spec/2-1-parser-type/sec_d.reml`
- `examples/docs-examples/spec/2-1-parser-type/sec_d_1.reml`
- `examples/docs-examples/spec/2-1-parser-type/sec_clilsp.reml`
- `examples/docs-examples/spec/2-1-parser-type/sec_g.reml`

## 変更概要
- フェーズ 3 の復元方針に従い、仕様コードブロックとサンプルを正準表記へ戻した。
- Rust Frontend 側の構文受理拡張に合わせ、従来のフォールバック表記を解消した。

## 主な修正点
- `Reply<T>` のバリアントをラベル付き引数へ復元。
- `SpanTrace` を `List<(name: String, span: Span)>` に復元。
- `RunConfig` にデフォルト値を復元し、`left_recursion` を文字列リテラル和型へ戻した。
- CLI/LSP 共有設定サンプルで `Any::from(DemandHint{...})` をインライン復元。

## 実装ギャップ
- 追加計画: `docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251224-3.md`

## 再検証
- `for file in examples/docs-examples/spec/2-1-parser-type/sec_*.reml; do compiler/rust/frontend/target/debug/reml_frontend "$file"; done`
- diagnostics 0 件（`sec_a`/`sec_c`/`sec_d`/`sec_clilsp` を含む全 `sec_*`）

# docs/spec/2-4-op-builder.md サンプル復元メモ（2025-12-26）

## 対象
- `examples/docs-examples/spec/2-4-op-builder/sec_a_2.reml`
- `examples/docs-examples/spec/2-4-op-builder/sec_a_3.reml`
- `examples/docs-examples/spec/2-4-op-builder/sec_b.reml`
- `examples/docs-examples/spec/2-4-op-builder/sec_i_1.reml`

## 変更概要
- フェーズ 3 の復元方針に従い、フォールバック表記を正準例へ戻した。
- `Type.method` 形式の宣言、`let sym(s)`、`rec expr`、`|a| -a`、`(c, t, f) -> ...` を復元した。

## 再検証
- `for file in examples/docs-examples/spec/2-4-op-builder/*.reml; do compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics "$file" > reports/spec-audit/ch2/2-4-op-builder__$(basename "$file" .reml)-20251223-diagnostics.json; done`
- diagnostics 0 件（`sec_a_1` / `sec_a_2` / `sec_a_3` / `sec_b` / `sec_b_1` / `sec_e` / `sec_i_1` / `sec_i_2` / `sec_i_3`）
