module Json.Megaparsec where

import Control.Monad (replicateM, void)
import Data.Char (chr)
import Data.Functor (($>))
import qualified Data.Map.Strict as Map
import Data.Void (Void)
import Numeric (readHex)
import Text.Megaparsec (Parsec, between, choice, eof, manyTill, optional, runParser, satisfy, sepBy, try, (<|>))
import qualified Text.Megaparsec as P
import Text.Megaparsec.Char (char, digitChar, hexDigitChar, space1, string)
import qualified Text.Megaparsec.Char.Lexer as L

data Json
  = JNull
  | JBool Bool
  | JNumber Double
  | JString String
  | JArray [Json]
  | JObject (Map.Map String Json)
  deriving (Eq, Show)

type Parser = Parsec Void String

spaceConsumer :: Parser ()
spaceConsumer = L.space space1 (L.skipLineComment "//") (L.skipBlockComment "/*" "*/")

lexeme :: Parser a -> Parser a
lexeme = L.lexeme spaceConsumer

symbol :: String -> Parser String
symbol = L.symbol spaceConsumer

jsonValue :: Parser Json
jsonValue = lexeme (choice [jsonNull, jsonBool, jsonNumber, jsonString, jsonArray, jsonObject])

jsonNull :: Parser Json
jsonNull = string "null" $> JNull

jsonBool :: Parser Json
jsonBool = (string "true" $> JBool True) <|> (string "false" $> JBool False)

jsonNumber :: Parser Json
jsonNumber = do
  num <- L.signed spaceConsumer L.float
  pure (JNumber num)

jsonString :: Parser Json
jsonString = JString <$> stringLiteral

jsonArray :: Parser Json
jsonArray = JArray <$> between (symbol "[") (symbol "]") (jsonValue `sepBy` symbol ",")

jsonObject :: Parser Json
jsonObject =
  let field = do
        key <- stringLiteral
        void (symbol ":")
        value <- jsonValue
        pure (key, value)
   in JObject . Map.fromList <$> between (symbol "{") (symbol "}") (field `sepBy` symbol ",")

stringLiteral :: Parser String
stringLiteral = char '"' *> manyTill charContent (char '"')
  where
    charContent = escapedChar <|> satisfy (`notElem` "\"\n")
    escapedChar = char '\\' *> choice
      [ char '"' $> '"'
      , char '\\' $> '\\'
      , char '/' $> '/'
      , char 'b' $> '\b'
      , char 'f' $> '\f'
      , char 'n' $> '\n'
      , char 'r' $> '\r'
      , char 't' $> '\t'
      , char 'u' *> unicodeEscape
      ]
    unicodeEscape = do
      digits <- replicateM 4 hexDigitChar
      let [(code, _)] = readHex digits
      pure (chr code)

parseJson :: String -> Either String Json
parseJson source =
  case runParser (spaceConsumer *> jsonValue <* spaceConsumer <* eof) "json" source of
    Left err -> Left (P.errorBundlePretty err)
    Right value -> Right value
