module TomlParser exposing (TomlValue(..), TomlDocument, parse, renderToString, testExamples)

{-| TOML風設定ファイルパーサー：キーバリューペアとテーブルを扱う題材。

対応する構文（TOML v1.0.0準拠の簡易版）：
- キーバリューペア: `key = "value"`
- テーブル: `[section]`
- 配列テーブル: `[[array_section]]`
- データ型: 文字列、整数、浮動小数点、真偽値、配列、インラインテーブル
- コメント: `# comment`

Elmの特徴：
- 純粋関数型でパーサーを構築
- Result型によるエラー処理
- Dict型によるテーブル表現

-}

import Dict exposing (Dict)
import String


-- TOML値型

type TomlValue
    = TomlString String
    | TomlInteger Int
    | TomlFloat Float
    | TomlBoolean Bool
    | TomlArray (List TomlValue)
    | TomlInlineTable (Dict String TomlValue)


type alias TomlTable =
    Dict String TomlValue


type alias TomlDocument =
    { root : TomlTable
    , tables : Dict (List String) TomlTable
    }


type alias ParseResult a =
    Result String ( a, String )


-- 基本パーサー

skipWhitespace : String -> String
skipWhitespace input =
    String.trimLeft input


skipComment : String -> String
skipComment input =
    if String.startsWith "#" input then
        case String.split "\n" input of
            _ :: rest ->
                String.join "\n" rest

            _ ->
                ""

    else
        input


skipWhitespaceAndComments : String -> String
skipWhitespaceAndComments input =
    let
        rest =
            input
                |> skipWhitespace
                |> skipComment
    in
    if rest /= input then
        skipWhitespaceAndComments rest

    else
        rest


expectString : String -> String -> ParseResult String
expectString expected input =
    if String.startsWith expected input then
        Ok ( expected, String.dropLeft (String.length expected) input )

    else
        Err ("Expected '" ++ expected ++ "'")


-- キー名のパース

isBareKeyChar : Char -> Bool
isBareKeyChar c =
    Char.isAlphaNum c || c == '-' || c == '_'


parseBareKey : String -> ParseResult String
parseBareKey input =
    let
        ( key, rest ) =
            spanWhile isBareKeyChar input
    in
    if String.isEmpty key then
        Err "Expected key"

    else
        Ok ( key, rest )


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


takeUntilUnescaped : String -> String -> ( String, String )
takeUntilUnescaped delimiter input =
    takeUntilUnescapedImpl delimiter input ""


takeUntilUnescapedImpl : String -> String -> String -> ( String, String )
takeUntilUnescapedImpl delimiter input acc =
    if String.startsWith ("\\" ++ delimiter) input then
        takeUntilUnescapedImpl delimiter
            (String.dropLeft 2 input)
            (acc ++ delimiter)

    else if String.startsWith delimiter input then
        ( acc, input )

    else if String.isEmpty input then
        ( acc, "" )

    else
        takeUntilUnescapedImpl delimiter
            (String.dropLeft 1 input)
            (acc ++ String.left 1 input)


parseQuotedKey : String -> ParseResult String
parseQuotedKey input =
    expectString "\"" input
        |> Result.andThen
            (\( _, rest ) ->
                let
                    ( key, rest2 ) =
                        takeUntilUnescaped "\"" rest
                in
                expectString "\"" rest2
                    |> Result.map (\( _, rest3 ) -> ( key, rest3 ))
            )


parseKey : String -> ParseResult String
parseKey input =
    if String.startsWith "\"" input then
        parseQuotedKey input

    else
        parseBareKey input


parseKeyPath : String -> ParseResult (List String)
parseKeyPath input =
    parseKeyPathImpl input []


parseKeyPathImpl : String -> List String -> ParseResult (List String)
parseKeyPathImpl input acc =
    let
        rest =
            skipWhitespace input
    in
    parseKey rest
        |> Result.andThen
            (\( key, rest2 ) ->
                let
                    rest3 =
                        skipWhitespace rest2
                in
                if String.startsWith "." rest3 then
                    parseKeyPathImpl (String.dropLeft 1 rest3) (acc ++ [ key ])

                else
                    Ok ( acc ++ [ key ], rest3 )
            )


