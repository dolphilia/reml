# 0.1 Kestrel の概要

Kestrel はパーサーコンビネーターに最適化された言語です。コンパイラやインタプリタを設計して実行するという難しい工程を最短で実現できることを目指します。Kestrel のコア仕様と標準APIは、書きやすさ・読みやすさ・エラーの品質が良さに徹底的に追求します。また、実用性能とUnicode前提であることも大切な点です。

---

## 0. 設計ゴール（非機能要件）

**実用性最優先**: 教育用途は結果的に使えれば良い程度の二次的位置付け

1. **実用性能**：末尾最適化、トランポリン、Packrat/左再帰を必要時だけON。FFI・LLVM連携による実用価値の確保。
2. **短く書ける**：演算子優先度や空白処理を"宣言"で終わらせる。
3. **読みやすい**：左→右に流れるパイプ、名前付き引数、推論の強さ。
4. **エラーが良い**：位置・期待集合・cut（コミット）・復旧・トレース。
5. **Unicode前提**：`byte/char/grapheme` の3レイヤを区別。

---

## 1. 言語コア仕様（Kestrel）

### 1.1 構文（抜粋）

* **宣言**

  ```kestrel
  let x = 42           // 不変（デフォルト）
  var y = 0            // 可変
  fn add(a: i64, b: i64) -> i64 = a + b
  ```

* **型エイリアス / 代数的データ型（ADT）**

  ```kestrel
  type Result<T,E> = Ok(value: T) | Err(error: E)
  type Expr = Int(i64) | Neg(Expr) | Add(Expr, Expr) | ...
  ```

* **パターンマッチ**

  ```kestrel
  match v with
  | Ok(x)  -> println(x)
  | Err(e) -> panic(e)
  ```

* **パイプ / 関数合成**

  ```kestrel
  value |> f |> g(arg=1)    // 左→右に読む
  let h = f >> g            // 合成（h(x) = g(f(x)))
  ```

* **インポート**

  ```kestrel
  use Nest.Parse
  use Nest.Parse.{Lex, Op, Err}
  ```

### 1.2 型と推論

* **Hindley-Milner 系推論**（明示注釈は任意、公開APIは型必須推奨）
* **ADT + ジェネリクス + 型クラス相当（Traits）**

  ```kestrel
  trait Show { fn show(self) -> String }
  impl Show for i64 { fn show(self) = self.toString() }
  ```

### 1.3 効果と安全性

* **例外なし**（`panic` はデバッグ用）。失敗は `Result` or パーサの `Error`。
* **末尾再帰最適化**、**トランポリン**（深い `many` でも安全）。
* **所有権/借用**は言語コアで最適化するが、構文は露出しない（**ゼロコスト抽象**志向）。

### 1.4 文字モデル

* `Byte` / `Char`（Unicode スカラ値）/ `Grapheme`（拡張書記素）を区別。
* 文字列は UTF-8。`text.iterGraphemes()` 等を標準装備。

---

## 2. 標準パーサAPI（Nest.Parse）仕様

Kestrel の“核”。**小さく強いコア**＋**宣言ビルダー**＋**エラー工具**。

### 2.1 パーサ型

```kestrel
type Parser<T> = fn(Input) -> ParseResult<T>
type ParseResult<T> =
  | Success(value: T, rest: Input, spans: SpanTrace)  // スパン追跡
  | Failure(error: ParseError)
```

* `Input` は **immutable view**（スライス）。
* `SpanTrace` は成功断片の位置情報（IDE用に保持可能）。

### 2.2 コア・コンビネータ（厳選）

* 変換系: `map`, `andThen (flatMap)`, `label`, `cut`, `recover`, `trace`
* 直列/選択: `then`, `skip`, `or` (= `alt`)
* 繰返し: `many`, `many1`, `opt`, `sepBy`, `sepBy1`
* 括弧: `between(open, close, p)`
* 再帰: `recursive(|self| ...)`
* チェーン: `chainl1(term, op)`, `chainr1(term, op)`
* 前後空白: `p.padded(space)` / `lexeme(space, p)` / `symbol(space, s)`

> **哲学**：**12〜15個**あれば、残りはユーティリティで表現できる。

### 2.3 字句工具（Nest.Parse.Lex）

```kestrel
let sc = Lex.spaceOrTabsOrNewlines
           | Lex.commentLine("//")
           | Lex.commentBlock("/*","*/")
           |> Lex.skipMany

let lexeme(p) = Lex.lexeme(sc, p)
let symbol(s) = Lex.symbol(sc, s)
let ident     = lexeme(Lex.identifier(start=/[A-Za-z_]/, rest=/[A-Za-z0-9_]/))
let intLit    = lexeme(Lex.int(10))
let floatLit  = lexeme(Lex.float())
```

