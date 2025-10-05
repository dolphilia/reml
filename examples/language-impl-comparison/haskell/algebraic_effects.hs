-- 代数的効果を使うミニ言語 - Haskell 版
-- Reml との比較: モナド変換子による効果のエミュレーション

{-# LANGUAGE DeriveFunctor #-}

module AlgebraicEffects where

import Control.Monad (foldM)
import Control.Monad.State (StateT, evalStateT, get, modify, put)
import Control.Monad.Except (ExceptT, runExceptT, throwError)
import Control.Monad.Trans.Class (lift)

-- ミニ言語の式定義
data Expr
  = Lit Int
  | Var String
  | Add Expr Expr
  | Mul Expr Expr
  | Div Expr Expr
  | Get
  | Put Expr
  | Fail String
  | Choose Expr Expr
  deriving (Show)

type Env = [(String, Int)]

-- 効果をモナド変換子スタックで表現
-- State<Int> × Except<String> × Choose（リスト）
-- Reml: with State<Int>, Except<String>, Choose
-- Haskell: StateT Int (ExceptT String []) a
type EffectM a = StateT Int (ExceptT String []) a

-- 式の評価関数（効果を持つ）
--
-- Reml の perform に相当する操作をモナド操作で記述：
-- - get/put: StateT の操作
-- - throwError: ExceptT の操作
-- - lift . lift: リストモナド（非決定性）への持ち上げ
eval :: Expr -> Env -> EffectM Int
eval (Lit n) _ = return n

eval (Var name) env =
  case lookup name env of
    Just value -> return value
    Nothing -> throwError $ "未定義変数: " ++ name

eval (Add left right) env = do
  l <- eval left env
  r <- eval right env
  return (l + r)

eval (Mul left right) env = do
  l <- eval left env
  r <- eval right env
  return (l * r)

eval (Div left right) env = do
  l <- eval left env
  r <- eval right env
  if r == 0
    then throwError "ゼロ除算"
    else return (l `div` r)

eval Get _ = get

eval (Put e) env = do
  v <- eval e env
  put v
  return v

eval (Fail msg) _ = throwError msg

eval (Choose left right) env = do
  -- 非決定的選択: lift を2回使ってリストモナドに到達
  -- Reml の choose_handler に相当する部分が複雑
  st <- get
  let leftM = evalStateT (eval left env) st
  let rightM = evalStateT (eval right env) st
  results <- lift $ lift [leftM, rightM]
  case results of
    Left err -> throwError err
    Right (value, newState) -> do
      put newState
      return value

-- すべての効果を処理して結果を返す
--
-- Reml の handle ... do ... do ... に相当するが、
-- Haskell ではモナド変換子のスタックを順に実行。
runWithAllEffects :: Expr -> Env -> Int -> Either String [(Int, Int)]
runWithAllEffects expr env initState =
  case runExceptT $ evalStateT (eval expr env) initState of
    [] -> Left "選択肢なし"
    results -> sequence results >>= \vals -> Right [(v, s) | (v, s) <- vals]

-- より簡易版: State のみをハンドル
runWithState :: Expr -> Env -> Int -> ExceptT String [] (Int, Int)
runWithState expr env initState = do
  value <- evalStateT (eval expr env) initState
  return (value, initState)

-- テストケース
exampleExpressions :: [(String, Expr)]
exampleExpressions =
  [ ("単純な加算", Add (Lit 10) (Lit 20))
  , ("乗算と除算", Div (Mul (Lit 6) (Lit 7)) (Lit 2))
  , ("状態の取得", Add Get (Lit 5))
  , ("状態の更新", Put (Add Get (Lit 1)))
  , ("ゼロ除算エラー", Div (Lit 10) (Lit 0))
  , ("非決定的選択", Choose (Lit 1) (Lit 2))
  , ("複雑な例", Add
      (Choose (Lit 10) (Lit 20))
      (Put (Add Get (Lit 1)))
    )
  ]

-- テスト実行関数
runExamples :: IO ()
runExamples = do
  let env = []
      initState = 0
  mapM_ (\(name, expr) -> do
    putStrLn $ "--- " ++ name ++ " ---"
    case runWithAllEffects expr env initState of
      Right results ->
        mapM_ (\(value, state) ->
          putStrLn $ "  結果: " ++ show value ++ ", 状態: " ++ show state
        ) results
      Left err ->
        putStrLn $ "  エラー: " ++ err
    ) exampleExpressions

-- Reml との比較メモ:
--
-- 1. **効果の表現**
--    Reml: effect State<S> { operation get() -> S; operation put(s: S) -> () }
--    Haskell: StateT Int (ExceptT String []) a（モナド変換子スタック）
--    - Reml は言語レベルで効果を定義
--    - Haskell は型クラス（Monad）で効果を合成
--
-- 2. **ハンドラーの実装**
--    Reml: handler state_handler<A>(init) for State<S> { ... }
--    Haskell: evalStateT/runExceptT でモナドを順に実行
--    - Reml はハンドラーが明示的で再利用可能
--    - Haskell は型レベルで効果を積み重ねるが、順序変更が困難
--
-- 3. **非決定性の扱い**
--    Reml: choose_handler で分岐を自動収集
--    Haskell: lift . lift でリストモナドに到達（煩雑）
--    - Reml は分岐が自然に追跡される
--    - Haskell は lift を多用する必要がある
--
-- 4. **型推論**
--    Reml: 効果が型レベルで推論される
--    Haskell: 型注釈が必要（特にモナド変換子）
--    - どちらも型安全だが、Haskell は型が複雑
--
-- 5. **可読性**
--    Reml: with State<Int>, Except<String>, Choose で効果が明確
--    Haskell: StateT Int (ExceptT String []) a が冗長
--    - Reml の方が効果の意図が分かりやすい
--
-- 6. **効果の順序変更**
--    Reml: ハンドラーの順序を簡単に入れ替え可能
--    Haskell: モナド変換子の順序を変えると型が変わり、コード全体の書き換えが必要
--    - Reml の方が柔軟性が高い
--
-- **結論**:
-- Haskell のモナド変換子は強力だが、複雑な効果の合成では煩雑になる。
-- Reml の代数的効果システムはより直感的で、効果の順序変更が容易。
-- 特に lift を多用する必要がない点で、Reml の方が記述性に優れる。

-- テスト実行例（main 関数）
-- main :: IO ()
-- main = runExamples