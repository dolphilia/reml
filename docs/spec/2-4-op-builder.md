# 2.4 演算子優先度ビルダー（Core.Parse.Op）

> 目的：**短い宣言で“正しい木”が立ち、エラーも良い**。
> 方針：最小で強い DSL を用意し、**infix/prefix/postfix/非結合**と**多文字演算子**、**ユニタリ演算子**までをカバー。実装は **Pratt（binding power）× 連鎖畳み込み**のハイブリッド。
> 前提：2.1/2.2/2.3 の型・消費/コミット・字句規約、1.4 の Unicode 文字モデル。

---

## A. コア API（最小面）

### A-1. ビルダーの入口

```reml
// 基本：operand（最下位=原子）から、演算子テーブルで階層を組む
fn precedence<A>(
  operand: Parser<A>,
  config: OpConfig = {}
) -> PrecedenceBuilder<A>
```

* `operand` は「数値/括弧/識別子/呼出/添字/ドット等」の\*\*“非演算子”**を含む**最下位パーサ\*\*。
* `config.operand_label`（省略可）で「被演算子の名前」（エラー文言）を指定できる。
* `config.space: Parser<()>` を与えると**演算子の直後/直前に統一トリビア処理**を適用（未指定なら各演算子側パーサに委ねる）。

### A-2. レベル宣言（fixity）

```reml
// 1 つの“優先度レベル”を追加
PrecedenceBuilder<A>.level(
  |lvl: Level<A>| {
    lvl.prefix(op: Parser<A -> A>)
    lvl.postfix(op: Parser<A -> A>)
    lvl.infixl(op: Parser<(A, A) -> A>)
    lvl.infixr(op: Parser<(A, A) -> A>)
    lvl.infixn(op: Parser<(A, A) -> A>)  // 非結合（a < b < c はエラー）
    lvl.ternary(op: Ternary<A>)          // 任意（下で定義）
  }
) -> PrecedenceBuilder<A>

type Ternary<A> = {
  head: Parser<()>        // 例: '?'
  mid:  Parser<()>        // 例: ':'
  build: (cond: A, t: A, f: A) -> A
}
```

* \*\*レベルは上から“強い → 弱い”\*\*順で積む（最上位が最も結合力が高い）。
* 各 `op` は **文字列トークン**でも**自由なパーサ**でも良い（`keyword`, `symbol`, `choice_longest` 等を使用可）。
* 1 レベル内に複数の演算子を**列挙**できる（`lvl.infixl(plusOp).infixl(minusOp)`）。

### A-3. 完成

```reml
PrecedenceBuilder<A>.build() -> Parser<A>
```

---

## B. 使い方（例）

```reml
let sc     = Lex.spaceOrTabsOrNewlines |> Lex.skipMany
let sym(s) = symbol(sc, s)
let int    = lexeme(sc, Lex.int(10)).map(parseI64!)

// operand: 括弧 / 単項マイナス(後述の prefix レベルでも可) / 数
let atom: Parser<i64> =
  (sym("(").then(cut_here()).then(rec expr).then(expect("')'", sym(")"))).map(|(_,v,_)| v))
  .or(int)
  .label("atom")

let expr: Parser<i64> =
  precedence(atom, { operand_label: "expression", space: sc })
    .level(|lvl| {                   // postfix
      lvl.postfix(sym("!").map(|_| (|a| fact(a))))
    })
    .level(|lvl| {                   // prefix（右結合）
      lvl.prefix(sym("-").map(|_| (|a| -a)))
      lvl.prefix(sym("+").map(|_| (|a|  a)))
    })
    .level(|lvl| {                   // べき乗は右結合
      lvl.infixr(sym("^").map(|_| (|a,b| pow(a,b))))
    })
    .level(|lvl| {                   // 乗除は左結合
      lvl.infixl(sym("*").map(|_| (|a,b| a*b)))
      lvl.infixl(sym("/").map(|_| (|a,b| a/b)))
    })
    .level(|lvl| {                   // 加減は左結合
      lvl.infixl(sym("+").map(|_| (|a,b| a+b)))
      lvl.infixl(sym("-").map(|_| (|a,b| a-b)))
    })
    .level(|lvl| {                   // 比較は非結合
      let cmp = choice([
        sym("<").map(|_| (|a,b| cmp_lt(a,b))),
        sym("<=").map(|_| (|a,b| cmp_le(a,b))),
        sym(">").map(|_| (|a,b| cmp_gt(a,b))),
        sym(">=").map(|_| (|a,b| cmp_ge(a,b))),
      ])
      lvl.infixn(cmp)
    })
    .build()
```

---

## C. 意味論（消費/コミット・長さ・曖昧性）

### C-1. 消費とコミット（2.1 の規則に合致）

* **二項演算子**：`term op term` を読む。`op` を消費した時点で **`cut_here()` 相当**を自動挿入し、右項が来なければ **committed エラー**（「*演算子 ‘+’ の後に expression が必要*」）。
* **prefix/postfix**：演算子を消費したら **そのオペランド欠如は committed**。
* **nonassoc**：同一レベルで同種類の `infixn` を**連続検出したらエラー**。診断には**両オペレータのスパン**と\*\*挿入候補（括弧）\*\*を含める。

### C-2. 最長一致

* 同一レベルの演算子が **共通接頭辞**を持つ場合（`<` と `<=` など）、**長い方を優先**する。
  実装は `choice_longest` + `attempt` を内部使用。

### C-3. 先読み

* 連続記号の曖昧性（例：`a- -b`）は**演算子パーサ側**で `lookahead`／`notFollowedBy` を使って解消可能。
* キーワード型演算子（`and`/`or`）には `keyword(sc, "and")` を使うと**識別子衝突**を回避。

