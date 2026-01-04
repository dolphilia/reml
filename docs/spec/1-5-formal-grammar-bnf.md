# 1.5 形式文法（BNF）

> 本節は Reml 言語の構文を形式的に記述するための BNF/EBNF を提供します。
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
CompilationUnit ::= ModuleHeader? { Attrs? (UseDecl | PubDecl) }+

ModuleHeader   ::= "module" ModulePath NL
ModulePath     ::= Ident { "." Ident }

UseDecl        ::= "use" UseTree NL
UseTree        ::= UsePath ["as" Ident]
                 | UsePath "." UseBrace
UsePath        ::= RootPath
                 | RelativePath
RootPath       ::= "::" ModulePath
RelativePath   ::= RelativeHead { "." Ident }
RelativeHead   ::= "self"
                 | SuperPath
                 | Ident
SuperPath      ::= "super" { "." "super" }
UseBrace       ::= "{" UseItem { "," UseItem } [","] "}"
UseItem        ::= Ident ["as" Ident] [ "." UseBrace ]

PubDecl        ::= ["pub"] Decl NL*
Decl            ::= ValDecl
                  | FnDecl
                  | ActivePatternDecl
                  | TypeDecl
                  | TraitDecl
                  | ImplDecl
                  | ExternDecl
                  | EffectDecl
                  | HandlerDecl
                  | ConductorDecl

Attrs           ::= Attribute+
Attribute       ::= "@" Ident [AttrArgs]
AttrArgs        ::= "(" AttrArg { "," AttrArg } [","] ")"
AttrArg         ::= Expr
```

`UseItem` の `.` 拡張は `use Core.Parse.{Lex, Op.{Infix, Prefix}}` のように再帰的に展開される。

### 1.1 共通構成要素

```
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
FnSignature     ::= "fn" Ident [GenericParams] "(" Params? ")" [RetType] [WhereClause] [EffectAnnot]
Params          ::= Param { "," Param }
Param           ::= Pattern [":" Type] ["=" Expr]
RetType         ::= "->" Type
EffectAnnot     ::= "!" "{" EffectTags? "}"
EffectTags      ::= Ident { "," Ident }

ActivePatternDecl ::= "pattern" "(|" Ident ("|_|")? "|)" "(" ParamList? ")" "=" Expr

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

EffectDecl      ::= "effect" Ident ":" Ident EffectBody NL
EffectBody      ::= "{" OperationDecl+ "}"
OperationDecl   ::= Attrs? "operation" Ident ":" Type NL

HandlerDecl     ::= "handler" Ident HandlerBody NL
HandlerBody     ::= "{" HandlerEntry+ "}"
HandlerEntry    ::= "operation" Ident "(" HandlerParams? ")" HandlerBlock
                  | "return" Ident HandlerBlock
HandlerParams   ::= Param { "," Param }
HandlerBlock    ::= Block

ConductorDecl   ::= "conductor" Ident ConductorBody NL*
ConductorBody   ::= "{" NL* ConductorSection* "}"
ConductorSection::= ConductorDslDef
                  | ConductorChannels
                  | ConductorExecution
                  | ConductorMonitoring

ConductorDslDef ::= Ident ":" Ident ["=" PipelineSpec] ConductorDslTail* NL*
ConductorDslTail::= NL* "|>" Ident "(" ConductorArgs? ")"
ConductorArgs   ::= ConductorArg { "," ConductorArg } [","]
ConductorArg    ::= [Ident ":"] Expr
PipelineSpec    ::= Expr

ConductorChannels ::= "channels" ConductorChannelBody NL*
ConductorChannelBody ::= "{" (ChannelRoute NL)* "}"
ChannelRoute    ::= ConductorEndpoint "~>" ConductorEndpoint ":" Type
ConductorEndpoint ::= Ident { "." Ident }

ConductorExecution ::= "execution" Block NL*

ConductorMonitoring ::= "monitoring" ConductorMonitoringSpec? Block NL*
ConductorMonitoringSpec ::= "with" ModulePath
                          | ConductorEndpoint
```

`conductor` ブロックにおけるセクション構成や監査要件は [1.1 構文 B.8](1-1-syntax.md#b8-dsl制御ブロック-conductor) および [guides/conductor-pattern.md](../guides/dsl/conductor-pattern.md) を参照してください。

### 2.1 OpBuilder DSL

```
OpBuilderLevelCall ::= Ident "." "level" "(" IntLiteral "," FixitySymbol "," SymbolList ")"
SymbolList         ::= "[" SymbolToken { "," SymbolToken } [","] "]"
SymbolToken        ::= StringLiteral

FixitySymbol       ::= ":prefix"
                     | ":postfix"
                     | ":infix_left"
                     | ":infix_right"
                     | ":infix_nonassoc"
                     | ":ternary"
