# 代数的効果を使うミニ言語 - Nim 版
# Reml との比較: 例外機構と Result 型による効果のエミュレーション

import sequtils, strformat, options

# ミニ言語の式定義
type
  ExprKind = enum
    ekLit, ekVar, ekAdd, ekMul, ekDiv, ekGet, ekPut, ekFail, ekChoose

  Expr = ref object
    case kind: ExprKind
    of ekLit:
      litValue: int
    of ekVar:
      varName: string
    of ekAdd, ekMul, ekDiv, ekChoose:
      left, right: Expr
    of ekGet:
      discard
    of ekPut, ekFail:
      putExpr: Expr
      failMsg: string

  Env = seq[tuple[name: string, value: int]]

# 式のコンストラクタ
proc lit(n: int): Expr =
  Expr(kind: ekLit, litValue: n)

proc variable(name: string): Expr =
  Expr(kind: ekVar, varName: name)

proc add(left, right: Expr): Expr =
  Expr(kind: ekAdd, left: left, right: right)

proc mul(left, right: Expr): Expr =
  Expr(kind: ekMul, left: left, right: right)

proc divide(left, right: Expr): Expr =
  Expr(kind: ekDiv, left: left, right: right)

proc getState(): Expr =
  Expr(kind: ekGet)

proc putState(e: Expr): Expr =
  Expr(kind: ekPut, putExpr: e)

proc fail(msg: string): Expr =
  Expr(kind: ekFail, failMsg: msg)

proc choose(left, right: Expr): Expr =
  Expr(kind: ekChoose, left: left, right: right)

# 効果の結果型
# State<Int> × Except<String> × Choose をタプルで表現
type
  EffectResult = tuple[success: bool, error: string, results: seq[tuple[value: int, state: int]]]

# 環境から変数を検索
proc lookupEnv(env: Env, name: string): Option[int] =
  for (k, v) in env:
    if k == name:
      return some(v)
  return none(int)

# 式の評価関数（効果を持つ）
#
# Reml の perform に相当する操作を手動で記述：
# - State: state を引数で渡して結果と共に返す
# - Except: EffectResult.success = false で表現
# - Choose: results をリストで収集
proc eval(expr: Expr, env: Env, state: int): EffectResult =
  case expr.kind
  of ekLit:
    (success: true, error: "", results: @[(value: expr.litValue, state: state)])

  of ekVar:
    let maybeValue = lookupEnv(env, expr.varName)
    if maybeValue.isSome:
      (success: true, error: "", results: @[(value: maybeValue.get, state: state)])
    else:
      (success: false, error: &"未定義変数: {expr.varName}", results: @[])

  of ekAdd:
    let leftResult = eval(expr.left, env, state)
    if not leftResult.success:
      return leftResult
    var allResults: seq[tuple[value: int, state: int]] = @[]
    for (lValue, lState) in leftResult.results:
      let rightResult = eval(expr.right, env, lState)
      if not rightResult.success:
        return rightResult
      for (rValue, rState) in rightResult.results:
        allResults.add((value: lValue + rValue, state: rState))
    (success: true, error: "", results: allResults)

  of ekMul:
    let leftResult = eval(expr.left, env, state)
    if not leftResult.success:
      return leftResult
    var allResults: seq[tuple[value: int, state: int]] = @[]
    for (lValue, lState) in leftResult.results:
      let rightResult = eval(expr.right, env, lState)
      if not rightResult.success:
        return rightResult
      for (rValue, rState) in rightResult.results:
        allResults.add((value: lValue * rValue, state: rState))
    (success: true, error: "", results: allResults)

  of ekDiv:
    let leftResult = eval(expr.left, env, state)
    if not leftResult.success:
      return leftResult
    var allResults: seq[tuple[value: int, state: int]] = @[]
    for (lValue, lState) in leftResult.results:
      let rightResult = eval(expr.right, env, lState)
      if not rightResult.success:
        return rightResult
      for (rValue, rState) in rightResult.results:
        if rValue == 0:
          return (success: false, error: "ゼロ除算", results: @[])
        allResults.add((value: lValue div rValue, state: rState))
    (success: true, error: "", results: allResults)

  of ekGet:
    (success: true, error: "", results: @[(value: state, state: state)])

  of ekPut:
    let result = eval(expr.putExpr, env, state)
    if not result.success:
      return result
    var allResults: seq[tuple[value: int, state: int]] = @[]
    for (v, _) in result.results:
      allResults.add((value: v, state: v))
    (success: true, error: "", results: allResults)

  of ekFail:
    (success: false, error: expr.failMsg, results: @[])

  of ekChoose:
    let leftResult = eval(expr.left, env, state)
    if not leftResult.success:
      return leftResult
    let rightResult = eval(expr.right, env, state)
    if not rightResult.success:
      return rightResult
    (success: true, error: "", results: leftResult.results & rightResult.results)

