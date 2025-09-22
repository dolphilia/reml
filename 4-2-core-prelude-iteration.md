# 4.2 Core Prelude & Iteration（フェーズ3 ドラフト）

> 目的：Reml の「例外なし」「左→右パイプ」「宣言的スタイル」を支える基本 API を標準化し、全ての DSL から同一の `Option`/`Result`/`Iter` モデルを利用できるようにする。

## 1. モジュール構成と import 規則

- `use Core;` で `Core.Prelude` と `Core.Iter` を一括導入できる。既存の Chapter 2 モジュール（`Core.Parse`）と同様、Prelude は**型推論を阻害しない軽量な宣言**のみを公開する。【F:0-1-overview.md†L90-L109】
- `Core.Prelude` は `Option`/`Result`/`Never` 型と演算子糖衣（`?`, パイプ `_` 占位、ガード補助）を提供し、例外を排した失敗制御スタイルを保証する。【F:1-1-syntax.md†L276-L295】【F:1-3-effects-safety.md†L64-L75】
- `Core.Iter` は不変データ構造と親和性の高い**遅延列 `Iter<T>`**を定義し、`|>` パイプと組み合わせた宣言的データフローを実現する。【F:1-1-syntax.md†L291-L339】
- Prelude / Iter はいずれも `@pure` がデフォルト。`effect` を要求する関数はシグネチャにタグを明記する（例：`Iter.collect_vec` は `effect {mut}`）。【F:1-3-effects-safety.md†L40-L75】

## 2. 失敗制御プリミティブ（Core.Prelude）

### 2.1 型定義

```reml
@must_use
pub type Option<T> =
  | Some(value: T)
  | None

@must_use
pub type Result<T, E> =
  | Ok(value: T)
  | Err(error: E)

pub type Never = Result<Never, Never> // 空集合を示す記号的型。実体化不可。
```

- `Option`/`Result` はどちらも `@must_use` を付与し、無視時にコンパイル警告を生成する。これにより暗黙の失敗無視を防ぐ。【F:1-3-effects-safety.md†L46-L75】
- `Never` は決して生成されない型として扱い、発散や「到達不能」を表現する。`match` において `Never` を扱う節は型推論上どの型にも適合する。

### 2.2 基本操作

| 関数 | シグネチャ | 説明 | 効果 |
| --- | --- | --- | --- |
| `Option.is_some` | `fn is_some<T>(self: Option<T>) -> Bool` | `Some` かを判定。 | `@pure` |
| `Option.map` | `fn map<T, U>(self: Option<T>, f: (T) -> U) -> Option<U>` | 値を変換。 | `@pure` |
| `Option.and_then` | `fn and_then<T, U>(self: Option<T>, f: (T) -> Option<U>) -> Option<U>` | 連鎖。 | `@pure` |
| `Option.unwrap_or` | `fn unwrap_or<T>(self: Option<T>, default: T) -> T` | `None` 時の代替値。 | `@pure` |
| `Option.expect` | `fn expect<T>(self: Option<T>, message: Str) -> T` | `None` の場合は `panic` を発生（開発時のみ）。 | `effect {debug}` |
| `Result.map` | `fn map<T, E, U>(self: Result<T, E>, f: (T) -> U) -> Result<U, E>` | 正常値を変換。 | `@pure` |
| `Result.map_err` | `fn map_err<T, E, F>(self: Result<T, E>, f: (E) -> F) -> Result<T, F>` | エラー値を変換。 | `@pure` |
| `Result.and_then` | `fn and_then<T, E, U>(self: Result<T, E>, f: (T) -> Result<U, E>) -> Result<U, E>` | 連鎖。 | `@pure` |
| `Result.or_else` | `fn or_else<T, E, F>(self: Result<T, E>, f: (E) -> Result<T, F>) -> Result<T, F>` | 代替計算。 | `@pure` |
| `Result.unwrap_or` | `fn unwrap_or<T, E>(self: Result<T, E>, default: T) -> T` | エラー時の代替値。 | `@pure` |
| `Result.expect` | `fn expect<T, E: Display>(self: Result<T, E>, message: Str) -> T` | `Err` で `panic`（開発用）。 | `effect {debug}` |
| `Result.to_option` | `fn to_option<T, E>(self: Result<T, E>) -> Option<T>` | `Err` を捨てて `Option` 化。 | `@pure` |
| `Result.from_option` | `fn from_option<T, E>(opt: Option<T>, err: E) -> Result<T, E>` | 代替エラーを付与。 | `@pure` |
| `ensure` | `fn ensure(cond: Bool, err: () -> E) -> Result<(), E>` | 条件が偽なら `Err(err())`。 | `@pure` |
| `ensure_not_null` | `fn ensure_not_null<T>(ptr: Option<T>, err: () -> E) -> Result<T, E>` | `Option` から `Result` へ昇格。 | `@pure` |

