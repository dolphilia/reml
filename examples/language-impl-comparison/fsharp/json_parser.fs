// JSON パーサー (F# 実装)
// JSON 構文を解析して汎用値型に変換する

module JsonParser

open System
open System.Collections.Generic

/// JSON 値型
type JsonValue =
  | JNull
  | JBool of bool
  | JNumber of float
  | JString of string
  | JArray of JsonValue list
  | JObject of Map<string, JsonValue>

/// トークン型
type Token =
  | LBrace
  | RBrace
  | LBracket
  | RBracket
  | Colon
  | Comma
  | StringLiteral of string
  | NumberLiteral of float
  | BoolLiteral of bool
  | NullLiteral

/// パース状態
type ParseState = { Tokens: Token list }

/// パースエラー
type ParseError =
  | UnexpectedEOF
  | UnexpectedToken of expected: string * found: Token

/// トークン化
let tokenize (source: string) : Token list =
  let rec loop (index: int) (acc: Token list) =
    if index >= source.Length then
      List.rev acc
    else
      let ch = source.[index]
      match ch with
      | ' ' | '\n' | '\t' | '\r' -> loop (index + 1) acc
      | '{' -> loop (index + 1) (LBrace :: acc)
      | '}' -> loop (index + 1) (RBrace :: acc)
      | '[' -> loop (index + 1) (LBracket :: acc)
      | ']' -> loop (index + 1) (RBracket :: acc)
      | ':' -> loop (index + 1) (Colon :: acc)
      | ',' -> loop (index + 1) (Comma :: acc)
      | 't' ->
          if source.Substring(index, 4) = "true" then
            loop (index + 4) (BoolLiteral true :: acc)
          else
            loop (index + 1) acc
      | 'f' ->
          if source.Substring(index, 5) = "false" then
            loop (index + 5) (BoolLiteral false :: acc)
          else
            loop (index + 1) acc
      | 'n' ->
          if source.Substring(index, 4) = "null" then
            loop (index + 4) (NullLiteral :: acc)
          else
            loop (index + 1) acc
      | '"' ->
          let endIndex = source.IndexOf('"', index + 1)
          let str = source.Substring(index + 1, endIndex - index - 1)
          loop (endIndex + 1) (StringLiteral str :: acc)
      | _ ->
          // 数値の読み取り (簡易実装)
          let mutable endIndex = index
          while endIndex < source.Length &&
                (Char.IsDigit(source.[endIndex]) ||
                 source.[endIndex] = '.' ||
                 source.[endIndex] = '-') do
            endIndex <- endIndex + 1
          let numStr = source.Substring(index, endIndex - index)
          match Double.TryParse(numStr) with
          | (true, num) -> loop endIndex (NumberLiteral num :: acc)
          | _ -> loop (index + 1) acc
  loop 0 []

/// 値のパース
let rec parseValue (state: ParseState) : Result<JsonValue * ParseState, ParseError> =
  match state.Tokens with
  | [] -> Error UnexpectedEOF
  | token :: rest ->
      match token with
      | NullLiteral -> Ok (JNull, { Tokens = rest })
      | BoolLiteral flag -> Ok (JBool flag, { Tokens = rest })
      | NumberLiteral num -> Ok (JNumber num, { Tokens = rest })
      | StringLiteral text -> Ok (JString text, { Tokens = rest })
      | LBracket -> parseArray { Tokens = rest }
      | LBrace -> parseObject { Tokens = rest }
      | other -> Error (UnexpectedToken ("値", other))

/// 配列のパース
and parseArray (state: ParseState) : Result<JsonValue * ParseState, ParseError> =
  match state.Tokens with
  | RBracket :: rest -> Ok (JArray [], { Tokens = rest })
  | _ ->
      let rec loop (current: ParseState) (acc: JsonValue list) =
        match parseValue current with
        | Error err -> Error err
        | Ok (value, next) ->
            let newAcc = acc @ [value]
            match next.Tokens with
            | Comma :: rest -> loop { Tokens = rest } newAcc
            | RBracket :: rest -> Ok (JArray newAcc, { Tokens = rest })
            | token :: _ -> Error (UnexpectedToken ("]", token))
            | [] -> Error UnexpectedEOF
      loop state []

/// オブジェクトのパース
and parseObject (state: ParseState) : Result<JsonValue * ParseState, ParseError> =
  match state.Tokens with
  | RBrace :: rest -> Ok (JObject Map.empty, { Tokens = rest })
  | _ ->
      let rec loop (current: ParseState) (acc: Map<string, JsonValue>) =
        match current.Tokens with
        | StringLiteral key :: Colon :: rest ->
            match parseValue { Tokens = rest } with
            | Error err -> Error err
            | Ok (value, next) ->
                let newAcc = Map.add key value acc
                match next.Tokens with
                | Comma :: rest' -> loop { Tokens = rest' } newAcc
                | RBrace :: rest' -> Ok (JObject newAcc, { Tokens = rest' })
                | token :: _ -> Error (UnexpectedToken ("}", token))
                | [] -> Error UnexpectedEOF
        | token :: _ -> Error (UnexpectedToken ("文字列", token))
        | [] -> Error UnexpectedEOF
      loop state Map.empty

/// メインパース関数
let parseJson (source: string) : Result<JsonValue, string> =
  let tokens = tokenize source
  let state = { Tokens = tokens }
  match parseValue state with
  | Error UnexpectedEOF -> Error "予期しない入力終端"
  | Error (UnexpectedToken (expected, found)) ->
      Error $"期待: {expected}, 実際: {found}"
  | Ok (value, rest) ->
      if List.isEmpty rest.Tokens then
        Ok value
      else
        Error "末尾に未消費トークンがあります"

// 利用例
// parseJson """{"name": "Alice", "age": 30}"""
// => Ok (JObject (map [("name", JString "Alice"); ("age", JNumber 30.0)]))