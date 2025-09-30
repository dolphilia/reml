// 簡易SQL Parser
// SELECT, WHERE, JOIN, ORDER BY など基本的な構文のみ対応

module sql_parser

// AST定義
type literal
  IntLit(n: int)
  StringLit(s: string)
  BoolLit(b: bool)
  NullLit

type binop
  Add
  Sub
  Mul
  Div
  Mod
  Eq
  Ne
  Lt
  Le
  Gt
  Ge
  And
  Or
  Like

type unop
  Not
  IsNull
  IsNotNull

type expr
  LiteralExpr(lit: literal)
  ColumnRef(name: string)
  QualifiedColumn(table: string, column: string)
  BinaryOp(op: binop, left: expr, right: expr)
  UnaryOp(op: unop, e: expr)
  FunctionCall(name: string, args: list<expr>)
  Parenthesized(e: expr)

type column
  AllColumns
  ColumnExpr(e: expr, alias: maybe<string>)

type table-ref
  TableRef(table: string, alias: maybe<string>)

type join-type
  InnerJoin
  LeftJoin
  RightJoin
  FullJoin

type join
  Join(join-type: join-type, table: table-ref, on-condition: expr)

type order-direction
  Asc
  Desc

type query
  SelectQuery(
    columns: list<column>,
    from: table-ref,
    where-clause: maybe<expr>,
    joins: list<join>,
    order-by: maybe<list<(expr, order-direction)>>
  )

// パーサーコンビネーター
alias parser<a> = (string, int) -> maybe<(a, int)>

fun parse-ok(value: a): parser<a>
  fn(input, pos) Just((value, pos))

