module SQLParser exposing (Query, parse, render)

{-| 簡易SQL Parser
SELECT, WHERE, JOIN, ORDER BY など基本的な構文のみ対応
-}

import Parser exposing (..)
import Set exposing (Set)


-- AST定義


type Query
    = SelectQuery
        { columns : List Column
        , from : TableRef
        , whereClause : Maybe Expr
        , joins : List Join
        , orderBy : Maybe (List ( Expr, OrderDirection ))
        }


type Column
    = AllColumns
    | ColumnExpr Expr (Maybe String)


type alias TableRef =
    { table : String
    , alias : Maybe String
    }


type alias Join =
    { joinType : JoinType
    , table : TableRef
    , onCondition : Expr
    }


type JoinType
    = InnerJoin
    | LeftJoin
    | RightJoin
    | FullJoin


type OrderDirection
    = Asc
    | Desc


type Expr
    = LiteralExpr Literal
    | ColumnRef String
    | QualifiedColumn String String
    | BinaryOp BinOp Expr Expr
    | UnaryOp UnOp Expr
    | FunctionCall String (List Expr)
    | Parenthesized Expr


type Literal
    = IntLit Int
    | StringLit String
    | BoolLit Bool
    | NullLit


type BinOp
    = Add
    | Sub
    | Mul
    | Div
    | Mod
    | Eq
    | Ne
    | Lt
    | Le
    | Gt
    | Ge
    | And
    | Or
    | Like


type UnOp
    = Not
    | IsNull
    | IsNotNull



-- Parser Combinators


spaces : Parser ()
spaces =
    chompWhile (\c -> c == ' ' || c == '\t' || c == '\n' || c == '\r')


lexeme : Parser a -> Parser a
lexeme p =
    succeed identity
        |. spaces
        |= p
        |. spaces


symbol : String -> Parser ()
symbol str =
    lexeme (Parser.keyword str)
        |> Parser.map (\_ -> ())


keyword : String -> Parser ()
keyword kw =
    succeed ()
        |. spaces
        |. Parser.keyword kw
        |. spaces


reservedWords : Set String
reservedWords =
    Set.fromList
        [ "select"
        , "from"
        , "where"
        , "join"
        , "inner"
        , "left"
        , "right"
        , "full"
        , "on"
        , "and"
        , "or"
        , "not"
        , "like"
        , "order"
        , "by"
        , "asc"
        , "desc"
        , "null"
        , "true"
        , "false"
        , "as"
        , "is"
        ]


identifier : Parser String
identifier =
    succeed identity
        |. spaces
        |= (variable
                { start = \c -> Char.isAlpha c || c == '_'
                , inner = \c -> Char.isAlphaNum c || c == '_'
                , reserved = reservedWords
                }
           )
        |. spaces


literalParser : Parser Literal
literalParser =
    oneOf
        [ succeed NullLit
            |. keyword "null"
        , succeed (BoolLit True)
            |. keyword "true"
        , succeed (BoolLit False)
            |. keyword "false"
        , succeed IntLit
            |= lexeme int
        , succeed StringLit
            |= lexeme stringLiteral
        ]


stringLiteral : Parser String
stringLiteral =
    succeed identity
        |. Parser.symbol "'"
        |= (getChompedString <|
                chompWhile (\c -> c /= '\'')
           )
        |. Parser.symbol "'"



-- Expression Parser


exprParser : Parser Expr
exprParser =
    orExpr


orExpr : Parser Expr
orExpr =
    andExpr
        |> Parser.andThen (\left -> orExprCont left)


orExprCont : Expr -> Parser Expr
orExprCont left =
    oneOf
        [ succeed (BinaryOp Or left)
            |. keyword "or"
            |= andExpr
            |> Parser.andThen orExprCont
        , succeed left
        ]


andExpr : Parser Expr
andExpr =
    comparisonExpr
        |> Parser.andThen (\left -> andExprCont left)


andExprCont : Expr -> Parser Expr
andExprCont left =
    oneOf
        [ succeed (BinaryOp And left)
            |. keyword "and"
            |= comparisonExpr
            |> Parser.andThen andExprCont
        , succeed left
        ]


comparisonExpr : Parser Expr
comparisonExpr =
    additiveExpr
        |> Parser.andThen
            (\left ->
                oneOf
                    [ succeed (BinaryOp Le left)
                        |. symbol "<="
                        |= additiveExpr
                    , succeed (BinaryOp Ge left)
                        |. symbol ">="
                        |= additiveExpr
                    , succeed (BinaryOp Ne left)
                        |. symbol "<>"
                        |= additiveExpr
                    , succeed (BinaryOp Ne left)
                        |. symbol "!="
                        |= additiveExpr
                    , succeed (BinaryOp Eq left)
                        |. symbol "="
                        |= additiveExpr
                    , succeed (BinaryOp Lt left)
                        |. symbol "<"
                        |= additiveExpr
                    , succeed (BinaryOp Gt left)
                        |. symbol ">"
                        |= additiveExpr
                    , succeed (BinaryOp Like left)
                        |. keyword "like"
                        |= additiveExpr
                    , succeed left
                    ]
            )


