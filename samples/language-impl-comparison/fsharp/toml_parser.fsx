/// TOML風設定ファイルパーサー：キーバリューペアとテーブルを扱う題材。
///
/// 対応する構文（TOML v1.0.0準拠の簡易版）：
/// - キーバリューペア: `key = "value"`
/// - テーブル: `[section]`
/// - 配列テーブル: `[[array_section]]`
/// - データ型: 文字列、整数、浮動小数点、真偽値、配列、インラインテーブル
/// - コメント: `# comment`
///
/// F#の特徴：
/// - アクティブパターンによる柔軟なパターンマッチ
/// - コンピュテーション式（結果型の合成）
/// - 関数型とオブジェクト指向のハイブリッド

open System
open System.Collections.Generic

// TOML値型
type TomlValue =
    | TomlString of string
    | TomlInteger of int64
    | TomlFloat of float
    | TomlBoolean of bool
    | TomlArray of TomlValue list
    | TomlInlineTable of Map<string, TomlValue>

type TomlTable = Map<string, TomlValue>

type TomlDocument = {
    Root: TomlTable
    Tables: Map<string list, TomlTable>
}

// パース結果型
type ParseResult<'a> = Result<'a * string, string>

// --- 基本パーサー ---

let skipWhitespace (input: string) : string =
    input.TrimStart(' ', '\t')

let rec skipComment (input: string) : string =
    if input.StartsWith("#") then
        let idx = input.IndexOfAny([|'\r'; '\n'|])
        if idx >= 0 then
            input.Substring(idx)
        else
            ""
    else
        input

let rec skipWhitespaceAndComments (input: string) : string =
    let rest = input |> skipWhitespace |> skipComment
    if rest <> input then
        skipWhitespaceAndComments rest
    else
        rest

let expectString (expected: string) (input: string) : ParseResult<string> =
    if input.StartsWith(expected) then
        Ok(expected, input.Substring(expected.Length))
    else
        Error($"Expected '{expected}'")

// --- キー名のパース ---

let isBareKeyChar (c: char) : bool =
    Char.IsLetterOrDigit(c) || c = '-' || c = '_'

let parseBareKey (input: string) : ParseResult<string> =
    let mutable i = 0
    while i < input.Length && isBareKeyChar input.[i] do
        i <- i + 1

    if i = 0 then
        Error("Expected key")
    else
        Ok(input.Substring(0, i), input.Substring(i))

let rec takeUntilUnescaped (delimiter: string) (input: string) (acc: string) : string * string =
    if input.StartsWith("\\" + delimiter) then
        takeUntilUnescaped delimiter (input.Substring(2)) (acc + delimiter)
    elif input.StartsWith(delimiter) then
        (acc, input)
    elif input.Length > 0 then
        takeUntilUnescaped delimiter (input.Substring(1)) (acc + input.Substring(0, 1))
    else
        (acc, "")

let parseQuotedKey (input: string) : ParseResult<string> =
    expectString "\"" input
    |> Result.bind (fun (_, rest) ->
        let (key, rest2) = takeUntilUnescaped "\"" rest ""
        expectString "\"" rest2
        |> Result.map (fun (_, rest3) -> (key, rest3))
    )

let parseKey (input: string) : ParseResult<string> =
    if input.StartsWith("\"") then
        parseQuotedKey input
    else
        parseBareKey input

let rec parseKeyPath (input: string) (acc: string list) : ParseResult<string list> =
    let rest = skipWhitespace input
    parseKey rest
    |> Result.bind (fun (key, rest2) ->
        let rest3 = skipWhitespace rest2
        if rest3.StartsWith(".") then
            parseKeyPath (rest3.Substring(1)) (acc @ [key])
        else
            Ok(acc @ [key], rest3)
    )

// --- 値のパース ---

