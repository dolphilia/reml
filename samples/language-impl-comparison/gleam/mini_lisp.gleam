// ミニ Lisp 評価機 (Gleam)
// Gleam の特徴: BEAM VM、Rustライクな構文、Result型、パターンマッチング

import gleam/io
import gleam/int
import gleam/list
import gleam/string
import gleam/result
import gleam/dict.{type Dict}

// S式の定義
pub type Expr {
  Num(Int)
  Sym(String)
  List(List(Expr))
}

// 評価結果
pub type Value {
  VNum(Int)
  VSym(String)
  VList(List(Value))
  VFunc(fn(List(Value)) -> Result(Value, String))
  VNil
}

// 環境
pub type Env =
  Dict(String, Value)

// エラー型
pub type EvalError =
  String

// S式の文字列化
pub fn expr_to_string(expr: Expr) -> String {
  case expr {
    Num(n) -> int.to_string(n)
    Sym(s) -> s
    List(items) -> {
      let items_str =
        items
        |> list.map(expr_to_string)
        |> string.join(" ")
      "(" <> items_str <> ")"
    }
  }
}

// 値の文字列化
pub fn value_to_string(value: Value) -> String {
  case value {
    VNum(n) -> int.to_string(n)
    VSym(s) -> s
    VList(items) -> {
      let items_str =
        items
        |> list.map(value_to_string)
        |> string.join(" ")
      "(" <> items_str <> ")"
    }
    VFunc(_) -> "<function>"
    VNil -> "nil"
  }
}

// 簡易パーサー（トークン化）
pub fn tokenize(input: String) -> List(String) {
  input
  |> string.replace("(", " ( ")
  |> string.replace(")", " ) ")
  |> string.split(" ")
  |> list.filter(fn(s) { s != "" })
}