additiveExpr : Parser Expr
additiveExpr =
    multiplicativeExpr
        |> Parser.andThen (\left -> additiveExprCont left)


additiveExprCont : Expr -> Parser Expr
additiveExprCont left =
    oneOf
        [ succeed (BinaryOp Add left)
            |. symbol "+"
            |= multiplicativeExpr
            |> Parser.andThen additiveExprCont
        , succeed (BinaryOp Sub left)
            |. symbol "-"
            |= multiplicativeExpr
            |> Parser.andThen additiveExprCont
        , succeed left
        ]


multiplicativeExpr : Parser Expr
multiplicativeExpr =
    postfixExpr
        |> Parser.andThen (\left -> multiplicativeExprCont left)


multiplicativeExprCont : Expr -> Parser Expr
multiplicativeExprCont left =
    oneOf
        [ succeed (BinaryOp Mul left)
            |. symbol "*"
            |= postfixExpr
            |> Parser.andThen multiplicativeExprCont
        , succeed (BinaryOp Div left)
            |. symbol "/"
            |= postfixExpr
            |> Parser.andThen multiplicativeExprCont
        , succeed (BinaryOp Mod left)
            |. symbol "%"
            |= postfixExpr
            |> Parser.andThen multiplicativeExprCont
        , succeed left
        ]


postfixExpr : Parser Expr
postfixExpr =
    unaryExpr
        |> Parser.andThen (\expr -> postfixExprCont expr)


postfixExprCont : Expr -> Parser Expr
postfixExprCont expr =
    oneOf
        [ succeed (UnaryOp IsNotNull expr)
            |. keyword "is"
            |. keyword "not"
            |. keyword "null"
            |> Parser.andThen postfixExprCont
        , succeed (UnaryOp IsNull expr)
            |. keyword "is"
            |. keyword "null"
            |> Parser.andThen postfixExprCont
        , succeed expr
        ]


unaryExpr : Parser Expr
unaryExpr =
    oneOf
        [ succeed (UnaryOp Not)
            |. keyword "not"
            |= Parser.lazy (\_ -> unaryExpr)
        , atomExpr
        ]


atomExpr : Parser Expr
atomExpr =
    oneOf
        [ succeed identity
            |. symbol "("
            |= Parser.lazy (\_ -> exprParser)
            |. symbol ")"
            |> Parser.map Parenthesized
        , succeed identity
            |= identifier
            |> Parser.andThen
                (\name ->
                    oneOf
                        [ -- Function call
                          succeed (FunctionCall name)
                            |. symbol "("
                            |= functionArgs
                            |. symbol ")"
                        , -- Qualified column
                          succeed (\col -> QualifiedColumn name col)
                            |. symbol "."
                            |= identifier
                        , -- Simple column
                          succeed (ColumnRef name)
                        ]
                )
        , succeed LiteralExpr
            |= literalParser
        ]


functionArgs : Parser (List Expr)
functionArgs =
    oneOf
        [ succeed identity
            |= Parser.sequence
                { start = ""
                , separator = ","
                , end = ""
                , spaces = spaces
                , item = Parser.lazy (\_ -> exprParser)
                , trailing = Parser.Forbidden
                }
        , succeed []
        ]



-- Column List Parser


columnListParser : Parser (List Column)
columnListParser =
    oneOf
        [ succeed [ AllColumns ]
            |. symbol "*"
        , Parser.sequence
            { start = ""
            , separator = ","
            , end = ""
            , spaces = spaces
            , item = columnExprParser
            , trailing = Parser.Forbidden
            }
        ]


columnExprParser : Parser Column
columnExprParser =
    succeed (\expr maybeAlias -> ColumnExpr expr maybeAlias)
        |= exprParser
        |= oneOf
            [ succeed Just
                |. keyword "as"
                |= identifier
            , succeed Nothing
            ]



-- Table Reference Parser


tableRefParser : Parser TableRef
tableRefParser =
    succeed (\table maybeAlias -> TableRef table maybeAlias)
        |= identifier
        |= oneOf
            [ succeed Just
                |. keyword "as"
                |= identifier
            , succeed Just
                |= identifier
            , succeed Nothing
            ]



-- Join Parser


joinParser : Parser Join
joinParser =
    succeed (\joinType table condition -> Join joinType table condition)
        |= joinTypeParser
        |= tableRefParser
        |. keyword "on"
        |= exprParser


joinTypeParser : Parser JoinType
joinTypeParser =
    oneOf
        [ succeed InnerJoin
            |. keyword "inner"
            |. keyword "join"
        , succeed LeftJoin
            |. keyword "left"
            |. keyword "join"
        , succeed RightJoin
            |. keyword "right"
            |. keyword "join"
        , succeed FullJoin
            |. keyword "full"
            |. keyword "join"
        , succeed InnerJoin
            |. keyword "join"
        ]



-- Order By Parser


orderByParser : Parser (List ( Expr, OrderDirection ))
orderByParser =
    succeed identity
        |. keyword "order"
        |. keyword "by"
        |= Parser.sequence
            { start = ""
            , separator = ","
            , end = ""
            , spaces = spaces
            , item = orderByItemParser
            , trailing = Parser.Forbidden
            }


