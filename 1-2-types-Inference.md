# 1.2 型と推論（Types & Type Inference）— Kestrel 言語コア仕様

> 目的：**書きやすさ・読みやすさ・高品質エラー**を壊さず、**実用性能**と**静的安全**を両立。
> 方針：**サブタイピングなし**（HM 系の推論をシンプルに保つ）。**ランク1の多相**を基本に、**型クラス風トレイト**で演算子等の静的オーバーロードを提供。

---

## A. 型の群（Type Language）

### A.1 プリミティブ

* 整数：`i8 i16 i32 i64 isize` / `u8 u16 u32 u64 usize`
* 浮動小数：`f32 f64`
* 真偽：`Bool`
* 文字：`Char`（Unicode スカラ値）
* 文字列：`String`（不変・UTF-8）
* 単位：`()`

### A.2 合成

* タプル：`(T1, T2, …, Tn)`（`n≥0`。`()` は単位）
* 配列（固定長）：`[T; N]`
* スライス／動的配列：`[T]`（標準ライブラリ型として提供、実体は `{ptr,len}`）
* レコード：`{ x: T1, y: T2, ... }`（構造的等値）
* 関数：`(A1, A2, …, An) -> R`（右結合、`A -> B -> C` ≡ `A -> (B -> C)`）
* 代数的データ型（ADT）：

  ```kestrel
  type Option<T> = | Some(T) | None
  type Result<T,E> = | Ok(T) | Err(E)
  ```

  *各コンストラクタは関数型を持つ：`Some : T -> Option<T>`*

### A.3 型変数・スキーム

* 型変数：小文字開始（`a, b, t1 …`）。
* **型スキーム**：`∀a1 … an. τ`（実装上は `Scheme{quantified: [a…], body: τ}`）。
* **多相はランク1**が既定（関数引数にスキームを直接入れない）。高ランクは将来拡張（明示注釈時のみ）。

### A.4 型エイリアス & ニュータイプ

* **エイリアス**（同義）：`type alias Id = i64`
* **ニュータイプ**（零コストの別名型）：`type UserId = new i64`（暗黙変換なし）

### A.5 種（Kind）（必要最小）

* `*`（具体型）／`* -> *`（型コンストラクタ）等。ADT 定義で内的に整合性を検査（ユーザ記述は不要）。

---

## B. トレイト（型クラス風）と静的オーバーロード

> **実装ステージング**: MVP（最小実装）では基本演算子のトレイトのみ、本格実装でユーザ定義トレイト、完全実装で辞書パッシングによる完全なtypeclass相当機能

### B.1 トレイト宣言（概略）

```kestrel
trait Add<A, B, R> { fn add(a: A, b: B) -> R }
impl Add<i64, i64, i64> for i64 { fn add(a,b) = a + b }
```

* **目的**：演算子・汎用 API の静的解決（Haskell の typeclass に近い）。
* **演算子**はトレイトに紐づく：`+` は `Add`、`-` は `Sub`、`*` は `Mul`、`/` は `Div`…（Nest.Parse.Op に合わせて標準定義）。
* **MVP（最小実装）**: 基本算術・比較演算子の組み込みトレイトのみ（i64, f64, Bool, String対応）
* **本格実装**: ユーザ定義トレイト、where制約、制約解決
* **完全実装**: 辞書パッシング、高階型クラス、特殊化

### B.2 解決と整合性

* **コヒーレンス**：`impl` は **トレイト定義モジュール**か**対象型のモジュール**のどちらかにのみ書ける（孤児規則で衝突防止）。
* **オーバーラップ禁止**（デフォルト）。将来 `where` 制約付きの安全な特殊化を検討。

### B.3 トレイト制約の表記

* 関数型に **制約**を付与：

  ```kestrel
  fn sum<T>(xs: [T]) -> T where Add<T,T,T>, Zero<T> = ...
  ```

  *推論中は**制約集合**として保持され、呼出側で解決／辞書渡しに具体化。*

---

## C. 型推論（Inference）

### C.1 基本戦略

* **Hindley–Milner（Algorithm W）** + 制約解決。
* **サブタイピングなし**、**ユニオン/インターセクションなし**（単純化）。
* 変数束縛 `let` で **一般化（generalization）**、使用時に **インスタンス化**。

### C.2 変数の“剛性”

* **柔軟（unification var）**：推論中に他型と単一化される。
* **剛体（rigid/スコープ外）**：注釈や `forall` で導入された量化変数は **occurs check** を厳密化し、誤推論を防ぐ。

### C.3 値制限（Value Restriction）

* **一般化は“確定的な値”のみ**：

  * 右辺が **ラムダ・コンストラクタ・数値/文字列リテラル・純式** → 一般化可。
  * **可変参照・I/O・外部呼び出し**を含む可能性がある右辺は **単相**（将来の効果システムで形式化；1.3 参照）。

### C.4 アノテーション

* **任意**（ローカル）／**推奨**（公開 API）。
* アノテがある場合は **双方向型付け**（bidirectional）で誤差を小さくし、エラー品質を上げる。

### C.5 演算子・リテラルの既定

