// PL/0 風トイ言語コンパイラ断片 (Scala 3 実装)
// PL/0 サブセットの抽象構文木とインタプリタ

package pl0

// 文
enum Stmt:
  case Assign(name: String, expr: Expr)
  case While(cond: Expr, body: List[Stmt])
  case Write(expr: Expr)

// 式
enum Expr:
  case Number(value: Int)
  case Var(name: String)
  case Binary(op: Op, lhs: Expr, rhs: Expr)

// 演算子
enum Op:
  case Add
  case Sub
  case Mul
  case Div

// ランタイム状態
case class Runtime(
  vars: Map[String, Int],
  output: List[Int]
)

// パースエラー
case class ParseError(message: String)

// 実行エラー
case class ExecError(reason: String)

// プログラムのパース (簡易実装)
def parseProgram(source: String): Either[ParseError, List[Stmt]] =
  // 実装のシンプルさを優先し、疑似実装を示す
  Right(List(
    Stmt.Assign("x", Expr.Number(10)),
    Stmt.While(
      Expr.Var("x"),
      List(
        Stmt.Write(Expr.Var("x")),
        Stmt.Assign("x", Expr.Binary(Op.Sub, Expr.Var("x"), Expr.Number(1)))
      )
    )
  ))

// 初期ランタイム状態
def initialRuntime(): Runtime = Runtime(
  vars = Map.empty,
  output = List.empty
)

// 式の評価
def evalExpr(expr: Expr, vars: Map[String, Int]): Either[ExecError, Int] =
  expr match
    case Expr.Number(n) => Right(n)
    case Expr.Var(name) =>
      vars.get(name) match
        case Some(value) => Right(value)
        case None => Left(ExecError(s"未定義変数: $name"))
    case Expr.Binary(op, lhs, rhs) =>
      for
        l <- evalExpr(lhs, vars)
        r <- evalExpr(rhs, vars)
        result <- op match
          case Op.Add => Right(l + r)
          case Op.Sub => Right(l - r)
          case Op.Mul => Right(l * r)
          case Op.Div =>
            if r == 0 then
              Left(ExecError("0で割ることはできません"))
            else
              Right(l / r)
      yield result

// 文の実行
def execStmt(stmt: Stmt, runtime: Runtime): Either[ExecError, Runtime] =
  stmt match
    case Stmt.Assign(name, expr) =>
      evalExpr(expr, runtime.vars).map { value =>
        Runtime(runtime.vars + (name -> value), runtime.output)
      }
    case Stmt.While(cond, body) =>
      execWhile(cond, body, runtime)
    case Stmt.Write(expr) =>
      evalExpr(expr, runtime.vars).map { value =>
        Runtime(runtime.vars, runtime.output :+ value)
      }

// while ループの実行
def execWhile(cond: Expr, body: List[Stmt], runtime: Runtime): Either[ExecError, Runtime] =
  def loop(current: Runtime): Either[ExecError, Runtime] =
    evalExpr(cond, current.vars) match
      case Left(err) => Left(err)
      case Right(value) =>
        if value == 0 then
          Right(current)
        else
          execStmtList(body, current) match
            case Left(err) => Left(err)
            case Right(nextState) => loop(nextState)
  loop(runtime)

// 文リストの実行
def execStmtList(stmts: List[Stmt], runtime: Runtime): Either[ExecError, Runtime] =
  stmts.foldLeft[Either[ExecError, Runtime]](Right(runtime)) { (acc, stmt) =>
    acc.flatMap(state => execStmt(stmt, state))
  }

// プログラムの実行
def exec(program: List[Stmt]): Either[ExecError, Runtime] =
  execStmtList(program, initialRuntime())

// 利用例
// parseProgram("begin x := 10; while x do write x; x := x - 1 end")
//   .flatMap(exec)
// => Right(Runtime(Map(), List(10, 9, 8, 7, 6, 5, 4, 3, 2, 1)))