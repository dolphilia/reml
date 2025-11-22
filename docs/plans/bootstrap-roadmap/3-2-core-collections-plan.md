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
| `List<T>` | `empty`/`singleton`/`push_front`/`concat`/`map`/`fold`/`to_iter`/`as_vec`【F:../../spec/3-2-core-collections.md†L21-L45】 |  `List` は `Vec<T>` を包むのみで `into_vec`/`as_slice` しか提供していない。Finger tree や `push_front` 系は未定義。【F:../../compiler/rust/runtime/src/prelude/collectors/list.rs†L19-L95】 | Finger tree をベースにした `ListCore` を `runtime/src/collections/persistent/list.rs`（新設）へ実装し、`ListCollector` の戻り値を差し替える。`to_iter`/`of_iter` は `Iter` の stage 情報を共有し、`List.as_vec` には `effect {mem}` を付与する。 |
| `Map<K,V>` / `Set<T>` | `empty_map`/`insert`/`update`/`merge`/`keys`、`contains`/`diff`/`partition`【F:../../spec/3-2-core-collections.md†L46-L69】 | `Map`/`Set` も `BTreeMap`/`BTreeSet` の薄いラッパーで、`into_*` 以外の公開 API が欠落し、`merge`/`diff`/`Collector` 連携の仕様ギャップがある。【F:../../compiler/rust/runtime/src/prelude/collectors/map.rs†L20-L107】【F:../../compiler/rust/runtime/src/prelude/collectors/set.rs†L20-L119】 | 赤黒木ベースの `PersistentMap`/`PersistentSet` を `collections/persistent/btree.rs` にまとめて実装し、`diff`/`merge` を Config/Data の `SchemaDiff` へ接続する。`MapCollector`/`SetCollector` は既存の `BTree*` を利用したまま API を添付する。 |
| 変換ヘルパ／Iter 終端 | `List.of_iter`/`Map.from_iter`/`Set.diff` や `list_to_vec`/`map_to_table` 等の変換 API【F:../../spec/3-2-core-collections.md†L70-L80】【F:../../spec/3-2-core-collections.md†L227-L244】 | `Iter` 側は `collect_list`/`collect_vec` のみを提供し、`collect_map`/`collect_table` や `Iter.try_collect` 経由の `Map.from_iter` が存在しない。【F:../../compiler/rust/runtime/src/prelude/iter/mod.rs†L145-L151】 | `Iter` に `collect_map`/`collect_set`/`collect_table` を追加し、`List.of_iter` などのヘルパは `Collector` をラップする構成で `Result`/`CollectError` をそのまま返す。変換 API 群を `Core.Collections` 名前空間にまとめ、差分適用時に `effect` 伝播を保証する。 |
| `Vec<T>` | `new`/`with_capacity`/`push`/`pop`/`reserve`/`shrink_to_fit`/`iter`/`to_list` および `collect_from`【F:../../spec/3-2-core-collections.md†L91-L116】 | `VecCollector` は存在するが `Core.Collections.Vec` としての API や `CollectError::OutOfMemory` 伝播は未整備。`reserve` 失敗を診断へ橋渡しする仕組みも無い。【F:../../compiler/rust/runtime/src/prelude/collectors/vec.rs†L18-L100】 | `Vec<T>` 用のラッパ型（`CoreVec<T>`）を導入し、`try_reserve` の `TryReserveError` を `CollectError::OutOfMemory` に写像する。`to_list` は `List` へコピーした上で `effect {mem}` を記録し、`Vec.collect_from` を `Iter::collect_vec` と共通実装にする。 |
| `Cell<T>` / `Ref<T>` | `new_cell`/`get`/`set` と `new_ref`/`clone_ref`/`borrow`/`borrow_mut`【F:../../spec/3-2-core-collections.md†L91-L134】 | `collectors/mod.rs` に `cell`/`ref` モジュールが存在せず、内部可変性や `effect {cell}`/`{rc}` を発火させる仕組みが未着手。【F:../../compiler/rust/runtime/src/prelude/collectors/mod.rs†L8-L20】 | `Cell` は `RefCell` + Copy 制約を満たす軽量構造として `effect {cell}` を記録し、`Ref` は `Arc` + `RwLock` ベースで `effect {rc}`/`{mut}` を付ける。両者とも `CollectorAuditTrail` へ内部可変性マーカーを追記する。 |
| `Table<K,V>` | `new_table`/`insert`/`remove`/`iter`/`to_map`/`load_csv`【F:../../spec/3-2-core-collections.md†L138-L149】 | `Table` は `Vec<(K,V)>` 保存と `into_entries` のみで、挿入・削除・CSV ロード・`effect {io}` は未実装。`TableCollector` も `seen` のみで監査フックが簡易的。【F:../../compiler/rust/runtime/src/prelude/collectors/table.rs†L20-L124】 | Robin Hood hashing + 挿入順リストを保持する `OrderedTable` を実装し、`insert`/`remove`/`iter`/`to_map` を公開する。`load_csv` は `Core.IO` 連携タスク（3-5）と協調し、`effect {io}`/`{mut}` を同時に記録する。 |

上記に付随して `Collections.audit_bridge`（仕様 §5）や差分 API の JSON 変換が丸ごと欠落している点も確認した。`CollectOutcome` が保持する `CollectorAuditTrail` を Config/Data 章の `ChangeSet` に流し込むブリッジ層を Phase 3.2 で構築する。

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

### 3. 可変コレクションと内部可変性（39週目）
**担当領域**: `Vec`/`Cell`/`Ref`/`Table`

3.1. 標準 `Vec` の API セットを仕様通り実装し、`effect {mut}`/`{mem}` の正確な付与を確認する。
3.2. `Cell<T>`/`Ref<T>` の内部可変性モデルを実装し、`effect {cell}`/`{rc}` を活用したテストケースを整備する。
3.3. `Table<K,V>` の挿入順序保持ロジック・CSV ローダを実装し、`Core.IO`/`Core.Text` と連携する統合テストを追加する。

### 4. Iter/Collector 相互運用（39-40週目）
**担当領域**: 遅延列との結合

4.1. `Iter` から各コレクションへ変換する API (`collect_list`, `collect_vec`, `Map.from_iter` 等) を実装し、重複キー検出やエラーハンドリングを確認する。
4.2. `IntoIter` 実装を整備し、`Iter` と永続構造の往復変換で所有権が崩れないことをテストする。
4.3. `Collector` 実装と `Iter.try_collect` の統合を検証し、失敗時の `CollectError` が `Diagnostic` に落とし込まれるか確認する。

### 5. Diagnostics / Config / Audit 連携（40週目）
**担当領域**: 他章との統合

5.1. `Core.Diagnostics` の `AuditEnvelope.change_set` と連携するための JSON 差分ユーティリティを実装し、`Map`/`Table` の変換を提供する。
5.2. Config/Data 章 (3-7) で利用する差分 API (`SchemaDiff`, `Change`) との互換アダプタを用意し、双方向変換テストを実施する。
5.3. `effect {audit}` を伴う操作 (`emit_metric` 等) の前提条件を確認し、Capability チェックのフックを追加する。

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
