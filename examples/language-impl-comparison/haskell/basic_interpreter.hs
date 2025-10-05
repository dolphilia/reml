module BasicInterpreter where

import Control.Monad (foldM)
import Data.List (find, sortOn)
import qualified Data.Map.Strict as Map

data Value
  = VNumber Double
  | VString String
  | VArray [Value]
  deriving (Eq, Show)

type Env = Map.Map String Value

data Statement
  = Let { var :: String, expr :: Expr }
  | Print [Expr]
  | If { cond :: Expr, thenBlock :: [Statement], elseBlock :: [Statement] }
  | For { forVar :: String, start :: Expr, end :: Expr, step :: Expr, body :: [Statement] }
  | While { cond :: Expr, body :: [Statement] }
  | Goto Int
  | Gosub Int
  | Return
  | Dim { var :: String, size :: Expr }
  | End
  deriving (Show)

data Expr
  = Number Double
  | String String
  | Variable String
  | ArrayAccess { arrayVar :: String, index :: Expr }
  | BinOp { op :: BinOperator, left :: Expr, right :: Expr }
  | UnaryOp { unOp :: UnaryOperator, operand :: Expr }
  deriving (Show)

data BinOperator
  = Add | Sub | Mul | Div
  | Eq | Ne | Lt | Le | Gt | Ge
  | And | Or
  deriving (Show, Eq)

data UnaryOperator
  = Neg | Not
  deriving (Show, Eq)

type Program = [(Int, Statement)]

data RuntimeState = RuntimeState
  { env :: Env
  , callStack :: [Int]
  , output :: [String]
  } deriving (Show)

data RuntimeError
  = UndefinedVariable String
  | UndefinedLabel Int
  | TypeMismatch { expected :: String, got :: String }
  | IndexOutOfBounds
  | DivisionByZero
  | StackUnderflow
  deriving (Show)

run :: Program -> Either RuntimeError [String]
run program = do
  let initialState = RuntimeState Map.empty [] []
  let sorted = sortOn fst program
  finalState <- executeProgram sorted 0 initialState
  return (output finalState)

executeProgram :: Program -> Int -> RuntimeState -> Either RuntimeError RuntimeState
executeProgram program pc state
  | pc >= length program = Right state
  | otherwise =
      let (_, stmt) = program !! pc
       in case stmt of
            End -> Right state

            Let v e -> do
              value <- evalExpr e (env state)
              let newEnv = Map.insert v value (env state)
              executeProgram program (pc + 1) (state { env = newEnv })

            Print exprs -> do
              values <- mapM (\e -> evalExpr e (env state)) exprs
              let text = unwords (map valueToString values)
              let newOutput = output state ++ [text]
              executeProgram program (pc + 1) (state { output = newOutput })

            If c tb eb -> do
              condVal <- evalExpr c (env state)
              let branch = if isTruthy condVal then tb else eb
              newState <- executeBlock branch state
              executeProgram program (pc + 1) newState

            For fv s e st bd -> do
              startVal <- evalExpr s (env state)
              endVal <- evalExpr e (env state)
              stepVal <- evalExpr st (env state)
              executeForLoop fv startVal endVal stepVal bd program pc state

            While c bd -> executeWhileLoop c bd program pc state

            Goto target -> do
              newPc <- findLine program target
              executeProgram program newPc state

            Gosub target -> do
              newPc <- findLine program target
              let newCallStack = callStack state ++ [pc + 1]
              executeProgram program newPc (state { callStack = newCallStack })

            Return -> case callStack state of
              [] -> Left StackUnderflow
              cs -> let returnPc = last cs
                        newCallStack = init cs
                     in executeProgram program returnPc (state { callStack = newCallStack })

            Dim v sz -> do
              sizeVal <- evalExpr sz (env state)
              case sizeVal of
                VNumber n -> do
                  let array = replicate (floor n) (VNumber 0.0)
                  let newEnv = Map.insert v (VArray array) (env state)
                  executeProgram program (pc + 1) (state { env = newEnv })
                _ -> Left (TypeMismatch "Number" "Other")

executeBlock :: [Statement] -> RuntimeState -> Either RuntimeError RuntimeState
executeBlock stmts state = foldM executeSingleStatement state stmts

executeSingleStatement :: RuntimeState -> Statement -> Either RuntimeError RuntimeState
executeSingleStatement state stmt = case stmt of
  Let v e -> do
    value <- evalExpr e (env state)
    let newEnv = Map.insert v value (env state)
    return (state { env = newEnv })

  Print exprs -> do
    values <- mapM (\e -> evalExpr e (env state)) exprs
    let text = unwords (map valueToString values)
    let newOutput = output state ++ [text]
    return (state { output = newOutput })

  _ -> return state

