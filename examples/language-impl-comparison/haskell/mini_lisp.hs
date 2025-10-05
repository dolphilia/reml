module MiniLisp where

import Control.Monad (foldM)
import Data.Char (isDigit)
import qualified Data.Map.Strict as Map

data Expr
  = Number Double
  | Symbol String
  | List [Expr]
  deriving (Eq, Show)

data Value
  = VNumber Double
  | VLambda { params :: [String], body :: Expr, closure :: Env }
  | VBuiltin ([Value] -> Either String Value)

type Env = Map.Map String Value

tokenize :: String -> [String]
tokenize = words . concatMap separate
  where
    separate '(' = " ( "
    separate ')' = " ) "
    separate c = [c]

parseExpr :: [String] -> Either String (Expr, [String])
parseExpr [] = Left "入力が空です"
parseExpr ("(" : rest) = parseList rest []
parseExpr (")" : _) = Left "対応しない閉じ括弧です"
parseExpr (tok : rest) = Right (atom tok, rest)
  where
    atom t =
      case reads t of
        [(n, "")] -> Number n
        _ -> Symbol t

parseList :: [String] -> [Expr] -> Either String (Expr, [String])
parseList [] _ = Left "リストが閉じられていません"
parseList (")" : rest) acc = Right (List (reverse acc), rest)
parseList tokens acc = do
  (expr, rest) <- parseExpr tokens
  parseList rest (expr : acc)

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
  let tokens = tokenize source
  (expr, rest) <- parseExpr tokens
  if null rest
    then eval defaultEnv expr
    else Left "未消費トークンがあります"
