import std/[tables, strutils, options]

# Value型
type
  ValueKind = enum
    vkNumber, vkString, vkArray

  Value = ref object
    case kind: ValueKind
    of vkNumber:
      numVal: float
    of vkString:
      strVal: string
    of vkArray:
      arrayVal: seq[Value]

# 演算子
type
  BinOperator = enum
    opAdd, opSub, opMul, opDiv
    opEq, opNe, opLt, opLe, opGt, opGe
    opAnd, opOr

  UnaryOperator = enum
    opNeg, opNot

# 式
type
  ExprKind = enum
    ekNumber, ekString, ekVariable, ekArrayAccess, ekBinOp, ekUnaryOp

  Expr = ref object
    case kind: ExprKind
    of ekNumber:
      numExpr: float
    of ekString:
      strExpr: string
    of ekVariable:
      varName: string
    of ekArrayAccess:
      arrayVar: string
      arrayIndex: Expr
    of ekBinOp:
      binOp: BinOperator
      binLeft: Expr
      binRight: Expr
    of ekUnaryOp:
      unaryOp: UnaryOperator
      unaryOperand: Expr

# 文
type
  StatementKind = enum
    skLet, skPrint, skIf, skFor, skWhile
    skGoto, skGosub, skReturn, skDim, skEnd

  Statement = ref object
    case kind: StatementKind
    of skLet:
      letVar: string
      letExpr: Expr
    of skPrint:
      printExprs: seq[Expr]
    of skIf:
      ifCond: Expr
      ifThen: seq[Statement]
      ifElse: seq[Statement]
    of skFor:
      forVar: string
      forStart: Expr
      forEnd: Expr
      forStep: Expr
      forBody: seq[Statement]
    of skWhile:
      whileCond: Expr
      whileBody: seq[Statement]
    of skGoto:
      gotoLine: int
    of skGosub:
      gosubLine: int
    of skReturn:
      discard
    of skDim:
      dimVar: string
      dimSize: Expr
    of skEnd:
      discard

# プログラム型
type
  ProgramLine = tuple[line: int, stmt: Statement]
  Program = seq[ProgramLine]

# ランタイム状態
type
  RuntimeState = object
    env: Table[string, Value]
    callStack: seq[int]
    output: seq[string]

# ランタイムエラー
type
  RuntimeErrorKind = enum
    reUndefinedVariable, reUndefinedLabel, reTypeMismatch
    reIndexOutOfBounds, reDivisionByZero, reStackUnderflow

  RuntimeError = object
    kind: RuntimeErrorKind
    message: string

# ヘルパー関数
proc newNumberValue(n: float): Value =
  Value(kind: vkNumber, numVal: n)

proc newStringValue(s: string): Value =
  Value(kind: vkString, strVal: s)

proc newArrayValue(arr: seq[Value]): Value =
  Value(kind: vkArray, arrayVal: arr)

proc newNumberExpr(n: float): Expr =
  Expr(kind: ekNumber, numExpr: n)

proc newStringExpr(s: string): Expr =
  Expr(kind: ekString, strExpr: s)

proc newVariableExpr(name: string): Expr =
  Expr(kind: ekVariable, varName: name)

proc newBinOpExpr(op: BinOperator, left, right: Expr): Expr =
  Expr(kind: ekBinOp, binOp: op, binLeft: left, binRight: right)

proc newError(kind: RuntimeErrorKind, msg: string): RuntimeError =
  RuntimeError(kind: kind, message: msg)

# 評価関数
proc isTruthy(value: Value): bool =
  case value.kind
  of vkNumber:
    value.numVal != 0.0
  of vkString:
    value.strVal.len > 0
  of vkArray:
    value.arrayVal.len > 0

proc valueToString(value: Value): string =
  case value.kind
  of vkNumber:
    $value.numVal
  of vkString:
    value.strVal
  of vkArray:
    "[Array]"