executeForLoop :: String -> Value -> Value -> Value -> [Statement] -> Program -> Int -> RuntimeState -> Either RuntimeError RuntimeState
executeForLoop v (VNumber s) (VNumber e) (VNumber st) bd program pc state =
  forLoopHelper v s e st bd program pc state
executeForLoop _ _ _ _ _ _ _ _ = Left (TypeMismatch "Number" "Other")

forLoopHelper :: String -> Double -> Double -> Double -> [Statement] -> Program -> Int -> RuntimeState -> Either RuntimeError RuntimeState
forLoopHelper v current end step bd program pc state
  | (step > 0.0 && current > end) || (step < 0.0 && current < end) =
      executeProgram program (pc + 1) state
  | otherwise = do
      let newEnv = Map.insert v (VNumber current) (env state)
      newState <- executeBlock bd (state { env = newEnv })
      forLoopHelper v (current + step) end step bd program pc newState

executeWhileLoop :: Expr -> [Statement] -> Program -> Int -> RuntimeState -> Either RuntimeError RuntimeState
executeWhileLoop c bd program pc state = do
  condVal <- evalExpr c (env state)
  if isTruthy condVal
    then do
      newState <- executeBlock bd state
      executeWhileLoop c bd program pc newState
    else executeProgram program (pc + 1) state

evalExpr :: Expr -> Env -> Either RuntimeError Value
evalExpr (Number n) _ = Right (VNumber n)
evalExpr (String s) _ = Right (VString s)
evalExpr (Variable name) e =
  maybe (Left (UndefinedVariable name)) Right (Map.lookup name e)

evalExpr (ArrayAccess av idx) e = do
  case Map.lookup av e of
    Nothing -> Left (UndefinedVariable av)
    Just (VArray arr) -> do
      idxVal <- evalExpr idx e
      case idxVal of
        VNumber i ->
          let index = floor i
           in if index >= 0 && index < length arr
                then Right (arr !! index)
                else Left IndexOutOfBounds
        _ -> Left (TypeMismatch "Number" "Other")
    Just _ -> Left (TypeMismatch "Array" "Other")

evalExpr (BinOp op l r) e = do
  lVal <- evalExpr l e
  rVal <- evalExpr r e
  evalBinOp op lVal rVal

evalExpr (UnaryOp op operand) e = do
  val <- evalExpr operand e
  evalUnaryOp op val

evalBinOp :: BinOperator -> Value -> Value -> Either RuntimeError Value
evalBinOp Add (VNumber l) (VNumber r) = Right (VNumber (l + r))
evalBinOp Sub (VNumber l) (VNumber r) = Right (VNumber (l - r))
evalBinOp Mul (VNumber l) (VNumber r) = Right (VNumber (l * r))
evalBinOp Div (VNumber l) (VNumber r)
  | r == 0.0 = Left DivisionByZero
  | otherwise = Right (VNumber (l / r))
evalBinOp Eq (VNumber l) (VNumber r) = Right (VNumber (if l == r then 1.0 else 0.0))
evalBinOp Ne (VNumber l) (VNumber r) = Right (VNumber (if l /= r then 1.0 else 0.0))
evalBinOp Lt (VNumber l) (VNumber r) = Right (VNumber (if l < r then 1.0 else 0.0))
evalBinOp Le (VNumber l) (VNumber r) = Right (VNumber (if l <= r then 1.0 else 0.0))
evalBinOp Gt (VNumber l) (VNumber r) = Right (VNumber (if l > r then 1.0 else 0.0))
evalBinOp Ge (VNumber l) (VNumber r) = Right (VNumber (if l >= r then 1.0 else 0.0))
evalBinOp And l r = Right (VNumber (if isTruthy l && isTruthy r then 1.0 else 0.0))
evalBinOp Or l r = Right (VNumber (if isTruthy l || isTruthy r then 1.0 else 0.0))
evalBinOp _ _ _ = Left (TypeMismatch "Number" "Other")

evalUnaryOp :: UnaryOperator -> Value -> Either RuntimeError Value
evalUnaryOp Neg (VNumber n) = Right (VNumber (-n))
evalUnaryOp Not v = Right (VNumber (if isTruthy v then 0.0 else 1.0))
evalUnaryOp _ _ = Left (TypeMismatch "Number" "Other")

isTruthy :: Value -> Bool
isTruthy (VNumber n) = n /= 0.0
isTruthy (VString s) = not (null s)
isTruthy (VArray a) = not (null a)

valueToString :: Value -> String
valueToString (VNumber n) = show n
valueToString (VString s) = s
valueToString (VArray _) = "[Array]"

findLine :: Program -> Int -> Either RuntimeError Int
findLine program target =
  case find (\(line, _) -> line == target) (zip [0..] program) of
    Just (idx, _) -> Right idx
    Nothing -> Left (UndefinedLabel target)
