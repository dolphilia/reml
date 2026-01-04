# 3.1 Core Prelude & Iteration

> 目的：Reml の「例外なし」「左→右パイプ」「宣言的スタイル」を支える基本 API を標準化し、全ての DSL から同一の `Option`/`Result`/`Iter` モデルを利用できるようにする。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `@pure`, `effect {mut}`, `effect {mem}`, `effect {debug}` |
| 依存モジュール | なし（基盤モジュール） |
| 相互参照 | [1.1 構文仕様](1-1-syntax.md), [1.3 効果システム](1-3-effects-safety.md), [3.2 Core Collections](3-2-core-collections.md) |

## 1. モジュール構成と import 規則

- `use Core;` で `Core.Prelude` と `Core.Iter` を一括導入できる。既存の Chapter 2 モジュール（`Core.Parse`）と同様、Prelude は**型推論を阻害しない軽量な宣言**のみを公開する。【F:0-1-project-purpose.md†L90-L109】
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
| `Option.ok_or` | `fn ok_or<T, E>(self: Option<T>, err: () -> E) -> Result<T, E>` | 欠損時に遅延評価したエラーを付与。 | `@pure` |
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

`Option.ok_or` と `Result.from_option` は `Map.get` などが返す `Option` を `Result` へ持ち上げる際の共通パターンを吸収し、Lisp/PL/0 などの DSL パーサで頻出する「存在しないキー」の扱いを 1 行で表現できるようにした。エラーメッセージを遅延評価できるため、`format` のコストを必要時まで遅らせることができる。【F:../examples/language-impl-samples/reml/mini_lisp_combinator.reml†L81-L92】

- `expect` 系は `effect {debug}` のみを要求し、本番ビルドでは使用を禁止する lint を用意する。`panic` は 0-1 章で述べた通りデバッグ用途でのみ許容される。【F:0-1-project-purpose.md†L90-L100】
- `ensure` はガード節やテンプレート DSL で利用する共通ヘルパ。`ensure_not_null` は FFI やプラグインから渡されるポインタ検証用。【F:1-3-effects-safety.md†L228-L268】

### 2.3 伝播演算子 `?`

- `expr?` は `Result<T, E>` または `Option<T>` を返す式に適用でき、`Err`/`None` を検出した瞬間に現在の関数・ブロックを同型のエラーで終了する。【F:1-1-syntax.md†L276-L295】
- `?` は `Core.Prelude` が定義する `Try` トレイト相当の内部インターフェイスにより実装される。標準ライブラリでは `Result` と `Option` のみが `Try` を実装し、外部型が拡張する場合は `effect {unsafe}` 承認を要する。
- `expr?` を含む式は `@pure` を保つが、`Result` が `effect` を伴う計算（`effect {io}` 等）を含む場合は呼び出し側の関数も同じ効果タグを要求する。
- `Never` を返す関数では `Result<Never, E>` を利用することで「到達不能」分岐を表現し、`?` による早期リターンと型推論の両立を図る。`Never` を経由したケースは exhaustiveness チェックを満たすため、診断に余計なハンドラを追加する必要がない。

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

### 2.5 診断・監査との連携

- `Result.tap_err` と `ensure` を併用することで、失敗時に `Diagnostic` を加工しつつ元のエラー型を維持できる。監査ログを出力する場合は `effect {audit}` を付与し、Chapter 3.6（Core Diagnostics & Audit）の共通語彙と整合させる。【F:notes/core-library-outline.md†L17-L19】【F:2-5-error.md†L50-L83】
- `Option.match` / `Result.match` の戻り値に `Diagnostic` や `AuditEvent` を割り当てることで、CLI・LSP 双方の出力整形に必要なメタデータを付与できる。`change_set` や `audit_id` の注入は共通ヘルパで次章以降に定義予定。
- `panic` に依存せず `Result` ベースで情報を集約することがコア哲学であり、`try_collect` 等の終端操作でまとめて報告するワークフローを推奨する。【F:notes/core-library-scope.md†L1-L46】

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
- `Collector` は関連型 `Error` を通じて収集時エラーを表現し、`IntoDiagnostic` トレイト経由で診断システムと連携する。
- `with_capacity` はメモリ事前確保により効率化を図り、`reserve` は動的拡張をサポートする。
- `finish` は所有権を消費して結果を返し、`into_inner` は型変換のみを行う軽量版として提供される。

