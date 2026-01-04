# パーサーコンビネーターのための言語・ライブラリ

パーサーコンビネーターが書きやすい言語・ライブラリを決める要素を洗い出し、その上で**理想の（架空の）言語＋ライブラリ**を設計する。

> この文書はremlを作り始める最初のアイデアとなったものです

---

# 1) パーサーコンビネーターに向いた“言語”の特徴

1. **関数が一級市民 & 高階関数が軽い**

* パーサー = `Input -> Result<Value, Error> × Rest` な**関数**。
* 関数合成（`map/flatMap/alt/many` 等）を大量に使うため、**ラムダが軽く**クロージャのキャプチャが素直だと書きやすい。

2. **型推論 + 代数的データ型(ADT) + パターンマッチ**

* `Result`, `Option`, `Either`, `NonEmptyList<Expected>` などが自然に表現できる。
* エラーを「どこで／何を期待していたか」の**期待集合**として持てる。
* 返り値やASTを**型で設計 → マッチで分解**でき、読みやすい。

3. **演算子定義/中置記法 or パイプ演算子**

* `term.chainl(op)` / `p1 | p2` / `p.map(f).then(q)` を**短い記法**で書けると直感的。
* `|>`（パイプ）で**左から右へレシピ**のように書けると理解しやすい。

4. **トランポリン/継続 or 末尾再帰最適化でスタック安全**

* `many` や `rep` は深い再帰になりがち。**スタック枯渇を避ける**ランタイム/コンパイラ特性があると実践的。

5. **効果を分離できる（例：Try/Cut/Trace/Recover）**

* バックトラック、コミット（cut）、デバッグトレース、回復（error recovery）を**型や注釈で分離**できると安全。

6. **UTF-8/Unicodeに素直**

* 「文字」= バイト列ではない。**グラフェム**や**コードポイント**単位で扱えるプリミティブがあると現実的。

---

# 2) 「理解しやすく書きやすい」ライブラリの特徴

* **極小コア**：`map / ap / then / or / many / opt / between / sepBy / chainl / chainr / recursive` の**12-15個**に集約。
* **字句解析ヘルパ**：`lexeme`, `symbol`, `space`, `comment`。**空白・コメントを自動で食う**。
* **演算子優先度ビルダー**：`precedence { left "+", "-"; left "*", "/"; right "^" }` で**宣言して終わり**。
* **エラー設計**：

  * 位置（行・列・オフセット・コンテキスト）
  * 期待集合（例：`Expected<"identifier"|"(" | integer>`）
  * **cut** の後は上位に伝播（「ここで引き返さない」を宣言）
  * 提案/修正候補（近似トークン）
* **左再帰サポート or PEG/Packrat 切替**：

  * 既定は LL(∗) で `chainl/chainr`。
  * **必要に応じ Packrat** と **左再帰解析**を有効化可能（性能とメモリを明示選択）。
* **ストリーミング/インクリメンタル**：REPL/IDE用途で**差分再パース**が可能。
* **トレース/可視化**：1 行で**失敗経路の可視化**が出せる。

---

# 3) 最適な“架空の”言語とライブラリの提案

## 言語: **Reml**

* **ML 系の型推論 + ADT + パターンマッチ + パイプ `|>`**
* **ゼロコスト抽象**（インライン展開/逃げないラムダ）
* **末尾再帰最適化 & トランポリン**
* 文字列は **UTF-8**、`byte`, `char`, `grapheme` の3階層

## ライブラリ: **Core.Parse**

* コア 12-15個のコンビネータ + 字句/演算子/エラー拡張
* `precedence { ... }`、`cut`, `label`, `recover`, `trace`
* **左再帰ON/OFF**、**Packrat ON/OFF** を関数 1 つで切替

---

# 4) その言語で“どう書けるか” — サンプルコード

> 目標：**四則演算 + 単項マイナス + べき乗（右結合） + 括弧**、
> 字句（空白/コメント）処理、期待集合つきエラー、演算子宣言だけで優先度と結合性を決める。