### 2.4 演算子優先度ビルダー（Nest.Parse.Op）

```kestrel
precedence(atom) {
  right "^" using (|a,b| Expr::Pow(a,b))
  left  "*" using (|a,b| Expr::Mul(a,b))
  left  "/" using (|a,b| Expr::Div(a,b))
  left  "+" using (|a,b| Expr::Add(a,b))
  left  "-" using (|a,b| Expr::Sub(a,b))
}
```

* `left/right/nonassoc` を宣言。
* `"token"` だけでなく **パーサ**も置ける（多文字演算子やキーワード対応）。
* ビルダー内部は `chainl/chainr` へ展開。
* **左再帰**が必要な場合は `enableLeftRecursion(true)` を併用可能。

### 2.5 エラー設計（Nest.Parse.Err）

```kestrel
type ParseError = {
  at: Span,                        // 失敗位置
  expected: Set<Expectation>,      // 期待集合（例: token(")"), rule("expr") など）
  context: List<Label>,            // label で積んだ文脈
  notes: List<String>,             // 任意メモ
  committed: Bool,                 // cut 以降なら true
  hint: Option<FixIt>              // 近似候補
}
```

* `cut()`：バックトラック禁止の境界。以降の失敗は**上位へ即時伝播**。
* `label("atom")`：`expected` に**人間語**が出る。
* `recover(p, with: q)`：失敗時、**同期トークン**まで読み捨てて q を差し込む。
* `trace()`：パーサ呼び出しツリーとスパンをログ（IDEフック可能）。

### 2.6 実行戦略

* 既定は **LL(∗)** 相当の前進解析。
* `packrat(on=true)` でメモ化（線形時間）→ メモリとトレードオフ。
* `enableLeftRecursion(true)` で左再帰サポート（Packratと併用推奨）。
* ストリーミング入力：`Input` はリングバッファとインクレメンタル差分を持つ。

---

## 3. “書き心地”を示すサンプル

### 3.1 四則演算 + 単項 − + べき乗（右結合）+ 括弧

```kestrel
use Nest.Parse
use Nest.Parse.{Lex, Op, Err}

type Expr =
  | Int(i64) | Neg(Expr) | Add(Expr, Expr) | Sub(Expr, Expr)
  | Mul(Expr, Expr) | Div(Expr, Expr) | Pow(Expr, Expr) | Paren(Expr)

let sc      = Lex.spaceOrTabsOrNewlines | Lex.commentLine("//") |> Lex.skipMany
let lexeme  = (p) -> Lex.lexeme(sc, p)
let symbol  = (s) -> Lex.symbol(sc, s)
let intLit  = lexeme(Lex.int(10)).map(|n| Expr::Int(n))

let expr : Lazy<Parser<Expr>> = lazy { parseExpr() }

let atom : Parser<Expr> =
  choice(
    symbol("-").cut().then(lazy atom).map(|e| Expr::Neg(e)),
    symbol("(").then(expr).cut().then(symbol(")")).map(Expr::Paren),
    intLit
  ).label("number or '(' or unary '-'")

let parseExpr() =
  precedence(atom) {
    right "^" using (|a,b| Expr::Pow(a,b))
    left  "*" using (|a,b| Expr::Mul(a,b))
    left  "/" using (|a,b| Expr::Div(a,b))
    left  "+" using (|a,b| Expr::Add(a,b))
    left  "-" using (|a,b| Expr::Sub(a,b))
  }
  .between(sc)
  .ensureEof()
  .withErrorHints()
  .packrat(on=true)

fn eval(e: Expr) -> i64 =
  match e with
  | Int(n)   -> n
  | Neg(x)   -> -eval(x)
  | Add(a,b) -> eval(a) + eval(b)
  | Sub(a,b) -> eval(a) - eval(b)
  | Mul(a,b) -> eval(a) * eval(b)
  | Div(a,b) -> eval(a) / eval(b)
  | Pow(a,b) -> eval(a).pow(eval(b))
  | Paren(x) -> eval(x)

pub fn main() {
  for s in ["1+2*3", "2^3^2", "-(2+3)*4", "1+(2* )"] {
    match expr.parse(s) with
    | Ok(ast, _) -> println("{s} => {eval(ast)}")
    | Err(e)     -> println(Err.pretty(s, e))
  }
}
```

**ポイント**

