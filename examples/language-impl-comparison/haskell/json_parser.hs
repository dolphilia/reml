module JsonParser where
import Data.Char (chr, digitToInt, isDigit, isHexDigit, isSpace)
import Numeric (readHex)

data JsonValue
  = JNull
  | JBool Bool
  | JNumber Double
  | JString String
  | JArray [JsonValue]
  | JObject [(String, JsonValue)]
  deriving (Eq, Show)

parseJson :: String -> Either String JsonValue
parseJson source = do
  (value, rest) <- parseValue (dropWhile isSpace source)
  if all isSpace rest
    then Right value
    else Left "未消費文字が残っています"

parseValue :: String -> Either String (JsonValue, String)
parseValue [] = Left "入力が途中で終了しました"
parseValue input@(c : _)
  | c == 'n' = consumeLiteral "null" input >> pure (JNull, drop 4 input)
  | c == 't' = consumeLiteral "true" input >> pure (JBool True, drop 4 input)
  | c == 'f' = consumeLiteral "false" input >> pure (JBool False, drop 5 input)
  | c == '"' = do
      (text, rest) <- parseString (tail input)
      pure (JString text, rest)
  | c == '[' = parseArray (tail input)
  | c == '{' = parseObject (tail input)
  | c == '-' || isDigit c = parseNumber input
  | otherwise = Left ("想定外の文字: " <> [c])

parseArray :: String -> Either String (JsonValue, String)
parseArray input = go (dropWhile isSpace input) []
  where
    go [] _ = Left "配列が閉じていません"
    go (']' : rest) acc = Right (JArray (reverse acc), rest)
    go src acc = do
      (value, afterValue) <- parseValue (dropWhile isSpace src)
      let next = dropWhile isSpace afterValue
      case next of
        ',' : rest -> go rest (value : acc)
        ']' : rest -> Right (JArray (reverse (value : acc)), rest)
        [] -> Left "配列が途中で終了しました"
        other -> Left ("配列内の区切りが不正です: " <> take 1 other)

parseObject :: String -> Either String (JsonValue, String)
parseObject input = go (dropWhile isSpace input) []
  where
    go [] _ = Left "オブジェクトが閉じていません"
    go ('}' : rest) acc = Right (JObject (reverse acc), rest)
    go src = do
      (key, afterKey) <- case dropWhile isSpace src of
        '"' : rest -> parseString rest
        other -> Left ("キー文字列を期待しました: " <> take 1 other)
      let next = dropWhile isSpace afterKey
      case next of
        ':' : rest -> do
          (value, afterValue) <- parseValue (dropWhile isSpace rest)
          let continue = dropWhile isSpace afterValue
          case continue of
            ',' : more -> go more ((key, value) : acc)
            '}' : more -> Right (JObject (reverse ((key, value) : acc)), more)
            [] -> Left "オブジェクトが途中で終了しました"
            other -> Left ("オブジェクト内の区切りが不正です: " <> take 1 other)
        [] -> Left "キーと値の区切り ':' が不足しています"
        other -> Left ("':' を期待しましたが別の文字です: " <> take 1 other)

parseNumber :: String -> Either String (JsonValue, String)
parseNumber input =
  let (token, rest) = spanNumber input
   in case reads token of
        [(num, "")] -> Right (JNumber num, rest)
        _ -> Left ("数値の解釈に失敗しました: " <> token)

spanNumber :: String -> (String, String)
spanNumber src =
  let (token, rest) = break isBoundary src
   in (token, rest)
  where
    isBoundary c = c `elem` ",]} \n\r\t"

parseString :: String -> Either String (String, String)
parseString input = loop input []
  where
    loop [] _ = Left "文字列が閉じていません"
    loop ('"' : rest) acc = Right (reverse acc, rest)
    loop ('\\' : rest) acc =
      case rest of
        '"' : xs -> loop xs ('"' : acc)
        '\\' : xs -> loop xs ('\\' : acc)
        '/' : xs -> loop xs ('/' : acc)
        'b' : xs -> loop xs ('\b' : acc)
        'f' : xs -> loop xs ('\f' : acc)
        'n' : xs -> loop xs ('\n' : acc)
        'r' : xs -> loop xs ('\r' : acc)
        't' : xs -> loop xs ('\t' : acc)
        'u' : a : b : c : d : xs
          | all isHexDigit [a, b, c, d] ->
              let code = fst . head $ readHex [a, b, c, d]
               in loop xs (chr code : acc)
        _ -> Left "不正なエスケープシーケンスです"
    loop (c : rest) acc = loop rest (c : acc)

consumeLiteral :: String -> String -> Either String ()
consumeLiteral expected input
  | expected `prefixOf` input = Right ()
  | otherwise = Left ("リテラルを期待しました: " <> expected)
  where
    prefixOf pre xs = take (length pre) xs == pre