* **数値リテラル**は `Num<T>` 制約を持つ多相リテラル。曖昧時はデフォルト `i64` / `f64`（小数点の有無で分岐）。
* **演算子**は対応トレイトで解決。`a + b` は `Add<typeof a, typeof b, r>` の `r` を新鮮変数で導入し、単一化。

### C.6 失敗時の方針（エラー）

* **期待/実際**・**候補トレイト**・**不足制約**を列挙。
* 量化変数が関係する場合は **“ここで一般化/インスタンス化が必要”** を示す。
* 位置は **式ごとに最狭スパン**で報告（Nest.Parse.Err と連携）。

---

## D. パターンの型付け

### D.1 パターン規則

* `let (x, y) = e`：`e : (a, b)` を要求し、`x:a`, `y:b` を導入。
* レコード：`{x, y: y0}` は `{x: a, y: b}` と単一化、`x:a`, `y0:b`。
* コンストラクタ：`Some(x)` は `e : Option<a>`、`x:a`。
* ガード：`if cond` は `cond : Bool`。

### D.2 網羅性（型付け段階の情報）

* `match` の各分岐で **スクラッティの型**と**残余集合**を追跡。
* **非網羅**は警告/エラーのポリシーを切替（1.3 で最終決定）。

---

## E. モジュールと汎化境界

* **トップレベル `let`** はモジュール境界で一般化。
* `pub` シンボルは **公開型**で確定（型変数は外向けに量化）。
* `use` により導入されたトレイト/型は **名前解決表**に登録され、推論時に探索対象となる。

---

## F. 代表的な型（標準 API・コンビネータ想定）

> パーサーコンビネータ記述が短くなるように、要の関数型は**一読で意図が分かる**シグネチャに。

```kestrel
// Parser 型（簡略）
type Parser<T>    // 実体は Input -> Result<T, ParseError>

// コア・コンビネータ（抜粋）
fn map<A,B>(p: Parser<A>, f: A -> B) -> Parser<B>
fn then<A,B>(p: Parser<A>, q: Parser<B>) -> Parser<(A,B)>
fn or<A>(p: Parser<A>, q: Parser<A>) -> Parser<A>
fn many<A>(p: Parser<A>) -> Parser<[A]>
fn chainl1<A>(term: Parser<A>, op: Parser<(A,A)->A>) -> Parser<A>
fn between<A>(l: Parser<()>, p: Parser<A>, r: Parser<()>) -> Parser<A>

// 典型的な型推論の例
let int  = digit.many1().map(parseInt)            // Parser<i64>
let atom = or(int, between(sym("("), expr, sym(")")))
let expr = chainl1(atom, addOp)                   // Parser<i64>
```

---

## G. 実装上の規約（コンパイラ側）

1. **単一化（unify τ1 τ2）**：対称・逐次、**occurs check** あり。
2. **一般化**：`let x = e` の型 `τ` から、**外スコープに自由な変数**を除いた集合を量化。
3. **インスタンス化**：使用時に量化変数を新鮮変数へ置換。
4. **制約収集**：トレイト制約は `C = {Add<a,b,r>, …}` の集合として保持。
5. **制約解決**：

   * **第一段**：具体型が決まるたびに `impl` テーブルで一致検索（単一解であること）。
   * **第二段**：残余があれば**呼出側へエスカレーション**（関数型の `where` へ持ち上げ）。
6. **デフォルト**：残余が数値リテラルのみなら `i64`/`f64` を割当（曖昧ならエラー）。

---

## H. 例（推論の挙動）

### H.1 let 一般化

```kestrel
let id = |x| x               // id : ∀a. a -> a
let n  = id(42)              // inst a := i64 → i64
let s  = id("hi")            // inst a := String → String
```

### H.2 制約の持ち上げ

```kestrel
fn sum<T>(xs: [T]) -> T where Add<T,T,T>, Zero<T> =
  fold(xs, init=zero(), f=Add::add)
```

呼出側：

```kestrel
let r1 = sum([1,2,3])        // T := i64, 既存 impl で解決
let r2 = sum(users)          // エラー（Add<User,User,User> が未定義）
```

### H.3 演算子の推論

```kestrel
let f = |x, y| x + y         // 収集: Add<a,b,r>; 型: a -> b -> r
let g = |n| n + 1            // 収集: Add<a,i64,r>; 不足 → 呼出で決定
```

### H.4 数値リテラルの既定

```kestrel
let a = 10        // a : i64
let b = 10.0      // b : f64
let c: f32 = 10   // 単一化で c : f32（数値多相の縮退）
```

---

## I. エラーメッセージの形（例）

* **型不一致**

  ```
  type error: expected i64, found String
    --> main.ks:12:17
     12 | let n: i64 = "42"
                     ^^^^^^ expected i64 here
  ```
* **不足トレイト**

  ```
  constraint error: cannot resolve Add<User,User,User>
    --> calc.ks:7:12
     7 | users |> sum
               ^^^ requires Add<User,User,User> and Zero<User>
     help: define `impl Add<User,User,User>` or annotate with a concrete type
  ```
