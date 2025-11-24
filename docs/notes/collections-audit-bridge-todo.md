# TODO: Collections Audit Bridge

## 概要
- `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md` §5.1.2 の手順に従い、`Core.Collections` の差分（`collections.diff.*`）を `AuditEnvelope` に載せる架橋を整備する。
- 監査メタデータを保持する `CollectOutcome`/`EffectLabels` から `AuditEnvelope.change_set` への道筋は、`ConfigMergeOutcome` → `REML_COLLECTIONS_CHANGE_SET_PATH` → `FormatterContext` という 3 段階で構築する必要があるため、それぞれの責任範囲に TODO を分割する。

## 追跡アイテム
1. **ChangeSet の JSON 出力**  
   - `compiler/rust/runtime/src/config/mod.rs` の `ConfigMergeOutcome::change_set_json` を呼び出し、`write_change_set_to_temp_dir` で出力パスを得る。将来的には `MapCollector`/`TableCollector` の `CollectOutcome` から `ChangeSet` を生成するヘルパを追加して、実行時に差分が出るたびに JSON を更新する。
   - 出力された一時ファイルへのパスを `REML_COLLECTIONS_CHANGE_SET_PATH` で共有できるよう、`scripts/poc_dualwrite_compare.sh` や `examples/` のランチャーにラッパーを入れる想定。

2. **CLI 側での取り込みと検証**  
   - `compiler/rust/frontend/src/diagnostic/formatter.rs` の `FormatterContext::change_set` で `load_collections_change_set_from_env` を介して JSON を読み込み、`AuditEnvelope.change_set` の `collections` ブロックに展開する。
   - 同時に `metadata["collections.diff.total"]` などを `Diagnostic.extensions` へコピーし、`scripts/validate-diagnostic-json.sh --pattern collections.diff` で必須キーとして検証する仕組みを追加する。

3. **監査ログと KPI の整合**  
   - `reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` に `collections.diff.map/set/table` ケースを追加して JSON snapshot を充実させ、`reports/iterator-collector-summary.md` の `audit_bridge` 表や `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 表に `collections.audit_bridge_pass_rate`/`collector.effect.audit_presence` を反映する。
   - 同ファイルは `tooling/ci/collect-iterator-audit-metrics.py --scenario map_set_persistent --require-audit` の差分チェック対象に含める。

4. **検証用テスト**  
   - `compiler/rust/runtime/tests/core_collections_audit_bridge.rs` を新設し、`PersistentMap::merge_with_change_set`/`PersistentSet::diff_change_set`/`Table` 操作で `AuditBridge` の `ChangeSet` が期待どおり JSON に変換されることを確認する。
   - `scripts/poc_dualwrite_compare.sh --target map_diff` に `collections.audit_bridge` ファイル比較機能を組み込み、OCaml 実装との diff をレポートする。

## 参照
- `docs/spec/3-6-core-diagnostics-audit.md` § `ChangeSet` / `SchemaDiff` 説明
- `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md` §5.1.2
- `scripts/validate-diagnostic-json.sh`（新規 `--pattern collections.diff` ブロックを追加予定）