-- 値のパース

parseStringValue : String -> ParseResult TomlValue
parseStringValue input =
    if String.startsWith "\"\"\"" input then
        -- 複数行基本文字列
        expectString "\"\"\"" input
            |> Result.andThen
                (\( _, rest ) ->
                    let
                        ( content, rest2 ) =
                            takeUntil "\"\"\"" rest
                    in
                    expectString "\"\"\"" rest2
                        |> Result.map (\( _, rest3 ) -> ( TomlString content, rest3 ))
                )

    else if String.startsWith "'''" input then
        -- 複数行リテラル文字列
        expectString "'''" input
            |> Result.andThen
                (\( _, rest ) ->
                    let
                        ( content, rest2 ) =
                            takeUntil "'''" rest
                    in
                    expectString "'''" rest2
                        |> Result.map (\( _, rest3 ) -> ( TomlString content, rest3 ))
                )

    else if String.startsWith "'" input then
        -- リテラル文字列
        expectString "'" input
            |> Result.andThen
                (\( _, rest ) ->
                    let
                        ( content, rest2 ) =
                            takeUntil "'" rest
                    in
                    expectString "'" rest2
                        |> Result.map (\( _, rest3 ) -> ( TomlString content, rest3 ))
                )

    else if String.startsWith "\"" input then
        -- 基本文字列
        expectString "\"" input
            |> Result.andThen
                (\( _, rest ) ->
                    let
                        ( content, rest2 ) =
                            takeUntilUnescaped "\"" rest
                    in
                    expectString "\"" rest2
                        |> Result.map (\( _, rest3 ) -> ( TomlString content, rest3 ))
                )

    else
        Err "Expected string"


takeUntil : String -> String -> ( String, String )
takeUntil delimiter input =
    case String.split delimiter input of
        before :: _ ->
            ( before, String.dropLeft (String.length before) input )

        _ ->
            ( input, "" )


parseIntegerValue : String -> ParseResult TomlValue
parseIntegerValue input =
    let
        hasSign =
            String.startsWith "-" input

        sign =
            if hasSign then
                "-"

            else
                ""

        rest =
            if hasSign then
                String.dropLeft 1 input

            else
                input

        ( digits, rest2 ) =
            spanWhile (\c -> Char.isDigit c || c == '_') rest

        cleanDigits =
            String.filter (\c -> c /= '_') digits
    in
    if String.isEmpty cleanDigits then
        Err "Expected integer"

    else
        case String.toInt (sign ++ cleanDigits) of
            Just n ->
                Ok ( TomlInteger n, rest2 )

            Nothing ->
                Err "Invalid integer"


parseFloatValue : String -> ParseResult TomlValue
parseFloatValue input =
    let
        hasSign =
            String.startsWith "-" input

        sign =
            if hasSign then
                "-"

            else
                ""

        rest =
            if hasSign then
                String.dropLeft 1 input

            else
                input

        ( numStr, rest2 ) =
            spanWhile (\c -> Char.isDigit c || c == '.' || c == '_' || c == 'e' || c == 'E' || c == '+' || c == '-') rest

        cleanNumStr =
            String.filter (\c -> c /= '_') numStr
    in
    if String.isEmpty cleanNumStr || not (String.contains "." cleanNumStr) then
        Err "Expected float"

    else
        case String.toFloat (sign ++ cleanNumStr) of
            Just f ->
                Ok ( TomlFloat f, rest2 )

            Nothing ->
                Err "Invalid float"


parseBooleanValue : String -> ParseResult TomlValue
parseBooleanValue input =
    if String.startsWith "true" input then
        Ok ( TomlBoolean True, String.dropLeft 4 input )

    else if String.startsWith "false" input then
        Ok ( TomlBoolean False, String.dropLeft 5 input )

    else
        Err "Expected boolean"


