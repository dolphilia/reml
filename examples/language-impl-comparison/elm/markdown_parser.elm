-- Markdown風軽量マークアップパーサー - Elm実装
--
-- Unicode処理の注意点：
-- - ElmのStringはJavaScriptの文字列（UTF-16ベース）
-- - String.lengthはUTF-16コードユニット数を返す（サロゲートペアに注意）
-- - Grapheme（書記素クラスター）単位の操作は標準ライブラリにない
-- - 絵文字や結合文字の正確な扱いには追加のパッケージが必要
-- - Remlの3層モデルと比較すると、Elmは明示的な区別がない

module MarkdownParser exposing
    ( Block(..)
    , Document
    , Inline(..)
    , parse
    , renderToString
    )


-- Markdown AST のブロック要素


type Block
    = Heading { level : Int, inline : List Inline }
    | Paragraph { inline : List Inline }
    | UnorderedList { items : List (List Inline) }
    | OrderedList { items : List (List Inline) }
    | CodeBlock { lang : Maybe String, code : String }
    | HorizontalRule


type Inline
    = Text String
    | Strong (List Inline)
    | Emphasis (List Inline)
    | Code String
    | Link { text : List Inline, url : String }
    | LineBreak


type alias Document =
    List Block



-- パーサー状態


type alias ParseState =
    { input : String
    , position : Int
    }


type alias ParseResult a =
    Result String ( a, ParseState )



-- パブリックAPI：文字列からドキュメントをパース


parse : String -> Result String Document
parse input =
    let
        initialState =
            { input = input, position = 0 }
    in
    parseDocument initialState []


parseDocument : ParseState -> List Block -> Result String Document
parseDocument state blocks =
    case parseBlock state of
        Ok ( block, newState ) ->
            parseDocument newState (block :: blocks)

        Err "EOF" ->
            Ok (List.reverse blocks)

        Err msg ->
            Err msg



-- ブロック要素のパース（優先順位付き試行）


parseBlock : ParseState -> ParseResult Block
parseBlock state =
    let
        state1 =
            skipBlankLines state
    in
    if isEof state1 then
        Err "EOF"

    else
        let
            state2 =
                skipHSpace state1
        in
        case peekChar state2 of
            Just '#' ->
                parseHeading state2

            Just '`' ->
                case matchString state2 "```" of
                    Just _ ->
                        parseCodeBlock state2

                    Nothing ->
                        parseParagraph state2

            Just c ->
                if c == '-' || c == '*' || c == '_' then
                    case parseHorizontalRule state2 of
                        Ok result ->
                            Ok result

                        Err _ ->
                            parseUnorderedList state2

                else
                    parseParagraph state2

            Nothing ->
                Err "EOF"



-- 見出し行のパース（`# Heading` 形式）


parseHeading : ParseState -> ParseResult Block
parseHeading state =
    let
        state1 =
            skipHSpace state

        ( level, state2 ) =
            countHashes state1 0
    in
    if level == 0 || level > 6 then
        Err "見出しレベルは1-6の範囲内である必要があります"

    else
        let
            state3 =
                skipHSpace state2

            ( text, state4 ) =
                readUntilEol state3

            state5 =
                consumeNewline state4

            inline =
                [ Text (String.trim text) ]
        in
        Ok ( Heading { level = level, inline = inline }, state5 )


countHashes : ParseState -> Int -> ( Int, ParseState )
countHashes state n =
    case peekChar state of
        Just '#' ->
            countHashes (advanceChar state) (n + 1)

        _ ->
            ( n, state )



-- 水平線のパース（`---`, `***`, `___`）


