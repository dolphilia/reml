// 簡易SQL Parser
// SELECT, WHERE, JOIN, ORDER BY など基本的な構文のみ対応

module SQLParser

// AST定義
type Literal =
    | IntLit of int
    | StringLit of string
    | BoolLit of bool
    | NullLit

type BinOp =
    | Add | Sub | Mul | Div | Mod
    | Eq | Ne | Lt | Le | Gt | Ge
    | And | Or | Like

type UnOp =
    | Not
    | IsNull
    | IsNotNull

type Expr =
    | Literal of Literal
    | Column of string
    | QualifiedColumn of table:string * column:string
    | BinaryOp of op:BinOp * left:Expr * right:Expr
    | UnaryOp of op:UnOp * expr:Expr
    | FunctionCall of name:string * args:Expr list
    | Parenthesized of Expr

type Column =
    | AllColumns
    | ColumnExpr of expr:Expr * alias:string option

type TableRef = {
    Table: string
    Alias: string option
}

type JoinType =
    | InnerJoin
    | LeftJoin
    | RightJoin
    | FullJoin

type Join = {
    JoinType: JoinType
    Table: TableRef
    OnCondition: Expr
}

type OrderDirection = Asc | Desc

type Query = {
    Columns: Column list
    From: TableRef
    WhereClause: Expr option
    Joins: Join list
    OrderBy: (Expr * OrderDirection) list option
}

// パーサーコンビネーター
type Parser<'a> = string * int -> Result<'a * int, string>

