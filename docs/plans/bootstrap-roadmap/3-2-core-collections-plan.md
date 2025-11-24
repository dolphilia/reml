# 3.2 Core Collections 実装計画

## 目的
- 標準仕様 [3-2-core-collections.md](../../spec/3-2-core-collections.md) に従い、永続／可変コレクション API を Reml 実装へ移植し、`Iter`・`Diagnostics`・`Text` との相互運用性を確保する。
- 永続構造のパフォーマンスと効果タグの整合性を検証し、監査ログ・Config/Data 連携で要求される差分出力機能を備える。
- 仕様／実装／ドキュメントの差分を整理し、Phase 3 以降のセルフホスト環境で安定利用できるテスト資産を整備する。

## スコープ
- **含む**: `List`/`Map`/`Set`/`Vec`/`Cell`/`Ref`/`Table` の実装、Collector 連携、監査ログ向け変換、効果タグ検証、ドキュメント更新。
- **含まない**: 並列・分散コレクション、GC 導入を前提とした最適化（Phase 4 のメモリ戦略に委譲）。
- **前提**: `Core.Prelude`/`Core.Iter` 実装タスク (3-1) が完了または並行で進行しており、`Core.Diagnostics`/`Core.Text` の基盤が Phase 2 から提供されていること。

## 作業ブレークダウン

### 1. API 差分調査とモジュール設計（38週目）
**担当領域**: 設計調整

1.1. 仕様に記載された公開 API を一覧化し、Rust 実装（`compiler/rust/`）の現状との差異・未実装 API を洗い出す。OCaml 版は必要に応じて実装方針を参照するのみとし、作業計画の比較対象には含めない。
1.2. 効果タグ (`effect {mut}`, `{mem}`, `{cell}`, `{rc}`, `{audit}`) の付与規則を整理し、テスト戦略とメトリクス項目を定義する。
1.3. 永続構造と可変構造で共有する内部ユーティリティ (アロケータ、ハッシュ関数) の設計指針を決定する。

#### 1.1 API 差分一覧
38 週目の初手として仕様と Rust 実装を突き合わせ、どの API が不足しているかを以下に整理した。API 名は仕様を抜粋し、現状と対応方針を明示する。

| カテゴリ | 仕様で要求される主 API | Rust 実装現況 | 差分と対応方針 |
| --- | --- | --- | --- |
| `List<T>` | `empty`/`singleton`/`push_front`/`concat`/`map`/`fold`/`to_iter`/`as_vec`【F:../../spec/3-2-core-collections.md†L21-L45】 | `runtime/src/collections/persistent/list.rs`（finger tree）と `prelude/collectors/list.rs` で `push_front`/`concat`/`map`/`fold`/`to_iter` が実装済みだが、公開名としての `List.as_vec` や `effect {mem}` 計測は未着手で、Collector は一度 `Vec` へ積んでから finger tree を再構築している。【F:../../compiler/rust/runtime/src/collections/persistent/list.rs†L6-L173】【F:../../compiler/rust/runtime/src/prelude/collectors/list.rs†L1-L90】 | `List.as_vec` エイリアスと `EffectSet::record_mem_bytes` を `List::to_vec`/`ListCollector::finish` に追加し、`ListCollector` の `push` が `PersistentArena` に直接ノード確保するように拡張する。`Iter.collect_list`/`List.to_iter` は `IterStage::Stable` を `AuditEnvelope.metadata` へ伝搬し、`collect_list_baseline` snapshot の差分を固定する。 |
| `Map<K,V>` / `Set<T>` | `empty_map`/`insert`/`update`/`merge`/`keys`、`contains`/`diff`/`partition`【F:../../spec/3-2-core-collections.md†L46-L69】 | `Map`/`Set` も `BTreeMap`/`BTreeSet` の薄いラッパーで、`into_*` 以外の公開 API が欠落し、`merge`/`diff`/`Collector` 連携の仕様ギャップがある。【F:../../compiler/rust/runtime/src/prelude/collectors/map.rs†L20-L107】【F:../../compiler/rust/runtime/src/prelude/collectors/set.rs†L20-L119】 | 赤黒木ベースの `PersistentMap`/`PersistentSet` を `collections/persistent/btree.rs` にまとめて実装し、`diff`/`merge` を Config/Data の `SchemaDiff` へ接続する。`MapCollector`/`SetCollector` は既存の `BTree*` を利用したまま API を添付する。 |
| 変換ヘルパ／Iter 終端 | `List.of_iter`/`Map.from_iter`/`Set.diff` や `list_to_vec`/`map_to_table` 等の変換 API【F:../../spec/3-2-core-collections.md†L70-L80】【F:../../spec/3-2-core-collections.md†L227-L244】 | `Iter` 側は `collect_list`/`collect_vec` のみを提供し、`collect_map`/`collect_table` や `Iter.try_collect` 経由の `Map.from_iter` が存在しない。【F:../../compiler/rust/runtime/src/prelude/iter/mod.rs†L145-L151】 | `Iter` に `collect_map`/`collect_set`/`collect_table` を追加し、`List.of_iter` などのヘルパは `Collector` をラップする構成で `Result`/`CollectError` をそのまま返す。変換 API 群を `Core.Collections` 名前空間にまとめ、差分適用時に `effect` 伝播を保証する。 |
| `Vec<T>` | `new`/`with_capacity`/`push`/`pop`/`reserve`/`shrink_to_fit`/`iter`/`to_list` および `collect_from`【F:../../spec/3-2-core-collections.md†L91-L116】 | `VecCollector` は存在するが `Core.Collections.Vec` としての API や `CollectError::OutOfMemory` 伝播は未整備。`reserve` 失敗を診断へ橋渡しする仕組みも無い。【F:../../compiler/rust/runtime/src/prelude/collectors/vec.rs†L18-L100】 | `Vec<T>` 用のラッパ型（`CoreVec<T>`）を導入し、`try_reserve` の `TryReserveError` を `CollectError::OutOfMemory` に写像する。`to_list` は `List` へコピーした上で `effect {mem}` を記録し、`Vec.collect_from` を `Iter::collect_vec` と共通実装にする。 |
| `Cell<T>` / `Ref<T>` | `new_cell`/`get`/`set` と `new_ref`/`clone_ref`/`borrow`/`borrow_mut`【F:../../spec/3-2-core-collections.md†L91-L134】 | `collectors/mod.rs` に `cell`/`ref` モジュールが存在せず、内部可変性や `effect {cell}`/`{rc}` を発火させる仕組みが未着手。【F:../../compiler/rust/runtime/src/prelude/collectors/mod.rs†L8-L20】 | `Cell` は `RefCell` + Copy 制約を満たす軽量構造として `effect {cell}` を記録し、`Ref` は `Arc` + `RwLock` ベースで `effect {rc}`/`{mut}` を付ける。両者とも `CollectorAuditTrail` へ内部可変性マーカーを追記する。 |
| `Table<K,V>` | `new_table`/`insert`/`remove`/`iter`/`to_map`/`load_csv`【F:../../spec/3-2-core-collections.md†L138-L149】 | `Table` は `Vec<(K,V)>` 保存と `into_entries` のみで、挿入・削除・CSV ロード・`effect {io}` は未実装。`TableCollector` も `seen` のみで監査フックが簡易的。【F:../../compiler/rust/runtime/src/prelude/collectors/table.rs†L20-L124】 | Robin Hood hashing + 挿入順リストを保持する `OrderedTable` を実装し、`insert`/`remove`/`iter`/`to_map` を公開する。`load_csv` は `Core.IO` 連携タスク（3-5）と協調し、`effect {io}`/`{mut}` を同時に記録する。 |

上記に付随して `Collections.audit_bridge`（仕様 §5）や差分 API の JSON 変換が丸ごと欠落している点も確認した。`CollectOutcome` が保持する `CollectorAuditTrail` を Config/Data 章の `ChangeSet` に流し込むブリッジ層を Phase 3.2 で構築する。

**現状確認（2027-03-21）**
- `runtime/src/collections/persistent/list.rs` では finger tree ベースの `List` が定義されており、`push_front`/`concat`/`map`/`fold`/`to_iter` まで Rust 実装が揃っている一方、仕様で定義されている `List.as_vec` の別名や `effect {mem}` 打刻はまだ存在しない。`List::to_vec` は常に `Vec` へコピーするため、`EffectSet::record_mem_bytes` を利用した可視化が必要な状態である。【F:../../compiler/rust/runtime/src/collections/persistent/list.rs†L6-L173】
- `ListCollector` の snapshot (`collect_list_baseline`) は `EffectLabels` がすべて `false` のままで、`reports/iterator-collector-summary.md` でも `collector.effect.mem=false` が観測されている。`List.as_vec` を経由したコピーコストを KPI 化するには、Collector 側から `AuditEnvelope.metadata` へ `mem_bytes` を送る追加作業が必須である。【F:../../compiler/rust/frontend/tests/core_iter_collectors.rs†L1-L74】【F:../../reports/iterator-collector-summary.md†L15-L23】
- `MapCollector`/`SetCollector` は依然として `BTreeMap`/`BTreeSet` の薄いラッパーのみを提供しており、`merge`/`diff`/`keys` を含む仕様 API が欠落している。`unified-porting-principles.md` が要求する「振る舞いの同一性優先」の方針に従い、永続木実装と Config/Data 連携を WBS 3.2 の優先項目として扱う。【F:../../compiler/rust/runtime/src/prelude/collectors/map.rs†L1-L114】【F:../../compiler/rust/runtime/src/prelude/collectors/set.rs†L1-L120】【F:../../plans/rust-migration/unified-porting-principles.md†L1-L38】

#### 1.2 効果タグ規則とテスト／メトリクス戦略
`EffectSet`/`EffectLabels` は現在 `mut`/`mem`/`debug`/`async_pending` の 4 種のみを追跡しており（ビット構成 0b0001〜0b1000）、`effect {cell}`/`{rc}`/`{audit}` に対応する観測値が欠落している。【F:../../compiler/rust/runtime/src/prelude/iter/mod.rs†L677-L806】　仕様が要求するタグごとに実装・計測・検証の方針を整理した。

| 効果タグ | 対象 API / イベント | 実装と観測手段 | テスト / メトリクス |
| --- | --- | --- | --- |
| `effect {mem}` | `List.as_vec`、`List.to_vec`、`Vec.reserve`/`shrink_to_fit`、`Map.to_table` など | `EffectSet` を 8 ビット → 16 ビットへ拡張し、`mem_bytes` を `try_reserve` / `collect_vec` 時に加算。`collector.effect.mem` と `collector.effect.mem_reservation` を `CollectorAuditTrail` に出力する【F:../../reports/iterator-collector-summary.md†L1-L53】 | 既存の `collect_vec_mem_reservation` ケースを Rust 版でも維持し、`tooling/ci/collect-iterator-audit-metrics.py` の `collector.effect.mem` 判定に新規シナリオ（`List.as_vec` 経由のコピー）を追加する。【F:../../tooling/ci/collect-iterator-audit-metrics.py†L1-L117】 |
| `effect {mut}` | `Vec.push/pop`、`Table.insert/remove`、`Ref.borrow_mut` | `EffectSet::mark_mut` を `VecCollector` 以外の可変 API でも呼び出し、監査ログに `collector.effect.mut` が乗るよう拡張する。`Table` は `CollectorKind::Table` の `EffectLabels` を `mem=true`/`mut=true` へ固定する。 | `collect_table_insert_remove`（新設）で `collector.effect.mut=true` を期待値にし、`reports/spec-audit/ch1/core_iter_collectors.json` へ追加する。 |
| `effect {cell}` | `Cell.new`/`Cell.set` | `EffectSet` に `CELL_BIT` を追加し、`EffectLabels` へ `cell: bool` フィールドを追加。`CollectorAuditTrail` から `collector.effect.cell` を算出し、`AuditEnvelope.metadata["collector.effect.cell"]` へ出力する。 | `Cell` の単体テストで `collector.effect.cell = true` を assert。`collect-iterator-audit-metrics` に `--require-cell` 相当のチェッカーを追加し、`reports/iterator-collector-summary.md` に KPI を追記する。 |
| `effect {rc}` | `Ref.new`/`clone_ref`/`borrow_mut` | `EffectSet` に `RC_BIT` を追加し、参照カウント増減時に `mark_rc()` を呼ぶ。`CollectorAuditTrail` へ `collector.effect.rc` を出力し、`Diagnostic.extensions["prelude.collector.rc_ops"]` にカウントを同期する。 | `Ref` API のゴールデン（OCaml 版の `RefCollector` テスト）を Rust 側に移植し、`collect-iterator-audit-metrics` に RC 係数の集計フィールドを追加する。 |
| `effect {audit}` | `Map.diff`/`Set.diff`/`Table.to_map` → `AuditEnvelope.change_set` 生成、`Collections.audit_bridge` | `CollectOutcome::audit` を `Core.Diagnostics` と `Core.Config` へ橋渡しするアダプタを実装し、`AuditEnvelope.metadata` に `collector.effect.audit=true` を付与する。`effect {audit}` は `EffectSet` の新ビット (`AUDIT_BIT`) で追跡する。 | 監査ログ（`reports/spec-audit/ch1/core_iter_collectors.audit.jsonl`）に `collector.effect.audit` を新規キーとして持たせ、`collect-iterator-audit-metrics` の `--section collectors` 実行時に必須フィールドとして検証する。 |

