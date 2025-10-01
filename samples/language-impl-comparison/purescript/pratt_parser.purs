module PrattParser where

import Prelude

import Data.Either (Either(..))
import Data.Array as Array
import Data.String.CodeUnits as String
import Data.Maybe (Maybe(..))
import Data.Int as Int
import Effect (Effect)
import Effect.Console (log)

-- | トークン型定義
data Token
  = TNumber Int
  | TPlus
  | TMinus
  | TStar
  | TSlash
  | TPower
  | TLParen
  | TRParen
  | TEOF

derive instance eqToken :: Eq Token

instance showToken :: Show Token where
  show (TNumber n) = "TNumber " <> show n
  show TPlus = "TPlus"
  show TMinus = "TMinus"
  show TStar = "TStar"
  show TSlash = "TSlash"
  show TPower = "TPower"
  show TLParen = "TLParen"
  show TRParen = "TRParen"
  show TEOF = "TEOF"

-- | 式の抽象構文木
data Expr
  = Num Int
  | Binary BinOp Expr Expr
  | Unary UnOp Expr

data BinOp = Add | Sub | Mul | Div | Pow

data UnOp = Neg | Pos

instance showExpr :: Show Expr where
  show (Num n) = show n
  show (Binary Add l r) = "(" <> show l <> " + " <> show r <> ")"
  show (Binary Sub l r) = "(" <> show l <> " - " <> show r <> ")"
  show (Binary Mul l r) = "(" <> show l <> " * " <> show r <> ")"
  show (Binary Div l r) = "(" <> show l <> " / " <> show r <> ")"
  show (Binary Pow l r) = "(" <> show l <> " ^ " <> show r <> ")"
  show (Unary Neg e) = "(-" <> show e <> ")"
  show (Unary Pos e) = "(+" <> show e <> ")"

-- | 字句解析器の状態
type LexerState =
  { input :: String
  , pos :: Int
  }

-- | パーサーの状態
type ParserState =
  { tokens :: Array Token
  , pos :: Int
  }

-- | エラー型
type ParseError = String

-- | 字句解析
lexer :: String -> Either ParseError (Array Token)
lexer input = go { input, pos: 0 } []
  where
  go :: LexerState -> Array Token -> Either ParseError (Array Token)
  go state acc
    | state.pos >= String.length state.input = Right (acc <> [TEOF])
    | otherwise = case String.charAt state.pos state.input of
        Nothing -> Right (acc <> [TEOF])
        Just c
          | c == ' ' || c == '\n' || c == '\t' ->
              go (state { pos = state.pos + 1 }) acc
          | c == '+' ->
              go (state { pos = state.pos + 1 }) (acc <> [TPlus])
          | c == '-' ->
              go (state { pos = state.pos + 1 }) (acc <> [TMinus])
          | c == '*' ->
              go (state { pos = state.pos + 1 }) (acc <> [TStar])
          | c == '/' ->
              go (state { pos = state.pos + 1 }) (acc <> [TSlash])
          | c == '^' ->
              go (state { pos = state.pos + 1 }) (acc <> [TPower])
          | c == '(' ->
              go (state { pos = state.pos + 1 }) (acc <> [TLParen])
          | c == ')' ->
              go (state { pos = state.pos + 1 }) (acc <> [TRParen])
          | c >= '0' && c <= '9' ->
              let
                numStr = String.takeWhile (\ch -> ch >= '0' && ch <= '9')
                  (String.drop state.pos state.input)
                len = String.length numStr
              in case Int.fromString numStr of
                Just n ->
                  go (state { pos = state.pos + len }) (acc <> [TNumber n])
                Nothing ->
                  Left ("Invalid number: " <> numStr)
          | otherwise ->
              Left ("Unexpected character: " <> String.singleton c)

-- | 現在のトークンを取得
current :: ParserState -> Token
current state = case Array.index state.tokens state.pos of
  Just tok -> tok
  Nothing -> TEOF

-- | 次のトークンへ進む
advance :: ParserState -> ParserState
advance state = state { pos = state.pos + 1 }

-- | 束縛力（優先度）を取得
-- 左束縛力: トークンが左側の式をどれだけ強く引き寄せるか
bindingPower :: Token -> { left :: Int, right :: Int }
bindingPower TPlus = { left: 20, right: 21 }
bindingPower TMinus = { left: 20, right: 21 }
bindingPower TStar = { left: 30, right: 31 }
bindingPower TSlash = { left: 30, right: 31 }
bindingPower TPower = { left: 40, right: 39 }  -- 右結合
bindingPower _ = { left: 0, right: 0 }

-- | 前置演算子の束縛力
prefixBindingPower :: Token -> Int
prefixBindingPower TPlus = 50
prefixBindingPower TMinus = 50
prefixBindingPower _ = 0