```reml
// ── 依存 ──────────────────────────────────────────────
use Core.Parse // コア
use Core.Parse.Lex // lexeme/symbol/space 等
use Core.Parse.Op  // precedence ビルダー
use Core.Parse.Err // エラー型/整形

// ── AST ───────────────────────────────────────────────
type Expr =
  | Int(value: i64)
  | Neg(expr: Expr)
  | Add(lhs: Expr, rhs: Expr)
  | Sub(lhs: Expr, rhs: Expr)
  | Mul(lhs: Expr, rhs: Expr)
  | Div(lhs: Expr, rhs: Expr)
  | Pow(lhs: Expr, rhs: Expr) // 右結合
  | Paren(inner: Expr)

// ── 字句: 空白/コメントを共通吸収 ─────────────────────
let space  = Lex.spaceOrTabsOrNewlines
let lineC  = Lex.commentLine("//")
let blockC = Lex.commentBlock("/*","*/")
let sc     = Lex.skipMany(space | lineC | blockC)

let lexeme p = Lex.lexeme(sc, p)
let symbol s = Lex.symbol(sc, s)

// ── プリミティブ ──────────────────────────────────────
let intLit : Parser<Expr> =
  // 先頭ゼロ許容、負号は単項で処理する
  lexeme(Regex("[0-9]+")).map(|s| Expr::Int(s.toI64()))

let lparen = symbol("(")
let rparen = symbol(")")

// 遅延参照 (recursive)
let expr : Lazy<Parser<Expr>> = lazy { parseExpr() }

// 最小単位 (atom): 整数 or (expr) or 単項マイナス
let atom : Parser<Expr> =
  choice( // choice = 複数のorの糖衣
    // 単項マイナスは cut でコミットし、途中でやり直さない
    symbol("-").cut().then(lazy atom).map(|e| Expr::Neg(e)),
    lparen.then(expr).cut().then(rparen).map(|e| Expr::Paren(e)),
    intLit
  ).label("atom")

// ── 演算子優先度: 右結合の ^、左結合の */、+- ─────────────
let parseExpr () : Parser<Expr> =
  precedence(atom) {
    right "^" using (|a, b| Expr::Pow(a,b))
    left  "*" using (|a, b| Expr::Mul(a,b))
    left  "/" using (|a, b| Expr::Div(a,b))
    left  "+" using (|a, b| Expr::Add(a,b))
    left  "-" using (|a, b| Expr::Sub(a,b))
  }
  .between(sc) // 先頭/末尾のスキップ
  .ensureEof() // 入力を使い切る
  .withErrorHints() // 期待集合を整形
  .enableLeftRecursion(false) // 左再帰はガード扱いのため通常はOFF
  .packrat(on = true)         // メモ化で線形時間に

// ── 便利: 評価（おまけ） ──────────────────────────────
let rec eval (e: Expr) : i64 =
  match e with
  | Int(n)       -> n
  | Neg(x)       -> -eval(x)
  | Add(a,b)     -> eval(a) + eval(b)
  | Sub(a,b)     -> eval(a) - eval(b)
  | Mul(a,b)     -> eval(a) * eval(b)
  | Div(a,b)     -> eval(a) / eval(b)
  | Pow(a,b)     -> eval(a).pow(eval(b))
  | Paren(x)     -> eval(x)

// ── エントリ ──────────────────────────────────────────
pub fn main(args: List<String>) {
  let samples = [
    "1+2*3",
    "(1+2)*3-4/2",
    "-(2+3)^2*4 + 10/5",
    "2 ^ 3 ^ 2",     // 右結合 → 2^(3^2) = 512
    "1 + (2 * )"     // エラー例
  ]

  for s in samples {
    match expr.parse(s) with
    | Ok(ast, _) ->
        println("{s}  =>  AST={ast}  VAL={eval(ast)}")
    | Err(e) ->
        // 期待集合と位置を見やすく出力
        println("Parse error:\n" + Err.pretty(s, e))
  }
}
```

### ここでの“書き心地”のポイント

* **優先度の宣言が 5 行**で済む（`precedence{...}`）。
* **`lexeme/symbol` と `sc`**で空白・コメントを**意識から消せる**。
* \*\*`cut()`\*\*で「この分岐に入ったら戻らない」を明示 → エラーメッセージが素直。
* **`label("atom")` / `withErrorHints()`**で**期待集合**が「`数字` か `(` か `-` が欲しかった」と出る。
* \*\*`packrat(true)`\*\*は重いときだけON。小さなDSLならOFFにして軽量化。
* **`enableLeftRecursion(false)`**：通常は `chainl/chainr` や `precedence` で回避する。**true** は混入時の安全弁として使う。

---

# 5) まとめ（要件 → 言語/ライブラリ設計の対応）

| 欲しい体験       | 言語側の支援      | ライブラリ側の支援                     |
| ----------- | ----------- | ----------------------------- |
| 少ない記述で文法を合成 | パイプ/中置・型推論  | 極小コアAPI、`precedence`          |
| 読めるエラー      | ADT/パターンマッチ | 期待集合・位置情報・`cut/label/recover` |
| 実用性能        | 末尾最適化/軽量ラムダ | Packrat/左再帰ガード切替・部分メモ化     |
| 国際化         | UTF-8/文字階層  | `char/grapheme` パーサ           |
| IDE/REPL    | 効果分離        | トレース/インクリメンタル                 |

> **Reml × Core.Parse** は、FParsec の直感性、Megaparsec のエラー品質、FastParse の簡潔さ、PEG/Packrat の安定性を"必要な場面でだけ"取り込む構成です。