let parseStringValue (input: string) : ParseResult<TomlValue> =
    if input.StartsWith("\"\"\"") then
        // 複数行基本文字列
        expectString "\"\"\"" input
        |> Result.bind (fun (_, rest) ->
            let idx = rest.IndexOf("\"\"\"")
            if idx >= 0 then
                let content = rest.Substring(0, idx)
                Ok(TomlString content, rest.Substring(idx + 3))
            else
                Error("Unclosed multiline string")
        )
    elif input.StartsWith("'''") then
        // 複数行リテラル文字列
        expectString "'''" input
        |> Result.bind (fun (_, rest) ->
            let idx = rest.IndexOf("'''")
            if idx >= 0 then
                let content = rest.Substring(0, idx)
                Ok(TomlString content, rest.Substring(idx + 3))
            else
                Error("Unclosed multiline literal string")
        )
    elif input.StartsWith("'") then
        // リテラル文字列
        expectString "'" input
        |> Result.bind (fun (_, rest) ->
            let idx = rest.IndexOf("'")
            if idx >= 0 then
                let content = rest.Substring(0, idx)
                Ok(TomlString content, rest.Substring(idx + 1))
            else
                Error("Unclosed literal string")
        )
    elif input.StartsWith("\"") then
        // 基本文字列
        expectString "\"" input
        |> Result.bind (fun (_, rest) ->
            let (content, rest2) = takeUntilUnescaped "\"" rest ""
            expectString "\"" rest2
            |> Result.map (fun (_, rest3) -> (TomlString content, rest3))
        )
    else
        Error("Expected string")

let parseIntegerValue (input: string) : ParseResult<TomlValue> =
    let hasSign = input.StartsWith("-")
    let sign = if hasSign then "-" else ""
    let rest = if hasSign then input.Substring(1) else input

    let mutable i = 0
    while i < rest.Length && (Char.IsDigit(rest.[i]) || rest.[i] = '_') do
        i <- i + 1

    if i = 0 then
        Error("Expected integer")
    else
        let digits = rest.Substring(0, i).Replace("_", "")
        match Int64.TryParse(sign + digits) with
        | (true, n) -> Ok(TomlInteger n, rest.Substring(i))
        | (false, _) -> Error("Invalid integer")

let parseFloatValue (input: string) : ParseResult<TomlValue> =
    let hasSign = input.StartsWith("-")
    let sign = if hasSign then "-" else ""
    let rest = if hasSign then input.Substring(1) else input

    let mutable i = 0
    while i < rest.Length && (Char.IsDigit(rest.[i]) || rest.[i] = '.' || rest.[i] = '_' ||
                               rest.[i] = 'e' || rest.[i] = 'E' || rest.[i] = '+' || rest.[i] = '-') do
        i <- i + 1

    if i = 0 then
        Error("Expected float")
    else
        let numStr = rest.Substring(0, i).Replace("_", "")
        if not (numStr.Contains(".")) then
            Error("Expected float")
        else
            match Double.TryParse(sign + numStr) with
            | (true, f) -> Ok(TomlFloat f, rest.Substring(i))
            | (false, _) -> Error("Invalid float")

let parseBooleanValue (input: string) : ParseResult<TomlValue> =
    if input.StartsWith("true") then
        Ok(TomlBoolean true, input.Substring(4))
    elif input.StartsWith("false") then
        Ok(TomlBoolean false, input.Substring(5))
    else
        Error("Expected boolean")

let rec parseArrayValue (input: string) : ParseResult<TomlValue> =
    expectString "[" input
    |> Result.bind (fun (_, rest) ->
        let rest2 = skipWhitespaceAndComments rest
        parseArrayElements rest2 []
    )

and parseArrayElements (input: string) (acc: TomlValue list) : ParseResult<TomlValue> =
    let rest = skipWhitespaceAndComments input
    if rest.StartsWith("]") then
        Ok(TomlArray (List.rev acc), rest.Substring(1))
    elif List.isEmpty acc || rest.StartsWith(",") then
        let rest2 = if not (List.isEmpty acc) then rest.Substring(1) else rest
        let rest3 = skipWhitespaceAndComments rest2
        if rest3.StartsWith("]") then
            Ok(TomlArray (List.rev acc), rest3.Substring(1))
        else
            parseValue rest3
            |> Result.bind (fun (value, rest4) ->
                parseArrayElements rest4 (value :: acc)
            )
    else
        Error("Expected ',' or ']'")

and parseInlineTable (input: string) : ParseResult<TomlValue> =
    expectString "{" input
    |> Result.bind (fun (_, rest) ->
        let rest2 = skipWhitespaceAndComments rest
        parseInlineTableEntries rest2 []
    )