-- | Null Denotation (前置解析): トークンが式の先頭に来たときの解析
nud :: Token -> ParserState -> Either ParseError { expr :: Expr, state :: ParserState }
nud (TNumber n) state = Right { expr: Num n, state: advance state }
nud TLParen state = do
  let state' = advance state
  result <- parseExpr 0 state'
  case current result.state of
    TRParen -> Right { expr: result.expr, state: advance result.state }
    tok -> Left ("Expected ')', found: " <> show tok)
nud TMinus state = do
  let state' = advance state
  result <- parseExpr (prefixBindingPower TMinus) state'
  Right { expr: Unary Neg result.expr, state: result.state }
nud TPlus state = do
  let state' = advance state
  result <- parseExpr (prefixBindingPower TPlus) state'
  Right { expr: Unary Pos result.expr, state: result.state }
nud tok _ = Left ("Unexpected token in prefix position: " <> show tok)

-- | Left Denotation (中置解析): 左側に式があるときの解析
led :: Token -> Expr -> ParserState -> Either ParseError { expr :: Expr, state :: ParserState }
led TPlus left state = do
  let bp = bindingPower TPlus
  let state' = advance state
  result <- parseExpr bp.right state'
  Right { expr: Binary Add left result.expr, state: result.state }
led TMinus left state = do
  let bp = bindingPower TMinus
  let state' = advance state
  result <- parseExpr bp.right state'
  Right { expr: Binary Sub left result.expr, state: result.state }
led TStar left state = do
  let bp = bindingPower TStar
  let state' = advance state
  result <- parseExpr bp.right state'
  Right { expr: Binary Mul left result.expr, state: result.state }
led TSlash left state = do
  let bp = bindingPower TSlash
  let state' = advance state
  result <- parseExpr bp.right state'
  Right { expr: Binary Div left result.expr, state: result.state }
led TPower left state = do
  let bp = bindingPower TPower
  let state' = advance state
  result <- parseExpr bp.right state'
  Right { expr: Binary Pow left result.expr, state: result.state }
led tok _ _ = Left ("Unexpected token in infix position: " <> show tok)

-- | Pratt Parser の核心: 優先度ベース式解析
parseExpr :: Int -> ParserState -> Either ParseError { expr :: Expr, state :: ParserState }
parseExpr minBp state = do
  let tok = current state
  result <- nud tok state
  go minBp result.expr result.state
  where
  go :: Int -> Expr -> ParserState -> Either ParseError { expr :: Expr, state :: ParserState }
  go minBp' left state' =
    let tok = current state'
        bp = bindingPower tok
    in if bp.left < minBp'
       then Right { expr: left, state: state' }
       else do
         result <- led tok left state'
         go minBp' result.expr result.state

-- | パース実行
parse :: String -> Either ParseError Expr
parse input = do
  tokens <- lexer input
  result <- parseExpr 0 { tokens, pos: 0 }
  case current result.state of
    TEOF -> Right result.expr
    tok -> Left ("Unexpected token at end: " <> show tok)

-- | 評価
eval :: Expr -> Int
eval (Num n) = n
eval (Binary Add l r) = eval l + eval r
eval (Binary Sub l r) = eval l - eval r
eval (Binary Mul l r) = eval l * eval r
eval (Binary Div l r) = eval l / eval r
eval (Binary Pow l r) = powInt (eval l) (eval r)
eval (Unary Neg e) = -(eval e)
eval (Unary Pos e) = eval e

-- | 整数累乗
powInt :: Int -> Int -> Int
powInt base exp
  | exp == 0 = 1
  | exp < 0 = 0  -- 簡易実装のため負の指数は0とする
  | otherwise = base * powInt base (exp - 1)

-- | テスト実行
main :: Effect Unit
main = do
  log "=== Pratt Parser Tests ==="
  test "2 + 3 * 4" "(2 + (3 * 4))" 14
  test "2 * 3 + 4" "((2 * 3) + 4)" 10
  test "2 ^ 3 ^ 2" "(2 ^ (3 ^ 2))" 512  -- 右結合
  test "(1 + 2) * 3" "((1 + 2) * 3)" 9
  test "-5 + 3" "((-5) + 3)" (-2)
  test "2 * -3" "(2 * (-3))" (-6)
  test "10 - 2 - 3" "((10 - 2) - 3)" 5  -- 左結合
  test "2 + 3 * 4 ^ 2" "(2 + (3 * (4 ^ 2)))" 50
  where
  test input expectedAst expectedVal =
    case parse input of
      Left err -> log ("FAIL: " <> input <> " -> " <> err)
      Right ast ->
        let astStr = show ast
            val = eval ast
        in if astStr == expectedAst && val == expectedVal
           then log ("PASS: " <> input <> " = " <> show val)
           else log ("FAIL: " <> input <> " -> " <> astStr <> " = " <> show val <>
                     " (expected: " <> expectedAst <> " = " <> show expectedVal <> ")")
