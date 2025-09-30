// PL/0 風トイ言語コンパイラ断片 (F# 実装)
// PL/0 サブセットの抽象構文木とインタプリタ

module PL0

open System.Collections.Generic

/// 文
type Stmt =
  | Assign of name: string * expr: Expr
  | While of cond: Expr * body: Stmt list
  | Write of expr: Expr

/// 式
and Expr =
  | Number of int
  | Var of string
  | Binary of op: Op * lhs: Expr * rhs: Expr

/// 演算子
and Op = Add | Sub | Mul | Div

/// ランタイム状態
type Runtime = {
  Vars: Dictionary<string, int>
  Output: int list
}

/// パースエラー
type ParseError = { Message: string }

/// 実行エラー
type ExecError = { Reason: string }

/// プログラムのパース (簡易実装)
let parseProgram (source: string) : Result<Stmt list, ParseError> =
  // 実装のシンプルさを優先し、疑似実装を示す
  Ok [
    Assign ("x", Number 10)
    While (
      Var "x",
      [
        Write (Var "x")
        Assign ("x", Binary (Sub, Var "x", Number 1))
      ]
    )
  ]

/// 初期ランタイム状態
let initialRuntime () : Runtime = {
  Vars = Dictionary<string, int>()
  Output = []
}

/// 式の評価
let rec evalExpr (expr: Expr) (vars: Dictionary<string, int>) : Result<int, ExecError> =
  match expr with
  | Number n -> Ok n
  | Var name ->
      match vars.TryGetValue(name) with
      | (true, value) -> Ok value
      | _ -> Error { Reason = $"未定義変数: {name}" }
  | Binary (op, lhs, rhs) ->
      match evalExpr lhs vars, evalExpr rhs vars with
      | Ok l, Ok r ->
          match op with
          | Add -> Ok (l + r)
          | Sub -> Ok (l - r)
          | Mul -> Ok (l * r)
          | Div ->
              if r = 0 then
                Error { Reason = "0で割ることはできません" }
              else
                Ok (l / r)
      | Error err, _ -> Error err
      | _, Error err -> Error err

/// 文の実行
let rec execStmt (stmt: Stmt) (runtime: Runtime) : Result<Runtime, ExecError> =
  match stmt with
  | Assign (name, expr) ->
      match evalExpr expr runtime.Vars with
      | Ok value ->
          runtime.Vars.[name] <- value
          Ok { runtime with Vars = runtime.Vars }
      | Error err -> Error err
  | While (cond, body) ->
      execWhile cond body runtime
  | Write expr ->
      match evalExpr expr runtime.Vars with
      | Ok value ->
          Ok { runtime with Output = runtime.Output @ [value] }
      | Error err -> Error err

/// while ループの実行
and execWhile (cond: Expr) (body: Stmt list) (runtime: Runtime) : Result<Runtime, ExecError> =
  let rec loop (current: Runtime) =
    match evalExpr cond current.Vars with
    | Error err -> Error err
    | Ok value ->
        if value = 0 then
          Ok current
        else
          match execStmtList body current with
          | Error err -> Error err
          | Ok nextState -> loop nextState
  loop runtime

/// 文リストの実行
and execStmtList (stmts: Stmt list) (runtime: Runtime) : Result<Runtime, ExecError> =
  stmts
  |> List.fold (fun acc stmt ->
      match acc with
      | Error err -> Error err
      | Ok state -> execStmt stmt state
  ) (Ok runtime)

/// プログラムの実行
let exec (program: Stmt list) : Result<Runtime, ExecError> =
  execStmtList program (initialRuntime())

// 利用例
// parseProgram "begin x := 10; while x do write x; x := x - 1 end"
// |> Result.bind exec
// => Ok { Output = [10; 9; 8; 7; 6; 5; 4; 3; 2; 1]; ... }