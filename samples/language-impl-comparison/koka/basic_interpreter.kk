// Koka版Basic言語インタープリタ
// 効果システムを活用した実装

import std/core
import std/core/types
import std/num/float64

// Value型
type value
  VNumber(n: float64)
  VString(s: string)
  VArray(elements: list<value>)

// 演算子
type bin-operator
  OpAdd
  OpSub
  OpMul
  OpDiv
  OpEq
  OpNe
  OpLt
  OpLe
  OpGt
  OpGe
  OpAnd
  OpOr

type unary-operator
  OpNeg
  OpNot

// 式
type expr
  ENumber(n: float64)
  EString(s: string)
  EVariable(name: string)
  EArrayAccess(var-name: string, index: expr)
  EBinOp(op: bin-operator, left: expr, right: expr)
  EUnaryOp(op: unary-operator, operand: expr)

// 文
type statement
  SLet(var-name: string, expr: expr)
  SPrint(exprs: list<expr>)
  SIf(cond: expr, then-block: list<statement>, else-block: list<statement>)
  SFor(var-name: string, start: expr, end: expr, step: expr, body: list<statement>)
  SWhile(cond: expr, body: list<statement>)
  SGoto(line: int)
  SGosub(line: int)
  SReturn
  SDim(var-name: string, size: expr)
  SEnd

// プログラム型
alias program = list<(int, statement)>

// 環境型
alias env = list<(string, value)>

// ランタイム状態
struct runtime-state
  env: env
  call-stack: list<int>
  output: list<string>

// ランタイムエラー
type runtime-error
  UndefinedVariable(name: string)
  UndefinedLabel(line: int)
  TypeMismatch(expected: string, got: string)
  IndexOutOfBounds
  DivisionByZero
  StackUnderflow

// エラー効果
effect runtime
  ctl throw-error(err: runtime-error): a

// 環境の操作
fun env-lookup(env: env, name: string): <runtime> value
  match env.find(fn((k, _)) k == name)
    Just((_, v)) -> v
    Nothing -> throw-error(UndefinedVariable(name))

fun env-insert(env: env, name: string, value: value): env
  Cons((name, value), env.filter(fn((k, _)) k != name))

// 評価関数
fun is-truthy(v: value): bool
  match v
    VNumber(n) -> n != 0.0
    VString(s) -> s != ""
    VArray(a) -> !a.is-nil

fun value-to-string(v: value): string
  match v
    VNumber(n) -> n.show
    VString(s) -> s
    VArray(_) -> "[Array]"

fun find-line(program: program, target: int): <runtime> int
  match program.index-of(fn((line, _)) line == target)
    Just(idx) -> idx
    Nothing -> throw-error(UndefinedLabel(target))

fun eval-expr(e: expr, env: env): <runtime> value
  match e
    ENumber(n) -> VNumber(n)
    EString(s) -> VString(s)
    EVariable(name) -> env-lookup(env, name)
    EArrayAccess(var-name, index-expr) ->
      match env-lookup(env, var-name)
        VArray(arr) ->
          match eval-expr(index-expr, env)
            VNumber(idx) ->
              val i = idx.int
              match arr.drop(i).head
                Just(v) -> v
                Nothing -> throw-error(IndexOutOfBounds)
            _ -> throw-error(TypeMismatch("Number", "Other"))
        _ -> throw-error(TypeMismatch("Array", "Other"))
    EBinOp(op, left, right) ->
      val l = eval-expr(left, env)
      val r = eval-expr(right, env)
      eval-binop(op, l, r)
    EUnaryOp(op, operand) ->
      val v = eval-expr(operand, env)
      eval-unaryop(op, v)

fun eval-binop(op: bin-operator, left: value, right: value): <runtime> value
  match (op, left, right)
    (OpAdd, VNumber(l), VNumber(r)) -> VNumber(l + r)
    (OpSub, VNumber(l), VNumber(r)) -> VNumber(l - r)
    (OpMul, VNumber(l), VNumber(r)) -> VNumber(l * r)
    (OpDiv, VNumber(l), VNumber(r)) ->
      if r == 0.0 then
        throw-error(DivisionByZero)
      else
        VNumber(l / r)
    (OpEq, VNumber(l), VNumber(r)) ->
      VNumber(if l == r then 1.0 else 0.0)
    (OpNe, VNumber(l), VNumber(r)) ->
      VNumber(if l != r then 1.0 else 0.0)
    (OpLt, VNumber(l), VNumber(r)) ->
      VNumber(if l < r then 1.0 else 0.0)
    (OpLe, VNumber(l), VNumber(r)) ->
      VNumber(if l <= r then 1.0 else 0.0)
    (OpGt, VNumber(l), VNumber(r)) ->
      VNumber(if l > r then 1.0 else 0.0)
    (OpGe, VNumber(l), VNumber(r)) ->
      VNumber(if l >= r then 1.0 else 0.0)
    (OpAnd, l, r) ->
      VNumber(if is-truthy(l) && is-truthy(r) then 1.0 else 0.0)
    (OpOr, l, r) ->
      VNumber(if is-truthy(l) || is-truthy(r) then 1.0 else 0.0)
    _ -> throw-error(TypeMismatch("Number", "Other"))

fun eval-unaryop(op: unary-operator, operand: value): <runtime> value
  match (op, operand)
    (OpNeg, VNumber(n)) -> VNumber(0.0 - n)
    (OpNot, v) -> VNumber(if is-truthy(v) then 0.0 else 1.0)
    _ -> throw-error(TypeMismatch("Number", "Other"))

