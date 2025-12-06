# Core.Collections Ref capability (2025-12-06)

- Run ID (collector suite): `ref_internal_mutation` / script hash recorded in `reports/spec-audit/ch3/collections_ref-20251206.json`
- コマンド:
  ```
  python3 tooling/ci/collect-iterator-audit-metrics.py \
    --suite collectors \
    --scenario ref_internal_mutation \
    --output reports/spec-audit/ch3/collections_ref-20251206.json \
    --require-success \
    --require-cell
  ```
- 結果: `collector.effect.cell_rc` シナリオは `pass_rate=1.0`、`cell_mutations_total=1`、`rc_ops_total=2` を記録。`collector.effect.rc`/`collector.effect.mut` のメタデータが `reports/spec-audit/ch1/core_iter_collectors.json` へ伝搬していることを確認。
- 付記: 既知の `collector.table.csv_import` 閾値が未実装のため、同コマンドは `threshold metric missing: collector.table.csv_import` で終了コード 1 を返す。`ref_internal_mutation` KPI への影響はなく、生成された JSON は `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md#3.2.3` の KPI 更新に使用する。