parseArrayValue : String -> ParseResult TomlValue
parseArrayValue input =
    expectString "[" input
        |> Result.andThen
            (\( _, rest ) ->
                let
                    rest2 =
                        skipWhitespaceAndComments rest
                in
                parseArrayElements rest2 []
            )


parseArrayElements : String -> List TomlValue -> ParseResult TomlValue
parseArrayElements input acc =
    let
        rest =
            skipWhitespaceAndComments input
    in
    if String.startsWith "]" rest then
        Ok ( TomlArray (List.reverse acc), String.dropLeft 1 rest )

    else if List.isEmpty acc || String.startsWith "," rest then
        let
            rest2 =
                if not (List.isEmpty acc) then
                    String.dropLeft 1 rest

                else
                    rest

            rest3 =
                skipWhitespaceAndComments rest2
        in
        if String.startsWith "]" rest3 then
            Ok ( TomlArray (List.reverse acc), String.dropLeft 1 rest3 )

        else
            parseValue rest3
                |> Result.andThen
                    (\( value, rest4 ) ->
                        parseArrayElements rest4 (value :: acc)
                    )

    else
        Err "Expected ',' or ']'"


parseInlineTable : String -> ParseResult TomlValue
parseInlineTable input =
    expectString "{" input
        |> Result.andThen
            (\( _, rest ) ->
                let
                    rest2 =
                        skipWhitespaceAndComments rest
                in
                parseInlineTableEntries rest2 []
            )


parseInlineTableEntries : String -> List ( String, TomlValue ) -> ParseResult TomlValue
parseInlineTableEntries input acc =
    let
        rest =
            skipWhitespaceAndComments input
    in
    if String.startsWith "}" rest then
        Ok ( TomlInlineTable (Dict.fromList (List.reverse acc)), String.dropLeft 1 rest )

    else if List.isEmpty acc || String.startsWith "," rest then
        let
            rest2 =
                if not (List.isEmpty acc) then
                    String.dropLeft 1 rest

                else
                    rest

            rest3 =
                skipWhitespaceAndComments rest2
        in
        if String.startsWith "}" rest3 then
            Ok ( TomlInlineTable (Dict.fromList (List.reverse acc)), String.dropLeft 1 rest3 )

        else
            parseKey rest3
                |> Result.andThen
                    (\( key, rest4 ) ->
                        let
                            rest5 =
                                skipWhitespace rest4
                        in
                        expectString "=" rest5
                            |> Result.andThen
                                (\( _, rest6 ) ->
                                    let
                                        rest7 =
                                            skipWhitespaceAndComments rest6
                                    in
                                    parseValue rest7
                                        |> Result.andThen
                                            (\( value, rest8 ) ->
                                                parseInlineTableEntries rest8 (( key, value ) :: acc)
                                            )
                                )
                    )

    else
        Err "Expected ',' or '}'"


parseValue : String -> ParseResult TomlValue
parseValue input =
    let
        rest =
            skipWhitespaceAndComments input
    in
    if String.startsWith "\"" rest || String.startsWith "'" rest then
        parseStringValue rest

    else if String.startsWith "true" rest || String.startsWith "false" rest then
        parseBooleanValue rest

    else if String.startsWith "[" rest then
        parseArrayValue rest

    else if String.startsWith "{" rest then
        parseInlineTable rest

    else
        -- 数値（浮動小数点または整数）
        case parseFloatValue rest of
            Ok result ->
                Ok result

            Err _ ->
                parseIntegerValue rest


-- キーバリューペアのパース

type DocumentElement
    = KeyValue (List String) TomlValue
    | Table (List String)
    | ArrayTable (List String)


