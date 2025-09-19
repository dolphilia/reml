# 2.2 コア・コンビネータ

> 目的：**小さく強い核**で、書きやすさ・読みやすさ・高品質エラー・実用性能（ゼロコピー／Packrat／左再帰）を同時に満たす。
> 前提：2.1 の型と実行意味（`Reply{consumed, committed}`）に準拠。**Unicode 前提**。
> 方針：\*\*最小公理系（15個前後）**を厳選し、残りは**派生（derived）\*\*として提供。

---

## A. コア（最小公理系）

> これだけで通常のパーサは書ける。各シグネチャは Kestrel 風擬似記法。

### A-1. 基本

```kestrel
fn ok<T>(v: T) -> Parser<T>                    // 成功・非消費
fn fail(msg: String = "") -> Parser<Never>     // 失敗・非消費（期待集合は空）
fn eof() -> Parser<()>                         // 入力末尾のみ成功（非消費）
fn rule<T>(name: String, p: Parser<T>) -> Parser<T> // 名前/ID 付与（Packrat/診断）
fn label<T>(name: String, p: Parser<T>) -> Parser<T> // 失敗時の期待名を差し替え
```

* `eof` は `RunConfig.require_eof` と相補。
* `rule` は **ParserId** を固定化し、メモキーとトレースに使う。

### A-2. 直列・選択

```kestrel
fn then<A,B>(p: Parser<A>, q: Parser<B>) -> Parser<(A,B)>     // 直列
fn andThen<A,B>(p: Parser<A>, f: A -> Parser<B>) -> Parser<B> // = flatMap
fn skipL<A,B>(p: Parser<A>, q: Parser<B>) -> Parser<B>        // 左を捨てる
fn skipR<A,B>(p: Parser<A>, q: Parser<B>) -> Parser<A>        // 右を捨てる
fn or<A>(p: Parser<A>, q: Parser<A>) -> Parser<A>             // 左優先の選択
fn choice<A>(xs: [Parser<A>]) -> Parser<A>                    // 左から順に or
```

* **失敗統合規則**（2.1 準拠）

  * `or` は：`p` が `Err(consumed=true or committed=true)` なら **`q` を試さない**。
  * `p` が **空失敗**（`consumed=false, committed=false`）なら `q` を試す。

### A-3. 変換・コミット・回復

```kestrel
fn map<A,B>(p: Parser<A>, f: A -> B) -> Parser<B>
fn cut<T>(p: Parser<T>) -> Parser<T>                 // p 内の失敗を committed=true に
fn cut_here() -> Parser<()>                           // ゼロ幅コミット
fn attempt<T>(p: Parser<T>) -> Parser<T>              // 失敗時に消費を巻き戻す（空失敗化）
fn recover<T>(p: Parser<T>, until: Parser<()>, with: T) -> Parser<T>
// p 失敗時、入力を until まで読み捨て with で継続（診断を残す）
fn trace<T>(p: Parser<T>) -> Parser<T>                // 追跡ON時のみスパンを収集
```

* **使用指針**

  * 迷ったら **`attempt` を選択分岐の直前**に置く（`try` 相当）。
  * \*\*「ここからはこの構文で確定」\*\*という位置に **`cut_here()`**。
  * エラーから**同期**して処理を続けたい時は **`recover`**。

### A-4. 繰り返し・任意

```kestrel
fn opt<A>(p: Parser<A>) -> Parser<Option<A>>              // 空成功可（非消費）
fn many<A>(p: Parser<A>) -> Parser<[A]>                   // 0回以上
fn many1<A>(p: Parser<A>) -> Parser<[A]>                  // 1回以上
fn repeat<A>(p: Parser<A>, min: usize, max: Option<usize>) -> Parser<[A]>
fn sepBy<A,S>(p: Parser<A>, sep: Parser<S>) -> Parser<[A]>
fn sepBy1<A,S>(p: Parser<A>, sep: Parser<S>) -> Parser<[A]>
fn manyTill<A,End>(p: Parser<A>, end: Parser<End>) -> Parser<[A]>
```

* **無限ループ安全**：`many` 系は **空成功パーサ**を検出したらエラーにする（メッセージ：「繰り返し本体が空成功」）。

### A-5. 括り・前後関係

```kestrel
fn between<A>(open: Parser<()>, p: Parser<A>, close: Parser<()>) -> Parser<A>
fn preceded<A,B>(pre: Parser<A>, p: Parser<B>) -> Parser<B>
fn terminated<A,B>(p: Parser<A>, post: Parser<B>) -> Parser<A>
fn delimited<A,B,C>(a: Parser<A>, b: Parser<B>, c: Parser<C>) -> Parser<B>
```

### A-6. 先読み・否定

```kestrel
fn lookahead<A>(p: Parser<A>) -> Parser<A>          // 成功しても非消費
fn notFollowedBy<A>(p: Parser<A>) -> Parser<()>     // p が失敗すれば成功（非消費）
```

* `lookahead` は**成功しても消費しない**ため、分岐予告や曖昧性解消に有効。
* `notFollowedBy` はキーワード衝突（`ident` だが直後が英数字ならNG 等）に便利。

### A-7. チェーン（演算子の左/右結合）

```kestrel
fn chainl1<A>(term: Parser<A>, op: Parser<(A, A) -> A>) -> Parser<A>
fn chainr1<A>(term: Parser<A>, op: Parser<(A, A) -> A>) -> Parser<A>
```

* **実装規約**：内部で `attempt` を適切に使い、`term op term op ...` の途中失敗が**手前の選択**へ波及しないようにする。
* べき乗など右結合は `chainr1`。

### A-8. スパン・位置

