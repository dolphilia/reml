// 簡易SQL Parser - Scala 3実装
// SELECT, WHERE, JOIN, ORDER BY対応
// scala-parser-combinatorsライブラリを使用

import scala.util.parsing.combinator._
import scala.util.parsing.input.CharSequenceReader

// AST定義
enum OrderDirection:
  case Asc, Desc

enum JoinType:
  case InnerJoin, LeftJoin, RightJoin, FullJoin

enum BinOp:
  case Add, Sub, Mul, Div, Mod
  case Eq, Ne, Lt, Le, Gt, Ge
  case And, Or, Like

enum UnOp:
  case Not, IsNull, IsNotNull

enum Literal:
  case IntLit(value: Int)
  case FloatLit(value: Double)
  case StringLit(value: String)
  case BoolLit(value: Boolean)
  case NullLit

enum Expr:
  case Literal(lit: sql_parser.Literal)
  case Column(name: String)
  case QualifiedColumn(table: String, column: String)
  case BinaryOp(op: BinOp, left: Expr, right: Expr)
  case UnaryOp(op: UnOp, expr: Expr)
  case FunctionCall(name: String, args: List[Expr])
  case Parenthesized(expr: Expr)

enum Column:
  case AllColumns
  case ColumnExpr(expr: Expr, alias: Option[String])

case class TableRef(table: String, alias: Option[String])

case class Join(
  joinType: JoinType,
  table: TableRef,
  onCondition: Expr
)

case class OrderBy(columns: List[(Expr, OrderDirection)])

case class Query(
  columns: List[Column],
  fromTable: TableRef,
  whereClause: Option[Expr],
  joins: List[Join],
  orderBy: Option[OrderBy]
)

