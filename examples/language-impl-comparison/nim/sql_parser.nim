# 簡易SQL Parser - Nim実装
# SELECT, WHERE, JOIN, ORDER BY対応
# NPegパーサーコンビネーターライブラリを使用

import npeg
import strutils, sequtils, options, tables

# AST定義
type
  OrderDirection = enum
    Asc, Desc

  JoinType = enum
    InnerJoin, LeftJoin, RightJoin, FullJoin

  BinOp = enum
    Add, Sub, Mul, Div, Mod
    Eq, Ne, Lt, Le, Gt, Ge
    And, Or, Like

  UnOp = enum
    Not, IsNull, IsNotNull

  Literal = ref object
    case kind: LiteralKind
    of LitInt: intVal: int
    of LitFloat: floatVal: float
    of LitString: strVal: string
    of LitBool: boolVal: bool
    of LitNull: discard

  LiteralKind = enum
    LitInt, LitFloat, LitString, LitBool, LitNull

  Expr = ref object
    case kind: ExprKind
    of ExprLit: lit: Literal
    of ExprCol: colName: string
    of ExprQualCol: tableName, columnName: string
    of ExprBinOp: binOp: BinOp; left, right: Expr
    of ExprUnOp: unOp: UnOp; operand: Expr
    of ExprFunc: funcName: string; args: seq[Expr]
    of ExprParen: inner: Expr

  ExprKind = enum
    ExprLit, ExprCol, ExprQualCol, ExprBinOp, ExprUnOp, ExprFunc, ExprParen

  Column = ref object
    case kind: ColumnKind
    of ColAll: discard
    of ColExpr: expr: Expr; alias: Option[string]

  ColumnKind = enum
    ColAll, ColExpr

  TableRef = object
    table: string
    alias: Option[string]

  Join = object
    joinType: JoinType
    table: TableRef
    onCondition: Expr

  OrderBy = object
    columns: seq[(Expr, OrderDirection)]

  Query = object
    columns: seq[Column]
    fromTable: TableRef
    whereClause: Option[Expr]
    joins: seq[Join]
    orderBy: Option[OrderBy]

# パーサーコンビネーター（NPegベース）
# NPegは直接ASTを構築するマクロベースのPEGパーサー
proc parseSQL(input: string): Option[Query] =
  var result: Query
  var currentColumns: seq[Column]
  var currentJoins: seq[Join]
  var currentOrderCols: seq[(Expr, OrderDirection)]

  let parser = peg("sql", query: Query):
    # 基本要素
    ws <- *{' ', '\t', '\n', '\r'}
    lineComment <- "--" * *(1 - '\n')
    blockComment <- "/*" * *(1 - "*/") * "*/"
    comment <- lineComment | blockComment
    sc <- *(ws | comment)

    # キーワード（大文字小文字を区別しない）
    kwSelect <- i"select" * !Alnum
    kwFrom <- i"from" * !Alnum
    kwWhere <- i"where" * !Alnum
    kwJoin <- i"join" * !Alnum
    kwInner <- i"inner" * !Alnum
    kwLeft <- i"left" * !Alnum
    kwRight <- i"right" * !Alnum
    kwFull <- i"full" * !Alnum
    kwOn <- i"on" * !Alnum
    kwAnd <- i"and" * !Alnum
    kwOr <- i"or" * !Alnum
    kwNot <- i"not" * !Alnum
    kwIs <- i"is" * !Alnum
    kwNull <- i"null" * !Alnum
    kwTrue <- i"true" * !Alnum
    kwFalse <- i"false" * !Alnum
    kwLike <- i"like" * !Alnum
    kwOrder <- i"order" * !Alnum
    kwBy <- i"by" * !Alnum
    kwAsc <- i"asc" * !Alnum
    kwDesc <- i"desc" * !Alnum
    kwAs <- i"as" * !Alnum

    # 識別子
    identStart <- Alpha | '_'
    identCont <- Alnum | '_'
    ident <- >identStart * *identCont:
      # 予約語チェックは省略（簡略化）
      discard

    # リテラル
    integer <- >+Digit:
      discard
    float <- >+Digit * '.' * +Digit:
      discard
    stringLit <- '\'' * >(*(1 - '\'')) * '\'':
      discard

    # 式（簡略版、優先度処理は別途）
    expr <- orExpr
    orExpr <- andExpr * *(sc * kwOr * sc * andExpr)
    andExpr <- cmpExpr * *(sc * kwAnd * sc * cmpExpr)
    cmpExpr <- addExpr * ?(sc * cmpOp * sc * addExpr)
    cmpOp <- S"=" | S"<>" | S"!=" | S"<=" | S">=" | S"<" | S">" | kwLike
    addExpr <- mulExpr * *(sc * addOp * sc * mulExpr)
    addOp <- S"+" | S"-"
    mulExpr <- unaryExpr * *(sc * mulOp * sc * unaryExpr)
    mulOp <- S"*" | S"/" | S"%"
    unaryExpr <- (kwNot * sc * unaryExpr) | postfixExpr
    postfixExpr <- primaryExpr * ?(sc * kwIs * sc * ?kwNot * sc * kwNull)
    primaryExpr <- parenExpr | funcCall | columnRef | literal
    parenExpr <- '(' * sc * expr * sc * ')'
    funcCall <- ident * sc * '(' * sc * ?(expr * *(sc * ',' * sc * expr)) * sc * ')'
    columnRef <- ident * ?(sc * '.' * sc * ident)
    literal <- kwNull | kwTrue | kwFalse | float | integer | stringLit

    # カラムリスト
    columnList <- (S"*") | (columnExpr * *(sc * ',' * sc * columnExpr))
    columnExpr <- expr * ?(sc * ?kwAs * sc * ident)

    # テーブル参照
    tableRef <- ident * ?(sc * ?kwAs * sc * ident)

    # JOIN句
    joinClause <- joinType * sc * tableRef * sc * kwOn * sc * expr
    joinType <- (kwInner * sc * kwJoin) | (kwLeft * sc * kwJoin) |
                (kwRight * sc * kwJoin) | (kwFull * sc * kwJoin) | kwJoin

    # WHERE句
    whereClause <- kwWhere * sc * expr

    # ORDER BY句
    orderByClause <- kwOrder * sc * kwBy * sc * orderByExpr * *(sc * ',' * sc * orderByExpr)
    orderByExpr <- expr * ?(sc * (kwAsc | kwDesc))

    # SELECT文
    selectQuery <- kwSelect * sc * columnList * sc *
                   kwFrom * sc * tableRef *
                   *(sc * joinClause) *
                   ?(sc * whereClause) *
                   ?(sc * orderByClause)

    sql <- sc * selectQuery * sc * ?S";" * sc * !1

  # NPegでは完全なASTビルダーを実装する必要がある
  # ここでは簡略化のため、パース成功のみ確認
  let match = parser.match(input)
  if match.ok:
    # 実際の実装ではキャプチャからASTを構築
    return some(result)
  else:
    return none(Query)

