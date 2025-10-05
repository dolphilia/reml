{-|
JSON拡張版：コメント・トレーリングカンマ対応。

標準JSONからの拡張点：
1. コメント対応（`//` 行コメント、`/* */` ブロックコメント）
2. トレーリングカンマ許可（配列・オブジェクトの最後の要素の後）
3. より詳細なエラーメッセージ

実用的な設定ファイル形式として：
- `package.json` 風の設定ファイル
- `.babelrc`, `.eslintrc` など開発ツールの設定
- VS Code の `settings.json`
-}
module JsonExtended
  ( JsonValue(..)
  , ParseError(..)
  , parse
  , renderToString
  , testExtendedJson
  ) where

import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import Data.Char (isDigit, isSpace)
import Data.List (isPrefixOf)
import Control.Monad (void)

-- 型定義

data JsonValue
  = JNull
  | JBool Bool
  | JNumber Double
  | JString String
  | JArray [JsonValue]
  | JObject (Map String JsonValue)
  deriving (Show, Eq)

data ParseError
  = UnexpectedEOF
  | InvalidValue String
  | UnclosedString
  | UnclosedBlockComment
  | ExpectedChar Char
  | InvalidNumber String
  deriving (Show, Eq)

data State = State
  { stateInput :: String
  , statePos :: Int
  } deriving (Show, Eq)

-- パース

parse :: String -> Either ParseError JsonValue
parse input =
  let initialState = State input 0
  in do
    st1 <- skipWhitespaceAndComments initialState
    (value, st2) <- parseValue st1
    finalState <- skipWhitespaceAndComments st2
    if statePos finalState >= length (stateInput finalState)
      then Right value
      else Left $ InvalidValue "入力の終端に到達していません"

-- 空白とコメントをスキップ

skipWhitespaceAndComments :: State -> Either ParseError State
skipWhitespaceAndComments state =
  let stateAfterWs = skipWs state
  in if statePos stateAfterWs >= length (stateInput stateAfterWs)
    then Right stateAfterWs
    else
      let remaining = drop (statePos stateAfterWs) (stateInput stateAfterWs)
      in if "//" `isPrefixOf` remaining
        then skipWhitespaceAndComments (skipLineComment stateAfterWs)
        else if "/*" `isPrefixOf` remaining
          then skipBlockComment stateAfterWs >>= skipWhitespaceAndComments
          else Right stateAfterWs

skipWs :: State -> State
skipWs state@(State input pos)
  | pos >= length input = state
  | isSpace (input !! pos) = skipWs (State input (pos + 1))
  | otherwise = state

skipLineComment :: State -> State
skipLineComment (State input pos) =
  let newPos = pos + 2
      remaining = drop newPos input
      idx = case break (== '\n') remaining of
        (_, "") -> length remaining
        (_, _:_) -> length (takeWhile (/= '\n') remaining) + 1
  in State input (newPos + idx)

skipBlockComment :: State -> Either ParseError State
skipBlockComment (State input pos) =
  let newPos = pos + 2
      remaining = drop newPos input
  in case findSubstring "*/" remaining of
    Nothing -> Left UnclosedBlockComment
    Just idx -> Right $ State input (newPos + idx + 2)

findSubstring :: String -> String -> Maybe Int
findSubstring needle haystack = go 0 haystack
  where
    go _ [] = Nothing
    go n str
      | needle `isPrefixOf` str = Just n
      | otherwise = go (n + 1) (tail str)

-- 値のパース

