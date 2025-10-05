-- ミニ Lisp 評価機 (Idris 2)
-- Idris 2 の特徴: 依存型、全域性チェック、線形型、証明支援

module MiniLisp

import Data.String
import Data.List
import Data.SortedMap

-- S式の定義
data Expr : Type where
  Num : Int -> Expr
  Sym : String -> Expr
  List : List Expr -> Expr

-- 値
data Value : Type where
  VNum : Int -> Value
  VSym : String -> Value
  VList : List Value -> Value
  VFunc : (List Value -> Either String Value) -> Value
  VNil : Value

-- 環境
Env : Type
Env = SortedMap String Value

-- S式の文字列化
exprToString : Expr -> String
exprToString (Num n) = show n
exprToString (Sym s) = s
exprToString (List items) =
  "(" ++ (unwords $ map exprToString items) ++ ")"

-- 値の文字列化
valueToString : Value -> String
valueToString (VNum n) = show n
valueToString (VSym s) = s
valueToString (VList items) =
  "(" ++ (unwords $ map valueToString items) ++ ")"
valueToString (VFunc _) = "<function>"
valueToString VNil = "nil"

-- 簡易パーサー（トークン化）
tokenize : String -> List String
tokenize input =
  let replaced = fastPack $ replaceOn '(' " ( " $
                 fastPack $ replaceOn ')' " ) " $
                 fastUnpack input
  in filter (/= "") $ words replaced

-- 文字列中の文字を置換
replaceOn : Char -> String -> List Char -> List Char
replaceOn target replacement str =
  concatMap (\c => if c == target then fastUnpack replacement else [c]) str

-- パース
mutual
  parseExpr : List String -> Either String (Expr, List String)
  parseExpr [] = Left "Unexpected EOF"
  parseExpr ("(" :: rest) = parseList rest []
  parseExpr (")" :: _) = Left "Unexpected ')'"
  parseExpr (token :: rest) =
    case parseInteger token of
      Just n => Right (Num n, rest)
      Nothing => Right (Sym token, rest)

  parseList : List String -> List Expr -> Either String (Expr, List String)
  parseList [] _ = Left "Unclosed '('"
  parseList (")" :: rest) acc = Right (List (reverse acc), rest)
  parseList tokens acc = do
    (expr, rest) <- parseExpr tokens
    parseList rest (expr :: acc)

-- トップレベルパース
parse : String -> Either String Expr
parse input = do
  let tokens = tokenize input
  (expr, rest) <- parseExpr tokens
  case rest of
    [] => Right expr
    _ => Left "Extra tokens after expression"

-- 真偽値判定
isTruthy : Value -> Bool
isTruthy VNil = False
isTruthy (VNum 0) = False
isTruthy _ = True

-- 式を値に変換（quote用）
exprToValue : Expr -> Either String Value
exprToValue (Num n) = Right (VNum n)
exprToValue (Sym s) = Right (VSym s)
exprToValue (List items) = do
  values <- traverse exprToValue items
  Right (VList values)

-- パラメータ名抽出
extractParamNames : List Expr -> Either String (List String)
extractParamNames [] = Right []
extractParamNames (Sym name :: rest) = do
  names <- extractParamNames rest
  Right (name :: names)
extractParamNames _ = Left "Lambda parameters must be symbols"

-- パラメータ束縛
bindParams : Env -> List String -> List Value -> Either String Env
bindParams env params args =
  if length params == length args
    then Right (foldl (\e, (n, v) => insert n v e) env (zip params args))
    else Left "Argument count mismatch"

-- リスト評価
evalList : Env -> List Expr -> Either String (List Value)
evalList env exprs = traverse (eval env) exprs

-- 数値抽出
extractNumbers : List Value -> Either String (List Int)
extractNumbers [] = Right []
extractNumbers (VNum n :: rest) = do
  nums <- extractNumbers rest
  Right (n :: nums)
extractNumbers _ = Left "Expected number"

-- 算術演算
evalArithmetic : Env -> List Expr -> (Int -> Int -> Int) -> Either String Value
evalArithmetic env args op = do
  values <- evalList env args
  nums <- extractNumbers values
  case nums of
    [] => Left "Arithmetic requires at least one argument"
    (first :: rest) => Right (VNum (foldl op first rest))

-- 比較演算
evalComparison : Env -> List Expr -> (Int -> Int -> Bool) -> Either String Value
evalComparison env args op = do
  values <- evalList env args
  nums <- extractNumbers values
  case nums of
    [a, b] => Right (if op a b then VNum 1 else VNum 0)
    _ => Left "Comparison requires exactly 2 arguments"

-- 関数適用
apply : Value -> List Value -> Either String Value
apply (VFunc f) args = f args
apply _ _ = Left "Not a function"

