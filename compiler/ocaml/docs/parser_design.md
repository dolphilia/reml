# Parser 設計ノート — Phase 1 OCaml 実装

このドキュメントは Phase 1 (M1マイルストーン) の Parser 実装における構文要素の棚卸しと設計判断を記録する。

## 1. 構文要素の棚卸し

### 1.1 字句要素 (Lexical Elements)

#### 予約語 (Keywords)
仕様書 [1-1-syntax.md](../../../docs/spec/1-1-syntax.md) §A.3 より抽出:

**モジュール/可視性**:
- `module`, `use`, `as`, `pub`, `self`, `super`

**宣言と定義**:
- `let`, `var`, `fn`, `type`, `alias`, `new`, `trait`, `impl`, `extern`
- `effect`, `operation`, `handler`, `conductor`, `channels`, `execution`, `monitoring`

**制御構文**:
- `if`, `then`, `else`, `match`, `with`, `for`, `in`, `while`, `loop`, `return`, `defer`, `unsafe`

**効果操作**:
- `perform`, `do`, `handle`

**型制約**:
- `where`

**真偽リテラル**:
- `true`, `false`

**将来予約**:
- `break`, `continue` (Phase 1 では未実装だがトークン予約)

#### 演算子トークン (固定)
仕様書 §A.3 より:
- パイプ: `|>`, `~>` (チャネル専用)
- 区切り: `.`, `,`, `;`, `:`
- 代入: `=`, `:=`
- 矢印: `->`, `=>`
- 括弧: `(`, `)`, `[`, `]`, `{`, `}`
- 算術: `+`, `-`, `*`, `/`, `%`, `^`
- 比較: `==`, `!=`, `<`, `<=`, `>`, `>=`
- 論理: `&&`, `||`, `!`
- その他: `?`, `..`

#### 識別子 (Identifiers)
- Unicode XID 準拠: `XID_Start + XID_Continue*`
- 例: `parse`, `ユーザー`, `_aux1`

#### リテラル (Literals)
- **整数**: `42`, `0b1010`, `0o755`, `0xFF`, `1_000`
- **浮動小数**: `3.14`, `1e-9`, `2_048.0`
- **文字**: `'A'` (Unicode スカラ値)
- **文字列**:
  - 通常: `"hello\n"` (C系エスケープ)
  - 生: `r"^\d+$"` (バックスラッシュ非解釈)
  - 複数行: `"""line1\nline2"""`
- **ブール**: `true`, `false`
- **タプル**: `(a, b, c)`, 単位 `()`
- **配列**: `[1, 2, 3]`
- **レコード**: `{ x: 1, y: 2 }`

#### コメント
- 行コメント: `// ...` (改行まで)
- ブロックコメント: `/* ... */` (入れ子可)

### 1.2 演算子優先順位 (固定テーブル)

仕様書 §D.1 より、Phase 1 では以下の固定優先順位を実装:

| 優先 | 形式   | 演算子                      | 結合性 |
|-----:|--------|----------------------------|:------:|
|   9  | 後置   | `(...)`, `[...]`, `.`, `?` | 左     |
|   8  | 単項   | `!`, `-`                   | 右     |
|   7  | べき乗 | `^`                        | 右     |
|   6  | 乗除剰 | `*`, `/`, `%`              | 左     |
|   5  | 加減   | `+`, `-`                   | 左     |
|   4  | 比較   | `<`, `<=`, `>`, `>=`       | 非結合 |
|   3  | 同値   | `==`, `!=`                 | 非結合 |
|   2  | 論理AND| `&&`                       | 左     |
|   1  | 論理OR | `||`                       | 左     |
|   0  | パイプ | `|>`                       | 左     |

**Phase 1 の制約**:
- ユーザー定義演算子は Phase 2 で実装
- `precedence` 宣言の解析は行うが、固定テーブルのみ使用
- テーブル定義は将来の拡張を見据えて外部化可能な構造にする

### 1.3 宣言の種類

仕様書 §B.4 より:

