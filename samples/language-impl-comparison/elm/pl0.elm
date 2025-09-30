-- PL/0 風トイ言語コンパイラ断片 (Elm 実装)
-- PL/0 サブセットの抽象構文木とインタプリタ

module PL0 exposing (..)

import Dict exposing (Dict)


-- 文
type Stmt
    = Assign { name : String, expr : Expr }
    | While { cond : Expr, body : List Stmt }
    | Write { expr : Expr }


-- 式
type Expr
    = Number Int
    | Var String
    | Binary { op : Op, lhs : Expr, rhs : Expr }


-- 演算子
type Op
    = Add
    | Sub
    | Mul
    | Div


-- ランタイム状態
type alias Runtime =
    { vars : Dict String Int
    , output : List Int
    }


-- パースエラー
type alias ParseError =
    { message : String }


-- 実行エラー
type alias ExecError =
    { reason : String }


-- プログラムのパース (簡易実装)
parseProgram : String -> Result ParseError (List Stmt)
parseProgram source =
    -- 実装のシンプルさを優先し、疑似実装を示す
    Ok
        [ Assign { name = "x", expr = Number 10 }
        , While
            { cond = Var "x"
            , body =
                [ Write { expr = Var "x" }
                , Assign
                    { name = "x"
                    , expr = Binary { op = Sub, lhs = Var "x", rhs = Number 1 }
                    }
                ]
            }
        ]


-- 初期ランタイム状態
initialRuntime : Runtime
initialRuntime =
    { vars = Dict.empty
    , output = []
    }


-- 式の評価
evalExpr : Expr -> Dict String Int -> Result ExecError Int
evalExpr expr vars =
    case expr of
        Number n ->
            Ok n

        Var name ->
            Dict.get name vars
                |> Result.fromMaybe { reason = "未定義変数: " ++ name }

        Binary { op, lhs, rhs } ->
            evalExpr lhs vars
                |> Result.andThen
                    (\l ->
                        evalExpr rhs vars
                            |> Result.andThen
                                (\r ->
                                    case op of
                                        Add ->
                                            Ok (l + r)

                                        Sub ->
                                            Ok (l - r)

                                        Mul ->
                                            Ok (l * r)

                                        Div ->
                                            if r == 0 then
                                                Err { reason = "0で割ることはできません" }

                                            else
                                                Ok (l // r)
                                )
                    )


-- 文の実行
execStmt : Stmt -> Runtime -> Result ExecError Runtime
execStmt stmt runtime =
    case stmt of
        Assign { name, expr } ->
            evalExpr expr runtime.vars
                |> Result.map
                    (\value ->
                        { runtime
                            | vars = Dict.insert name value runtime.vars
                        }
                    )

        While { cond, body } ->
            execWhile cond body runtime

        Write { expr } ->
            evalExpr expr runtime.vars
                |> Result.map
                    (\value ->
                        { runtime
                            | output = runtime.output ++ [ value ]
                        }
                    )


-- while ループの実行
execWhile : Expr -> List Stmt -> Runtime -> Result ExecError Runtime
execWhile cond body runtime =
    let
        loop : Runtime -> Result ExecError Runtime
        loop current =
            evalExpr cond current.vars
                |> Result.andThen
                    (\value ->
                        if value == 0 then
                            Ok current

                        else
                            execStmtList body current
                                |> Result.andThen loop
                    )
    in
    loop runtime


-- 文リストの実行
execStmtList : List Stmt -> Runtime -> Result ExecError Runtime
execStmtList stmts runtime =
    List.foldl
        (\stmt acc ->
            acc
                |> Result.andThen
                    (\state -> execStmt stmt state)
        )
        (Ok runtime)
        stmts


-- プログラムの実行
exec : List Stmt -> Result ExecError Runtime
exec program =
    execStmtList program initialRuntime


-- 利用例
-- parseProgram "begin x := 10; while x do write x; x := x - 1 end"
--   |> Result.andThen exec
-- => Ok { vars = Dict.empty, output = [10, 9, 8, 7, 6, 5, 4, 3, 2, 1] }