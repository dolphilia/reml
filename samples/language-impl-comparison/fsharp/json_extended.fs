/// JSON拡張版：コメント・トレーリングカンマ対応。
///
/// 標準JSONからの拡張点：
/// 1. コメント対応（`//` 行コメント、`/* */` ブロックコメント）
/// 2. トレーリングカンマ許可（配列・オブジェクトの最後の要素の後）
/// 3. より詳細なエラーメッセージ
///
/// 実用的な設定ファイル形式として：
/// - `package.json` 風の設定ファイル
/// - `.babelrc`, `.eslintrc` など開発ツールの設定
/// - VS Code の `settings.json`
module JsonExtended

open System
open System.Collections.Generic

// 型定義

type JsonValue =
  | JNull
  | JBool of bool
  | JNumber of float
  | JString of string
  | JArray of JsonValue list
  | JObject of Map<string, JsonValue>

type ParseError =
  | UnexpectedEOF
  | InvalidValue of string
  | UnclosedString
  | UnclosedBlockComment
  | ExpectedChar of char
  | InvalidNumber of string

type State =
  { Input: string
    Pos: int }

// パース

let parse (input: string) : Result<JsonValue, ParseError> =
  let rec skipWhitespaceAndComments (state: State) : Result<State, ParseError> =
    let skipWs (st: State) =
      let mutable pos = st.Pos
      while pos < st.Input.Length && (st.Input.[pos] = ' ' || st.Input.[pos] = '\n' || st.Input.[pos] = '\t' || st.Input.[pos] = '\r') do
        pos <- pos + 1
      { st with Pos = pos }

    let skipLineComment (st: State) =
      let newPos = st.Pos + 2
      let idx = st.Input.IndexOf('\n', newPos)
      let finalPos = if idx = -1 then st.Input.Length else idx + 1
      { st with Pos = finalPos }

    let skipBlockComment (st: State) : Result<State, ParseError> =
      let newPos = st.Pos + 2
      let idx = st.Input.IndexOf("*/", newPos)
      if idx = -1 then
        Error UnclosedBlockComment
      else
        Ok { st with Pos = idx + 2 }

    let stateAfterWs = skipWs state
    if stateAfterWs.Pos >= stateAfterWs.Input.Length then
      Ok stateAfterWs
    else if stateAfterWs.Pos + 1 < stateAfterWs.Input.Length && stateAfterWs.Input.Substring(stateAfterWs.Pos, 2) = "//" then
      skipWhitespaceAndComments (skipLineComment stateAfterWs)
    else if stateAfterWs.Pos + 1 < stateAfterWs.Input.Length && stateAfterWs.Input.Substring(stateAfterWs.Pos, 2) = "/*" then
      match skipBlockComment stateAfterWs with
      | Ok st -> skipWhitespaceAndComments st
      | Error e -> Error e
    else
      Ok stateAfterWs

  let rec parseValue (state: State) : Result<JsonValue * State, ParseError> =
    match skipWhitespaceAndComments state with
    | Error e -> Error e
    | Ok cleanState ->
      if cleanState.Pos >= cleanState.Input.Length then
        Error UnexpectedEOF
      else
        let remaining = cleanState.Input.Substring(cleanState.Pos)
        if remaining.StartsWith("null") then
          Ok (JNull, { cleanState with Pos = cleanState.Pos + 4 })
        else if remaining.StartsWith("true") then
          Ok (JBool true, { cleanState with Pos = cleanState.Pos + 4 })
        else if remaining.StartsWith("false") then
          Ok (JBool false, { cleanState with Pos = cleanState.Pos + 5 })
        else if remaining.StartsWith("\"") then
          parseString cleanState
        else if remaining.StartsWith("[") then
          parseArray cleanState
        else if remaining.StartsWith("{") then
          parseObject cleanState
        else
          parseNumber cleanState

  and parseString (state: State) : Result<JsonValue * State, ParseError> =
    let rec findEnd (pos: int) (acc: string) =
      if pos >= state.Input.Length then
        Error UnclosedString
      else
        match state.Input.[pos] with
        | '"' -> Ok (acc, pos + 1)
        | '\\' when pos + 1 < state.Input.Length ->
          let escaped =
            match state.Input.[pos + 1] with
            | 'n' -> "\n"
            | 't' -> "\t"
            | 'r' -> "\r"
            | '\\' -> "\\"
            | '"' -> "\""
            | ch -> string ch
          findEnd (pos + 2) (acc + escaped)
        | ch -> findEnd (pos + 1) (acc + string ch)

    match findEnd (state.Pos + 1) "" with
    | Ok (str, endPos) -> Ok (JString str, { state with Pos = endPos })
    | Error e -> Error e

  and parseNumber (state: State) : Result<JsonValue * State, ParseError> =
    let isNumChar ch =
      ch = '-' || ch = '+' || ch = '.' || ch = 'e' || ch = 'E' || (ch >= '0' && ch <= '9')

    let mutable endPos = state.Pos
    while endPos < state.Input.Length && isNumChar state.Input.[endPos] do
      endPos <- endPos + 1

    let numStr = state.Input.Substring(state.Pos, endPos - state.Pos)
    match Double.TryParse(numStr) with
    | (true, num) -> Ok (JNumber num, { state with Pos = endPos })
    | (false, _) -> Error (InvalidNumber numStr)

  and parseArray (state: State) : Result<JsonValue * State, ParseError> =
    let stateAfterBracket = { state with Pos = state.Pos + 1 }
    match skipWhitespaceAndComments stateAfterBracket with
    | Error e -> Error e
    | Ok cleanState ->
      if cleanState.Pos < cleanState.Input.Length && cleanState.Input.[cleanState.Pos] = ']' then
        Ok (JArray [], { cleanState with Pos = cleanState.Pos + 1 })
      else
        parseArrayElements cleanState []

  and parseArrayElements (state: State) (acc: JsonValue list) : Result<JsonValue * State, ParseError> =
    match parseValue state with
    | Error e -> Error e
    | Ok (value, stateAfterValue) ->
      match skipWhitespaceAndComments stateAfterValue with
      | Error e -> Error e
      | Ok cleanState ->
        let newAcc = acc @ [value]
        if cleanState.Pos >= cleanState.Input.Length then
          Error UnexpectedEOF
        else
          match cleanState.Input.[cleanState.Pos] with
          | ',' ->
            let stateAfterComma = { cleanState with Pos = cleanState.Pos + 1 }
            match skipWhitespaceAndComments stateAfterComma with
            | Error e -> Error e
            | Ok stateAfterWs ->
              if stateAfterWs.Pos < stateAfterWs.Input.Length && stateAfterWs.Input.[stateAfterWs.Pos] = ']' then
                // トレーリングカンマ
                Ok (JArray newAcc, { stateAfterWs with Pos = stateAfterWs.Pos + 1 })
              else
                parseArrayElements stateAfterWs newAcc
          | ']' ->
            Ok (JArray newAcc, { cleanState with Pos = cleanState.Pos + 1 })
          | _ ->
            Error (ExpectedChar ',')

  and parseObject (state: State) : Result<JsonValue * State, ParseError> =
    let stateAfterBrace = { state with Pos = state.Pos + 1 }
    match skipWhitespaceAndComments stateAfterBrace with
    | Error e -> Error e
    | Ok cleanState ->
      if cleanState.Pos < cleanState.Input.Length && cleanState.Input.[cleanState.Pos] = '}' then
        Ok (JObject Map.empty, { cleanState with Pos = cleanState.Pos + 1 })
      else
        parseObjectPairs cleanState Map.empty

  and parseObjectPairs (state: State) (acc: Map<string, JsonValue>) : Result<JsonValue * State, ParseError> =
    match parseString state with
    | Error e -> Error e
    | Ok (JString key, stateAfterKey) ->
      match skipWhitespaceAndComments stateAfterKey with
      | Error e -> Error e
      | Ok cleanState1 ->
        if cleanState1.Pos >= cleanState1.Input.Length || cleanState1.Input.[cleanState1.Pos] <> ':' then
          Error (ExpectedChar ':')
        else
          let stateAfterColon = { cleanState1 with Pos = cleanState1.Pos + 1 }
          match skipWhitespaceAndComments stateAfterColon with
          | Error e -> Error e
          | Ok cleanState2 ->
            match parseValue cleanState2 with
            | Error e -> Error e
            | Ok (value, stateAfterValue) ->
              match skipWhitespaceAndComments stateAfterValue with
              | Error e -> Error e
              | Ok cleanState3 ->
                let newAcc = Map.add key value acc
                if cleanState3.Pos >= cleanState3.Input.Length then
                  Error UnexpectedEOF
                else
                  match cleanState3.Input.[cleanState3.Pos] with
                  | ',' ->
                    let stateAfterComma = { cleanState3 with Pos = cleanState3.Pos + 1 }
                    match skipWhitespaceAndComments stateAfterComma with
                    | Error e -> Error e
                    | Ok stateAfterWs ->
                      if stateAfterWs.Pos < stateAfterWs.Input.Length && stateAfterWs.Input.[stateAfterWs.Pos] = '}' then
                        // トレーリングカンマ
                        Ok (JObject newAcc, { stateAfterWs with Pos = stateAfterWs.Pos + 1 })
                      else
                        parseObjectPairs stateAfterWs newAcc
                  | '}' ->
                    Ok (JObject newAcc, { cleanState3 with Pos = cleanState3.Pos + 1 })
                  | _ ->
                    Error (ExpectedChar ',')
    | Ok _ -> Error (InvalidValue "オブジェクトのキーは文字列である必要があります")
    | Error e -> Error e

  let initialState = { Input = input; Pos = 0 }
  match skipWhitespaceAndComments initialState with
  | Error e -> Error e
  | Ok st1 ->
    match parseValue st1 with
    | Error e -> Error e
    | Ok (value, st2) ->
      match skipWhitespaceAndComments st2 with
      | Error e -> Error e
      | Ok finalState ->
        if finalState.Pos >= finalState.Input.Length then
          Ok value
        else
          Error (InvalidValue "入力の終端に到達していません")