```

DSL と `FixitySymbol` の意味論は [2.4 演算子優先度ビルダー](2-4-op-builder.md#b-使い方api-と-dsl) に記載されている。

---

## 3. 文とブロック

```
Block           ::= Attrs? "{" BlockElems? "}"
BlockElems      ::= { Stmt StmtSep }* [Expr]
StmtSep         ::= NL | ";"

Stmt            ::= ValDecl
                  | AssignStmt
                  | DeferStmt
                  | ReturnStmt
                  | Expr

ReturnStmt      ::= "return" Expr NL

LValue          ::= PostfixExpr
```

ブロック式へ付与する属性（`@cfg` など）の扱いは [1.1 構文 B.6](1-1-syntax.md#b6-属性attributes) に準拠します。

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
RelExpr         ::= RangeExpr { ("<" | "<=" | ">" | ">=") RangeExpr }
RangeExpr       ::= AddExpr { ".." AddExpr }
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
                  | ActivePatternApp
                  | "(" Expr ")"
                  | TupleLiteral
                  | RecordLiteral
                  | ArrayLiteral
                  | Lambda
                  | IfExpr
                  | MatchExpr
                  | WhileExpr
                  | ForExpr
                  | LoopExpr
                  | UnsafeBlock
                  | InlineAsmExpr
                  | LlvmIrExpr
                  | Block
                  | PerformExpr
                  | DoExpr
                  | HandleExpr

TupleLiteral    ::= "(" Expr "," Expr { "," Expr } [","] ")"
RecordLiteral   ::= "{" FieldInit { "," FieldInit } [","] "}"
FieldInit       ::= Ident ":" Expr
ArrayLiteral    ::= "[" Expr { "," Expr } [","] "]"

IfExpr          ::= "if" Expr "then" Expr ["else" Expr]
MatchExpr       ::= "match" Expr "with" MatchArm { MatchArm }
MatchArm        ::= "|" Pattern MatchArmTail "->" Expr
MatchArmTail    ::= MatchGuard? MatchAlias? | MatchAlias? MatchGuard?
MatchGuard      ::= ("when" | "if") Expr
MatchAlias      ::= "as" Ident
WhileExpr       ::= "while" Expr Block
ForExpr         ::= "for" Pattern "in" Expr Block
LoopExpr        ::= "loop" Block
UnsafeBlock     ::= Attrs? "unsafe" Block
InlineAsmExpr   ::= "inline_asm" "(" StringLiteral InlineAsmTail? ")"
InlineAsmTail   ::= "," InlineAsmArg { "," InlineAsmArg } [","]
InlineAsmArg    ::= InlineAsmOutputs
                  | InlineAsmInputs
                  | InlineAsmClobbers
                  | InlineAsmOptions
InlineAsmOutputs ::= "outputs" "(" InlineAsmOutputList? ")"
InlineAsmInputs  ::= "inputs" "(" InlineAsmInputList? ")"
InlineAsmOutputList ::= InlineAsmOutput { "," InlineAsmOutput } [","]
InlineAsmInputList  ::= InlineAsmInput { "," InlineAsmInput } [","]
InlineAsmOutput ::= StringLiteral ":" LValue
InlineAsmInput  ::= StringLiteral ":" Expr
InlineAsmClobbers ::= "clobbers" "(" StringLiteral { "," StringLiteral } [","] ")"
InlineAsmOptions  ::= "options" "(" StringLiteral { "," StringLiteral } [","] ")"

LlvmIrExpr      ::= "llvm_ir!" "(" Type ")" LlvmIrBlock
LlvmIrBlock     ::= "{" StringLiteral LlvmIrTail? "}"
LlvmIrTail      ::= "," LlvmIrInputs
LlvmIrInputs    ::= "inputs" "(" LlvmIrInputList? ")"
LlvmIrInputList ::= Expr { "," Expr } [","]

Lambda          ::= "|" ParamList? "|" ["->" Type] LambdaBody
ParamList       ::= Param { "," Param }
LambdaBody      ::= Expr | Block

EffectPath      ::= Ident { "." Ident }
PerformExpr     ::= "perform" EffectPath "(" Args? ")"
DoExpr          ::= "do" EffectPath "(" Args? ")"
HandleExpr      ::= "handle" Expr "with" HandlerLiteral
HandlerLiteral  ::= "handler" Ident HandlerBody
```

効果構文の産出規則は Phase 2-5 時点では `-Zalgebraic-effects` フラグ有効時の PoC 提供に限定される。[^effects-syntax-poc-phase25]
PoC 期間中の残余効果記録 (`Σ_before`/`Σ_after`) と監査メトリクスは `EFFECT-002` Step4 の設計に従い、`extensions.effects.sigma.*` と CI 指標 (`syntax.effect_construct_acceptance`, `effects.syntax_poison_rate`) を Phase 2-7 手順と同期させる。[^effects-sigma-poc-phase25]

