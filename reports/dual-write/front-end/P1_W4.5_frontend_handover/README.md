# P1 W4.5 Front-end Handover Bundle

W4.5 のクロージングレビューで確定した dual-write 成果物を集約したディレクトリ。Parser Recover は Pass、Streaming / Type & Effect / CLI・LSP は Pending のまま Phase P2 へ引き渡す。

## ディレクトリ構成

- `ast-ir/` — `w3-type-inference/2027-01-15-w3-typeck/{typed-ast,constraints,impl-registry}.{ocaml,rust}.json`
- `diag/recover/20280210-w4-diag-recover-else-r4/`
- `diag/streaming/20280410-w4-diag-streaming-r21/`
- `diag/effects/20280418-w4-diag-effects-r3/`
- `diag/effects/20280601-w4-diag-type-effect-rust-typeck-r7/`
- `diag/cli-lsp/20280430-w4-diag-cli-lsp/`

各ケース内には `summary.{md,json}`、`diagnostics.{ocaml,rust}.json`、`audit_metadata.*`、`parser-metrics.*`、`effects-metrics.*`、`expected_tokens.*`、`typeck-debug.*` を保存する想定。現状は空ディレクトリのため、Run 再実行後に成果物をコピーする。

## リンク

- `docs/plans/rust-migration/1-0-front-end-transition.md#w4.5-p1-クロージングレビューp2-ハンドオーバー準備`
- `docs/plans/rust-migration/1-1-ast-and-ir-alignment.md#1-1-11-p2-連携メモw4.5`
- `docs/plans/rust-migration/1-2-diagnostic-compatibility.md#1-2-22-w4.5-診断クロージングメモ`
- `docs/plans/rust-migration/1-3-dual-write-runbook.md#1.3.6-w4.5-引き継ぎパッケージ作成手順`
