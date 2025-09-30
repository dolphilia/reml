/// YAML風パーサー：インデント管理が重要な題材。
///
/// 対応する構文（簡易版）：
/// - スカラー値: 文字列、数値、真偽値、null
/// - リスト: `- item1`
/// - マップ: `key: value`
/// - ネストしたインデント構造
///
/// F#の特徴：
/// - アクティブパターンによる柔軟なパターンマッチ
/// - コンピュテーション式（結果型の合成）
/// - 関数型とオブジェクト指向のハイブリッド

open System
open System.Collections.Generic

// YAML値型
type YamlValue =
    | Scalar of string
    | YamlList of YamlValue list
    | YamlMap of Map<string, YamlValue>
    | Null

// パース結果型
type ParseResult<'a> = Result<'a * string, string>

// --- 基本パーサー ---

let hspace (input: string) : ParseResult<unit> =
    let mutable i = 0
    while i < input.Length && (input.[i] = ' ' || input.[i] = '\t') do
        i <- i + 1
    Ok((), input.Substring(i))

let expectString (expected: string) (input: string) : ParseResult<string> =
    if input.StartsWith(expected) then
        Ok(expected, input.Substring(expected.Length))
    else
        Error($"Expected '{expected}'")

let newline (input: string) : ParseResult<unit> =
    if input.StartsWith("\r\n") then
        Ok((), input.Substring(2))
    elif input.StartsWith("\n") then
        Ok((), input.Substring(1))
    elif input.StartsWith("\r") then
        Ok((), input.Substring(1))
    else
        Error("Expected newline")

// インデント検証
let expectIndent (level: int) (input: string) : ParseResult<unit> =
    let mutable i = 0
    while i < input.Length && input.[i] = ' ' do
        i <- i + 1

    if i = level then
        Ok((), input.Substring(i))
    else
        Error($"インデント不一致: 期待 {level}, 実際 {i}")

// --- スカラー値パーサー ---

let parseScalar (input: string) : ParseResult<YamlValue> =
    if input.StartsWith("null") then
        Ok(Null, input.Substring(4))
    elif input.StartsWith("~") then
        Ok(Null, input.Substring(1))
    elif input.StartsWith("true") then
        Ok(Scalar "true", input.Substring(4))
    elif input.StartsWith("false") then
        Ok(Scalar "false", input.Substring(5))
    else
        // 文字列（引用符なし：行末まで）
        let lines = input.Split([|'\n'|], 2)
        if lines.Length > 0 then
            let trimmed = lines.[0].Trim()
            if trimmed <> "" then
                let rest = if lines.Length > 1 then lines.[1] else ""
                Ok(Scalar trimmed, rest)
            else
                Error("Empty scalar")
        else
            Error("Empty input")

// --- リストパーサー ---

let rec parseListItem (indent: int) (input: string) : ParseResult<YamlValue> =
    expectIndent indent input
    |> Result.bind (fun (_, rest) ->
        expectString "-" rest
        |> Result.bind (fun (_, rest2) ->
            hspace rest2
            |> Result.bind (fun (_, rest3) ->
                parseValue (indent + 2) rest3
            )
        )
    )

and parseList (indent: int) (input: string) : ParseResult<YamlValue> =
    let rec parseListItems acc input =
        match parseListItem indent input with
        | Ok(item, rest) ->
            let rest2 = skipOptionalNewline rest
            parseListItems (item :: acc) rest2
        | Error(_) ->
            if List.isEmpty acc then
                Error("Expected at least one list item")
            else
                Ok(YamlList(List.rev acc), input)

    parseListItems [] input

and skipOptionalNewline (input: string) : string =
    match newline input with
    | Ok(_, rest) -> rest
    | Error(_) -> input

// --- マップパーサー ---

and parseMapEntry (indent: int) (input: string) : ParseResult<string * YamlValue> =
    expectIndent indent input
    |> Result.bind (fun (_, rest) ->
        let colonIdx = rest.IndexOf(':')
        if colonIdx >= 0 then
            let key = rest.Substring(0, colonIdx).Trim()
            let rest2 = rest.Substring(colonIdx + 1)
            hspace rest2
            |> Result.bind (fun (_, rest3) ->
                // 値が同じ行にあるか、次の行にネストされているか
                match parseValue indent rest3 with
                | Ok(value, rest4) ->
                    Ok((key, value), rest4)
                | Error(_) ->
                    newline rest3
                    |> Result.bind (fun (_, rest4) ->
                        parseValue (indent + 2) rest4
                        |> Result.map (fun (value, rest5) ->
                            ((key, value), rest5)
                        )
                    )
            )
        else
            Error("Expected ':'")
    )

