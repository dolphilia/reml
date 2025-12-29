# docs-examples 修正メモ（ch3 / 2025-12-30）

## 対象
- `docs/spec/3-12-core-cli.md`
- `examples/docs-examples/spec/3-12-core-cli/sec_5.reml`

## 修正内容
- `match` を `match ... with` 形式へ修正し、アームで複数式を扱うためにブロック `{ ... }` を導入。
- `if` を `if ... then ... else ...` 形式へ統一。

## 検証
- `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/docs-examples/spec/3-12-core-cli/sec_2.reml`
- `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/docs-examples/spec/3-12-core-cli/sec_5.reml`

## 関連リンク
- `docs/plans/docs-examples-audit/1-2-spec-sample-fix-targets.md`