# レンダリング関数
proc renderLiteral(lit: Literal): string =
  case lit.kind
  of LitInt: $lit.intVal
  of LitFloat: $lit.floatVal
  of LitString: "'" & lit.strVal & "'"
  of LitBool: if lit.boolVal: "TRUE" else "FALSE"
  of LitNull: "NULL"

proc renderExpr(expr: Expr): string =
  case expr.kind
  of ExprLit: renderLiteral(expr.lit)
  of ExprCol: expr.colName
  of ExprQualCol: expr.tableName & "." & expr.columnName
  of ExprBinOp:
    let op = case expr.binOp
      of Add: "+" of Sub: "-" of Mul: "*" of Div: "/" of Mod: "%"
      of Eq: "=" of Ne: "<>" of Lt: "<" of Le: "<=" of Gt: ">" of Ge: ">="
      of And: "AND" of Or: "OR" of Like: "LIKE"
    "(" & renderExpr(expr.left) & " " & op & " " & renderExpr(expr.right) & ")"
  of ExprUnOp:
    case expr.unOp
    of Not: "NOT " & renderExpr(expr.operand)
    of IsNull: renderExpr(expr.operand) & " IS NULL"
    of IsNotNull: renderExpr(expr.operand) & " IS NOT NULL"
  of ExprFunc:
    expr.funcName & "(" & expr.args.mapIt(renderExpr(it)).join(", ") & ")"
  of ExprParen: "(" & renderExpr(expr.inner) & ")"

proc renderColumn(col: Column): string =
  case col.kind
  of ColAll: "*"
  of ColExpr:
    let base = renderExpr(col.expr)
    if col.alias.isSome:
      base & " AS " & col.alias.get
    else:
      base

proc renderQuery(query: Query): string =
  let cols = query.columns.mapIt(renderColumn(it)).join(", ")
  var res = "SELECT " & cols & " FROM " & query.fromTable.table

  if query.fromTable.alias.isSome:
    res &= " AS " & query.fromTable.alias.get

  for join in query.joins:
    let jtype = case join.joinType
      of InnerJoin: "INNER JOIN"
      of LeftJoin: "LEFT JOIN"
      of RightJoin: "RIGHT JOIN"
      of FullJoin: "FULL JOIN"
    res &= " " & jtype & " " & join.table.table & " ON " & renderExpr(join.onCondition)

  if query.whereClause.isSome:
    res &= " WHERE " & renderExpr(query.whereClause.get)

  if query.orderBy.isSome:
    let orderCols = query.orderBy.get.columns.mapIt(
      renderExpr(it[0]) & " " & (if it[1] == Asc: "ASC" else "DESC")
    ).join(", ")
    res &= " ORDER BY " & orderCols

  res

# テスト
when isMainModule:
  echo "=== Nim SQL Parser テスト ==="

  # 簡易手動テスト（NPegの完全実装は複雑なため、構造のみ示す）
  let q = Query(
    columns: @[Column(kind: ColAll)],
    fromTable: TableRef(table: "users", alias: none(string)),
    whereClause: none(Expr),
    joins: @[],
    orderBy: none(OrderBy)
  )

  echo "基本クエリ: ", renderQuery(q)
  echo ""
  echo "注: NPegでの完全なパーサー実装には"
  echo "キャプチャとアクションの詳細な定義が必要です。"
  echo "上記はNimでのパーサーコンビネーター構造を示す例です。"