parseKeyValuePair : String -> ParseResult DocumentElement
parseKeyValuePair input =
    let
        rest =
            skipWhitespaceAndComments input
    in
    parseKeyPath rest
        |> Result.andThen
            (\( path, rest2 ) ->
                let
                    rest3 =
                        skipWhitespace rest2
                in
                expectString "=" rest3
                    |> Result.andThen
                        (\( _, rest4 ) ->
                            let
                                rest5 =
                                    skipWhitespaceAndComments rest4
                            in
                            parseValue rest5
                                |> Result.map (\( value, rest6 ) -> ( KeyValue path value, rest6 ))
                        )
            )


-- テーブルヘッダーのパース

parseTableHeader : String -> ParseResult DocumentElement
parseTableHeader input =
    let
        rest =
            skipWhitespaceAndComments input
    in
    if String.startsWith "[[" rest then
        expectString "[[" rest
            |> Result.andThen
                (\( _, rest2 ) ->
                    let
                        rest3 =
                            skipWhitespace rest2
                    in
                    parseKeyPath rest3
                        |> Result.andThen
                            (\( path, rest4 ) ->
                                let
                                    rest5 =
                                        skipWhitespace rest4
                                in
                                expectString "]]" rest5
                                    |> Result.map (\( _, rest6 ) -> ( ArrayTable path, rest6 ))
                            )
                )

    else if String.startsWith "[" rest then
        expectString "[" rest
            |> Result.andThen
                (\( _, rest2 ) ->
                    let
                        rest3 =
                            skipWhitespace rest2
                    in
                    parseKeyPath rest3
                        |> Result.andThen
                            (\( path, rest4 ) ->
                                let
                                    rest5 =
                                        skipWhitespace rest4
                                in
                                expectString "]" rest5
                                    |> Result.map (\( _, rest6 ) -> ( Table path, rest6 ))
                            )
                )

    else
        Err "Expected table header"


-- ドキュメント要素のパース

parseDocumentElement : String -> ParseResult DocumentElement
parseDocumentElement input =
    let
        rest =
            skipWhitespaceAndComments input
    in
    if String.isEmpty rest then
        Err "End of input"

    else if String.startsWith "[" rest then
        parseTableHeader rest

    else
        parseKeyValuePair rest


skipNewline : String -> String
skipNewline input =
    if String.startsWith "\r\n" input then
        String.dropLeft 2 input

    else if String.startsWith "\n" input then
        String.dropLeft 1 input

    else if String.startsWith "\r" input then
        String.dropLeft 1 input

    else
        input


parseDocumentElements : String -> List DocumentElement -> Result String (List DocumentElement)
parseDocumentElements input acc =
    let
        rest =
            skipWhitespaceAndComments input
    in
    if String.isEmpty rest then
        Ok (List.reverse acc)

    else
        case parseDocumentElement rest of
            Ok ( elem, rest2 ) ->
                let
                    rest3 =
                        skipNewline rest2
                in
                parseDocumentElements rest3 (elem :: acc)

            Err "End of input" ->
                Ok (List.reverse acc)

            Err msg ->
                Err msg


-- ドキュメント構築

insertNested : TomlTable -> List String -> TomlValue -> TomlTable
insertNested table path value =
    case path of
        [] ->
            table

        [ key ] ->
            Dict.insert key value table

        key :: rest ->
            let
                nested =
                    case Dict.get key table of
                        Just (TomlInlineTable t) ->
                            t

                        _ ->
                            Dict.empty

                updatedNested =
                    insertNested nested rest value
            in
            Dict.insert key (TomlInlineTable updatedNested) table


type alias BuildState =
    { currentTable : List String
    , root : TomlTable
    , tables : Dict (List String) TomlTable
    }


buildDocument : List DocumentElement -> BuildState
buildDocument elements =
    List.foldl buildDocumentStep
        { currentTable = []
        , root = Dict.empty
        , tables = Dict.empty
        }
        elements