1. **値束縛**: `let`, `var` (パターン束縛対応)
2. **関数宣言**: `fn` (名前付き引数、デフォルト引数、戻り値型)
3. **型宣言**: `type` (ADT, alias, newtype)
4. **トレイト定義**: `trait`
5. **実装**: `impl` (トレイト実装 / 型固有メソッド)
6. **外部宣言**: `extern` (FFI)
7. **効果宣言**: `effect`, `handler` (実験段階、`-Zalgebraic-effects` フラグ)
8. **Conductor**: `conductor` (DSL制御ブロック)
9. **モジュール**: `module` (ファイルヘッダ)
10. **インポート**: `use` (単純/中括弧展開/再エクスポート)

### 1.4 式の種類

仕様書 §C より:

- **リテラル**: 数値、文字列、ブール、タプル、配列、レコード
- **変数参照**: 識別子、モジュールパス (`::`/`self`/`super`)
- **関数適用**: `f(x, y)`, 名前付き引数 `f(a=1)`
- **ラムダ**: `|x, y| x + y`, `|x: i64| -> i64 { x * 2 }`
- **パイプ**: `x |> f |> g`, 占位 `_`
- **二項演算**: 算術、比較、論理
- **単項演算**: `!`, `-`
- **フィールドアクセス**: `obj.field`, `tuple.0`
- **添字**: `arr[i]`
- **伝播**: `expr?`
- **条件**: `if cond then expr1 else expr2`
- **パターンマッチ**: `match expr with | pat -> expr`
- **ループ**: `while`, `for`, `loop`
- **ブロック**: `{ ... }`
- **unsafe ブロック**: `unsafe { ... }`
- **return**: `return expr`
- **defer**: `defer expr`
- **代入**: `name := expr` (var 束縛のみ)

### 1.5 パターンの種類

仕様書 §C.3 より:

- **変数**: `x`
- **ワイルドカード**: `_`
- **タプル**: `(x, y, _)`
- **レコード**: `{ x, y: y0 }`, フィールド省略可
- **代数型コンストラクタ**: `Some(x)`, `Add(Int(a), b)`
- **ガード**: `p if cond`
- **残余束縛**: `{ x, .. }`, `let { name, version, .. } = manifest`

### 1.6 属性 (Attributes)

仕様書 §B.6 より:

- 形式: `@name` または `@name(args)`
- 対象: 宣言 (`fn`, `type`, `trait`, `impl`, `extern`) とブロック (`{ ... }`, `unsafe { ... }`)
- 主要属性 (Phase 1 で解析のみ、検証は Phase 2+):
  - `@pure`, `@no_panic`, `@no_alloc` (効果契約)
  - `@inline` (最適化ヒント)
  - `@dsl_export(category, capabilities, version)` (DSL エントリーポイント)
  - `@requires_capability(stage)` (Capability 要求)
  - `@handles(...)` (効果ハンドラ宣言)
  - `@cfg(predicate)` (条件付きコンパイル)

## 2. AST ノード設計

### 2.1 基本型

```ocaml
(* Span: 位置情報 *)
type span = {
  start: int;  (* byte offset *)
  end_: int;   (* byte offset *)
}

(* 識別子 *)
type ident = {
  name: string;
  span: span;
}

(* モジュールパス *)
type module_path =
  | Root of ident list               (* ::Core.Parse *)
  | Relative of relative_head * ident list
and relative_head =
  | Self
  | Super of int                     (* super.super → 2 *)
  | PlainIdent of ident
```

### 2.2 式ノード

```ocaml
type expr = {
  kind: expr_kind;
  span: span;
}
and expr_kind =
  | Literal of literal
  | Var of ident
  | ModulePath of module_path * ident
  | Call of expr * arg list
  | Lambda of param list * type_annot option * expr
  | Pipe of expr * expr
  | Binary of binary_op * expr * expr
  | Unary of unary_op * expr
  | FieldAccess of expr * ident
  | Index of expr * expr
  | Propagate of expr                (* expr? *)
  | If of expr * expr * expr option
  | Match of expr * match_arm list
  | While of expr * expr
  | For of pattern * expr * expr
  | Loop of expr
  | Block of stmt list
  | Unsafe of expr
  | Return of expr option
  | Defer of expr
  | Assign of ident * expr           (* name := expr *)

and literal =
  | Int of string * int_base         (* "42", Base10 *)
  | Float of string
  | Char of string
  | String of string * string_kind
  | Bool of bool
  | Unit
  | Tuple of expr list
  | Array of expr list
  | Record of (ident * expr) list

and int_base = Base2 | Base8 | Base10 | Base16
and string_kind = Normal | Raw | Multiline

and binary_op =
  | Add | Sub | Mul | Div | Mod | Pow
  | Eq | Ne | Lt | Le | Gt | Ge
  | And | Or
  | PipeOp

and unary_op = Not | Neg

and arg =
  | PosArg of expr
  | NamedArg of ident * expr

and match_arm = {
  pattern: pattern;
  guard: expr option;
  body: expr;
  arm_span: span;
}
```

