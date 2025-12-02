# Core Runtime Capability ダッシュボード

Phase 3 の Runtime Stage 監査を `collect-iterator-audit-metrics.py --section runtime --require-success` で収集した結果。検証ソースは `reports/runtime-capabilities-validation.json`（`scripts/validate-runtime-capabilities.sh tooling/runtime/capabilities/default.json` の出力）を使用した。

## runtime.capability_validation

| 指標 | 値 | 補足 |
|------|-----|------|
| pass_rate | 1.0 | 1 件の検証 JSON すべてが `validation.status = ok`。 |
| runtime_candidate_total | 3 | `default`/`x86_64-pc-windows-msvc`/`arm64-apple-darwin` の 3 ターゲットを確認。すべて Stage 情報が埋まっている。 |
| stage_trace_entries | 6 | CLI/Env/JSON/Runtime の判定履歴が `stage_trace` に 6 行で保存されている。 |
| コマンド | `python3 tooling/ci/collect-iterator-audit-metrics.py --section runtime --require-success --runtime-source reports/runtime-capabilities-validation.json --output reports/audit/dashboard/core_runtime-20251202.json` | `reports/audit/dashboard/core_runtime-20251202.json` は CI アーカイブ用の JSON 出力。 |

### 主要候補の Stage

- `default`: `stable`
- `x86_64-pc-windows-msvc`: `beta`
- `arm64-apple-darwin`: `beta`

### 次のアクション

1. Stage override を追加／更新した場合は `scripts/validate-runtime-capabilities.sh` を再実行し、同コマンドで `pass_rate = 1.0` を再確認する。
2. `runtime_candidate_total` が変動した場合は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md#0.3.7` に追記し、本ダッシュボードを更新する。
3. 失敗が発生した場合は `reports/runtime-capabilities-validation.json` を添付し、`docs/plans/bootstrap-roadmap/0-4-risk-handling.md#runtime-stage-validation` を再オープンする。
