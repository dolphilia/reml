module MiniLisp.Megaparsec where

import Control.Monad (void)
import qualified Data.Map.Strict as Map
import Data.Void (Void)
import Text.Megaparsec (Parsec, between, eof, many, runParser, satisfy, some, try, (<|>))
import qualified Text.Megaparsec as P
import Text.Megaparsec.Char (char, space1, string)
import qualified Text.Megaparsec.Char.Lexer as L

data Expr
  = Number Double
  | Symbol String
  | List [Expr]
  deriving (Eq, Show)

type Parser = Parsec Void String

spaceConsumer :: Parser ()
spaceConsumer = L.space space1 (L.skipLineComment "//") (L.skipBlockComment "/*" "*/")

lexeme :: Parser a -> Parser a
lexeme = L.lexeme spaceConsumer

symbol :: String -> Parser String
symbol = L.symbol spaceConsumer

parseExpr :: Parser Expr
parseExpr = lexeme (parseNumber <|> parseSymbol <|> parseList)

parseNumber :: Parser Expr
parseNumber = do
  n <- L.signed spaceConsumer L.float
  pure (Number n)

parseSymbol :: Parser Expr
parseSymbol = do
  first <- satisfy symbolStart
  rest <- many (satisfy symbolBody)
  pure (Symbol (first : rest))
  where
    symbolStart c = not (c `elem` "()" || c == ' ' || c == '\n')
    symbolBody c = not (c `elem` "()" || c == ' ' || c == '\n')

parseList :: Parser Expr
parseList = do
  void (symbol "(")
  exprs <- many parseExpr
  void (symbol ")")
  pure (List exprs)

parseTop :: String -> Either String Expr
parseTop source =
  case runParser (spaceConsumer *> parseExpr <* P.optional spaceConsumer <* eof) "mini-lisp" source of
    Left err -> Left (P.errorBundlePretty err)
    Right expr -> Right expr

-- 以下、以前の Haskell 版と同等の評価器をそのまま利用

data Value
  = VNumber Double
  | VLambda { params :: [String], body :: Expr, closure :: Env }
  | VBuiltin ([Value] -> Either String Value)

type Env = Map.Map String Value

eval :: Env -> Expr -> Either String Value
eval _ (Number n) = Right (VNumber n)
eval env (Symbol name) =
  maybe (Left ("未定義シンボル: " <> name)) Right (Map.lookup name env)
eval env (List (Symbol "lambda" : List paramsExpr : bodyExpr : [])) = do
  paramsSym <- traverse expectSymbol paramsExpr
  Right (VLambda paramsSym bodyExpr env)
  where
    expectSymbol (Symbol s) = Right s
    expectSymbol other = Left ("シンボルを期待しました: " <> show other)
eval env (List (Symbol "if" : condExpr : thenExpr : elseExpr : [])) = do
  condVal <- eval env condExpr
  case condVal of
    VNumber 0 -> eval env elseExpr
    VNumber _ -> eval env thenExpr
    _ -> Left "if は数値条件のみ対応します"
eval env (List (fnExpr : argExprs)) = do
  fnVal <- eval env fnExpr
  args <- traverse (eval env) argExprs
  apply fnVal args
eval _ (List []) = Left "空の式は評価できません"

apply :: Value -> [Value] -> Either String Value
apply (VBuiltin f) args = f args
apply (VLambda ps bodyExpr captured) args
  | length ps /= length args = Left "引数の数が一致しません"
  | otherwise =
      let newEnv = Map.union (Map.fromList (zip ps args)) captured
       in eval newEnv bodyExpr
apply (VNumber _) _ = Left "数値は適用できません"

defaultEnv :: Env
defaultEnv =
  Map.fromList
    [ ("+", numeric2 (pureOp (+)))
    , ("-", numeric2 (pureOp (-)))
    , ("*", numeric2 (pureOp (*)))
    , ("/", numeric2 safeDiv)
    , (">", numeric2 (pureOp (\a b -> if a > b then 1 else 0)))
    , ("=", numeric2 (pureOp (\a b -> if a == b then 1 else 0)))
    ]
  where
    numeric2 op = VBuiltin $ \case
      [VNumber a, VNumber b] -> VNumber <$> op a b
      _ -> Left "数値 2 引数を期待します"
    safeDiv _ 0 = Left "0 で割れません"
    safeDiv a b = Right (a / b)
    pureOp f a b = Right (f a b)

run :: String -> Either String Value
run source = do
  expr <- parseTop source
  eval defaultEnv expr
