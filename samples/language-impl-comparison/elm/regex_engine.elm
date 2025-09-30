module RegexEngine exposing (main)

import Browser
import Html exposing (Html, div, pre, text)

-- 正規表現エンジン：パース + 評価の両方を実装。
--
-- 対応する正規表現構文（簡易版）：
-- - リテラル: `abc`
-- - 連結: `ab`
-- - 選択: `a|b`
-- - 繰り返し: `a*`, `a+`, `a?`
-- - グループ: `(abc)`
-- - 文字クラス: `[a-z]`, `[^0-9]`, `\d`, `\w`, `\s`
-- - アンカー: `^`, `$`
-- - ドット: `.` (任意の1文字)

-- 正規表現のAST
type Regex
    = Literal String
    | CharClass CharSet
    | Dot
    | Concat (List Regex)
    | Alternation (List Regex)
    | Repeat Regex RepeatKind
    | Group Regex
    | Anchor AnchorKind

type CharSet
    = CharRange Char Char
    | CharList (List Char)
    | Predefined PredefinedClass
    | Negated CharSet
    | Union (List CharSet)

type PredefinedClass
    = Digit
    | Word
    | Whitespace
    | NotDigit
    | NotWord
    | NotWhitespace

type RepeatKind
    = ZeroOrMore
    | OneOrMore
    | ZeroOrOne

type AnchorKind
    = Start
    | End

-- パーサー型
type alias Parser a =
    String -> Result String ( a, String )

-- パーサーコンビネーター
ok : a -> Parser a
ok value input =
    Ok ( value, input )

fail : String -> Parser a
fail message _ =
    Err message

andThen : (a -> Parser b) -> Parser a -> Parser b
andThen f parser input =
    case parser input of
        Ok ( value, rest ) ->
            f value rest

        Err err ->
            Err err

map : (a -> b) -> Parser a -> Parser b
map f parser =
    andThen (\value -> ok (f value)) parser

choice : List (Parser a) -> Parser a
choice parsers input =
    case parsers of
        [] ->
            Err "no choice matched"

        p :: ps ->
            case p input of
                Ok result ->
                    Ok result

                Err _ ->
                    choice ps input

many : Parser a -> Parser (List a)
many parser input =
    case parser input of
        Ok ( value, rest ) ->
            case many parser rest of
                Ok ( values, finalRest ) ->
                    Ok ( value :: values, finalRest )

                Err _ ->
                    Ok ( [ value ], rest )

        Err _ ->
            Ok ( [], input )

many1 : Parser a -> Parser (List a)
many1 parser =
    andThen
        (\first ->
            andThen
                (\rest -> ok (first :: rest))
                (many parser)
        )
        parser

optional : Parser a -> Parser (Maybe a)
optional parser input =
    case parser input of
        Ok ( value, rest ) ->
            Ok ( Just value, rest )

        Err _ ->
            Ok ( Nothing, input )

char : Char -> Parser Char
char c input =
    case String.uncons input of
        Just ( ch, rest ) ->
            if ch == c then
                Ok ( ch, rest )

            else
                Err ("expected " ++ String.fromChar c)

        Nothing ->
            Err "end of input"

string : String -> Parser String
string s input =
    if String.startsWith s input then
        Ok ( s, String.dropLeft (String.length s) input )

    else
        Err ("expected " ++ s)

satisfy : (Char -> Bool) -> Parser Char
satisfy pred input =
    case String.uncons input of
        Just ( ch, rest ) ->
            if pred ch then
                Ok ( ch, rest )

            else
                Err "predicate failed"

        Nothing ->
            Err "end of input"

digit : Parser Char
digit =
    satisfy (\c -> c >= '0' && c <= '9')

sepBy1 : Parser a -> Parser sep -> Parser (List a)
sepBy1 parser sep =
    andThen
        (\first ->
            andThen
                (\rest -> ok (first :: rest))
                (many (andThen (\_ -> parser) sep))
        )
        parser

-- 正規表現パーサー
parseRegex : String -> Result String Regex
parseRegex input =
    case regexExpr input of
        Ok ( regex, "" ) ->
            Ok regex

        Ok ( _, rest ) ->
            Err ("unexpected input: " ++ rest)

        Err err ->
            Err err

regexExpr : Parser Regex
regexExpr =
    alternationExpr

alternationExpr : Parser Regex
alternationExpr =
    map
        (\alts ->
            case alts of
                [ single ] ->
                    single

                _ ->
                    Alternation alts
        )
        (sepBy1 concatExpr (string "|"))

