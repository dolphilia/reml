// 正規表現エンジン：パース + 評価の両方を実装。
//
// 対応する正規表現構文（簡易版）：
// - リテラル: `abc`
// - 連結: `ab`
// - 選択: `a|b`
// - 繰り返し: `a*`, `a+`, `a?`, `a{2,5}`
// - グループ: `(abc)`
// - 文字クラス: `[a-z]`, `[^0-9]`, `\d`, `\w`, `\s`
// - アンカー: `^`, `$`
// - ドット: `.` (任意の1文字)

// 正規表現のAST
type Regex =
    | Literal of string
    | CharClass of CharSet
    | Dot
    | Concat of Regex list
    | Alternation of Regex list
    | Repeat of Regex * RepeatKind
    | Group of Regex
    | Anchor of AnchorKind

and CharSet =
    | CharRange of char * char
    | CharList of char list
    | Predefined of PredefinedClass
    | Negated of CharSet
    | Union of CharSet list

and PredefinedClass =
    | Digit
    | Word
    | Whitespace
    | NotDigit
    | NotWord
    | NotWhitespace

and RepeatKind =
    | ZeroOrMore
    | OneOrMore
    | ZeroOrOne
    | Exactly of int
    | Range of int * int option

and AnchorKind =
    | Start
    | End

// パーサーコンビネーター
type Parser<'a> = string -> Result<'a * string, string>