### 2.3 パターンノード

```ocaml
type pattern = {
  kind: pattern_kind;
  span: span;
}
and pattern_kind =
  | PatVar of ident
  | PatWildcard
  | PatTuple of pattern list
  | PatRecord of (ident * pattern option) list * bool  (* bool = has_rest *)
  | PatConstructor of ident * pattern list
  | PatGuard of pattern * expr
```

### 2.4 宣言ノード

```ocaml
type decl = {
  attrs: attribute list;
  vis: visibility;
  kind: decl_kind;
  span: span;
}
and visibility = Public | Private

and attribute = {
  name: ident;
  args: expr list;
  attr_span: span;
}

and decl_kind =
  | LetDecl of pattern * type_annot option * expr
  | VarDecl of pattern * type_annot option * expr
  | FnDecl of fn_decl
  | TypeDecl of type_decl
  | TraitDecl of trait_decl
  | ImplDecl of impl_decl
  | ExternDecl of extern_decl
  | EffectDecl of effect_decl
  | HandlerDecl of handler_decl
  | ConductorDecl of conductor_decl

and fn_decl = {
  name: ident;
  generic_params: ident list;
  params: param list;
  ret_type: type_annot option;
  where_clause: constraint_ list;
  effect_annot: ident list option;
  body: fn_body;
}
and fn_body = FnExpr of expr | FnBlock of stmt list

and param = {
  pat: pattern;
  ty: type_annot option;
  default: expr option;
  param_span: span;
}

and type_decl =
  | AliasDecl of ident * ident list * type_annot
  | SumDecl of ident * ident list * variant list
  | NewtypeDecl of ident * ident list * type_annot

and variant = {
  variant_name: ident;
  variant_types: type_annot list;
  variant_span: span;
}

and trait_decl = {
  trait_name: ident;
  trait_params: ident list;
  trait_where: constraint_ list;
  trait_items: trait_item list;
}
and trait_item = {
  item_attrs: attribute list;
  item_sig: fn_signature;
  item_default: fn_body option;
}

and impl_decl = {
  impl_params: ident list;
  impl_trait: (ident * type_annot list) option;  (* trait<Args> for Type *)
  impl_type: type_annot;
  impl_where: constraint_ list;
  impl_items: impl_item list;
}
and impl_item =
  | ImplFn of fn_decl
  | ImplLet of pattern * type_annot option * expr

and extern_decl = {
  extern_abi: string;
  extern_items: extern_item list;
}
and extern_item = {
  extern_attrs: attribute list;
  extern_sig: fn_signature;
}

and effect_decl = {
  effect_name: ident;
  effect_tag: ident;
  operations: operation_decl list;
}
and operation_decl = {
  op_name: ident;
  op_type: type_annot;
  op_span: span;
}

and handler_decl = {
  handler_name: ident;
  handler_body: expr;
}

and conductor_decl = {
  conductor_name: ident;
  conductor_body: conductor_section list;
}
and conductor_section =
  | DslDef of ident * ident * expr option * ident list  (* name: type = init |> pipe *)
  | Channels of channel_route list
  | Execution of stmt list
  | Monitoring of ident * stmt list

and channel_route = {
  from_endpoint: ident;
  to_endpoint: ident;
  channel_type: type_annot;
  route_span: span;
}

and constraint_ = {
  constraint_trait: ident;
  constraint_types: type_annot list;
  constraint_span: span;
}

and fn_signature = {
  sig_name: ident;
  sig_params: ident list;
  sig_args: param list;
  sig_ret: type_annot option;
  sig_where: constraint_ list;
  sig_effects: ident list option;
}
```

