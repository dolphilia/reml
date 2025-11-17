# effect_handler dual-write 比較 (2025-11-18)

| 観測項目 | OCaml | Rust | 備考 |
| --- | --- | --- | --- |
| 診断件数 | 0 | 0 | `effects.resume.untyped` はいずれも発生せず。 |
| audit.fingerprint | `ocaml-cli/20251118T0112Z` | `rust-poc/20251118T0112Z` | `CollectIteratorAudit` ID を共通に設定。 |
| TraceEvent | `ExprEnter:handle`, `ExprLeave:handle` | 同上 + `ExprEnter:operation` | Rust 版は operation アーム毎に `TraceEvent::ExprEnter` を出力。 |
| RunConfig.extensions.effects | `{ "type_row_mode": "ty-integrated" }` | 同一 | `scripts/poc_dualwrite_compare.sh effect_handler` で CLI フラグを同期。 |
| JSON diff | 空 | 空 | `jq --sort-keys` 差分ゼロ (`diff-harness` 保存先: `reports/dual-write/front-end/w37-effect-handler/`). |

- 実行コマンド: `scripts/poc_dualwrite_compare.sh effect_handler --ci-run rust-frontend-w37-20251118.2`
- CI ログ: `reports/spec-audit/ch1/effect_handler-20251118-diagnostics.json` に `ci_run_id` を記録。
- 参照資料: `docs/plans/rust-migration/1-2-diagnostic-compatibility.md#effect-handler-acceptance`（今回追記）。