- `expect` 系は `effect {debug}` のみを要求し、本番ビルドでは使用を禁止する lint を用意する。`panic` は 0-1 章で述べた通りデバッグ用途でのみ許容される。【F:0-1-overview.md†L90-L100】
- `ensure` はガード節やテンプレート DSL で利用する共通ヘルパ。`ensure_not_null` は FFI やプラグインから渡されるポインタ検証用。【F:1-3-effects-safety.md†L228-L268】

### 2.3 伝播演算子 `?`

- `expr?` は `Result<T, E>` または `Option<T>` を返す式に適用でき、`Err`/`None` を検出した瞬間に現在の関数・ブロックを同型のエラーで終了する。【F:1-1-syntax.md†L276-L295】
- `?` は `Core.Prelude` が定義する `Try` トレイト相当の内部インターフェイスにより実装される。標準ライブラリでは `Result` と `Option` のみが `Try` を実装し、外部型が拡張する場合は `effect {unsafe}` 承認を要する。
- `expr?` を含む式は `@pure` を保つが、`Result` が `effect` を伴う計算（`effect {io}` 等）を含む場合は呼び出し側の関数も同じ効果タグを要求する。

### 2.4 パターン補助とパイプ連携

| 関数 | シグネチャ | 用途 |
| --- | --- | --- |
| `Result.match` | `fn match<T, E, R>(self: Result<T, E>, ok: (T) -> R, err: (E) -> R) -> R` | `match` 式を関数化し、パイプ内で利用。 |
| `Result.tap_ok` | `fn tap_ok<T, E>(self: Result<T, E>, f: (T) -> ()) -> Result<T, E>` | 成功値を観察し副作用を実行（`effect` を転写）。 |
| `Result.tap_err` | 同上 | エラー観察。 |
| `Option.match` | `fn match<T, R>(self: Option<T>, some: (T) -> R, none: () -> R) -> R` | `Option` 版。 |
| `Option.tap_some` | `fn tap_some<T>(self: Option<T>, f: (T) -> ()) -> Option<T>` | 値を消費せず観察。 |
| `Option.unwrap_or_else` | `fn unwrap_or_else<T>(self: Option<T>, default: () -> T) -> T` | 遅延評価の代替値。 |

- `tap_*` は返り値をそのまま返し、副作用を `effect` タグとして伝搬する。監査ログ出力や計測に利用することを想定。`Result.tap_err` は `effect {audit}` を明示すれば監査 API と安全に連携できる。【F:2-5-error.md†L60-L87】
- `match` 関数は `|>` パイプと組み合わせることで、DSL 内でも宣言的な分岐を保つ。

## 3. 反復子 API（Core.Iter）

### 3.1 `Iter<T>` の性質

- `Iter<T>` は遅延評価される単方向列。`Iterator` のように `next(self) -> Option<T>` を内部的に持つが、Reml の `|>` パイプと親和性の高い関数ベース API を提供する。【F:1-1-syntax.md†L291-L339】
- `Iter` 自体はイミュータブル。内部でキャッシュやバッファを持たないため、必要なら `Iter.buffered(size)` のような明示的 API を利用する。
- すべての変換関数は遅延で、終端操作（`collect`/`fold` 等）が呼ばれるまで評価されない。終端操作は引き続き `@pure` を保つが、供給元が `effect` を要求する場合は `Iter` の構築時点でタグが付与される。

### 3.2 生成関数

```reml
fn empty<T>() -> Iter<T>
fn once<T>(value: T) -> Iter<T>
fn repeat<T>(value: T) -> Iter<T>
fn from_list<T>(values: List<T>) -> Iter<T>
fn from_result<T, E>(value: Result<T, E>) -> Iter<T> // Ok -> 単一要素, Err -> 空 + エラー保留
fn range(start: Int, end: Int, step: Int = 1) -> Iter<Int>
fn unfold<S, T>(state: S, f: (S) -> Option<(S, T)>) -> Iter<T>
fn try_unfold<S, T, E>(state: S, f: (S) -> Result<Option<(S, T)>, E>) -> Result<Iter<T>, E>
```

- `from_result` は `Err` の場合に `Iter.try_collect` など終端操作までエラーを保留し、短絡時に伝播させる。`Result` と `Iter` の橋渡しを担う。
- `unfold` は `Option` を返すクロージャで制御し、`None` を返したら列が終了する。エラーを伴う場合は `try_unfold` を用い、`Result` を通じて呼び出し側へ伝播する。

### 3.3 変換アダプタ