proc findLine(program: Program, target: int): Result[int, RuntimeError] =
  for i, line in program:
    if line.line == target:
      return ok(i)
  err(newError(reUndefinedLabel, "未定義ラベル: " & $target))

proc evalExpr(expr: Expr, env: Table[string, Value]): Result[Value, RuntimeError] =
  case expr.kind
  of ekNumber:
    ok(newNumberValue(expr.numExpr))
  of ekString:
    ok(newStringValue(expr.strExpr))
  of ekVariable:
    if expr.varName in env:
      ok(env[expr.varName])
    else:
      err(newError(reUndefinedVariable, "未定義変数: " & expr.varName))
  of ekArrayAccess:
    if expr.arrayVar notin env:
      return err(newError(reUndefinedVariable, "未定義変数: " & expr.arrayVar))
    let arrayVal = env[expr.arrayVar]
    if arrayVal.kind != vkArray:
      return err(newError(reTypeMismatch, "配列ではありません"))
    let indexRes = evalExpr(expr.arrayIndex, env)
    if indexRes.isErr:
      return err(indexRes.error)
    let indexVal = indexRes.value
    if indexVal.kind != vkNumber:
      return err(newError(reTypeMismatch, "インデックスは数値である必要があります"))
    let idx = int(indexVal.numVal)
    if idx < 0 or idx >= arrayVal.arrayVal.len:
      return err(newError(reIndexOutOfBounds, "インデックス範囲外"))
    ok(arrayVal.arrayVal[idx])
  of ekBinOp:
    let leftRes = evalExpr(expr.binLeft, env)
    if leftRes.isErr:
      return err(leftRes.error)
    let rightRes = evalExpr(expr.binRight, env)
    if rightRes.isErr:
      return err(rightRes.error)
    let left = leftRes.value
    let right = rightRes.value

    case expr.binOp
    of opAdd:
      if left.kind == vkNumber and right.kind == vkNumber:
        ok(newNumberValue(left.numVal + right.numVal))
      else:
        err(newError(reTypeMismatch, "型不一致"))
    of opSub:
      if left.kind == vkNumber and right.kind == vkNumber:
        ok(newNumberValue(left.numVal - right.numVal))
      else:
        err(newError(reTypeMismatch, "型不一致"))
    of opMul:
      if left.kind == vkNumber and right.kind == vkNumber:
        ok(newNumberValue(left.numVal * right.numVal))
      else:
        err(newError(reTypeMismatch, "型不一致"))
    of opDiv:
      if left.kind == vkNumber and right.kind == vkNumber:
        if right.numVal == 0.0:
          err(newError(reDivisionByZero, "0で割ることはできません"))
        else:
          ok(newNumberValue(left.numVal / right.numVal))
      else:
        err(newError(reTypeMismatch, "型不一致"))
    of opEq:
      if left.kind == vkNumber and right.kind == vkNumber:
        ok(newNumberValue(if left.numVal == right.numVal: 1.0 else: 0.0))
      else:
        err(newError(reTypeMismatch, "型不一致"))
    of opNe:
      if left.kind == vkNumber and right.kind == vkNumber:
        ok(newNumberValue(if left.numVal != right.numVal: 1.0 else: 0.0))
      else:
        err(newError(reTypeMismatch, "型不一致"))
    of opLt:
      if left.kind == vkNumber and right.kind == vkNumber:
        ok(newNumberValue(if left.numVal < right.numVal: 1.0 else: 0.0))
      else:
        err(newError(reTypeMismatch, "型不一致"))
    of opLe:
      if left.kind == vkNumber and right.kind == vkNumber:
        ok(newNumberValue(if left.numVal <= right.numVal: 1.0 else: 0.0))
      else:
        err(newError(reTypeMismatch, "型不一致"))
    of opGt:
      if left.kind == vkNumber and right.kind == vkNumber:
        ok(newNumberValue(if left.numVal > right.numVal: 1.0 else: 0.0))
      else:
        err(newError(reTypeMismatch, "型不一致"))
    of opGe:
      if left.kind == vkNumber and right.kind == vkNumber:
        ok(newNumberValue(if left.numVal >= right.numVal: 1.0 else: 0.0))
      else:
        err(newError(reTypeMismatch, "型不一致"))
    of opAnd:
      ok(newNumberValue(if isTruthy(left) and isTruthy(right): 1.0 else: 0.0))
    of opOr:
      ok(newNumberValue(if isTruthy(left) or isTruthy(right): 1.0 else: 0.0))
  of ekUnaryOp:
    let operandRes = evalExpr(expr.unaryOperand, env)
    if operandRes.isErr:
      return err(operandRes.error)
    let operand = operandRes.value

    case expr.unaryOp
    of opNeg:
      if operand.kind == vkNumber:
        ok(newNumberValue(-operand.numVal))
      else:
        err(newError(reTypeMismatch, "型不一致"))
    of opNot:
      ok(newNumberValue(if isTruthy(operand): 0.0 else: 1.0))