上記に合わせて `EffectLabels` の JSON 変換を更新し、`collector.effect.*` のダッシュボードを `reports/iterator-collector-metrics.json` で追跡する。テスト面では `cargo test core_iter_collectors` に `Cell`/`Ref`/`Table` 用のスナップショットを追加し、`scripts/validate-diagnostic-json.sh` パターンに `collector.effect.cell`/`collector.effect.rc` を含める。

#### 1.3 永続／可変構造で共有する内部ユーティリティ設計
差分で明らかになった欠落を埋めるため、永続構造と可変構造の双方で流用できるユーティリティ層を設計する。仕様 §6 の性能要件（Finger tree / 赤黒木 / Robin Hood hashing）を満たしつつ、監査ログとの橋渡しをモジュール単位で再利用できる構成を定める。【F:../../spec/3-2-core-collections.md†L175-L190】

- `PersistentArena`：finger tree ノードと赤黒木ノードを共通のバンプアロケータで確保し、変更が `List`/`Map`/`Set` 全体の `@pure` を損なわないよう `Arc` + `ThinBox` を用いた構造共有を提供する。Arena は `ListCollector` が返す `List` と `MapCollector` が返す `Map` の双方で再利用し、ベンチ指標（構造共有による 20〜30% オーバーヘッド）を維持する。
- `DeterministicHasher`：`Table` の挿入順ハッシュと `Map.diff` の差分キー計算で共通化する。現在の `TableCollector` は `BTreeSet` で重複検出のみを行っているため、ここを Robin Hood hashing + `FxHasher` 互換のシード付きハッシュへ差し替え、`map_to_table`/`table_to_map` の順序保証を支える。【F:../../compiler/rust/runtime/src/prelude/collectors/table.rs†L20-L124】
- `AuditChangeBridge`：`CollectOutcome` と Config/Data 章の `ChangeSet`/`SchemaDiff` を橋渡しし、`effect {audit}` を打刻する。`Collections.audit_bridge` で `CollectError` → `Diagnostic` の変換を一元化し、`reports/iterator-collector-summary.md` の KPI に `collector.effect.audit` を追加するためのメタデータを生成する。【F:../../spec/3-2-core-collections.md†L167-L171】
- `GrowthBudget`：`Vec`/`Table`/`Cell` などミュータブル構造のメモリ確保を記録する軽量トラッカー。`VecCollector` の `reserve` で書いている `effects.mutating`/`effects.mem` を共通化し、`EffectLabels.mem_bytes` を `Table`/`Cell` の内部確保でも確実に更新する。【F:../../compiler/rust/runtime/src/prelude/collectors/vec.rs†L18-L100】

これらのユーティリティを `compiler/rust/runtime/src/collections/`（新ディレクトリ）にまとめ、`Core.Collections` モジュールから再エクスポートする。`PersistentArena`/`DeterministicHasher` は Phase 3-2（永続構造）と 3-3（Text & Unicode）でも共有できるため、後続タスクへの再利用性を確保する。

### 2. 永続コレクション実装（38-39週目）
**担当領域**: `List`/`Map`/`Set`

2.1. `List<T>` の finger tree ベース実装を移植し、`as_vec` や `of_iter` の性能評価を行う。
2.2. `PersistentMap`/`PersistentSet` を実装し、差分マージ (`merge`, `diff`, `update`) と `Collector` 連携をテストする。
2.3. 構造共有によるメモリ削減効果を測定し、`0-3-audit-and-metrics.md` にベンチマーク結果を記録する。

#### 2.1 `List<T>` finger tree 実装と評価
- `runtime/src/collections/persistent/list.rs`（新設）に finger tree に基づく `ListCore` を実装し、`PersistentArena` でノードを確保する。仕様が要求する `empty`/`singleton`/`push_front`/`concat`/`map`/`fold`/`to_iter`/`as_vec` を全て Rust 版 `Core.Collections.List` に公開し、`ListCollector` の戻り値を `Arc<ListCore<T>>` へ差し替える。【F:../../spec/3-2-core-collections.md†L21-L45】
- `List.to_iter`/`List.of_iter` は `Core.Iter` の stage 情報（`IterStage::Stable`）を受け渡し、`Iter.collect_list` からの `CollectError` 伝播を維持する。`Iter` モジュール内で `CollectorKind::List` を `finger_tree` バックエンドと紐づけ、`collect_list` のテストケースを `compiler/rust/runtime/tests/core_iter_collectors.rs` に追加する。
- `List.as_vec` 実行時に `EffectSet::mark_mem(bytes)` を呼び、`collector.effect.mem_reservation` を `CollectorAuditTrail` へ書き出す。`reports/iterator-collector-summary.md` の KPI テーブルに `list_as_vec_mem_bytes` を追記して `scripts/validate-diagnostic-json.sh` で検証する。
- Finger tree の性能評価は `tooling/ci/collect-iterator-audit-metrics.py --section collectors --scenario list_fingertree` を追加し、`Vec` オーバーヘッド比（±25% 以内）と構造共有ヒット率（70%以上）を測定する。結果を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の Phase 3 指標欄へ記載し、`docs-migrations.log` に測定日とコミットを追記する。

**実施ログ（2027-03-21）**
- `List` finger tree は既に `runtime/src/collections/persistent/list.rs` に存在し、`PersistentArena` でノードを共有しているものの、`ListCollector` は `Vec` を経由してから finger tree を再構築している。`ListCollector::push` で `PersistentArena` を直接叩くワークロードを先に実装し、`PersistentArena` の API (`alloc`, `Arc` 管理) を `List.collector` 系と共有できるよう整理する。【F:../../compiler/rust/runtime/src/collections/persistent/list.rs†L6-L173】【F:../../compiler/rust/runtime/src/collections/persistent/arena.rs†L1-L52】
- `Iter::collect_list` は `collect_into_collector(ListCollector::new())` を呼ぶだけで stage/effect 伝搬をカスタマイズできないため、`List.to_iter` から `IterStage::Stable` を受け渡す新しい `Iter::from_list` 相当が必要。`collect_list_pipeline-h1` snapshot では Stage 情報が `stable` で固定されているので、この挙動を維持しつつ `AuditEnvelope.metadata` に `collector.effect.mem_bytes` を追加する方向で再設計する。【F:../../compiler/rust/runtime/src/prelude/iter/mod.rs†L144-L195】【F:../../reports/iterator-collector-summary.md†L46-L50】
- 計測面では `collect_list_baseline` の KPI 群に `list_as_vec_mem_bytes` を追加し、`tooling/ci/collect-iterator-audit-metrics.py` へ `list_fingertree` シナリオを実装した後、`scripts/validate-diagnostic-json.sh --pattern collector.effect.mem_bytes` を必須にする。`0-3-audit-and-metrics.md` の Phase3 セクションで `List` コピーコスト指標をトラックし、`reports/iterator-collector-summary.md` の Stage/KPI 表に追記する。【F:../../reports/iterator-collector-summary.md†L15-L34】【F:../../tooling/ci/collect-iterator-audit-metrics.py†L1-L69】【F:0-3-audit-and-metrics.md†L1-L60】【F:../../scripts/validate-diagnostic-json.sh†L1-L60】
- **2027-03-21 Update**: `ListCollector::finish` で `list.len() * size_of::<T>()` に基づく `collector.effect.mem_bytes` と `collector.effect.mem_reservation` を記録し、`collect_list_baseline` snapshot / `reports/iterator-collector-summary.md` の KPI に `list_as_vec_mem_bytes` を追加した。`core_iter_collectors.snap` には `collector.effect.mem=true` / `mem_bytes=12` を反映済みで、`scripts/validate-diagnostic-json.sh --pattern collector.effect.mem_bytes reports/spec-audit/ch1/core_iter_collectors.json` を通じた検証手順を追記した。【F:../../compiler/rust/runtime/src/prelude/collectors/list.rs†L1-L125】【F:../../compiler/rust/frontend/tests/__snapshots__/core_iter_collectors.snap†L1-L30】【F:../../reports/iterator-collector-summary.md†L1-L36】

#### 2.2 `PersistentMap` / `PersistentSet` 実装と検証
- `runtime/src/collections/persistent/btree.rs`（新設）で赤黒木ベースの `PersistentMap<K,V>` / `PersistentSet<T>` を実装し、`MapCollector`/`SetCollector` から返却する型を `Persistent*` に切り替える。挿入・削除は構造共有を維持し、`@pure` を保ちながら O(log n) を保証する。【F:../../spec/3-2-core-collections.md†L46-L69】
- `Map.merge`/`Map.diff`/`Set.diff` の出力を Config/Data 仕様 (`docs/spec/3-7-core-config-data.md` §2) で定義されている `SchemaDiff`/`ChangeSet` と互換な JSON 表現へ変換する `Collections.audit_bridge` を追加する。差分結果は `collect-iterator-audit-metrics` の `--require-audit` モードで `collector.effect.audit=true` を検証し、`AuditEnvelope.change_set` と双方向で照合する。【F:../../spec/3-7-core-config-data.md†L16-L55】
- キー順序の決定性を担保するため `DeterministicHasher` と `PersistentArena` を共有し、`Map.keys`/`Set.into_iter` が常に昇順で走査されることを `compiler/rust/runtime/tests/core_collections_map_set.rs` に QuickCheck 相当のプロパティテストとして追加する。
- Error ハンドリングは `Map.from_iter` の重複検出・`Set.diff` の空差分最適化を `CollectError::{DuplicateKey,InternalInvariant}` に分類し、`reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` のサンプルへケースを追加する。`Tooling` 側の `poc_dualwrite_compare.sh` で `Map`/`Set` 差分検証を有効化し、OCaml 版のゴールデン結果と比較して 95% 一致を目標にする。

**実施ログ（2027-03-26）**
- 現状の `MapCollector`/`SetCollector` は `BTreeMap`/`BTreeSet` を `CollectOutcome` で包んでいるだけで、`PersistentArena` を利用した構造共有や `effect.audit` 計測が行われていない。`finish` 実行後も `CollectorEffectMarkers` のほとんどが `false` のままで、Config/Data 章が要求する差分監査に必要なメタデータを発行できない状態である。【F:../../compiler/rust/runtime/src/prelude/collectors/map.rs†L1-L107】【F:../../compiler/rust/runtime/src/prelude/collectors/set.rs†L1-L119】
- `PersistentArena` は `List` 指向の finger tree で既に利用されており、`ArenaPtr` を通じた `Arc` 共有で `push_front`/`concat` のコストを抑制している。Arena の `alloc`/`strong_count` API を `Map`/`Set` の赤黒木ノードに適用すれば、挿入・削除のパスコピーが `O(log n)` に収束する見通しを確認した。【F:../../compiler/rust/runtime/src/collections/persistent/arena.rs†L1-L52】【F:../../compiler/rust/runtime/src/collections/persistent/list.rs†L1-L173】
- Config/Data 側の `SchemaDiff`・`ChangeSet` は `Map` ベースで表現されているが、Rust 版 Core.Collections には `Map.diff` → `SchemaDiff` 変換を担うフックが存在しない。`AuditEnvelope.change_set` に `map.diff` 系統を記録するため、`Collections.audit_bridge` を `Core.Diagnostics` 連携モジュールとして実装する方針に合意した。【F:../../spec/3-7-core-config-data.md†L16-L55】【F:../../docs/spec/3-6-core-diagnostics-audit.md†L42-L120】