// パーサーコンビネーター
object SQLParser extends RegexParsers:
  override def skipWhitespace = true

  // 予約語
  private val reserved = Set(
    "select", "from", "where", "join", "inner", "left",
    "right", "full", "on", "and", "or", "not", "like",
    "order", "by", "asc", "desc", "null", "true", "false", "as"
  )

  def keyword(kw: String): Parser[Unit] =
    s"(?i)$kw\\b".r ^^ { _ => () }

  def identifier: Parser[String] =
    """[a-zA-Z_][a-zA-Z0-9_]*""".r.filter { id =>
      !reserved.contains(id.toLowerCase)
    } withFailureMessage "Reserved word cannot be used as identifier"

  // リテラル
  def integer: Parser[Literal] =
    """\d+""".r ^^ { s => Literal.IntLit(s.toInt) }

  def floatLit: Parser[Literal] =
    """\d+\.\d+""".r ^^ { s => Literal.FloatLit(s.toDouble) }

  def stringLit: Parser[Literal] =
    """'([^']*)'""".r ^^ { s =>
      Literal.StringLit(s.substring(1, s.length - 1))
    }

  def literal: Parser[Literal] =
    keyword("null") ^^^ Literal.NullLit |
    keyword("true") ^^^ Literal.BoolLit(true) |
    keyword("false") ^^^ Literal.BoolLit(false) |
    floatLit |
    integer |
    stringLit

  // 式（演算子優先度を考慮）
  def expr: Parser[Expr] = orExpr

  def orExpr: Parser[Expr] =
    andExpr ~ rep(keyword("or") ~> andExpr) ^^ {
      case first ~ rest =>
        rest.foldLeft(first) { (acc, e) =>
          Expr.BinaryOp(BinOp.Or, acc, e)
        }
    }

  def andExpr: Parser[Expr] =
    cmpExpr ~ rep(keyword("and") ~> cmpExpr) ^^ {
      case first ~ rest =>
        rest.foldLeft(first) { (acc, e) =>
          Expr.BinaryOp(BinOp.And, acc, e)
        }
    }

  def cmpExpr: Parser[Expr] =
    addExpr ~ opt(cmpOp ~ addExpr) ^^ {
      case left ~ None => left
      case left ~ Some(op ~ right) => Expr.BinaryOp(op, left, right)
    }

  def cmpOp: Parser[BinOp] =
    "=" ^^^ BinOp.Eq |
    "<>" ^^^ BinOp.Ne |
    "!=" ^^^ BinOp.Ne |
    "<=" ^^^ BinOp.Le |
    ">=" ^^^ BinOp.Ge |
    "<" ^^^ BinOp.Lt |
    ">" ^^^ BinOp.Gt |
    keyword("like") ^^^ BinOp.Like

  def addExpr: Parser[Expr] =
    mulExpr ~ rep(addOp ~ mulExpr) ^^ {
      case first ~ rest =>
        rest.foldLeft(first) { case (acc, op ~ e) =>
          Expr.BinaryOp(op, acc, e)
        }
    }

  def addOp: Parser[BinOp] =
    "+" ^^^ BinOp.Add |
    "-" ^^^ BinOp.Sub

  def mulExpr: Parser[Expr] =
    unaryExpr ~ rep(mulOp ~ unaryExpr) ^^ {
      case first ~ rest =>
        rest.foldLeft(first) { case (acc, op ~ e) =>
          Expr.BinaryOp(op, acc, e)
        }
    }

  def mulOp: Parser[BinOp] =
    "*" ^^^ BinOp.Mul |
    "/" ^^^ BinOp.Div |
    "%" ^^^ BinOp.Mod

  def unaryExpr: Parser[Expr] =
    keyword("not") ~> unaryExpr ^^ { e =>
      Expr.UnaryOp(UnOp.Not, e)
    } |
    postfixExpr

  def postfixExpr: Parser[Expr] =
    primaryExpr ~ opt(keyword("is") ~> opt(keyword("not")) <~ keyword("null")) ^^ {
      case e ~ None => e
      case e ~ Some(Some(_)) => Expr.UnaryOp(UnOp.IsNotNull, e)
      case e ~ Some(None) => Expr.UnaryOp(UnOp.IsNull, e)
    }

  def primaryExpr: Parser[Expr] =
    "(" ~> expr <~ ")" ^^ { e => Expr.Parenthesized(e) } |
    functionCall |
    columnRef |
    literal ^^ { lit => Expr.Literal(lit) }

  def functionCall: Parser[Expr] =
    identifier ~ ("(" ~> repsep(expr, ",") <~ ")") ^^ {
      case name ~ args => Expr.FunctionCall(name, args)
    }

  def columnRef: Parser[Expr] =
    identifier ~ opt("." ~> identifier) ^^ {
      case first ~ None => Expr.Column(first)
      case table ~ Some(col) => Expr.QualifiedColumn(table, col)
    }

  // カラムリスト
  def columnList: Parser[List[Column]] =
    "*" ^^^ List(Column.AllColumns) |
    repsep(columnExpr, ",")

  def columnExpr: Parser[Column] =
    expr ~ opt(opt(keyword("as")) ~> identifier) ^^ {
      case e ~ alias => Column.ColumnExpr(e, alias)
    }

  // テーブル参照
  def tableRef: Parser[TableRef] =
    identifier ~ opt(opt(keyword("as")) ~> identifier) ^^ {
      case table ~ alias => TableRef(table, alias)
    }

  // JOIN句
  def joinClause: Parser[Join] =
    joinType ~ tableRef ~ (keyword("on") ~> expr) ^^ {
      case jt ~ tbl ~ cond => Join(jt, tbl, cond)
    }

  def joinType: Parser[JoinType] =
    keyword("inner") ~ keyword("join") ^^^ JoinType.InnerJoin |
    keyword("left") ~ keyword("join") ^^^ JoinType.LeftJoin |
    keyword("right") ~ keyword("join") ^^^ JoinType.RightJoin |
    keyword("full") ~ keyword("join") ^^^ JoinType.FullJoin |
    keyword("join") ^^^ JoinType.InnerJoin

  // WHERE句
  def whereClause: Parser[Expr] =
    keyword("where") ~> expr

  // ORDER BY句
  def orderByClause: Parser[OrderBy] =
    keyword("order") ~ keyword("by") ~> repsep(orderExpr, ",") ^^ { cols =>
      OrderBy(cols)
    }

  def orderExpr: Parser[(Expr, OrderDirection)] =
    expr ~ opt(
      keyword("asc") ^^^ OrderDirection.Asc |
      keyword("desc") ^^^ OrderDirection.Desc
    ) ^^ {
      case e ~ dir => (e, dir.getOrElse(OrderDirection.Asc))
    }

  // SELECT文
  def selectQuery: Parser[Query] =
    keyword("select") ~> columnList ~
    (keyword("from") ~> tableRef) ~
    rep(joinClause) ~
    opt(whereClause) ~
    opt(orderByClause) <~ opt(";") ^^ {
      case cols ~ from ~ joins ~ where ~ order =>
        Query(cols, from, where, joins, order)
    }

  // パブリックAPI
  def parse(input: String): Either[String, Query] =
    parseAll(selectQuery, input) match
      case Success(result, _) => Right(result)
      case NoSuccess(msg, _) => Left(msg)

