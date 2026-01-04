# 3.2 Core Collections

> 目的：イミュータブル／ミュータブル双方の代表的なデータ構造を標準化し、`Iter`・`Result`／`Option` と組み合わせた宣言的データフローを支える。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `@pure`, `effect {mut}`, `effect {mem}`, `effect {cell}`, `effect {rc}`, `effect {audit}`, `effect {io}` |
| 依存モジュール | `Core.Prelude`, `Core.Iter`, `Core.Diagnostics`, `Core.Text` |
| 相互参照 | [3.1 Core Prelude & Iteration](3-1-core-prelude-iteration.md), [3.3 Core Text & Unicode](3-3-core-text-unicode.md), [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md) |

## 1. モジュール編成と import 規則

- `use Core.Collections;` で永続コンテナ (`List`, `Map`, `Set`) と可変コンテナ (`Vec`, `Cell`, `Table`) を導入する想定。`use Core;` の上位共通モジュールからは Prelude/Iter と並列で参照される。【F:notes/core-library-outline.md†L13-L18】
- 永続コンテナは `@pure` を維持し、構築・変換は構造共有を前提とした O(log n) もしくはアモルタイゼーションでの O(1) を目標とする。
- 可変コンテナは `effect {mut}` を要求し、`Iter.collect_vec` などの終端操作から利用する際には効果タグが伝搬する。【F:3-1-core-prelude-iteration.md†L118-L137】
- すべてのコンテナ型は `IntoIter` を実装し、`Iter` との往復を容易にする。詳細は 4.2 の `Collector` 契約を参照。

## 2. 永続コレクション API（`@pure`）

### 2.1 `List<T>`

```reml
pub type Box<T>

@must_use
pub type List<T> =
  | Cons(head: T, tail: Box<List<T>>)
  | Nil

fn empty<T>() -> List<T>
fn singleton<T>(value: T) -> List<T>
fn push_front<T>(list: List<T>, value: T) -> List<T>
fn concat<T>(left: List<T>, right: List<T>) -> List<T>
fn map<T, U>(list: List<T>, f: (T) -> U) -> List<U>
fn fold<T, S>(list: List<T>, init: S, f: (S, T) -> S) -> S
fn to_iter<T>(list: List<T>) -> Iter<T>
fn as_vec<T>(list: &List<T>) -> Vec<T> // `effect {mem}`
```

- `push_front` は O(1)、`concat` は差分配列ベースの木構造を用いて平均 O(log n) を維持する。
- 内部実装は finger tree をベースとし、構造共有によりメモリ効率を最大化する。
- `map`/`fold` は末尾再帰最適化され、大きなリストでもスタックオーバーフローしない。
- `as_vec` は明示的に `effect {mem}` を要求し、可変操作へ移行する際のコストを可視化する。

### 2.2 `Map<K, V>` と `Set<T>`

```reml
pub type PersistentMap<K, V>
pub type PersistentSet<T>

pub type Map<K, V> = PersistentMap<K, V>
pub type Set<T> = PersistentSet<T>

fn empty_map<K, V>() -> Map<K, V>
fn get<K: Ord, V>(map: Map<K, V>, key: K) -> Option<V>
fn insert<K: Ord, V>(map: Map<K, V>, key: K, value: V) -> Map<K, V>
fn update<K: Ord, V>(map: Map<K, V>, key: K, f: (Option<V>) -> Option<V>) -> Map<K, V>
fn merge<K: Ord, V>(base: Map<K, V>, delta: Map<K, V>, f: (V, V) -> V) -> Map<K, V>
fn keys<K: Ord, V>(map: Map<K, V>) -> Iter<K>

fn empty_set<T>() -> Set<T>
fn contains<T: Ord>(set: Set<T>, value: T) -> Bool
fn insert<T: Ord>(set: Set<T>, value: T) -> Set<T>
fn diff<T: Ord>(left: Set<T>, right: Set<T>) -> Set<T>
fn partition<T: Ord>(set: Set<T>, pred: (T) -> Bool) -> (Set<T>, Set<T>)
```

