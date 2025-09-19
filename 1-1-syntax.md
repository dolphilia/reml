# 1.1 構文（Syntax）— Kestrel 言語コア仕様

> 目的：**短く書けて、読みやすく、エラーが説明的**になること。
> 前提：**UTF-8 / Unicode 前提**、式指向、静的型（1.2 で詳細）、パーサーコンビネーターを実装しやすい**素直な構文**。

---

## A. 字句（Lexical）

### A.1 文字集合とエンコーディング

* ソースは **UTF-8**。エラー・位置情報はコードポイント／行・列で報告。

### A.2 空白・改行・コメント

* 空白はトークンを分離するために使用。
* 改行は **文末の候補**（B.3 参照）。
* コメント：

  * 行コメント：`// ...`（改行まで）
  * ブロックコメント：`/* ... */`（入れ子可）

### A.3 識別子とキーワード

* 識別子：`XID_Start` + `XID_Continue*`（Unicode 準拠）。
  例）`parse`, `ユーザー`, `_aux1`。
* 予約語（抜粋）：
  `let`, `var`, `fn`, `type`, `match`, `with`, `if`, `then`, `else`, `use`, `pub`, `return`, `true`, `false`, `as`, `where`.
* 演算子トークン（固定）：`|>`, `.` , `,`, `;`, `:`, `=`, `->`, `=>`, `(` `)` `[` `]` `{` `}`,
  `+ - * / % ^`, `== != < <= > >=`, `&& ||`, `!`, `?`, `..`.

### A.4 リテラル

* 整数：`42`, `0b1010`, `0o755`, `0xFF`, 下線区切り可（`1_000`）。
* 浮動小数：`3.14`, `1e-9`, `2_048.0`.
* 文字：`'A'`（Unicode スカラ値、1.4 参照）。
* 文字列：

  * 通常：`"hello\n"`（C系エスケープ）
  * 生：`r"^\d+$"`（バックスラッシュ非解釈）
  * 複数行：`"""line1\nline2"""`（内部改行保持）
* ブール：`true`, `false`
* タプル：`(a, b, c)`／**単位**：`()`
* 配列：`[1, 2, 3]`
* レコード：`{ x: 1, y: 2 }`（順序不問）

---

## B. トップレベルと宣言

### B.1 モジュールとインポート

* ファイル = 1 モジュール。明示名は任意：`module math.number`（将来仕様、現状省略可）。
* 依存の導入：

  ```kestrel
  use Nest.Parse
  use Nest.Parse.{Lex, Op as Operator, Err}
  ```

  `as` で別名、`{ ... }` で限定インポート。

### B.2 可視性

* 既定は **非公開**。`pub` を前置で公開：`pub fn parse(...) = ...`

### B.3 文の終端

* **行末**が文末として解釈される（オフサイドではなく単純な行末）。
* ただし以下では行継続（文末とみなさない）：

  * 行末が **二項演算子／コンマ／ドット／開き括弧/ブラケット** で終わる
  * 次行が **閉じ括弧**で始まる
* `;` は同一行での**明示区切り**として使用可。

### B.4 宣言の種類

* 値束縛：

  ```kestrel
  let x = 42           // 不変
  var y = 0            // 可変（ミュータブル）
  let n: i64 = 10      // 型注釈（任意）
  let (a, b) = pair    // パターン束縛（C.3）
  ```
* 関数：

  * 式本体（単行）：

    ```kestrel
    fn add(a: i64, b: i64) -> i64 = a + b
    ```
  * ブロック本体（複数行）：

    ```kestrel
    fn fact(n: i64) -> i64 {
      if n <= 1 then 1 else n * fact(n - 1)
    }
    ```
* 型／代数的データ型（ADT）：

  ```kestrel
  type Expr =
    | Int(i64)
    | Add(Expr, Expr)
    | Neg(Expr)
  ```
* （将来）trait/impl は 1.2／1.3 節へ。

---

## C. 式・項・パターン

### C.1 式は**式指向**（最後の式が値）

* ブロック `{ ... }` の**最後の式**がそのブロックの値。
* `return expr` は関数内のみ（早期脱出）。省略可能（末尾が戻り値）。

### C.2 関数適用・引数

* 関数呼び出し：`f(x, y)`
* **名前付き引数**：`render(src=doc, width=80)`
* **デフォルト引数**（定義側）：`fn render(src: Doc, width: i32 = 80) = ...`
* 可変長（将来）：`fn log(...args: String) = ...`
* **部分適用**（占位）：`pipe(xs) |> map(_ + 1)`
  `_` は左側パイプ値の**代入位置**（D.3 に詳細）。