module Parser =
    let ok value : Parser<'a> =
        fun input -> Ok(value, input)

    let fail message : Parser<'a> =
        fun _input -> Error message

    let bind (f: 'a -> Parser<'b>) (parser: Parser<'a>) : Parser<'b> =
        fun input ->
            match parser input with
            | Ok(value, rest) -> f value rest
            | Error err -> Error err

    let map (f: 'a -> 'b) (parser: Parser<'a>) : Parser<'b> =
        bind (fun value -> ok (f value)) parser

    let choice (parsers: Parser<'a> list) : Parser<'a> =
        fun input ->
            let rec tryParsers ps =
                match ps with
                | [] -> Error "no choice matched"
                | p :: rest ->
                    match p input with
                    | Ok result -> Ok result
                    | Error _ -> tryParsers rest
            tryParsers parsers

    let rec many (parser: Parser<'a>) : Parser<'a list> =
        fun input ->
            match parser input with
            | Ok(value, rest) ->
                match many parser rest with
                | Ok(values, finalRest) -> Ok(value :: values, finalRest)
                | Error _ -> Ok([value], rest)
            | Error _ -> Ok([], input)

    let many1 (parser: Parser<'a>) : Parser<'a list> =
        bind (fun first ->
            bind (fun rest -> ok (first :: rest)) (many parser)
        ) parser

    let optional (parser: Parser<'a>) : Parser<'a option> =
        fun input ->
            match parser input with
            | Ok(value, rest) -> Ok(Some value, rest)
            | Error _ -> Ok(None, input)

    let char (c: char) : Parser<char> =
        fun input ->
            if String.length input > 0 && input.[0] = c then
                Ok(c, input.Substring(1))
            else
                Error $"expected {c}"

    let str (s: string) : Parser<string> =
        fun input ->
            if input.StartsWith(s) then
                Ok(s, input.Substring(s.Length))
            else
                Error $"expected {s}"

    let satisfy (pred: char -> bool) : Parser<char> =
        fun input ->
            if String.length input > 0 && pred input.[0] then
                Ok(input.[0], input.Substring(1))
            else
                Error "predicate failed"

    let digit : Parser<char> =
        satisfy (fun c -> c >= '0' && c <= '9')

    let integer : Parser<int> =
        bind (fun digits ->
            let num = List.fold (fun acc d -> acc * 10 + (int d - int '0')) 0 digits
            ok num
        ) (many1 digit)

    let sepBy1 (parser: Parser<'a>) (sep: Parser<'b>) : Parser<'a list> =
        bind (fun first ->
            bind (fun rest -> ok (first :: rest))
                (many (bind (fun _ -> parser) sep))
        ) parser

// 正規表現パーサー
module RegexParser =
    open Parser

    let rec regexExpr() = alternationExpr()

    and alternationExpr() =
        map (fun alts ->
            match alts with
            | [single] -> single
            | _ -> Alternation alts
        ) (sepBy1 (concatExpr()) (str "|"))

    and concatExpr() =
        map (fun terms ->
            match terms with
            | [single] -> single
            | _ -> Concat terms
        ) (many1 (postfixTerm()))

    and postfixTerm() =
        bind (fun baseRegex ->
            map (fun repeatOpt ->
                match repeatOpt with
                | Some kind -> Repeat(baseRegex, kind)
                | None -> baseRegex
            ) (optional (repeatSuffix()))
        ) (atom())

    and atom() =
        choice [
            // 括弧グループ
            bind (fun _ ->
                bind (fun inner ->
                    bind (fun _ -> ok (Group inner)) (str ")")
                ) (regexExpr())
            ) (str "(")

            // アンカー
            map (fun _ -> Anchor Start) (str "^")
            map (fun _ -> Anchor End) (str "$")

            // ドット
            map (fun _ -> Dot) (str ".")

            // 文字クラス
            charClass()

            // 定義済みクラス
            predefinedClass()

            // エスケープ文字
            escapeChar()

            // 通常のリテラル
            map (fun c -> Literal (string c))
                (satisfy (fun c ->
                    not (List.contains c ['('; ')'; '['; ']'; '{'; '}'; '*'; '+'; '?'; '.'; '|'; '^'; '$'; '\\'])
                ))
        ]

    and escapeChar() =
        bind (fun _ ->
            map (fun c ->
                Literal (
                    match c with
                    | 'n' -> "\n"
                    | 't' -> "\t"
                    | 'r' -> "\r"
                    | _ -> string c
                )
            ) (satisfy (fun c ->
                List.contains c ['n'; 't'; 'r'; '\\'; '('; ')'; '['; ']'; '{'; '}'; '*'; '+'; '?'; '.'; '|'; '^'; '$']
            ))
        ) (str "\\")

    and predefinedClass() =
        bind (fun _ ->
            map (fun cls -> CharClass (Predefined cls))
                (choice [
                    map (fun _ -> Digit) (char 'd')
                    map (fun _ -> Word) (char 'w')
                    map (fun _ -> Whitespace) (char 's')
                    map (fun _ -> NotDigit) (char 'D')
                    map (fun _ -> NotWord) (char 'W')
                    map (fun _ -> NotWhitespace) (char 'S')
                ])
        ) (str "\\")

    and charClass() =
        bind (fun _ ->
            bind (fun negated ->
                bind (fun items ->
                    bind (fun _ ->
                        let unionSet = Union items
                        ok (CharClass (
                            match negated with
                            | Some _ -> Negated unionSet
                            | None -> unionSet
                        ))
                    ) (str "]")
                ) (many1 (charClassItem()))
            ) (optional (str "^"))
        ) (str "[")

    and charClassItem() =
        choice [
            // 範囲
            bind (fun start ->
                map (fun endOpt ->
                    match endOpt with
                    | Some endChar -> CharRange(start, endChar)
                    | None -> CharList [start]
                ) (optional (bind (fun _ -> satisfy (fun c -> c <> ']')) (str "-")))
            ) (satisfy (fun c -> c <> ']' && c <> '-'))

            // 単一文字
            map (fun c -> CharList [c]) (satisfy (fun c -> c <> ']'))
        ]

    and repeatSuffix() =
        choice [
            map (fun _ -> ZeroOrMore) (str "*")
            map (fun _ -> OneOrMore) (str "+")
            map (fun _ -> ZeroOrOne) (str "?")

            // {n,m} 形式
            bind (fun _ ->
                bind (fun n ->
                    bind (fun rangeOpt ->
                        bind (fun _ ->
                            ok (
                                match rangeOpt with
                                | None -> Exactly n
                                | Some None -> Range(n, None)
                                | Some (Some m) -> Range(n, Some m)
                            )
                        ) (str "}")
                    ) (optional (bind (fun _ -> optional integer) (str ",")))
                ) integer
            ) (str "{")
        ]

    let parseRegex input =
        match regexExpr() input with
        | Ok(regex, "") -> Ok regex
        | Ok(_, rest) -> Error $"unexpected input: {rest}"
        | Error err -> Error err

// マッチングエンジン
module Matcher =
    let rec matchRegex regex text =
        matchFromPos regex text 0

    and matchFromPos regex text pos =
        match regex with
        | Literal s ->
            if pos + s.Length <= text.Length then
                text.Substring(pos, s.Length) = s
            else
                false

        | CharClass cs ->
            if pos < text.Length then
                charMatchesClass text.[pos] cs
            else
                false

        | Dot ->
            pos < text.Length

        | Concat terms ->
            let mutable currentPos = pos
            let mutable matched = true
            for term in terms do
                if not matched then
                    ()
                elif matchFromPos term text currentPos then
                    currentPos <- currentPos + 1
                else
                    matched <- false
            matched

        | Alternation alts ->
            List.exists (fun alt -> matchFromPos alt text pos) alts

        | Repeat(inner, kind) ->
            match kind with
            | ZeroOrMore -> matchRepeatZeroOrMore inner text pos
            | OneOrMore -> matchRepeatOneOrMore inner text pos
            | ZeroOrOne -> matchRepeatZeroOrOne inner text pos
            | Exactly n -> matchRepeatExactly inner text pos n
            | Range(minCount, maxOpt) -> matchRepeatRange inner text pos minCount maxOpt

        | Group inner ->
            matchFromPos inner text pos

        | Anchor kind ->
            match kind with
            | Start -> pos = 0
            | End -> pos >= text.Length

    and charMatchesClass ch cs =
        match cs with
        | CharRange(start, endChar) ->
            ch >= start && ch <= endChar

        | CharList chars ->
            List.contains ch chars

        | Predefined cls ->
            match cls with
            | Digit -> ch >= '0' && ch <= '9'
            | Word -> (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || (ch >= '0' && ch <= '9') || ch = '_'
            | Whitespace -> List.contains ch [' '; '\t'; '\n'; '\r']
            | NotDigit -> not (ch >= '0' && ch <= '9')
            | NotWord -> not ((ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || (ch >= '0' && ch <= '9') || ch = '_')
            | NotWhitespace -> not (List.contains ch [' '; '\t'; '\n'; '\r'])

        | Negated inner ->
            not (charMatchesClass ch inner)

        | Union sets ->
            List.exists (charMatchesClass ch) sets

    and matchRepeatZeroOrMore inner text pos =
        matchRepeatLoop inner text pos 0 0 999999

    and matchRepeatOneOrMore inner text pos =
        if matchFromPos inner text pos then
            matchRepeatZeroOrMore inner text (pos + 1)
        else
            false

    and matchRepeatZeroOrOne inner text pos =
        matchFromPos inner text pos || true

    and matchRepeatExactly inner text pos n =
        matchRepeatLoop inner text pos 0 n n

    and matchRepeatRange inner text pos minCount maxOpt =
        let maxCount = match maxOpt with Some m -> m | None -> 999999
        matchRepeatLoop inner text pos 0 minCount maxCount

    and matchRepeatLoop inner text pos count minCount maxCount =
        if count = maxCount then
            true
        elif count >= minCount && not (matchFromPos inner text pos) then
            true
        elif matchFromPos inner text pos then
            matchRepeatLoop inner text (pos + 1) (count + 1) minCount maxCount
        elif count >= minCount then
            true
        else
            false

// テスト例
let testExamples() =
    let examples = [
        ("a+", "aaa", true)
        ("a+", "b", false)
        ("[0-9]+", "123", true)
        ("[0-9]+", "abc", false)
        ("\\d{2,4}", "12", true)
        ("\\d{2,4}", "12345", true)
        ("(abc)+", "abcabc", true)
        ("a|b", "a", true)
        ("a|b", "b", true)
        ("a|b", "c", false)
        ("^hello$", "hello", true)
        ("^hello$", "hello world", false)
    ]

    examples
    |> List.iter (fun (pattern, text, expected) ->
        match RegexParser.parseRegex pattern with
        | Ok regex ->
            let result = Matcher.matchRegex regex text
            let status = if result = expected then "✓" else "✗"
            printfn "%s パターン: '%s', テキスト: '%s', 期待: %b, 結果: %b" status pattern text expected result
        | Error err ->
            printfn "✗ パーサーエラー: %s - %s" pattern err
    )

// 実行
testExamples()