- `Map` のデフォルト実装は平衡二分木（赤黒木）。`keys` は順序付き反復を提供し、監査ログや CLI 出力で安定性を確保する。
- `merge` と `diff` は `Core.Data` の `SchemaDiff` や `Change` と整合し、監査ログで差分を共有する前提を提供する。【F:3-7-core-config-data.md†L16-L55】
- `Set` は `Map<T, Unit>` の薄いラッパーであり、`Collector` 実装を共有する。

#### 2.2.1 Set の実行時表現（Backend/Runtime） {#set-runtime-abi}

- `Set<T>` はランタイムのヒープオブジェクトとして扱い、Backend は不透明ポインタ（`ptr`）で受け渡す。
- 最小 ABI は `compiler/runtime/native/include/reml_runtime.h` に定義された `reml_set_new` / `reml_set_insert` / `reml_set_contains` / `reml_set_len` を利用し、`REML_TAG_SET` で型識別する。
- 現行実装の `reml_set_t` は可変配列ベースで、重複判定はポインタ同値に限定する。`Ord` に基づく比較フックは将来拡張（Phase 4 以降）で導入予定。
- `reml_set_insert` は永続構造として新しい Set を返し、要素は参照カウントで保持する。破棄は `dec_ref` 経由で `REML_TAG_SET` のデストラクタが担当する。

### 2.3 バッチ変換ヘルパ

| 関数 | シグネチャ | 効果 | 説明 |
| --- | --- | --- | --- |
| `List.of_iter` | `fn of_iter<T>(iter: Iter<T>) -> List<T>` | `@pure` | `Iter` から永続リストを生成。 |
| `Map.from_iter` | `fn from_iter<K: Ord, V>(iter: Iter<(K,V)>) -> Result<Map<K,V>, CollectError>` | `@pure` | キー重複時は `CollectError::DuplicateKey`。 |
| `Map.merge` | `fn merge<K, V>(base: Map<K, V>, delta: Map<K, V>, f: (V, V) -> V) -> Map<K, V>` | `@pure` | 差分マージ。 |
| `Map.from_pairs` | `fn from_pairs<K: Ord, V>(pairs: List<(K, V)>) -> Result<Map<K, V>, CollectError>` | `@pure` | 小規模セットアップ向けの軽量ビルダ。 |
| `Set.diff` | `fn diff<T>(left: Set<T>, right: Set<T>) -> Set<T>` | `@pure` | 差集合。 |
| `Set.partition` | `fn partition<T>(set: Set<T>, pred: (T) -> Bool) -> (Set<T>, Set<T>)` | `@pure` | 条件で 2 分割。 |

`Map.from_pairs` は DSL の標準環境やテーブル初期値を記述するためのユーティリティで、`List` など永続構造からの変換時に重複キーを `CollectError::DuplicateKey` として検出する。初期化コードでの `Map.insert` 連鎖を排除し、Lisp サンプルの `default_env` のような定数マップを構造的に宣言できる。【F:../examples/language-impl-samples/reml/mini_lisp_combinator.reml†L118-L138】

> **NOTE**: `examples/core-collections/usage.reml` は永続リスト → `Map.from_pairs` → `Vec.collect_from` → `Table.to_map` → `Cell`/`Ref` のパイプラインを手本として示し、`CollectError::DuplicateKey`・`effect {mem}`・`effect {mut}`・`effect {cell}`・`effect {rc}` の発火点と手動実行手順 (`cargo run --bin reml -- examples/core-collections/usage.reml`) を補足するドキュメント上の証跡です。【F:../examples/core-collections/usage.reml†L1-L52】

### 2.4 順序保証とイミュータブル更新契約