proc executeSingleStatement(stmt: Statement, state: var RuntimeState): Result[void, RuntimeError] =
  case stmt.kind
  of skLet:
    let valRes = evalExpr(stmt.letExpr, state.env)
    if valRes.isErr:
      return err(valRes.error)
    state.env[stmt.letVar] = valRes.value
    ok()
  of skPrint:
    var parts: seq[string] = @[]
    for expr in stmt.printExprs:
      let valRes = evalExpr(expr, state.env)
      if valRes.isErr:
        return err(valRes.error)
      parts.add(valueToString(valRes.value))
    state.output.add(parts.join(" "))
    ok()
  else:
    ok()

proc executeBlock(stmts: seq[Statement], state: var RuntimeState): Result[void, RuntimeError] =
  for stmt in stmts:
    let res = executeSingleStatement(stmt, state)
    if res.isErr:
      return err(res.error)
  ok()

proc executeForLoop(varName: string, current, endVal, step: float,
                    body: seq[Statement], program: Program, pc: int,
                    state: RuntimeState): Result[seq[string], RuntimeError]

proc executeWhileLoop(cond: Expr, body: seq[Statement], program: Program,
                      pc: int, state: RuntimeState): Result[seq[string], RuntimeError]

proc executeProgram(program: Program, pc: int, state: RuntimeState): Result[seq[string], RuntimeError] =
  if pc >= program.len:
    return ok(state.output)

  var newState = state
  let stmt = program[pc].stmt

  case stmt.kind
  of skEnd:
    ok(state.output)
  of skLet:
    let valRes = evalExpr(stmt.letExpr, state.env)
    if valRes.isErr:
      return err(valRes.error)
    newState.env[stmt.letVar] = valRes.value
    executeProgram(program, pc + 1, newState)
  of skPrint:
    var parts: seq[string] = @[]
    for expr in stmt.printExprs:
      let valRes = evalExpr(expr, state.env)
      if valRes.isErr:
        return err(valRes.error)
      parts.add(valueToString(valRes.value))
    newState.output.add(parts.join(" "))
    executeProgram(program, pc + 1, newState)
  of skIf:
    let condRes = evalExpr(stmt.ifCond, state.env)
    if condRes.isErr:
      return err(condRes.error)
    let branch = if isTruthy(condRes.value): stmt.ifThen else: stmt.ifElse
    let blockRes = executeBlock(branch, newState)
    if blockRes.isErr:
      return err(blockRes.error)
    executeProgram(program, pc + 1, newState)
  of skFor:
    let startRes = evalExpr(stmt.forStart, state.env)
    if startRes.isErr:
      return err(startRes.error)
    let endRes = evalExpr(stmt.forEnd, state.env)
    if endRes.isErr:
      return err(endRes.error)
    let stepRes = evalExpr(stmt.forStep, state.env)
    if stepRes.isErr:
      return err(stepRes.error)
    if startRes.value.kind != vkNumber or endRes.value.kind != vkNumber or stepRes.value.kind != vkNumber:
      return err(newError(reTypeMismatch, "FOR ループには数値が必要です"))
    executeForLoop(stmt.forVar, startRes.value.numVal, endRes.value.numVal,
                   stepRes.value.numVal, stmt.forBody, program, pc, state)
  of skWhile:
    executeWhileLoop(stmt.whileCond, stmt.whileBody, program, pc, state)
  of skGoto:
    let newPcRes = findLine(program, stmt.gotoLine)
    if newPcRes.isErr:
      return err(newPcRes.error)
    executeProgram(program, newPcRes.value, state)
  of skGosub:
    let newPcRes = findLine(program, stmt.gosubLine)
    if newPcRes.isErr:
      return err(newPcRes.error)
    newState.callStack.add(pc + 1)
    executeProgram(program, newPcRes.value, newState)
  of skReturn:
    if state.callStack.len == 0:
      return err(newError(reStackUnderflow, "スタックアンダーフロー"))
    let returnPc = state.callStack[^1]
    newState.callStack.setLen(state.callStack.len - 1)
    executeProgram(program, returnPc, newState)
  of skDim:
    let sizeRes = evalExpr(stmt.dimSize, state.env)
    if sizeRes.isErr:
      return err(sizeRes.error)
    if sizeRes.value.kind != vkNumber:
      return err(newError(reTypeMismatch, "型不一致"))
    let size = int(sizeRes.value.numVal)
    var arr: seq[Value] = @[]
    for i in 0..<size:
      arr.add(newNumberValue(0.0))
    newState.env[stmt.dimVar] = newArrayValue(arr)
    executeProgram(program, pc + 1, newState)