### 2.5 型注釈ノード

```ocaml
type type_annot = {
  ty_kind: type_kind;
  ty_span: span;
}
and type_kind =
  | TyIdent of ident
  | TyApp of ident * type_annot list        (* Vec<T> *)
  | TyTuple of type_annot list
  | TyRecord of (ident * type_annot) list
  | TyFn of type_annot list * type_annot    (* A -> B *)
```

### 2.6 文ノード

```ocaml
type stmt =
  | DeclStmt of decl
  | ExprStmt of expr
  | AssignStmt of ident * expr
  | DeferStmt of expr
```

### 2.7 トップレベル

```ocaml
type use_tree =
  | UsePath of module_path * ident option   (* use ::Core.Parse as P *)
  | UseBrace of module_path * use_item list (* use Core.{Lex, Op as Operator} *)
and use_item = {
  item_name: ident;
  item_alias: ident option;
  item_nested: use_item list option;        (* ネスト展開対応 *)
}

type use_decl = {
  use_pub: bool;
  use_tree: use_tree;
  use_span: span;
}

type module_header = {
  module_path: module_path;
  header_span: span;
}

type compilation_unit = {
  header: module_header option;
  uses: use_decl list;
  decls: decl list;
}
```

## 3. 設計判断と留意事項

### 3.1 Span 情報
- すべての AST ノードに `span` フィールドを付与
- バイトオフセットで記録（行・列番号は診断時に計算）
- エラーメッセージ生成時に [2-5-error.md](../../../docs/spec/2-5-error.md) の `Span` 型へ変換

### 3.2 属性の扱い
- Phase 1 では構文解析のみ実施
- 属性の意味検証 (効果契約など) は Phase 2 で実装
- `@cfg` の評価は字句解析後、構文解析前に実施 (別パス)

### 3.3 効果宣言
- `effect`, `handler`, `conductor` は `-Zalgebraic-effects` フラグ下でのみ有効
- Phase 1 では構文のみ対応、意味解析は Phase 2+

### 3.4 Unicode 対応
- 識別子は Unicode XID 準拠で解析
- 文字列リテラルは UTF-8 として扱う
- Grapheme クラスター対応は Phase 3 以降 (仕様 [1-4-test-unicode-model.md](../../../docs/spec/1-4-test-unicode-model.md))

### 3.5 将来拡張への配慮
- 演算子優先順位テーブルを外部 JSON 形式で定義可能にする準備
- AST に `extensions: Map<string, Any>` フィールドを予約 (Phase 2+)
- ストリーミングパーサ API ([2-7-core-parse-streaming.md](../../../docs/spec/2-7-core-parse-streaming.md)) を見据え、AST 生成を純粋関数化

## 4. Menhir 統合方針

### 4.1 パーサジェネレータ選定理由
- **Menhir**: LR(1) パーサ、エラー回復サポート、OCaml エコシステムで成熟
- 代替案の ocamlyacc は機能が限定的なため不採用

### 4.2 優先順位指定
```ocaml
%left PIPE
%left OR
%left AND
%nonassoc EQ NE
%nonassoc LT LE GT GE
%left PLUS MINUS
%left STAR SLASH PERCENT
%right POW
%right UNARY_MINUS UNARY_NOT
%left DOT LPAREN LBRACKET QUESTION
```

### 4.3 エラー回復戦略
- セミコロン欠落時の自動挿入判定 (文末規則 §B.3 に準拠)
- 括弧不一致の検出と提案
- 予期しないトークンでの同期ポイント設定

## 5. 次ステップ

1. `ast.ml` の実装
2. `lexer.mll` の実装 (Unicode XID 対応)
3. `parser.mly` の実装 (演算子優先順位、エラー回復)
4. Span 情報の正確な付与
5. Golden AST テストの整備

---

**更新履歴**:
- 2025-10-06: 初版作成 (Phase 1 M1 マイルストーン向け構文要素棚卸し)