* **優先度宣言 5 行**で終わり。
* `cut()` により `- ( ... )` での曖昧系が**早期に確定**、エラーが素直。
* `withErrorHints()` で \*\*“ここで number か '(' か '-' が欲しかった”\*\*と出る。
* Packrat をスイッチ1つでON。

### 3.2 JSON（抜粋：値・配列・オブジェクト）

```kestrel
type J =
  | JNull | JBool(Bool) | JNum(f64) | JStr(String)
  | JArr(List<J>) | JObj(List<(String, J)>)

let sc = Lex.spaceOrTabsOrNewlines |> Lex.skipMany
let str = Lex.jsonString().lexeme(sc).map(JStr)           // 既製ヘルパ
let num = Lex.number().lexeme(sc).map(|n| JNum(n.toFloat()))

let jvalue : Lazy<Parser<J>> = lazy { value() }

let jnull = Lex.symbol(sc, "null").map(|_| JNull)
let jbool = (Lex.symbol(sc, "true").map(|_| JBool(true))
          | Lex.symbol(sc, "false").map(|_| JBool(false)))

let jarray =
  symbol("[").then(
    jvalue.sepBy(symbol(","))
  ).then(symbol("]"))
   .map(|xs| JArr(xs))
   .label("array")

let pair =
  str.then(symbol(":").cut()).then(jvalue).map(|(JStr(k), v)| (k, v))

let jobject =
  symbol("{").then(
    pair.sepBy(symbol(","))
  ).then(symbol("}"))
   .map(|ps| JObj(ps))
   .label("object")

fn value() = choice(jnull, jbool, num, str, jarray, jobject)
               .between(sc)
               .ensureEof()
```

**ポイント**

* `jsonString()` のような**字句ヘルパ**で現実的な文字列を一発。
* `":"` の直後に `cut()` を入れることで**キー後の失敗を上に伝播**（「キーは読めた、値が欠けてる」を明確化）。
* 期待集合と `label` により、`{ "a": 1, }` のような失敗が**具体的に説明**される。

---

## 4. ミニ言語仕様（BNF抜粋）

```bnf
Module   ::= { UseDecl | TypeDecl | FnDecl | LetDecl }+
UseDecl  ::= "use" Path ("{" Ident ("," Ident)* "}")? EOL
TypeDecl ::= "type" Ident "=" SumType
SumType  ::= Variant ("|" Variant)*
Variant  ::= Ident "(" Fields? ")"
Fields   ::= Type ("," Type)*
FnDecl   ::= "fn" Ident "(" Params? ")" RetType? "=" Expr
LetDecl  ::= ("let" | "var") Ident ("=" Expr)? EOL

Expr     ::= PipeExpr
PipeExpr ::= AppExpr ("|>" AppExpr)*
AppExpr  ::= Term (Args)?
Term     ::= Literal | Ident | "(" Expr ")"
Args     ::= "(" NamedArg? ("," NamedArg)* ")"
NamedArg ::= Ident ":" Expr
Literal  ::= Int | Float | String | Bool

Types    ::= Builtin | Ident "<" Types ">" | "(" Types ")"
Builtin  ::= "i64" | "f64" | "Bool" | "String" | ...
```

* **EOL は改行または `;`**。
* **関数は式**（`=` の右辺は式）。
* **名前付き引数**、**型推論**、**ADT**、**パターンマッチ**が第一級。

---

## 5. 実装ガイド（言語処理系の観点）

* **フロントエンド**：Kestrel 自身も `Nest.Parse` で自己記述可能（ブートストラップ）。
* **エラーフォーマッタ**：`Err.pretty(src, e)` は**三点リーダ付近強調**・**期待候補上位5件**を提示。
* **最適化**：

  * `lexeme/symbol` は**合成時に内側へ押し込む**（空白食いを重複させない）。
  * `precedence` は**演算子テーブルを固定配列にコンパイル**。
  * Packrat は**ルール単位の部分メモ化**（メモリ上限を超えたらLRUで捨てる）。
* **IDE 連携**：`SpanTrace` により**ノード範囲**・**フォールバック候補**・**自動修正**を提示可能。

---

## 6. まとめ（Kestrelの“要点”）

* **言語側**：パイプ・型推論・ADT・マッチ・末尾最適化・Unicode。
* **ライブラリ側**：**少数精鋭のコンビネータ**＋**宣言的 precedence**＋**cut/label/recover/trace**。
* **運用**：Packrat/左再帰を**必要時だけ**スイッチ、エラーは**期待集合ベース**で“人間語”。