and parseMap (indent: int) (input: string) : ParseResult<YamlValue> =
    let rec parseMapEntries acc input =
        match parseMapEntry indent input with
        | Ok((key, value), rest) ->
            let rest2 = skipOptionalNewline rest
            parseMapEntries ((key, value) :: acc) rest2
        | Error(_) ->
            if List.isEmpty acc then
                Error("Expected at least one map entry")
            else
                Ok(YamlMap(Map.ofList (List.rev acc)), input)

    parseMapEntries [] input

// --- 値パーサー（再帰的） ---

and parseValue (indent: int) (input: string) : ParseResult<YamlValue> =
    if input.Contains("-") then
        match parseList indent input with
        | Ok(result) -> Ok(result)
        | Error(_) -> parseMapOrScalar indent input
    else
        parseMapOrScalar indent input

and parseMapOrScalar (indent: int) (input: string) : ParseResult<YamlValue> =
    match parseMap indent input with
    | Ok(result) -> Ok(result)
    | Error(_) -> parseScalar input

// --- ドキュメントパーサー ---

let skipBlankLines (input: string) : string =
    input.Split('\n')
    |> Array.skipWhile (fun line -> line.Trim() = "")
    |> String.concat "\n"

let parse (input: string) : Result<YamlValue, string> =
    let input2 = skipBlankLines input
    match parseValue 0 input2 with
    | Ok(doc, _) -> Ok(doc)
    | Error(msg) -> Error(msg)

// --- レンダリング（検証用） ---

let rec renderToString (doc: YamlValue) : string =
    renderValue doc 0

and renderValue (value: YamlValue) (indent: int) : string =
    let indentStr = String.replicate indent " "

    match value with
    | Scalar s -> s
    | Null -> "null"
    | YamlList items ->
        items
        |> List.map (fun item ->
            sprintf "%s- %s" indentStr (renderValue item (indent + 2))
        )
        |> String.concat "\n"
    | YamlMap entries ->
        entries
        |> Map.toList
        |> List.map (fun (key, value) ->
            match value with
            | Scalar _ | Null ->
                sprintf "%s%s: %s" indentStr key (renderValue value 0)
            | _ ->
                sprintf "%s%s:\n%s" indentStr key (renderValue value (indent + 2))
        )
        |> String.concat "\n"

// --- テスト ---

let testExamples () =
    let examples = [
        ("simple_scalar", "hello")
        ("simple_list", "- item1\n- item2\n- item3")
        ("simple_map", "key1: value1\nkey2: value2")
        ("nested_map", "parent:\n  child1: value1\n  child2: value2")
        ("nested_list", "items:\n  - item1\n  - item2")
        ("mixed", "name: John\nage: 30\nhobbies:\n  - reading\n  - coding")
    ]

    examples
    |> List.iter (fun (name, yamlStr) ->
        printfn "--- %s ---" name
        match parse yamlStr with
        | Ok(doc) ->
            printfn "パース成功:"
            printfn "%s" (renderToString doc)
        | Error(err) ->
            printfn "パースエラー: %s" err
        printfn ""
    )

// テスト実行
testExamples ()

(*
F#の特徴：

1. **Result型とコンビネータ**
   - Result.bindで連鎖的なエラー処理
   - パイプライン演算子との組み合わせが強力

2. **アクティブパターン**
   - パターンマッチを拡張可能
   - 本実装では簡易化のため未使用

3. **相互再帰**
   - `and` キーワードで相互再帰を自然に表現
   - パーサーの再帰構造が明快

4. **課題**
   - インデント管理が手動で煩雑
   - エラーメッセージの位置情報が不足

Remlとの比較：
- Remlはパーサーコンビネーターライブラリで高レベル
- F#は手書きだが型システムの恩恵が大きい
*)