{-# LANGUAGE LambdaCase #-}

-- 簡易SQL Parser
-- SELECT, WHERE, JOIN, ORDER BY など基本的な構文のみ対応

module Main where

import Control.Applicative
import Control.Monad
import Data.Char (isAlphaNum, isDigit, isLetter, isSpace, toLower)
import Data.List (intercalate)

-- AST定義

data Query = SelectQuery
  { columns :: [Column]
  , from :: TableRef
  , whereClause :: Maybe Expr
  , joins :: [Join]
  , orderBy :: Maybe [(Expr, OrderDirection)]
  }
  deriving (Show)

data Column
  = AllColumns
  | ColumnExpr Expr (Maybe String)
  deriving (Show)

data TableRef = TableRef
  { table :: String
  , tableAlias :: Maybe String
  }
  deriving (Show)

data Join = Join
  { joinType :: JoinType
  , joinTable :: TableRef
  , onCondition :: Expr
  }
  deriving (Show)

data JoinType
  = InnerJoin
  | LeftJoin
  | RightJoin
  | FullJoin
  deriving (Show)

data OrderDirection
  = Asc
  | Desc
  deriving (Show, Eq)

data Expr
  = LiteralExpr Literal
  | ColumnRef String
  | QualifiedColumn String String
  | BinaryOp BinOp Expr Expr
  | UnaryOp UnOp Expr
  | FunctionCall String [Expr]
  | Parenthesized Expr
  deriving (Show)

data Literal
  = IntLit Int
  | StringLit String
  | BoolLit Bool
  | NullLit
  deriving (Show)

data BinOp
  = Add | Sub | Mul | Div | Mod
  | Eq | Ne | Lt | Le | Gt | Ge
  | And | Or | Like
  deriving (Show)

data UnOp
  = Not
  | IsNull
  | IsNotNull
  deriving (Show)

-- パーサーコンビネーター

newtype Parser a = Parser { runParser :: String -> Maybe (a, String) }

instance Functor Parser where
  fmap f (Parser p) = Parser $ \input ->
    case p input of
      Just (result, rest) -> Just (f result, rest)
      Nothing -> Nothing

instance Applicative Parser where
  pure x = Parser $ \input -> Just (x, input)
  Parser pf <*> Parser px = Parser $ \input ->
    case pf input of
      Just (f, rest) ->
        case px rest of
          Just (x, rest') -> Just (f x, rest')
          Nothing -> Nothing
      Nothing -> Nothing

instance Monad Parser where
  return = pure
  Parser p >>= f = Parser $ \input ->
    case p input of
      Just (result, rest) -> runParser (f result) rest
      Nothing -> Nothing

instance Alternative Parser where
  empty = Parser $ const Nothing
  Parser p1 <|> Parser p2 = Parser $ \input ->
    case p1 input of
      Just result -> Just result
      Nothing -> p2 input

satisfy :: (Char -> Bool) -> Parser Char
satisfy pred = Parser $ \case
  (c:cs) | pred c -> Just (c, cs)
  _ -> Nothing

char :: Char -> Parser Char
char c = satisfy (== c)

string :: String -> Parser String
string str = Parser $ \input ->
  if take (length str) input == str
    then Just (str, drop (length str) input)
    else Nothing

skipWhitespace :: Parser ()
skipWhitespace = Parser $ \input ->
  Just ((), dropWhile isSpace input)

lexeme :: Parser a -> Parser a
lexeme p = skipWhitespace *> p <* skipWhitespace

symbol :: String -> Parser ()
symbol str = lexeme (string str) *> pure ()

keyword :: String -> Parser ()
keyword kw = lexeme $ do
  skipWhitespace
  _ <- Parser $ \input ->
    let len = length kw
        slice = take len input
    in if map toLower slice == map toLower kw
       then let rest = drop len input
            in case rest of
                 (c:_) | isAlphaNum c -> Nothing
                 _ -> Just ((), rest)
       else Nothing
  skipWhitespace
  pure ()

reservedWords :: [String]
reservedWords =
  [ "select", "from", "where", "join", "inner", "left", "right", "full"
  , "on", "and", "or", "not", "like", "order", "by", "asc", "desc"
  , "null", "true", "false", "as", "is"
  ]

identifier :: Parser String
identifier = lexeme $ do
  skipWhitespace
  first <- satisfy (\c -> isLetter c || c == '_')
  rest <- many (satisfy (\c -> isAlphaNum c || c == '_'))
  let name = first : rest
  if map toLower name `elem` reservedWords
    then empty
    else do
      skipWhitespace
      pure name

integer :: Parser Int
integer = lexeme $ do
  skipWhitespace
  digits <- some (satisfy isDigit)
  skipWhitespace
  pure (read digits)

stringLiteral :: Parser String
stringLiteral = lexeme $ do
  skipWhitespace
  _ <- char '\''
  content <- many (satisfy (/= '\''))
  _ <- char '\''
  skipWhitespace
  pure content

literalParser :: Parser Literal
literalParser =
  (keyword "null" *> pure NullLit)
    <|> (keyword "true" *> pure (BoolLit True))
    <|> (keyword "false" *> pure (BoolLit False))
    <|> (IntLit <$> integer)
    <|> (StringLit <$> stringLiteral)

-- 式パーサー（演算子優先度対応）

exprParser :: Parser Expr
exprParser = orExpr

orExpr :: Parser Expr
orExpr = do
  left <- andExpr
  orExprCont left

orExprCont :: Expr -> Parser Expr
orExprCont left =
  (do
      keyword "or"
      right <- andExpr
      orExprCont (BinaryOp Or left right))
    <|> pure left

andExpr :: Parser Expr
andExpr = do
  left <- comparisonExpr
  andExprCont left

andExprCont :: Expr -> Parser Expr
andExprCont left =
  (do
      keyword "and"
      right <- comparisonExpr
      andExprCont (BinaryOp And left right))
    <|> pure left

comparisonExpr :: Parser Expr
comparisonExpr = do
  left <- additiveExpr
  (symbol "<=" *> (BinaryOp Le left <$> additiveExpr))
    <|> (symbol ">=" *> (BinaryOp Ge left <$> additiveExpr))
    <|> (symbol "<>" *> (BinaryOp Ne left <$> additiveExpr))
    <|> (symbol "!=" *> (BinaryOp Ne left <$> additiveExpr))
    <|> (symbol "=" *> (BinaryOp Eq left <$> additiveExpr))
    <|> (symbol "<" *> (BinaryOp Lt left <$> additiveExpr))
    <|> (symbol ">" *> (BinaryOp Gt left <$> additiveExpr))
    <|> (keyword "like" *> (BinaryOp Like left <$> additiveExpr))
    <|> pure left

additiveExpr :: Parser Expr
additiveExpr = do
  left <- multiplicativeExpr
  additiveExprCont left

additiveExprCont :: Expr -> Parser Expr
additiveExprCont left =
  (do
      symbol "+"
      right <- multiplicativeExpr
      additiveExprCont (BinaryOp Add left right))
    <|> (do
            symbol "-"
            right <- multiplicativeExpr
            additiveExprCont (BinaryOp Sub left right))
    <|> pure left

multiplicativeExpr :: Parser Expr
multiplicativeExpr = do
  left <- postfixExpr
  multiplicativeExprCont left

multiplicativeExprCont :: Expr -> Parser Expr
multiplicativeExprCont left =
  (do
      symbol "*"
      right <- postfixExpr
      multiplicativeExprCont (BinaryOp Mul left right))
    <|> (do
            symbol "/"
            right <- postfixExpr
            multiplicativeExprCont (BinaryOp Div left right))
    <|> (do
            symbol "%"
            right <- postfixExpr
            multiplicativeExprCont (BinaryOp Mod left right))
    <|> pure left

postfixExpr :: Parser Expr
postfixExpr = do
  expr <- unaryExpr
  postfixExprCont expr

postfixExprCont :: Expr -> Parser Expr
postfixExprCont expr =
  (do
      keyword "is"
      (do
          keyword "not"
          keyword "null"
          postfixExprCont (UnaryOp IsNotNull expr))
        <|> (do
                keyword "null"
                postfixExprCont (UnaryOp IsNull expr)))
    <|> pure expr

unaryExpr :: Parser Expr
unaryExpr =
  (keyword "not" *> (UnaryOp Not <$> unaryExpr))
    <|> atomExpr

atomExpr :: Parser Expr
atomExpr =
  (do
      symbol "("
      expr <- exprParser
      symbol ")"
      pure (Parenthesized expr))
    <|> (do
            name <- identifier
            (do
                symbol "("
                args <- functionArgs
                symbol ")"
                pure (FunctionCall name args))
              <|> (do
                      symbol "."
                      col <- identifier
                      pure (QualifiedColumn name col))
              <|> pure (ColumnRef name))
    <|> (LiteralExpr <$> literalParser)

functionArgs :: Parser [Expr]
functionArgs =
  (do
      first <- exprParser
      rest <- many (symbol "," *> exprParser)
      pure (first : rest))
    <|> pure []

-- クエリパーサー

columnListParser :: Parser [Column]
columnListParser =
  (symbol "*" *> pure [AllColumns])
    <|> (do
            first <- columnExprParser
            rest <- many (symbol "," *> columnExprParser)
            pure (first : rest))

columnExprParser :: Parser Column
columnExprParser = do
  expr <- exprParser
  alias <- optional ((keyword "as" *> identifier) <|> identifier)
  pure (ColumnExpr expr alias)

tableRefParser :: Parser TableRef
tableRefParser = do
  tbl <- identifier
  alias <- optional ((keyword "as" *> identifier) <|> identifier)
  pure (TableRef tbl alias)

joinTypeParser :: Parser JoinType
joinTypeParser =
  (keyword "inner" *> keyword "join" *> pure InnerJoin)
    <|> (keyword "left" *> keyword "join" *> pure LeftJoin)
    <|> (keyword "right" *> keyword "join" *> pure RightJoin)
    <|> (keyword "full" *> keyword "join" *> pure FullJoin)
    <|> (keyword "join" *> pure InnerJoin)

joinParser :: Parser Join
joinParser = do
  jt <- joinTypeParser
  tbl <- tableRefParser
  keyword "on"
  condition <- exprParser
  pure (Join jt tbl condition)

orderByItemParser :: Parser (Expr, OrderDirection)
orderByItemParser = do
  expr <- exprParser
  dir <- (keyword "asc" *> pure Asc)
         <|> (keyword "desc" *> pure Desc)
         <|> pure Asc
  pure (expr, dir)

orderByParser :: Parser [(Expr, OrderDirection)]
orderByParser = do
  keyword "order"
  keyword "by"
  first <- orderByItemParser
  rest <- many (symbol "," *> orderByItemParser)
  pure (first : rest)

selectQueryParser :: Parser Query
selectQueryParser = do
  keyword "select"
  cols <- columnListParser
  keyword "from"
  tbl <- tableRefParser
  js <- many joinParser
  wc <- optional (keyword "where" *> exprParser)
  ob <- optional orderByParser
  pure (SelectQuery cols tbl wc js ob)

parseSQL :: String -> Maybe Query
parseSQL input =
  case runParser parser input of
    Just (result, rest)
      | all isSpace rest -> Just result
    _ -> Nothing
  where
    parser = skipWhitespace *> selectQueryParser <* skipWhitespace <* optional (symbol ";") <* skipWhitespace

-- レンダリング（検証用）

renderQuery :: Query -> String
renderQuery (SelectQuery cols tbl wc js ob) =
  "SELECT " ++ colsStr ++ " " ++ fromStr ++ " " ++ joinsStr ++ whereStr ++ orderStr
  where
    colsStr = intercalate ", " (map renderColumn cols)
    fromStr = "FROM " ++ table tbl ++ maybe "" (" AS " ++) (tableAlias tbl)
    joinsStr = unwords (map renderJoin js)
    whereStr = maybe "" (\e -> " WHERE " ++ renderExpr e) wc
    orderStr = maybe "" renderOrderBy ob

renderColumn :: Column -> String
renderColumn AllColumns = "*"
renderColumn (ColumnExpr expr alias) =
  renderExpr expr ++ maybe "" (" AS " ++) alias

renderJoin :: Join -> String
renderJoin (Join jt tbl cond) =
  joinTypeStr ++ " " ++ table tbl ++ " ON " ++ renderExpr cond
  where
    joinTypeStr = case jt of
      InnerJoin -> "INNER JOIN"
      LeftJoin -> "LEFT JOIN"
      RightJoin -> "RIGHT JOIN"
      FullJoin -> "FULL JOIN"

renderExpr :: Expr -> String
renderExpr (LiteralExpr lit) = renderLiteral lit
renderExpr (ColumnRef name) = name
renderExpr (QualifiedColumn tbl col) = tbl ++ "." ++ col
renderExpr (BinaryOp op left right) =
  "(" ++ renderExpr left ++ " " ++ renderBinOp op ++ " " ++ renderExpr right ++ ")"
renderExpr (UnaryOp Not e) = "NOT " ++ renderExpr e
renderExpr (UnaryOp IsNull e) = renderExpr e ++ " IS NULL"
renderExpr (UnaryOp IsNotNull e) = renderExpr e ++ " IS NOT NULL"
renderExpr (FunctionCall name args) =
  name ++ "(" ++ intercalate ", " (map renderExpr args) ++ ")"
renderExpr (Parenthesized e) = "(" ++ renderExpr e ++ ")"

renderLiteral :: Literal -> String
renderLiteral (IntLit n) = show n
renderLiteral (StringLit s) = "'" ++ s ++ "'"
renderLiteral (BoolLit b) = if b then "TRUE" else "FALSE"
renderLiteral NullLit = "NULL"

renderBinOp :: BinOp -> String
renderBinOp Add = "+"
renderBinOp Sub = "-"
renderBinOp Mul = "*"
renderBinOp Div = "/"
renderBinOp Mod = "%"
renderBinOp Eq = "="
renderBinOp Ne = "<>"
renderBinOp Lt = "<"
renderBinOp Le = "<="
renderBinOp Gt = ">"
renderBinOp Ge = ">="
renderBinOp And = "AND"
renderBinOp Or = "OR"
renderBinOp Like = "LIKE"

renderOrderBy :: [(Expr, OrderDirection)] -> String
renderOrderBy items =
  " ORDER BY " ++ intercalate ", " (map renderItem items)
  where
    renderItem (expr, dir) =
      renderExpr expr ++ " " ++ (if dir == Asc then "ASC" else "DESC")

-- テスト

main :: IO ()
main = do
  let testCases =
        [ "SELECT * FROM users"
        , "SELECT name, age FROM users WHERE age > 18"
        , "SELECT u.name, o.total FROM users u INNER JOIN orders o ON u.id = o.user_id"
        , "SELECT name FROM users WHERE active = true ORDER BY name ASC"
        ]

  putStrLn "=== SQL Parser Test ==="
  mapM_ testCase testCases
  where
    testCase sql = do
      putStrLn $ "\nInput: " ++ sql
      case parseSQL sql of
        Just query -> do
          putStrLn "Parsed: OK"
          putStrLn $ "Rendered: " ++ renderQuery query
        Nothing ->
          putStrLn "Error: Parse failed"