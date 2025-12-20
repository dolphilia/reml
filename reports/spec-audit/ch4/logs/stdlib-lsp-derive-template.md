# Phase 4 stdlib Core.Lsp.Derive ログ（テンプレート）

- 生成時刻: YYYY-MM-DDTHH:MM:SSZ
- 対象: CH3-LSP-402

## 実行詳細

### CH3-LSP-402

- ファイル: `examples/practical/core_lsp/auto_derive_basic.reml`
- 期待出力: `expected/practical/core_lsp/auto_derive_basic.stdout`
- 実際出力: `expected/practical/core_lsp/auto_derive_basic.stdout` と一致
- CLI: `compiler/rust/frontend/target/debug/reml_frontend --parse-driver --output lsp-derive examples/practical/core_lsp/auto_derive_basic.reml`
- run_id: `TODO`
- 備考: `DeriveModel` が空であるため `capabilities` は全て `false` になる。
