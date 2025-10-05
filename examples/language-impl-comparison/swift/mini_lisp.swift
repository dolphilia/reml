// ミニ Lisp 評価機 (Swift 実装)
// S式構文を持つ式を解析して評価する

import Foundation

// 式の抽象構文木
enum Expr {
  case number(Double)
  case symbol(String)
  case list([Expr])
}

// 評価値
enum Value {
  case vNumber(Double)
  case vLambda(params: [String], body: Expr, env: Env)
  case vBuiltin(([Value]) -> Result<Value, String>)
}

typealias Env = [String: Value]

// パースエラー
enum ParseError: Error {
  case unexpectedToken(String)
  case unmatchedParen
  case emptyInput
}

// トークン化: S式の括弧をスペースで区切る
func tokenize(_ source: String) -> [String] {
  return source
    .replacingOccurrences(of: "(", with: " ( ")
    .replacingOccurrences(of: ")", with: " ) ")
    .split(separator: " ")
    .map(String.init)
    .filter { !$0.isEmpty }
}

// 式のパース
func parseExpr(_ tokens: [String]) -> Result<(Expr, [String]), ParseError> {
  guard !tokens.isEmpty else {
    return .failure(.emptyInput)
  }
  let token = tokens[0]
  let rest = Array(tokens.dropFirst())
  return parseToken(token, rest: rest)
}

func parseToken(_ token: String, rest: [String]) -> Result<(Expr, [String]), ParseError> {
  if token == "(" {
    return parseList(rest, acc: [])
  } else if token == ")" {
    return .failure(.unmatchedParen)
  } else if let num = Double(token) {
    return .success((.number(num), rest))
  } else {
    return .success((.symbol(token), rest))
  }
}

func parseList(_ tokens: [String], acc: [Expr]) -> Result<(Expr, [String]), ParseError> {
  guard !tokens.isEmpty else {
    return .failure(.unmatchedParen)
  }
  let token = tokens[0]
  let rest = Array(tokens.dropFirst())

  if token == ")" {
    return .success((.list(acc.reversed()), rest))
  } else {
    switch parseToken(token, rest: rest) {
    case .success(let (expr, next)):
      return parseList(next, acc: [expr] + acc)
    case .failure(let err):
      return .failure(err)
    }
  }
}

// 式の評価
func evalExpr(_ expr: Expr, env: Env) -> Result<Value, String> {
  switch expr {
  case .number(let n):
    return .success(.vNumber(n))
  case .symbol(let name):
    guard let value = env[name] else {
      return .failure("未定義シンボル: \(name)")
    }
    return .success(value)
  case .list(let items):
    return evalList(items, env: env)
  }
}

func evalList(_ items: [Expr], env: Env) -> Result<Value, String> {
  guard !items.isEmpty else {
    return .failure("空のリストは評価できません")
  }
  let head = items[0]
  let rest = Array(items.dropFirst())

  switch evalExpr(head, env: env) {
  case .success(let callee):
    switch evaluateArgs(rest, env: env) {
    case .success(let args):
      return apply(callee, args: args)
    case .failure(let err):
      return .failure(err)
    }
  case .failure(let err):
    return .failure(err)
  }
}

func evaluateArgs(_ exprs: [Expr], env: Env) -> Result<[Value], String> {
  var values: [Value] = []
  for expr in exprs {
    switch evalExpr(expr, env: env) {
    case .success(let value):
      values.append(value)
    case .failure(let err):
      return .failure(err)
    }
  }
  return .success(values)
}

func apply(_ callee: Value, args: [Value]) -> Result<Value, String> {
  switch callee {
  case .vBuiltin(let fn):
    return fn(args)
  case .vLambda(let params, let body, let lambdaEnv):
    return applyLambda(params: params, body: body, lambdaEnv: lambdaEnv, args: args)
  case .vNumber:
    return .failure("数値を関数として適用できません")
  }
}

func applyLambda(params: [String], body: Expr, lambdaEnv: Env, args: [Value]) -> Result<Value, String> {
  guard params.count == args.count else {
    return .failure("引数の数が一致しません")
  }
  var newEnv = lambdaEnv
  for (param, value) in zip(params, args) {
    newEnv[param] = value
  }
  return evalExpr(body, env: newEnv)
}

// 組み込み数値演算
func builtinNumeric(_ op: @escaping (Double, Double) -> Double) -> ([Value]) -> Result<Value, String> {
  return { args in
    guard args.count == 2 else {
      return .failure("数値演算は2引数のみ対応します")
    }
    guard case .vNumber(let lhs) = args[0],
          case .vNumber(let rhs) = args[1] else {
      return .failure("数値演算は数値のみ対応します")
    }
    return .success(.vNumber(op(lhs, rhs)))
  }
}

// デフォルト環境
func defaultEnv() -> Env {
  return [
    "+": .vBuiltin(builtinNumeric(+)),
    "-": .vBuiltin(builtinNumeric(-)),
    "*": .vBuiltin(builtinNumeric(*)),
    "/": .vBuiltin(builtinNumeric(/))
  ]
}

// メイン評価関数
func eval(_ source: String) -> Result<Value, String> {
  let tokens = tokenize(source)
  switch parseExpr(tokens) {
  case .success(let (expr, rest)):
    if rest.isEmpty {
      return evalExpr(expr, env: defaultEnv())
    } else {
      return .failure("末尾に未消費トークンがあります")
    }
  case .failure(.emptyInput):
    return .failure("入力が空です")
  case .failure(.unmatchedParen):
    return .failure("括弧が一致しません")
  case .failure(.unexpectedToken(let token)):
    return .failure("予期しないトークン: \(token)")
  }
}

// 利用例
// eval("(+ 40 2)") => .success(.vNumber(42.0))