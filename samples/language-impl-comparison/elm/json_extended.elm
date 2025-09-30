module JsonExtended exposing
  ( JsonValue(..)
  , parse
  , renderToString
  , testExtendedJson
  )

{-| JSON拡張版：コメント・トレーリングカンマ対応。

標準JSONからの拡張点：
1. コメント対応（`//` 行コメント、`/* */` ブロックコメント）
2. トレーリングカンマ許可（配列・オブジェクトの最後の要素の後）
3. より詳細なエラーメッセージ

実用的な設定ファイル形式として：
- `package.json` 風の設定ファイル
- `.babelrc`, `.eslintrc` など開発ツールの設定
- VS Code の `settings.json`
-}

import Dict exposing (Dict)


-- 型定義


type JsonValue
  = JNull
  | JBool Bool
  | JNumber Float
  | JString String
  | JArray (List JsonValue)
  | JObject (Dict String JsonValue)


type alias State =
  { input : String
  , pos : Int
  }


type ParseError
  = UnexpectedEOF
  | InvalidValue String
  | UnclosedString
  | UnclosedBlockComment
  | ExpectedChar Char
  | InvalidNumber String


-- パース


parse : String -> Result ParseError JsonValue
parse input =
  let
    initialState = { input = input, pos = 0 }
  in
  skipWhitespaceAndComments initialState
    |> Result.andThen parseValue
    |> Result.andThen (\(value, state) ->
      skipWhitespaceAndComments state
        |> Result.andThen (\finalState ->
          if finalState.pos >= String.length finalState.input then
            Ok value
          else
            Err (InvalidValue "入力の終端に到達していません")
        )
    )


-- 空白とコメントをスキップ


skipWhitespaceAndComments : State -> Result ParseError State
skipWhitespaceAndComments state =
  let
    stateAfterWs = skipWs state
  in
  if stateAfterWs.pos >= String.length stateAfterWs.input then
    Ok stateAfterWs
  else
    case String.slice stateAfterWs.pos (stateAfterWs.pos + 2) stateAfterWs.input of
      "//" ->
        skipWhitespaceAndComments (skipLineComment stateAfterWs)

      "/*" ->
        skipBlockComment stateAfterWs
          |> Result.andThen skipWhitespaceAndComments

      _ ->
        Ok stateAfterWs


skipWs : State -> State
skipWs state =
  case String.uncons (String.dropLeft state.pos state.input) of
    Nothing ->
      state

    Just (ch, _) ->
      if ch == ' ' || ch == '\n' || ch == '\t' || ch == '\r' then
        skipWs { state | pos = state.pos + 1 }
      else
        state


skipLineComment : State -> State
skipLineComment state =
  let
    newPos = state.pos + 2
    remaining = String.dropLeft newPos state.input
  in
  case String.indexes "\n" remaining of
    [] ->
      { state | pos = String.length state.input }

    idx :: _ ->
      { state | pos = newPos + idx + 1 }


skipBlockComment : State -> Result ParseError State
skipBlockComment state =
  let
    newPos = state.pos + 2
    remaining = String.dropLeft newPos state.input
  in
  case String.indexes "*/" remaining of
    [] ->
      Err UnclosedBlockComment

    idx :: _ ->
      Ok { state | pos = newPos + idx + 2 }


-- 値のパース


parseValue : State -> Result ParseError (JsonValue, State)
parseValue state =
  skipWhitespaceAndComments state
    |> Result.andThen (\cleanState ->
      let
        remaining = String.dropLeft cleanState.pos cleanState.input
      in
      if String.isEmpty remaining then
        Err UnexpectedEOF
      else if String.startsWith "null" remaining then
        Ok (JNull, { cleanState | pos = cleanState.pos + 4 })
      else if String.startsWith "true" remaining then
        Ok (JBool True, { cleanState | pos = cleanState.pos + 4 })
      else if String.startsWith "false" remaining then
        Ok (JBool False, { cleanState | pos = cleanState.pos + 5 })
      else if String.startsWith "\"" remaining then
        parseString cleanState
      else if String.startsWith "[" remaining then
        parseArray cleanState
      else if String.startsWith "{" remaining then
        parseObject cleanState
      else
        parseNumber cleanState
    )


-- 文字列リテラルのパース