concatExpr : Parser Regex
concatExpr =
    map
        (\terms ->
            case terms of
                [ single ] ->
                    single

                _ ->
                    Concat terms
        )
        (many1 postfixTerm)

postfixTerm : Parser Regex
postfixTerm =
    andThen
        (\base ->
            map
                (\repeatOpt ->
                    case repeatOpt of
                        Just kind ->
                            Repeat base kind

                        Nothing ->
                            base
                )
                (optional repeatSuffix)
        )
        atom

atom : Parser Regex
atom =
    choice
        [ -- 括弧グループ
          andThen
            (\_ ->
                andThen
                    (\inner ->
                        andThen
                            (\_ -> ok (Group inner))
                            (string ")")
                    )
                    regexExpr
            )
            (string "(")

        -- アンカー
        , map (\_ -> Anchor Start) (string "^")
        , map (\_ -> Anchor End) (string "$")

        -- ドット
        , map (\_ -> Dot) (string ".")

        -- 文字クラス
        , charClass

        -- 定義済みクラス
        , predefinedClass

        -- エスケープ文字
        , escapeChar

        -- 通常のリテラル
        , map
            (\c -> Literal (String.fromChar c))
            (satisfy
                (\c ->
                    not
                        (List.member c
                            [ '(', ')', '[', ']', '{', '}', '*', '+', '?', '.', '|', '^', '$', '\\' ]
                        )
                )
            )
        ]

escapeChar : Parser Regex
escapeChar =
    andThen
        (\_ ->
            map
                (\c ->
                    Literal
                        (case c of
                            'n' ->
                                "\n"

                            't' ->
                                "\t"

                            'r' ->
                                "\r"

                            _ ->
                                String.fromChar c
                        )
                )
                (satisfy
                    (\c ->
                        List.member c
                            [ 'n', 't', 'r', '\\', '(', ')', '[', ']', '{', '}', '*', '+', '?', '.', '|', '^', '$' ]
                    )
                )
        )
        (string "\\")

predefinedClass : Parser Regex
predefinedClass =
    andThen
        (\_ ->
            map
                (\class ->
                    CharClass (Predefined class)
                )
                (choice
                    [ map (\_ -> Digit) (char 'd')
                    , map (\_ -> Word) (char 'w')
                    , map (\_ -> Whitespace) (char 's')
                    , map (\_ -> NotDigit) (char 'D')
                    , map (\_ -> NotWord) (char 'W')
                    , map (\_ -> NotWhitespace) (char 'S')
                    ]
                )
        )
        (string "\\")

charClass : Parser Regex
charClass =
    andThen
        (\_ ->
            andThen
                (\negated ->
                    andThen
                        (\items ->
                            andThen
                                (\_ ->
                                    let
                                        unionSet =
                                            Union items
                                    in
                                    ok
                                        (CharClass
                                            (case negated of
                                                Just _ ->
                                                    Negated unionSet

                                                Nothing ->
                                                    unionSet
                                            )
                                        )
                                )
                                (string "]")
                        )
                        (many1 charClassItem)
                )
                (optional (string "^"))
        )
        (string "[")

charClassItem : Parser CharSet
charClassItem =
    choice
        [ -- 範囲
          andThen
            (\start ->
                map
                    (\endOpt ->
                        case endOpt of
                            Just end ->
                                CharRange start end

                            Nothing ->
                                CharList [ start ]
                    )
                    (optional
                        (andThen
                            (\_ -> satisfy (\c -> c /= ']'))
                            (string "-")
                        )
                    )
            )
            (satisfy (\c -> c /= ']' && c /= '-'))

        -- 単一文字
        , map (\c -> CharList [ c ]) (satisfy (\c -> c /= ']'))
        ]

repeatSuffix : Parser RepeatKind
repeatSuffix =
    choice
        [ map (\_ -> ZeroOrMore) (string "*")
        , map (\_ -> OneOrMore) (string "+")
        , map (\_ -> ZeroOrOne) (string "?")
        ]

-- マッチングエンジン
matchRegex : Regex -> String -> Bool
matchRegex regex text =
    matchFromPos regex text 0