```kestrel
fn spanned<A>(p: Parser<A>) -> Parser<(A, Span)>      // 値とスパン
fn position() -> Parser<Span>                         // ゼロ幅で現在位置
```

* AST 構築で**位置情報**を付与するための基本ユーティリティ。

---

## B. 前後空白（字句インターフェイス）

> 文字モデル/Unicode の扱いは 1.4、Lex は 2.3 で詳細化。

```kestrel
fn padded<A>(p: Parser<A>, space: Parser<()>) -> Parser<A>  // 前後に space を食う
fn lexeme<A>(space: Parser<()>, p: Parser<A>) -> Parser<A>  // 後ろのみ space
fn symbol(space: Parser<()>, s: Str) -> Parser<()>          // 文字列シンボル＋lexeme
```

* **推奨**：`let sc = Lex.spaceOrTabsOrNewlines | Lex.comment... |> Lex.skipMany` を `space` に。
* `symbol(sc, "(")` → `(` を読んで後続の空白/コメントを食う。

---

## C. 便利だが派生（derived）に落とすもの

> コアを太らせないため、以下は **コアの合成**で提供（実装は標準ライブラリ側）。

```kestrel
fn separatedPair<A,B,S>(a: Parser<A>, sep: Parser<S>, b: Parser<B>) -> Parser<(A,B)>
fn tuple2<A,B>(a: Parser<A>, b: Parser<B>) -> Parser<(A,B)>        // ~ then/map
fn list1<A,S>(elem: Parser<A>, sep: Parser<S>) -> Parser<[A]>      // ~ sepBy1
fn atomic<T>(p: Parser<T>) -> Parser<T>                             // = label+cut の糖衣
fn expect<T>(name: String, p: Parser<T>) -> Parser<T>               // = label(name, cut(p))
fn separatedListTrailing<A,S>(elem: Parser<A>, sep: Parser<S>) -> Parser<[A]> // 末尾区切り許容
```

---

## D. 消費／コミットの要点（実務上の指針）

* **分岐の手前に `attempt`**：

  ```kestrel
  attempt(sym("if").then(expr).then(block))
    .or(attempt(sym("while").then(expr).then(block)))
    .or(stmtSimple)
  ```

  → 先頭のキーワード以降で失敗しても、**空失敗**として次の分岐へ進める。
* **「ここからはこの形」→ `cut_here()`**：

  ```kestrel
  sym("let").then(ident).then(cut_here()).then(sym("=").then(expr))
  ```

  → `let x` まで来たら **`=` が絶対必要**。以降の失敗は**コミット済み**として報告。
* **繰り返しの本体は空成功禁止**：`many(p)` の `p` が空成功だと**停止しない**。ライブラリが検出してエラーに。
* **`lookahead` は非消費**：曖昧性の解消・キーワードの後判定に。

---

## E. 例：四則演算（べき乗右結合、カッコ、単項 -）

```kestrel
use Nest.Parse
use Nest.Parse.Lex

let sc     = Lex.spaceOrTabsOrNewlines |> Lex.skipMany
let sym(s) = symbol(sc, s)
let int    = lexeme(sc, Lex.int(10))

let expr: Parser<i64> = rule("expr", chainl1(term, addOp))
let term: Parser<i64> = rule("term", chainl1(factor, mulOp))

let addOp: Parser<(i64,i64)->i64> =
  (sym("+").map(|_| (|a,b| a+b)))
  .or(sym("-").map(|_| (|a,b| a-b)))

let mulOp: Parser<(i64,i64)->i64> =
  (sym("*").map(|_| (|a,b| a*b)))
  .or(sym("/").map(|_| (|a,b| a/b)))

let factor: Parser<i64> = rule("factor",
  (sym("(").then(cut(expr)).then(sym(")")).map(|(_,v,_)| v))   // 括弧に cut
    .or(sym("-").then(factor).map(|(_,x)| -x))                  // 単項 -
    .or(int)
)
```

* `cut(expr)` により、開き括弧の後は**閉じ括弧が必須**。
* べき乗を足すなら `chainr1(base, powOp)` を `term` より上に挿入。

---

## F. エラー品質のための慣習

* **`rule` と `label` を積極的に**：失敗メッセージに人間語が出る。
* **`expect(name, p)`（= `label+cut`）** で「ここは絶対に *name*」を宣言。
* **`recover`** は**同期トークン**を必ず明示（例：`;` や行末）し、診断に「ここまでスキップ」を記録。

---

## G. 仕様チェックリスト

* [ ] コア 8 群（基本／直列選択／変換・コミット／繰返し／括り／先読み／チェーン／スパン・位置）。
* [ ] `attempt` を含め、**消費／コミットの 2bit**と整合する選択規則。
* [ ] `many` 系の**空成功検出**。
* [ ] `eof`・`position`・`spanned` のゼロ幅挙動。
* [ ] `rule` による **ParserId** 固定と診断（Packrat/左再帰に必須）。
* [ ] 前後空白 3 兄弟（`padded/lexeme/symbol`）は Unicode の `Lex` と噛み合う。
* [ ] すべて**純関数**（2.1／1.3 の効果方針）。

---

### まとめ

* **15 個前後の最小コア**で、実務に必要な記法（分岐、繰返し、先読み、コミット、位置）が網羅され、
* `attempt / cut / label / recover` の**四点セット**で **高品質エラー**と**制御可能なバックトラック**を両立。
* 追加の便利関数は **派生**として提供し、コアを痩せたまま保つ。

この仕様を土台に、次は **2.3 字句工具（Nest.Parse.Lex）** で Unicode 前提のトークン化ヘルパを詰めます。