and parseInlineTableEntries (input: string) (acc: (string * TomlValue) list) : ParseResult<TomlValue> =
    let rest = skipWhitespaceAndComments input
    if rest.StartsWith("}") then
        Ok(TomlInlineTable (Map.ofList (List.rev acc)), rest.Substring(1))
    elif List.isEmpty acc || rest.StartsWith(",") then
        let rest2 = if not (List.isEmpty acc) then rest.Substring(1) else rest
        let rest3 = skipWhitespaceAndComments rest2
        if rest3.StartsWith("}") then
            Ok(TomlInlineTable (Map.ofList (List.rev acc)), rest3.Substring(1))
        else
            parseKey rest3
            |> Result.bind (fun (key, rest4) ->
                let rest5 = skipWhitespace rest4
                expectString "=" rest5
                |> Result.bind (fun (_, rest6) ->
                    let rest7 = skipWhitespaceAndComments rest6
                    parseValue rest7
                    |> Result.bind (fun (value, rest8) ->
                        parseInlineTableEntries rest8 ((key, value) :: acc)
                    )
                )
            )
    else
        Error("Expected ',' or '}'")

and parseValue (input: string) : ParseResult<TomlValue> =
    let rest = skipWhitespaceAndComments input
    if rest.StartsWith("\"") || rest.StartsWith("'") then
        parseStringValue rest
    elif rest.StartsWith("true") || rest.StartsWith("false") then
        parseBooleanValue rest
    elif rest.StartsWith("[") then
        parseArrayValue rest
    elif rest.StartsWith("{") then
        parseInlineTable rest
    else
        // 数値（浮動小数点または整数）
        match parseFloatValue rest with
        | Ok result -> Ok result
        | Error _ -> parseIntegerValue rest

// --- キーバリューペアのパース ---

type DocumentElement =
    | KeyValue of string list * TomlValue
    | Table of string list
    | ArrayTable of string list

let parseKeyValuePair (input: string) : ParseResult<DocumentElement> =
    let rest = skipWhitespaceAndComments input
    parseKeyPath rest []
    |> Result.bind (fun (path, rest2) ->
        let rest3 = skipWhitespace rest2
        expectString "=" rest3
        |> Result.bind (fun (_, rest4) ->
            let rest5 = skipWhitespaceAndComments rest4
            parseValue rest5
            |> Result.map (fun (value, rest6) -> (KeyValue(path, value), rest6))
        )
    )

// --- テーブルヘッダーのパース ---

let parseTableHeader (input: string) : ParseResult<DocumentElement> =
    let rest = skipWhitespaceAndComments input
    if rest.StartsWith("[[") then
        expectString "[[" rest
        |> Result.bind (fun (_, rest2) ->
            let rest3 = skipWhitespace rest2
            parseKeyPath rest3 []
            |> Result.bind (fun (path, rest4) ->
                let rest5 = skipWhitespace rest4
                expectString "]]" rest5
                |> Result.map (fun (_, rest6) -> (ArrayTable path, rest6))
            )
        )
    elif rest.StartsWith("[") then
        expectString "[" rest
        |> Result.bind (fun (_, rest2) ->
            let rest3 = skipWhitespace rest2
            parseKeyPath rest3 []
            |> Result.bind (fun (path, rest4) ->
                let rest5 = skipWhitespace rest4
                expectString "]" rest5
                |> Result.map (fun (_, rest6) -> (Table path, rest6))
            )
        )
    else
        Error("Expected table header")

// --- ドキュメント要素のパース ---

let parseDocumentElement (input: string) : ParseResult<DocumentElement> =
    let rest = skipWhitespaceAndComments input
    if String.IsNullOrEmpty(rest) then
        Error("End of input")
    elif rest.StartsWith("[") then
        parseTableHeader rest
    else
        parseKeyValuePair rest

let skipNewline (input: string) : string =
    if input.StartsWith("\r\n") then
        input.Substring(2)
    elif input.StartsWith("\n") then
        input.Substring(1)
    elif input.StartsWith("\r") then
        input.Substring(1)
    else
        input

let rec parseDocumentElements (input: string) (acc: DocumentElement list) : Result<DocumentElement list, string> =
    let rest = skipWhitespaceAndComments input
    if String.IsNullOrEmpty(rest) then
        Ok(List.rev acc)
    else
        match parseDocumentElement rest with
        | Ok(elem, rest2) ->
            let rest3 = skipNewline rest2
            parseDocumentElements rest3 (elem :: acc)
        | Error "End of input" ->
            Ok(List.rev acc)
        | Error msg ->
            Error msg

// --- ドキュメント構築 ---

