// ミニ Lisp 評価機 (Scala 3 実装)
// S式構文を持つ式を解析して評価する

package miniLisp

import scala.util.{Try, Success, Failure}

// 式の抽象構文木
enum Expr:
  case Number(value: Double)
  case Symbol(name: String)
  case List(items: scala.collection.immutable.List[Expr])

// 評価値
enum Value:
  case VNumber(value: Double)
  case VLambda(params: scala.collection.immutable.List[String], body: Expr, env: Env)
  case VBuiltin(fn: scala.collection.immutable.List[Value] => Either[String, Value])

type Env = Map[String, Value]

// パースエラー
enum ParseError:
  case UnexpectedToken(token: String)
  case UnmatchedParen
  case EmptyInput

// トークン化: S式の括弧をスペースで区切る
def tokenize(source: String): scala.collection.immutable.List[String] =
  source
    .replace("(", " ( ")
    .replace(")", " ) ")
    .split("\\s+")
    .toList
    .filter(_.nonEmpty)

// 式のパース
def parseExpr(tokens: scala.collection.immutable.List[String]): Either[ParseError, (Expr, scala.collection.immutable.List[String])] =
  tokens match
    case Nil => Left(ParseError.EmptyInput)
    case token :: rest => parseToken(token, rest)

def parseToken(token: String, rest: scala.collection.immutable.List[String]): Either[ParseError, (Expr, scala.collection.immutable.List[String])] =
  if token == "(" then
    parseList(rest, Nil)
  else if token == ")" then
    Left(ParseError.UnmatchedParen)
  else
    Try(token.toDouble) match
      case Success(num) => Right((Expr.Number(num), rest))
      case Failure(_) => Right((Expr.Symbol(token), rest))

def parseList(tokens: scala.collection.immutable.List[String], acc: scala.collection.immutable.List[Expr]): Either[ParseError, (Expr, scala.collection.immutable.List[String])] =
  tokens match
    case Nil => Left(ParseError.UnmatchedParen)
    case ")" :: rest => Right((Expr.List(acc.reverse), rest))
    case token :: rest =>
      parseToken(token, rest) match
        case Right((expr, next)) => parseList(next, expr :: acc)
        case Left(err) => Left(err)

// 式の評価
def evalExpr(expr: Expr, env: Env): Either[String, Value] =
  expr match
    case Expr.Number(n) => Right(Value.VNumber(n))
    case Expr.Symbol(name) =>
      env.get(name) match
        case Some(value) => Right(value)
        case None => Left(s"未定義シンボル: $name")
    case Expr.List(items) => evalList(items, env)

def evalList(items: scala.collection.immutable.List[Expr], env: Env): Either[String, Value] =
  items match
    case Nil => Left("空のリストは評価できません")
    case head :: rest =>
      for
        callee <- evalExpr(head, env)
        args <- evaluateArgs(rest, env)
        result <- apply(callee, args)
      yield result

def evaluateArgs(exprs: scala.collection.immutable.List[Expr], env: Env): Either[String, scala.collection.immutable.List[Value]] =
  exprs.foldLeft[Either[String, scala.collection.immutable.List[Value]]](Right(Nil)) { (acc, expr) =>
    for
      values <- acc
      value <- evalExpr(expr, env)
    yield values :+ value
  }

def apply(callee: Value, args: scala.collection.immutable.List[Value]): Either[String, Value] =
  callee match
    case Value.VBuiltin(fn) => fn(args)
    case Value.VLambda(params, body, lambdaEnv) => applyLambda(params, body, lambdaEnv, args)
    case Value.VNumber(_) => Left("数値を関数として適用できません")

def applyLambda(params: scala.collection.immutable.List[String], body: Expr, lambdaEnv: Env, args: scala.collection.immutable.List[Value]): Either[String, Value] =
  if params.length != args.length then
    Left("引数の数が一致しません")
  else
    val newEnv = lambdaEnv ++ params.zip(args).toMap
    evalExpr(body, newEnv)

// 組み込み数値演算
def builtinNumeric(op: (Double, Double) => Double): scala.collection.immutable.List[Value] => Either[String, Value] =
  args => args match
    case scala.collection.immutable.List(Value.VNumber(lhs), Value.VNumber(rhs)) => Right(Value.VNumber(op(lhs, rhs)))
    case _ => Left("数値演算は2引数の数値のみ対応します")

// デフォルト環境
def defaultEnv(): Env = Map(
  "+" -> Value.VBuiltin(builtinNumeric(_ + _)),
  "-" -> Value.VBuiltin(builtinNumeric(_ - _)),
  "*" -> Value.VBuiltin(builtinNumeric(_ * _)),
  "/" -> Value.VBuiltin(builtinNumeric(_ / _))
)

// メイン評価関数
def eval(source: String): Either[String, Value] =
  val tokens = tokenize(source)
  parseExpr(tokens) match
    case Left(ParseError.EmptyInput) => Left("入力が空です")
    case Left(ParseError.UnmatchedParen) => Left("括弧が一致しません")
    case Left(ParseError.UnexpectedToken(token)) => Left(s"予期しないトークン: $token")
    case Right((expr, rest)) =>
      if rest.isEmpty then
        evalExpr(expr, defaultEnv())
      else
        Left("末尾に未消費トークンがあります")

// 利用例
// eval("(+ 40 2)") => Right(VNumber(42.0))