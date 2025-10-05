-- JSON パーサー (Elm 実装)
-- JSON 構文を解析して汎用値型に変換する

module JsonParser exposing (..)

import Dict exposing (Dict)


-- JSON 値型
type JsonValue
    = JNull
    | JBool Bool
    | JNumber Float
    | JString String
    | JArray (List JsonValue)
    | JObject (Dict String JsonValue)


-- トークン型
type Token
    = LBrace
    | RBrace
    | LBracket
    | RBracket
    | Colon
    | Comma
    | StringLiteral String
    | NumberLiteral Float
    | BoolLiteral Bool
    | NullLiteral


-- パース状態
type alias ParseState =
    { tokens : List Token }


-- パースエラー
type ParseError
    = UnexpectedEOF
    | UnexpectedToken { expected : String, found : Token }


-- トークン化 (簡易実装)
tokenize : String -> List Token
tokenize source =
    tokenizeHelper 0 source []


tokenizeHelper : Int -> String -> List Token -> List Token
tokenizeHelper index source acc =
    if index >= String.length source then
        List.reverse acc

    else
        let
            ch =
                String.slice index (index + 1) source
        in
        case ch of
            " " ->
                tokenizeHelper (index + 1) source acc

            "\n" ->
                tokenizeHelper (index + 1) source acc

            "\t" ->
                tokenizeHelper (index + 1) source acc

            "\r" ->
                tokenizeHelper (index + 1) source acc

            "{" ->
                tokenizeHelper (index + 1) source (LBrace :: acc)

            "}" ->
                tokenizeHelper (index + 1) source (RBrace :: acc)

            "[" ->
                tokenizeHelper (index + 1) source (LBracket :: acc)

            "]" ->
                tokenizeHelper (index + 1) source (RBracket :: acc)

            ":" ->
                tokenizeHelper (index + 1) source (Colon :: acc)

            "," ->
                tokenizeHelper (index + 1) source (Comma :: acc)

            "t" ->
                if String.slice index (index + 4) source == "true" then
                    tokenizeHelper (index + 4) source (BoolLiteral True :: acc)

                else
                    tokenizeHelper (index + 1) source acc

            "f" ->
                if String.slice index (index + 5) source == "false" then
                    tokenizeHelper (index + 5) source (BoolLiteral False :: acc)

                else
                    tokenizeHelper (index + 1) source acc

            "n" ->
                if String.slice index (index + 4) source == "null" then
                    tokenizeHelper (index + 4) source (NullLiteral :: acc)

                else
                    tokenizeHelper (index + 1) source acc

            "\"" ->
                case String.indexes "\"" (String.dropLeft (index + 1) source) |> List.head of
                    Just endOffset ->
                        let
                            str =
                                String.slice (index + 1) (index + 1 + endOffset) source
                        in
                        tokenizeHelper (index + 2 + endOffset) source (StringLiteral str :: acc)

                    Nothing ->
                        tokenizeHelper (index + 1) source acc

            _ ->
                -- 数値の読み取り (簡易実装)
                let
                    numStr =
                        String.dropLeft index source
                            |> String.toList
                            |> List.takeWhile (\c -> Char.isDigit c || c == '.' || c == '-')
                            |> String.fromList
                in
                case String.toFloat numStr of
                    Just num ->
                        tokenizeHelper (index + String.length numStr) source (NumberLiteral num :: acc)

                    Nothing ->
                        tokenizeHelper (index + 1) source acc


-- 値のパース
parseValue : ParseState -> Result ParseError ( JsonValue, ParseState )
parseValue state =
    case state.tokens of
        [] ->
            Err UnexpectedEOF

        token :: rest ->
            case token of
                NullLiteral ->
                    Ok ( JNull, { tokens = rest } )

                BoolLiteral flag ->
                    Ok ( JBool flag, { tokens = rest } )

                NumberLiteral num ->
                    Ok ( JNumber num, { tokens = rest } )

                StringLiteral text ->
                    Ok ( JString text, { tokens = rest } )

                LBracket ->
                    parseArray { tokens = rest }

                LBrace ->
                    parseObject { tokens = rest }

                other ->
                    Err (UnexpectedToken { expected = "値", found = other })


-- 配列のパース
parseArray : ParseState -> Result ParseError ( JsonValue, ParseState )
parseArray state =
    case state.tokens of
        RBracket :: rest ->
            Ok ( JArray [], { tokens = rest } )

        _ ->
            parseArrayHelper state []


parseArrayHelper : ParseState -> List JsonValue -> Result ParseError ( JsonValue, ParseState )
parseArrayHelper current acc =
    parseValue current
        |> Result.andThen
            (\( value, next ) ->
                let
                    newAcc =
                        acc ++ [ value ]
                in
                case next.tokens of
                    Comma :: rest ->
                        parseArrayHelper { tokens = rest } newAcc

                    RBracket :: rest ->
                        Ok ( JArray newAcc, { tokens = rest } )

                    token :: _ ->
                        Err (UnexpectedToken { expected = "]", found = token })

                    [] ->
                        Err UnexpectedEOF
            )


-- オブジェクトのパース
parseObject : ParseState -> Result ParseError ( JsonValue, ParseState )
parseObject state =
    case state.tokens of
        RBrace :: rest ->
            Ok ( JObject Dict.empty, { tokens = rest } )

        _ ->
            parseObjectHelper state Dict.empty


parseObjectHelper : ParseState -> Dict String JsonValue -> Result ParseError ( JsonValue, ParseState )
parseObjectHelper current acc =
    case current.tokens of
        StringLiteral key :: Colon :: rest ->
            parseValue { tokens = rest }
                |> Result.andThen
                    (\( value, next ) ->
                        let
                            newAcc =
                                Dict.insert key value acc
                        in
                        case next.tokens of
                            Comma :: rest2 ->
                                parseObjectHelper { tokens = rest2 } newAcc

                            RBrace :: rest2 ->
                                Ok ( JObject newAcc, { tokens = rest2 } )

                            token :: _ ->
                                Err (UnexpectedToken { expected = "}", found = token })

                            [] ->
                                Err UnexpectedEOF
                    )

        token :: _ ->
            Err (UnexpectedToken { expected = "文字列", found = token })

        [] ->
            Err UnexpectedEOF


-- メインパース関数
parseJson : String -> Result String JsonValue
parseJson source =
    let
        tokens =
            tokenize source

        state =
            { tokens = tokens }
    in
    parseValue state
        |> Result.mapError
            (\err ->
                case err of
                    UnexpectedEOF ->
                        "予期しない入力終端"

                    UnexpectedToken { expected } ->
                        "期待: " ++ expected
            )
        |> Result.andThen
            (\( value, rest ) ->
                if List.isEmpty rest.tokens then
                    Ok value

                else
                    Err "末尾に未消費トークンがあります"
            )


-- 利用例
-- parseJson """{"name": "Alice", "age": 30}"""
-- => Ok (JObject (Dict.fromList [("name", JString "Alice"), ("age", JNumber 30.0)]))