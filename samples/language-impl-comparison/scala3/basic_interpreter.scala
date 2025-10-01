import scala.collection.mutable

// Value型
enum Value:
  case Number(value: Double)
  case Str(value: String)
  case Array(elements: Vector[Value])

// 演算子
enum BinOperator:
  case Add, Sub, Mul, Div
  case Eq, Ne, Lt, Le, Gt, Ge
  case And, Or

enum UnaryOperator:
  case Neg, Not

// 式
enum Expr:
  case Number(value: Double)
  case Str(value: String)
  case Variable(name: String)
  case ArrayAccess(varName: String, index: Expr)
  case BinOp(op: BinOperator, left: Expr, right: Expr)
  case UnaryOp(op: UnaryOperator, operand: Expr)

// 文
enum Statement:
  case Let(varName: String, expr: Expr)
  case Print(exprs: List[Expr])
  case If(cond: Expr, thenBlock: List[Statement], elseBlock: List[Statement])
  case For(varName: String, start: Expr, end: Expr, step: Expr, body: List[Statement])
  case While(cond: Expr, body: List[Statement])
  case Goto(line: Int)
  case Gosub(line: Int)
  case Return
  case Dim(varName: String, size: Expr)
  case End

// プログラム型
type Program = List[(Int, Statement)]

// ランタイム状態
case class RuntimeState(
  env: mutable.Map[String, Value],
  callStack: List[Int],
  output: List[String]
)

// ランタイムエラー
enum RuntimeError(val message: String) extends Exception(message):
  case UndefinedVariable(name: String) extends RuntimeError(s"未定義変数: $name")
  case UndefinedLabel(line: Int) extends RuntimeError(s"未定義ラベル: $line")
  case TypeMismatch(expected: String, got: String) extends RuntimeError(s"型不一致: 期待 $expected, 実際 $got")
  case IndexOutOfBounds extends RuntimeError("インデックス範囲外")
  case DivisionByZero extends RuntimeError("0で割ることはできません")
  case StackUnderflow extends RuntimeError("スタックアンダーフロー")