parseValue :: State -> Either ParseError (JsonValue, State)
parseValue state = do
  cleanState <- skipWhitespaceAndComments state
  let remaining = drop (statePos cleanState) (stateInput cleanState)
  if null remaining
    then Left UnexpectedEOF
    else if "null" `isPrefixOf` remaining
      then Right (JNull, cleanState { statePos = statePos cleanState + 4 })
      else if "true" `isPrefixOf` remaining
        then Right (JBool True, cleanState { statePos = statePos cleanState + 4 })
        else if "false" `isPrefixOf` remaining
          then Right (JBool False, cleanState { statePos = statePos cleanState + 5 })
          else if "\"" `isPrefixOf` remaining
            then parseString cleanState
            else if "[" `isPrefixOf` remaining
              then parseArray cleanState
              else if "{" `isPrefixOf` remaining
                then parseObject cleanState
                else parseNumber cleanState

-- 文字列リテラルのパース

parseString :: State -> Either ParseError (JsonValue, State)
parseString (State input pos) =
  case findStringEnd input (pos + 1) "" of
    Left err -> Left err
    Right (str, endPos) -> Right (JString str, State input endPos)

findStringEnd :: String -> Int -> String -> Either ParseError (String, Int)
findStringEnd input pos acc
  | pos >= length input = Left UnclosedString
  | input !! pos == '"' = Right (acc, pos + 1)
  | input !! pos == '\\', pos + 1 < length input =
      let escaped = case input !! (pos + 1) of
            'n' -> "\n"
            't' -> "\t"
            'r' -> "\r"
            '\\' -> "\\"
            '"' -> "\""
            ch -> [ch]
      in findStringEnd input (pos + 2) (acc ++ escaped)
  | otherwise = findStringEnd input (pos + 1) (acc ++ [input !! pos])

-- 数値のパース

parseNumber :: State -> Either ParseError (JsonValue, State)
parseNumber (State input pos) =
  let numStr = takeWhile isNumChar (drop pos input)
      endPos = pos + length numStr
  in case reads numStr :: [(Double, String)] of
    [(num, "")] -> Right (JNumber num, State input endPos)
    _ -> Left $ InvalidNumber numStr

isNumChar :: Char -> Bool
isNumChar ch = ch `elem` "+-." || ch == 'e' || ch == 'E' || isDigit ch

-- 配列のパース（トレーリングカンマ対応）

parseArray :: State -> Either ParseError (JsonValue, State)
parseArray (State input pos) = do
  let stateAfterBracket = State input (pos + 1)
  cleanState <- skipWhitespaceAndComments stateAfterBracket
  let remaining = drop (statePos cleanState) (stateInput cleanState)
  if "]" `isPrefixOf` remaining
    then Right (JArray [], cleanState { statePos = statePos cleanState + 1 })
    else parseArrayElements cleanState []

parseArrayElements :: State -> [JsonValue] -> Either ParseError (JsonValue, State)
parseArrayElements state acc = do
  (value, stateAfterValue) <- parseValue state
  cleanState <- skipWhitespaceAndComments stateAfterValue
  let newAcc = acc ++ [value]
      remaining = drop (statePos cleanState) (stateInput cleanState)
  if null remaining
    then Left UnexpectedEOF
    else case head remaining of
      ',' -> do
        let stateAfterComma = cleanState { statePos = statePos cleanState + 1 }
        stateAfterWs <- skipWhitespaceAndComments stateAfterComma
        let nextRemaining = drop (statePos stateAfterWs) (stateInput stateAfterWs)
        if "]" `isPrefixOf` nextRemaining
          then Right (JArray newAcc, stateAfterWs { statePos = statePos stateAfterWs + 1 })
          else parseArrayElements stateAfterWs newAcc
      ']' -> Right (JArray newAcc, cleanState { statePos = statePos cleanState + 1 })
      _ -> Left $ ExpectedChar ','

-- オブジェクトのパース（トレーリングカンマ対応）

parseObject :: State -> Either ParseError (JsonValue, State)
parseObject (State input pos) = do
  let stateAfterBrace = State input (pos + 1)
  cleanState <- skipWhitespaceAndComments stateAfterBrace
  let remaining = drop (statePos cleanState) (stateInput cleanState)
  if "}" `isPrefixOf` remaining
    then Right (JObject Map.empty, cleanState { statePos = statePos cleanState + 1 })
    else parseObjectPairs cleanState Map.empty