# すべての効果を処理して結果を返す
#
# Reml の handle ... do ... do ... に相当するが、
# Nim では手動で Result を検査して分岐。
proc runWithAllEffects(expr: Expr, env: Env, initState: int): EffectResult =
  eval(expr, env, initState)

# テストケース
proc exampleExpressions(): seq[tuple[name: string, expr: Expr]] =
  @[
    (name: "単純な加算", expr: add(lit(10), lit(20))),
    (name: "乗算と除算", expr: divide(mul(lit(6), lit(7)), lit(2))),
    (name: "状態の取得", expr: add(getState(), lit(5))),
    (name: "状態の更新", expr: putState(add(getState(), lit(1)))),
    (name: "ゼロ除算エラー", expr: divide(lit(10), lit(0))),
    (name: "非決定的選択", expr: choose(lit(1), lit(2))),
    (name: "複雑な例", expr: add(
      choose(lit(10), lit(20)),
      putState(add(getState(), lit(1)))
    ))
  ]

# テスト実行関数
proc runExamples() =
  let examples = exampleExpressions()
  let env: Env = @[]
  let initState = 0

  for (name, expr) in examples:
    echo &"--- {name} ---"
    let result = runWithAllEffects(expr, env, initState)
    if result.success:
      for (value, state) in result.results:
        echo &"  結果: {value}, 状態: {state}"
    else:
      echo &"  エラー: {result.error}"

# Reml との比較メモ:
#
# 1. **効果の表現**
#    Reml: effect State<S> { operation get() -> S; operation put(s: S) -> () }
#    Nim: EffectResult = tuple[success: bool, error: string, results: seq[...]]
#    - Reml は言語レベルで効果を定義
#    - Nim は手動でタプルを管理（ボイラープレートが多い）
#
# 2. **ハンドラーの実装**
#    Reml: handler state_handler<A>(init) for State<S> { ... }
#    Nim: eval 関数内で state を明示的に渡す
#    - Reml はハンドラーが宣言的
#    - Nim は手続き的でエラーハンドリングが煩雑
#
# 3. **非決定性の扱い**
#    Reml: choose_handler で分岐を自動収集
#    Nim: results をリストで手動管理
#    - Reml は分岐が自然に追跡される
#    - Nim は明示的なリスト操作が必要
#
# 4. **型安全性**
#    Reml: 効果が型レベルで強制される
#    Nim: EffectResult の success フィールドで検査（実行時）
#    - Reml の方が型安全
#
# 5. **可読性**
#    Reml: with State<Int>, Except<String>, Choose で効果が明確
#    Nim: if not result.success のチェックが頻出
#    - Reml の方が効果の意図が分かりやすい
#
# **結論**:
# Nim の手動管理アプローチは柔軟だが、ボイラープレートが多い。
# Reml の代数的効果システムはより宣言的で、エラーハンドリングが簡潔。

# テスト実行例
# runExamples()