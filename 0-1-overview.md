# 0.1 Reml の概要

Reml (Readable & Expressive Meta Language) はパーサーコンビネーターに最適化された言語です。コンパイラやインタプリタを設計して実行するという難しい工程を最短で実現できることを目指します。Reml のコア仕様と標準APIは、書きやすさ・読みやすさ・エラーの品質が良さに徹底的に追求します。また、実用性能とUnicode前提であることも大切な点です。

---

## 0. 設計ゴール（非機能要件）

**実用性最優先**: 教育用途は結果的に使えれば良い程度の二次的位置付け

1. **実用性能**：末尾最適化、トランポリン、Packrat/左再帰を必要時だけON。FFI・LLVM連携による実用価値の確保。
2. **短く書ける**：演算子優先度や空白処理を"宣言"で終わらせる。
3. **読みやすい**：左→右に流れるパイプ、名前付き引数、推論の強さ。
4. **エラーが良い**：位置・期待集合・cut（コミット）・復旧・トレース。
5. **Unicode前提**：`byte/char/grapheme` の3レイヤを区別。

---

## 0.5 横断テーマと配置

Reml はコア哲学（小さく強いコア・宣言的な操作・高品質な診断）を、以下の横断テーマとして全仕様に貫く。

- **型安全な設定**：`Core.Config`（[2-7](2-7-config.md)）と CLI ガイドで、宣言 DSL → スキーマ検証 → 差分適用 → 実運用(Audit) の安全線を確立する。
- **ツール連携**：`RunConfig.lsp` / 構造化ログ（[2-6](2-6-execution-strategy.md)）と IDE/LSP ガイドで、診断・補完・監査を共通 JSON メタデータに揃える。
- **プラグイン拡張**：`ParserPlugin` / `CapabilitySet`（[2-1](2-1-parser-type.md):I 節）と DSL プラグインガイドで、外部 DSL の登録・互換・署名検証まで一貫して扱う。

これらの柱は `0-2-project-purpose.md` の目的群と同期し、フェーズ更新時も設計意図を再確認できるよう整理されている。

---

## 1. 言語コア仕様（Reml）

### 1.1 構文（抜粋）

* **宣言**

  ```reml
  let x = 42           // 不変（デフォルト）
  var y = 0            // 可変
  fn add(a: i64, b: i64) -> i64 = a + b
  ```

* **型エイリアス / 代数的データ型（ADT）**

  ```reml
  type Result<T,E> = Ok(value: T) | Err(error: E)
  type Expr = Int(i64) | Neg(Expr) | Add(Expr, Expr) | ...
  ```

* **パターンマッチ**

  ```reml
  match v with
  | Ok(x)  -> println(x)
  | Err(e) -> panic(e)
  ```

* **パイプ / 関数合成**

  ```reml
  value |> f |> g(arg=1)    // 左→右に読む
  let h = f >> g            // 合成（h(x) = g(f(x)))
  ```

* **インポート**

  ```reml
  use Core.Parse
  use Core.Parse.{Lex, Op, Err}
  ```

### 1.2 型と推論

* **Hindley-Milner 系推論**（明示注釈は任意、公開APIは型必須推奨）
* **ADT + ジェネリクス + 型クラス相当（Traits）**

  ```reml
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

## 2. 標準パーサAPI（Core.Parse）仕様

Reml の"核"。**小さく強いコア**＋**宣言ビルダー**＋**エラー工具**。

### 2.1 パーサ型

```reml
type Parser<T> = fn(&mut State) -> Reply<T>
type Reply<T> =
  | Ok(value: T, rest: Input, span: Span, consumed: Bool)
  | Err(error: ParseError, consumed: Bool, committed: Bool)