### C.3 パターン（束縛・`match` で共通）

* 変数：`x`
* ワイルドカード：`_`
* タプル：`(x, y, _)`
* レコード：`{ x, y: y0 }`（`x: x` は `x` に省略可）
* 代数型：`Some(x)`, `Add(Int(a), b)`
* ガード：`p if cond`

### C.4 制御構文

* `if` 式：

  ```kestrel
  if cond then expr1 else expr2
  ```
* `match` 式（パターンマッチ）：

  ```kestrel
  match expr with
  | Some(x) -> x
  | None    -> 0
  ```

  網羅性は 1.3/2.5 で扱う（警告/エラー方針）。

### C.5 無名関数（ラムダ）

* 単行：`|x, y| x + y`
* 型注釈：`|x: i64| -> i64 { x * 2 }`
* ブロック：`|it| { let y = it + 1; y * y }`

### C.6 ブロックと束縛

```kestrel
{
  let x = 1
  let y = 2
  x + y          // ← ブロックの値
}
```

* 行間区切り、同一行は `;` で区切り可。
* スコープは**静的（レキシカル）**。シャドウイングは許可（ツールで警告可）。

---

## D. 演算子と優先順位

### D.1 組み込み演算子の表

（高い → 低い / `assoc` は結合性）

| 優先 | 形式      | 演算子 / 構文                                | assoc | 例                         |     |     |            |     |
| -: | ------- | --------------------------------------- | :---: | ------------------------- | --- | --- | ---------- | --- |
|  9 | **後置**  | 関数呼び出し `(...)` / 添字 `[...]` / フィールド `.` |   L   | `f(x)`, `arr[i]`, `rec.x` |     |     |            |     |
|  8 | **単項**  | `!`（論理否定）, `-`（算術負）                     |   R   | `-x`, `!ok`               |     |     |            |     |
|  7 | べき乗     | `^`                                     |   R   | `a ^ b`                   |     |     |            |     |
|  6 | 乗除剰     | `*` `/` `%`                             |   L   | `a*b`, `a/b`              |     |     |            |     |
|  5 | 加減      | `+` `-`                                 |   L   | `a+b`, `a-b`              |     |     |            |     |
|  4 | 比較      | `< <= > >=`                             |   N   | `a < b`                   |     |     |            |     |
|  3 | 同値      | `== !=`                                 |   N   | `x == y`                  |     |     |            |     |
|  2 | 論理 AND  | `&&`                                    |   L   | `p && q`                  |     |     |            |     |
|  1 | 論理 OR   | \`                                      |       | \`                        | L   | \`p |            | q\` |
|  0 | **パイプ** | \`                                      |  >\`  | L                         | \`x | > f | > g(a=1)\` |     |

* **関数適用（後置）** は最強優先（演算子より強い）。
* `^` は右結合（`2 ^ 3 ^ 2 == 2 ^ (3 ^ 2)`)。
* 比較/同値は**非結合**（連鎖不可）：`a < b < c` はエラー。
* **パイプ `|>`** は最弱：左から右へ**データフロー**を明示。

### D.2 パイプの規則

* `x |> f` は `f(x)`。
* `x |> g(a=1)` は `g(x, a=1)`（**左値は第1引数**に入る）。
* **占位 `_`** を使うと位置を指定：
  `x |> fold(init=0, f=(_ + 1))` → `fold(x, init=0, f=...)` / `x |> pow(_, 3)` → `pow(x, 3)`
  `x |> between("(", ")", _)` → 第3引数に挿入。
* **ネスト**は左結合で直列化：`a|>f|>g|>h`。

---

## E. データリテラルとアクセス

### E.1 タプル / レコード / 配列

```kestrel
let t  = (1, true, "s")
let p  = { x: 10, y: 20 }
let xs = [1, 2, 3]
```

* アクセス：`t.0`, `p.x`, `xs[2]`
* 末尾カンマ許可：`(a, b,)`, `{x:1, y:2,}`

### E.2 代数的データ型（ADT）

```kestrel
type Option<T> = | Some(T) | None
let v = Some(42)
match v with | Some(n) -> n | None -> 0
```

* コンストラクタ呼び出しは**関数適用と同形**：`Some(x)`。

---

## F. エラーを良くするための構文上の指針

* **ラベル化される構文点**：`match`, `if`, `fn`, `{`/`(`/`[` の開きに対し、パーサが「ここで **何が期待されるか**」を言語側で明確化できるよう、曖昧な省略記法は採用しない。
* **行継続規則**（B.3）により、改行起因の誤解釈を防ぐ。
* **パイプ**と\*\*占位 `_`\*\*はデシュガ可能（2.5 の期待集合にも反映）。

---

## G. 例（仕様の運用感）

```kestrel
use Nest.Parse.{Lex, Op}

// 値と関数
let sep = ", "
fn join3(a: String, b: String, c: String) -> String =
  a + sep + b + sep + c

// ラムダとパイプ
let r = "1 2 3"
  |> split(" ")
  |> map(|s| parseInt(s))
  |> fold(init=0, f=(_ + 1))
  //           ↑ パイプ値の占位

// ADT と match
type Expr = | Int(i64) | Add(Expr, Expr) | Neg(Expr)
fn eval(e: Expr) -> i64 =
  match e with
  | Int(n)     -> n
  | Neg(x)     -> -eval(x)
  | Add(a, b)  -> eval(a) + eval(b)

// ブロックは最後の式が値
fn abs(x: i64) -> i64 {
  if x < 0 then -x else x
}
```

---

## H. 形式的な最小 EBNF（1.1 の範囲）

> 型や意味は 1.2 以降。ここでは**形だけ**。

```
Module      ::= { UseDecl | TopDecl }+
UseDecl     ::= "use" Path ( "{" Ident ("," Ident)* "}" )? ( "as" Ident )? NL

TopDecl     ::= ValDecl | FnDecl | TypeDecl | PubDecl
PubDecl     ::= "pub" TopDecl

ValDecl     ::= ("let" | "var") Pattern ( ":" Type )? "=" Expr NL
FnDecl      ::= "fn" Ident "(" Params? ")" Ret? ( "=" Expr | Block )
Params      ::= Param ( "," Param )*
Param       ::= Ident ( ":" Type )? ( "=" Expr )?
Ret         ::= "->" Type

TypeDecl    ::= "type" Ident "=" SumType NL
SumType     ::= Variant ( "|" Variant )*
Variant     ::= Ident "(" Types? ")"
Types       ::= Type ( "," Type )*

Block       ::= "{" { StmtSep }* (Stmt { StmtSep }+)* Expr? "}"
Stmt        ::= ValDecl | Expr
StmtSep     ::= NL | ";"

Expr        ::= PipeExpr
PipeExpr    ::= OrExpr ( "|>" CallExpr )*
CallExpr    ::= PostfixExpr ( "(" Args? ")" )?
Args        ::= NamedArg ( "," NamedArg )*
NamedArg    ::= (Ident ":")? Expr

PostfixExpr ::= Primary ( "." Ident | "[" Expr "]" )*
Primary     ::= Literal
              | Ident
              | "(" Expr ")"
              | "(" Expr "," Expr ("," Expr)* ","? ")"
              | "{" FieldInits? "}"
              | "[" Expr ("," Expr)* ","? "]"
              | Lambda
FieldInits  ::= FieldInit ( "," FieldInit )* ","?
FieldInit   ::= Ident ":" Expr

Lambda      ::= "|" ParamList? "|" ( "->" Type )? ( Expr | Block )
ParamList   ::= Param ( "," Param )*

Literal     ::= IntLit | FloatLit | StringLit | CharLit | "true" | "false"

Pattern     ::= "_" | Ident | TuplePat | RecordPat | ConstrPat
TuplePat    ::= "(" Pattern ( "," Pattern )* ","? ")"
RecordPat   ::= "{" FieldPat ( "," FieldPat )* ","? "}"
FieldPat    ::= Ident ( ":" Pattern )?
ConstrPat   ::= Ident "(" Pattern ( "," Pattern )* ","? ")"

NL          ::= 行末（B.3 の規則に従う）
```

---

### まとめ

* **行末ベースの簡潔な文法**＋**式指向**＋\*\*強い後置（適用/アクセス）\*\*で、DSL/コンビネータ記述が短く素直に書けます。
* **パイプ `|>` と占位 `_`**がデシュガ可能な**一貫ルール**で、読みやすいデータフローを保証。
* **パターン・ADT・ブロック終端式**で、構文も AST も“自然に”Kestrel→Core→IR へ落ちます。

次は **1.2 型と推論** に進めます。必要なら、この 1.1 を元に**具体的な字句正規表現**や**テスト用ミニ文法**も出します。
