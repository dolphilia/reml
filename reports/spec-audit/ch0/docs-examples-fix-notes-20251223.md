# docs-examples 修正メモ (ch0 / 2025-12-23)

## 代表診断
- `parser.syntax.expected_tokens`: トップレベル式のため構文エラー。

## 修正対象
- `examples/docs-examples/spec/0-3-code-style-guide/sec_3_4.reml`: パイプ例を関数ブロックに移動。
- `examples/docs-examples/spec/0-3-code-style-guide/sec_3_5.reml`: 制御構文例を関数ブロックに移動。

## 仕様整合メモ
- `docs/spec/0-3-code-style-guide.md` の 3.4 / 3.5 コードブロックも同様に更新。
- 再検証は `compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics <sample>` を再実行。
