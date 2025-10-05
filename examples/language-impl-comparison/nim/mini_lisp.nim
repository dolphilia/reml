# ミニ Lisp 評価機 - Nim 版
# Reml との比較ポイント: マクロシステム、DSL 構築能力、型推論

import std/[tables, strutils, sequtils, options]

# S 式構文を持つ式を解析して評価する
type
  Expr = ref object
    case kind: ExprKind
    of ekNumber: numVal: float
    of ekSymbol: symName: string
    of ekList: items: seq[Expr]

  ExprKind = enum
    ekNumber, ekSymbol, ekList

  # 評価に利用する値表現
  Value = ref object
    case kind: ValueKind
    of vkNumber: number: float
    of vkLambda:
      params: seq[string]
      body: Expr
      env: Env
    of vkBuiltin: builtin: NativeFn

  ValueKind = enum
    vkNumber, vkLambda, vkBuiltin

  NativeFn = proc(args: seq[Value]): Value {.closure.}

  Env = Table[string, Value]

  ParseError = object of CatchableError

# === コンストラクタヘルパー ===

proc newNumber(n: float): Expr =
  Expr(kind: ekNumber, numVal: n)

proc newSymbol(name: string): Expr =
  Expr(kind: ekSymbol, symName: name)

proc newList(items: seq[Expr]): Expr =
  Expr(kind: ekList, items: items)

proc newValueNumber(n: float): Value =
  Value(kind: vkNumber, number: n)

proc newValueBuiltin(fn: NativeFn): Value =
  Value(kind: vkBuiltin, builtin: fn)

# === Nim マクロ: DSL 構文の例 ===
# Reml では演算子優先度やパーサーコンビネーターで記述するところを、
# Nim ではマクロで AST を直接構築できる

# 簡易的な S 式 DSL マクロ（実用段階では構文変換を拡張）
template sexp(body: untyped): Expr =
  body

# === トークナイズと パース ===

proc tokenize(source: string): seq[string] =
  source
    .replace("(", " ( ")
    .replace(")", " ) ")
    .splitWhitespace()
    .filterIt(it.len > 0)

proc parseExpr(tokens: var seq[string]): Expr =
  if tokens.len == 0:
    raise newException(ParseError, "予期しない入力の終端")

  let token = tokens[0]
  tokens.delete(0)

  if token == "(":
    return parseList(tokens)
  elif token == ")":
    raise newException(ParseError, "予期しない閉じ括弧")
  else:
    try:
      return newNumber(parseFloat(token))
    except ValueError:
      return newSymbol(token)

proc parseList(tokens: var seq[string]): Expr =
  var items: seq[Expr] = @[]
  while tokens.len > 0 and tokens[0] != ")":
    items.add(parseExpr(tokens))

  if tokens.len == 0:
    raise newException(ParseError, "括弧が閉じられていません")

  tokens.delete(0)  # remove ")"
  newList(items)

# === 評価 ===

proc evalExpr(expr: Expr, env: Env): Value

proc evalList(items: seq[Expr], env: Env): Value =
  if items.len == 0:
    raise newException(ValueError, "空のリストは評価できません")

  let callee = evalExpr(items[0], env)
  let args = items[1..^1].mapIt(evalExpr(it, env))

  case callee.kind
  of vkBuiltin:
    return callee.builtin(args)
  of vkLambda:
    var newEnv = callee.env
    if callee.params.len != args.len:
      raise newException(ValueError, "引数の数が一致しません")
    for i, param in callee.params:
      newEnv[param] = args[i]
    return evalExpr(callee.body, newEnv)
  of vkNumber:
    raise newException(ValueError, "数値を関数としては適用できません")

proc evalExpr(expr: Expr, env: Env): Value =
  case expr.kind
  of ekNumber:
    return newValueNumber(expr.numVal)
  of ekSymbol:
    if not env.hasKey(expr.symName):
      raise newException(ValueError, "未定義シンボル: " & expr.symName)
    return env[expr.symName]
  of ekList:
    return evalList(expr.items, env)

# === デフォルト環境 ===

proc builtinNumeric(op: proc(a, b: float): float): NativeFn =
  result = proc(args: seq[Value]): Value =
    if args.len != 2:
      raise newException(ValueError, "数値演算は 2 引数のみ対応します")
    if args[0].kind != vkNumber or args[1].kind != vkNumber:
      raise newException(ValueError, "数値以外を演算できません")
    return newValueNumber(op(args[0].number, args[1].number))

proc defaultEnv(): Env =
  result = initTable[string, Value]()
  result["+"] = newValueBuiltin(builtinNumeric(proc(a, b: float): float = a + b))
  result["-"] = newValueBuiltin(builtinNumeric(proc(a, b: float): float = a - b))
  result["*"] = newValueBuiltin(builtinNumeric(proc(a, b: float): float = a * b))
  result["/"] = newValueBuiltin(builtinNumeric(proc(a, b: float): float = a / b))

# === メイン評価関数 ===

proc eval*(source: string): Value =
  var tokens = tokenize(source)
  let expr = parseExpr(tokens)

  if tokens.len > 0:
    raise newException(ParseError, "末尾に未消費トークンがあります")

  let env = defaultEnv()
  evalExpr(expr, env)

# === テスト ===

when isMainModule:
  echo "=== Nim ミニ Lisp 評価機 ==="

  try:
    let result1 = eval("(+ 40 2)")
    echo "Result: ", result1.number  # => 42.0

    let result2 = eval("(* (+ 1 2) (- 5 3))")
    echo "Result: ", result2.number  # => 6.0
  except ParseError as e:
    echo "Parse error: ", e.msg
  except ValueError as e:
    echo "Eval error: ", e.msg

# === Reml との比較メモ ===

# 1. **マクロシステム**
#    Nim: テンプレート、マクロで AST を直接操作可能
#         compile-time execution で強力なメタプログラミング
#    Reml: マクロは標準では提供せず、パーサーコンビネーターで記述
#         DSL は通常の構文拡張として実装
#    - Nim の優位性: メタプログラミングが非常に強力
#    - Reml の優位性: 構文が単純で、学習コストが低い

# 2. **型システム**
#    Nim: 静的型付けだが、object variant（case object）で ADT 風に記述
#         型推論はある程度サポートされるが、Reml ほど強力ではない
#    Reml: Hindley-Milner 型推論で、型注釈をほぼ省略可能
#    - Reml の方が型推論が強力

# 3. **エラーハンドリング**
#    Nim: 例外機構（try-except）が主流、Option 型もサポート
#         Result 型は外部ライブラリで提供
#    Reml: Result<T, E> を標準で提供し、? 演算子で簡潔に記述
#    - Reml の方が関数型スタイルに統一されている

# 4. **性能**
#    Nim: C バックエンドにより、非常に高速
#         コンパイル時実行も高速
#    Reml: 実装次第だが、同等の性能を目指す
#    - Nim は既に成熟した高性能言語

# 5. **DSL 構築**
#    Nim: マクロによる構文拡張が非常に強力
#         UFCS（Uniform Function Call Syntax）で流暢なインターフェース
#    Reml: パーサーコンビネーターと演算子オーバーロードで DSL を構築
#         パイプライン演算子 |> で同様の流暢さを実現
#    - どちらも DSL 構築に適しているが、アプローチが異なる
#    - Nim: コンパイル時 AST 操作
#    - Reml: ランタイムパーサーコンビネーター

# **結論**:
# Nim は強力なマクロシステムとメタプログラミング機能を持ち、
# DSL 構築において非常に柔軟。Reml はマクロなしでも、
# パーサーコンビネーターと型システムで同等の表現力を目指す。