- `List`/`Map`/`Set` はいずれも永続データ構造であり、`insert`/`update` などの操作は既存構造を破壊せず新しいバージョンを返す。内部では構造共有を利用して `O(1)`（`List.push_front`）〜`O(log n)`（`Map.insert`/`Set.insert`）以内に抑え、Chapter 0-1 が定義する性能基準を満たす。【F:0-1-project-purpose.md†L11-L37】
- `List.fold` と `Iter.fold` はどちらも入力の左端から右端へ評価し、`List` の再帰実装も末尾再帰最適化済みである。DSL 作者は `List` 基盤と `Iter` 基盤を同じ計算モデルとして把握でき、学習コストを抑えられる。【F:0-1-project-purpose.md†L29-L44】
- `Map.to_iter`/`Map.keys`/`Map.values` は常にキー昇順で結果を生成する。`MapCollector` や `Map.from_iter` は挿入段階でキー比較を行い、同一キーが観測された場合は `CollectError::DuplicateKey` を返す設計を共有する。監査ログや差分出力が安定した順序で得られ、`Diagnostic` の行番号が再現しやすくなる。【F:3-6-core-diagnostics-audit.md†L42-L88】
- `Table` は挿入順を保持し、`TableCollector` や `table_to_map` で `Map` に変換する際に `Ord` に基づく再ソートが入る。挿入順での検証が必要な DSL では `Table` を終端型として選択し、安定表示が必要な箇所だけを `Map` へ昇格させる運用を推奨する。 
- 差分適用（`Map.merge`/`Set.diff`）は各入力を昇順で走査し、`Iter.try_collect` 経由で `Map` に再収集する場合も計算量は `O(m log(n/m+1))` に収束する。構成ファイル数万キー規模の変更検知でも線形スケーリングを維持できるようここに明記する。【F:0-1-project-purpose.md†L11-L37】

> **NOTE**: `Table.load_csv` は `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` の `Core.IO.CsvReader` を呼び出して UTF-8/BOM/CRLF を検証しつつ `EffectSet::mark_io()` を記録する設計で、`docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md` §3.3 にある `collect_table_csv` KPI を補完します。`examples/core-collections/README.md`/`usage.reml` のパイプライン説明は `Table.insert` から `Table.to_map` への流れと、`collect_table_csv` シナリオを含む監査メトリクスの手順を示しています。

## 3. 可変コレクション（`effect {mut}`）

| 型 | 主要操作 | 効果タグ |
| --- | --- | --- |
| `Vec<T>` | `push`, `pop`, `reserve`, `shrink_to_fit`, `iter` | `effect {mut}`, 一部 `effect {mem}` |
| `Cell<T>` | `new_cell`, `get`, `set` | `effect {cell}` |
| `Ref<T>` | `new_ref`, `clone_ref`, `borrow`, `borrow_mut` | `effect {rc}`, `effect {mut}` |
| `Table<K,V>` | `insert`, `remove`, `iter`, `to_map` | `effect {mut}`, `effect {mem}` |

### 3.1 `Vec<T>`

```reml
pub type Vec<T>

fn new<T>() -> Vec<T>                                  // `effect {mut}`
fn with_capacity<T>(cap: usize) -> Vec<T>               // `effect {mut, mem}`
fn push<T>(vec: &mut Vec<T>, value: T) -> ()            // `effect {mut}`
fn pop<T>(vec: &mut Vec<T>) -> Option<T>                // `effect {mut}`
fn reserve<T>(vec: &mut Vec<T>, additional: usize) -> ()// `effect {mut, mem}`
fn shrink_to_fit<T>(vec: &mut Vec<T>) -> ()             // `effect {mut, mem}`
fn iter<T>(vec: &Vec<T>) -> Iter<T>                     // `@pure`
fn to_list<T>(vec: Vec<T>) -> List<T>                   // `effect {mem}`
```

- `Vec::collect_from(iter: Iter<T>) -> Result<Vec<T>, CollectError>` を提供し、`Iter.try_collect` のデフォルト実装と一致させる。メモリ不足時は `CollectError::OutOfMemory` を返す。
- `to_list` は永続構造へのコピーを明示し、宣言的パイプラインへの復帰コストを伝える。

### 3.2 `Cell<T>` / `Ref<T>`

