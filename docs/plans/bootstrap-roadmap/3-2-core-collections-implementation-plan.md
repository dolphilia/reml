# 3.2 Core Collections 実装フォローアップ計画

## 背景と今回の発見
- 永続集合系は fingerprints tree／赤黒木の基盤が `compiler/runtime/src/collections/persistent/` に整備されており、`ListCollector`/`MapCollector`/`SetCollector` が `CollectorAuditTrail::record_change_set` で `collector.effect.audit` を立てる運用 (`compiler/runtime/src/prelude/collectors/mod.rs:210-405`) を含めて監査出力が収益化されている。`reports/iterator-collector-summary.md:1-90` の KPI もこの経路を前提としており、snapshot テスト (`compiler/frontend/tests/core_iter_collectors.rs:53-175`) で `effects.mem_bytes` や `collector.effect.*` を固定化している。
- 可変コレクション（`CoreVec`/`EffectfulVec`、`EffectfulCell`、`EffectfulRef`、`EffectfulTable`）は `EffectSet` の `mut`/`mem`/`cell`/`rc` ビットを記録する実装になっているものの、仕様に記載された helper API (`Vec.collect_from` が `Result` を返す、`Cell`/`Ref` 公開 API 名、`Table.load_csv` と `effect {io}`) とドキュメント（`docs/spec/3-2-core-collections.md:25-139`）との整合が完全ではない。
- API 名義・エラー挙動・IO の差分ブリッジを Plan `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md:1-400` で追跡しているが、現状 Rust 実装では `Map.from_pairs`/`Set.partition`/`Map.update`/`Table.load_csv`/`Vec.collect_from` などの interface が欠落しており、`MapCollector::push` ですら `CollectErrorKind::DuplicateKey` しか投げないことから仕様にある `Map.update` の `f` 処理や `Table.collect_table` の `CollectError::UnstableOrder` などは別途実装が必要である。

## 実装ステップ
以下を優先的な作業パックとして進め、実装→テスト→ドキュメントの順で差分を維持する。

1. **永続 API 名と変換ヘルパの完成**
   - `List.as_vec` 用のエイリアスメソッドを実装し、`ListCollector::finish` が記録する `collector.effect.mem_bytes` と寄せて `List` → `Vec` コピーコストを明示した。
   - `PersistentMap` に `keys()` を追加してキー一覧を取得できるようにし、`PersistentSet` に `partition()` を設けて predicate ごとに 2 つの集合へ分割できるようにした（`compiler/runtime/src/collections/persistent/btree.rs`）。
   - これら変更を `docs/spec/3-2-core-collections.md:48-80` のバッチヘルパ一覧と合わせて再確認し、`examples/core-collections/usage.reml` の手順と `reports/iterator-collector-summary.md` の KPI 記述を突き合わせておく。必要であれば `scripts/validate-diagnostic-json.sh` への `collector.effect.*` チェック追加も併記。

2. **Mutable 系 API の effect 完結**
   - `Vec.collect_from`（`CoreVec::collect_from`/`EffectfulVec::collect_from`）を `VecCollector` 経路で `Result<CoreVec<T>, CollectError>` にし、`CollectError::OutOfMemory` を `map_try_reserve_error` (`compiler/runtime/src/collections/mutable/vec/error.rs:1-17`) から流し込む。`docs/spec/3-2-core-collections.md:104-117` の `CollectError` 仕様に照らして `collect_vec_mem_reservation` KPI との整合を確認し、`FromIterator` 実装では失敗時に panic する形で従来の `collect()` を再現する。
   - `Cell`/`Ref` API の名前と効果タグ (`effect {cell}`/`effect {rc}`) をドキュメントに揃える。`EffectfulCell::set` で `mark_cell()`/`mark_mut()`、`EffectfulRef::borrow_mut` で `mark_rc()`/`mark_mut()` を保証し、`CollectorEffectMarkers::cell_mutations` を `CollectorAuditTrail` に出力する (`compiler/runtime/src/prelude/collectors/mod.rs:188-405`)。
   - `EffectfulTable` の `insert`/`remove`/`to_map` に `EffectSet` を反映し、`TableCollector` の `push` で `CollectError::DuplicateKey` に加えて `CollectError::UnstableOrder` を `Diagnostic::collector_unstable_order` として返せるようにする。

3. **CSV/IO 連携と Capability ブリッジ**
   - `EffectSet` に `io` ビットを追加し、`EffectLabels`/`CollectorAuditTrail` の extension/metadata 出力を `collector.effect.io` で拡張。`EffectfulTable::record_io` を `Table.load_csv` で呼び出すことで `effect {io}` を `collector.effect.*` 経路へ伝搬する実装が完了した（`compiler/runtime/src/prelude/iter/mod.rs:741-980`）。
   - `Table::load_csv` を `File`/`BufReader` ベースで追加し、空行はスキップ・カンマ 1 つ目をキー 2 つ目を値として `EffectfulTable::insert` に流し `effect {mut}`/`effect {io}` を同時に記録。`records` は `EffectfulTable::record_io` で `collector.effect.io` を立てたまま `tables` に変換され、KPI 目的の `collect_table_csv` シナリオ向けエビデンスを補完 (`compiler/runtime/src/collections/mutable/table.rs:108-395`)。
   - `core.collections.table.csv_load` Capability を `register_table_csv_capability()` で `CapabilityRegistry` に安定登録し、`Table::load_csv` の呼び出し時に `register_table_csv_capability` を起動して `Stage=Stable`/`effect_scope={"io","mut","mem"}` を保証。`handles/table_csv.rs`/`handles/mod.rs`/`lib.rs` を通じて再利用可能な registrars を整え、`docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md:238-265` の Capability 章と連携した。
   - `CollectOutcome::audit` 側の `core.collections.audit` チェック (`compiler/runtime/src/prelude/collectors/mod.rs:397`) や `scripts/validate-diagnostic-json.sh --suite collectors`/`tooling/ci/collect-iterator-audit-metrics.py --scenario audit_cap` gate への組み込みは次フェーズへ継続とし、`docs/notes/stdlib/collections-audit-bridge-todo.md` から引き継ぐ。

4. **ドキュメントとサンプルの同期**
   - `docs/spec/3-2-core-collections.md` §3.3 に `Table.load_csv`/`collect_table_csv` の注釈を加え、`examples/core-collections/README.md`/`usage.reml` を指して `effect {io}` や `CollectError` の発火点を示した。
   - `examples/core-collections/README.md` に CLI 実行手順と `collect_table_csv` シナリオを補足し、`README.md` および `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` に Core.Collections の進捗ブロックを維持する予定を追記した。

## 検証と CI 統合
- `cargo test core_collections_vec core_collections_cell_ref core_collections_table` を Phase3 CI に組み込み、`CollectError` や `collector.effect.*` 出力を `scripts/validate-diagnostic-json.sh --suite collectors` で gate。
- `tooling/ci/collect-iterator-audit-metrics.py --section collectors --scenario vec_mem_exhaustion|cell_internal_mutation|table_csv_import|audit_cap` を自動化し、`reports/spec-audit/ch1/core_iter_collectors.json`/`.audit.jsonl` を KPI source に指定。`reports/iterator-collector-summary.md` の `status` カラムと `reports/spec-audit/diffs/README.md` の dual-write 表にレポート。