orderByItemParser : Parser ( Expr, OrderDirection )
orderByItemParser =
    succeed (\expr dir -> ( expr, dir ))
        |= exprParser
        |= oneOf
            [ succeed Asc
                |. keyword "asc"
            , succeed Desc
                |. keyword "desc"
            , succeed Asc
            ]



-- SELECT Query Parser


selectQueryParser : Parser Query
selectQueryParser =
    succeed
        (\columns from joins whereClause orderBy ->
            SelectQuery
                { columns = columns
                , from = from
                , whereClause = whereClause
                , joins = joins
                , orderBy = orderBy
                }
        )
        |. keyword "select"
        |= columnListParser
        |. keyword "from"
        |= tableRefParser
        |= Parser.loop [] joinParserLoop
        |= oneOf
            [ succeed Just
                |. keyword "where"
                |= exprParser
            , succeed Nothing
            ]
        |= oneOf
            [ succeed Just
                |= orderByParser
            , succeed Nothing
            ]


joinParserLoop : List Join -> Parser (Step (List Join) (List Join))
joinParserLoop acc =
    oneOf
        [ succeed (\join -> Loop (join :: acc))
            |= joinParser
        , succeed (Done (List.reverse acc))
        ]



-- Public API


parse : String -> Result (List DeadEnd) Query
parse input =
    run
        (succeed identity
            |. spaces
            |= selectQueryParser
            |. spaces
            |. oneOf [ symbol ";", succeed () ]
            |. spaces
            |. end
        )
        input



-- Rendering (for verification)


render : Query -> String
render (SelectQuery { columns, from, whereClause, joins, orderBy }) =
    let
        colsStr =
            renderColumns columns

        fromStr =
            "FROM " ++ from.table ++ Maybe.withDefault "" (Maybe.map (\a -> " AS " ++ a) from.alias)

        joinsStr =
            joins
                |> List.map renderJoin
                |> String.join " "

        whereStr =
            case whereClause of
                Just expr ->
                    " WHERE " ++ renderExpr expr

                Nothing ->
                    ""

        orderStr =
            case orderBy of
                Just items ->
                    " ORDER BY "
                        ++ (items
                                |> List.map
                                    (\( expr, dir ) ->
                                        renderExpr expr
                                            ++ (case dir of
                                                    Asc ->
                                                        " ASC"

                                                    Desc ->
                                                        " DESC"
                                               )
                                    )
                                |> String.join ", "
                           )

                Nothing ->
                    ""
    in
    "SELECT " ++ colsStr ++ " " ++ fromStr ++ " " ++ joinsStr ++ whereStr ++ orderStr


renderColumns : List Column -> String
renderColumns columns =
    columns
        |> List.map
            (\col ->
                case col of
                    AllColumns ->
                        "*"

                    ColumnExpr expr maybeAlias ->
                        renderExpr expr ++ Maybe.withDefault "" (Maybe.map (\a -> " AS " ++ a) maybeAlias)
            )
        |> String.join ", "


renderJoin : Join -> String
renderJoin { joinType, table, onCondition } =
    let
        joinTypeStr =
            case joinType of
                InnerJoin ->
                    "INNER JOIN"

                LeftJoin ->
                    "LEFT JOIN"

                RightJoin ->
                    "RIGHT JOIN"

                FullJoin ->
                    "FULL JOIN"
    in
    joinTypeStr ++ " " ++ table.table ++ " ON " ++ renderExpr onCondition


renderExpr : Expr -> String
renderExpr expr =
    case expr of
        LiteralExpr lit ->
            renderLiteral lit

        ColumnRef name ->
            name

        QualifiedColumn table col ->
            table ++ "." ++ col

        BinaryOp op left right ->
            "(" ++ renderExpr left ++ " " ++ renderBinOp op ++ " " ++ renderExpr right ++ ")"

        UnaryOp op e ->
            case op of
                Not ->
                    "NOT " ++ renderExpr e

                IsNull ->
                    renderExpr e ++ " IS NULL"

                IsNotNull ->
                    renderExpr e ++ " IS NOT NULL"

        FunctionCall name args ->
            name ++ "(" ++ (args |> List.map renderExpr |> String.join ", ") ++ ")"

        Parenthesized e ->
            "(" ++ renderExpr e ++ ")"


renderLiteral : Literal -> String
renderLiteral lit =
    case lit of
        IntLit n ->
            String.fromInt n

        StringLit s ->
            "'" ++ s ++ "'"

        BoolLit b ->
            if b then
                "TRUE"

            else
                "FALSE"

        NullLit ->
            "NULL"


renderBinOp : BinOp -> String
renderBinOp op =
    case op of
        Add ->
            "+"

        Sub ->
            "-"

        Mul ->
            "*"

        Div ->
            "/"

        Mod ->
            "%"

        Eq ->
            "="

        Ne ->
            "<>"

        Lt ->
            "<"

        Le ->
            "<="

        Gt ->
            ">"

        Ge ->
            ">="

        And ->
            "AND"

        Or ->
            "OR"

        Like ->
            "LIKE"