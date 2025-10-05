{-# LANGUAGE OverloadedStrings #-}

-- Markdown風軽量マークアップパーサー - Haskell実装
--
-- Unicode処理の注意点：
-- - HaskellのStringは[Char]でUnicodeコードポイントのリスト
-- - Textはより効率的だがコードポイント単位の操作
-- - Grapheme（書記素クラスター）処理にはtext-icu等の外部ライブラリが必要
-- - Remlの3層モデルと比較すると、Haskellも明示的な区別が必要

module MarkdownParser
  ( Block(..)
  , Inline(..)
  , Document
  , parseMarkdown
  , renderToString
  ) where

import Data.Text (Text)
import qualified Data.Text as T
import Control.Applicative ((<|>))
import Data.Char (isSpace)
import Data.Maybe (fromMaybe)

-- | Markdown AST のブロック要素
data Block
  = Heading { level :: Int, inline :: [Inline] }
  | Paragraph { inline :: [Inline] }
  | UnorderedList { items :: [[Inline]] }
  | OrderedList { items :: [[Inline]] }
  | CodeBlock { lang :: Maybe Text, code :: Text }
  | HorizontalRule
  deriving (Show, Eq)

-- | Markdown AST のインライン要素
data Inline
  = Text Text
  | Strong [Inline]
  | Emphasis [Inline]
  | Code Text
  | Link { text :: [Inline], url :: Text }
  | LineBreak
  deriving (Show, Eq)

type Document = [Block]

-- | パーサー状態
--
-- Unicode処理の課題：
-- - Textはコードポイント単位だが、Grapheme（書記素クラスター）ではない
-- - 「🇯🇵」のような国旗絵文字は2つのコードポイント
-- - 正確な表示幅計算にはtext-icuやunicode-data等が必要
data ParseState = ParseState
  { input :: Text
  , position :: Int  -- コードポイント単位のインデックス（Textの制約）
  }

newtype Parser a = Parser { runParser :: ParseState -> Either String (a, ParseState) }

instance Functor Parser where
  fmap f (Parser p) = Parser $ \s -> case p s of
    Left err -> Left err
    Right (x, s') -> Right (f x, s')

instance Applicative Parser where
  pure x = Parser $ \s -> Right (x, s)
  (Parser pf) <*> (Parser px) = Parser $ \s -> case pf s of
    Left err -> Left err
    Right (f, s') -> case px s' of
      Left err -> Left err
      Right (x, s'') -> Right (f x, s'')

instance Monad Parser where
  return = pure
  (Parser p) >>= f = Parser $ \s -> case p s of
    Left err -> Left err
    Right (x, s') -> runParser (f x) s'

-- | パーサーの選択（左優先）
(<||>) :: Parser a -> Parser a -> Parser a
(Parser p1) <||> (Parser p2) = Parser $ \s -> case p1 s of
  Left _ -> p2 s
  Right res -> Right res

-- | パーサーの失敗
failParser :: String -> Parser a
failParser msg = Parser $ \_ -> Left msg

-- | 現在位置の1文字を取得
--
-- 注意：HaskellのCharはUnicodeコードポイントだが、Graphemeではない
peekChar :: Parser (Maybe Char)
peekChar = Parser $ \s ->
  let remaining = T.drop (position s) (input s)
  in Right (if T.null remaining then Nothing else Just (T.head remaining), s)

-- | 1文字を消費
advanceChar :: Parser ()
advanceChar = Parser $ \s ->
  Right ((), s { position = position s + 1 })

-- | 固定文字列をマッチ
matchString :: Text -> Parser Bool
matchString target = Parser $ \s ->
  let remaining = T.drop (position s) (input s)
  in if T.isPrefixOf target remaining
     then Right (True, s { position = position s + T.length target })
     else Right (False, s)

-- | 水平空白をスキップ
skipHSpace :: Parser ()
skipHSpace = do
  mc <- peekChar
  case mc of
    Just c | c == ' ' || c == '\t' -> advanceChar >> skipHSpace
    _ -> return ()

-- | 行末まで読む
readUntilEol :: Parser Text
readUntilEol = Parser $ \s ->
  let remaining = T.drop (position s) (input s)
      (line, _) = T.break (== '\n') remaining
  in Right (line, s { position = position s + T.length line })

-- | 改行を消費
consumeNewline :: Parser Bool
consumeNewline = do
  mc <- peekChar
  case mc of
    Just '\n' -> advanceChar >> return True
    _ -> return False

-- | EOFチェック
isEof :: Parser Bool
isEof = Parser $ \s ->
  Right (position s >= T.length (input s), s)

-- | 見出し行のパース（`# Heading` 形式）
parseHeading :: Parser Block
parseHeading = do
  skipHSpace

  -- `#` の連続をカウント
  lvl <- countHashes 0

  if lvl == 0 || lvl > 6
    then failParser "見出しレベルは1-6の範囲内である必要があります"
    else do
      skipHSpace
      txt <- readUntilEol
      _ <- consumeNewline
      return $ Heading lvl [Text (T.strip txt)]
  where
    countHashes n = do
      mc <- peekChar
      case mc of
        Just '#' -> advanceChar >> countHashes (n + 1)
        _ -> return n

-- | 水平線のパース（`---`, `***`, `___`）
parseHorizontalRule :: Parser Block
parseHorizontalRule = do
  skipHSpace
  txt <- readUntilEol
  _ <- consumeNewline

  let trimmed = T.strip txt
      isRule = (T.all (== '-') trimmed && T.length trimmed >= 3)
            || (T.all (== '*') trimmed && T.length trimmed >= 3)
            || (T.all (== '_') trimmed && T.length trimmed >= 3)

  if isRule
    then return HorizontalRule
    else failParser "水平線として認識できません"

-- | コードブロックのパース（```言語名）
parseCodeBlock :: Parser Block
parseCodeBlock = do
  matched <- matchString "```"
  if not matched
    then failParser "コードブロック開始が見つかりません"
    else do
      langLine <- readUntilEol
      _ <- consumeNewline

      let language = let trimmed = T.strip langLine
                     in if T.null trimmed then Nothing else Just trimmed

      codeLines <- readCodeLines []
      _ <- consumeNewline

      return $ CodeBlock language (T.intercalate "\n" codeLines)
  where
    readCodeLines acc = do
      matched <- matchString "```"
      if matched
        then return (reverse acc)
        else do
          eof <- isEof
          if eof
            then return (reverse acc)
            else do
              line <- readUntilEol
              _ <- consumeNewline
              readCodeLines (line : acc)

-- | リスト項目のパース（簡易版：`-` または `*`）
parseUnorderedList :: Parser Block
parseUnorderedList = do
  items <- parseItems []
  if null items
    then failParser "リスト項目が見つかりません"
    else return $ UnorderedList items
  where
    parseItems acc = do
      skipHSpace
      mc <- peekChar
      case mc of
        Just c | c == '-' || c == '*' -> do
          advanceChar
          skipHSpace
          txt <- readUntilEol
          _ <- consumeNewline
          parseItems ([Text (T.strip txt)] : acc)
        _ -> return (reverse acc)

-- | 段落のパース（簡易版：空行まで）
parseParagraph :: Parser Block
parseParagraph = do
  lines <- readLines []
  let txt = T.intercalate " " lines
  return $ Paragraph [Text (T.strip txt)]
  where
    readLines acc = do
      eof <- isEof
      if eof
        then return (reverse acc)
        else do
          mc <- peekChar
          case mc of
            Just '\n' -> do
              advanceChar
              mc2 <- peekChar
              case mc2 of
                Just '\n' -> return (reverse acc)
                _ -> readLines ("" : acc)
            _ -> do
              line <- readUntilEol
              _ <- consumeNewline
              readLines (line : acc)

-- | ブロック要素のパース（優先順位付き試行）
parseBlock :: Parser Block
parseBlock = do
  skipBlankLines
  eof <- isEof
  if eof
    then failParser "EOF"
    else do
      skipHSpace
      mc <- peekChar
      case mc of
        Just '#' -> parseHeading
        Just '`' -> do
          matched <- matchString "```"
          if matched
            then parseCodeBlock
            else parseParagraph
        Just c | c == '-' || c == '*' || c == '_' ->
          parseHorizontalRule <||> parseUnorderedList
        _ -> parseParagraph
  where
    skipBlankLines = do
      mc <- peekChar
      case mc of
        Just '\n' -> advanceChar >> skipBlankLines
        _ -> return ()

-- | ドキュメント全体のパース
parseDocument :: Parser Document
parseDocument = parseBlocks []
  where
    parseBlocks acc = do
      result <- (Just <$> parseBlock) <||> return Nothing
      case result of
        Just block -> parseBlocks (block : acc)
        Nothing -> return (reverse acc)

-- | パブリックAPI：文字列からドキュメントをパース
parseMarkdown :: Text -> Either String Document
parseMarkdown input =
  let initialState = ParseState input 0
  in case runParser parseDocument initialState of
    Left err -> Left err
    Right (doc, _) -> Right doc

-- | 簡易的なレンダリング（検証用）
renderToString :: Document -> Text
renderToString doc = T.concat $ map renderBlock doc
  where
    renderInline :: [Inline] -> Text
    renderInline inlines = T.concat $ map renderInlineElem inlines

    renderInlineElem :: Inline -> Text
    renderInlineElem (Text t) = t
    renderInlineElem (Strong inner) = "**" <> renderInline inner <> "**"
    renderInlineElem (Emphasis inner) = "*" <> renderInline inner <> "*"
    renderInlineElem (Code t) = "`" <> t <> "`"
    renderInlineElem (Link txt url) = "[" <> renderInline txt <> "](" <> url <> ")"
    renderInlineElem LineBreak = "\n"

    renderBlock :: Block -> Text
    renderBlock (Heading lvl inlines) =
      T.replicate lvl "#" <> " " <> renderInline inlines <> "\n\n"
    renderBlock (Paragraph inlines) =
      renderInline inlines <> "\n\n"
    renderBlock (UnorderedList items) =
      T.concat (map (\item -> "- " <> renderInline item <> "\n") items) <> "\n"
    renderBlock (OrderedList items) =
      T.concat (zipWith (\i item -> T.pack (show i) <> ". " <> renderInline item <> "\n")
                        [1..] items) <> "\n"
    renderBlock (CodeBlock mLang codeText) =
      let langStr = fromMaybe "" mLang
      in "```" <> langStr <> "\n" <> codeText <> "\n```\n\n"
    renderBlock HorizontalRule = "---\n\n"

-- Unicode 3層モデル比較：
--
-- Haskellでは String = [Char] で Char はUnicodeコードポイント
-- Textはより効率的だが、やはりコードポイント単位の操作
-- Grapheme（書記素クラスター）処理には text-icu 等の外部ライブラリが必要：
--
-- import Data.Text.ICU (normalize, NormalizationMode(..))
-- import Data.Text.ICU.Break (breakCharacter)
--
-- countGraphemes :: Text -> Int
-- countGraphemes = length . breakCharacter (...)
--
-- countCodepoints :: Text -> Int
-- countCodepoints = T.length
--
-- byteLength :: Text -> Int
-- byteLength = BS.length . encodeUtf8
--
-- この明示性の欠如が、絵文字や結合文字の扱いでバグを生む可能性がある。