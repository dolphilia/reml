-- 正規表現エンジン：パース + 評価の両方を実装。
--
-- 対応する正規表現構文（簡易版）：
-- - リテラル: `abc`
-- - 連結: `ab`
-- - 選択: `a|b`
-- - 繰り返し: `a*`, `a+`, `a?`, `a{2,5}`
-- - グループ: `(abc)`
-- - 文字クラス: `[a-z]`, `[^0-9]`, `\d`, `\w`, `\s`
-- - アンカー: `^`, `$`
-- - ドット: `.` (任意の1文字)

import Control.Applicative (Alternative(..), (<|>))
import Control.Monad (void)
import Data.Char (isDigit, isAlphaNum, isSpace)
import Data.List (find)

-- 正規表現のAST
data Regex
    = Literal String
    | CharClass CharSet
    | Dot
    | Concat [Regex]
    | Alternation [Regex]
    | Repeat Regex RepeatKind
    | Group Regex
    | Anchor AnchorKind
    deriving (Show, Eq)

data CharSet
    = CharRange Char Char
    | CharList [Char]
    | Predefined PredefinedClass
    | Negated CharSet
    | Union [CharSet]
    deriving (Show, Eq)

data PredefinedClass
    = Digit
    | Word
    | Whitespace
    | NotDigit
    | NotWord
    | NotWhitespace
    deriving (Show, Eq)

data RepeatKind
    = ZeroOrMore
    | OneOrMore
    | ZeroOrOne
    | Exactly Int
    | Range Int (Maybe Int)
    deriving (Show, Eq)

data AnchorKind
    = Start
    | End
    deriving (Show, Eq)

-- パーサー型
newtype Parser a = Parser { runParser :: String -> Maybe (a, String) }

instance Functor Parser where
    fmap f (Parser p) = Parser $ \input ->
        case p input of
            Nothing -> Nothing
            Just (value, rest) -> Just (f value, rest)

instance Applicative Parser where
    pure value = Parser $ \input -> Just (value, input)
    Parser pf <*> Parser px = Parser $ \input ->
        case pf input of
            Nothing -> Nothing
            Just (f, rest1) ->
                case px rest1 of
                    Nothing -> Nothing
                    Just (x, rest2) -> Just (f x, rest2)

instance Monad Parser where
    return = pure
    Parser p >>= f = Parser $ \input ->
        case p input of
            Nothing -> Nothing
            Just (value, rest) -> runParser (f value) rest

instance Alternative Parser where
    empty = Parser $ const Nothing
    Parser p1 <|> Parser p2 = Parser $ \input ->
        case p1 input of
            Just result -> Just result
            Nothing -> p2 input

-- パーサーコンビネーター
satisfy :: (Char -> Bool) -> Parser Char
satisfy pred = Parser $ \input ->
    case input of
        (c:rest) | pred c -> Just (c, rest)
        _ -> Nothing

char :: Char -> Parser Char
char c = satisfy (== c)

string :: String -> Parser String
string "" = pure ""
string (c:cs) = (:) <$> char c <*> string cs

many1 :: Parser a -> Parser [a]
many1 p = (:) <$> p <*> many p

optional :: Parser a -> Parser (Maybe a)
optional p = (Just <$> p) <|> pure Nothing

digit :: Parser Char
digit = satisfy isDigit

integer :: Parser Int
integer = read <$> many1 digit

sepBy1 :: Parser a -> Parser sep -> Parser [a]
sepBy1 p sep = (:) <$> p <*> many (sep *> p)

-- 正規表現パーサー
parseRegex :: String -> Maybe Regex
parseRegex input =
    case runParser regexExpr input of
        Just (regex, "") -> Just regex
        _ -> Nothing

regexExpr :: Parser Regex
regexExpr = alternationExpr

alternationExpr :: Parser Regex
alternationExpr = do
    alts <- sepBy1 concatExpr (string "|")
    pure $ case alts of
        [single] -> single
        _ -> Alternation alts