parseString : State -> Result ParseError (JsonValue, State)
parseString state =
  let
    newPos = state.pos + 1
  in
  findStringEnd state.input newPos ""
    |> Result.map (\(str, endPos) ->
      (JString str, { state | pos = endPos })
    )


findStringEnd : String -> Int -> String -> Result ParseError (String, Int)
findStringEnd input pos acc =
  case String.uncons (String.dropLeft pos input) of
    Nothing ->
      Err UnclosedString

    Just ('"', _) ->
      Ok (acc, pos + 1)

    Just ('\\', rest) ->
      case String.uncons rest of
        Nothing ->
          Err UnclosedString

        Just (ch, _) ->
          let
            escaped =
              case ch of
                'n' -> "\n"
                't' -> "\t"
                'r' -> "\r"
                '\\' -> "\\"
                '"' -> "\""
                _ -> String.fromChar ch
          in
          findStringEnd input (pos + 2) (acc ++ escaped)

    Just (ch, _) ->
      findStringEnd input (pos + 1) (acc ++ String.fromChar ch)


-- 数値のパース


parseNumber : State -> Result ParseError (JsonValue, State)
parseNumber state =
  let
    remaining = String.dropLeft state.pos state.input
    numStr = takeWhile isNumChar remaining
    endPos = state.pos + String.length numStr
  in
  case String.toFloat numStr of
    Nothing ->
      Err (InvalidNumber numStr)

    Just num ->
      Ok (JNumber num, { state | pos = endPos })


isNumChar : Char -> Bool
isNumChar ch =
  ch == '-' || ch == '+' || ch == '.' || ch == 'e' || ch == 'E' ||
  (ch >= '0' && ch <= '9')


takeWhile : (Char -> Bool) -> String -> String
takeWhile predicate str =
  case String.uncons str of
    Nothing ->
      ""

    Just (ch, rest) ->
      if predicate ch then
        String.cons ch (takeWhile predicate rest)
      else
        ""


-- 配列のパース（トレーリングカンマ対応）


parseArray : State -> Result ParseError (JsonValue, State)
parseArray state =
  let
    stateAfterBracket = { state | pos = state.pos + 1 }
  in
  skipWhitespaceAndComments stateAfterBracket
    |> Result.andThen (\cleanState ->
      let
        ch = String.slice cleanState.pos (cleanState.pos + 1) cleanState.input
      in
      if ch == "]" then
        Ok (JArray [], { cleanState | pos = cleanState.pos + 1 })
      else
        parseArrayElements cleanState []
    )


parseArrayElements : State -> List JsonValue -> Result ParseError (JsonValue, State)
parseArrayElements state acc =
  parseValue state
    |> Result.andThen (\(value, stateAfterValue) ->
      skipWhitespaceAndComments stateAfterValue
        |> Result.andThen (\cleanState ->
          let
            ch = String.slice cleanState.pos (cleanState.pos + 1) cleanState.input
            newAcc = acc ++ [value]
          in
          if ch == "," then
            let
              stateAfterComma = { cleanState | pos = cleanState.pos + 1 }
            in
            skipWhitespaceAndComments stateAfterComma
              |> Result.andThen (\stateAfterWs ->
                let
                  nextCh = String.slice stateAfterWs.pos (stateAfterWs.pos + 1) stateAfterWs.input
                in
                if nextCh == "]" then
                  -- トレーリングカンマ
                  Ok (JArray newAcc, { stateAfterWs | pos = stateAfterWs.pos + 1 })
                else
                  parseArrayElements stateAfterWs newAcc
              )
          else if ch == "]" then
            Ok (JArray newAcc, { cleanState | pos = cleanState.pos + 1 })
          else
            Err (ExpectedChar ',')
        )
    )


-- オブジェクトのパース（トレーリングカンマ対応）


parseObject : State -> Result ParseError (JsonValue, State)
parseObject state =
  let
    stateAfterBrace = { state | pos = state.pos + 1 }
  in
  skipWhitespaceAndComments stateAfterBrace
    |> Result.andThen (\cleanState ->
      let
        ch = String.slice cleanState.pos (cleanState.pos + 1) cleanState.input
      in
      if ch == "}" then
        Ok (JObject Dict.empty, { cleanState | pos = cleanState.pos + 1 })
      else
        parseObjectPairs cleanState Dict.empty
    )