`MatchArmTail` はガードとエイリアスの順不同を受理し、AST では `MatchGuard` → `MatchAlias` の順に正規化する。ガードの正規形は `when` で、`if` は互換目的で受理する際に警告を伴う。

`MatchExpr` はスクラティニー（`match` の対象となる `Expr`）を 1 回だけ評価し、その値に対してアームを上から順に照合する。各アームは **パターン照合 → ガード → エイリアス → 本体** の順で評価され、ガードが偽の場合は次のアームへフォールスルーする。部分アクティブパターン（`(|Name|_|)`）の `None` は「照合失敗」として扱い、診断を出さず次アームへ進む。


[^pipe-desugar]: `PipeExpr` は左結合で畳み込まれ、各段が `value |> f(args)` → `f(value, args)` のようにデシュガリングされる。評価順序と短絡は [1.1 構文 C.9](1-1-syntax.md#c9-評価順序と短絡規則) に従い、左から右へ段階的に適用する。

[^effects-sigma-poc-phase25]:
    Phase 2-5 `EFFECT-002 Step4`（2026-04-18 完了）で `extensions.effects.sigma.*`／`audit.metadata["effect.syntax.constructs.*"]` のフォーマットと PoC 指標 (`syntax.effect_construct_acceptance`, `effects.syntax_poison_rate`) を確定し、`docs/notes/effects/effect-system-tracking.md` と `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に移行条件を整理した。

---

## 5. パターン

パターンの結合優先度（高→低）は `BindingPrimary`（Active/Regex/Range/Slice/Constructor などの原子的要素）→ `BindingPattern`（`as`/`@`）→ `OrPattern` の順とし、`OrPattern` は左結合で畳み込む。`BindingPattern` の `as`/`@` は直前の `BindingPrimary` にのみ結合する右結合とする。

```
Pattern         ::= OrPattern

OrPattern       ::= BindingPattern { "|" BindingPattern }
                  // 左結合。`pat1 | pat2 | pat3` は `(pat1 | pat2) | pat3` と等価。

BindingPattern  ::= BindingPrimary
                  | BindingPrimary "as" Ident
                  | Ident "@" BindingPrimary

BindingPrimary  ::= WildcardPattern
                  | NamePattern
                  | TuplePattern
                  | RecordPattern
                  | ConstructorPattern
                  | SlicePattern
                  | RangePattern
                  | RegexPattern
                  | ActivePatternApp

WildcardPattern ::= "_"
NamePattern     ::= Ident
TuplePattern    ::= "(" Pattern { "," Pattern } [","] ")"
RecordPattern   ::= "{" FieldPattern { "," FieldPattern } [","] "}"
FieldPattern    ::= Ident [":" Pattern]
ConstructorPattern ::= Ident "(" Pattern { "," Pattern } [","] ")"
SlicePattern    ::= "[" SlicePatternItem { "," SlicePatternItem } [","] "]"
SlicePatternItem::= Pattern | ".." [Ident]
RangePattern    ::= RangeBound? ".." ["="] RangeBound?
RegexPattern    ::= "r\"" RegexBody "\""
RangeBound      ::= Literal | Ident | ConstructorPattern
ActivePatternApp ::= "(|" Ident ("|_|")? "|)" Pattern?
```

`OrPattern` は左結合で解釈されるため、`pat1 | pat2 | pat3` は `(pat1 | pat2) | pat3` と等価となる。`ActivePatternApp` の `(|Name|_|)` 形式は `Option<T>` を返し、`Some` のときにマッチ成功、`None` のときは次のアームへ進む。`(|Name|)` 形式は常に成功する完全パターンとして扱われる。`RegexBody` の詳細はリテラル正規化規則（1.1 A.4/A.5）に従う。

`BindingPattern` で同一識別子を `as`/`@` に跨って束縛した場合は `pattern.binding.duplicate_name` を報告する。`RegexPattern` は文字列/バイト列を対象とし、それ以外の型に適用した場合は `pattern.regex.unsupported_target` を返す。`SlicePattern` で `..` が 2 回以上現れた場合は `pattern.slice.multiple_rest`、対象がコレクションでない場合は `pattern.slice.type_mismatch` を報告する。`RangePattern` は両端を省略可能で、両端省略時はワイルドカードと等価となる。境界の型不一致は `pattern.range.type_mismatch`、整数リテラルで下限が上限を超える場合は `pattern.range.bound_inverted` を発行する。`..=` は字句上 `..` と `=` に分割される。

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

[^effects-syntax-poc-phase25]:
    Phase 2-5 `SYNTAX-003 S0` の決定により、Formal BNF 上の効果構文は `-Zalgebraic-effects` フラグを通じた PoC 導入に限定される。正式な構文受理は Phase 2-7 で `parser.mly` と型・効果解析を統合した後に提供される予定。計画と差分登録の詳細は `docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-003-proposal.md` および `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の `SYNTAX-003` 項を参照。
