{-# LANGUAGE OverloadedStrings #-}

module TomlParser where

import Control.Applicative ((<|>), many, some, optional)
import Control.Monad (void)
import Data.Char (isAlphaNum, isDigit)
import Data.List (intercalate)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import Data.Void (Void)
import Text.Megaparsec
  ( Parsec,
    ParseErrorBundle,
    between,
    choice,
    eof,
    lookAhead,
    manyTill,
    parse,
    satisfy,
    sepBy,
    sepBy1,
    try,
    (<?>),
  )
import Text.Megaparsec.Char
  ( char,
    digitChar,
    eol,
    letterChar,
    space,
    space1,
    string,
  )
import qualified Text.Megaparsec.Char.Lexer as L

-- | TOML風設定ファイルパーサー：キーバリューペアとテーブルを扱う題材。
--
-- 対応する構文（TOML v1.0.0準拠の簡易版）：
-- - キーバリューペア: `key = "value"`
-- - テーブル: `[section]`
-- - 配列テーブル: `[[array_section]]`
-- - データ型: 文字列、整数、浮動小数点、真偽値、配列、インラインテーブル
-- - コメント: `# comment`
--
-- Haskellの特徴：
-- - Megaparsecによる高品質なエラーメッセージ
-- - モナディックコンビネーター
-- - 代数的データ型による型安全性

type Parser = Parsec Void Text

-- | TOML値の表現。
data TomlValue
  = TomlString Text
  | TomlInteger Integer
  | TomlFloat Double
  | TomlBoolean Bool
  | TomlArray [TomlValue]
  | TomlInlineTable (Map Text TomlValue)
  deriving (Show, Eq)

type TomlTable = Map Text TomlValue

data TomlDocument = TomlDocument
  { docRoot :: TomlTable,
    docTables :: Map [Text] TomlTable
  }
  deriving (Show, Eq)

-- | ドキュメント要素。
data DocumentElement
  = KeyValue [Text] TomlValue
  | Table [Text]
  | ArrayTable [Text]
  deriving (Show, Eq)

-- | 空白とコメントのスキップ。
sc :: Parser ()
sc = L.space space1 (L.skipLineComment "#") (L.skipBlockComment "" "")

-- | 字句解析：トークン後の空白とコメントをスキップ。
lexeme :: Parser a -> Parser a
lexeme = L.lexeme sc

-- | 特定の文字列トークンをパース。
symbol :: Text -> Parser Text
symbol = L.symbol sc

-- | キー名のパース（ベアキーまたは引用符付きキー）。
parseKey :: Parser Text
parseKey = lexeme $ choice
  [ quotedKey,
    bareKey
  ]
  <?> "key"
  where
    bareKey = T.pack <$> some (satisfy isBareKeyChar)
    isBareKeyChar c = isAlphaNum c || c == '-' || c == '_'

    quotedKey = do
      void $ char '"'
      content <- T.pack <$> manyTill L.charLiteral (char '"')
      return content

-- | ドットで区切られたキーパス（例：`section.subsection.key`）。
parseKeyPath :: Parser [Text]
parseKeyPath = parseKey `sepBy1` symbol "." <?> "key path"

-- | 文字列値のパース（基本文字列・リテラル文字列・複数行対応）。
parseStringValue :: Parser TomlValue
parseStringValue = lexeme $ choice
  [ multilineBasicString,
    multilineLiteralString,
    basicString,
    literalString
  ]
  <?> "string value"
  where
    basicString = do
      void $ char '"'
      content <- T.pack <$> manyTill L.charLiteral (char '"')
      return $ TomlString content

    literalString = do
      void $ char '\''
      content <- T.pack <$> manyTill (satisfy (/= '\'')) (char '\'')
      return $ TomlString content

    multilineBasicString = do
      void $ string "\"\"\""
      content <- T.pack <$> manyTill L.charLiteral (string "\"\"\"")
      return $ TomlString content

    multilineLiteralString = do
      void $ string "'''"
      content <- T.pack <$> manyTill (satisfy (/= '\'')) (string "'''")
      return $ TomlString content

-- | 整数値のパース。
parseIntegerValue :: Parser TomlValue
parseIntegerValue = lexeme $ do
  sign <- optional (char '-')
  digits <- some (digitChar <|> char '_')
  let cleanDigits = filter (/= '_') digits
  let n = read cleanDigits :: Integer
  return $ TomlInteger (if sign == Just '-' then -n else n)
  <?> "integer"

-- | 浮動小数点値のパース。
parseFloatValue :: Parser TomlValue
parseFloatValue = lexeme $ try $ do
  sign <- optional (char '-')
  intPart <- some digitChar
  void $ char '.'
  fracPart <- some digitChar
  let numStr = intPart ++ "." ++ fracPart
  let f = read numStr :: Double
  return $ TomlFloat (if sign == Just '-' then -f else f)
  <?> "float"

-- | 真偽値のパース。
parseBooleanValue :: Parser TomlValue
parseBooleanValue = lexeme $ choice
  [ TomlBoolean True <$ string "true",
    TomlBoolean False <$ string "false"
  ]
  <?> "boolean"

-- | 配列のパース。
parseArrayValue :: Parser TomlValue
parseArrayValue = do
  void $ symbol "["
  values <- parseValue `sepBy` symbol ","
  optional (symbol ",") -- トレーリングカンマ許可
  void $ symbol "]"
  return $ TomlArray values
  <?> "array"

-- | インラインテーブルのパース（`{ key = value, ... }`）。
parseInlineTable :: Parser TomlValue
parseInlineTable = do
  void $ symbol "{"
  entries <- parseEntry `sepBy` symbol ","
  optional (symbol ",") -- トレーリングカンマ許可
  void $ symbol "}"
  return $ TomlInlineTable (Map.fromList entries)
  <?> "inline table"
  where
    parseEntry = do
      key <- parseKey
      void $ symbol "="
      value <- parseValue
      return (key, value)

-- | TOML値のパース（再帰的）。
parseValue :: Parser TomlValue
parseValue = choice
  [ try parseStringValue,
    try parseFloatValue,
    try parseIntegerValue,
    parseBooleanValue,
    parseArrayValue,
    parseInlineTable
  ]
  <?> "value"

-- | キーバリューペアのパース（`key = value`）。
parseKeyValuePair :: Parser DocumentElement
parseKeyValuePair = do
  path <- parseKeyPath
  void $ symbol "="
  value <- parseValue
  return $ KeyValue path value
  <?> "key-value pair"

-- | テーブルヘッダーのパース（`[section]` または `[[array_section]]`）。
parseTableHeader :: Parser DocumentElement
parseTableHeader = choice
  [ arrayTableHeader,
    tableHeader
  ]
  <?> "table header"
  where
    tableHeader = do
      void $ symbol "["
      path <- parseKeyPath
      void $ symbol "]"
      return $ Table path

    arrayTableHeader = do
      void $ symbol "[["
      path <- parseKeyPath
      void $ symbol "]]"
      return $ ArrayTable path

-- | ドキュメント要素のパース。
parseDocumentElement :: Parser DocumentElement
parseDocumentElement = choice
  [ try parseTableHeader,
    parseKeyValuePair
  ]
  <?> "document element"

-- | ドキュメント全体のパース。
parseDocument :: Parser TomlDocument
parseDocument = do
  sc
  elements <- many (parseDocumentElement <* sc)
  eof
  return $ buildDocument elements

-- | ドキュメント要素からTomlDocumentを構築。
buildDocument :: [DocumentElement] -> TomlDocument
buildDocument elements =
  let (_, root, tables) = foldl processElement ([], Map.empty, Map.empty) elements
   in TomlDocument root tables
  where
    processElement (currentTable, root, tables) element =
      case element of
        Table path ->
          let newTables = if Map.member path tables
                            then tables
                            else Map.insert path Map.empty tables
           in (path, root, newTables)

        ArrayTable path ->
          let newTables = if Map.member path tables
                            then tables
                            else Map.insert path Map.empty tables
           in (path, root, newTables)

        KeyValue path value ->
          if null currentTable
            then
              -- ルートテーブルに追加
              let newRoot = insertNested root path value
               in (currentTable, newRoot, tables)
            else
              -- 現在のテーブルに追加
              let table = Map.findWithDefault Map.empty currentTable tables
                  updatedTable = insertNested table path value
                  newTables = Map.insert currentTable updatedTable tables
               in (currentTable, root, newTables)

-- | ネストしたキーパスに値を挿入する補助関数。
insertNested :: TomlTable -> [Text] -> TomlValue -> TomlTable
insertNested table [] _ = table
insertNested table [key] value = Map.insert key value table
insertNested table (key : rest) value =
  let nested = case Map.lookup key table of
        Just (TomlInlineTable t) -> t
        _ -> Map.empty
      updatedNested = insertNested nested rest value
   in Map.insert key (TomlInlineTable updatedNested) table

-- | パブリックAPI：TOML文字列をパース。
parseToml :: Text -> Either (ParseErrorBundle Text Void) TomlDocument
parseToml = parse parseDocument "toml"

-- | レンダリング（検証用）。
renderTomlDocument :: TomlDocument -> Text
renderTomlDocument doc =
  let rootOutput = renderTable (docRoot doc) []
      tableOutput = T.concat
        [ T.concat ["\n[", T.intercalate "." path, "]\n", renderTable table []]
        | (path, table) <- Map.toList (docTables doc)
        ]
   in rootOutput <> tableOutput

renderTable :: TomlTable -> [Text] -> Text
renderTable table prefix =
  T.concat
    [ case value of
        TomlInlineTable nested -> renderTable nested (prefix ++ [key])
        _ -> fullKey <> " = " <> renderValue value <> "\n"
    | (key, value) <- Map.toList table,
      let fullKey = if null prefix then key else T.intercalate "." (prefix ++ [key])
    ]

renderValue :: TomlValue -> Text
renderValue (TomlString s) = "\"" <> s <> "\""
renderValue (TomlInteger n) = T.pack (show n)
renderValue (TomlFloat f) = T.pack (show f)
renderValue (TomlBoolean True) = "true"
renderValue (TomlBoolean False) = "false"
renderValue (TomlArray items) =
  "[" <> T.intercalate ", " (map renderValue items) <> "]"
renderValue (TomlInlineTable entries) =
  "{ " <> T.intercalate ", " [k <> " = " <> renderValue v | (k, v) <- Map.toList entries] <> " }"

-- | テスト例。
testExample :: IO ()
testExample = do
  let exampleToml = T.unlines
        [ "# Reml パッケージ設定",
          "",
          "[package]",
          "name = \"my_project\"",
          "version = \"0.1.0\"",
          "authors = [\"Author Name\"]",
          "",
          "[dependencies]",
          "core = \"1.0\"",
          "",
          "[dev-dependencies]",
          "test_framework = \"0.5\"",
          "",
          "[[plugins]]",
          "name = \"system\"",
          "version = \"1.0\"",
          "",
          "[[plugins]]",
          "name = \"memory\"",
          "version = \"1.0\""
        ]

  putStrLn "--- reml.toml 風設定のパース ---"
  case parseToml exampleToml of
    Right doc -> do
      putStrLn "パース成功:"
      T.putStrLn $ renderTomlDocument doc
    Left err -> do
      putStrLn "パースエラー:"
      print err

main :: IO ()
main = testExample

{-
Haskellの特徴：

1. **Megaparsecによる高品質パーサー**
   - 豊富なコンビネーター
   - 優れたエラーメッセージ
   - バックトラック制御が容易

2. **型安全性**
   - 代数的データ型による値の表現
   - パターンマッチによる網羅性チェック

3. **純粋関数型**
   - すべてのパーサーが純粋関数
   - 合成可能なコンビネーター

4. **モナディックスタイル**
   - do記法による読みやすいパーサー定義
   - エラー処理の自然な統合

Remlとの比較：
- Remlは言語組み込みのパーサーコンビネーターライブラリ
- Haskellは外部ライブラリ（Megaparsec）だが成熟度が高い
- 両者ともcut/commitによる精密なエラー制御が可能
- Remlの3層Unicode処理がより明示的
-}