module Pl0.Megaparsec where

import Control.Monad (void)
import Data.Void (Void)
import Text.Megaparsec (Parsec, between, eof, runParser, sepBy1, (<|>))
import qualified Text.Megaparsec as P
import Text.Megaparsec.Char (char, letterChar, space1, string)
import qualified Text.Megaparsec.Char.Lexer as L
import Text.Megaparsec.Expr (Operator (..), makeExprParser)

data Op = Add | Sub | Mul | Div deriving (Eq, Show)

data Expr
  = Number Int
  | Var String
  | Binary Op Expr Expr
  deriving (Eq, Show)

data Stmt
  = Assign String Expr
  | While Expr [Stmt]
  | Write Expr
  deriving (Eq, Show)

type Parser = Parsec Void String

spaceConsumer :: Parser ()
spaceConsumer = L.space space1 (L.skipLineComment "//") (L.skipBlockComment "/*" "*/")

lexeme :: Parser a -> Parser a
lexeme = L.lexeme spaceConsumer

symbol :: String -> Parser String
symbol = L.symbol spaceConsumer

identifier :: Parser String
identifier = lexeme $ (:) <$> letterChar <*> P.many (letterChar <|> P.digitChar <|> char '_')

integer :: Parser Int
integer = lexeme (L.signed spaceConsumer L.decimal)

expr :: Parser Expr
expr = makeExprParser term table
  where
    term =
      lexeme (Number <$> L.decimal)
        <|> Var <$> identifier
        <|> between (symbol "(") (symbol ")") expr
    table =
      [ [binary "*" (Binary Mul), binary "/" (Binary Div)]
      , [binary "+" (Binary Add), binary "-" (Binary Sub)]
      ]
    binary name f = InfixL (f <$ symbol name)

stmt :: Parser Stmt
stmt =
  whileStmt
    <|> writeStmt
    <|> assignStmt

assignStmt :: Parser Stmt
assignStmt = do
  name <- identifier
  void (symbol ":=")
  Assign name <$> expr

writeStmt :: Parser Stmt
writeStmt = Write <$> (symbol "write" *> expr)

whileStmt :: Parser Stmt
whileStmt = do
  void (symbol "while")
  cond <- expr
  void (symbol "do")
  body <- block
  pure (While cond body)

block :: Parser [Stmt]
block = between (symbol "begin") (symbol "end") (stmt `sepBy1` symbol ";")

program :: Parser [Stmt]
program = spaceConsumer *> block <* spaceConsumer <* eof

parsePl0 :: String -> Either String [Stmt]
parsePl0 source =
  case runParser program "pl0" source of
    Left err -> Left (P.errorBundlePretty err)
    Right ast -> Right ast