object BasicInterpreter:

  def run(program: Program): Either[RuntimeError, List[String]] =
    val state = RuntimeState(mutable.Map.empty, Nil, Nil)
    val sorted = program.sortBy(_._1)
    executeProgram(sorted, 0, state)

  private def executeProgram(
    program: Program,
    pc: Int,
    state: RuntimeState
  ): Either[RuntimeError, List[String]] =
    if pc >= program.length then
      Right(state.output.reverse)
    else
      val (_, stmt) = program(pc)
      stmt match
        case Statement.End =>
          Right(state.output.reverse)

        case Statement.Let(varName, expr) =>
          evalExpr(expr, state.env).flatMap { value =>
            state.env(varName) = value
            executeProgram(program, pc + 1, state)
          }

        case Statement.Print(exprs) =>
          val results = exprs.map(evalExpr(_, state.env))
          results.collectFirst { case Left(err) => err } match
            case Some(error) => Left(error)
            case None =>
              val values = results.collect { case Right(v) => v }
              val output = values.map(valueToString).mkString(" ")
              executeProgram(program, pc + 1, state.copy(output = output :: state.output))

        case Statement.If(cond, thenBlock, elseBlock) =>
          evalExpr(cond, state.env).flatMap { condVal =>
            val branch = if isTruthy(condVal) then thenBlock else elseBlock
            executeBlock(branch, state).flatMap { newState =>
              executeProgram(program, pc + 1, newState)
            }
          }

        case Statement.For(varName, start, end, step, body) =>
          for
            startVal <- evalExpr(start, state.env)
            endVal <- evalExpr(end, state.env)
            stepVal <- evalExpr(step, state.env)
            result <- (startVal, endVal, stepVal) match
              case (Value.Number(s), Value.Number(e), Value.Number(st)) =>
                executeForLoop(varName, s, e, st, body, program, pc, state)
              case _ =>
                Left(RuntimeError.TypeMismatch("Number", "Other"))
          yield result

        case Statement.While(cond, body) =>
          executeWhileLoop(cond, body, program, pc, state)

        case Statement.Goto(target) =>
          findLine(program, target).flatMap { newPc =>
            executeProgram(program, newPc, state)
          }

        case Statement.Gosub(target) =>
          findLine(program, target).flatMap { newPc =>
            executeProgram(program, newPc, state.copy(callStack = (pc + 1) :: state.callStack))
          }

        case Statement.Return =>
          state.callStack match
            case returnPc :: rest =>
              executeProgram(program, returnPc, state.copy(callStack = rest))
            case Nil =>
              Left(RuntimeError.StackUnderflow)

        case Statement.Dim(varName, sizeExpr) =>
          evalExpr(sizeExpr, state.env).flatMap {
            case Value.Number(size) =>
              val array = Vector.fill(size.toInt)(Value.Number(0.0))
              state.env(varName) = Value.Array(array)
              executeProgram(program, pc + 1, state)
            case _ =>
              Left(RuntimeError.TypeMismatch("Number", "Other"))
          }

  private def executeBlock(
    block: List[Statement],
    state: RuntimeState
  ): Either[RuntimeError, RuntimeState] =
    block.foldLeft[Either[RuntimeError, RuntimeState]](Right(state)) { (acc, stmt) =>
      acc.flatMap(s => executeSingleStatement(stmt, s))
    }

  private def executeSingleStatement(
    stmt: Statement,
    state: RuntimeState
  ): Either[RuntimeError, RuntimeState] =
    stmt match
      case Statement.Let(varName, expr) =>
        evalExpr(expr, state.env).map { value =>
          state.env(varName) = value
          state
        }

      case Statement.Print(exprs) =>
        val results = exprs.map(evalExpr(_, state.env))
        results.collectFirst { case Left(err) => err } match
          case Some(error) => Left(error)
          case None =>
            val values = results.collect { case Right(v) => v }
            val output = values.map(valueToString).mkString(" ")
            Right(state.copy(output = output :: state.output))

      case _ =>
        Right(state)

  private def executeForLoop(
    varName: String,
    current: Double,
    end: Double,
    step: Double,
    body: List[Statement],
    program: Program,
    pc: Int,
    state: RuntimeState
  ): Either[RuntimeError, List[String]] =
    if (step > 0.0 && current > end) || (step < 0.0 && current < end) then
      executeProgram(program, pc + 1, state)
    else
      state.env(varName) = Value.Number(current)
      executeBlock(body, state).flatMap { newState =>
        executeForLoop(varName, current + step, end, step, body, program, pc, newState)
      }

  private def executeWhileLoop(
    cond: Expr,
    body: List[Statement],
    program: Program,
    pc: Int,
    state: RuntimeState
  ): Either[RuntimeError, List[String]] =
    evalExpr(cond, state.env).flatMap { condVal =>
      if isTruthy(condVal) then
        executeBlock(body, state).flatMap { newState =>
          executeWhileLoop(cond, body, program, pc, newState)
        }
      else
        executeProgram(program, pc + 1, state)
    }

  private def evalExpr(expr: Expr, env: mutable.Map[String, Value]): Either[RuntimeError, Value] =
    expr match
      case Expr.Number(n) =>
        Right(Value.Number(n))

      case Expr.Str(s) =>
        Right(Value.Str(s))

      case Expr.Variable(name) =>
        env.get(name).toRight(RuntimeError.UndefinedVariable(name))

      case Expr.ArrayAccess(varName, indexExpr) =>
        for
          arrayVal <- env.get(varName).toRight(RuntimeError.UndefinedVariable(varName))
          array <- arrayVal match
            case Value.Array(arr) => Right(arr)
            case _ => Left(RuntimeError.TypeMismatch("Array", "Other"))
          indexVal <- evalExpr(indexExpr, env)
          index <- indexVal match
            case Value.Number(idx) => Right(idx.toInt)
            case _ => Left(RuntimeError.TypeMismatch("Number", "Other"))
          result <- array.lift(index).toRight(RuntimeError.IndexOutOfBounds)
        yield result

      case Expr.BinOp(op, left, right) =>
        for
          l <- evalExpr(left, env)
          r <- evalExpr(right, env)
          result <- evalBinOp(op, l, r)
        yield result

      case Expr.UnaryOp(op, operand) =>
        evalExpr(operand, env).flatMap(evalUnaryOp(op, _))

  private def evalBinOp(op: BinOperator, left: Value, right: Value): Either[RuntimeError, Value] =
    (op, left, right) match
      case (BinOperator.Add, Value.Number(l), Value.Number(r)) =>
        Right(Value.Number(l + r))
      case (BinOperator.Sub, Value.Number(l), Value.Number(r)) =>
        Right(Value.Number(l - r))
      case (BinOperator.Mul, Value.Number(l), Value.Number(r)) =>
        Right(Value.Number(l * r))
      case (BinOperator.Div, Value.Number(l), Value.Number(r)) =>
        if r == 0.0 then Left(RuntimeError.DivisionByZero)
        else Right(Value.Number(l / r))
      case (BinOperator.Eq, Value.Number(l), Value.Number(r)) =>
        Right(Value.Number(if l == r then 1.0 else 0.0))
      case (BinOperator.Ne, Value.Number(l), Value.Number(r)) =>
        Right(Value.Number(if l != r then 1.0 else 0.0))
      case (BinOperator.Lt, Value.Number(l), Value.Number(r)) =>
        Right(Value.Number(if l < r then 1.0 else 0.0))
      case (BinOperator.Le, Value.Number(l), Value.Number(r)) =>
        Right(Value.Number(if l <= r then 1.0 else 0.0))
      case (BinOperator.Gt, Value.Number(l), Value.Number(r)) =>
        Right(Value.Number(if l > r then 1.0 else 0.0))
      case (BinOperator.Ge, Value.Number(l), Value.Number(r)) =>
        Right(Value.Number(if l >= r then 1.0 else 0.0))
      case (BinOperator.And, l, r) =>
        Right(Value.Number(if isTruthy(l) && isTruthy(r) then 1.0 else 0.0))
      case (BinOperator.Or, l, r) =>
        Right(Value.Number(if isTruthy(l) || isTruthy(r) then 1.0 else 0.0))
      case _ =>
        Left(RuntimeError.TypeMismatch("Number", "Other"))

  private def evalUnaryOp(op: UnaryOperator, operand: Value): Either[RuntimeError, Value] =
    (op, operand) match
      case (UnaryOperator.Neg, Value.Number(n)) =>
        Right(Value.Number(-n))
      case (UnaryOperator.Not, v) =>
        Right(Value.Number(if isTruthy(v) then 0.0 else 1.0))
      case _ =>
        Left(RuntimeError.TypeMismatch("Number", "Other"))

  private def isTruthy(value: Value): Boolean =
    value match
      case Value.Number(n) => n != 0.0
      case Value.Str(s) => s.nonEmpty
      case Value.Array(a) => a.nonEmpty

  private def valueToString(value: Value): String =
    value match
      case Value.Number(n) => n.toString
      case Value.Str(s) => s
      case Value.Array(_) => "[Array]"

  private def findLine(program: Program, target: Int): Either[RuntimeError, Int] =
    program.indexWhere(_._1 == target) match
      case -1 => Left(RuntimeError.UndefinedLabel(target))
      case idx => Right(idx)

// テスト実行例
@main def testBasicInterpreter(): Unit =
  val program: Program = List(
    (10, Statement.Let("x", Expr.Number(0.0))),
    (20, Statement.Let("x", Expr.BinOp(BinOperator.Add, Expr.Variable("x"), Expr.Number(1.0)))),
    (30, Statement.Print(List(Expr.Variable("x")))),
    (40, Statement.If(
      Expr.BinOp(BinOperator.Lt, Expr.Variable("x"), Expr.Number(5.0)),
      List(Statement.Goto(20)),
      Nil
    )),
    (50, Statement.End)
  )

  BasicInterpreter.run(program) match
    case Right(output) =>
      output.foreach(println)
    case Left(error) =>
      println(s"エラー: ${error.message}")