```

* `State` には `Input`（UTF-8 のゼロコピー切片）、`RunConfig`、Packrat メモ表などが格納される。
* `Reply` の `consumed/committed` ビットでバックトラック可否と `cut` 境界を表現する。
* `Span`／`SpanTrace` で成功位置を記録し、IDE 連携やハイライトに利用する。

### 2.2 コア・コンビネータ（厳選）

* 変換系: `map`, `andThen`（flatMap）, `label`, `cut`, `attempt`, `recover`, `trace`
* 直列/選択: `then`, `skipL`, `skipR`, `or`, `choice`
* 繰返し: `many`, `many1`, `opt`, `repeat`, `sepBy`, `sepBy1`, `manyTill`
* 括り: `between(open, p, close)`, `preceded`, `terminated`, `delimited`
* 再帰: `recursive(|self| ...)`
* チェーン: `chainl1(term, op)`, `chainr1(term, op)`
* 前後空白: `padded(p, space)`, `lexeme(space, p)`, `symbol(space, s)`

> **哲学**：**12〜15個**あれば、残りはユーティリティで表現できる。

### 2.3 字句工具（Core.Parse.Lex）

```reml
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

### 2.4 演算子優先度ビルダー（Core.Parse.Op）

```reml
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

### 2.5 エラー設計（Core.Parse.Err）

```reml
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

## 3. 実装アプローチ

Remlは段階的な実装を想定しており、以下の順序で開発されます：

### 3.1 MVP（最小実装）

* **基本型**: `i64`, `Bool`, 単相関数
* **構文**: let/if/fn/app、基本演算子のみのトレイト
* **メモリ**: プリミティブ中心（GC不要）
* **目標**: IR実行器で `main` が走ること

### 3.2 本格実装

* **データ型**: タプル/配列/文字列（RC管理）、クロージャ
* **型システム**: モノモルフィゼーションでジェネリクス
* **トレイト**: ユーザ定義トレイト、where制約、制約解決

### 3.3 完全実装

* **高度な機能**: ADT/`match`/型クラス辞書パッシング
* **エラー処理**: `Result`/`Option`の一級化、`?`演算子
* **最適化**: DWARF デバッグ情報、最適化フラグ連携

詳細な実装例とサンプルコードは、各仕様書で具体的に説明されています。

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

* **フロントエンド**：Reml 自身も `Core.Parse` で自己記述可能（ブートストラップ）。
* **エラーフォーマッタ**：`Err.pretty(src, e)` は**三点リーダ付近強調**・**期待候補上位5件**を提示。
* **最適化**：

  * `lexeme/symbol` は**合成時に内側へ押し込む**（空白食いを重複させない）。
  * `precedence` は**演算子テーブルを固定配列にコンパイル**。
  * Packrat は**ルール単位の部分メモ化**（メモリ上限を超えたらLRUで捨てる）。
* **IDE 連携**：`SpanTrace` により**ノード範囲**・**フォールバック候補**・**自動修正**を提示可能。

---

## 6. まとめ（Remlの"要点"）

* **言語側**：パイプ・型推論・ADT・マッチ・末尾最適化・Unicode。
* **ライブラリ側**：**少数精鋭のコンビネータ**＋**宣言的 precedence**＋**cut/label/recover/trace**。
* **運用**：Packrat/左再帰を**必要時だけ**スイッチ、エラーは**期待集合ベース**で"人間語"。

---

## 関連仕様

### 言語コア仕様

* [1.1 構文](1-1-syntax.md) - 詳細な構文定義
* [1.2 型と推論](1-2-types-Inference.md) - 型システムの完全仕様
* [1.3 効果と安全性](1-3-effects-safety.md) - 効果システムと安全性
* [1.4 文字モデル](1-4-test-unicode-model.md) - Unicode処理の詳細

### 標準パーサーAPI仕様

* [2.1 パーサ型](2-1-parser-type.md) - パーサの型と実行モデル
* [2.2 コア・コンビネータ](2-2-core-combinator.md) - 基本コンビネータ詳細
* [2.3 字句レイヤ](2-3-lexer.md) - 字句解析の実装
* [2.4 演算子優先度ビルダー](2-4-op-builder.md) - 演算子の宣言的実装
* [2.5 エラー設計](2-5-error.md) - エラー処理の完全仕様
* [2.6 実行戦略](2-6-execution-strategy.md) - 実行時の戦略と最適化

### 実装関連

* [3.1 BNF文法仕様](3-1-bnf.md) - 形式的文法定義
* [a-jit.md](a-jit.md) - LLVM連携とコンパイル戦略
* [b-first-idea.md](b-first-idea.md) - 設計の原点となったアイデア