let run (parser: Parser<'a>) (input: string) : Result<'a, string> =
    match parser (input, 0) with
    | Ok (result, pos) when pos >= input.Length -> Ok result
    | Ok (_, pos) -> Error $"Unexpected input at position {pos}"
    | Error msg -> Error msg

let ok value : Parser<'a> = fun (input, pos) -> Ok (value, pos)

let fail msg : Parser<'a> = fun (input, pos) -> Error msg

let bind (parser: Parser<'a>) (f: 'a -> Parser<'b>) : Parser<'b> =
    fun (input, pos) ->
        match parser (input, pos) with
        | Ok (result, pos') -> (f result) (input, pos')
        | Error msg -> Error msg

let map (f: 'a -> 'b) (parser: Parser<'a>) : Parser<'b> =
    bind parser (fun a -> ok (f a))

let orElse (parser1: Parser<'a>) (parser2: Parser<'a>) : Parser<'a> =
    fun (input, pos) ->
        match parser1 (input, pos) with
        | Ok _ as result -> result
        | Error _ -> parser2 (input, pos)

let many (parser: Parser<'a>) : Parser<'a list> =
    let rec loop acc (input, pos) =
        match parser (input, pos) with
        | Ok (result, pos') -> loop (result :: acc) (input, pos')
        | Error _ -> Ok (List.rev acc, pos)
    loop []

let optional (parser: Parser<'a>) : Parser<'a option> =
    fun (input, pos) ->
        match parser (input, pos) with
        | Ok (result, pos') -> Ok (Some result, pos')
        | Error _ -> Ok (None, pos)

// 基本パーサー
let skipWhitespace : Parser<unit> =
    fun (input, pos) ->
        let rec skip p =
            if p >= input.Length then p
            elif System.Char.IsWhiteSpace(input.[p]) then skip (p + 1)
            else p
        Ok ((), skip pos)

let lexeme (parser: Parser<'a>) : Parser<'a> =
    bind skipWhitespace (fun _ ->
        bind parser (fun result ->
            bind skipWhitespace (fun _ -> ok result)))

let symbol (str: string) : Parser<unit> =
    lexeme (fun (input, pos) ->
        if pos + str.Length <= input.Length &&
           input.Substring(pos, str.Length) = str then
            Ok ((), pos + str.Length)
        else
            Error $"Expected '{str}'")

let keyword (kw: string) : Parser<unit> =
    bind skipWhitespace (fun _ ->
        fun (input, pos) ->
            if pos + kw.Length <= input.Length then
                let slice = input.Substring(pos, kw.Length)
                if slice.ToLower() = kw.ToLower() then
                    let nextPos = pos + kw.Length
                    if nextPos < input.Length &&
                       System.Char.IsLetterOrDigit(input.[nextPos]) then
                        Error "Keyword boundary"
                    else
                        (bind skipWhitespace (fun _ -> ok ())) (input, nextPos)
                else
                    Error $"Expected keyword '{kw}'"
            else
                Error $"Expected keyword '{kw}'")

let reservedWords = Set.ofList [
    "select"; "from"; "where"; "join"; "inner"; "left"; "right"; "full";
    "on"; "and"; "or"; "not"; "like"; "order"; "by"; "asc"; "desc";
    "null"; "true"; "false"; "as"; "is"
]

let identifier : Parser<string> =
    bind skipWhitespace (fun _ ->
        fun (input, pos) ->
            if pos >= input.Length then
                Error "Expected identifier"
            else
                let start = input.[pos]
                if System.Char.IsLetter(start) || start = '_' then
                    let rec collect p acc =
                        if p >= input.Length then (acc, p)
                        else
                            let c = input.[p]
                            if System.Char.IsLetterOrDigit(c) || c = '_' then
                                collect (p + 1) (acc + string c)
                            else
                                (acc, p)
                    let (name, nextPos) = collect (pos + 1) (string start)
                    if reservedWords.Contains(name.ToLower()) then
                        Error $"Reserved word '{name}' cannot be used as identifier"
                    else
                        (bind skipWhitespace (fun _ -> ok name)) (input, nextPos)
                else
                    Error "Expected identifier")

let intLiteral : Parser<int> =
    bind skipWhitespace (fun _ ->
        fun (input, pos) ->
            if pos >= input.Length || not (System.Char.IsDigit(input.[pos])) then
                Error "Expected integer"
            else
                let rec collect p acc =
                    if p >= input.Length then (acc, p)
                    else
                        let c = input.[p]
                        if System.Char.IsDigit(c) then
                            collect (p + 1) (acc + string c)
                        else
                            (acc, p)
                let (digits, nextPos) = collect pos ""
                match System.Int32.TryParse(digits) with
                | true, n -> (bind skipWhitespace (fun _ -> ok n)) (input, nextPos)
                | false, _ -> Error "Invalid integer")

let stringLiteral : Parser<string> =
    bind skipWhitespace (fun _ ->
        fun (input, pos) ->
            if pos >= input.Length || input.[pos] <> '\'' then
                Error "Expected string literal"
            else
                let rec collect p acc =
                    if p >= input.Length then
                        Error "Unclosed string"
                    elif input.[p] = '\'' then
                        Ok (acc, p + 1)
                    else
                        collect (p + 1) (acc + string input.[p])
                match collect (pos + 1) "" with
                | Ok (str, nextPos) -> (bind skipWhitespace (fun _ -> ok str)) (input, nextPos)
                | Error msg -> Error msg)

let literalParser : Parser<Literal> =
    orElse (bind (keyword "null") (fun _ -> ok NullLit))
        (orElse (bind (keyword "true") (fun _ -> ok (BoolLit true)))
            (orElse (bind (keyword "false") (fun _ -> ok (BoolLit false)))
                (orElse (map IntLit intLiteral)
                    (map StringLit stringLiteral))))

// 式パーサー（演算子優先度対応）
let rec exprParser : Parser<Expr> = fun input -> orExpr input

and orExpr : Parser<Expr> =
    bind andExpr (fun left -> orExprCont left)

and orExprCont (left: Expr) : Parser<Expr> =
    orElse
        (bind (keyword "or") (fun _ ->
            bind andExpr (fun right ->
                orExprCont (BinaryOp(Or, left, right)))))
        (ok left)

and andExpr : Parser<Expr> =
    bind comparisonExpr (fun left -> andExprCont left)

and andExprCont (left: Expr) : Parser<Expr> =
    orElse
        (bind (keyword "and") (fun _ ->
            bind comparisonExpr (fun right ->
                andExprCont (BinaryOp(And, left, right)))))
        (ok left)

and comparisonExpr : Parser<Expr> =
    bind additiveExpr (fun left ->
        orElse (bind (symbol "<=") (fun _ -> map (fun r -> BinaryOp(Le, left, r)) additiveExpr))
            (orElse (bind (symbol ">=") (fun _ -> map (fun r -> BinaryOp(Ge, left, r)) additiveExpr))
                (orElse (bind (symbol "<>") (fun _ -> map (fun r -> BinaryOp(Ne, left, r)) additiveExpr))
                    (orElse (bind (symbol "!=") (fun _ -> map (fun r -> BinaryOp(Ne, left, r)) additiveExpr))
                        (orElse (bind (symbol "=") (fun _ -> map (fun r -> BinaryOp(Eq, left, r)) additiveExpr))
                            (orElse (bind (symbol "<") (fun _ -> map (fun r -> BinaryOp(Lt, left, r)) additiveExpr))
                                (orElse (bind (symbol ">") (fun _ -> map (fun r -> BinaryOp(Gt, left, r)) additiveExpr))
                                    (orElse (bind (keyword "like") (fun _ -> map (fun r -> BinaryOp(Like, left, r)) additiveExpr))
                                        (ok left)))))))))

and additiveExpr : Parser<Expr> =
    bind multiplicativeExpr (fun left -> additiveExprCont left)

and additiveExprCont (left: Expr) : Parser<Expr> =
    orElse
        (bind (symbol "+") (fun _ ->
            bind multiplicativeExpr (fun right ->
                additiveExprCont (BinaryOp(Add, left, right)))))
        (orElse
            (bind (symbol "-") (fun _ ->
                bind multiplicativeExpr (fun right ->
                    additiveExprCont (BinaryOp(Sub, left, right)))))
            (ok left))

and multiplicativeExpr : Parser<Expr> =
    bind postfixExpr (fun left -> multiplicativeExprCont left)

and multiplicativeExprCont (left: Expr) : Parser<Expr> =
    orElse
        (bind (symbol "*") (fun _ ->
            bind postfixExpr (fun right ->
                multiplicativeExprCont (BinaryOp(Mul, left, right)))))
        (orElse
            (bind (symbol "/") (fun _ ->
                bind postfixExpr (fun right ->
                    multiplicativeExprCont (BinaryOp(Div, left, right)))))
            (orElse
                (bind (symbol "%") (fun _ ->
                    bind postfixExpr (fun right ->
                        multiplicativeExprCont (BinaryOp(Mod, left, right)))))
                (ok left)))

and postfixExpr : Parser<Expr> =
    bind unaryExpr (fun expr -> postfixExprCont expr)

and postfixExprCont (expr: Expr) : Parser<Expr> =
    orElse
        (bind (keyword "is") (fun _ ->
            orElse
                (bind (keyword "not") (fun _ ->
                    bind (keyword "null") (fun _ ->
                        postfixExprCont (UnaryOp(IsNotNull, expr)))))
                (bind (keyword "null") (fun _ ->
                    postfixExprCont (UnaryOp(IsNull, expr))))))
        (ok expr)

and unaryExpr : Parser<Expr> =
    orElse
        (bind (keyword "not") (fun _ ->
            map (fun e -> UnaryOp(Not, e)) unaryExpr))
        atomExpr

and atomExpr : Parser<Expr> =
    orElse
        (bind (symbol "(") (fun _ ->
            bind exprParser (fun expr ->
                bind (symbol ")") (fun _ ->
                    ok (Parenthesized expr)))))
        (orElse
            (bind identifier (fun name ->
                orElse
                    (bind (symbol "(") (fun _ ->
                        bind functionArgs (fun args ->
                            bind (symbol ")") (fun _ ->
                                ok (FunctionCall(name, args))))))
                    (orElse
                        (bind (symbol ".") (fun _ ->
                            map (fun col -> QualifiedColumn(name, col)) identifier))
                        (ok (Column name)))))
            (map Literal literalParser))

and functionArgs : Parser<Expr list> =
    orElse
        (bind exprParser (fun first ->
            bind (many (bind (symbol ",") (fun _ -> exprParser))) (fun rest ->
                ok (first :: rest))))
        (ok [])

// クエリパーサー
let columnListParser : Parser<Column list> =
    orElse
        (bind (symbol "*") (fun _ -> ok [AllColumns]))
        (bind exprParser (fun first ->
            bind (optional (orElse (bind (keyword "as") (fun _ -> identifier)) identifier)) (fun alias ->
                bind (many (bind (symbol ",") (fun _ ->
                    bind exprParser (fun expr ->
                        map (fun a -> ColumnExpr(expr, a)) (optional (orElse (bind (keyword "as") (fun _ -> identifier)) identifier)))))) (fun rest ->
                    ok (ColumnExpr(first, alias) :: rest)))))

let tableRefParser : Parser<TableRef> =
    bind identifier (fun table ->
        map (fun alias -> { Table = table; Alias = alias })
            (optional (orElse (bind (keyword "as") (fun _ -> identifier)) identifier)))

let joinTypeParser : Parser<JoinType> =
    orElse (bind (keyword "inner") (fun _ -> bind (keyword "join") (fun _ -> ok InnerJoin)))
        (orElse (bind (keyword "left") (fun _ -> bind (keyword "join") (fun _ -> ok LeftJoin)))
            (orElse (bind (keyword "right") (fun _ -> bind (keyword "join") (fun _ -> ok RightJoin)))
                (orElse (bind (keyword "full") (fun _ -> bind (keyword "join") (fun _ -> ok FullJoin)))
                    (bind (keyword "join") (fun _ -> ok InnerJoin)))))

let joinParser : Parser<Join> =
    bind joinTypeParser (fun joinType ->
        bind tableRefParser (fun table ->
            bind (keyword "on") (fun _ ->
                map (fun condition -> { JoinType = joinType; Table = table; OnCondition = condition })
                    exprParser)))

let orderByItemParser : Parser<Expr * OrderDirection> =
    bind exprParser (fun expr ->
        map (fun dir -> (expr, dir))
            (orElse (bind (keyword "asc") (fun _ -> ok Asc))
                (orElse (bind (keyword "desc") (fun _ -> ok Desc))
                    (ok Asc))))

let orderByParser : Parser<(Expr * OrderDirection) list> =
    bind (keyword "order") (fun _ ->
        bind (keyword "by") (fun _ ->
            bind orderByItemParser (fun first ->
                bind (many (bind (symbol ",") (fun _ -> orderByItemParser))) (fun rest ->
                    ok (first :: rest)))))

let selectQueryParser : Parser<Query> =
    bind (keyword "select") (fun _ ->
        bind columnListParser (fun columns ->
            bind (keyword "from") (fun _ ->
                bind tableRefParser (fun from ->
                    bind (many joinParser) (fun joins ->
                        bind (optional (bind (keyword "where") (fun _ -> exprParser))) (fun whereClause ->
                            bind (optional orderByParser) (fun orderBy ->
                                ok {
                                    Columns = columns
                                    From = from
                                    WhereClause = whereClause
                                    Joins = joins
                                    OrderBy = orderBy
                                })))))))

let parse (input: string) : Result<Query, string> =
    bind skipWhitespace (fun _ ->
        bind selectQueryParser (fun query ->
            bind (optional (symbol ";")) (fun _ ->
                bind skipWhitespace (fun _ -> ok query))))
    |> fun p -> run p input

// レンダリング（検証用）
let rec renderExpr (expr: Expr) : string =
    match expr with
    | Literal lit -> renderLiteral lit
    | Column name -> name
    | QualifiedColumn(table, col) -> $"{table}.{col}"
    | BinaryOp(op, left, right) ->
        $"({renderExpr left} {renderBinOp op} {renderExpr right})"
    | UnaryOp(op, e) ->
        match op with
        | Not -> $"NOT {renderExpr e}"
        | IsNull -> $"{renderExpr e} IS NULL"
        | IsNotNull -> $"{renderExpr e} IS NOT NULL"
    | FunctionCall(name, args) ->
        let argsStr = args |> List.map renderExpr |> String.concat ", "
        $"{name}({argsStr})"
    | Parenthesized e -> $"({renderExpr e})"

and renderLiteral (lit: Literal) : string =
    match lit with
    | IntLit n -> string n
    | StringLit s -> $"'{s}'"
    | BoolLit b -> if b then "TRUE" else "FALSE"
    | NullLit -> "NULL"

and renderBinOp (op: BinOp) : string =
    match op with
    | Add -> "+" | Sub -> "-" | Mul -> "*" | Div -> "/" | Mod -> "%"
    | Eq -> "=" | Ne -> "<>" | Lt -> "<" | Le -> "<=" | Gt -> ">" | Ge -> ">="
    | And -> "AND" | Or -> "OR" | Like -> "LIKE"

let renderColumn (col: Column) : string =
    match col with
    | AllColumns -> "*"
    | ColumnExpr(expr, alias) ->
        renderExpr expr + (alias |> Option.map (fun a -> $" AS {a}") |> Option.defaultValue "")

let renderQuery (query: Query) : string =
    let colsStr = query.Columns |> List.map renderColumn |> String.concat ", "
    let fromStr = $"FROM {query.From.Table}" +
                  (query.From.Alias |> Option.map (fun a -> $" AS {a}") |> Option.defaultValue "")
    let joinsStr = query.Joins |> List.map (fun j ->
        let joinType = match j.JoinType with
                       | InnerJoin -> "INNER JOIN"
                       | LeftJoin -> "LEFT JOIN"
                       | RightJoin -> "RIGHT JOIN"
                       | FullJoin -> "FULL JOIN"
        $"{joinType} {j.Table.Table} ON {renderExpr j.OnCondition}"
    ) |> String.concat " "
    let whereStr = query.WhereClause |> Option.map (fun e -> $" WHERE {renderExpr e}") |> Option.defaultValue ""
    let orderStr = query.OrderBy |> Option.map (fun items ->
        let cols = items |> List.map (fun (e, dir) ->
            let dirStr = if dir = Asc then "ASC" else "DESC"
            $"{renderExpr e} {dirStr}"
        ) |> String.concat ", "
        $" ORDER BY {cols}"
    ) |> Option.defaultValue ""
    $"SELECT {colsStr} {fromStr} {joinsStr}{whereStr}{orderStr}"

// テスト
[<EntryPoint>]
let main argv =
    let testCases = [
        "SELECT * FROM users"
        "SELECT name, age FROM users WHERE age > 18"
        "SELECT u.name, o.total FROM users u INNER JOIN orders o ON u.id = o.user_id"
        "SELECT name FROM users WHERE active = true ORDER BY name ASC"
    ]

    printfn "=== SQL Parser Test ==="
    for sql in testCases do
        printfn "\nInput: %s" sql
        match parse sql with
        | Ok query ->
            printfn "Parsed: OK"
            printfn "Rendered: %s" (renderQuery query)
        | Error msg ->
            printfn "Error: %s" msg

    0