// パース
pub fn parse(tokens: List(String)) -> Result(#(Expr, List(String)), String) {
  case tokens {
    [] -> Error("Unexpected EOF")
    ["(" , ..rest] -> parse_list(rest, [])
    [")", ..] -> Error("Unexpected ')'")
    [token, ..rest] ->
      case int.parse(token) {
        Ok(n) -> Ok(#(Num(n), rest))
        Error(_) -> Ok(#(Sym(token), rest))
      }
  }
}

fn parse_list(
  tokens: List(String),
  acc: List(Expr),
) -> Result(#(Expr, List(String)), String) {
  case tokens {
    [] -> Error("Unclosed '('")
    [")", ..rest] -> Ok(#(List(list.reverse(acc)), rest))
    _ -> {
      use #(expr, rest) <- result.try(parse(tokens))
      parse_list(rest, [expr, ..acc])
    }
  }
}

// トップレベルパース
pub fn parse_expr(input: String) -> Result(Expr, String) {
  let tokens = tokenize(input)
  use #(expr, rest) <- result.try(parse(tokens))
  case rest {
    [] -> Ok(expr)
    _ -> Error("Extra tokens after expression")
  }
}

// 評価
pub fn eval(env: Env, expr: Expr) -> Result(Value, EvalError) {
  case expr {
    Num(n) -> Ok(VNum(n))
    Sym(s) -> {
      case dict.get(env, s) {
        Ok(val) -> Ok(val)
        Error(_) -> Error("Unbound variable: " <> s)
      }
    }
    List([]) -> Ok(VNil)
    List([Sym("quote"), arg]) -> expr_to_value(arg)
    List([Sym("if"), cond, then_expr, else_expr]) -> {
      use cond_val <- result.try(eval(env, cond))
      case is_truthy(cond_val) {
        True -> eval(env, then_expr)
        False -> eval(env, else_expr)
      }
    }
    List([Sym("define"), Sym(name), value_expr]) -> {
      use value <- result.try(eval(env, value_expr))
      // Gleamは環境の破壊的更新ができないため、値を返すのみ
      Ok(value)
    }
    List([Sym("lambda"), List(params), body]) -> {
      use param_names <- result.try(extract_param_names(params))
      Ok(VFunc(fn(args) {
        use new_env <- result.try(bind_params(env, param_names, args))
        eval(new_env, body)
      }))
    }
    List([Sym("+"), ..args]) -> eval_arithmetic(env, args, fn(a, b) { a + b })
    List([Sym("-"), ..args]) -> eval_arithmetic(env, args, fn(a, b) { a - b })
    List([Sym("*"), ..args]) -> eval_arithmetic(env, args, fn(a, b) { a * b })
    List([Sym("="), ..args]) -> eval_comparison(env, args, fn(a, b) { a == b })
    List([Sym("<"), ..args]) -> eval_comparison(env, args, fn(a, b) { a < b })
    List([func_expr, ..arg_exprs]) -> {
      use func <- result.try(eval(env, func_expr))
      use args <- result.try(eval_list(env, arg_exprs))
      apply(func, args)
    }
  }
}

// 式を値に変換（quote用）
fn expr_to_value(expr: Expr) -> Result(Value, String) {
  case expr {
    Num(n) -> Ok(VNum(n))
    Sym(s) -> Ok(VSym(s))
    List(items) -> {
      use values <- result.try(list.try_map(items, expr_to_value))
      Ok(VList(values))
    }
  }
}

// 真偽値判定
fn is_truthy(value: Value) -> Bool {
  case value {
    VNil -> False
    VNum(0) -> False
    _ -> True
  }
}

// パラメータ名抽出
fn extract_param_names(params: List(Expr)) -> Result(List(String), String) {
  list.try_map(params, fn(param) {
    case param {
      Sym(name) -> Ok(name)
      _ -> Error("Lambda parameters must be symbols")
    }
  })
}

// パラメータ束縛
fn bind_params(
  env: Env,
  params: List(String),
  args: List(Value),
) -> Result(Env, String) {
  case list.length(params) == list.length(args) {
    False -> Error("Argument count mismatch")
    True -> {
      let bindings = list.zip(params, args)
      Ok(list.fold(bindings, env, fn(acc, pair) {
        let #(name, value) = pair
        dict.insert(acc, name, value)
      }))
    }
  }
}

// リスト評価
fn eval_list(env: Env, exprs: List(Expr)) -> Result(List(Value), String) {
  list.try_map(exprs, fn(expr) { eval(env, expr) })
}

// 算術演算
fn eval_arithmetic(
  env: Env,
  args: List(Expr),
  op: fn(Int, Int) -> Int,
) -> Result(Value, String) {
  use values <- result.try(eval_list(env, args))
  use nums <- result.try(extract_numbers(values))
  case nums {
    [] -> Error("Arithmetic requires at least one argument")
    [first, ..rest] -> Ok(VNum(list.fold(rest, first, op)))
  }
}

// 比較演算
fn eval_comparison(
  env: Env,
  args: List(Expr),
  op: fn(Int, Int) -> Bool,
) -> Result(Value, String) {
  use values <- result.try(eval_list(env, args))
  use nums <- result.try(extract_numbers(values))
  case nums {
    [a, b] -> {
      case op(a, b) {
        True -> Ok(VNum(1))
        False -> Ok(VNum(0))
      }
    }
    _ -> Error("Comparison requires exactly 2 arguments")
  }
}

// 数値抽出
fn extract_numbers(values: List(Value)) -> Result(List(Int), String) {
  list.try_map(values, fn(value) {
    case value {
      VNum(n) -> Ok(n)
      _ -> Error("Expected number")
    }
  })
}

// 関数適用
fn apply(func: Value, args: List(Value)) -> Result(Value, String) {
  case func {
    VFunc(f) -> f(args)
    _ -> Error("Not a function")
  }
}

// 初期環境
pub fn initial_env() -> Env {
  dict.new()
  |> dict.insert("nil", VNil)
  |> dict.insert("t", VNum(1))
}

// テスト実行
pub fn main() {
  io.println("=== Mini Lisp Evaluator (Gleam) ===")

  let env = initial_env()

  // 基本的な式
  test(env, "42", "42")
  test(env, "(+ 1 2 3)", "6")
  test(env, "(- 10 3)", "7")
  test(env, "(* 2 3 4)", "24")

  // 比較
  test(env, "(= 5 5)", "1")
  test(env, "(< 3 5)", "1")

  // quote
  test(env, "(quote (1 2 3))", "(1 2 3)")

  // if式
  test(env, "(if 1 10 20)", "10")
  test(env, "(if 0 10 20)", "20")

  // lambda（Gleamの制約により完全なクロージャは難しい）
  test(env, "((lambda (x) (+ x 1)) 5)", "6")
  test(env, "((lambda (x y) (* x y)) 3 4)", "12")

  io.println("\nAll tests completed.")
}

fn test(env: Env, input: String, expected: String) {
  case parse_expr(input) {
    Error(msg) -> io.println("PARSE ERROR: " <> input <> " -> " <> msg)
    Ok(expr) ->
      case eval(env, expr) {
        Error(msg) -> io.println("EVAL ERROR: " <> input <> " -> " <> msg)
        Ok(value) -> {
          let result = value_to_string(value)
          case result == expected {
            True -> io.println("PASS: " <> input <> " = " <> result)
            False ->
              io.println(
                "FAIL: "
                <> input
                <> " = "
                <> result
                <> " (expected: "
                <> expected
                <> ")",
              )
          }
        }
      }
  }
}
