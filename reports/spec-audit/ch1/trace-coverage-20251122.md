# Trace Coverage (Rust Frontend) - 2025-11-22

- コマンド: `scripts/poc_dualwrite_compare.sh effect_handler --trace --run-id rust-frontend-w39-20251122.1`
- CI_RUN_ID: `rust-frontend-w39-20251122.1`
- `git rev-parse HEAD`: `f9e10ae676bca22ed8a41e96d79f667310274990`
- RunConfig: `trace=true`, `packrat=true`, Streaming Flow `chunk=4096`, `merge_warnings=true`
- 参照ファイル:
  - `reports/spec-audit/ch1/effect_handler-20251118-diagnostics.json`
  - `reports/spec-audit/ch1/effect_handler-20251118-trace.md`
  - `reports/spec-audit/ch1/block_scope-20251118-trace.md`

## Trace coverage >= 4 の確認

| サンプル | 対応する `trace_ids` | 備考 |
| --- | --- | --- |
| `effect_handler.reml` | `syntax:expr::handle`, `syntax:expr::perform`, `syntax:effect::decl`, `syntax:operation::resume` | `FrontendDiagnostic.extensions.trace_ids` に 4 本すべてが出現し、`effect_handler-20251118-trace.md` のイベント（Enter/Leave＋EffectEnter/Exit）が一致した。 |
| `block_scope.reml` | `syntax:expr::block`, `syntax:expr::let`, `syntax:expr::var` | ブロック内の let/var でも `ExprEnter/ExprLeave` が発火することを確認。 |

- `trace_ids` は診断 JSON（`diagnostics[*].extensions.trace_ids[]`）へ 1 診断あたり最大 4 個まで記録され、`reports/spec-audit/diffs/SYNTAX-003-ch1-rust-gap.md` から本ログにリンク済み。
- `Trace coverage >= 4` の閾値は handle / perform / resume / block を必須とし、`docs/notes/spec-integrity-audit-checklist.md#rust-gap-トラッキング表` (`SYNTAX-003`) に証跡を添付した。