// 文の実行
fun execute-single-statement(stmt: statement, state: runtime-state): <runtime> runtime-state
  match stmt
    SLet(var-name, e) ->
      val v = eval-expr(e, state.env)
      state(env = env-insert(state.env, var-name, v))
    SPrint(exprs) ->
      val values = exprs.map(fn(e) eval-expr(e, state.env))
      val output-line = values.map(value-to-string).join(" ")
      state(output = Cons(output-line, state.output))
    _ -> state

fun execute-block(stmts: list<statement>, state: runtime-state): <runtime> runtime-state
  stmts.foldl(state, fn(s, stmt) execute-single-statement(stmt, s))

fun execute-for-loop(
  var-name: string,
  current: float64,
  end: float64,
  step: float64,
  body: list<statement>,
  program: program,
  pc: int,
  state: runtime-state
): <runtime> list<string>
  if (step > 0.0 && current > end) || (step < 0.0 && current < end) then
    execute-program(program, pc + 1, state)
  else
    val new-state = state(env = env-insert(state.env, var-name, VNumber(current)))
    val block-state = execute-block(body, new-state)
    execute-for-loop(var-name, current + step, end, step, body, program, pc, block-state)

fun execute-while-loop(
  cond: expr,
  body: list<statement>,
  program: program,
  pc: int,
  state: runtime-state
): <runtime> list<string>
  val cond-val = eval-expr(cond, state.env)
  if is-truthy(cond-val) then
    val new-state = execute-block(body, state)
    execute-while-loop(cond, body, program, pc, new-state)
  else
    execute-program(program, pc + 1, state)

fun execute-program(program: program, pc: int, state: runtime-state): <runtime> list<string>
  if pc >= program.length then
    state.output.reverse
  else
    val (_, stmt) = program.drop(pc).head.default((0, SEnd))
    match stmt
      SEnd -> state.output.reverse
      SLet(var-name, e) ->
        val v = eval-expr(e, state.env)
        val new-state = state(env = env-insert(state.env, var-name, v))
        execute-program(program, pc + 1, new-state)
      SPrint(exprs) ->
        val values = exprs.map(fn(e) eval-expr(e, state.env))
        val output-line = values.map(value-to-string).join(" ")
        val new-state = state(output = Cons(output-line, state.output))
        execute-program(program, pc + 1, new-state)
      SIf(cond, then-block, else-block) ->
        val cond-val = eval-expr(cond, state.env)
        val branch = if is-truthy(cond-val) then then-block else else-block
        val new-state = execute-block(branch, state)
        execute-program(program, pc + 1, new-state)
      SFor(var-name, start, end, step, body) ->
        match (eval-expr(start, state.env), eval-expr(end, state.env), eval-expr(step, state.env))
          (VNumber(s), VNumber(e), VNumber(st)) ->
            execute-for-loop(var-name, s, e, st, body, program, pc, state)
          _ -> throw-error(TypeMismatch("Number", "Other"))
      SWhile(cond, body) ->
        execute-while-loop(cond, body, program, pc, state)
      SGoto(target) ->
        val new-pc = find-line(program, target)
        execute-program(program, new-pc, state)
      SGosub(target) ->
        val new-pc = find-line(program, target)
        val new-state = state(call-stack = Cons(pc + 1, state.call-stack))
        execute-program(program, new-pc, new-state)
      SReturn ->
        match state.call-stack
          Cons(return-pc, rest) ->
            val new-state = state(call-stack = rest)
            execute-program(program, return-pc, new-state)
          Nil -> throw-error(StackUnderflow)
      SDim(var-name, size-expr) ->
        match eval-expr(size-expr, state.env)
          VNumber(size) ->
            val arr = list(1, size.int).map(fn(_) VNumber(0.0))
            val new-state = state(env = env-insert(state.env, var-name, VArray(arr)))
            execute-program(program, pc + 1, new-state)
          _ -> throw-error(TypeMismatch("Number", "Other"))

// メイン実行関数
pub fun run(program: program): either<runtime-error, list<string>>
  val initial-state = Runtime-state([], [], [])
  val sorted = program.sort(fn((a, _), (b, _)) a.compare(b))

  try
    Right(execute-program(sorted, 0, initial-state))
  with ctl throw-error(err)
    Left(err)

// テスト実行例
pub fun main(): <console> ()
  val program = [
    (10, SLet("x", ENumber(0.0))),
    (20, SLet("x", EBinOp(OpAdd, EVariable("x"), ENumber(1.0)))),
    (30, SPrint([EVariable("x")])),
    (40, SIf(
      EBinOp(OpLt, EVariable("x"), ENumber(5.0)),
      [SGoto(20)],
      []
    )),
    (50, SEnd)
  ]

  match run(program)
    Right(output) ->
      output.foreach(println)
    Left(err) ->
      match err
        UndefinedVariable(name) -> println("エラー: 未定義変数 " ++ name)
        UndefinedLabel(line) -> println("エラー: 未定義ラベル " ++ line.show)
        TypeMismatch(expected, got) ->
          println("エラー: 型不一致 (期待: " ++ expected ++ ", 実際: " ++ got ++ ")")
        IndexOutOfBounds -> println("エラー: インデックス範囲外")
        DivisionByZero -> println("エラー: 0で割ることはできません")
        StackUnderflow -> println("エラー: スタックアンダーフロー")