parseHorizontalRule : ParseState -> ParseResult Block
parseHorizontalRule state =
    let
        state1 =
            skipHSpace state

        ( text, state2 ) =
            readUntilEol state1

        state3 =
            consumeNewline state2

        trimmed =
            String.trim text

        isRule =
            (String.all (\c -> c == '-') trimmed && String.length trimmed >= 3)
                || (String.all (\c -> c == '*') trimmed && String.length trimmed >= 3)
                || (String.all (\c -> c == '_') trimmed && String.length trimmed >= 3)
    in
    if isRule then
        Ok ( HorizontalRule, state3 )

    else
        Err "水平線として認識できません"



-- コードブロックのパース（```言語名）


parseCodeBlock : ParseState -> ParseResult Block
parseCodeBlock state =
    case matchString state "```" of
        Nothing ->
            Err "コードブロック開始が見つかりません"

        Just state1 ->
            let
                ( langLine, state2 ) =
                    readUntilEol state1

                state3 =
                    consumeNewline state2

                lang =
                    let
                        trimmed =
                            String.trim langLine
                    in
                    if String.isEmpty trimmed then
                        Nothing

                    else
                        Just trimmed

                ( codeLines, state4 ) =
                    readCodeLines state3 []

                state5 =
                    consumeNewline state4

                code =
                    String.join "\n" codeLines
            in
            Ok ( CodeBlock { lang = lang, code = code }, state5 )


readCodeLines : ParseState -> List String -> ( List String, ParseState )
readCodeLines state acc =
    case matchString state "```" of
        Just endState ->
            ( List.reverse acc, endState )

        Nothing ->
            if isEof state then
                ( List.reverse acc, state )

            else
                let
                    ( line, state2 ) =
                        readUntilEol state

                    state3 =
                        consumeNewline state2
                in
                readCodeLines state3 (line :: acc)



-- リスト項目のパース（簡易版：`-` または `*`）


parseUnorderedList : ParseState -> ParseResult Block
parseUnorderedList state =
    let
        ( items, stateEnd ) =
            parseListItems state []
    in
    if List.isEmpty items then
        Err "リスト項目が見つかりません"

    else
        Ok ( UnorderedList { items = List.reverse items }, stateEnd )


parseListItems : ParseState -> List (List Inline) -> ( List (List Inline), ParseState )
parseListItems state acc =
    let
        state1 =
            skipHSpace state
    in
    case peekChar state1 of
        Just c ->
            if c == '-' || c == '*' then
                let
                    state2 =
                        advanceChar state1

                    state3 =
                        skipHSpace state2

                    ( text, state4 ) =
                        readUntilEol state3

                    state5 =
                        consumeNewline state4

                    inline =
                        [ Text (String.trim text) ]
                in
                parseListItems state5 (inline :: acc)

            else
                ( acc, state )

        Nothing ->
            ( acc, state )



-- 段落のパース（簡易版：空行まで）


parseParagraph : ParseState -> ParseResult Block
parseParagraph state =
    let
        ( lines, stateEnd ) =
            readParagraphLines state []

        text =
            String.join " " lines |> String.trim

        inline =
            [ Text text ]
    in
    Ok ( Paragraph { inline = inline }, stateEnd )


readParagraphLines : ParseState -> List String -> ( List String, ParseState )
readParagraphLines state acc =
    if isEof state then
        ( List.reverse acc, state )

    else
        case peekChar state of
            Just '\n' ->
                let
                    state1 =
                        advanceChar state
                in
                case peekChar state1 of
                    Just '\n' ->
                        ( List.reverse acc, state1 )

                    _ ->
                        readParagraphLines state1 ("" :: acc)

            Just _ ->
                let
                    ( line, state2 ) =
                        readUntilEol state

                    state3 =
                        consumeNewline state2
                in
                readParagraphLines state3 (line :: acc)

            Nothing ->
                ( List.reverse acc, state )



-- パーサーユーティリティ


peekChar : ParseState -> Maybe Char
peekChar { input, position } =
    if position >= String.length input then
        Nothing

    else
        String.slice position (position + 1) input
            |> String.uncons
            |> Maybe.map Tuple.first