buildDocumentStep : DocumentElement -> BuildState -> BuildState
buildDocumentStep elem state =
    case elem of
        Table path ->
            let
                newTables =
                    if not (Dict.member path state.tables) then
                        Dict.insert path Dict.empty state.tables

                    else
                        state.tables
            in
            { state | currentTable = path, tables = newTables }

        ArrayTable path ->
            let
                newTables =
                    if not (Dict.member path state.tables) then
                        Dict.insert path Dict.empty state.tables

                    else
                        state.tables
            in
            { state | currentTable = path, tables = newTables }

        KeyValue path value ->
            if List.isEmpty state.currentTable then
                -- ルートテーブルに追加
                { state | root = insertNested state.root path value }

            else
                -- 現在のテーブルに追加
                let
                    table =
                        Dict.get state.currentTable state.tables
                            |> Maybe.withDefault Dict.empty

                    updatedTable =
                        insertNested table path value

                    newTables =
                        Dict.insert state.currentTable updatedTable state.tables
                in
                { state | tables = newTables }


-- パブリックAPI

parse : String -> Result String TomlDocument
parse input =
    parseDocumentElements input []
        |> Result.map
            (\elements ->
                let
                    finalState =
                        buildDocument elements
                in
                { root = finalState.root
                , tables = finalState.tables
                }
            )


-- レンダリング（検証用）

renderToString : TomlDocument -> String
renderToString doc =
    let
        rootOutput =
            renderTable doc.root []

        tableOutput =
            Dict.toList doc.tables
                |> List.map
                    (\( path, table ) ->
                        "\n[" ++ String.join "." path ++ "]\n" ++ renderTable table []
                    )
                |> String.join ""
    in
    rootOutput ++ tableOutput


renderTable : TomlTable -> List String -> String
renderTable table prefix =
    Dict.toList table
        |> List.map
            (\( key, value ) ->
                let
                    fullKey =
                        if List.isEmpty prefix then
                            key

                        else
                            String.join "." (prefix ++ [ key ])
                in
                case value of
                    TomlInlineTable nested ->
                        renderTable nested (prefix ++ [ key ])

                    _ ->
                        fullKey ++ " = " ++ renderValue value ++ "\n"
            )
        |> String.join ""


renderValue : TomlValue -> String
renderValue value =
    case value of
        TomlString s ->
            "\"" ++ s ++ "\""

        TomlInteger n ->
            String.fromInt n

        TomlFloat f ->
            String.fromFloat f

        TomlBoolean True ->
            "true"

        TomlBoolean False ->
            "false"

        TomlArray items ->
            let
                itemsStr =
                    List.map renderValue items
                        |> String.join ", "
            in
            "[" ++ itemsStr ++ "]"

        TomlInlineTable entries ->
            let
                entriesStr =
                    Dict.toList entries
                        |> List.map (\( k, v ) -> k ++ " = " ++ renderValue v)
                        |> String.join ", "
            in
            "{ " ++ entriesStr ++ " }"


-- テスト

testExamples : String
testExamples =
    let
        exampleToml =
            """# Reml パッケージ設定

[package]
name = "my_project"
version = "0.1.0"
authors = ["Author Name"]

[dependencies]
core = "1.0"

[dev-dependencies]
test_framework = "0.5"

[[plugins]]
name = "system"
version = "1.0"

[[plugins]]
name = "memory"
version = "1.0"
"""
    in
    case parse exampleToml of
        Ok doc ->
            "パース成功:\n" ++ renderToString doc

        Err err ->
            "パースエラー: " ++ err


{-
Elmの特徴：

1. **純粋関数型アプローチ**
   - すべてのパーサーが純粋関数
   - 副作用なしで状態を扱う

2. **Result型によるエラー処理**
   - エラー処理がResult.andThenで連鎖
   - 型安全なエラーハンドリング

3. **Dict型によるテーブル表現**
   - ネストしたテーブルはDictで表現
   - イミュータブルなデータ構造

4. **課題**
   - 手書きパーサーのため、エラーメッセージの質が実装依存
   - バックトラックの実装が煩雑

Remlとの比較：
- Remlはパーサーコンビネーターライブラリによる高レベル抽象化
- Elmは手書きパーサーでより明示的な制御が必要
- Remlのcut/commitによるエラー位置特定がより正確
-}