// レンダリング

let rec renderToString (value: JsonValue) (indentLevel: int) : string =
  let indent = String.replicate indentLevel "  "
  let nextIndent = String.replicate (indentLevel + 1) "  "

  match value with
  | JNull -> "null"
  | JBool true -> "true"
  | JBool false -> "false"
  | JNumber num -> num.ToString()
  | JString str -> "\"" + str + "\""
  | JArray items ->
    if List.isEmpty items then
      "[]"
    else
      let itemsStr =
        items
        |> List.map (fun item -> nextIndent + renderToString item (indentLevel + 1))
        |> String.concat ",\n"
      "[\n" + itemsStr + "\n" + indent + "]"
  | JObject pairs ->
    if Map.isEmpty pairs then
      "{}"
    else
      let pairsStr =
        pairs
        |> Map.toList
        |> List.map (fun (key, value) ->
          nextIndent + "\"" + key + "\": " + renderToString value (indentLevel + 1)
        )
        |> String.concat ",\n"
      "{\n" + pairsStr + "\n" + indent + "}"

// テスト

let testExtendedJson () =
  let testCases =
    [
      ("コメント対応", """
{
  // これは行コメント
  "name": "test",
  /* これは
     ブロックコメント */
  "version": "1.0"
}
""")
      ("トレーリングカンマ", """
{
  "items": [
    1,
    2,
    3,
  ],
  "config": {
    "debug": true,
    "port": 8080,
  }
}
""")
    ]

  testCases
  |> List.iter (fun (name, jsonStr) ->
    printfn "--- %s ---" name
    match parse jsonStr with
    | Ok value ->
      printfn "パース成功:"
      printfn "%s" (renderToString value 0)
    | Error err ->
      printfn "パースエラー: %A" err
    printfn ""
  )