// レンダリング関数
object SQLRenderer:
  import Literal.*, Expr.*, Column.*, BinOp.*, UnOp.*, OrderDirection.*

  def renderLiteral(lit: Literal): String = lit match
    case IntLit(n) => n.toString
    case FloatLit(f) => f.toString
    case StringLit(s) => s"'$s'"
    case BoolLit(b) => if b then "TRUE" else "FALSE"
    case NullLit => "NULL"

  def renderBinOp(op: BinOp): String = op match
    case Add => "+" case Sub => "-" case Mul => "*"
    case Div => "/" case Mod => "%"
    case Eq => "=" case Ne => "<>" case Lt => "<"
    case Le => "<=" case Gt => ">" case Ge => ">="
    case And => "AND" case Or => "OR" case Like => "LIKE"

  def renderExpr(expr: Expr): String = expr match
    case Literal(lit) => renderLiteral(lit)
    case Column(name) => name
    case QualifiedColumn(table, col) => s"$table.$col"
    case BinaryOp(op, left, right) =>
      s"(${renderExpr(left)} ${renderBinOp(op)} ${renderExpr(right)})"
    case UnaryOp(Not, e) => s"NOT ${renderExpr(e)}"
    case UnaryOp(IsNull, e) => s"${renderExpr(e)} IS NULL"
    case UnaryOp(IsNotNull, e) => s"${renderExpr(e)} IS NOT NULL"
    case FunctionCall(name, args) =>
      s"$name(${args.map(renderExpr).mkString(", ")})"
    case Parenthesized(e) => s"(${renderExpr(e)})"

  def renderColumn(col: Column): String = col match
    case AllColumns => "*"
    case ColumnExpr(e, None) => renderExpr(e)
    case ColumnExpr(e, Some(alias)) => s"${renderExpr(e)} AS $alias"

  def renderQuery(q: Query): String =
    val cols = q.columns.map(renderColumn).mkString(", ")
    val from = s"FROM ${q.fromTable.table}" +
      q.fromTable.alias.map(a => s" AS $a").getOrElse("")

    val joins = q.joins.map { j =>
      val jt = j.joinType match
        case JoinType.InnerJoin => "INNER JOIN"
        case JoinType.LeftJoin => "LEFT JOIN"
        case JoinType.RightJoin => "RIGHT JOIN"
        case JoinType.FullJoin => "FULL JOIN"
      s"$jt ${j.table.table} ON ${renderExpr(j.onCondition)}"
    }.mkString(" ")

    val where = q.whereClause.map(e => s" WHERE ${renderExpr(e)}").getOrElse("")

    val order = q.orderBy.map { ob =>
      val cols = ob.columns.map { (e, dir) =>
        s"${renderExpr(e)} ${if dir == Asc then "ASC" else "DESC"}"
      }.mkString(", ")
      s" ORDER BY $cols"
    }.getOrElse("")

    s"SELECT $cols $from $joins$where$order".trim

// テスト
@main def sqlParserTest(): Unit =
  println("=== Scala 3 SQL Parser テスト ===")

  val testSQL = "SELECT name, age FROM users WHERE age > 18 ORDER BY name ASC"
  SQLParser.parse(testSQL) match
    case Right(query) =>
      println(s"パース成功: $testSQL")
      println(s"レンダリング: ${SQLRenderer.renderQuery(query)}")
    case Left(error) =>
      println(s"パースエラー: $error")

  println()
  val complexSQL = """
    SELECT u.name, COUNT(o.id) AS order_count
    FROM users u
    INNER JOIN orders o ON u.id = o.user_id
    WHERE u.age > 18
    ORDER BY order_count DESC
  """
  SQLParser.parse(complexSQL) match
    case Right(query) =>
      println(s"複雑なクエリのパース成功")
      println(s"レンダリング: ${SQLRenderer.renderQuery(query)}")
    case Left(error) =>
      println(s"パースエラー: $error")