```reml
trait Collector<T, C> {
  type Error: IntoDiagnostic;

  fn new() -> Self;                                                     // `@pure`
  fn with_capacity(capacity: usize) -> Self;                            // `effect {mem}`
  fn push(self: &mut Self, value: T) -> Result<(), Self::Error>;         // `effect {mut}`
  fn reserve(self: &mut Self, additional: usize) -> Result<(), Self::Error>; // `effect {mut, mem}`
  fn finish(self) -> C;                                                 // `effect {mem}`
  fn into_inner(self) -> C;                                             // `@pure`
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

- `ensure` が返す `Result` を `?` で伝播しつつ、`Iter.try_fold` で集計することで「例外なし」「宣言的スタイル」の原則を保つ。【F:0-1-project-purpose.md†L90-L109】【F:1-3-effects-safety.md†L228-L268】
- `Iter.try_fold` 内のクロージャが `Err` を返した場合、残りの要素は評価されない。これにより診断や監査で必要な早期中断を行える。

### 3.6 Collections / Text への橋渡し

- `Iter.collect_list` や `Iter.try_collect` の戻り値は Chapter 3.2 で定義する永続コレクション、および 3.3 で定義する `String`/`GraphemeSeq` と組み合わせられる想定である。【F:notes/core-library-outline.md†L13-L16】
- 可変コンテナ（`Vec`/`Cell`）を収集先とする場合、`Collector` 実装が `effect {mut}` を宣言し、`Iter` 側はタグを転写する。これにより `mut` 効果を局所化しつつ宣言的パイプラインを維持できる。
- Unicode 分解・正規化は `Iter.map`/`Iter.flat_map` と `Core.Text` の helper を接続することで段階的に適用でき、Lex レイヤでの字句検査とも互換となる。【F:notes/core-library-scope.md†L7-L24】

#### 3.6.1 標準コレクションへの収集契約

- `Iter.collect_list`/`Iter.collect_vec` は入力イテレータの訪問順をそのまま保持し、`List`/`Vec` での添字アクセスや診断表示がパイプラインの実行順と一致するように保証する。これにより「実用に耐える性能」を満たしつつ、再実行時のトレース容易性を確保する。【F:0-1-project-purpose.md†L11-L37】
- `MapCollector` は内部でキーを `Ord` に従って挿入し、永続 `Map` が常に昇順イテレーションを返す前提を維持する。診断ログや監査ログで差分を比較する際に行番号がぶれないよう、`Map.try_collect` 系はキー競合時に `CollectError::DuplicateKey` を返す契約を共有する。【F:3-6-core-diagnostics-audit.md†L42-L88】
- `ListCollector`/`SetCollector` は構造共有を前提とした参照カウントを維持し、`Iter` パイプライン側からは `@pure` 契約を壊さない。`List.fold` と `Iter.fold` はどちらも左結合で評価し、DSL 作者が `List` ベースと `Iter` ベースを同じ計算モデルとして学習できるようにする。【F:0-1-project-purpose.md†L29-L44】【F:3-2-core-collections.md†L24-L86】
- `Iter.from_list` → `Iter.try_collect(MapCollector)` のような二段変換では、中間リストが保持していた順序が `Map` でソートされる点を仕様上で明示し、`Table` や `Vec` に収集するパスと挙動を比較しやすくする。必要に応じて `TableCollector` を用いることで挿入順を保持したまま DSL のエラーメッセージを生成できる。

#### 3.6.2 Iterator Stage 監査と辞書メタデータ

- `Iterator` 系のトレイト辞書は `StageRequirement::{Exact | AtLeast}` と `CapabilityId` を保持し、型推論フェーズから Core IR まで `effect.stage.iterator.*` を伝播させる。辞書生成時に `IteratorDictInfo`（実装詳細）が作成され、`required` / `actual` / `kind` / `capability` / `source` を記録する。【F:1-2-types-Inference.md†L90-L130】
- `DictMethodCall` はループヘッダ／ボディに `EffectMarker` を付与し、監査ログ (`AuditEnvelope.metadata`) および `Diagnostic.extensions.effects.iterator.*` に同一キーで出力する。これにより [`collect-iterator-audit-metrics.py`](../../tooling/ci/collect-iterator-audit-metrics.py) が Stage 不整合を自動検出できる。
- CI では [`tooling/ci/sync-iterator-audit.sh`](../../tooling/ci/sync-iterator-audit.sh) がメトリクス JSON と `scripts/verify_llvm_ir.sh` の出力ログを突合し、`0-3-audit-and-metrics.md` に貼り付け可能な Markdown レポートを生成する。pass_rate < 1.0 または `verify` ログに失敗が含まれる場合、スクリプトは非ゼロで終了しフェイルファストする。

### 3.7 効果許容ポリシーと `@pure` 両立サンプル（実験段階）

Prelude/Iter は `@pure` を基本としつつ、効果ハンドラ経由で副作用付き処理を分離できる設計とする。

```reml
@handles(Console)
pub fn collect_logs(iter: Iter<Text>) -> Result<List<Text>, Diagnostic> ! {} =
  handle iter.try_fold(List::empty(), |acc, msg| {
    do Console.log(msg)
    Ok(acc.push(msg))
  }) with
    handler Console {
      operation log(msg, resume) {
        audit.log("iter.log", msg)
        resume(())
      }
      return result {
        result
      }
    }