matchFromPos : Regex -> String -> Int -> Bool
matchFromPos regex text pos =
    case regex of
        Literal s ->
            String.startsWith s (String.dropLeft pos text)

        CharClass cs ->
            case String.uncons (String.dropLeft pos text) of
                Nothing ->
                    False

                Just ( ch, _ ) ->
                    charMatchesClass ch cs

        Dot ->
            case String.uncons (String.dropLeft pos text) of
                Just _ ->
                    True

                Nothing ->
                    False

        Concat terms ->
            List.foldl
                (\term ( matched, currentPos ) ->
                    if not matched then
                        ( False, currentPos )

                    else
                        ( matchFromPos term text currentPos, currentPos + 1 )
                )
                ( True, pos )
                terms
                |> Tuple.first

        Alternation alts ->
            List.any (\alt -> matchFromPos alt text pos) alts

        Repeat inner kind ->
            case kind of
                ZeroOrMore ->
                    matchRepeatZeroOrMore inner text pos

                OneOrMore ->
                    matchRepeatOneOrMore inner text pos

                ZeroOrOne ->
                    matchRepeatZeroOrOne inner text pos

        Group inner ->
            matchFromPos inner text pos

        Anchor kind ->
            case kind of
                Start ->
                    pos == 0

                End ->
                    pos >= String.length text

charMatchesClass : Char -> CharSet -> Bool
charMatchesClass ch cs =
    case cs of
        CharRange start end ->
            ch >= start && ch <= end

        CharList chars ->
            List.member ch chars

        Predefined class ->
            case class of
                Digit ->
                    ch >= '0' && ch <= '9'

                Word ->
                    (ch >= 'a' && ch <= 'z')
                        || (ch >= 'A' && ch <= 'Z')
                        || (ch >= '0' && ch <= '9')
                        || ch == '_'

                Whitespace ->
                    List.member ch [ ' ', '\t', '\n', '\r' ]

                NotDigit ->
                    not (ch >= '0' && ch <= '9')

                NotWord ->
                    not
                        ((ch >= 'a' && ch <= 'z')
                            || (ch >= 'A' && ch <= 'Z')
                            || (ch >= '0' && ch <= '9')
                            || ch == '_'
                        )

                NotWhitespace ->
                    not (List.member ch [ ' ', '\t', '\n', '\r' ])

        Negated inner ->
            not (charMatchesClass ch inner)

        Union sets ->
            List.any (\set -> charMatchesClass ch set) sets

matchRepeatZeroOrMore : Regex -> String -> Int -> Bool
matchRepeatZeroOrMore inner text pos =
    matchRepeatLoop inner text pos 0 0 999999

matchRepeatOneOrMore : Regex -> String -> Int -> Bool
matchRepeatOneOrMore inner text pos =
    if matchFromPos inner text pos then
        matchRepeatZeroOrMore inner text (pos + 1)

    else
        False

matchRepeatZeroOrOne : Regex -> String -> Int -> Bool
matchRepeatZeroOrOne inner text pos =
    matchFromPos inner text pos || True

matchRepeatLoop : Regex -> String -> Int -> Int -> Int -> Int -> Bool
matchRepeatLoop inner text pos count minCount maxCount =
    if count == maxCount then
        True

    else if count >= minCount && not (matchFromPos inner text pos) then
        True

    else if matchFromPos inner text pos then
        matchRepeatLoop inner text (pos + 1) (count + 1) minCount maxCount

    else if count >= minCount then
        True

    else
        False

-- テスト例
testExamples : List ( String, String, Bool )
testExamples =
    [ ( "a+", "aaa", True )
    , ( "a+", "b", False )
    , ( "[0-9]+", "123", True )
    , ( "[0-9]+", "abc", False )
    , ( "(abc)+", "abcabc", True )
    , ( "a|b", "a", True )
    , ( "a|b", "b", True )
    , ( "a|b", "c", False )
    , ( "^hello$", "hello", True )
    , ( "^hello$", "hello world", False )
    ]

runTests : String
runTests =
    testExamples
        |> List.map
            (\( pattern, textStr, expected ) ->
                case parseRegex pattern of
                    Ok regex ->
                        let
                            result =
                                matchRegex regex textStr

                            status =
                                if result == expected then
                                    "✓"

                                else
                                    "✗"
                        in
                        status ++ " パターン: '" ++ pattern ++ "', テキスト: '" ++ textStr ++ "', 期待: " ++ (if expected then "True" else "False") ++ ", 結果: " ++ (if result then "True" else "False")

                    Err err ->
                        "✗ パーサーエラー: " ++ pattern ++ " - " ++ err
            )
        |> String.join "\n"

-- Main
main : Html msg
main =
    div []
        [ pre [] [ text runTests ]
        ]