```reml
pub type Borrow<T>
pub type BorrowMut<T>

pub type Cell<T: Copy>

fn new_cell<T: Copy>(value: T) -> Cell<T>       // `effect {cell}`
fn get<T: Copy>(cell: &Cell<T>) -> T            // `@pure`
fn set<T: Copy>(cell: &Cell<T>, value: T) -> () // `effect {cell}`

pub type Ref<T>

fn new_ref<T>(value: T) -> Ref<T>               // `effect {rc}`
fn clone_ref<T>(value: &Ref<T>) -> Ref<T>       // `effect {rc}`
fn borrow<T>(value: &Ref<T>) -> Borrow<T>       // `@pure`
fn borrow_mut<T>(value: &Ref<T>) -> BorrowMut<T>// `effect {mut, rc}`
```

- `Cell` は Copy 制約を課し、内部可変性の範囲を明示。`effect {cell}` を `mut` から切り離して監査時の判別を容易にする。
- `Ref` は参照カウントを保持し、共有所有権を Chapter 3.9（Core.Ffi/Core.Unsafe）で定義する契約と整合させる。

### 3.3 `Table<K, V>`

```reml
pub type Path

pub type Table<K, V>

fn new_table<K, V>() -> Table<K, V>                                // `effect {mut}`
fn insert<K, V>(table: &mut Table<K, V>, key: K, value: V) -> ()    // `effect {mut}`
fn remove<K, V>(table: &mut Table<K, V>, key: &K) -> Option<V>      // `effect {mut}`
fn iter<K, V>(table: &Table<K, V>) -> Iter<(K, V)>                  // `@pure`
fn to_map<K: Ord, V>(table: Table<K, V>) -> Map<K, V>               // `effect {mem}`
fn load_csv<K, V>(path: Path) -> Result<Table<K, V>, Diagnostic>    // `effect {io, mut}`
```

- 反復順序は挿入順を保持する。CLI/監査ログに同一順序で記録でき、`change_set` との突き合わせが容易になる。
- IO 連携関数（`load_csv` など）は 3.5（Core IO & Path）で定義する `IO.Reader` と連携し、`defer` によるリソース解放を利用する。【F:notes/core-library-scope.md†L15-L24】

## 4. Iter / Collector との相互運用

| コレクタ | 効果タグ | 失敗時エラー型 | 備考 |
| --- | --- | --- | --- |
| `ListCollector<T>` | `@pure` | なし | 永続リストに構造共有で格納。 |
| `VecCollector<T>` | `effect {mut, mem}` | `CollectError::OutOfMemory` | 動的確保失敗を伝播。 |
| `MapCollector<K,V>` | `@pure` | `CollectError::DuplicateKey` | キー衝突時に衝突キーを返す。 |
| `SetCollector<T>` | `@pure` | `CollectError::DuplicateKey` | Map と同一実装。 |
| `TableCollector<K,V>` | `effect {mut}` | `CollectError::DuplicateKey` | 挿入順を維持。 |

- `Collector::push` が `Err` を返した場合、`Iter.try_collect` 全体が短絡し、`Result` を通じて呼び出し側へ伝播する。【F:3-1-core-prelude-iteration.md†L132-L145】
- `CollectError` は `Core.Diagnostics` の `Diagnostic` へ変換するユーティリティ（`Collections.audit_bridge`）を提供予定。

## 5. 監査・差分との連携

- 永続コレクションの `diff` 系 API は `audit_id` と `change_set` を組み合わせ、Chapter 3.6/3.7 の監査フローで直接利用する計画。【F:notes/core-library-outline.md†L17-L19】
- `Table` や `Vec` など可変コンテナの操作履歴は `Core.Diagnostics` で提供する `ChangeTrace` に統合し、再現性を確保する。
- 監査モードでは `CollectError` を `Diagnostic` へマッピングする標準関数（`Collections.audit_bridge`）を提供し、`change_set` に失敗キーや差分を付与する。

## 6. パフォーマンスベンチマーク

### 6.1 永続コレクションの性能特性