proc executeForLoop(varName: string, current, endVal, step: float,
                    body: seq[Statement], program: Program, pc: int,
                    state: RuntimeState): Result[seq[string], RuntimeError] =
  if (step > 0.0 and current > endVal) or (step < 0.0 and current < endVal):
    return executeProgram(program, pc + 1, state)

  var newState = state
  newState.env[varName] = newNumberValue(current)
  let blockRes = executeBlock(body, newState)
  if blockRes.isErr:
    return err(blockRes.error)
  executeForLoop(varName, current + step, endVal, step, body, program, pc, newState)

proc executeWhileLoop(cond: Expr, body: seq[Statement], program: Program,
                      pc: int, state: RuntimeState): Result[seq[string], RuntimeError] =
  let condRes = evalExpr(cond, state.env)
  if condRes.isErr:
    return err(condRes.error)

  if isTruthy(condRes.value):
    var newState = state
    let blockRes = executeBlock(body, newState)
    if blockRes.isErr:
      return err(blockRes.error)
    executeWhileLoop(cond, body, program, pc, newState)
  else:
    executeProgram(program, pc + 1, state)

proc run*(program: Program): Result[seq[string], RuntimeError] =
  var state = RuntimeState(
    env: initTable[string, Value](),
    callStack: @[],
    output: @[]
  )
  var sorted = program
  sorted.sort(proc (a, b: ProgramLine): int = cmp(a.line, b.line))
  executeProgram(sorted, 0, state)

# テスト実行例
when isMainModule:
  let program: Program = @[
    (10, Statement(kind: skLet, letVar: "x", letExpr: newNumberExpr(0.0))),
    (20, Statement(kind: skLet, letVar: "x",
                   letExpr: newBinOpExpr(opAdd, newVariableExpr("x"), newNumberExpr(1.0)))),
    (30, Statement(kind: skPrint, printExprs: @[newVariableExpr("x")])),
    (40, Statement(kind: skIf,
                   ifCond: newBinOpExpr(opLt, newVariableExpr("x"), newNumberExpr(5.0)),
                   ifThen: @[Statement(kind: skGoto, gotoLine: 20)],
                   ifElse: @[])),
    (50, Statement(kind: skEnd))
  ]

  let result = run(program)
  if result.isOk:
    for line in result.value:
      echo line
  else:
    echo "エラー: ", result.error.message