---

## D. 構築法（内部アルゴリズム：実装規約）

* 各レベルは **Pratt の binding power** へ落とす：

  * `prefix`: 右側の bp を**そのレベル以上**で再帰。
  * `postfix`: 直前の値に**繰り返し**適用（`while` で吸い尽くす）。
  * `infixl`: `foldl`（左畳み）。
  * `infixr`: `foldr`（右畳み）。
  * `infixn`: 2 項のみ許可、連鎖はエラー。
* **パーサの合成**は 2.2 の `attempt/cut` 規則に準拠。
* **Packrat**は `rule()` で付与される `ParserId` をキーに**線形化**。
* **左再帰サポート**（`RunConfig.left_recursion=true`）時、先読み/種成長（seed-growing）で `prefix`/`infix` を拡張可能。ただし通常は**不要**（`precedence` が左再帰を回避）。

---

## E. 拡張：演算子パーサの“型”

```reml
// sugar：文字列を演算子パーサへ持ち上げ
fn op_str<A>(space: Parser<()>, s: Str, f: any) -> Parser<typeof f>
// 例: op_str(sc, "+", (|a,b| Add(a,b)))
```

* すべての `lvl.*` は \*\*「パーサが返すのは“作用関数”」\*\*という統一ルール。

  * `prefix`: `Parser<A -> A>`
  * `postfix`: `Parser<A -> A>`
  * `infix*`: `Parser<(A, A) -> A>`
  * `ternary`: `head/mid` は `Parser<()>`、`build` が `(A,A,A)->A`

---

## F. エラー設計（2.5 と整合）

* **期待集合**：演算子位置では `expected = {"operator '<op>'", "…", operand_label}` を組む。
* **欠落オペランド**：

  ```
  error: expected expression after operator '+'
    --> file.ks:12:17
     12 | x + 
               ^ missing expression (did you mean 'x + ( ... )' ?)
  ```
* **非結合違反**：

  ```
  error: non-associative operators cannot chain
    --> file.ks:5:9
     5 | a < b < c
           ^^^^^^ second '<' here
     help: use parentheses: (a < b) && (b < c)
  ```
* **曖昧/優先順位ミス**には**具体的な括弧挿入提案**を出す（`notes` に追記）。

---

## G. パフォーマンス規約

* **ASCII 高速経路**を `op_str` に内蔵（`string` の高速比較）。
* `space` 指定がある場合、**演算子直後の空白/コメントを一括吸収**し、**operand は `cut` 後に読む**。
* メモ化（Packrat）が **ON** の時は `(ParserId, byte_off)` をキーに\*\*`Reply` を丸ごと保存\*\*。
* 1 レベル内オペレータ群は `choice_longest` で構成、**バックトラックを最小化**。

---

## H. ベストプラクティス

* **“括弧は強化”**：`precedence` の外で `atom` を定義し、`("...", expr, ")")` に **`cut_here()`** を入れる。
* **単項マイナス**は `prefix` で扱う（“符号付き数値”は字句ではなく構文で）。
* **比較/等値**は `infixn` にして連鎖を禁止、上に `&&`/`||` を置く。
* 演算子トークンに `keyword` を使い、識別子との衝突を排除。
* **長い演算子優先**に注意：`<=` と `<` は **同一レベル**に同時登録しても安全（内部で最長一致）。

---

## I. 追加ユースケース（スニペット）

### I-1. 右結合の `?:` 三項

```reml
precedence(cond)
  .level(|lvl| {
    lvl.ternary({
      head: sym("?"),
      mid:  sym(":"),
      build: (c,t,f) -> IfExpr(c, t, f)
    })
  })
  .build()
```

### I-2. パイプ演算子（最弱）

```reml
precedence(atom)
  // ... 他レベル ...
  .level(|lvl| {
    lvl.infixl(keyword(sc, "|>").map(|_| (|x,f| f(x))))
  })
  .build()
```

### I-3. Postfix 呼出/添字/ドットを operand 側に

```reml
let primary =
  atom.andThen(
    many(
      choice([
        sym("(").then(args).then(sym(")")).map(|(_,a,_)| (|recv| Call(recv,a))),
        sym("[").then(expr).then(sym("]")).map(|(_,i,_)| (|recv| Index(recv,i))),
        sym(".").then(ident).map(|(_,id)| (|recv| Field(recv,id))),
      ])
    )
  ).map(|(base, posts)| posts.fold(base, (|acc,f| f(acc))))
```

---

## J. チェックリスト

* [ ] `precedence(operand, config)` → `builder.level{ prefix / postfix / infixl / infixr / infixn / ternary }` → `build()`。
* [ ] 文字列トークン**だけでなく任意のパーサ**を演算子として置ける。
* [ ] **最長一致**・**attempt/cut 自動化**・**右項欠落は committed**。
* [ ] **非結合連鎖エラー**・**括弧提案**。
* [ ] `space` 指定で**一貫したトリビア処理**。
* [ ] Packrat/左再帰（任意）と整合。
* [ ] 代表的ユースケースを短く記述可能（単項/三項/パイプ/比較）。

---

### まとめ

このビルダーは、**最小の宣言**で

* 多文字・予約語含む**多様な演算子**、
* **結合性・優先度**、
* **高品質なエラー（欠落オペランド/非結合違反/括弧提案）**、
  を一度に解決します。内部は Pratt をベースに `attempt/cut` と **最長一致**を仕込み、**線形時間・ゼロコピー**の実用性能を保ちます。
  これで「演算子の地形づくり」は **5–10 行の宣言**で完了します。
