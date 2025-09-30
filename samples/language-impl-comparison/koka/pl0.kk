// PL/0 風トイ言語コンパイラ断片 - Koka 版
// Reml との比較ポイント: 代数的データ型、再帰的評価、状態管理

module samples/language-impl-comparison/pl0

import std/core

// PL/0 風サブセットの抽象構文木
type stmt
  Assign(name: string, expr: expr)
  While(cond: expr, body: list<stmt>)
  Write(expr: expr)

// 式は 4 則演算のみ対応
type expr
  Number(n: int)
  Var(name: string)
  Binary(op: op, lhs: expr, rhs: expr)

type op
  Add
  Sub
  Mul
  Div

// 実行時の状態
struct runtime
  vars: list<(string, int)>
  output: list<int>

// エラー型
type parse-error
  ParseError(message: string)

type exec-error
  ExecError(reason: string)

// プログラムのパース（簡略化された疑似実装）
pub fun parse-program(source: string) : result<parse-error, list<stmt>>
  // 実装のシンプルさを優先し、ハードコードされた例を返す
  Ok([
    Assign("x", Number(5)),
    While(
      Var("x"),
      [
        Write(Var("x")),
        Assign("x", Binary(Sub, Var("x"), Number(1)))
      ]
    )
  ])

// プログラム全体の実行
pub fun exec(program: list<stmt>) : exn runtime
  match exec-stmts(program, initial-runtime())
    Error(err) -> throw(err.reason)
    Ok(rt) -> rt

// 文のリストを順次実行
fun exec-stmts(stmts: list<stmt>, runtime: runtime) : result<exec-error, runtime>
  match stmts
    Nil -> Ok(runtime)
    Cons(stmt, rest) ->
      match exec-stmt(stmt, runtime)
        Error(err) -> Error(err)
        Ok(new-runtime) -> exec-stmts(rest, new-runtime)

// 単一の文を実行
fun exec-stmt(stmt: stmt, runtime: runtime) : result<exec-error, runtime>
  match stmt
    Assign(name, expr) ->
      match eval-expr(expr, runtime.vars)
        Error(err) -> Error(err)
        Ok((value, vars)) -> Ok(runtime(vars = vars.update(name, value)))

    While(cond, body) ->
      exec-while(cond, body, runtime)

    Write(expr) ->
      match eval-expr(expr, runtime.vars)
        Error(err) -> Error(err)
        Ok((value, vars)) ->
          Ok(runtime(vars = vars, output = runtime.output ++ [value]))

// while ループの実行
fun exec-while(cond: expr, body: list<stmt>, runtime: runtime) : result<exec-error, runtime>
  match eval-expr(cond, runtime.vars)
    Error(err) -> Error(err)
    Ok((value, vars)) ->
      if value == 0 then
        Ok(runtime(vars = vars))
      else
        val current = runtime(vars = vars)
        match exec-stmts(body, current)
          Error(err) -> Error(err)
          Ok(next-runtime) -> exec-while(cond, body, next-runtime)

// 式を評価
fun eval-expr(expr: expr, vars: list<(string, int)>) : result<exec-error, (int, list<(string, int)>)>
  match expr
    Number(n) -> Ok((n, vars))

    Var(name) ->
      match vars.lookup(name)
        Just(value) -> Ok((value, vars))
        Nothing -> Error(ExecError("未定義変数: " ++ name))

    Binary(op, lhs, rhs) ->
      match eval-expr(lhs, vars)
        Error(err) -> Error(err)
        Ok((l-val, vars1)) ->
          match eval-expr(rhs, vars1)
            Error(err) -> Error(err)
            Ok((r-val, vars2)) ->
              match apply-op(op, l-val, r-val)
                Error(err) -> Error(err)
                Ok(result) -> Ok((result, vars2))

// 演算子を適用
fun apply-op(op: op, lhs: int, rhs: int) : result<exec-error, int>
  match op
    Add -> Ok(lhs + rhs)
    Sub -> Ok(lhs - rhs)
    Mul -> Ok(lhs * rhs)
    Div ->
      if rhs == 0 then
        Error(ExecError("0 で割ることはできません"))
      else
        Ok(lhs / rhs)

// 初期ランタイム状態
fun initial-runtime() : runtime
  Runtime([], [])

// 環境の検索
fun lookup(env: list<(string, int)>, name: string) : maybe<int>
  match env
    Nil -> Nothing
    Cons((k, v), rest) ->
      if k == name then Just(v) else rest.lookup(name)

// 環境の更新
fun update(env: list<(string, int)>, name: string, value: int) : list<(string, int)>
  Cons((name, value), env.filter(fn((k, _)) k != name))

// 利用例
// parse-program("begin x := 5; while x do write x; x := x - 1 end")
//   .map(exec)
//   => Runtime(output = [5, 4, 3, 2, 1], ...)

// Reml との比較メモ:
// 1. Koka: struct で名前付きフィールドを持つレコード型を定義
//    Reml: 匿名レコード型 { field: Type } も利用可能
// 2. Koka: パターンマッチで result 型を明示的に処理
//    Reml: ? 演算子で簡潔に Result を扱える
// 3. Koka: 環境は list<(string, int)> で単純なリストとして実装
//    Reml: Map<Text, i64> で効率的な辞書構造を利用
// 4. Koka: 効果システムにより、exn 効果が型に現れる
//    Reml: 同様に効果を型で追跡可能（with 節）
// 5. 両言語とも代数的データ型で AST を表現し、型安全性を確保