parseObjectPairs : State -> Dict String JsonValue -> Result ParseError (JsonValue, State)
parseObjectPairs state acc =
  parseString state
    |> Result.andThen (\(keyValue, stateAfterKey) ->
      case keyValue of
        JString key ->
          skipWhitespaceAndComments stateAfterKey
            |> Result.andThen (expectChar ':')
            |> Result.andThen skipWhitespaceAndComments
            |> Result.andThen parseValue
            |> Result.andThen (\(value, stateAfterValue) ->
              skipWhitespaceAndComments stateAfterValue
                |> Result.andThen (\cleanState ->
                  let
                    ch = String.slice cleanState.pos (cleanState.pos + 1) cleanState.input
                    newAcc = Dict.insert key value acc
                  in
                  if ch == "," then
                    let
                      stateAfterComma = { cleanState | pos = cleanState.pos + 1 }
                    in
                    skipWhitespaceAndComments stateAfterComma
                      |> Result.andThen (\stateAfterWs ->
                        let
                          nextCh = String.slice stateAfterWs.pos (stateAfterWs.pos + 1) stateAfterWs.input
                        in
                        if nextCh == "}" then
                          -- トレーリングカンマ
                          Ok (JObject newAcc, { stateAfterWs | pos = stateAfterWs.pos + 1 })
                        else
                          parseObjectPairs stateAfterWs newAcc
                      )
                  else if ch == "}" then
                    Ok (JObject newAcc, { cleanState | pos = cleanState.pos + 1 })
                  else
                    Err (ExpectedChar ',')
                )
            )

        _ ->
          Err (InvalidValue "オブジェクトのキーは文字列である必要があります")
    )


expectChar : Char -> State -> Result ParseError State
expectChar expected state =
  case String.uncons (String.dropLeft state.pos state.input) of
    Nothing ->
      Err UnexpectedEOF

    Just (ch, _) ->
      if ch == expected then
        Ok { state | pos = state.pos + 1 }
      else
        Err (ExpectedChar expected)


-- レンダリング


renderToString : JsonValue -> Int -> String
renderToString value indentLevel =
  let
    indent = String.repeat indentLevel "  "
    nextIndent = String.repeat (indentLevel + 1) "  "
  in
  case value of
    JNull ->
      "null"

    JBool True ->
      "true"

    JBool False ->
      "false"

    JNumber num ->
      String.fromFloat num

    JString str ->
      "\"" ++ str ++ "\""

    JArray items ->
      if List.isEmpty items then
        "[]"
      else
        let
          itemsStr =
            items
              |> List.map (\item -> nextIndent ++ renderToString item (indentLevel + 1))
              |> String.join ",\n"
        in
        "[\n" ++ itemsStr ++ "\n" ++ indent ++ "]"

    JObject pairs ->
      if Dict.isEmpty pairs then
        "{}"
      else
        let
          pairsStr =
            pairs
              |> Dict.toList
              |> List.map (\(key, val) ->
                nextIndent ++ "\"" ++ key ++ "\": " ++ renderToString val (indentLevel + 1)
              )
              |> String.join ",\n"
        in
        "{\n" ++ pairsStr ++ "\n" ++ indent ++ "}"


-- テスト


testExtendedJson : String
testExtendedJson =
  let
    testCases =
      [ ( "コメント対応"
        , """
{
  // これは行コメント
  "name": "test",
  /* これは
     ブロックコメント */
  "version": "1.0"
}
"""
        )
      , ( "トレーリングカンマ"
        , """
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
"""
        )
      ]

    testResult (name, jsonStr) =
      "--- " ++ name ++ " ---\n" ++
        case parse jsonStr of
          Ok value ->
            "パース成功:\n" ++ renderToString value 0

          Err err ->
            "パースエラー: " ++ errorToString err
  in
  testCases
    |> List.map testResult
    |> String.join "\n\n"


errorToString : ParseError -> String
errorToString err =
  case err of
    UnexpectedEOF ->
      "予期しないEOF"

    InvalidValue msg ->
      "不正な値: " ++ msg

    UnclosedString ->
      "文字列が閉じられていません"

    UnclosedBlockComment ->
      "ブロックコメントが閉じられていません"

    ExpectedChar ch ->
      "'" ++ String.fromChar ch ++ "' が必要です"

    InvalidNumber str ->
      "不正な数値: " ++ str