# 3.2 Core Collections 実装フォローアップ計画

## 背景と今回の発見
- 永続集合系は fingerprints tree／赤黒木の基盤が `compiler/rust/runtime/src/collections/persistent/` に整備されており、`ListCollector`/`MapCollector`/`SetCollector` が `CollectorAuditTrail::record_change_set` で `collector.effect.audit` を立てる運用 (`compiler/rust/runtime/src/prelude/collectors/mod.rs:210-405`) を含めて監査出力が収益化されている。`reports/iterator-collector-summary.md:1-90` の KPI もこの経路を前提としており、snapshot テスト (`compiler/rust/frontend/tests/core_iter_collectors.rs:53-175`) で `effects.mem_bytes` や `collector.effect.*` を固定化している。
- 可変コレクション（`CoreVec`/`EffectfulVec`、`EffectfulCell`、`EffectfulRef`、`EffectfulTable`）は `EffectSet` の `mut`/`mem`/`cell`/`rc` ビットを記録する実装になっているものの、仕様に記載された helper API (`Vec.collect_from` が `Result` を返す、`Cell`/`Ref` 公開 API 名、`Table.load_csv` と `effect {io}`) とドキュメント（`docs/spec/3-2-core-collections.md:25-139`）との整合が完全ではない。
- API 名義・エラー挙動・IO の差分ブリッジを Plan `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md:1-400` で追跡しているが、現状 Rust 実装では `Map.from_pairs`/`Set.partition`/`Map.update`/`Table.load_csv`/`Vec.collect_from` などの interface が欠落しており、`MapCollector::push` ですら `CollectErrorKind::DuplicateKey` しか投げないことから仕様にある `Map.update` の `f` 処理や `Table.collect_table` の `CollectError::UnstableOrder` などは別途実装が必要である。

## 実装ステップ
以下を優先的な作業パックとして進め、実装→テスト→ドキュメントの順で差分を維持する。

1. **永続 API 名と変換ヘルパの完成**
   - `List.as_vec` 用のエイリアスメソッドを実装し、`ListCollector::finish` が記録する `collector.effect.mem_bytes` と寄せて `List` → `Vec` コピーコストを明示した。
   - `PersistentMap` に `keys()` を追加してキー一覧を取得できるようにし、`PersistentSet` に `partition()` を設けて predicate ごとに 2 つの集合へ分割できるようにした（`compiler/rust/runtime/src/collections/persistent/btree.rs`）。
   - これら変更を `docs/spec/3-2-core-collections.md:48-80` のバッチヘルパ一覧と合わせて再確認し、`examples/core-collections/usage.reml` の手順と `reports/iterator-collector-summary.md` の KPI 記述を突き合わせておく。必要であれば `scripts/validate-diagnostic-json.sh` への `collector.effect.*` チェック追加も併記。

2. **Mutable 系 API の effect 完結**
   - `Vec.collect_from` と `Iter.collect_vec` を共通 `Result<CoreVec<T>, CollectError>` 経路に統一し、`CollectError::OutOfMemory` が `map_try_reserve_error` (`compiler/rust/runtime/src/collections/mutable/vec/error.rs:1-17`) から伝搬されるようにする。`docs/spec/3-2-core-collections.md:104-117` の `CollectError` 仕様に対応し、`reports/iterator-collector-summary.md` の `collect_vec_mem_reservation` KPI を更新。
   - `Cell`/`Ref` API の名前と効果タグ (`effect {cell}`/`effect {rc}`) をドキュメントに揃える。`EffectfulCell::set` で `mark_cell()`/`mark_mut()`、`EffectfulRef::borrow_mut` で `mark_rc()`/`mark_mut()` を保証し、`CollectorEffectMarkers::cell_mutations` を `CollectorAuditTrail` に出力する (`compiler/rust/runtime/src/prelude/collectors/mod.rs:188-405`)。
   - `EffectfulTable` の `insert`/`remove`/`to_map` に `EffectSet` を反映し、`TableCollector` の `push` で `CollectError::DuplicateKey` に加えて `CollectError::UnstableOrder` を `Diagnostic::collector_unstable_order` として返せるようにする。

3. **CSV/IO 連携と Capability ブリッジ**
   - `Table.load_csv(path)` を `Core.IO.CsvReader` + `Core.Text` で実装し、`EffectSet::mark_io()` と `mark_mut()` を同時に呼ぶ。`docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` の CSV リーダー設計を参照し、`docs/spec/3-2-core-collections.md:129-146` の `effect {io}` 記述に一致させる。
   - `core.collections.audit` Capability を `CapabilityRegistry` に登録し、`CollectOutcome::audit` (`compiler/rust/runtime/src/prelude/collectors/mod.rs:397`) で `set_collections_change_set_env` を通じて `REML_COLLECTIONS_CHANGE_SET_PATH` をセット。`docs/notes/collections-audit-bridge-todo.md` と `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md:332-377` の通り `collectors::ensure_core_collections_audit` を利用し、`CollectError::CapabilityDenied` の `Diagnostic.extensions` に `collector.capability` を反映。
   - `scripts/validate-diagnostic-json.sh` と `tooling/ci/collect-iterator-audit-metrics.py --scenario audit_cap` で `collector.effect.audit`/`collections.diff.total`/`collector.capability` を gate 化し、`reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` の `audit_cap` ケースへ KPI を追加 (`reports/iterator-collector-summary.md:90-260`)。

4. **ドキュメントとサンプルの同期**
   - `docs/spec/3-2-core-collections.md` §2.3 に `examples/core-collections/usage.reml` への NOTE を追記し、`List → Map → Vec → Table → Cell/Ref` のパイプラインと `CollectError`・`effect` 発火点を解説 (`docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md:389-398`)。
   - `examples/core-collections/README.md` で手動実行手順と KPI シナリオ（`collect-iterator-audit-metrics.py --scenario core_collections_example`）を整備し、`README.md` と `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` に Core.Collections の進捗ブロックを追加。
   - `docs/plans/bootstrap-roadmap/assets/metrics/core_collections_persistent.csv` / `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 表に `vec_mut_ops_per_sec`/`list_as_vec_mem_bytes`/`cell_mutations_total`/`ref_borrow_conflict_rate` を追記し、`reports/iterator-collector-metrics.json` とのリンクを明示。

## 検証と CI 統合
- `cargo test core_collections_vec core_collections_cell_ref core_collections_table` を Phase3 CI に組み込み、`CollectError` や `collector.effect.*` 出力を `scripts/validate-diagnostic-json.sh --suite collectors` で gate。
- `tooling/ci/collect-iterator-audit-metrics.py --section collectors --scenario vec_mem_exhaustion|cell_internal_mutation|table_csv_import|audit_cap` を自動化し、`reports/spec-audit/ch1/core_iter_collectors.json`/`.audit.jsonl` を KPI source に指定。`reports/iterator-collector-summary.md` の `status` カラムと `reports/spec-audit/diffs/README.md` の dual-write 表にレポート。