* **汎化の値制限**

  ```
  generalization blocked: expression may be effectful
    --> parse.ks:3:9
     3 | let p = readLine() |> map(...)
             ^ consider adding a type annotation or using a pure binding
  ```

---

## J. ドメイン型拡張（Draft）

> データパイプラインや機械学習 DSL、クラウド設定などで必要となる型を言語レベルで扱うための拡張案。フェーズ2以降で標準 API と連携しつつ精緻化する。

1. **テンソル型**
   - 型表記: `Tensor<Shape, T>`。`Shape` は `Vec<usize>` もしくは定数配列型。
   - 基本規則: `Tensor<S, T> + Tensor<S, T> -> Tensor<S, T>`。異なる `Shape` の場合は `TensorOp` trait の制約解決を要求。
   - ブロードキャスト: `Tensor<[m, n], T> + Tensor<[1, n], T>` のようなケースでは `Broadcast<S, R>` trait を導入し、`Shape` の互換性を判定する。

2. **列型 / スキーマ型**
   - 列型表記: `Column<T, Meta>`。`Meta` は統計情報や制約を表す（例: `Meta<{ nullable = false }>`）。
   - スキーマ型: `Schema<{ field1: Column<T1>, field2: Column<T2>, ... }>`。
   - スキーマ差分: `SchemaDiff<Old, New>` を型レベルで生成し、差分適用 DSL で利用する。

3. **リソース ID 型**
   - 表記: `Resource<P, K>`。`P`（Provider）と `K`（Kind）を型レベルタグとし、異なるプロバイダ間の混用を防止。
   - 例: `Resource<Aws, S3Bucket>`, `Resource<Gcp, PubSubTopic>`。
   - 型制約: `ResourceOps<P, K>` trait で操作を限定し、不正な FFI 呼び出しを型で検出。

4. **効果タグ付き関数型**
   - 記法案: `fn(args) -> T effect {db, audit}`。
   - 推論: 効果タグは呼び出しチェーンで和集合を形成し、`@requires(effect)` 属性で静的検査。
   - `1-3-effects-safety.md` の拡張効果分類と連携。

5. **スキーマ進化の推論規則**
   - `Schema<A>` と `Schema<B>` の単一化に失敗した場合、`SchemaDiff<A,B>` 制約を生成。
   - マイグレーション DSL は `SchemaDiff` を解決する `upgrade` / `downgrade` 関数を要求。

#### サンプル（Draft）

```kestrel
schema DbConfig {
  url: Column<String>
  pool_size: Column<i32> = 8
}

fn migrate(cfg: Schema<DbConfig>) -> Schema<DbConfig>
  effect {config, audit} = {
  cfg
    .compute(|c| c.pool_size = max(c.pool_size, 4))
    .requires(|c| c.url.startsWith("postgres://"))
}

fn train(model: Tensor<[batch, features], f32>, weights: Tensor<[features, 1], f32>)
  -> Tensor<[batch, 1], f32>
  effect {gpu} = model.matmul(weights)
```

### 推論規則（案）

* テンソル演算: `TensorOp` trait の形で演算をモジュール化し、`Shape` の整合性やブロードキャスト可否を制約解決で判断。
* スキーマ型: フィールドアクセス `config.database.url` は `Schema<...>` から `Column<String>` に推論し、`requires` 句は `Constraint` trait で実行。
* 効果付き関数: `effect` タグは関数型推論で和集合を形成し、呼び出し側の効果要求に反映。

```kestrel
fn combine_effects(a: EffectSet, b: EffectSet) -> EffectSet =
  a.union(b)

fn call_with_effects<T>(f: Fn -> T effect E, g: Fn -> T effect F) -> T effect combine_effects(E, F)
```

サンプル：

```kestrel
let apply = |cfg: Schema<AppConfig>| -> AppConfig effect {config, audit} {
  audit.log("config.apply", SchemaDiff::between(cfg, cfg))
  cfg.realize()
}

let migrate = |old: Schema<AppConfig>, new: Schema<AppConfig>| {
  match SchemaDiff::between(old, new) with
  | Ok(_)      -> Ok(new)
  | Err(diff)  -> Err(diff)
}

let (_ : SchemaDiff<AppConfig>) =
  SchemaDiff::between(appSchema, prodSchema)
```

---

## K. まとめ（設計の要点）

* **HM + トレイト制約**という最小で強力な骨格。
* **サブタイピングなし**で推論を安定化、**bidirectional + アノテ**でエラー品質を確保。
* **数値多相の既定**と**演算子=トレイト**で、日常コードを短く自然に。
* **一般化の値制限**と**剛体変数**で予期せぬ推論"暴走"を抑止。
* **パターン型付け**・**網羅性**・**制約の持ち上げ**が、Kestrel→Core→IR の変換を素直に支える。

---

## 関連仕様

* [1.1 構文](1-1-syntax.md) - 言語構文の詳細
* [1.3 効果と安全性](1-3-effects-safety.md) - 効果システムとの連携
* [1.4 文字モデル](1-4-test-unicode-model.md) - Unicode型システム
* [2.5 エラー設計](2-5-error.md) - 型エラーの報告
* [a-jit.md](a-jit.md) - LLVM連携での型情報利用
