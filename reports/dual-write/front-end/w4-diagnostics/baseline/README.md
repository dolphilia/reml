# W4 診断互換試験: OCaml ベースライン

- `npm ci --prefix tooling/lsp/tests/client_compat` および `npm run ci --prefix tooling/lsp/tests/client_compat` を実行し、LSP V2 フィクスチャ 9 件が pass。
- `scripts/validate-diagnostic-json.sh $(cat tmp/w4-parser-diag-paths.txt)` を実行し、`compiler/ocaml/tests/golden/diagnostics/` から抽出した 10 ケースが Schema v2.0.0-draft に適合。
  - `effects/syntax-constructs.json.golden` は診断 JSON ではないため自動除外（2027-11-07 時点で validator 側にフィルタを実装し、TODO DIAG-RUST-03 を再発防止に切り替え）。
- `collect-iterator-audit-metrics.py` を parser / effects / streaming セクションで実行し、結果を `parser-metrics.ocaml.json` / `effects-metrics.ocaml.json` / `streaming-metrics.ocaml.json` に保存。ログは `collect-iterator-audit-metrics.log`。
  - Parser/Streaming で `diagnostic.audit_presence_rate` の pass_fraction=1.0 (`domain/multi-domain.json.golden` の audit メタデータを補完済み)。