```

- `Iter.try_fold` 自体は `@pure` を維持し、外部効果をハンドラへ委譲する。残余効果が空の場合、呼び出し側は `@pure` な関数として扱える。
- `@handles` を付与した関数に `stage` を要求する場合は `@requires_capability(stage="experimental")` を併用し、Capability Registry で opt-in した環境でのみ利用可能にする。

効果許容ポリシーは以下を原則とする。

1. Prelude/Iter が提供する公開 API は既定で `@pure`。内包処理で効果が必要な場合はハンドラで捕捉する。
2. `Iter` のクロージャが効果を発生させる場合でも、残余効果集合 `Σ_after` が空であれば `Iter` 利用側で `@pure` 契約を維持できる。
3. 実験的効果を扱う補助 API は `Stage` を記録し、`../notes/algebraic-effects-implementation-roadmap-revised.md` の昇格手順に従って安定化する。


## 4. 高度な収集操作

### 4.1 専用コレクタ

```reml
struct ListCollector<T>;
struct VecCollector<T>;
struct MapCollector<K, V>;
struct SetCollector<T>;
struct StringCollector;

fn collect_list<T>(iter: Iter<T>) -> List<T>                           // `@pure`
fn collect_vec<T>(iter: Iter<T>) -> Result<Vec<T>, MemoryError>         // `effect {mut, mem}`
fn collect_map<K: Ord, V>(iter: Iter<(K, V)>) -> Result<Map<K, V>, CollectError> // `@pure`
fn collect_set<T: Ord>(iter: Iter<T>) -> Result<Set<T>, CollectError>   // `@pure`
fn collect_string(iter: Iter<char>) -> Result<String, StringError>      // `effect {mem}`
```

> **NOTE**: `Set<T>` の API は [3.2 Core Collections](3-2-core-collections.md) に準拠し、実行時表現の詳細は [3.2 §2.2.1](3-2-core-collections.md#set-runtime-abi) を参照。

### 4.2 カスタムコレクタの実装例

```reml
struct HistogramCollector {
  buckets: Map<Range<f64>, u32>,
}

impl Collector<f64, Histogram> for HistogramCollector {
  type Error = HistogramError;

  fn new() -> Self {
    Self { buckets: Map::empty() }
  }

  fn push(self: &mut Self, value: f64) -> Result<(), Self::Error> {
    let bucket = self.find_bucket(value)
      .ok_or(HistogramError::OutOfRange(value))?;
    self.buckets = self.buckets.update(bucket, |count| count.unwrap_or(0) + 1);
    Ok(())
  }

  fn finish(self) -> Histogram {
    Histogram::new(self.buckets)
  }
}
```

## 5. パフォーマンス考慮事項

### 5.1 遅延評価の最適化

- `Iter` チェーンは実際に終端操作が呼ばれるまで評価されない。
- `buffered` オペレーターで先読みバッファを調整できる。
- `collect` 系操作では事前容量指定により再配置コストを削減する。

### 5.2 メモリ使用量の制御

```reml
fn process_large_dataset(data: Iter<Record>) -> Result<Summary, ProcessError> =
  data
    |> Iter.buffered(1000)  // 1000件先読みバッファ
    |> Iter.map(validate_record)
    |> Iter.try_fold(Summary::empty(), |summary, record| {
        summary.update(record?)
      })
```

## 6. 相互運用と将来拡張

### 6.1 標準ライブラリとの連携

- [3.2 Core Collections](3-2-core-collections.md) で定義する永続データ構造との双方向変換
- [3.3 Core Text & Unicode](3-3-core-text-unicode.md) での文字列処理パイプライン
- [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md) での監査ログ統合

### 6.2 非同期処理との将来統合

```reml
// 将来の async Stream との統合例（予定）
fn from_async_stream<T>(stream: AsyncStream<T>) -> Iter<Future<T>>  // `effect {io.async}`
fn to_async_stream<T>(iter: Iter<T>) -> AsyncStream<T>              // `effect {io.async}`
```

### 6.3 使用例リンク

- `Option`/`Result` の `tap` 系ヘルパと `Iter.try_collect` の組み合わせサンプルは [3.2 Core Collections](3-2-core-collections.md#7-使用例iter-パイプライン) を参照。
- Unicode 正規化／Lex 連携を含む文字列処理の例は [3.3 Core Text & Unicode](3-3-core-text-unicode.md#8-使用例lex-連携と-grapheme-操作) を参照。
