module Pl0 where

import Control.Monad (foldM)
import qualified Data.Map.Strict as Map

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

data Runtime = Runtime
  { vars :: Map.Map String Int
  , output :: [Int]
  }
  deriving (Eq, Show)

emptyRuntime :: Runtime
emptyRuntime = Runtime Map.empty []

type Exec = Either String

execProgram :: [Stmt] -> Exec Runtime
execProgram = foldM execStmt emptyRuntime

execStmt :: Runtime -> Stmt -> Exec Runtime
execStmt rt (Assign name expr) = do
  value <- evalExpr (vars rt) expr
  let newVars = Map.insert name value (vars rt)
  pure rt { vars = newVars }
execStmt rt (Write expr) = do
  value <- evalExpr (vars rt) expr
  pure rt { output = output rt <> [value] }
execStmt rt (While cond body) = loop rt
  where
    loop state = do
      condVal <- evalExpr (vars state) cond
      if condVal == 0
        then pure state
        else do
          state' <- foldM execStmt state body
          loop state'

evalExpr :: Map.Map String Int -> Expr -> Exec Int
evalExpr _ (Number n) = Right n
evalExpr env (Var name) =
  maybe (Left ("未定義変数: " <> name)) Right (Map.lookup name env)
evalExpr env (Binary op lhs rhs) = do
  l <- evalExpr env lhs
  r <- evalExpr env rhs
  apply op l r
  where
    apply Add a b = Right (a + b)
    apply Sub a b = Right (a - b)
    apply Mul a b = Right (a * b)
    apply Div _ 0 = Left "0 で割れません"
    apply Div a b = Right (a `div` b)
