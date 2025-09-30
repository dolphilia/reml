// ミニ Lisp 評価機 (F# 実装)
// S式構文を持つ式を解析して評価する

module MiniLisp

open System
open System.Collections.Generic

/// 式の抽象構文木
type Expr =
  | Number of float
  | Symbol of string
  | List of Expr list

/// 評価値
type Value =
  | VNumber of float
  | VLambda of params': string list * body: Expr * env: Env
  | VBuiltin of (Value list -> Result<Value, string>)

and Env = Dictionary<string, Value>

/// パースエラー
type ParseError =
  | UnexpectedToken of string
  | UnmatchedParen
  | EmptyInput

/// トークン化: S式の括弧をスペースで区切る
let tokenize (source: string) : string list =
  source
    .Replace("(", " ( ")
    .Replace(")", " ) ")
    .Split([| ' '; '\n'; '\t'; '\r' |], StringSplitOptions.RemoveEmptyEntries)
  |> Array.toList

/// 式のパース
let rec parseExpr (tokens: string list) : Result<Expr * string list, ParseError> =
  match tokens with
  | [] -> Error EmptyInput
  | token :: rest -> parseToken token rest

and parseToken (token: string) (rest: string list) : Result<Expr * string list, ParseError> =
  if token = "(" then
    parseList rest []
  elif token = ")" then
    Error UnmatchedParen
  else
    match Double.TryParse(token) with
    | (true, num) -> Ok (Number num, rest)
    | _ -> Ok (Symbol token, rest)

and parseList (tokens: string list) (acc: Expr list) : Result<Expr * string list, ParseError> =
  match tokens with
  | [] -> Error UnmatchedParen
  | ")" :: rest -> Ok (List (List.rev acc), rest)
  | token :: rest ->
      match parseToken token rest with
      | Ok (expr, next) -> parseList next (expr :: acc)
      | Error err -> Error err

/// 式の評価
let rec evalExpr (expr: Expr) (env: Env) : Result<Value, string> =
  match expr with
  | Number n -> Ok (VNumber n)
  | Symbol name ->
      match env.TryGetValue(name) with
      | (true, value) -> Ok value
      | _ -> Error $"未定義シンボル: {name}"
  | List items -> evalList items env

and evalList (items: Expr list) (env: Env) : Result<Value, string> =
  match items with
  | [] -> Error "空のリストは評価できません"
  | head :: rest ->
      match evalExpr head env with
      | Error err -> Error err
      | Ok callee ->
          match evaluateArgs rest env with
          | Error err -> Error err
          | Ok args -> apply callee args

and evaluateArgs (exprs: Expr list) (env: Env) : Result<Value list, string> =
  exprs
  |> List.fold (fun acc expr ->
      match acc with
      | Error err -> Error err
      | Ok values ->
          match evalExpr expr env with
          | Ok value -> Ok (values @ [value])
          | Error err -> Error err
  ) (Ok [])

and apply (callee: Value) (args: Value list) : Result<Value, string> =
  match callee with
  | VBuiltin fn -> fn args
  | VLambda (params', body, lambdaEnv) -> applyLambda params' body lambdaEnv args
  | VNumber _ -> Error "数値を関数として適用できません"

and applyLambda (params': string list) (body: Expr) (lambdaEnv: Env) (args: Value list) : Result<Value, string> =
  if List.length params' <> List.length args then
    Error "引数の数が一致しません"
  else
    let newEnv = Dictionary<string, Value>(lambdaEnv)
    List.iter2 (fun param value -> newEnv.[param] <- value) params' args
    evalExpr body newEnv

/// 組み込み数値演算
let builtinNumeric (op: float -> float -> float) : Value list -> Result<Value, string> =
  fun args ->
    match args with
    | [VNumber lhs; VNumber rhs] -> Ok (VNumber (op lhs rhs))
    | _ -> Error "数値演算は2引数の数値のみ対応します"

/// デフォルト環境
let defaultEnv () : Env =
  let env = Dictionary<string, Value>()
  env.["+"] <- VBuiltin (builtinNumeric (+))
  env.["-"] <- VBuiltin (builtinNumeric (-))
  env.["*"] <- VBuiltin (builtinNumeric (*))
  env.["/"] <- VBuiltin (builtinNumeric (/))
  env

/// メイン評価関数
let eval (source: string) : Result<Value, string> =
  let tokens = tokenize source
  match parseExpr tokens with
  | Error EmptyInput -> Error "入力が空です"
  | Error UnmatchedParen -> Error "括弧が一致しません"
  | Error (UnexpectedToken token) -> Error $"予期しないトークン: {token}"
  | Ok (expr, rest) ->
      if List.isEmpty rest then
        evalExpr expr (defaultEnv())
      else
        Error "末尾に未消費トークンがあります"

// 利用例
// eval "(+ 40 2)" => Ok (VNumber 42.0)