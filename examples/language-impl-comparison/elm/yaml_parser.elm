module YamlParser exposing (YamlValue(..), parse, renderToString, testExamples)

{-| YAML風パーサー：インデント管理が重要な題材。

対応する構文（簡易版）：
- スカラー値: 文字列、数値、真偽値、null
- リスト: `- item1`
- マップ: `key: value`
- ネストしたインデント構造

Elmの特徴：
- 純粋関数型でパーサーを構築
- Result型によるエラー処理
- Maybe型によるオプショナルな値の表現

-}

import Dict exposing (Dict)
import String


-- YAML値型

type YamlValue
    = Scalar String
    | YamlList (List YamlValue)
    | YamlMap (Dict String YamlValue)
    | Null


type alias ParseResult a =
    Result String ( a, String )


-- 基本パーサー

hspace : String -> ParseResult ()
hspace input =
    let
        ( _, rest ) =
            spanWhile (\c -> c == ' ' || c == '\t') input
    in
    Ok ( (), rest )


spanWhile : (Char -> Bool) -> String -> ( String, String )
spanWhile predicate input =
    let
        chars =
            String.toList input

        taken =
            List.takeWhile predicate chars

        remaining =
            List.drop (List.length taken) chars
    in
    ( String.fromList taken, String.fromList remaining )


expectString : String -> String -> ParseResult String
expectString expected input =
    if String.startsWith expected input then
        Ok ( expected, String.dropLeft (String.length expected) input )

    else
        Err ("Expected '" ++ expected ++ "'")


newline : String -> ParseResult ()
newline input =
    if String.startsWith "\r\n" input then
        Ok ( (), String.dropLeft 2 input )

    else if String.startsWith "\n" input then
        Ok ( (), String.dropLeft 1 input )

    else if String.startsWith "\r" input then
        Ok ( (), String.dropLeft 1 input )

    else
        Err "Expected newline"


-- インデント検証

expectIndent : Int -> String -> ParseResult ()
expectIndent level input =
    let
        ( spaces, rest ) =
            takeSpaces input

        actual =
            String.length spaces
    in
    if actual == level then
        Ok ( (), rest )

    else
        Err ("インデント不一致: 期待 " ++ String.fromInt level ++ ", 実際 " ++ String.fromInt actual)


takeSpaces : String -> ( String, String )
takeSpaces input =
    spanWhile (\c -> c == ' ') input


-- スカラー値パーサー

parseScalar : String -> ParseResult YamlValue
parseScalar input =
    if String.startsWith "null" input then
        Ok ( Null, String.dropLeft 4 input )

    else if String.startsWith "~" input then
        Ok ( Null, String.dropLeft 1 input )

    else if String.startsWith "true" input then
        Ok ( Scalar "true", String.dropLeft 4 input )

    else if String.startsWith "false" input then
        Ok ( Scalar "false", String.dropLeft 5 input )

    else
        -- 文字列（引用符なし：行末まで）
        case String.split "\n" input of
            line :: rest ->
                let
                    trimmed =
                        String.trim line
                in
                if trimmed /= "" then
                    Ok ( Scalar trimmed, String.join "\n" rest )

                else
                    Err "Empty scalar"

            [] ->
                Err "Empty input"


-- リストパーサー

parseListItem : Int -> String -> ParseResult YamlValue
parseListItem indent input =
    expectIndent indent input
        |> Result.andThen
            (\( _, rest ) ->
                expectString "-" rest
                    |> Result.andThen
                        (\( _, rest2 ) ->
                            hspace rest2
                                |> Result.andThen
                                    (\( _, rest3 ) ->
                                        parseValue (indent + 2) rest3
                                    )
                        )
            )


parseList : Int -> String -> ParseResult YamlValue
parseList indent input =
    parseListItems indent input []


parseListItems : Int -> String -> List YamlValue -> ParseResult YamlValue
parseListItems indent input acc =
    case parseListItem indent input of
        Ok ( item, rest ) ->
            let
                rest2 =
                    skipOptionalNewline rest
            in
            parseListItems indent rest2 (item :: acc)

        Err _ ->
            if List.isEmpty acc then
                Err "Expected at least one list item"

            else
                Ok ( YamlList (List.reverse acc), input )


skipOptionalNewline : String -> String
skipOptionalNewline input =
    case newline input of
        Ok ( _, rest ) ->
            rest

        Err _ ->
            input


-- マップパーサー