advanceChar : ParseState -> ParseState
advanceChar state =
    { state | position = state.position + 1 }


matchString : ParseState -> String -> Maybe ParseState
matchString state target =
    let
        remaining =
            String.dropLeft state.position state.input
    in
    if String.startsWith target remaining then
        Just { state | position = state.position + String.length target }

    else
        Nothing


skipHSpace : ParseState -> ParseState
skipHSpace state =
    case peekChar state of
        Just c ->
            if c == ' ' || c == '\t' then
                skipHSpace (advanceChar state)

            else
                state

        Nothing ->
            state


skipBlankLines : ParseState -> ParseState
skipBlankLines state =
    case peekChar state of
        Just '\n' ->
            skipBlankLines (advanceChar state)

        _ ->
            state


readUntilEol : ParseState -> ( String, ParseState )
readUntilEol state =
    let
        remaining =
            String.dropLeft state.position state.input

        beforeNewline =
            case String.indexes "\n" remaining |> List.head of
                Just idx ->
                    String.left idx remaining

                Nothing ->
                    remaining

        newPosition =
            state.position + String.length beforeNewline
    in
    ( beforeNewline, { state | position = newPosition } )


consumeNewline : ParseState -> ParseState
consumeNewline state =
    case peekChar state of
        Just '\n' ->
            advanceChar state

        _ ->
            state


isEof : ParseState -> Bool
isEof { input, position } =
    position >= String.length input



-- 簡易的なレンダリング（検証用）


renderToString : Document -> String
renderToString doc =
    doc
        |> List.map renderBlock
        |> String.concat


renderInline : List Inline -> String
renderInline inlines =
    inlines
        |> List.map
            (\i ->
                case i of
                    Text s ->
                        s

                    Strong inner ->
                        "**" ++ renderInline inner ++ "**"

                    Emphasis inner ->
                        "*" ++ renderInline inner ++ "*"

                    Code s ->
                        "`" ++ s ++ "`"

                    Link { text, url } ->
                        "[" ++ renderInline text ++ "](" ++ url ++ ")"

                    LineBreak ->
                        "\n"
            )
        |> String.concat


renderBlock : Block -> String
renderBlock block =
    case block of
        Heading { level, inline } ->
            let
                prefix =
                    String.repeat level "#"
            in
            prefix ++ " " ++ renderInline inline ++ "\n\n"

        Paragraph { inline } ->
            renderInline inline ++ "\n\n"

        UnorderedList { items } ->
            let
                itemsStr =
                    items
                        |> List.map (\item -> "- " ++ renderInline item ++ "\n")
                        |> String.concat
            in
            itemsStr ++ "\n"

        OrderedList { items } ->
            let
                itemsStr =
                    items
                        |> List.indexedMap (\i item -> String.fromInt (i + 1) ++ ". " ++ renderInline item ++ "\n")
                        |> String.concat
            in
            itemsStr ++ "\n"

        CodeBlock { lang, code } ->
            let
                langStr =
                    Maybe.withDefault "" lang
            in
            "```" ++ langStr ++ "\n" ++ code ++ "\n```\n\n"

        HorizontalRule ->
            "---\n\n"



-- Unicode 3層モデル比較：
--
-- ElmのStringはJavaScriptの文字列（UTF-16ベース）なので：
-- - String.length はUTF-16コードユニット数を返す
-- - サロゲートペア（例：絵文字「😀」）は2カウントされる
-- - Grapheme（書記素クラスター）単位の操作は標準にない
--
-- 例：
-- str = "🇯🇵"  -- 国旗絵文字（2つのコードポイント、1つのgrapheme）
-- String.length str  -- => 4 (UTF-16コードユニット数)
--
-- Remlの3層モデル（Byte/Char/Grapheme）と比較すると、
-- Elmは明示的な区別がなく、絵文字や結合文字の扱いでバグが発生しやすい。