concatExpr :: Parser Regex
concatExpr = do
    terms <- many1 postfixTerm
    pure $ case terms of
        [single] -> single
        _ -> Concat terms

postfixTerm :: Parser Regex
postfixTerm = do
    base <- atom
    repeatOpt <- optional repeatSuffix
    pure $ case repeatOpt of
        Just kind -> Repeat base kind
        Nothing -> base

atom :: Parser Regex
atom = groupParser
    <|> anchorStart
    <|> anchorEnd
    <|> dotParser
    <|> charClassParser
    <|> predefinedClassParser
    <|> escapeCharParser
    <|> literalParser

groupParser :: Parser Regex
groupParser = do
    void $ string "("
    inner <- regexExpr
    void $ string ")"
    pure $ Group inner

anchorStart :: Parser Regex
anchorStart = string "^" *> pure (Anchor Start)

anchorEnd :: Parser Regex
anchorEnd = string "$" *> pure (Anchor End)

dotParser :: Parser Regex
dotParser = string "." *> pure Dot

escapeCharParser :: Parser Regex
escapeCharParser = do
    void $ string "\\"
    c <- satisfy (`elem` "ntr\\()[]{}*+?.|^$")
    pure $ Literal [case c of
        'n' -> '\n'
        't' -> '\t'
        'r' -> '\r'
        _ -> c]

predefinedClassParser :: Parser Regex
predefinedClassParser = do
    void $ string "\\"
    cls <- (char 'd' *> pure Digit)
        <|> (char 'w' *> pure Word)
        <|> (char 's' *> pure Whitespace)
        <|> (char 'D' *> pure NotDigit)
        <|> (char 'W' *> pure NotWord)
        <|> (char 'S' *> pure NotWhitespace)
    pure $ CharClass (Predefined cls)

charClassParser :: Parser Regex
charClassParser = do
    void $ string "["
    negated <- optional (string "^")
    items <- many1 charClassItem
    void $ string "]"
    let unionSet = Union items
    pure $ CharClass $ case negated of
        Just _ -> Negated unionSet
        Nothing -> unionSet

charClassItem :: Parser CharSet
charClassItem = rangeParser <|> singleCharParser
  where
    rangeParser = do
        start <- satisfy (\c -> c /= ']' && c /= '-')
        endOpt <- optional (string "-" *> satisfy (/= ']'))
        pure $ case endOpt of
            Just end -> CharRange start end
            Nothing -> CharList [start]
    singleCharParser = CharList . (:[]) <$> satisfy (/= ']')

literalParser :: Parser Regex
literalParser = do
    c <- satisfy (\ch -> ch `notElem` "()[]{}*+?.|^$\\")
    pure $ Literal [c]

repeatSuffix :: Parser RepeatKind
repeatSuffix =
    (string "*" *> pure ZeroOrMore)
    <|> (string "+" *> pure OneOrMore)
    <|> (string "?" *> pure ZeroOrOne)
    <|> bracedRepeat

bracedRepeat :: Parser RepeatKind
bracedRepeat = do
    void $ string "{"
    n <- integer
    rangeOpt <- optional $ do
        void $ string ","
        optional integer
    void $ string "}"
    pure $ case rangeOpt of
        Nothing -> Exactly n
        Just Nothing -> Range n Nothing
        Just (Just m) -> Range n (Just m)

-- マッチングエンジン
matchRegex :: Regex -> String -> Bool
matchRegex regex text = matchFromPos regex text 0