parseMapEntry : Int -> String -> ParseResult ( String, YamlValue )
parseMapEntry indent input =
    expectIndent indent input
        |> Result.andThen
            (\( _, rest ) ->
                let
                    ( keyLine, rest2 ) =
                        takeUntil ":" rest

                    key =
                        String.trim keyLine
                in
                expectString ":" rest2
                    |> Result.andThen
                        (\( _, rest3 ) ->
                            hspace rest3
                                |> Result.andThen
                                    (\( _, rest4 ) ->
                                        -- 値が同じ行にあるか、次の行にネストされているか
                                        case parseValue indent rest4 of
                                            Ok ( value, rest5 ) ->
                                                Ok ( ( key, value ), rest5 )

                                            Err _ ->
                                                newline rest4
                                                    |> Result.andThen
                                                        (\( _, rest5 ) ->
                                                            parseValue (indent + 2) rest5
                                                                |> Result.map
                                                                    (\( value, rest6 ) ->
                                                                        ( ( key, value ), rest6 )
                                                                    )
                                                        )
                                    )
                        )
            )


takeUntil : String -> String -> ( String, String )
takeUntil delimiter input =
    case String.split delimiter input of
        before :: after ->
            ( before, String.join delimiter after )

        [] ->
            ( input, "" )


parseMap : Int -> String -> ParseResult YamlValue
parseMap indent input =
    parseMapEntries indent input []


parseMapEntries : Int -> String -> List ( String, YamlValue ) -> ParseResult YamlValue
parseMapEntries indent input acc =
    case parseMapEntry indent input of
        Ok ( ( key, value ), rest ) ->
            let
                rest2 =
                    skipOptionalNewline rest
            in
            parseMapEntries indent rest2 (( key, value ) :: acc)

        Err _ ->
            if List.isEmpty acc then
                Err "Expected at least one map entry"

            else
                Ok ( YamlMap (Dict.fromList (List.reverse acc)), input )


-- 値パーサー（再帰的）

parseValue : Int -> String -> ParseResult YamlValue
parseValue indent input =
    if String.contains "-" input then
        case parseList indent input of
            Ok result ->
                Ok result

            Err _ ->
                parseMapOrScalar indent input

    else
        parseMapOrScalar indent input


parseMapOrScalar : Int -> String -> ParseResult YamlValue
parseMapOrScalar indent input =
    case parseMap indent input of
        Ok result ->
            Ok result

        Err _ ->
            parseScalar input


-- ドキュメントパーサー

skipBlankLines : String -> String
skipBlankLines input =
    input
        |> String.lines
        |> List.dropWhile (\line -> String.trim line == "")
        |> String.join "\n"


parse : String -> Result String YamlValue
parse input =
    let
        input2 =
            skipBlankLines input
    in
    case parseValue 0 input2 of
        Ok ( doc, _ ) ->
            Ok doc

        Err msg ->
            Err msg


-- レンダリング（検証用）

renderToString : YamlValue -> String
renderToString doc =
    renderValue doc 0


renderValue : YamlValue -> Int -> String
renderValue value indent =
    let
        indentStr =
            String.repeat indent " "
    in
    case value of
        Scalar s ->
            s

        Null ->
            "null"

        YamlList items ->
            items
                |> List.map (\item -> indentStr ++ "- " ++ renderValue item (indent + 2))
                |> String.join "\n"

        YamlMap entries ->
            entries
                |> Dict.toList
                |> List.map
                    (\( key, val ) ->
                        case val of
                            Scalar _ ->
                                indentStr ++ key ++ ": " ++ renderValue val 0

                            Null ->
                                indentStr ++ key ++ ": " ++ renderValue val 0

                            _ ->
                                indentStr ++ key ++ ":\n" ++ renderValue val (indent + 2)
                    )
                |> String.join "\n"


-- テスト

testExamples : List ( String, String )
testExamples =
    let
        examples =
            [ ( "simple_scalar", "hello" )
            , ( "simple_list", "- item1\n- item2\n- item3" )
            , ( "simple_map", "key1: value1\nkey2: value2" )
            , ( "nested_map", "parent:\n  child1: value1\n  child2: value2" )
            , ( "nested_list", "items:\n  - item1\n  - item2" )
            , ( "mixed", "name: John\nage: 30\nhobbies:\n  - reading\n  - coding" )
            ]
    in
    List.map
        (\( name, yamlStr ) ->
            case parse yamlStr of
                Ok doc ->
                    ( name, "パース成功:\n" ++ renderToString doc )

                Err err ->
                    ( name, "パースエラー: " ++ err )
        )
        examples


{-| Elmの特徴：

1. **純粋関数型**
   - すべてのパーサーが純粋関数
   - 副作用なし、テストしやすい

2. **Result型とMaybe型**
   - エラー処理がResult型で統一
   - オプショナルな値はMaybe型

3. **型安全性**
   - コンパイル時に型チェック
   - ランタイムエラーが起きにくい

4. **課題**
   - 再帰的な定義がやや冗長
   - パーサーコンビネーターライブラリがないため手書き実装

Remlとの比較：
- Remlはパーサーコンビネーターライブラリで簡潔
- Elmは手書きだが型安全性が高い

-}