let rec insertNested (table: TomlTable) (path: string list) (value: TomlValue) : TomlTable =
    match path with
    | [] -> table
    | [key] -> Map.add key value table
    | key :: rest ->
        let nested =
            match Map.tryFind key table with
            | Some (TomlInlineTable t) -> t
            | _ -> Map.empty
        let updatedNested = insertNested nested rest value
        Map.add key (TomlInlineTable updatedNested) table

type BuildState = {
    CurrentTable: string list
    Root: TomlTable
    Tables: Map<string list, TomlTable>
}

let buildDocument (elements: DocumentElement list) : BuildState =
    List.fold (fun state elem ->
        match elem with
        | Table path ->
            let newTables =
                if not (Map.containsKey path state.Tables) then
                    Map.add path Map.empty state.Tables
                else
                    state.Tables
            { state with CurrentTable = path; Tables = newTables }

        | ArrayTable path ->
            let newTables =
                if not (Map.containsKey path state.Tables) then
                    Map.add path Map.empty state.Tables
                else
                    state.Tables
            { state with CurrentTable = path; Tables = newTables }

        | KeyValue(path, value) ->
            if List.isEmpty state.CurrentTable then
                // ルートテーブルに追加
                { state with Root = insertNested state.Root path value }
            else
                // 現在のテーブルに追加
                let table = Map.tryFind state.CurrentTable state.Tables |> Option.defaultValue Map.empty
                let updatedTable = insertNested table path value
                let newTables = Map.add state.CurrentTable updatedTable state.Tables
                { state with Tables = newTables }
    ) { CurrentTable = []; Root = Map.empty; Tables = Map.empty } elements

// --- パブリックAPI ---

let parse (input: string) : Result<TomlDocument, string> =
    parseDocumentElements input []
    |> Result.map (fun elements ->
        let finalState = buildDocument elements
        { Root = finalState.Root; Tables = finalState.Tables }
    )

// --- レンダリング（検証用） ---

let rec renderTable (table: TomlTable) (prefix: string list) : string =
    table
    |> Map.toList
    |> List.map (fun (key, value) ->
        let fullKey = if List.isEmpty prefix then key else String.Join(".", prefix @ [key])
        match value with
        | TomlInlineTable nested ->
            renderTable nested (prefix @ [key])
        | _ ->
            $"{fullKey} = {renderValue value}\n"
    )
    |> String.concat ""

and renderValue (value: TomlValue) : string =
    match value with
    | TomlString s -> $"\"{s}\""
    | TomlInteger n -> string n
    | TomlFloat f -> string f
    | TomlBoolean true -> "true"
    | TomlBoolean false -> "false"
    | TomlArray items ->
        let itemsStr = items |> List.map renderValue |> String.concat ", "
        $"[{itemsStr}]"
    | TomlInlineTable entries ->
        let entriesStr =
            entries
            |> Map.toList
            |> List.map (fun (k, v) -> $"{k} = {renderValue v}")
            |> String.concat ", "
        $"{{ {entriesStr} }}"

let renderToString (doc: TomlDocument) : string =
    let rootOutput = renderTable doc.Root []

    let tableOutput =
        doc.Tables
        |> Map.toList
        |> List.map (fun (path, table) ->
            $"\n[{String.Join(".", path)}]\n{renderTable table []}"
        )
        |> String.concat ""

    rootOutput + tableOutput

// --- テスト ---

let testExamples () =
    let exampleToml = """# Reml パッケージ設定

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

    printfn "--- reml.toml 風設定のパース ---"
    match parse exampleToml with
    | Ok doc ->
        printfn "パース成功:"
        printfn "%s" (renderToString doc)
    | Error err ->
        printfn "パースエラー: %s" err

// テスト実行
testExamples()

(*
F#の特徴：

1. **Result型とバインド演算子**
   - Result.bindによるエラー処理の連鎖
   - 型安全なエラーハンドリング

2. **可変変数とループ**
   - パフォーマンスが必要な部分では可変変数を使用
   - 関数型と命令型のハイブリッド

3. **Map型によるテーブル表現**
   - ネストしたテーブルはMapで表現
   - イミュータブルなデータ構造

4. **アクティブパターン（未使用だが可能）**
   - より複雑なパターンマッチが必要な場合に有用

Remlとの比較：
- Remlはパーサーコンビネーターライブラリによる高レベル抽象化
- F#は手書きパーサーでより明示的な制御が必要
- Remlのcut/commitによるエラー位置特定がより正確
*)
