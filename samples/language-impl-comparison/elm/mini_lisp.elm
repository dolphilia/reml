-- ミニ Lisp 評価機 (Elm 実装)
-- S式構文を持つ式を解析して評価する

module MiniLisp exposing (..)

import Dict exposing (Dict)


-- 式の抽象構文木
type Expr
    = Number Float
    | Symbol String
    | List (List Expr)


-- 評価値
type Value
    = VNumber Float
    | VLambda
        { params : List String
        , body : Expr
        , env : Env
        }
    | VBuiltin (List Value -> Result String Value)


type alias Env =
    Dict String Value


-- パースエラー
type ParseError
    = UnexpectedToken String
    | UnmatchedParen
    | EmptyInput


-- トークン化: S式の括弧をスペースで区切る
tokenize : String -> List String
tokenize source =
    source
        |> String.replace "(" " ( "
        |> String.replace ")" " ) "
        |> String.words
        |> List.filter (not << String.isEmpty)


-- 式のパース
parseExpr : List String -> Result ParseError ( Expr, List String )
parseExpr tokens =
    case tokens of
        [] ->
            Err EmptyInput

        token :: rest ->
            parseToken token rest


parseToken : String -> List String -> Result ParseError ( Expr, List String )
parseToken token rest =
    if token == "(" then
        parseList rest []

    else if token == ")" then
        Err UnmatchedParen

    else
        case String.toFloat token of
            Just num ->
                Ok ( Number num, rest )

            Nothing ->
                Ok ( Symbol token, rest )


parseList : List String -> List Expr -> Result ParseError ( Expr, List String )
parseList tokens acc =
    case tokens of
        [] ->
            Err UnmatchedParen

        ")" :: rest ->
            Ok ( List (List.reverse acc), rest )

        token :: rest ->
            case parseToken token rest of
                Ok ( expr, next ) ->
                    parseList next (expr :: acc)

                Err err ->
                    Err err


-- 式の評価
evalExpr : Expr -> Env -> Result String Value
evalExpr expr env =
    case expr of
        Number n ->
            Ok (VNumber n)

        Symbol name ->
            Dict.get name env
                |> Result.fromMaybe ("未定義シンボル: " ++ name)

        List items ->
            evalList items env


evalList : List Expr -> Env -> Result String Value
evalList items env =
    case items of
        [] ->
            Err "空のリストは評価できません"

        head :: rest ->
            evalExpr head env
                |> Result.andThen
                    (\callee ->
                        evaluateArgs rest env
                            |> Result.andThen (apply callee)
                    )


evaluateArgs : List Expr -> Env -> Result String (List Value)
evaluateArgs exprs env =
    List.foldl
        (\expr acc ->
            acc
                |> Result.andThen
                    (\values ->
                        evalExpr expr env
                            |> Result.map (\value -> values ++ [ value ])
                    )
        )
        (Ok [])
        exprs


apply : Value -> List Value -> Result String Value
apply callee args =
    case callee of
        VBuiltin fn ->
            fn args

        VLambda { params, body, env } ->
            applyLambda params body env args

        VNumber _ ->
            Err "数値を関数として適用できません"


applyLambda : List String -> Expr -> Env -> List Value -> Result String Value
applyLambda params body lambdaEnv args =
    if List.length params /= List.length args then
        Err "引数の数が一致しません"

    else
        let
            newEnv =
                List.map2 Tuple.pair params args
                    |> Dict.fromList
                    |> Dict.union lambdaEnv
        in
        evalExpr body newEnv


-- 組み込み数値演算
builtinNumeric : (Float -> Float -> Float) -> List Value -> Result String Value
builtinNumeric op args =
    case args of
        [ VNumber lhs, VNumber rhs ] ->
            Ok (VNumber (op lhs rhs))

        _ ->
            Err "数値演算は2引数の数値のみ対応します"


-- デフォルト環境
defaultEnv : Env
defaultEnv =
    Dict.fromList
        [ ( "+", VBuiltin (builtinNumeric (+)) )
        , ( "-", VBuiltin (builtinNumeric (-)) )
        , ( "*", VBuiltin (builtinNumeric (*)) )
        , ( "/", VBuiltin (builtinNumeric (/)) )
        ]


-- メイン評価関数
eval : String -> Result String Value
eval source =
    let
        tokens =
            tokenize source
    in
    parseExpr tokens
        |> Result.mapError
            (\err ->
                case err of
                    EmptyInput ->
                        "入力が空です"

                    UnmatchedParen ->
                        "括弧が一致しません"

                    UnexpectedToken token ->
                        "予期しないトークン: " ++ token
            )
        |> Result.andThen
            (\( expr, rest ) ->
                if List.isEmpty rest then
                    evalExpr expr defaultEnv

                else
                    Err "末尾に未消費トークンがあります"
            )


-- 利用例
-- eval "(+ 40 2)" => Ok (VNumber 42.0)