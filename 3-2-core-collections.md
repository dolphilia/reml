# 4.3 Core Collections（フェーズ3 ドラフト）

Status: Draft（内部レビュー中）

> 目的：イミュータブル／ミュータブル双方の代表的なデータ構造を標準化し、`Iter`・`Result`／`Option` と組み合わせた宣言的データフローを支える。

## 0. ドラフトメタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | Draft（フェーズ3） |
| 効果タグ | `@pure`, `effect {mut}`, `effect {mem}`, `effect {cell}`, `effect {rc}`, `effect {audit}` |
| 依存モジュール | `Core.Prelude`, `Core.Iter`, `Core.Diagnostics`, `Core.Text` |
| 相互参照 | [3.1 Core Prelude & Iteration](3-1-core-prelude-iteration.md), [3.3 Core Text & Unicode](3-3-core-text-unicode.md), 3.6（Core Diagnostics & Audit, 執筆予定） |

## 1. モジュール編成と import 規則

- `use Core.Collections;` で永続コンテナ (`List`, `Map`, `Set`) と可変コンテナ (`Vec`, `Cell`, `Table`) を導入する想定。`use Core;` の上位共通モジュールからは Prelude/Iter と並列で参照される。【F:notes/core-library-outline.md†L13-L18】
- 永続コンテナは `@pure` を維持し、構築・変換は構造共有を前提とした O(log n) もしくはアモルタイゼーションでの O(1) を目標とする。
- 可変コンテナは `effect {mut}` を要求し、`Iter.collect_vec` などの終端操作から利用する際には効果タグが伝搬する。【F:3-1-core-prelude-iteration.md†L118-L137】
- すべてのコンテナ型は `IntoIter` を実装し、`Iter` との往復を容易にする。詳細は 4.2 の `Collector` 契約を参照。

## 2. 永続コレクション API（`@pure`）

### 2.1 `List<T>`

```reml
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
- `as_vec` は明示的に `effect {mem}` を要求し、可変操作へ移行する際のコストを可視化する。

### 2.2 `Map<K, V>` と `Set<T>`

```reml
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
- `merge` と `diff` は `Core.Data` の `SchemaDiff` や `Change` と整合し、監査ログで差分を共有する前提を提供する。【F:2-8-data.md†L16-L55】
- `Set` は `Map<T, Unit>` の薄いラッパーであり、`Collector` 実装を共有する。

### 2.3 バッチ変換ヘルパ

| 関数 | シグネチャ | 効果 | 説明 |
| --- | --- | --- | --- |
| `List.of_iter` | `fn of_iter<T>(iter: Iter<T>) -> List<T>` | `@pure` | `Iter` から永続リストを生成。 |
| `Map.from_iter` | `fn from_iter<K: Ord, V>(iter: Iter<(K,V)>) -> Result<Map<K,V>, CollectError>` | `@pure` | キー重複時は `CollectError::DuplicateKey`。 |
| `Map.merge` | `fn merge<K, V>(base: Map<K, V>, delta: Map<K, V>, f: (V, V) -> V) -> Map<K, V>` | `@pure` | 差分マージ。 |
| `Set.diff` | `fn diff<T>(left: Set<T>, right: Set<T>) -> Set<T>` | `@pure` | 差集合。 |
| `Set.partition` | `fn partition<T>(set: Set<T>, pred: (T) -> Bool) -> (Set<T>, Set<T>)` | `@pure` | 条件で 2 分割。 |

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

## 6. 未決事項とレビュー指針

1. `Map`/`Set` の基盤実装を木型とハッシュ型で切り替えるフラグ（`feature {hash_map}`）を導入するか、別型として並立させるか。
2. `Cell` の効果タグを独立 (`effect {cell}`) で維持するか、`mut` に内包させるか。監査要件との整合性をレビューする。
3. `Table` の反復順序保証を `Vec` ベースで行う場合のメモリコストと、監査ログ出力に必要な安定ハッシュとの両立について検討が必要。
4. FFI / Async 章（4.10）で扱う所有権モデルと `Ref<T>` の連携仕様をレビューし、互換性指針を固める。

## 7. 使用例（Iter パイプライン）

```reml
use Core;

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

> 関連: [3.1 Core Prelude & Iteration](3-1-core-prelude-iteration.md#3-5-パイプライン使用例) / [2.5 エラー設計](2-5-error.md)
