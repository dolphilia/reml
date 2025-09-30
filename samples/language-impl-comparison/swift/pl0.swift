// PL/0 風トイ言語コンパイラ断片 (Swift 実装)
// PL/0 サブセットの抽象構文木とインタプリタ

import Foundation

// 文
enum Stmt {
  case assign(name: String, expr: Expr)
  case whileLoop(cond: Expr, body: [Stmt])
  case write(expr: Expr)
}

// 式
enum Expr {
  case number(Int)
  case variable(String)
  case binary(op: Op, lhs: Box<Expr>, rhs: Box<Expr>)
}

// Box型 (参照型として使用)
class Box<T> {
  let value: T
  init(_ value: T) {
    self.value = value
  }
}

// 演算子
enum Op {
  case add
  case sub
  case mul
  case div
}

// ランタイム状態
struct Runtime {
  var vars: [String: Int]
  var output: [Int]
}

// パースエラー
struct ParseError: Error {
  let message: String
}

// 実行エラー
struct ExecError: Error {
  let reason: String
}

// プログラムのパース (簡易実装)
func parseProgram(_ source: String) -> Result<[Stmt], ParseError> {
  // 実装のシンプルさを優先し、疑似実装を示す
  return .success([
    .assign(name: "x", expr: .number(10)),
    .whileLoop(
      cond: .variable("x"),
      body: [
        .write(expr: .variable("x")),
        .assign(
          name: "x",
          expr: .binary(
            op: .sub,
            lhs: Box(.variable("x")),
            rhs: Box(.number(1))
          )
        )
      ]
    )
  ])
}

// 初期ランタイム状態
func initialRuntime() -> Runtime {
  return Runtime(vars: [:], output: [])
}

// 式の評価
func evalExpr(_ expr: Expr, vars: [String: Int]) -> Result<Int, ExecError> {
  switch expr {
  case .number(let n):
    return .success(n)
  case .variable(let name):
    guard let value = vars[name] else {
      return .failure(ExecError(reason: "未定義変数: \(name)"))
    }
    return .success(value)
  case .binary(let op, let lhs, let rhs):
    switch (evalExpr(lhs.value, vars: vars), evalExpr(rhs.value, vars: vars)) {
    case (.success(let l), .success(let r)):
      switch op {
      case .add:
        return .success(l + r)
      case .sub:
        return .success(l - r)
      case .mul:
        return .success(l * r)
      case .div:
        guard r != 0 else {
          return .failure(ExecError(reason: "0で割ることはできません"))
        }
        return .success(l / r)
      }
    case (.failure(let err), _):
      return .failure(err)
    case (_, .failure(let err)):
      return .failure(err)
    }
  }
}

// 文の実行
func execStmt(_ stmt: Stmt, runtime: Runtime) -> Result<Runtime, ExecError> {
  switch stmt {
  case .assign(let name, let expr):
    switch evalExpr(expr, vars: runtime.vars) {
    case .success(let value):
      var newVars = runtime.vars
      newVars[name] = value
      return .success(Runtime(vars: newVars, output: runtime.output))
    case .failure(let err):
      return .failure(err)
    }
  case .whileLoop(let cond, let body):
    return execWhile(cond: cond, body: body, runtime: runtime)
  case .write(let expr):
    switch evalExpr(expr, vars: runtime.vars) {
    case .success(let value):
      return .success(Runtime(vars: runtime.vars, output: runtime.output + [value]))
    case .failure(let err):
      return .failure(err)
    }
  }
}

// while ループの実行
func execWhile(cond: Expr, body: [Stmt], runtime: Runtime) -> Result<Runtime, ExecError> {
  func loop(current: Runtime) -> Result<Runtime, ExecError> {
    switch evalExpr(cond, vars: current.vars) {
    case .failure(let err):
      return .failure(err)
    case .success(let value):
      if value == 0 {
        return .success(current)
      } else {
        switch execStmtList(body, runtime: current) {
        case .failure(let err):
          return .failure(err)
        case .success(let nextState):
          return loop(current: nextState)
        }
      }
    }
  }
  return loop(current: runtime)
}

// 文リストの実行
func execStmtList(_ stmts: [Stmt], runtime: Runtime) -> Result<Runtime, ExecError> {
  var current = runtime
  for stmt in stmts {
    switch execStmt(stmt, runtime: current) {
    case .success(let nextState):
      current = nextState
    case .failure(let err):
      return .failure(err)
    }
  }
  return .success(current)
}

// プログラムの実行
func exec(_ program: [Stmt]) -> Result<Runtime, ExecError> {
  return execStmtList(program, runtime: initialRuntime())
}

// 利用例
// parseProgram("begin x := 10; while x do write x; x := x - 1 end")
//   .flatMap(exec)
// => .success(Runtime(vars: [:], output: [10, 9, 8, 7, 6, 5, 4, 3, 2, 1]))