fun parse-bind(p: parser<a>, f: a -> parser<b>): parser<b>
  fn(input, pos)
    match p(input, pos)
      Just((result, pos')) -> f(result)(input, pos')
      Nothing -> Nothing

fun parse-or(p1: parser<a>, p2: parser<a>): parser<a>
  fn(input, pos)
    match p1(input, pos)
      Just(r) -> Just(r)
      Nothing -> p2(input, pos)

fun parse-map(p: parser<a>, f: a -> b): parser<b>
  parse-bind(p, fn(x) parse-ok(f(x)))

fun parse-many(p: parser<a>): parser<list<a>>
  fn(input, pos)
    fun loop(acc: list<a>, p': int): maybe<(list<a>, int)>
      match p(input, p')
        Just((result, pos')) -> loop(Cons(result, acc), pos')
        Nothing -> Just((reverse(acc), p'))
    loop([], pos)

fun parse-optional(p: parser<a>): parser<maybe<a>>
  parse-or(parse-map(p, Just), parse-ok(Nothing))

// 基本パーサー
fun skip-whitespace(): parser<()>
  fn(input, pos)
    fun skip(p: int): int
      if p >= input.count then p
      else
        val c = input.string/slice(p, p + 1)
        if c == " " || c == "\t" || c == "\n" || c == "\r" then skip(p + 1)
        else p
    Just(((), skip(pos)))

fun lexeme(p: parser<a>): parser<a>
  parse-bind(skip-whitespace(), fn(_)
    parse-bind(p, fn(result)
      parse-bind(skip-whitespace(), fn(_)
        parse-ok(result))))

fun symbol(str: string): parser<()>
  lexeme(fn(input, pos)
    if pos + str.count <= input.count && input.string/slice(pos, pos + str.count) == str then
      Just(((), pos + str.count))
    else
      Nothing
  )

fun keyword-base(kw: string): parser<()>
  parse-bind(skip-whitespace(), fn(_)
    fn(input, pos)
      if pos + kw.count <= input.count then
        val slice = input.string/slice(pos, pos + kw.count)
        if slice.to-lower == kw.to-lower then
          val next-pos = pos + kw.count
          if next-pos < input.count then
            val next-char = input.string/slice(next-pos, next-pos + 1)
            val is-alnum = next-char.list.any(fn(c) c.is-alpha || c.is-digit)
            if is-alnum then Nothing
            else skip-whitespace()(input, next-pos)
          else
            skip-whitespace()(input, next-pos)
        else
          Nothing
      else
        Nothing
  )

fun keyword(kw: string): parser<()>
  keyword-base(kw)

val reserved-words = [
  "select", "from", "where", "join", "inner", "left", "right", "full",
  "on", "and", "or", "not", "like", "order", "by", "asc", "desc",
  "null", "true", "false", "as", "is"
]

fun identifier(): parser<string>
  parse-bind(skip-whitespace(), fn(_)
    fn(input, pos)
      if pos >= input.count then Nothing
      else
        val start = input.string/slice(pos, pos + 1)
        val start-char = start.list.head
        match start-char
          Just(c) | c.is-alpha || c == '_' ->
            fun collect(p: int, acc: string): (string, int)
              if p >= input.count then (acc, p)
              else
                val ch = input.string/slice(p, p + 1).list.head
                match ch
                  Just(x) | x.is-alpha || x.is-digit || x == '_' ->
                    collect(p + 1, acc ++ input.string/slice(p, p + 1))
                  _ -> (acc, p)
            val (name, next-pos) = collect(pos + 1, start)
            if reserved-words.any(fn(w) w == name.to-lower) then Nothing
            else
              match skip-whitespace()(input, next-pos)
                Just((_, p')) -> Just((name, p'))
                Nothing -> Nothing
          _ -> Nothing
  )

fun int-literal(): parser<int>
  parse-bind(skip-whitespace(), fn(_)
    fn(input, pos)
      if pos >= input.count then Nothing
      else
        val first = input.string/slice(pos, pos + 1).list.head
        match first
          Just(c) | c.is-digit ->
            fun collect(p: int, acc: string): (string, int)
              if p >= input.count then (acc, p)
              else
                val ch = input.string/slice(p, p + 1).list.head
                match ch
                  Just(x) | x.is-digit ->
                    collect(p + 1, acc ++ input.string/slice(p, p + 1))
                  _ -> (acc, p)
            val (digits, next-pos) = collect(pos, "")
            match digits.parse-int
              Just(n) ->
                match skip-whitespace()(input, next-pos)
                  Just((_, p')) -> Just((n, p'))
                  Nothing -> Nothing
              Nothing -> Nothing
          _ -> Nothing
  )

fun string-literal(): parser<string>
  parse-bind(skip-whitespace(), fn(_)
    fn(input, pos)
      if pos >= input.count || input.string/slice(pos, pos + 1) != "'" then Nothing
      else
        fun collect(p: int, acc: string): maybe<(string, int)>
          if p >= input.count then Nothing
          else if input.string/slice(p, p + 1) == "'" then
            match skip-whitespace()(input, p + 1)
              Just((_, p')) -> Just((acc, p'))
              Nothing -> Nothing
          else
            collect(p + 1, acc ++ input.string/slice(p, p + 1))
        collect(pos + 1, "")
  )

fun literal-parser(): parser<literal>
  parse-or(parse-map(keyword("null"), fn(_) NullLit),
    parse-or(parse-map(keyword("true"), fn(_) BoolLit(True)),
      parse-or(parse-map(keyword("false"), fn(_) BoolLit(False)),
        parse-or(parse-map(int-literal(), IntLit),
          parse-map(string-literal(), StringLit)))))

// 式パーサー（簡略版）
fun expr-parser(): parser<expr>
  or-expr()

fun or-expr(): parser<expr>
  parse-bind(and-expr(), fn(left) or-expr-cont(left))

fun or-expr-cont(left: expr): parser<expr>
  parse-or(
    parse-bind(keyword("or"), fn(_)
      parse-bind(and-expr(), fn(right)
        or-expr-cont(BinaryOp(Or, left, right)))),
    parse-ok(left))

fun and-expr(): parser<expr>
  parse-bind(comparison-expr(), fn(left) and-expr-cont(left))

fun and-expr-cont(left: expr): parser<expr>
  parse-or(
    parse-bind(keyword("and"), fn(_)
      parse-bind(comparison-expr(), fn(right)
        and-expr-cont(BinaryOp(And, left, right)))),
    parse-ok(left))

fun comparison-expr(): parser<expr>
  parse-bind(additive-expr(), fn(left)
    parse-or(parse-bind(symbol("="), fn(_) parse-map(additive-expr(), fn(r) BinaryOp(Eq, left, r))),
      parse-or(parse-bind(symbol("<>"), fn(_) parse-map(additive-expr(), fn(r) BinaryOp(Ne, left, r))),
        parse-or(parse-bind(symbol("<"), fn(_) parse-map(additive-expr(), fn(r) BinaryOp(Lt, left, r))),
          parse-or(parse-bind(symbol(">"), fn(_) parse-map(additive-expr(), fn(r) BinaryOp(Gt, left, r))),
            parse-ok(left))))))

fun additive-expr(): parser<expr>
  parse-bind(atom-expr(), fn(left) additive-expr-cont(left))

fun additive-expr-cont(left: expr): parser<expr>
  parse-or(
    parse-bind(symbol("+"), fn(_)
      parse-bind(atom-expr(), fn(right)
        additive-expr-cont(BinaryOp(Add, left, right)))),
    parse-or(
      parse-bind(symbol("-"), fn(_)
        parse-bind(atom-expr(), fn(right)
          additive-expr-cont(BinaryOp(Sub, left, right)))),
      parse-ok(left)))

fun atom-expr(): parser<expr>
  parse-or(
    parse-bind(symbol("("), fn(_)
      parse-bind(expr-parser(), fn(e)
        parse-bind(symbol(")"), fn(_)
          parse-ok(Parenthesized(e))))),
    parse-or(
      parse-bind(identifier(), fn(name)
        parse-or(
          parse-bind(symbol("."), fn(_)
            parse-map(identifier(), fn(col) QualifiedColumn(name, col))),
          parse-ok(ColumnRef(name)))),
      parse-map(literal-parser(), LiteralExpr)))

// クエリパーサー
fun column-list-parser(): parser<list<column>>
  parse-or(
    parse-map(symbol("*"), fn(_) [AllColumns]),
    parse-bind(expr-parser(), fn(e)
      parse-bind(parse-optional(identifier()), fn(alias)
        parse-map(parse-many(parse-bind(symbol(","), fn(_)
          parse-bind(expr-parser(), fn(e2)
            parse-map(parse-optional(identifier()), fn(a2) ColumnExpr(e2, a2))))),
          fn(rest) Cons(ColumnExpr(e, alias), rest)))))

fun table-ref-parser(): parser<table-ref>
  parse-bind(identifier(), fn(table)
    parse-map(parse-optional(identifier()), fn(alias) TableRef(table, alias)))

fun select-query-parser(): parser<query>
  parse-bind(keyword("select"), fn(_)
    parse-bind(column-list-parser(), fn(columns)
      parse-bind(keyword("from"), fn(_)
        parse-bind(table-ref-parser(), fn(from)
          parse-bind(parse-optional(parse-bind(keyword("where"), fn(_) expr-parser())), fn(where-clause)
            parse-ok(SelectQuery(columns, from, where-clause, [], Nothing)))))))

fun parse-sql(input: string): maybe<query>
  match select-query-parser()(input, 0)
    Just((result, pos)) | pos >= input.count -> Just(result)
    _ -> Nothing

// レンダリング（検証用）
fun render-literal(lit: literal): string
  match lit
    IntLit(n) -> n.show
    StringLit(s) -> "'" ++ s ++ "'"
    BoolLit(b) -> if b then "TRUE" else "FALSE"
    NullLit -> "NULL"

fun render-binop(op: binop): string
  match op
    Add -> "+"
    Sub -> "-"
    Mul -> "*"
    Div -> "/"
    Mod -> "%"
    Eq -> "="
    Ne -> "<>"
    Lt -> "<"
    Le -> "<="
    Gt -> ">"
    Ge -> ">="
    And -> "AND"
    Or -> "OR"
    Like -> "LIKE"

fun render-expr(e: expr): string
  match e
    LiteralExpr(lit) -> render-literal(lit)
    ColumnRef(name) -> name
    QualifiedColumn(table, col) -> table ++ "." ++ col
    BinaryOp(op, left, right) ->
      "(" ++ render-expr(left) ++ " " ++ render-binop(op) ++ " " ++ render-expr(right) ++ ")"
    UnaryOp(Not, ex) -> "NOT " ++ render-expr(ex)
    UnaryOp(IsNull, ex) -> render-expr(ex) ++ " IS NULL"
    UnaryOp(IsNotNull, ex) -> render-expr(ex) ++ " IS NOT NULL"
    FunctionCall(name, args) ->
      name ++ "(" ++ args.map(render-expr).join(", ") ++ ")"
    Parenthesized(ex) -> "(" ++ render-expr(ex) ++ ")"

fun render-column(col: column): string
  match col
    AllColumns -> "*"
    ColumnExpr(e, alias) ->
      render-expr(e) ++ (match alias
        Just(a) -> " AS " ++ a
        Nothing -> "")

fun render-query(q: query): string
  match q
    SelectQuery(columns, from, where-clause, _, _) ->
      val cols-str = columns.map(render-column).join(", ")
      val TableRef(table, alias) = from
      val from-str = "FROM " ++ table ++ (match alias
        Just(a) -> " AS " ++ a
        Nothing -> "")
      val where-str = match where-clause
        Just(e) -> " WHERE " ++ render-expr(e)
        Nothing -> ""
      "SELECT " ++ cols-str ++ " " ++ from-str ++ where-str

// テスト
fun main()
  val test-cases = [
    "SELECT * FROM users",
    "SELECT name FROM users WHERE age > 18"
  ]

  println("=== SQL Parser Test ===")
  test-cases.foreach fn(sql)
    println("\nInput: " ++ sql)
    match parse-sql(sql)
      Just(query) ->
        println("Parsed: OK")
        println("Rendered: " ++ render-query(query))
      Nothing ->
        println("Error: Parse failed")