parseObjectPairs :: State -> Map String JsonValue -> Either ParseError (JsonValue, State)
parseObjectPairs state acc = do
  (JString key, stateAfterKey) <- parseString state
  cleanState1 <- skipWhitespaceAndComments stateAfterKey
  let remaining1 = drop (statePos cleanState1) (stateInput cleanState1)
  if not (":" `isPrefixOf` remaining1)
    then Left $ ExpectedChar ':'
    else do
      let stateAfterColon = cleanState1 { statePos = statePos cleanState1 + 1 }
      cleanState2 <- skipWhitespaceAndComments stateAfterColon
      (value, stateAfterValue) <- parseValue cleanState2
      cleanState3 <- skipWhitespaceAndComments stateAfterValue
      let newAcc = Map.insert key value acc
          remaining3 = drop (statePos cleanState3) (stateInput cleanState3)
      if null remaining3
        then Left UnexpectedEOF
        else case head remaining3 of
          ',' -> do
            let stateAfterComma = cleanState3 { statePos = statePos cleanState3 + 1 }
            stateAfterWs <- skipWhitespaceAndComments stateAfterComma
            let nextRemaining = drop (statePos stateAfterWs) (stateInput stateAfterWs)
            if "}" `isPrefixOf` nextRemaining
              then Right (JObject newAcc, stateAfterWs { statePos = statePos stateAfterWs + 1 })
              else parseObjectPairs stateAfterWs newAcc
          '}' -> Right (JObject newAcc, cleanState3 { statePos = statePos cleanState3 + 1 })
          _ -> Left $ ExpectedChar ','

-- レンダリング

renderToString :: JsonValue -> Int -> String
renderToString value indentLevel =
  let indent = replicate (indentLevel * 2) ' '
      nextIndent = replicate ((indentLevel + 1) * 2) ' '
  in case value of
    JNull -> "null"
    JBool True -> "true"
    JBool False -> "false"
    JNumber num -> show num
    JString str -> "\"" ++ str ++ "\""
    JArray items ->
      if null items
        then "[]"
        else
          let itemsStr = unlines $ map (\item -> nextIndent ++ renderToString item (indentLevel + 1) ++ ",") (init items)
                      ++ [nextIndent ++ renderToString (last items) (indentLevel + 1)]
          in "[\n" ++ itemsStr ++ "\n" ++ indent ++ "]"
    JObject pairs ->
      if Map.null pairs
        then "{}"
        else
          let pairsList = Map.toList pairs
              pairsStr = unlines $ map (\(key, val) -> nextIndent ++ "\"" ++ key ++ "\": " ++ renderToString val (indentLevel + 1) ++ ",") (init pairsList)
                        ++ [nextIndent ++ "\"" ++ fst (last pairsList) ++ "\": " ++ renderToString (snd (last pairsList)) (indentLevel + 1)]
          in "{\n" ++ pairsStr ++ "\n" ++ indent ++ "}"

-- テスト

testExtendedJson :: IO ()
testExtendedJson = do
  let testCases =
        [ ("コメント対応", "{\n  // これは行コメント\n  \"name\": \"test\",\n  /* これは\n     ブロックコメント */\n  \"version\": \"1.0\"\n}\n")
        , ("トレーリングカンマ", "{\n  \"items\": [\n    1,\n    2,\n    3,\n  ],\n  \"config\": {\n    \"debug\": true,\n    \"port\": 8080,\n  }\n}\n")
        ]
  mapM_ (\(name, jsonStr) -> do
    putStrLn $ "--- " ++ name ++ " ---"
    case parse jsonStr of
      Right value -> do
        putStrLn "パース成功:"
        putStrLn $ renderToString value 0
      Left err ->
        putStrLn $ "パースエラー: " ++ show err
    putStrLn ""
    ) testCases