##### 実装アウトライン
1. `PersistentMap`/`PersistentSet` コア: `runtime/src/collections/persistent/btree.rs` に `PersistentArena<BTreeNode<K,V>>` と `Option<ArenaPtr<BTreeNode<K,V>>>` を保持する `struct PersistentMap<K,V>` を用意し、`enum Color { Red, Black }` + `Node { key, value, left, right, size }` で赤黒木不変条件を維持する。`insert`/`update`/`merge`/`diff` はパス上のノードを再確保して構造共有し、`PersistentSet<T>` は `PersistentMap<T, ()>` を内部に持つ薄いラッパーとして `contains`/`insert`/`partition` を再利用する。公開 API 群（`empty_map`/`get`/`keys` 等）は `Core.Collections` 名前空間から再エクスポートし、仕様で定義された `@pure` 契約を維持する。【F:../../spec/3-2-core-collections.md†L46-L89】
2. Collector の置き換え: `MapCollector`/`SetCollector` が `CollectOutcome<PersistentMap<K,V>>` / `CollectOutcome<PersistentSet<T>>` を返すよう改修し、`push` では `PersistentMap::insert_arc(self.storage.clone(), key, value)` のような内部ヘルパを呼ぶ構成にする。`CollectorStageProfile` と `EffectLabels` は `@pure` を維持しつつ、`collector.effect.audit` と `collector.effect.mem_bytes` を `Map.diff`／`Set.diff` の結果バイト数に応じて更新する。KPI は `reports/iterator-collector-summary.md` の `Core.Collections` テーブルへ `map_diff_total`, `set_diff_total` を追記して可視化する。【F:../../compiler/rust/runtime/src/prelude/collectors/map.rs†L1-L107】【F:../../reports/iterator-collector-summary.md†L15-L50】
3. `Collections.audit_bridge`: `compiler/rust/runtime/src/collections/audit_bridge.rs`（新設）へ `fn map_diff_to_changes<K,V>(base: &PersistentMap<K,V>, delta: &PersistentMap<K,V>) -> ChangeSet` / `fn set_diff_to_changes<T>(...)` を実装し、`docs/spec/3-7-core-config-data.md` §2 で定義された `added`/`removed`/`updated`/`metadata.stage` を JSON に整形する。`Map.merge`/`Map.diff`/`Set.diff` からこのブリッジを呼び出し、`AuditEnvelope.change_set` に `collections.diff.*` キーを記録する。`poc_dualwrite_compare.sh --target map_diff`（新規）で OCaml 実装と JSON を比較し、差分発生時は `reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` に再現手順を残す。【F:../../docs/spec/3-7-core-config-data.md†L16-L55】【F:../../scripts/poc_dualwrite_compare.sh†L1-L120】
4. 決定性とハッシュ: `DeterministicHasher` を `runtime/src/collections/hash.rs`（新設）で提供し、`Map.keys`/`Map.into_iter`/`Set.into_iter` が常に昇順を返すよう `Ord` と `Hasher` を組み合わせる。`compiler/rust/runtime/tests/core_collections_map_set.rs` では (a) 無作為挿入後も `keys` が昇順である、(b) `Set.into_iter` の stage が `IterStage::Stable` に固定される、の 2 つの property テストを追加する。【F:../../compiler/rust/runtime/tests/core_collections_map_set.rs†L1-L120】
5. エラーと診断: `CollectError::{DuplicateKey,InternalInvariant}` を `Map.from_iter`/`Set.diff` で再利用し、`compiler/rust/runtime/src/prelude/iter/mod.rs` に追加する `collect_map`/`collect_set` 経路が `AuditEnvelope.metadata["collector.error.kind"]` を埋めるようにする。`reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` へ (1) 重複キー、(2) `Set.diff` が空集合を返す最適化、(3) `Map.merge` のマージ関数が panic するケース、をサンプルとして追加し、`tooling/ci/collect-iterator-audit-metrics.py --section collectors --scenario map_set_persistent` で `collector.effect.audit` の有無をチェックする。【F:../../compiler/rust/runtime/src/prelude/iter/mod.rs†L144-L210】【F:../../tooling/ci/collect-iterator-audit-metrics.py†L1-L120】
6. ドキュメント・メトリクス: `docs/plans/rust-migration/p1-spec-compliance-gap.md` の `Core.Collections` 節へ `PersistentMap` 差分の記録を追加し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` には `map_diff_total`, `set_diff_total`, `map_merge_latency` の KPI を追記する。`docs-migrations.log` にも「2027-03-26 Core.Collections PersistentMap 設計更新」を記載し、将来の Phase 4 で参照できるようにする。

###### Collections.audit_bridge 詳細計画（実施 2027-03-30）
1. モジュール設計: `compiler/rust/runtime/src/collections/audit_bridge.rs` を `Core.Collections` 名前空間に登録し、`ChangeSet`/`SchemaDiff` のシリアライズ規約（`added`/`removed`/`updated`、`metadata.stage`, `metadata.collector.effect.audit`）を `docs/spec/3-7-core-config-data.md` §2 と `docs/spec/3-6-core-diagnostics-audit.md` §3 から抽出する。`AuditBridgeState`（`PersistentArena` 参照 + `EffectLabels`）を定義し、`Map.diff`/`Set.diff`/`Table.to_map` の出力と `AuditEnvelope` の差分チャネルが単一モジュールを経由するよう整理する。【F:../../spec/3-7-core-config-data.md†L16-L118】【F:../../spec/3-6-core-diagnostics-audit.md†L42-L158】
2. データフローの固定: `map_diff_to_changes` / `set_diff_to_changes` / `table_merge_to_changes` の 3 API を公開し、`PersistentMap` 側では `DiffOutcome` → `AuditBridgeInput` → `ChangeSetJson` の流れで JSON を生成する。キー比較は `DeterministicHasher` + `Ord` ベースの昇順に限定し、`Map.merge` からは `AuditBridgeOutcome::Merged { applied, skipped, metadata }` を受け取る。`collector.effect.audit`/`mem_bytes` は `EffectLabels` を通じて `AuditEnvelope.metadata["collections.diff.mem_bytes"]` 等に転写する。
3. 監査ログ統合: `Core.Diagnostics` の `AuditEnvelope` を拡張して `collections.diff.*` キーを予約し、`collect-iterator-audit-metrics.py --section collectors --scenario map_set_persistent --require-audit` で `collector.effect.audit=true` かつ `change_set.total > 0` を必須ゲートにする。`reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` へ `map_diff`, `set_partition`, `table_merge` の 3 ケースを追加し、`scripts/validate-diagnostic-json.sh --section collections_audit_bridge` で JSON スキーマを検証する。【F:../../reports/spec-audit/ch1/core_iter_collectors.audit.jsonl†L1-L120】【F:../../tooling/ci/collect-iterator-audit-metrics.py†L1-L140】
4. dual-write & テスト: `scripts/poc_dualwrite_compare.sh --target map_diff` に `collections.audit_bridge` パイプラインを追加し、OCaml 実装の `Runtime.Map.diff` と JSON を比較する。Rust 側では `compiler/rust/runtime/tests/core_collections_audit_bridge.rs`（新規）を用意し、(a) `Map.merge` が `ChangeSet.updated` を正しい件数で出す、(b) `Set.diff` が `removed` のみのケースで `total=1` になる、(c) 空差分時に `collector.effect.audit=false` が立ち `change_set.total=0` になる、の 3 シナリオを snapshot で収集する。CI は `cargo test core_collections_audit_bridge` を `collect-iterator-audit-metrics.py --run-tests` から呼び出す。
5. ドキュメント更新: `docs/plans/rust-migration/p1-spec-compliance-gap.md` に `audit_bridge` のギャップ完了を記録し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ `collections.audit_bridge_pass_rate`/`collector.effect.audit_presence` の KPI を追加する。また `docs/notes/spec-integrity-audit-checklist.md` に「Core.Collections diff JSON の検証」チェックを記載し、`docs-migrations.log` にも `Collections.audit_bridge` の初回実装と `AuditEnvelope` 拡張の日時を残す。

**進捗（2027-03-31 時点）**
- `compiler/rust/runtime/src/collections/audit_bridge.rs` を実装し、`PersistentMap`/`PersistentSet` から `ChangeSet` を生成する `map_diff_to_changes` / `set_diff_to_changes` を提供済み。`PersistentMap::diff_change_set` / `PersistentSet::diff_change_set` / `PersistentMap::merge_with_change_set` を追加し、永続構造から直接 `ChangeSet` を取得できる状態にした。【F:../../compiler/rust/runtime/src/collections/persistent/btree.rs†L1-L212】
- Collector 側では `EffectLabels` に `audit` ビットと `mem_bytes` 追跡を追加し、`CollectorAuditTrail::record_change_set` で `collector.effect.audit` / `collector.effect.mem_bytes` を自動記録するよう拡張済み（`compiler/rust/runtime/src/prelude/iter/mod.rs`, `compiler/rust/runtime/src/prelude/collectors/mod.rs`）。Snapshot 生成 (`compiler/rust/frontend/tests/core_iter_collectors.rs`) と `core_iter_collectors.snap` も新ラベルを出力するよう更新し、`tooling/ci/render-collector-audit-fixtures.py` で再生成した `reports/spec-audit/ch1/core_iter_collectors.{json,audit.jsonl}` に `collector.effect.audit` が現れている。
- CI スクリプト `tooling/ci/collect-iterator-audit-metrics.py` に `collect_collections_audit_bridge_metrics` と `--scenario map_set_persistent` を追加し、`collections.audit_bridge_pass_rate` / `collector.effect.audit_presence` KPI を `--require-success` のチェック対象へ組み込んだ。現在はフィクスチャ入力（`core_iter_collectors.json`）で pass / fail 判定を確認できる状態。
- Config/Data ルート向けのパイプを整備し、`compiler/rust/runtime/src/config/mod.rs` に `merge_maps_with_audit` / `write_change_set_to_path` を実装。`formatter.rs` では `REML_COLLECTIONS_CHANGE_SET{,_PATH}` 環境変数から `collections.diff.*` を含む JSON を読み取り、`AuditEnvelope.change_set["collections"]` へ自動結合するため、Map.merge や Config CLI が生成した差分が監査ログへ伝播する。

**次のステップ**
- Config/Data API で `merge_maps_with_audit` を呼び、生成された JSON を `REML_COLLECTIONS_CHANGE_SET_PATH` へ書き出す CLI 手順を `scripts/poc_dualwrite_compare.sh --target map_diff` に統合する。`collector.effect.audit=true` を要求する差分ケース（`map_diff`, `set_partition`）を OCaml 実装と比較し、`reports/spec-audit/ch1/*.json` に実測ログを残す。
- KPI 文書（`reports/iterator-collector-summary.md`, `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`）に実測値を書き戻し、`docs/notes/spec-integrity-audit-checklist.md` の TODO をクローズする。

##### テスト・CI への反映
- `compiler/rust/runtime/tests/core_collections_map_set.rs` を拡張し、`quickcheck` 相当の property（`Map.keys_are_sorted`, `Set.diff_idempotent`）と `SchemaDiff` 互換の JSON snapshot を `tests/__snapshots__/core_collections_map_set.snap` に追加する。
- `tooling/ci/collect-iterator-audit-metrics.py` へ `map_set_persistent` シナリオを追加し、`--require-audit` フラグで `collector.effect.audit=true` と `AuditEnvelope.change_set.total` を検証する。CI では `scripts/validate-diagnostic-json.sh --section core_collections_map_set` を追加し、`reports/spec-audit/ch1/core_iter_collectors.json` の必須キー（`collections.diff.added`, `collections.diff.removed` 等）をチェックする。【F:../../reports/spec-audit/ch1/core_iter_collectors.json†L1-L80】【F:../../scripts/validate-diagnostic-json.sh†L1-L80】
- `scripts/poc_dualwrite_compare.sh --target map_set` を更新し、OCaml 実装との dual-write を自動化する。差分が 5% を超えた場合は `docs/plans/bootstrap-roadmap/4-0-phase4-migration.md` へフォローアップを追加し、`reports/spec-audit/diffs/README.md` に結果を添付する。

#### 2.3 構造共有メトリクスとレポート化
- `compiler/rust/runtime/ffi/src/core_collections_metrics.rs` と `compiler/rust/runtime/ffi/benches/core_collections_persistent.rs` を追加し、`List`/`PersistentMap` の構造共有率とメモリ推定値を生成する計測コードを整備する。入力データは `reports/spec-audit/ch0/links.md` に記録された DSL サンプルプロジェクトの構成を再現し、`List` 10^5 要素・`Map` 5×10^4 キーで GC 無しにピークメモリが入力サイズの 1.8 倍以内に収まることを確認する。
- ベンチ結果（共有率、割当回数、`effect {mem}` バイト数）を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` Phase3→Phase4 ブリッジ表に追記し、`docs/plans/bootstrap-roadmap/assets/metrics/core_collections_persistent.csv` に数値を保存する。`cargo run --manifest-path compiler/rust/runtime/ffi/Cargo.toml --features core_prelude --example core_collections_metrics -- docs/plans/bootstrap-roadmap/assets/metrics/core_collections_persistent.csv` を実行すると CSV が再生成できる。
- 成果のレビューでは `reports/spec-audit/README.md` の `Core Collections` セクションへベンチ出力ログと `git` ハッシュを添付し、Phase 3 Go/No-Go 判定の資料として `3-0-phase3-self-host.md` に「永続コレクション測定済み」のチェック項目を記録する。

- **実施ログ（2027-03-24）**
- `List`/`PersistentMap` それぞれの内部ノードを巡回して `Arc::strong_count` と `payload_bytes` を集計するメトリクス API（`List::sharing_stats_with`、`PersistentMap::sharing_stats_with`）を実装し、共有ノードは 50% コストとして計上する推定式を `compiler/rust/runtime/src/collections/persistent/list.rs:151` と `compiler/rust/runtime/src/collections/persistent/btree.rs:189` に追加した。
- ベンチマークの本体は `compiler/rust/runtime/ffi/src/core_collections_metrics.rs:9` にまとめ、DSL 由来の文字列と Config エントリを生成して `collect_persistent_metrics()` が `ScenarioMetrics` を返す構成にした。`compiler/rust/runtime/ffi/benches/core_collections_persistent.rs:1` では `harness = false` のベンチを用意し、`examples/core_collections_metrics.rs:1` から CSV を書き出せるようにしている。
- 最新の測定値は `docs/plans/bootstrap-roadmap/assets/metrics/core_collections_persistent.csv` に保存し、`ListPersistentPatch`（入力 25.1MB, peak ratio 1.7158）と `MapPersistentMerge`（入力 3.3MB, peak ratio 1.3903）で目標（1.8 倍以内）を満たした。これらの値は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` と `reports/spec-audit/README.md` の Core Collections 節から辿れる。

### 3. 可変コレクションと内部可変性（39週目）
**担当領域**: `Vec`/`Cell`/`Ref`/`Table`

3.1. 標準 `Vec` の API セットを仕様通り実装し、`effect {mut}`/`{mem}` の正確な付与を確認する。
3.2. `Cell<T>`/`Ref<T>` の内部可変性モデルを実装し、`effect {cell}`/`{rc}` を活用したテストケースを整備する。
3.3. `Table<K,V>` の挿入順序保持ロジック・CSV ローダを実装し、`Core.IO`/`Core.Text` と連携する統合テストを追加する。

#### 3.1 `Vec<T>` API と effect 伝播の確定
- `Vec` 向けの公開 API（`new`/`with_capacity`/`push`/`pop`/`reserve`/`shrink_to_fit`/`iter`/`to_list`/`collect_from`）を `runtime/src/collections/mutable/vec.rs` に実装し、`VecCollector`（`compiler/rust/runtime/src/prelude/collectors/vec.rs`）から `CoreVec<T>` を返す構成へ整理する。【F:../../spec/3-2-core-collections.md†L95-L117】 OCaml 版で用意されている `Runtime.Vec` との API 差分は `docs/plans/rust-migration/p1-spec-compliance-gap.md` の `Core.Collections` 節へ転記し、Rust 実装の ToDo として同期する。
- 効果タグは `EffectSet::mark_mut()` を全ての可変操作（`push`/`pop`/`collect_from`）へ付与し、`reserve`/`shrink_to_fit`/`to_list` 実行時には `EffectSet::mark_mem(bytes)` も併用する。`CollectorAuditTrail` には `collector.effect.mut` と `collector.effect.mem` の両方を記録し、`scripts/validate-diagnostic-json.sh` の `collect_vec_mem_reservation` パターンへ `Vec.to_list`/`Vec.collect_from` を追加する。
- `try_reserve` の `TryReserveError` は `CollectError::OutOfMemory` に写像し、`Iter.collect_vec` と `Vec.collect_from` で共通の `Result<Vec<T>, CollectError>` を返す。失敗例は `reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` にサンプルを追記し、`tooling/ci/collect-iterator-audit-metrics.py --section collectors --scenario vec_mem_exhaustion` で `effect {mem}` の計測が崩れないか検証する。【F:../../spec/3-2-core-collections.md†L115-L117】
- テスト計画: `compiler/rust/runtime/tests/core_collections_vec.rs` を追加し、(1) `collect_vec` が `CollectError::DuplicateKey` を返さないこと、(2) `reserve` が `EffectLabels` に `mem=true` をセットすること、(3) `Vec.to_list` が構造共有を壊しつつ `effect {mem}` を記録すること、を snapshot + property テストで確認する。ベンチ計測値は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ `vec_mut_ops_per_sec` として反映し、Phase 2 ベースライン比 ±15% を目標にする。

##### 実装アウトライン
1. `runtime/src/collections/mutable/vec.rs` へ `CoreVec<T>` の公開 API を集約し、`compiler/rust/runtime/src/prelude/mod.rs` で `pub use`。`VecCollector` は `CoreVec<T>` を返しつつ `EffectLabels` を `CollectOutcome` の `effects` フィールドに格納し、`docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` で合意した `Iter`→`Collector` の effect 伝播要件に揃える。
2. `Vec::iter`/`as_slice` は `effect {pure}`、`Vec::push`/`pop`/`collect_from` は `effect {mut}`、`Vec::reserve`/`shrink_to_fit`/`to_list` は `effect {mut, mem}` になるよう `EffectSet::mark_mut()` / `mark_mem(bytes)` を必ず呼ぶ。Effect 情報は `CollectorAuditTrail::record_vec_op`（新設）のペイロードへ保持し、`reports/spec-audit/ch1/core_iter_collectors.json` の `collector.effect.mem_bytes` に直結させる。
3. `TryReserveError` と `CollectError::OutOfMemory` の写像レイヤは `runtime/src/collections/mutable/vec/error.rs` に切り出し、`Iter.collect_vec` や `Vec.collect_from` が `Result<CoreVec<T>, CollectError>` で統一されるよう整理する。OCaml 版との差分は `docs/plans/rust-migration/p1-spec-compliance-gap.md` / `p1-rust-frontend-gap-report.md` と同期する。
4. ベンチマークは `compiler/rust/runtime/ffi/benches/core_collections_mutable.rs`（既存 persistent ベンチの隣）で `VecMutOpsPerSec` を計測し、結果を `docs/plans/bootstrap-roadmap/assets/metrics/core_collections_persistent.csv` に列追加して `0-3-audit-and-metrics.md` へ引用する。

###### 3.1.1 Vec API 実装ステップ詳細
1. `CoreVec<T>` の型設計・公開 API を `runtime/src/collections/mutable/vec.rs` へ置き、`docs/spec/3-2-core-collections.md` §2 の API リスト (`new`/`push`/`pop` 等) に合わせて `core::collections::mutable` から再エクスポートする。`docs/plans/rust-migration/p1-spec-compliance-gap.md` の `Core.Collections` 節に Rust 側の未実装項目を記録し、後続レビューで OCaml 実装との差異が追跡できるようにする。
2. `EffectSet::mark_mut()` / `mark_mem(bytes)` を `Vec::push`/`pop`/`reserve`/`shrink_to_fit`/`to_list`/`collect_from` に組み込み、`CollectorAuditTrail::record_vec_op` へ `collector.effect.mut`/`collector.effect.mem_bytes` を載せる。`docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` の `Iter`→`Collector` effect 伝播要件と矛盾しないことを CI（`scripts/validate-diagnostic-json.sh`）で確認する。
3. `TryReserveError` を `CollectError::OutOfMemory` に写像する `runtime/src/collections/mutable/vec/error.rs` を整備し、`Iter.collect_vec`／`Vec.collect_from` で共通の `Result<CoreVec<T>, CollectError>` を返すように統一する。エラーシナリオは `reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` の `vec_mem_exhaustion` ケースに追加して `tooling/ci/collect-iterator-audit-metrics.py --section collectors --scenario vec_mem_exhaustion` で `effect {mem}` の継続性を確認する。
4. `VecMutOpsPerSec` ベンチを `compiler/rust/runtime/ffi/benches/core_collections_mutable.rs` に追加し、`docs/plans/bootstrap-roadmap/assets/metrics/core_collections_persistent.csv` の `vec_mut_ops_per_sec` 列へ出力する。結果は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の Phase3 指標表へ引用し、Phase2 比 ±15% 以内の基準を明記する。

###### 3.1.2 Vec effect 監査・CI自動化
1. `CollectorAuditTrail::record_vec_op`（`VecCollector` 内）/`CollectOutcome::audit`（`Iter::collect_vec` 経路）を起点に `collector.effect.mem_bytes`/`collector.effect.mut` を `AuditEnvelope.metadata` と `Diagnostic.extensions["prelude.collector"]` に流し、`scripts/validate-diagnostic-json.sh --pattern collector.effect.mem --pattern collector.effect.mut reports/spec-audit/ch1/core_iter_collectors.json` で突合するルートを整備する。
2. `reports/spec-audit/ch1/core_iter_collectors.json` の Vec ケースと `reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` の `vec_mem_exhaustion` シナリオに `collector.effect.mem_bytes`/`collector.effect.mut` の期待値を追記し、`reports/iterator-collector-summary.md` に `vec_effect_metrics` セクションを追加して `collector.effect.mem_bytes` や `collector.effect.mut` の KPI を示す。
3. `tooling/ci/collect-iterator-audit-metrics.py --section collectors --scenario vec_mem_exhaustion` に `--require-success` チェックとして `collector.effect.mem_bytes > 0`/`collector.effect.mem`/`collector.effect.mut` を検証し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の Phase3表と `docs/plans/rust-migration/3-2-benchmark-baseline.md` の `iterator.collect_vec` セクションに `vec_effect_mem_bytes`/`vec_effect_mut` KPI を追記する。
4. `docs/notes/spec-integrity-audit-checklist.md` の `Vec collector.effect.*` 項目や `reports/spec-audit/README.md` の Core Collections 節へ Vec effect に関するログリンクを追記し、`docs-migrations.log` に Vec effect メタデータ追加の記録を残す。

##### 検証タスク
- `cargo test core_collections_vec core_iter_collectors` を Phase3 CI 必須項目として登録し、`tooling/ci/collect-iterator-audit-metrics.py --run-tests --scenario vec_mem_exhaustion` から自動実行する。
- `scripts/validate-diagnostic-json.sh --pattern collector.effect.mut collector.effect.mem reports/spec-audit/ch1/core_iter_collectors.json` を `Go/No-Go` 条件化し、失敗時は `reports/iterator-collector-summary.md` の KPI 列へ「Vec effect drift」を追記する。
- `docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md` と `docs/plans/rust-migration/3-2-benchmark-baseline.md` に `Vec` 仕様反映済みであることを脚注し、監査チーム・Rust 移植チームの両方に合図を送る。

##### 実施ログ（2027-03-29）
- `CoreVec`/`EffectfulVec` を `compiler/rust/runtime/src/collections/mutable/vec.rs` へ実装し、`push` などの可変 API で `EffectSet::mark_mut()`、コピー系 API で `record_mem_bytes()` を発火させる構造を整備した。【I:../../compiler/rust/runtime/src/collections/mutable/vec.rs†L1-L210】
- `compiler/rust/runtime/src/collections/mod.rs` に `mutable` モジュールを追加し、Rust ランタイムから `CoreVec` 等を公開できる状態にした。【I:../../compiler/rust/runtime/src/collections/mod.rs†L1-L7】
- `cargo test --manifest-path compiler/rust/frontend/Cargo.toml core_iter_adapters_snapshot` を再実行し、`compiler/rust/frontend/tests/snapshots/core_iter_adapters__core_iter_adapters.snap` を更新して `EffectLabels` のシリアライズ順序変化を確認した。【I:../../compiler/rust/frontend/tests/snapshots/core_iter_adapters__core_iter_adapters.snap†L1-L40】
- `scripts/validate-diagnostic-json.sh` に `--suite collectors` を追加し、`reports/spec-audit/ch1/core_iter_collectors.json` を既定対象として `collector.effect.mut`/`collector.effect.mem` の欠落を検出できる検証ルートを作成した。【I:../../scripts/validate-diagnostic-json.sh†L1-L120】

##### 実施タスクリスト（39週目前半）
1. **API 実装と Collector 連携**: `runtime/src/collections/mutable/vec.rs` と `compiler/rust/runtime/src/prelude/collectors/vec.rs` を同時更新し、`CoreVec<T>`／`EffectfulVec<T>` の切り替えを実装。所要 2 日、担当 = Runtime チーム (`@runtime-rs`)、レビュー = `@prelude-core`。【F:../../compiler/rust/runtime/src/collections/mutable/vec.rs†L1-L200】
2. **効果トレースと監査ログ**: `CollectorAuditTrail` と `AuditEnvelope.metadata` に `collector.effect.mut`/`mem` を載せる。`reports/spec-audit/ch1/core_iter_collectors.json` の snapshot 更新と `scripts/validate-diagnostic-json.sh` へのキー追加を同じコミットで実施。所要 1 日、担当 = Diagnostics (`@diag-core`)。【F:../../reports/spec-audit/ch1/core_iter_collectors.json†L1-L120】
3. **KPI・ベンチ反映**: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ `vec_mut_ops_per_sec` を追記し、`assets/metrics/core_collections_persistent.csv` の列追加を行う。`reports/iterator-collector-summary.md` に KPI を写経。所要 0.5 日、担当 = Metrics (`@audit-metrics`)。【F:0-3-audit-and-metrics.md†L1-L150】
4. **CI 統合**: `tooling/ci/collect-iterator-audit-metrics.py` と `scripts/validate-diagnostic-json.sh` に Vec シナリオを組み込み、`cargo test core_collections_vec` を `ci.yml` へ追加。所要 0.5 日、担当 = Tooling (`@tooling-ci`)。【F:../../tooling/ci/collect-iterator-audit-metrics.py†L1-L160】【F:../../scripts/validate-diagnostic-json.sh†L1-L140】

#### 3.2 `Cell<T>` / `Ref<T>` 内部可変性モデル
- `Cell<T>` は `Copy` 制約付きの軽量内部可変性として `runtime/src/collections/mutable/cell.rs` に実装し、`EffectSet::mark_cell()` を `new_cell`/`set` で呼び出す。`Core.Diagnostics.ChangeTrace` の `collector.effect.cell` を `AuditEnvelope.metadata` に書き出し、`reports/iterator-collector-summary.md` の KPI テーブルへ `cell_mutations_total` を追加する。【F:../../spec/3-2-core-collections.md†L118-L135】
- `Ref<T>` は `Arc<RefInner<T>>` + `parking_lot::RwLock` で実装し、`clone_ref`/`borrow_mut` 時に `EffectSet::mark_rc()` および `mark_mut()` を付与する。`Core.Async/FFI` 章で要求される参照カウント契約（`docs/spec/3-9-core-async-ffi-unsafe.md` §4）と整合するよう、`runtime/src/collections/mutable/ref.rs` で `RuntimeBridge` 用の `RefHandle` を定義し、`poc_dualwrite_compare.sh` の `--section ref_count` で OCaml 版との差分を計測する。【F:../../spec/3-2-core-collections.md†L96-L136】
- 効果伝播の検証として `compiler/rust/runtime/tests/core_collections_cell_ref.rs` を新設し、(1) `Cell.set` 呼び出し後に `collector.effect.cell=true` になる、(2) `Ref.borrow_mut` が `collector.effect.rc=true` と `mut=true` を両方立てる、(3) 二重借用が `CollectError::BorrowConflict` を返して `Diagnostic::effect_violation` に落ちる、を確認する。`scripts/validate-diagnostic-json.sh` に `collector.effect.cell`/`collector.effect.rc` の必須キーを追加し、`collect-iterator-audit-metrics --require-cell` オプションで CI ゲート化する。
- `docs-migrations.log` と `docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md` に `Cell/Ref effect trace` の更新を記録し、監査チームが `Core.Diagnostics` へ新規メタデータを導入するタイミングを共有する。

##### 実施ログ（2027-03-29）
- `EffectfulCell`/`EffectfulRef` を `compiler/rust/runtime/src/collections/mutable/{cell,ref}.rs` に実装し、`UnsafeCell` と `Arc<RwLock<T>>` から `EffectSet` を直接更新する仕組みを構築した。【I:../../compiler/rust/runtime/src/collections/mutable/cell.rs†L1-L133】【I:../../compiler/rust/runtime/src/collections/mutable/ref.rs†L1-L210】
- `Ref` の `Debug` 表示に `strong_count` を露出させ、監査ログで参照カウント変化と `effect {rc}` を確認できるようにした。【I:../../compiler/rust/runtime/src/collections/mutable/ref.rs†L89-L95】
- `cargo test --manifest-path compiler/rust/frontend/Cargo.toml` を実行し、Mutable コレクション追加後も 32 テスト（`diagnostic::formatter` 等）が完走することを確認。結果は `reports/spec-audit/ch0/links.md#core-collections-mutable-20270329` へ追記した。
- `reports/iterator-collector-summary.md` に `cell_mutations_total` / `ref_borrow_conflict_rate` KPI と `collect_cell_ref_effects` セクションを追加し、`collect-iterator-audit-metrics.py --suite collectors --scenario ref_internal_mutation` で対照確認できるようにした。

##### 実装アウトライン
1. `Cell<T>` は `UnsafeCell<T>` を内包した `#[repr(transparent)] struct Cell<T>` とし、`impl<T: Copy>` で `new`, `get`, `set`, `replace` を提供。`set`/`replace` は `EffectSet::mark_cell()` を呼び、`get` は `@pure`。`docs/spec/1-3-effects-safety.md` の `cell` 章へ API リストを反映する。
2. `Ref<T>` は `Arc<RefInner<T>>` に `RwLock<T>`・`AtomicUsize active_mut_borrows`・`EffectLabels` を持たせる。`borrow_mut` は `EffectSet::mark_rc()` + `mark_mut()`、`borrow` は `mark_rc()` のみ。`clone_ref` は `EffectSet::mark_rc()` を呼び、`Drop` 時に `EffectSet::release_rc()`（新設）で監査ログへ `rc.dec` を送る。
3. FFI/RuntimeBridge 経路は `compiler/rust/runtime/ffi/src/handles/ref_handle.rs` に分離し、`docs/guides/runtime-bridges.md` の契約に従い `RefHandle::clone`/`drop` の監査を `CapabilityRegistry` へ記録する。`docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` へ参照を追記する。
4. `reports/iterator-collector-summary.md` では `cell_mutations_total` / `ref_borrow_conflicts` の KPI を追加し、`docs/notes/core-library-outline.md` に内部可変性レイヤのメモを残す。`docs/plans/rust-migration/p2-spec-compliance-gap.md` にも `Cell/Ref` の差分を写す。

###### 3.2.1 Cell 効果トレースの整備
- `EffectSet` に `CELL_BIT` を追加し、`EffectLabels` に `cell: bool` フィールドを拡張して `collector.effect.cell` メタデータを扱えるようにする。`set`/`replace`/`clone` の各パスで `EffectSet::mark_cell()` を呼び出し、`CollectorAuditTrail::record_cell_op`（新設）で `mem_bytes` などの付帯情報と併せて `AuditEnvelope.metadata` に展開する。【F:../../compiler/rust/runtime/src/prelude/iter/mod.rs†L680-L880】
- `EffectfulCell` の `new_cell` 相当のコンストラクタと `set`/`replace` に `EffectSet::mark_mut()` とのダブル記録を組み込み、`core.collections.cell` 名前空間へ公開する `Collector` 側と `Iter` 側の API を一致させる。`reports/spec-audit/ch1/core_iter_collectors.json` の `collector.effect.cell` を `scripts/validate-diagnostic-json.sh --pattern collector.effect.cell` で固定し、`collect-iterator-audit-metrics.py --suite collectors --scenario cell_internal_mutation` を `--require-cell` 付きで CI へ追加する。
- KPI は `reports/iterator-collector-summary.md` の `cell_mutations_total` 列に記録し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` と `docs/plans/bootstrap-roadmap/assets/metrics/core_collections_persistent.csv` の `cell_mutations_total` 行へ参照を伸ばす。`docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md` に `cell` 効果のキーを追記し、診断チームと共有する。

###### 3.2.2 Ref 効果と参照カウント契約
- `EffectSet` に `RC_BIT` を加え、`EffectSet::mark_rc()` / `EffectSet::release_rc()` を導入して `clone_ref`/`borrow`/`borrow_mut`/`drop` で `collector.effect.rc` の上下を追跡する。`EffectLabels` には `rc: bool` を追加したうえで `EffectfulRef::effect_labels` で `collector.effect.rc` を返すようにし、`Diagnostic`/`AuditEnvelope` に `ref.borrow_rcs` カウントを残す。【F:../../compiler/rust/runtime/src/prelude/iter/mod.rs†L680-L900】
- `Ref` 本体は `Arc<RefInner<T>>` + `parking_lot::RwLock`（又は `std::sync::RwLock`）で実装し、`borrow_mut` の成功時に `EffectSet::mark_mut()` を併用する。`CollectError::BorrowConflict` の戻りは `Diagnostic::effect_violation` に変換されるよう `prelude::collectors::ref` 経路で制御し、`scripts/validate-diagnostic-json.sh --pattern collector.effect.rc` で `collector.effect.rc=true` が 1 件以上得られることを保証する。
- ベンチマーク側では `collect-iterator-audit-metrics.py --suite collectors --scenario ref_internal_mutation` に `collector.effect.rc`/`collector.effect.mut` の成功判定を置き、`reports/iterator-collector-summary.md` の `ref_borrow_conflict_rate` / `ref_rc_clone_ops` セクションへデータを流す。`docs/notes/core-library-outline.md#ref-internal-mutation` に挙動メモを追加し、`docs/plans/rust-migration/p2-spec-compliance-gap.md` と `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md` で差分のトラッキング行を同期する。

###### 3.2.3 RuntimeBridge と FFI ハンドル
- `compiler/rust/runtime/ffi/src/handles/ref_handle.rs` に `RefHandle` を分離し、`CapabilityRegistry` の `core.collections.ref` エントリを登録する。`RefHandle::clone`/`RefHandle::drop` で `EffectSet::mark_rc()`/`release_rc()` を呼び出し、`docs/guides/runtime-bridges.md` の `Ref` 契約と `docs/spec/3-9-core-async-ffi-unsafe.md` §4 の `async` 参照ライフタイム要件を満たすようにする。
- `poc_dualwrite_compare.sh --section ref_count` と `reports/spec-audit/diffs/README.md` へ `collector.effect.rc` に関するログを残し、OCaml/ Rust の差分が 5% を超えた場合は `docs/plans/bootstrap-roadmap/4-0-phase4-migration.md` へリスク追加を作成する。`docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` の `CapabilityRegistry` を更新し、`core.collections.ref` が `CapabilityStage::Persistent`/`RefHandle` によるアクセス保証を報告するようにする。
- **2027-05-30 Update**: `compiler/rust/runtime/ffi/src/handles/ref_handle.rs`/`handles/mod.rs` と `compiler/rust/runtime/ffi/src/lib.rs` で `RefHandle` の再公開と `register_ref_capability()` の橋渡しを実装し、`compiler/rust/runtime/ffi/src/capability_handle.rs` に `RefCapability` variant を追加して `core.collections.ref` を `CapabilityRegistry` に登録した。ドキュメント側では `docs/guides/runtime-bridges.md` に `RefHandle` の契約と `collector.effect.rc` への対応を、`docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` に新たなステージ連携行を追記して進捗を記録した。

###### 3.2.4 ドキュメント・監査連携
- `reports/iterator-collector-summary.md` の `collect_cell_ref_effects` セクションに `cell_mutations_total` / `ref_borrow_conflict_rate` / `collector.effect.rc` の説明を追記し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 表および `docs/plans/bootstrap-roadmap/assets/metrics/core_collections_persistent.csv` の `RefInternalMutation` 行で新規メトリクスを追跡する。`docs/notes/core-library-outline.md` に内部可変性スタックのメモを立て、`docs/plans/rust-migration/p2-spec-compliance-gap.md` の該当セクションとリンクさせて仕様整合性の監査ルートを明示する。
- `scripts/validate-diagnostic-json.sh --suite collectors --pattern collector.effect.cell --pattern collector.effect.rc` により `collector.effect.cell`/`collector.effect.rc` の存在を gate として強制し、`collect-iterator-audit-metrics.py --suite collectors --scenario ref_internal_mutation --require-success --require-cell` で `collector.effect.cell`/`collector.effect.rc` を出力したケースを検証する。CI や週次レビューでは `reports/iterator-collector-metrics.json` を生成して `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に KPI を転記し、`docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md` §8 の `Core Collections effect 連携` から監査チームへ通知する。

##### 検証ポイント
- `cargo test core_collections_cell_ref` を CI に組み込み、`tooling/ci/collect-iterator-audit-metrics.py --scenario ref_internal_mutation --require-cell` で effect を監視。
- `scripts/poc_dualwrite_compare.sh --section ref_count` の出力ログを `reports/spec-audit/diffs/README.md` に保存し、Rust/OCaml の borrow 契約差分を追跡。
- `docs-migrations.log` に `Cell/Ref effect trace` 追記を残し、`docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md` での監査項目追加とリンクさせる。
- `reports/spec-audit/ch1/core_iter_collectors.json`/`.audit.jsonl` に `collector.effect.rc_ops` を含め、`scripts/validate-diagnostic-json.sh --suite collectors --pattern collector.effect.rc_ops` で `Ref` 操作の `collector.effect.rc`/`rc_ops` を確認するルートを整備する。

##### 実施タスクリスト（39週目中盤）
1. **`Cell` 実装と effect 追跡**: `runtime/src/collections/mutable/cell.rs` に `Cell<T>` API を移植し、`EffectSet::mark_cell()` を装着。`docs/spec/1-3-effects-safety.md` の `cell` 節へ参照を追加。担当 = Runtime (`@runtime-rs`)、所要 1 日。【F:../../compiler/rust/runtime/src/collections/mutable/cell.rs†L1-L160】
2. **`Ref`/`RefHandle` パイプライン**: `runtime/src/collections/mutable/ref.rs` と `runtime/ffi/src/handles/ref_handle.rs` を同時実装し、`Arc<RefInner<T>>` + `RwLock` + effect 計測を整備。`Core.Async/FFI` 仕様 (`docs/spec/3-9-core-async-ffi-unsafe.md`) へのリンク注記を追加。担当 = Runtime + FFI (`@runtime-rs`, `@ffi-bridge`)、所要 2 日。【F:../../compiler/rust/runtime/src/collections/mutable/ref.rs†L1-L220】
3. **テストと dual-write**: `compiler/rust/runtime/tests/core_collections_cell_ref.rs` で効果と Borrow 再現を検証し、`scripts/poc_dualwrite_compare.sh --section ref_count` で OCaml 版と比較。担当 = Testing (`@prelude-core`)、所要 1 日。【F:../../compiler/rust/runtime/tests/core_collections_cell_ref.rs†L1-L200】
4. **監査・ドキュメント同期**: `reports/iterator-collector-summary.md` に KPI を追加し、`docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md`/`docs-migrations.log` に effect メタデータ導入を記録。担当 = Diagnostics (`@diag-core`)、所要 0.5 日。【F:../../reports/iterator-collector-summary.md†L1-L80】

##### 実施ログ（2027-04-23）
- `EffectSet` に `rc_ops` を追加して `EffectLabels` を拡張し、`CollectorAuditTrail` の `extension_payload`/`audit_metadata` から `collector.effect.rc_ops` を出力するように整備した。これで `collector.effect.cell`/`collector.effect.rc` も JSON/Audit に残るようになった。【F:../../compiler/rust/runtime/src/prelude/iter/mod.rs†L678-L847】【F:../../compiler/rust/runtime/src/prelude/collectors/mod.rs†L221-L380】
- `EffectfulRef` に `with_effects` ヘルパーを導入し、clone/borrow/borrow_mut/drop の各経路で `EffectSet::mark_rc()`（必要に応じて `mark_mut()`）を打刻することで `collector.effect.rc_ops`/`collector.effect.rc` を `Ref` 操作が一貫して記録するようにした。【F:../../compiler/rust/runtime/src/collections/mutable/ref.rs†L156-L220】

#### 3.3 `Table<K,V>` の順序保持と IO 連携
- `Table` の挿入順序保持要件に従い、`runtime/src/collections/mutable/table.rs` で `VecDeque<(K,V)>` + `DeterministicHasher` を組み合わせたロジックを実装する。`insert`/`remove` は `EffectSet::mark_mut()` を必ずセットし、`map_to_table`/`table_to_map` 変換では `effect {mem}` を `EffectLabels` に記録する。【F:../../spec/3-2-core-collections.md†L138-L200】
- `TableCollector`（`compiler/rust/runtime/src/prelude/collectors/table.rs`）を拡張し、`CollectError::DuplicateKey` と `CollectError::UnstableOrder` を追加で返せるようにする。`Iter.collect_table`（新設）では重複キーを `Diagnostic::collector_duplicate_key` へ変換し、`reports/spec-audit/ch1/core_iter_collectors.json` に期待される effect/diagnostic を追記する。
- `Table.load_csv` は `Core.IO`/`Core.Text` と結合し、`docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` で定義されている CSV リーダ (`Core.IO.CsvReader`) を再利用する。`effect {io}` と `effect {mut}` の複合を `EffectSet::mark_io()` + `mark_mut()` で記録し、`RuntimeBridge` から `Capability Stage` チェック（`docs/spec/3-8-core-runtime-capability.md` §10）を通過するよう `CapabilityRegistry` へ `core.collections.table.csv_load` を登録する。
- テスト計画: `compiler/rust/runtime/tests/core_collections_table.rs` に (1) 挿入順が保持される property テスト、(2) `table_to_map` がキー昇順へソートされること、(3) `load_csv` が `Core.Diagnostics.ChangeTrace` にファイルパスと effect 情報を記録すること、を追加。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ `table_insert_throughput` と `csv_load_latency` のメトリクスを記載し、Phase 3 の `Go/No-Go` ゲートで比較する。

##### 実装アウトライン
1. `runtime/src/collections/mutable/table.rs` に `struct TableInner<K,V>` を作り、`VecDeque<EntryId>` と `IndexMap<K, (EntryId, V)>` を併用する。`EntryId(u64)` は挿入順序を保持し、`insert` 時に `EffectSet::mark_mut()` を、`map_to_table`/`table_to_map` 時に `EffectSet::mark_mem(bytes)` を呼ぶ。
2. `TableCollector` は `CollectOutcome` の `effects` に `collector.effect.mut`/`collector.effect.mem`/`collector.effect.audit` を書き込み、`Iter.collect_table` / `Iter.try_collect(TableCollector)` のパスで `EffectSet::merge_from_collector` を実行する。`CollectError::UnstableOrder` は `Docker` ベースの CSV fixture で再現し、`Diagnostic::collector_unstable_order` として `docs/spec/3-6-core-diagnostics-audit.md` に追加する。
3. CSV 連携は `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` の `Core.IO.CsvReader` を呼び出して `Vec<Column>` を生成し、UTF-8/BOM/CRLF 検証を `docs/spec/3-3-core-text-unicode.md` の `GraphemeCursor` 実装へ委譲する。`CapabilityRegistry` の `core.collections.table.csv_load` エントリは `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` に記す。
4. `docs/notes/core-library-outline.md` と `docs/plans/rust-migration/3-2-benchmark-baseline.md` へ `table_ordered_insert_ps` / `csv_load_latency` の KPI を追加し、IO チームと共有する。`docs-migrations.log` へ `Table.load_csv` 追加を追記し、再編履歴を残す。

##### 検証・フォローアップ
- `cargo test core_collections_table core_iter_try_collect` を Phase3 self-host 判定基準へ加え、`tooling/ci/collect-iterator-audit-metrics.py --scenario table_csv_import --require-audit` で自動検証する。
- `scripts/validate-diagnostic-json.sh --section core_collections_table` を整備し、`reports/spec-audit/ch1/core_iter_collectors.json` / `.audit.jsonl` に `collector.effect.mut=true` `collector.effect.mem=true` `collector.effect.audit=true` が揃っているかチェックする。
- `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` と `reports/spec-audit/ch3/README.md` に `Table.load_csv` の依存・成果をリンクし、IO/Diagnostics/Collections 三者でリスク共有する。

##### 実施ログ（2027-03-29）
- `EffectfulTable` と `Table<K,V>` を `compiler/rust/runtime/src/collections/mutable/table.rs` に追加し、挿入順序・`BTreeMap` インデックスを併用した順序保持ロジックと `EffectSet` 連携を実装した。【I:../../compiler/rust/runtime/src/collections/mutable/table.rs†L1-L224】
- `scripts/validate-diagnostic-json.sh --suite collectors` を利用して `reports/spec-audit/ch1/core_iter_collectors.json` の `collector.effect.mem` ケースを再検証し、`collect_table_csv` シナリオ用 KPI (`table_insert_throughput`, `csv_load_latency`) を `reports/iterator-collector-summary.md` へ追加した。
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に上記 KPI を登録し、Phase3 self-host 判定の指標表へ反映した。

##### 実施タスクリスト（39週目後半）
1. **`Table` コア実装**: `runtime/src/collections/mutable/table.rs` に `TableInner` を実装し、`VecDeque<EntryId>` + `IndexMap` で順序保持を実現。`EffectSet::mark_mut()`/`mark_mem()`/`mark_audit()` を各操作に付与。担当 = Runtime (`@runtime-rs`)、所要 2 日。【F:../../compiler/rust/runtime/src/collections/mutable/table.rs†L1-L260】
2. **Collector/Iter/Diagnostics 更新**: `compiler/rust/runtime/src/prelude/collectors/table.rs` と `prelude/iter/mod.rs` を更新し、`CollectError::UnstableOrder`、`Iter.collect_table`、`iter.collector.effect.*` を整備。`reports/spec-audit/ch1/core_iter_collectors.json` / `.audit.jsonl` の snapshot で effect/diagnostic を記録。担当 = Prelude + Diagnostics (`@prelude-core`, `@diag-core`)、所要 1.5 日。 
3. **CSV 連携・Capability**: `Core.IO.CsvReader` fixture（`docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md`）を利用し `Table.load_csv` を実装。`CapabilityRegistry` に `core.collections.table.csv_load` を登録し、`docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` と同期。担当 = IO + Runtime (`@io-path`, `@runtime-rs`)、所要 1 日。【F:../../docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md†L80-L210】
    4. **CI/メトリクス統合**: `tooling/ci/collect-iterator-audit-metrics.py --scenario table_csv_import`、`scripts/validate-diagnostic-json.sh --section core_collections_table` を追加し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `table_insert_throughput`/`csv_load_latency` を登録。担当 = Tooling + Metrics (`@tooling-ci`, `@audit-metrics`)、所要 0.5 日。【F:../../tooling/ci/collect-iterator-audit-metrics.py†L160-L260】【F:0-3-audit-and-metrics.md†L150-L260】

- **2027-06-01 Update**: `collections::mutable::Table` を `VecDeque` + `DeterministicHasher` ベースで再実装し、`EffectfulTable` の効果記録と `TableCollector` による `collector.effect.*` 連携 (`mem_bytes`, `mut` フラグ) を再構成した上で `Iter.collect_table` を追加し、`CollectErrorKind::UnstableOrder` でフィードバックチャンネルを拡張した。`compiler/rust/runtime/ffi/Cargo.toml` に `indexmap` 依存を追加し、仕様に忠実な順序保証と Audit 用メタデータの橋渡しを整えた。【F:../../compiler/rust/runtime/src/collections/mutable/table.rs†L1-L220】【F:../../compiler/rust/runtime/src/prelude/collectors/table.rs†L1-L80】【F:../../compiler/rust/runtime/src/prelude/iter/mod.rs†L120-L220】【F:../../compiler/rust/runtime/src/prelude/collectors/mod.rs†L500-L540】【F:../../compiler/rust/runtime/ffi/Cargo.toml†L1-L20】

### 4. Iter/Collector 相互運用（39-40週目）
**担当領域**: 遅延列との結合

4.1. `Iter` から各コレクションへ変換する API (`collect_list`, `collect_vec`, `Map.from_iter` 等) を実装し、重複キー検出やエラーハンドリングを確認する。
4.2. `IntoIter` 実装を整備し、`Iter` と永続構造の往復変換で所有権が崩れないことをテストする。
4.3. `Collector` 実装と `Iter.try_collect` の統合を検証し、失敗時の `CollectError` が `Diagnostic` に落とし込まれるか確認する。

#### 4.1 `Iter.collect_*` API の実装・効果管理
- `compiler/rust/runtime/src/prelude/iter/mod.rs` に `collect_map`/`collect_set`/`collect_table`/`collect_cell_seq` を追加し、既存の `collect_list`/`collect_vec` と同じインターフェイスで Collector を差し替えられるようにする。各終端 API の効果タグは仕様 (Chapter 3.1 §3.4) に合わせ、`collect_list`/`collect_map` は `@pure`、`collect_vec`/`collect_table` は `effect {mut}`、`collect_table`/`collect_vec` の内部コピーでは `effect {mem}` も必須とする。【F:../../spec/3-1-core-prelude-iteration.md†L137-L196】
- `CollectorKind` 列挙（`compiler/rust/runtime/src/prelude/iter/collector.rs`）へ `List`, `Vec`, `Map`, `Set`, `Table` を定義し、`EffectLabels` を `CollectorAuditTrail` へ転写する。`iter.collect_*` の実行パスで `EffectSet::merge_from_collector(kind.effect_labels())` を呼び出し、`Iter` 側からでも `collector.effect.*` を観測できるよう `Diagnostic.extensions["iter.collector.effect.*"]` を追加する。
- CI フロー: `tooling/ci/collect-iterator-audit-metrics.py --section collectors --scenario iter_collectors` を新設し、`collect_list`（`@pure`）と `collect_vec`（`effect {mut,mem}`）が正しいタグを出力しているか比較する。`reports/iterator-collector-summary.md` の `Iter.collectors` テーブルへ `effect_check` 列を追加し、`scripts/validate-diagnostic-json.sh` の `ITER_COLLECTOR_REQUIRED_KEYS` に `iter.collector.kind`/`iter.collector.effect.*` を追記する。
- テスト: `compiler/rust/runtime/tests/core_iter_collectors.rs` を拡張して (1) `collect_map` が重複キーに対して `CollectError::DuplicateKey` を返す、(2) `collect_table` が `collector.effect.mut=true` を報告する、(3) `collect_list` が stage `IterStage::Stable` を保持する、の 3 ケースを snapshot。OCaml 版との dual-write 比較は `scripts/poc_dualwrite_compare.sh --target iter_collectors` で自動実行し、`reports/spec-audit/ch1/core_iter_collectors.json` に結果を格納する。

##### 実施ステップ
1. `CollectorKind` と関連 `EffectLabels` の拡張を `Iter`/`Collector` モジュールに導入し、各 `collect_*` パスから `AuditEnvelope.metadata` に `collector.effect.*` を書き込む仕組みを入れる。`Iter.collect_map`/`collect_set` では `@pure` を維持しつつ `TryCollectError` との整合性を確認する。
2. `Iterator::collect_vec`/`collect_table` で `effect {mut}` と `effect {mem}` を記録するため、Collector 側で `GrowthBudget` を参照し `mem_bytes` を `CollectOutcome` に転写。`list_as_vec_mem_bytes` の KPI との整合性を保つため `reports/iterator-collector-summary.md` の `Collectors` セクションに新規行を追加する。
3. CI スクリプト（`collect-iterator-audit-metrics.py`、`validate-diagnostic-json.sh`）に `iter.collector.effect.*` チェックを組み込み、`reports/spec-audit/ch1/core_iter_collectors.json` に snapshot を追記する。`docs/notes/core-library-outline.md` に dual-write プロセスの結果を記録し、検証ログの再現手順を残す。

#### 4.2 永続コレクションの `IntoIter` と Stage 監査
- `runtime/src/collections/persistent/list.rs` / `btree.rs` に `impl IntoIterator for List/Map/Set` を実装し、`IntoIter` が `IterStage::Stable` を携えた `Iter` を返すよう `Iter::from_persistent`（新設）を介して統一する。`map_to_iter` など従来の ad-hoc 実装は `IntoIter` ベースに置き換え、所有権移動時に `Arc`/`PersistentArena` の参照カウントが正しく減少するか追跡する。【F:../../spec/3-2-core-collections.md†L21-L89】
- `Effect` 的には `IntoIter` は `@pure` のままにし、`CollectorKind` 情報を `IteratorDictInfo.stage` と `AuditEnvelope.metadata["iter.stage.kind"]` へ記録する。`compiler/rust/runtime/tests/core_collections_into_iter.rs` を新設し、`List::into_iter`→`collect_vec` の往復で参照が再利用されること、`Map::into_iter` がキー昇順を保つこと、`Set::into_iter` が `effect.stage.iterator.exact=Stable` を出力することを検証する。
- CI 側では `tooling/ci/sync-iterator-audit.sh` に `--verify-stage CoreCollections` パラメータを追加し、`reports/spec-audit/ch0/links.md` 内の DSL サンプルを再生して Stage メタデータが `effect.stage.iterator.*` に現れているか自動確認する。結果サマリは `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の Stage 行へ追記する。

##### 実施ステップ
1. `List`/`Map`/`Set` の `IntoIterator` 実装を `runtime/src/collections/persistent` に統合し、`Iter::from_persistent` を介して `Iter` stage 情報を伴わせた `AuditEnvelope` を返すよう統一する。各 `IntoIter` は `PersistentArena` の `Arc` を適切に解放するユニットテストを含む。
2. `IteratorDictInfo.stage` を更新するアダプタを `compiler/rust/runtime/src/prelude/iter/stage.rs` へ追加し、`Map`/`Set` の `IntoIter` で `IterStage::Stable` を保つことを検証する。`reports/iterator-collector-summary.md` では `IntoIter` 周りは `Collectors` より `IterStages` セクションへ分離し、Stage メタデータの KPI を記載する。
3. `tooling/ci/sync-iterator-audit.sh --verify-stage CoreCollections` を `reports/iterator-collector-summary.md` の Stage 検証フローに追加し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の Stage チェックリストに `iter.stage.kind` の期待値を列挙する。

#### 4.3 `Collector` × `Iter.try_collect` の診断統合
- `Iter.try_collect` の `Result` 伝播と `Collector::Error` を接続するため、`compiler/rust/runtime/src/prelude/iter/try_collect.rs`（新設）で `CollectorBridge<E>` を実装する。`CollectError` から `Diagnostic` へのマッピングは `Core.Diagnostics` で既に使用している `IntoDiagnostic` トレイトを利用し、`EffectSet` に `collector.effect.audit` を転写して監査ログと整合させる。【F:../../spec/3-1-core-prelude-iteration.md†L149-L190】【F:../../spec/3-6-core-diagnostics-audit.md†L42-L120】
- `Collector` 実装ごとに `push`/`reserve`/`finish` の `effect` ラベルを列挙し、`Iter.try_collect` が `collector.effect.*` を `AuditEnvelope.metadata` に書き込む。`reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` へ `try_collect` 由来のサンプル（成功・失敗）を追加し、`scripts/validate-diagnostic-json.sh --section iter` 実行時に `iter.try_collect.diagnostic` を検証する。
- テスト: `compiler/rust/runtime/tests/core_iter_try_collect.rs` を追加し、(1) `Iter<Result<T, Diagnostic>>.try_collect(VecCollector)` が最初の `Err` で短絡し effect を保持する、(2) `Iter.try_collect(MapCollector)` が `CollectError::DuplicateKey` を `Diagnostic::collector_duplicate_key` に変換する、(3) `Iter.try_collect(TableCollector)` が `collector.effect.mut=true`/`collector.effect.mem=true` を記録する、の 3 ケースを確認する。CI では `cargo test core_iter_try_collect` を `tooling/ci/collect-iterator-audit-metrics.py --run-tests` から呼び出し、失敗時に `reports/iterator-collector-summary.md` の `status` カラムを更新する。

##### 実施ステップ
1. `CollectorBridge` を `iter/try_collect.rs` に定義し、`Collector::finish` で得られる `CollectOutcome` を `EffectSet` に反映させて `AuditEnvelope` へ渡す。`CollectError` 別の `Diagnostic` 変換マッピング表を `docs/spec/3-6-core-diagnostics-audit.md` に準拠して記録し、`Diagnostic.extensions["collector.error.kind"]` を追加する。
2. 各 `Collector` の `try_collect` UI を `compiler/rust/runtime/src/prelude/collectors` 側で拡張してエラーの `effect` ラベルと `GrowthBudget` を `CollectOutcome` へ書き出す。`CollectOutcome::audit` を `Core.Diagnostics` へ橋渡す `AuditChangeBridge` の雛形を `docs/plans/bootstrap-roadmap/5-core-diagnostics-config-plan.md` にメモしておき、メタデータの一貫性を保つ。
3. `reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` へ `try_collect` の成功/失敗ケースを追加し、`scripts/validate-diagnostic-json.sh --section iter` で `iter.try_collect.diagnostic` をチェック。`docs/notes/core-library-outline.md` に `Collector` 失敗ケースのリストを TODO 頭で残し、追跡用の検証手順を示す。

##### 実施ログ（2027-06-10 Update）
- `compiler/rust/runtime/src/prelude/iter/try_collect.rs` を草案として作成し、`CollectorBridge` に `CollectOutcome` の `EffectLabels` を `EffectSet` にマージするパスと `AuditEnvelope.metadata["collector.effect.*"]` の付与処理を記述した。`CollectError` から `Diagnostic` への変換は `docs/spec/3-6-core-diagnostics-audit.md` の `DiagnosticKind` 定義を参照してテーブル化し、必要な拡張ポイントを TODO として注記した。【F:../../compiler/rust/runtime/src/prelude/iter/try_collect.rs†L1-L120】【F:../../docs/spec/3-6-core-diagnostics-audit.md†L42-L120】
- `MapCollector`/`VecCollector`/`TableCollector` の `CollectOutcome` 側で `growth_budget.mut_bytes` や `collector.effect.audit` を設定し、`reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` の `try_collect_collectors` シナリオで `collector.effect.mut`/`collector.effect.mem` を期待値に追加。`scripts/validate-diagnostic-json.sh --section iter` では `iter.try_collect.diagnostic` のキーを必須にし、失敗ケースを即座に検出できるようにした。【F:../../reports/spec-audit/ch1/core_iter_collectors.audit.jsonl†L1-L120】【F:../../scripts/validate-diagnostic-json.sh†L1-L160】
- `docs/notes/core-library-outline.md` に `Iter.try_collect` の失敗パターン一覧を TODO として残し、OCaml 実装との dual-write 比較手順（`scripts/poc_dualwrite_compare.sh --target iter_try_collect`）と、`reports/iterator-collector-summary.md` の `Iter.collectors` 表に記録する `status` カラム更新ルールをメモした。【F:../../docs/notes/core-library-outline.md†L1-L120】【F:../../reports/iterator-collector-summary.md†L15-L34】

##### 実施タスクリスト
1. `Collector` 側で `push`/`reserve`/`finish` 項目に `collector.effect.*` フラグと `growth_budget` 情報を書き込む。`compiler/rust/runtime/src/prelude/collectors/{list,vec,map,set,table}.rs` を更新し、`CollectOutcome::audit` に必要なメタデータを詰め込む。【F:../../compiler/rust/runtime/src/prelude/collectors/list.rs†L1-L160】【F:../../compiler/rust/runtime/src/prelude/collectors/vec.rs†L1-L120】
2. `CollectorBridge` を `Iter.try_collect` へ接続し、`CollectError` を `Diagnostic::collector_*` に写像するマッピング表を `docs/spec/3-6-core-diagnostics-audit.md` へ付記。`CollectOutcome` から `AuditEnvelope` へ `collector.effect.audit` を伝搬し、`Diagnostic.extensions["collector.error.kind"]` に値を保持する変換パイプラインを整備する。【F:../../docs/spec/3-6-core-diagnostics-audit.md†L42-L120】
3. `scripts/validate-diagnostic-json.sh --section iter` と `tooling/ci/collect-iterator-audit-metrics.py --section collectors` に `iter.try_collect` の成功/失敗ケースを追加し、`reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` で `collector.effect.*` を検証。CI 失敗時は `reports/iterator-collector-summary.md` の `status` カラムを更新するルールも同時に定義する。【F:../../tooling/ci/collect-iterator-audit-metrics.py†L1-L260】【F:../../reports/spec-audit/ch1/core_iter_collectors.audit.jsonl†L1-L120】

### 5. Diagnostics / Config / Audit 連携（40週目）
**担当領域**: 他章との統合

5.1. `Core.Diagnostics` の `AuditEnvelope.change_set` と連携するための JSON 差分ユーティリティを実装し、`Map`/`Table` の変換を提供する。
5.2. Config/Data 章 (3-7) で利用する差分 API (`SchemaDiff`, `Change`) との互換アダプタを用意し、双方向変換テストを実施する。
5.3. `effect {audit}` を伴う操作 (`emit_metric` 等) の前提条件を確認し、Capability チェックのフックを追加する。

#### 5.1 `AuditEnvelope.change_set` とコレクションの JSON 差分
- `compiler/rust/runtime/src/collections/audit_change_bridge.rs` などに `AuditChangeBridge` モジュールを追加し、`CollectOutcome::collector.effect.audit=true` の際に `Map`/`Table` の差分を `AuditEnvelope.change_set` に構築するロジックを実装する。差分は `docs/spec/3-6-core-diagnostics-audit.md` の `ChangeSet`/`SchemaDiff` フォーマットを再利用し、Collector 側で算出した `added`/`removed`/`updated` エントリを JSON 変換する。
- JSON 出力は `reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` の `collections.diff` セクションと整合させ、`scripts/validate-diagnostic-json.sh --section core_iter_collectors.audit --pattern collections.diff` で `key`/`kind`/`value` が揃っていることを確認する。単体テストは `compiler/rust/runtime/tests/core_collections_audit_bridge.rs` で `PersistentMap` の `merge`/`diff` と `Table` の `insert`/`remove` を再現し、`AuditEnvelope.change_set.total` が仕様通り出力されることと `effect {audit}` のフラグが `true` になることを検証する。
- `reports/iterator-collector-summary.md` に `audit_bridge` の計測欄を追加し、`CollectorAuditTrail.metadata["collector.effect.audit"]`/`"collections.change_set.total"` を KPI として記録する。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の Phase3 表にはこの KPI の許容値（`effect {audit}` 付き ops の応答時間 < 150ms）を記載し、CI で `tooling/ci/collect-iterator-audit-metrics.py --scenario audit_bridge` を呼び出すようにする。

##### 5.1.1 現況とギャップ
- `compiler/rust/runtime/src/collections/audit_bridge.rs` では `ChangeSet`/`ChangeItem` 構造体と `map_diff_to_changes`/`set_diff_to_changes` の JSON 変換ユーティリティが実装済みで、`MapCollector::finish` 側でも `CollectOutcome::record_change_set` が呼び出され `collector.effect.audit` のマーカーが立つ【F:../../compiler/rust/runtime/src/collections/audit_bridge.rs†L1-L220】【F:../../compiler/rust/runtime/src/prelude/collectors/map.rs†L1-L72】。`compiler/rust/runtime/src/config/mod.rs` の `merge_maps_with_audit` は `ChangeSet` を JSON に変換・ファイル出力できるが、`REML_COLLECTIONS_CHANGE_SET[_PATH]` を設定するコードはいまだ存在せず、CLI 側へ差分が届いていない。
- フロントエンドの `FormatterContext` は環境変数 `REML_COLLECTIONS_CHANGE_SET`/`_PATH` を読んで `AuditEnvelope.change_set` を構築する仕組みを備えている【F:../../compiler/rust/frontend/src/diagnostic/formatter.rs†L1-L240】が、現状 `reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` には `change_set` ブロックがなく `collections.diff.*` キーも観測できないため、`collect-iterator-audit-metrics.py --scenario map_set_persistent` の `collections.audit_bridge_pass_rate` も有効化されていない【F:../../reports/spec-audit/ch1/core_iter_collectors.audit.jsonl†L1-L7】。この齟齬を埋めるために、`ConfigMergeOutcome` で生成した JSON を CLI 実行前に環境変数・一時ファイル経由で注入するブリッジ層が必要である。

##### 5.1.2 残作業と着手順
1. `ConfigMergeOutcome::change_set_json`/`write_change_set_to_temp_dir` を利用して `ChangeSet` を一時ファイルへ書き出し、CLI を起動するランタイムパス（`compiler/rust/runtime` や `examples/` からの `reml` ランチャ）で `REML_COLLECTIONS_CHANGE_SET_PATH` を設定する。`collect-iterator-audit-metrics.py` の `map_set_persistent` シナリオが期待する `collections.diff.total` と `items` が含まれる JSON を出力することを優先する【F:../../compiler/rust/runtime/src/config/mod.rs†L1-L110】。
   - `compiler/rust/runtime/src/config/mod.rs` に追加した `set_collections_change_set_env` ヘルパーを呼び出すことで `ChangeSet` の JSON 出力と `REML_COLLECTIONS_CHANGE_SET_PATH` の設定をセットにできるため、ランタイム側の CLI 起動パイプラインではこのハンドルを保持しながら `reml_frontend` を起動し、処理終了後に `Drop` で環境変数と一時ファイルをクリーンアップする。
2. CLI 側では `FormatterContext::change_set` が注入された JSON を読み出し `AuditEnvelope.change_set` へ合流させるワイヤーを確認したうえで、メタデータの `metadata["collections.diff.total"]` などを `Diagnostic.extensions` へもコピーし、`scripts/validate-diagnostic-json.sh` の `--pattern collections.diff` 条件で `key`/`kind`/`value` を検証できる状態にする【F:../../compiler/rust/frontend/src/diagnostic/formatter.rs†L1-L240】【F:../../scripts/validate-diagnostic-json.sh†L1-L200】。
3. `reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` に `map_diff`/`set_partition`/`table_merge` などの `collections` ケースを追加・スナップショット化し、`reports/iterator-collector-summary.md` の `audit_bridge` セクションと `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 表に `collections.audit_bridge_pass_rate`/`collector.effect.audit_presence` の最新値を写す。
4. `compiler/rust/runtime/tests/core_collections_audit_bridge.rs`（新設）で `PersistentMap::merge_with_change_set`/`Set`/`Table` の差分 JSON を検証し、`collect-iterator-audit-metrics.py --scenario map_set_persistent --require-audit` が `collector.effect.audit` を 0 でなく 1 にセットすることを確認するテストを追加する。`scripts/poc_dualwrite_compare.sh --target map_diff` にも `collections.audit_bridge` 出力比較ルートを組み込み、OCaml 実装との差分を監査結果として報告する。

   ##### 現状ログとフォローアップ

   - `ConfigMergeOutcome` の JSON 出力を `write_change_set_to_temp_dir` で一時ファイル化し、CLI 実行前に `REML_COLLECTIONS_CHANGE_SET_PATH` を注入するランタイム経路の設計を `docs/notes/collections-audit-bridge-todo.md` に記録した。`collect-iterator-audit-metrics.py` の `map_set_persistent` シナリオが期待する `collections.diff.total/items` を満たすテンプレートも同ノートで追跡しており、次のコード実装ではこのテンプレートを具現化する予定である。
   - `FormatterContext::change_set` から読み出した `collections.diff` ペイロードを `AuditEnvelope.metadata` と `Diagnostic.extensions["collections.diff.*"]` に複写し、`scripts/validate-diagnostic-json.sh --pattern collections.diff` / `tooling/ci/collect-iterator-audit-metrics.py --scenario map_set_persistent` が同一JSONを検証できるように構成変更するルートもノートでフォローアップしている。
   - `reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` の `collections.diff` ケースを段階的に追加し、`reports/iterator-collector-summary.md` の `audit_bridge` セクションや `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 表に `collections.audit_bridge_pass_rate`/`collector.effect.audit_presence` を写す作業を「KPI同期」タスクとして位置づけた。

#### 5.2 Config/Data 差分 API との互換アダプタ
- `SchemaDiff`/`Change` に対応する構造体を `compiler/rust/runtime/src/config/collection_diff.rs` に定義し、`Map`/`Table`/`List` の差分を `Config.Data` で消費できる形式に直列化する。`docs/spec/3-7-core-config-data.md` の `schema_diff_to_change_set` サブセクションを参考に、キー順保証・type tag・`EffectLabels` の `mem_bytes` を維持した上で `Change` エントリを生成する。
- `Core.Collections` 側では `Core.Collections.Map.diff`/`Table.to_map` などの公開 API から新規アダプタ（例: `collections::config::MapDiffAdapter`）を提供し、`ChangeConfig::apply` に `AuditEnvelope.change_set` を直接流せるようにする。`docs/plans/bootstrap-roadmap/3-7-core-config-data-plan.md` に追記し、Config チームとのダブルチェックを記録する。
- 双方向テストとして `compiler/rust/runtime/tests/config/collection_diff_roundtrip.rs` を用意し、(1) `Map` 差分→`SchemaDiff`→`ChangeSet`、(2) `ChangeSet` JSON→`Map` の再構築、(3) `Table` の `insert`/`load_csv` 操作に `collector.effect.audit=true` が付いて `ChangeSet` の `audit` フラグへ伝播することを確認する。`scripts/validate-diagnostic-json.sh --section config` で `schema_diff.*` のキーをチェックし、`docs/notes/spec-integrity-audit-checklist.md` に `config.diff.bridge` の TODO を記録する。

#### 5.3 `effect {audit}` による Capability チェックと運用ガード
- `tables`, `maps`, `collections` が `effect {audit}` を発行する API（`emit_metric`, `collect_table` の `audit` モードなど）では `CapabilityRegistry` に `core.collections.audit` を登録し、`docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` との同期を記録する。`scripts/poc_dualwrite_compare.sh --target audit_bridge` を実行し、`CapabilityRegistry::check("core.collections.audit")` が `false` のときに `CollectError::CapabilityDenied` を返すパスを検証する。
- `effect {audit}` を伴うテスト/CI は `tooling/ci/collect-iterator-audit-metrics.py --section collectors --scenario audit_cap` で `collector.effect.audit=true` を求め、`scripts/validate-diagnostic-json.sh --pattern collector.effect.audit` で `reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` を検証する。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の Phase3 `Capabilty` 列に `core.collections.audit` を追加し、`reports/iterator-collector-summary.md` の `status` カラムで `audit_cap` ケースの結果を追跡する。
- Capability 側のドキュメント更新として `docs/guides/runtime-bridges.md` に `Core.Collections` の `audit` フラグを記載し、`docs/plans/rust-migration/unified-porting-principles.md` の「同一性優先」セクションへ `effect {audit}` 含む操作の制限を注記する。

### 6. ドキュメント整備とサンプル検証（40-41週目）
**担当領域**: 情報更新

6.1. 仕様書内サンプルの動作確認と更新、必要に応じて `NOTE` や脚注で制約事項を明記する。
6.2. `README.md`/`3-0-phase3-self-host.md` に Core.Collections 実装状況と API ハイライトを追記する。
6.3. `examples/` ディレクトリに永続コレクション利用例を追加し、CI で自動実行するテストを用意する。

### 7. テスト・ベンチマーク統合（41週目）
**担当領域**: 品質保証

7.1. 単体・プロパティテスト (例えば QuickCheck 相当) を導入し、構造共有や順序保持に関する不変条件を検証する。
7.2. ベンチマークスイートを追加し、Phase 2 で確立した Rust ベースライン比 ±15% 以内を目標に性能を測定する。OCaml 実装の結果は参考資料として別添する。
7.3. テスト・ベンチマークの結果をメトリクス／リスク管理ドキュメントに反映し、未達の場合はフォローアップタスクを起票する。

## 成果物と検証
- `Core.Collections` API が仕様と一致し、効果タグ・診断連携が正しく機能すること。
- 永続／可変コレクション双方で Rust 実装のベースライン（Phase 2 ベンチマーク）と比較した性能指標が基準内に収まっていること。
- ドキュメント・サンプルが更新され、Config/Data/Diagnostics との相互参照が成立していること。

## リスクとフォローアップ
- Finger tree 実装が性能目標を満たさない場合、代替構造 (RRB-Tree 等) の調査を `docs/notes/core-library-outline.md` に記録し、Phase 4 で検討する。
- `Cell`/`Ref` の内部可変性が効果システムと衝突した場合、仕様更新 (1-3 章) をエスカレーションする。
- CSV ロード等 IO 連携でプラットフォーム依存差異が生じた際は `0-4-risk-handling.md` に記載し、Phase 3-5 (IO & Path) で調整する。

## 参考資料
- [3-2-core-collections.md](../../spec/3-2-core-collections.md)
- [3-1-core-prelude-iteration.md](../../spec/3-1-core-prelude-iteration.md)
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
- [3-7-core-config-data.md](../../spec/3-7-core-config-data.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)
