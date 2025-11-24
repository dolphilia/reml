# TODO: Collections Audit Bridge

## 概要
- `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md` §5.1.2 の手順に従い、`Core.Collections` の差分（`collections.diff.*`）を `AuditEnvelope` に載せる架橋を整備する。
- 監査メタデータを保持する `CollectOutcome`/`EffectLabels` から `AuditEnvelope.change_set` への道筋は、`ConfigMergeOutcome` → `REML_COLLECTIONS_CHANGE_SET_PATH` → `FormatterContext` という 3 段階で構築する必要があるため、それぞれの責任範囲に TODO を分割する。

## 追跡アイテム
1. **ChangeSet の JSON 出力**  
   - `compiler/rust/runtime/src/config/mod.rs` の `ConfigMergeOutcome::change_set_json` を呼び出し、`write_change_set_to_temp_dir` で出力パスを得る。将来的には `MapCollector`/`TableCollector` の `CollectOutcome` から `ChangeSet` を生成するヘルパを追加して、実行時に差分が出るたびに JSON を更新する。
   - 出力された一時ファイルへのパスを `REML_COLLECTIONS_CHANGE_SET_PATH` で共有できるよう、`scripts/poc_dualwrite_compare.sh` や `examples/` のランチャーにラッパーを入れる想定。
   - `set_collections_change_set_env` ヘルパーをこのパス構築と環境変数の注入処理として利用する。CLI 起動中は `CollectionsChangeSetEnv` を保持し、完了後に `Drop` で変数とファイルをクリアすることでランタイムフローを制御する。

2. **CLI 側での取り込みと検証**  
   - `compiler/rust/frontend/src/diagnostic/formatter.rs` の `FormatterContext::change_set` で `load_collections_change_set_from_env` を介して JSON を読み込み、`AuditEnvelope.change_set` の `collections` ブロックに展開する。
   - 同時に `metadata["collections.diff.total"]` などを `Diagnostic.extensions` へコピーし、`scripts/validate-diagnostic-json.sh --pattern collections.diff` で必須キーとして検証する仕組みを追加する。

3. **監査ログと KPI の整合**  
   - `reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` に `collections.diff.map/set/table` ケースを追加して JSON snapshot を充実させ、`reports/iterator-collector-summary.md` の `audit_bridge` 表や `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 表に `collections.audit_bridge_pass_rate`/`collector.effect.audit_presence` を反映する。
   - 同ファイルは `tooling/ci/collect-iterator-audit-metrics.py --scenario map_set_persistent --require-audit` の差分チェック対象に含める。

4. **検証用テスト**  
   - `compiler/rust/runtime/tests/core_collections_audit_bridge.rs` を新設し、`PersistentMap::merge_with_change_set`/`PersistentSet::diff_change_set`/`Table` 操作で `AuditBridge` の `ChangeSet` が期待どおり JSON に変換されることを確認する。  
   - `scripts/poc_dualwrite_compare.sh --target map_diff` に `collections.audit_bridge` ファイル比較機能を組み込み、OCaml 実装との diff をレポートする。  

5. **CapabilityRegistry 連携（5.3.1）**  
   - `CollectOutcome` の `collector.effect.audit` を検出する箇所で `crate::registry::CapabilityRegistry::verify_capability_stage("core.collections.audit", StageRequirement::Exact(StageId::Stable), &["audit","mem"])` を呼び出し、Stage/Effect 要件を満たさない場合は `CollectError::CapabilityDenied` を返すユーティリティを `compiler/rust/runtime/src/prelude/collectors/mod.rs` に追加する。  
   - `CollectOutcome::audit` を実装して `ChangeSet` を `crate::config::set_collections_change_set_env` 経由で `REML_COLLECTIONS_CHANGE_SET_PATH` に書き出し、`FormatterContext` が読み取れるように `CollectionsChangeSetEnv` を握る構造を整備する。  
   - `scripts/validate-diagnostic-json.sh --pattern collector.effect.audit` と `tooling/ci/collect-iterator-audit-metrics.py --scenario audit_cap` で `collector.effect.audit=true` の存在を必須化し、`reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` の `collections` セクションを検証する。  

6. **CollectError::CapabilityDenied のパス（5.3.2）**  
   - `ListCollector`/`MapCollector`/`TableCollector` の `finish` で `CapabilityRegistry::require("core.collections.audit")` を呼び出し、チェック失敗時に `CollectErrorKind::CapabilityDenied` を返す扱いを `CollectError::with_detail` で拡張する。  
   - `Diagnostic.extensions["collector.capability"]` と `collector.effect.audit` を `GuardDiagnostic` に書き込み、`scripts/poc_dualwrite_compare.sh --target audit_bridge` の `collector.capability` フィールドに入る JSON を確保する。  
   - `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` Phase3 Capability 行・`reports/iterator-collector-summary.md` `status` カラム・`reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` の `capability` ケースに `collector.capability`/`collector.effect.audit` の成功と `CollectError::CapabilityDenied` の検出結果を記録するルーチンを `tooling/ci/collect-iterator-audit-metrics.py --scenario audit_cap` へ組み込む。

## 参照
- `docs/spec/3-6-core-diagnostics-audit.md` § `ChangeSet` / `SchemaDiff` 説明
- `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md` §5.1.2
- `scripts/validate-diagnostic-json.sh`（新規 `--pattern collections.diff` ブロックを追加予定）
