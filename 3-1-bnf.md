# 3.1 BNF 文法仕様

> この章は Reml 言語の構文を形式的に記述するための BNF/EBNF を提供します。
> 字句規則と予約語は [1.1 構文](1-1-syntax.md) の A 節、
> 型システムおよび効果に関する解釈はそれぞれ [1.2 型と推論](1-2-types-Inference.md)、[1.3 効果と安全性](1-3-effects-safety.md) を参照してください。

---

## 0. 記法

- `::=` は定義、`|` は選択 (OR) を表します。
- `{ ... }` は 0 回以上、`{ ... }+` は 1 回以上の繰り返しです。
- `[...]` は任意要素 (0 または 1 回) を示します。
- ターミナルは引用符 (`"token"`) で示し、非終端シンボルは CamelCase で記述します。
- ブロックコメント、空白、行末処理は [1.1 構文](1-1-syntax.md) A.2/A.3 に従って自動的に吸収されます。

---

## 1. トップレベル

```
CompilationUnit ::= { UseDecl | Attrs? PubDecl }+

UseDecl         ::= "use" Path [UseBrace] ["as" Ident] NL
UseBrace        ::= "{" Ident { "," Ident } "}"

PubDecl         ::= ["pub"] Decl NL*
Decl            ::= ValDecl
                  | FnDecl
                  | TypeDecl
                  | TraitDecl
                  | ImplDecl
                  | ExternDecl

Attrs           ::= Attribute+
Attribute       ::= "@" Ident [AttrArgs]
AttrArgs        ::= "(" AttrArg { "," AttrArg } [","] ")"
AttrArg         ::= Expr
```

### 1.1 共通構成要素

```
Path            ::= Ident { "::" Ident }
GenericParams   ::= "<" Ident { "," Ident } ">"
GenericArgs     ::= "<" Type { "," Type } ">"
WhereClause     ::= "where" Constraint { "," Constraint }
Constraint      ::= Ident "<" Type { "," Type } ">"
```

---

## 2. 宣言

```
ValDecl         ::= ("let" | "var") Pattern [":" Type] "=" Expr NL
AssignStmt      ::= LValue ":=" Expr NL
DeferStmt       ::= "defer" Expr NL

FnDecl          ::= FnSignature ("=" Expr | Block)
FnSignature     ::= "fn" Ident [GenericParams] "(" Params? ")" [RetType] [WhereClause]
Params          ::= Param { "," Param }
Param           ::= Pattern [":" Type] ["=" Expr]
RetType         ::= "->" Type

TypeDecl        ::= "type" TypeDeclBody NL
TypeDeclBody    ::= "alias" Ident [GenericParams] "=" Type
                  | Ident [GenericParams] "=" SumType
                  | Ident [GenericParams] "=" "new" Type
SumType         ::= Variant { "|" Variant }
Variant         ::= Ident "(" Types? ")"
Types           ::= Type { "," Type }

TraitDecl       ::= "trait" Ident [GenericParams] [WhereClause] TraitBody
TraitBody       ::= "{" TraitItem* "}"
TraitItem       ::= Attrs? FnSignature (";" | Block)

ImplDecl        ::= "impl" [GenericParams] ImplHead [WhereClause] ImplBody
ImplHead        ::= TraitRef "for" Type | Type
TraitRef        ::= Ident [GenericArgs]
ImplBody        ::= "{" ImplItem* "}"
ImplItem        ::= Attrs? (FnDecl | ValDecl)

ExternDecl      ::= "extern" StringLiteral ExternBody
ExternBody      ::= FnSignature ";" | "{" ExternItem* "}"
ExternItem      ::= Attrs? FnSignature ";"
```

---

## 3. 文とブロック

```
Block           ::= "{" BlockElems? "}"
BlockElems      ::= { Stmt StmtSep }* [Expr]
StmtSep         ::= NL | ";"

Stmt            ::= ValDecl
                  | AssignStmt
                  | DeferStmt
                  | Expr

LValue          ::= PostfixExpr
```

---

## 4. 式