matchFromPos :: Regex -> String -> Int -> Bool
matchFromPos regex text pos =
    case regex of
        Literal s ->
            take (length s) (drop pos text) == s

        CharClass cs ->
            case drop pos text of
                (c:_) -> charMatchesClass c cs
                _ -> False

        Dot ->
            pos < length text

        Concat terms ->
            let go [] currentPos = True
                go (term:rest) currentPos =
                    if matchFromPos term text currentPos
                    then go rest (currentPos + 1)
                    else False
            in go terms pos

        Alternation alts ->
            any (\alt -> matchFromPos alt text pos) alts

        Repeat inner kind ->
            case kind of
                ZeroOrMore -> matchRepeatZeroOrMore inner text pos
                OneOrMore -> matchRepeatOneOrMore inner text pos
                ZeroOrOne -> matchRepeatZeroOrOne inner text pos
                Exactly n -> matchRepeatExactly inner text pos n
                Range minCount maxOpt -> matchRepeatRange inner text pos minCount maxOpt

        Group inner ->
            matchFromPos inner text pos

        Anchor kind ->
            case kind of
                Start -> pos == 0
                End -> pos >= length text

charMatchesClass :: Char -> CharSet -> Bool
charMatchesClass ch cs =
    case cs of
        CharRange start end ->
            ch >= start && ch <= end

        CharList chars ->
            ch `elem` chars

        Predefined cls ->
            case cls of
                Digit -> isDigit ch
                Word -> isAlphaNum ch || ch == '_'
                Whitespace -> isSpace ch
                NotDigit -> not (isDigit ch)
                NotWord -> not (isAlphaNum ch || ch == '_')
                NotWhitespace -> not (isSpace ch)

        Negated inner ->
            not (charMatchesClass ch inner)

        Union sets ->
            any (charMatchesClass ch) sets

matchRepeatZeroOrMore :: Regex -> String -> Int -> Bool
matchRepeatZeroOrMore inner text pos =
    matchRepeatLoop inner text pos 0 0 999999

matchRepeatOneOrMore :: Regex -> String -> Int -> Bool
matchRepeatOneOrMore inner text pos =
    if matchFromPos inner text pos
    then matchRepeatZeroOrMore inner text (pos + 1)
    else False

matchRepeatZeroOrOne :: Regex -> String -> Int -> Bool
matchRepeatZeroOrOne inner text pos =
    matchFromPos inner text pos || True

matchRepeatExactly :: Regex -> String -> Int -> Int -> Bool
matchRepeatExactly inner text pos n =
    matchRepeatLoop inner text pos 0 n n

matchRepeatRange :: Regex -> String -> Int -> Int -> Maybe Int -> Bool
matchRepeatRange inner text pos minCount maxOpt =
    let maxCount = maybe 999999 id maxOpt
    in matchRepeatLoop inner text pos 0 minCount maxCount

matchRepeatLoop :: Regex -> String -> Int -> Int -> Int -> Int -> Bool
matchRepeatLoop inner text pos count minCount maxCount
    | count == maxCount = True
    | count >= minCount && not (matchFromPos inner text pos) = True
    | matchFromPos inner text pos = matchRepeatLoop inner text (pos + 1) (count + 1) minCount maxCount
    | count >= minCount = True
    | otherwise = False

-- テスト例
testExamples :: IO ()
testExamples = do
    let examples =
            [ ("a+", "aaa", True)
            , ("a+", "b", False)
            , ("[0-9]+", "123", True)
            , ("[0-9]+", "abc", False)
            , ("\\d{2,4}", "12", True)
            , ("\\d{2,4}", "12345", True)
            , ("(abc)+", "abcabc", True)
            , ("a|b", "a", True)
            , ("a|b", "b", True)
            , ("a|b", "c", False)
            , ("^hello$", "hello", True)
            , ("^hello$", "hello world", False)
            ]

    mapM_ testOne examples
  where
    testOne (pattern, text, expected) =
        case parseRegex pattern of
            Just regex -> do
                let result = matchRegex regex text
                    status = if result == expected then "✓" else "✗"
                putStrLn $ status ++ " パターン: '" ++ pattern ++ "', テキスト: '" ++ text ++ "', 期待: " ++ show expected ++ ", 結果: " ++ show result
            Nothing ->
                putStrLn $ "✗ パーサーエラー: " ++ pattern

-- Main
main :: IO ()
main = testExamples