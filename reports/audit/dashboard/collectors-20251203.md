# Core Collections Collectors ダッシュボード

`collector.effect.cell` / `collector.effect.rc` を Phase 3 gate に昇格させるため、以下の 2 コマンドを 2025-12-03(JST) に実行した。JSON 出力は `reports/audit/dashboard/collectors-20251203.json` に保存している。

1. `python3 tooling/ci/collect-iterator-audit-metrics.py --suite collectors --scenario ref_internal_mutation --require-success --require-cell --output reports/audit/dashboard/collectors-20251203.json`
2. `scripts/validate-diagnostic-json.sh --suite collectors`

## collect-iterator-audit-metrics 実行結果

| 指標 | 値 | 備考 |
| ---- | --- | ---- |
| enforcement.failures | `["threshold metric missing: collector.effect.cell_rc (ref_internal_mutation)", "threshold metric missing: collector.table.csv_import (table_csv_import)", "diagnostic.audit_presence_rate < 1.0"]` | `collector.effect.cell_rc` と `collector.table.csv_import` 用の snapshot が存在しないため `metric_missing`。さらに `diagnostic.audit_presence_rate` は `0.0` で、`cli.audit_id`/`cli.change_set`/`schema.version`/`timestamp` がいずれの診断にも付与されていない。 |
| collector.effect.audit_snapshot.pass_rate | `1.0` | Stage/Effect/Audit 自体は 7 件で計測できている。 |
| diagnostic.audit_presence_rate.pass_rate | `0.0` | `reports/spec-audit/ch1/core_iter_collectors.json` の全エントリから必須キーが欠落。 |

### 追加観測

- `core_iter_collectors.json` に `collect_cell_ref_effects` ケースが無いため、`--require-cell` で `collector.effect.cell_rc` メトリクスが `metric_missing` となり gate が失敗する。
- `thresholds.failures[]` に `collector.table.csv_import` も記録され、`table_csv_import` snapshot が生成されるまで CI gate が通らない状態である。

## scripts/validate-diagnostic-json 実行結果

- `core_iter_collectors.json` の 7 エントリ全てに対して `primary`・`audit_metadata`・`timestamp` が無いことが検出され、スキーマ違反で終了した。
- `audit.metadata.effect.required_capabilities` / `effect.actual_capabilities` も欠落しており、`collector.effect.*` を監査へ反映できていない。
- 監査ファイル (`core_iter_collectors.audit.jsonl`) は検証対象に含まれたが、上記スキーマ違反の時点で処理が止まったため成功扱いにはなっていない。

## アクションアイテム

1. `compiler/rust/frontend/tests/__snapshots__/core_iter_collectors.snap` に `collect_cell_ref_effects` / `table_csv_import` のケースを追加し、`reports/spec-audit/ch1/core_iter_collectors{.json,.audit.jsonl}` を再生成する。
2. collector snapshot でも `Diagnostic.primary`・`timestamp`・`audit_metadata` をセットできるよう `render-collector-audit-fixtures.py` と `CollectorAuditTrail` を修正する。
3. `collect-iterator-audit-metrics.py` の `--require-cell` / `--require-success` が通った Run ID を取得後、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` と `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md` に KPI 更新を反映する。
