# docs/spec/2-1-parser-type.md サンプル修正メモ（2025-12-24）

## 対象
- `examples/docs-examples/spec/2-1-parser-type/sec_a.reml`
- `examples/docs-examples/spec/2-1-parser-type/sec_c.reml`
- `examples/docs-examples/spec/2-1-parser-type/sec_d.reml`
- `examples/docs-examples/spec/2-1-parser-type/sec_d_1.reml`
- `examples/docs-examples/spec/2-1-parser-type/sec_clilsp.reml`
- `examples/docs-examples/spec/2-1-parser-type/sec_g.reml`

## 変更概要
- 仕様コードブロックとサンプルを同期し、Rust Frontend が受理する構文へ簡略化。
- 実装未対応の構文は暫定的にフォールバック表記へ置換。

## 主な修正点
- `Reply<T>` のバリアントをラベル付き引数から位置引数へ変更。
- `SpanTrace` を `List<(String, Span)>` に簡略化。
- `RunConfig` のデフォルト値と文字列リテラル型を削除し、許容値はコメントへ退避。
- `RunConfig.with_extension` を `= todo` 付きスタブに変更。
- CLI/LSP 共有設定サンプルを `fn configure` に移動し、ラムダと `DemandHint` の組み立てを分解。
- `G` 節コードブロックの終端を明示し、`.reml` 抽出範囲を正規化。

## 実装ギャップ
- 追加計画: `docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251224-3.md`
