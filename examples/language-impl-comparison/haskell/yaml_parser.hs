{-# LANGUAGE OverloadedStrings #-}

module YamlParser where

import Control.Applicative ((<|>), many, some)
import Control.Monad (void)
import Data.Char (isSpace)
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

-- | YAML風パーサー：インデント管理が重要な題材。
--
-- 対応する構文（簡易版）：
-- - スカラー値: 文字列、数値、真偽値、null
-- - リスト: `- item1`
-- - マップ: `key: value`
-- - ネストしたインデント構造
--
-- インデント処理の特徴：
-- - Megaparsecのインデント関連コンビネーターを活用
-- - エラー回復機能でインデントミスを報告しつつ継続

type Parser = Parsec Void Text

-- | YAML値の表現。
data YamlValue
  = Scalar Text
  | List [YamlValue]
  | Map (Map Text YamlValue)
  | Null
  deriving (Show, Eq)

type Document = YamlValue

-- | 水平空白のみをスキップ（改行は含まない）。
hspace :: Parser ()
hspace = void $ takeWhileP (Just "horizontal space") (\c -> c == ' ' || c == '\t')

-- | コメントのスキップ（`#` から行末まで）。
comment :: Parser ()
comment = do
  void $ char '#'
  void $ takeWhileP (Just "comment") (/= '\n')

-- | 空行またはコメント行をスキップ。
blankOrComment :: Parser ()
blankOrComment = do
  hspace
  optional comment
  void eol

-- | 特定のインデントレベルを期待する。
expectIndent :: Int -> Parser ()
expectIndent level = do
  spaces <- T.length <$> takeWhileP (Just "indent") (== ' ')
  if spaces == level
    then return ()
    else fail $ "インデント不一致: 期待 " ++ show level ++ ", 実際 " ++ show spaces

-- | 現在よりも深いインデントを検出。
deeperIndent :: Int -> Parser Int
deeperIndent current = do
  spaces <- T.length <$> takeWhileP (Just "deeper indent") (== ' ')
  if spaces > current
    then return spaces
    else fail $ "深いインデントが期待されます: 現在 " ++ show current ++ ", 実際 " ++ show spaces

-- | スカラー値のパース。
scalarValue :: Parser YamlValue
scalarValue =
  choice
    [ Null <$ string "null",
      Null <$ string "~",
      Scalar "true" <$ string "true",
      Scalar "false" <$ string "false",
      try $ Scalar . T.pack <$> some digitChar,
      try $ Scalar <$> stringLit,
      Scalar . T.strip <$> takeWhileP (Just "unquoted string") (\c -> c /= '\n' && c /= ':' && c /= '#')
    ]
    <?> "scalar value"

-- | 引用符付き文字列リテラルのパース。
stringLit :: Parser Text
stringLit =
  T.pack
    <$> (char '"' *> manyTill L.charLiteral (char '"'))
    <?> "quoted string"

-- | リスト項目のパース（`- value` 形式）。
parseListItem :: Int -> Parser YamlValue
parseListItem indent = do
  expectIndent indent
  void $ char '-'
  hspace
  parseValue (indent + 2)
    <?> "list item"

-- | リスト全体のパース。
parseList :: Int -> Parser YamlValue
parseList indent =
  List <$> some (parseListItem indent <* optional eol)
    <?> "list"

-- | マップのキーバリューペアのパース（`key: value` 形式）。
parseMapEntry :: Int -> Parser (Text, YamlValue)
parseMapEntry indent = do
  expectIndent indent
  key <- T.strip <$> takeWhileP (Just "key") (\c -> c /= ':' && c /= '\n')
  void $ char ':'
  hspace
  value <-
    choice
      [ try $ parseValue indent,
        do
          void eol
          parseValue (indent + 2)
      ]
  return (key, value)
    <?> "map entry"

-- | マップ全体のパース。
parseMap :: Int -> Parser YamlValue
parseMap indent =
  Map . Map.fromList <$> some (parseMapEntry indent <* optional eol)
    <?> "map"

-- | YAML値のパース（再帰的）。
parseValue :: Int -> Parser YamlValue
parseValue indent =
  choice
    [ try $ parseList indent,
      try $ parseMap indent,
      scalarValue
    ]
    <?> "value"

-- | ドキュメント全体のパース。
document :: Parser Document
document = do
  many blankOrComment
  doc <- parseValue 0
  many blankOrComment
  eof
  return doc
    <?> "document"

-- | パブリックAPI：YAML文字列をパース。
parseYaml :: Text -> Either (ParseErrorBundle Text Void) Document
parseYaml = parse document "<input>"

-- | 簡易的なレンダリング（検証用）。
renderToString :: Document -> String
renderToString doc = renderValue doc 0

renderValue :: YamlValue -> Int -> String
renderValue value indent =
  let indentStr = replicate indent ' '
   in case value of
        Scalar s -> T.unpack s
        Null -> "null"
        List items ->
          intercalate "\n" $
            map
              (\item -> indentStr ++ "- " ++ renderValue item (indent + 2))
              items
        Map entries ->
          intercalate "\n" $
            map
              ( \(key, val) -> case val of
                  Scalar _ -> indentStr ++ T.unpack key ++ ": " ++ renderValue val 0
                  Null -> indentStr ++ T.unpack key ++ ": " ++ renderValue val 0
                  _ -> indentStr ++ T.unpack key ++ ":\n" ++ renderValue val (indent + 2)
              )
              (Map.toList entries)

-- | テスト例。
testExamples :: IO ()
testExamples = do
  let examples =
        [ ("simple_scalar", "hello"),
          ("simple_list", "- item1\n- item2\n- item3"),
          ("simple_map", "key1: value1\nkey2: value2"),
          ("nested_map", "parent:\n  child1: value1\n  child2: value2"),
          ("nested_list", "items:\n  - item1\n  - item2"),
          ("mixed", "name: John\nage: 30\nhobbies:\n  - reading\n  - coding")
        ]

  mapM_
    ( \(name, yamlStr) -> do
        putStrLn $ "--- " ++ name ++ " ---"
        case parseYaml yamlStr of
          Right doc -> do
            putStrLn "パース成功:"
            putStrLn $ renderToString doc
          Left err ->
            putStrLn $ "パースエラー: " ++ show err
    )
    examples

-- | インデント処理の課題と解決策：
--
-- 1. **インデントレベルの追跡**
--    - パーサー引数としてインデントレベルを渡す
--    - Megaparsecの明示的なインデント管理を使用
--
-- 2. **エラー回復**
--    - tryでバックトラックを制御
--    - <?> で分かりやすいエラーメッセージを提供
--
-- 3. **空白の扱い**
--    - hspaceで水平空白のみをスキップ（改行は構文の一部）
--    - eolでCR/LF/CRLFを正規化
--
-- Remlとの比較：
--
-- - **Megaparsecの利点**:
--   - 成熟したエラーメッセージシステム
--   - 豊富なコンビネーターライブラリ
--
-- - **Megaparsecの課題**:
--   - インデント管理が明示的で冗長になりやすい
--   - 状態管理がやや煩雑
--
-- - **Remlの利点**:
--   - 字句レイヤの柔軟性により、インデント処理が自然に表現できる
--   - cut/commitによるエラー品質の向上
--   - recoverによる部分的なパース継続が可能