| 操作 | `List<T>` | `Map<K,V>` | `Set<T>` | 実装アルゴリズム |
| --- | --- | --- | --- | --- |
| 要素アクセス | O(n) | O(log n) | O(log n) | Finger tree / 赤黒木 |
| 挿入・削除 | O(1) front, O(n) arbitrary | O(log n) | O(log n) | 構造共有 |
| 連結・マージ | O(log min(m,n)) | O(m log(n/m+1)) | O(m log(n/m+1)) | 効率的マージ |
| メモリ使用量 | オーバーヘッド 20% | オーバーヘッド 30% | オーバーヘッド 25% | ポインターオーバーヘッド |

### 6.2 可変コレクションの性能特性

| 操作 | `Vec<T>` | `Table<K,V>` | 実装アルゴリズム |
| --- | --- | --- | --- |
| 要素アクセス | O(1) | O(1) average | 動的配列 / Robin Hood hashing |
| 挿入・削除 | O(1) amortized | O(1) average | 数学的期待値 |
| 再配置 | O(n) コピー | O(n) リハッシュ | 指数サイズ成長 |

## 7. 設計決定事項

### 7.1 解決済み設計問題

1. **ハッシュマップ vs ツリーマップ**: デフォルトは赤黒木ベースの `Map`/`Set` を採用。ハッシュベースは `Table` で提供。

2. **`Cell` の効果タグ**: `effect {cell}` を独立で保持し、監査時に内部可変性と所有権移転を区別する。

3. **`Table` の順序保証**: insertion order を維持し、監査ログでの再現性を保証する。メモリオーバーヘッドは 10-15% 。

4. **FFI 所有権モデル**: `Ref<T>` は [3.9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md) の ARC モデルと互換。参照カウントは thread-safe。

## 7. 使用例（Iter パイプライン）

```reml
use Core;

pub type Record

fn group_valid_users(rows: Iter<Record>) -> Result<Map<UserId, List<Record>>, Diagnostic> =
  rows
    |> Iter.filter_map(|row|
         ensure(row.status == Status::Active, || Diagnostic::invalid_value(row.status))?
           .then(|| Some((row.user_id, row)))
       )
    |> Iter.try_collect(MapCollector::new(|existing: Option<List<Record>>, next| {
         let list = existing.unwrap_or(List::empty())
           |> List.push_front(next)
           |> List.reverse();
         Ok(list)
       }))
```

- `Iter.filter_map` と `ensure` を組み合わせ、無効行は `Diagnostic` とともに早期終了させる。`?` が `Result` を伝播する点で Chapter 3.1 の方針と一致。
- `MapCollector` はキーディアップレートで `List` を構築し、永続構造間の構造共有により `@pure` を維持する。
- 本例は Config/Data 章で扱う `change_set` とも親和性が高く、監査ログに差分を記録する際の基盤となる。

## 8. コレクション間変換ヘルパ

```reml
pub type Table<K, V>

// 永続コレクション間の変換
fn list_to_set<T: Ord>(list: List<T>) -> Set<T>                 // `@pure`
fn map_keys<K: Ord, V>(map: Map<K, V>) -> Set<K>               // `@pure`
fn map_values<K, V>(map: Map<K, V>) -> List<V>                 // `@pure`

// 永続・可変間の変換
fn list_to_vec<T>(list: List<T>) -> Vec<T>                     // `effect {mem}`
fn vec_to_list<T>(vec: Vec<T>) -> List<T>                      // `effect {mem}`
fn map_to_table<K, V>(map: Map<K, V>) -> Table<K, V>           // `effect {mut, mem}`
fn table_to_map<K: Ord, V>(table: Table<K, V>) -> Map<K, V>    // `@pure`

// 特殊化された変換
fn collect_duplicates<T: Ord>(list: List<T>) -> Map<T, usize>  // `@pure`
fn group_by<T, K: Ord>(list: List<T>, f: (T) -> K) -> Map<K, List<T>> // `@pure`
```

> 関連: [3.1 Core Prelude & Iteration](3-1-core-prelude-iteration.md#3-5-パイプライン使用例) / [2.5 エラー設計](2-5-error.md) / [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md)