| 関数 | シグネチャ | 説明 |
| --- | --- | --- |
| `Iter.map` | `fn map<T, U>(self: Iter<T>, f: (T) -> U) -> Iter<U>` | 各要素を変換。 |
| `Iter.filter` | `fn filter<T>(self: Iter<T>, pred: (T) -> Bool) -> Iter<T>` | 条件保持。 |
| `Iter.filter_map` | `fn filter_map<T, U>(self: Iter<T>, f: (T) -> Option<U>) -> Iter<U>` | `Option` を畳み込み。 |
| `Iter.flat_map` | `fn flat_map<T, U>(self: Iter<T>, f: (T) -> Iter<U>) -> Iter<U>` | ネストを展開。 |
| `Iter.scan` | `fn scan<T, S>(self: Iter<T>, state: S, f: (S, T) -> (S, Option<U>)) -> Iter<U>` | 状態付き変換。 |
| `Iter.take` | `fn take<T>(self: Iter<T>, n: Int) -> Iter<T>` | 先頭 n 件。 |
| `Iter.drop` | `fn drop<T>(self: Iter<T>, n: Int) -> Iter<T>` | 先頭 n 件を破棄。 |
| `Iter.enumerate` | `fn enumerate<T>(self: Iter<T>) -> Iter<(Int, T)>` | インデックス付与。 |
| `Iter.zip` | `fn zip<A, B>(self: Iter<A>, other: Iter<B>) -> Iter<(A, B)>` | ペア化。 |
| `Iter.buffered` | `fn buffered<T>(self: Iter<T>, size: Int) -> Iter<T>` | `size` 件先読み（`effect {mem}`）。 |

- アダプタのシグネチャは `self` を消費し、元のイテレータは以後利用できない（関数型スタイルで所有権を明示する）。
- `buffered` は内部バッファを持つため `effect {mem}` としてメモリ確保を明示する。`size=0` を禁止し、`size` が負の場合は `Result` でエラーを返す。

### 3.4 終端操作

| 関数 | シグネチャ | 戻り値 | 備考 |
| --- | --- | --- | --- |
| `Iter.collect_list` | `fn collect_list<T>(self: Iter<T>) -> List<T>` | 完全リスト | `@pure` |
| `Iter.collect_vec` | `fn collect_vec<T>(self: Iter<T>) -> Vec<T>` | 可変ベクタ | `effect {mut}` |
| `Iter.fold` | `fn fold<T, S>(self: Iter<T>, init: S, f: (S, T) -> S) -> S` | 畳み込み | `@pure` |
| `Iter.reduce` | `fn reduce<T>(self: Iter<T>, f: (T, T) -> T) -> Option<T>` | 空列対策 | `@pure` |
| `Iter.all` | `fn all<T>(self: Iter<T>, pred: (T) -> Bool) -> Bool` | 全要素判定 | `@pure` |
| `Iter.any` | `fn any<T>(self: Iter<T>, pred: (T) -> Bool) -> Bool` | 1 件でも真 | `@pure` |
| `Iter.find` | `fn find<T>(self: Iter<T>, pred: (T) -> Bool) -> Option<T>` | 要素探索 | `@pure` |
| `Iter.try_fold` | `fn try_fold<T, S, E>(self: Iter<T>, init: S, f: (S, T) -> Result<S, E>) -> Result<S, E>` | 途中で失敗したら即座に終了。 | `@pure` |
| `Iter.try_collect` | `fn try_collect<T, C, E>(self: Iter<Result<T, E>>, builder: Collector<T, C>) -> Result<C, E>` | `Result` を包含した列の収集。 | `@pure` |

- `Collector<T, C>` は `Core.Iter` が提供するビルダインターフェイスで、`Vec`/`Set`/`Map` 等の収集先を抽象化する。`Collector::push` が `Err` を返した場合、`Iter.try_collect` 全体が伝播する。
- `Collector` は次の最小インターフェイスを持つ。`CollectError` は各収集先が定義する任意型であり、`Result` として呼び出し側へ伝播される。

```reml
trait Collector<T, C> {
  fn new() -> Self;
  fn push(self: &mut Self, value: T) -> Result<(), CollectError>;
  fn finish(self) -> C;
}
```

- `try_fold`/`try_collect` は `Result` ベースの短絡を提供し、`?` 演算子と組み合わせることで構造化エラー処理を維持できる。

### 3.5 パイプライン使用例

```reml
use Core;

fn sum_positive(xs: List<Int>) -> Result<Int, Diagnostic> =
  xs
    |> Iter.from_list
    |> Iter.map(|x| ensure(x >= 0, || Diagnostic::invalid_value(x))?)
    |> Iter.try_fold(0, |acc, x| Ok(acc + x))
```

- `ensure` が返す `Result` を `?` で伝播しつつ、`Iter.try_fold` で集計することで「例外なし」「宣言的スタイル」の原則を保つ。【F:0-1-overview.md†L90-L109】【F:1-3-effects-safety.md†L228-L268】
- `Iter.try_fold` 内のクロージャが `Err` を返した場合、残りの要素は評価されない。これにより診断や監査で必要な早期中断を行える。

## 4. 相互運用と今後の課題

1. `Core.Collections` で定義する永続リスト／マップと `Iter` のブリッジ（`Iter.collect_map` 等）を追加する。フェーズ3 内で同時策定予定。【F:4-0-standard-library-scope.md†L33-L41】
2. CLI/診断ガイドでは `Result.tap_err` と監査 API の連携サンプルを拡充し、`audit_id` との結び付けを明示する。フェーズ4 で追記。
3. `Core.Async`（将来拡張）の `Stream` 型と互換のアダプタ（`Iter.from_stream` 等）を調査メモに記載する。`effect {io.async}` の扱いが課題。