```
Expr            ::= PipeExpr
PipeExpr        ::= OrExpr { "|>" CallExpr }
CallExpr        ::= PostfixExpr [ "(" Args? ")" ]
Args            ::= Arg { "," Arg }
Arg             ::= [Ident ":"] Expr

OrExpr          ::= AndExpr { "||" AndExpr }
AndExpr         ::= EqExpr { "&&" EqExpr }
EqExpr          ::= RelExpr { ("==" | "!=") RelExpr }
RelExpr         ::= AddExpr { ("<" | "<=" | ">" | ">=") AddExpr }
AddExpr         ::= MulExpr { ("+" | "-") MulExpr }
MulExpr         ::= PowExpr { ("*" | "/" | "%") PowExpr }
PowExpr         ::= UnaryExpr { "^" UnaryExpr }
UnaryExpr       ::= PostfixExpr
                  | ("-" | "!") UnaryExpr

PostfixExpr     ::= Primary { PostfixOp }
PostfixOp       ::= "." Ident
                  | "[" Expr "]"
                  | "(" Args? ")"
                  | "?"

Primary         ::= Literal
                  | Ident
                  | "(" Expr ")"
                  | TupleLiteral
                  | RecordLiteral
                  | ArrayLiteral
                  | Lambda
                  | IfExpr
                  | MatchExpr
                  | WhileExpr
                  | ForExpr
                  | UnsafeBlock
                  | Block

TupleLiteral    ::= "(" Expr "," Expr { "," Expr } [","] ")"
RecordLiteral   ::= "{" FieldInit { "," FieldInit } [","] "}"
FieldInit       ::= Ident ":" Expr
ArrayLiteral    ::= "[" Expr { "," Expr } [","] "]"

IfExpr          ::= "if" Expr "then" Expr ["else" Expr]
MatchExpr       ::= "match" Expr "with" MatchArm { MatchArm }
MatchArm        ::= "|" Pattern "->" Expr
WhileExpr       ::= "while" Expr Block
ForExpr         ::= "for" Pattern "in" Expr Block
UnsafeBlock     ::= "unsafe" Block

Lambda          ::= "|" ParamList? "|" ["->" Type] LambdaBody
ParamList       ::= Param { "," Param }
LambdaBody      ::= Expr | Block
```

---

## 5. パターン

```
Pattern         ::= "_"
                  | Ident
                  | TuplePattern
                  | RecordPattern
                  | ConstructorPattern

TuplePattern    ::= "(" Pattern { "," Pattern } [","] ")"
RecordPattern   ::= "{" FieldPattern { "," FieldPattern } [","] "}"
FieldPattern    ::= Ident [":" Pattern]
ConstructorPattern ::= Ident "(" Pattern { "," Pattern } [","] ")"
```

---

## 6. 型式

```
Type            ::= SimpleType
                  | FnType
                  | TupleType
                  | RecordType

SimpleType      ::= Ident [GenericArgs]
FnType          ::= "(" Type { "," Type } ")" "->" Type
TupleType       ::= "(" Type { "," Type } [","] ")"
RecordType      ::= "{" FieldType { "," FieldType } [","] "}"
FieldType       ::= Ident ":" Type
```

---

## 7. リテラル & 字句要素

リテラルおよび字句レベルの正規化規則は [1.1 構文](1-1-syntax.md) A.4/A.5、および [1.4 文字モデル](1-4-test-unicode-model.md) を参照してください。ここではターミナル記号のみを列挙します。

```
Literal        ::= IntLiteral
                 | FloatLiteral
                 | StringLiteral
                 | CharLiteral
                 | "true"
                 | "false"

Ident          ::= *Unicode XID スタート + 続行を満たす識別子*
StringLiteral  ::= *UTF-8 文字列 (通常/生/複数行)*
IntLiteral     ::= *10/16/8/2 進または桁区切り付き整数*
FloatLiteral   ::= *指数/小数表記を含む浮動小数*
CharLiteral    ::= *Unicode スカラ値 1 文字*
NL             ::= *行末 (改行または `;`)*
```

---

## 8. 参考リンク

### 言語コア仕様

* [1.1 構文](1-1-syntax.md) - 字句・意味論の詳細
* [1.2 型と推論](1-2-types-Inference.md) - 型システムの解釈
* [1.3 効果と安全性](1-3-effects-safety.md) - 効果システムの解釈
* [1.4 文字モデル](1-4-test-unicode-model.md) - Unicode処理の詳細

### 標準パーサーAPI仕様

* [2.1 パーサ型](2-1-parser-type.md) - パーサの実装型
* [2.2 コア・コンビネータ](2-2-core-combinator.md) - 基本コンビネータ
* [2.3 字句レイヤ](2-3-lexer.md) - 字句解析の実装
* [2.4 演算子優先度ビルダー](2-4-op-builder.md) - 演算子の実装
* [2.5 エラー設計](2-5-error.md) - エラー処理の詳細
* [2.6 実行戦略](2-6-execution-strategy.md) - 実行時の戦略