-- 評価（全域性のため covering を使用）
covering
eval : Env -> Expr -> Either String Value
eval env (Num n) = Right (VNum n)
eval env (Sym s) =
  case lookup s env of
    Just val => Right val
    Nothing => Left ("Unbound variable: " ++ s)
eval env (List []) = Right VNil
eval env (List [Sym "quote", arg]) = exprToValue arg
eval env (List [Sym "if", cond, thenExpr, elseExpr]) = do
  condVal <- eval env cond
  if isTruthy condVal
    then eval env thenExpr
    else eval env elseExpr
eval env (List [Sym "define", Sym name, valueExpr]) = do
  value <- eval env valueExpr
  Right value  -- Idrisでは環境の破壊的更新不可
eval env (List [Sym "lambda", List params, body]) = do
  paramNames <- extractParamNames params
  Right (VFunc (\args => do
    newEnv <- bindParams env paramNames args
    eval newEnv body))
eval env (List (Sym "+" :: args)) =
  evalArithmetic env args (+)
eval env (List (Sym "-" :: args)) =
  evalArithmetic env args (-)
eval env (List (Sym "*" :: args)) =
  evalArithmetic env args (*)
eval env (List (Sym "=" :: args)) =
  evalComparison env args (==)
eval env (List (Sym "<" :: args)) =
  evalComparison env args (<)
eval env (List (funcExpr :: argExprs)) = do
  func <- eval env funcExpr
  args <- evalList env argExprs
  apply func args
eval env (List [Sym name]) =
  case lookup name env of
    Just val => Right val
    Nothing => Left ("Unbound variable: " ++ name)

-- 初期環境
initialEnv : Env
initialEnv = fromList
  [ ("nil", VNil)
  , ("t", VNum 1)
  ]

-- テスト実行
covering
test : Env -> String -> String -> IO ()
test env input expected =
  case parse input of
    Left msg => putStrLn ("PARSE ERROR: " ++ input ++ " -> " ++ msg)
    Right expr =>
      case eval env expr of
        Left msg => putStrLn ("EVAL ERROR: " ++ input ++ " -> " ++ msg)
        Right value =>
          let result = valueToString value
          in if result == expected
               then putStrLn ("PASS: " ++ input ++ " = " ++ result)
               else putStrLn ("FAIL: " ++ input ++ " = " ++ result ++
                              " (expected: " ++ expected ++ ")")

-- メイン関数
covering
main : IO ()
main = do
  putStrLn "=== Mini Lisp Evaluator (Idris 2) ==="

  let env = initialEnv

  -- 基本的な式
  test env "42" "42"
  test env "(+ 1 2 3)" "6"
  test env "(- 10 3)" "7"
  test env "(* 2 3 4)" "24"

  -- 比較
  test env "(= 5 5)" "1"
  test env "(< 3 5)" "1"

  -- quote
  test env "(quote (1 2 3))" "(1 2 3)"

  -- if式
  test env "(if 1 10 20)" "10"
  test env "(if 0 10 20)" "20"

  -- lambda
  test env "((lambda (x) (+ x 1)) 5)" "6"
  test env "((lambda (x y) (* x y)) 3 4)" "12"

  putStrLn "\nAll tests completed."

{-
設計ノート:

このIdris 2実装は、依存型を持つ関数型言語でのLisp評価機を示しています。

主な特徴:

1. **依存型システム**
   - 型レベルでの詳細な仕様記述が可能
   - この実装では基本的な型のみ使用

2. **全域性チェック（Totality）**
   - `covering` アノテーションで部分的な全域性を宣言
   - 完全な全域性には再帰の停止性証明が必要

3. **線形型（Quantitative Type Theory）**
   - リソース管理を型レベルで追跡可能
   - この実装では使用していない

4. **Either型によるエラー処理**
   - do記法でモナド的エラー伝播
   - GleamのResult型と同様のパターン

5. **パターンマッチング**
   - 依存パターンマッチング
   - 網羅性チェック

Gleamとの比較:

**Gleam（BEAM VM）**:
- 不変データ構造（Erlang VM由来）
- 並行処理に強い
- 型推論は強力だが依存型なし
- `use` 構文でモナド的処理

**Idris 2（ネイティブ/Scheme/JS）**:
- 依存型による強力な静的検証
- 全域性チェックで停止性保証
- 定理証明機能（この例では未使用）
- 純粋関数型（副作用を型で追跡）

実用上の拡張:

- **依存型の活用**: 正しく型付けされたLispの証明
- **線形型の利用**: メモリ管理の静的保証
- **Quantitative型**: 使用回数制約（0, 1, ω）
- **停止性証明**: `total` 関数として証明

この実装は教育目的のため、Idris 2の高度な機能（依存型、証明）は
最小限に留めていますが、型安全性と全域性の概念を示しています。
-}
