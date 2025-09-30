// ミニ Lisp 評価機 - Koka 版
// Reml との比較ポイント: 効果システムの利用、型推論、パターンマッチング

module samples/language-impl-comparison/mini-lisp

import std/core
import std/text/parse

// S 式構文を持つ式を解析して評価する
type expr
  Number(x: float64)
  Symbol(name: string)
  List(items: list<expr>)

// 評価に利用する値表現
type value
  VNumber(x: float64)
  VLambda(params: list<string>, body: expr, env: env)
  VBuiltin(fn: list<value> -> <exn, div> value)

// 環境は文字列から値へのマップ
alias env = list<(string, value)>

// パース時のエラー
type parse-error
  UnexpectedToken(token: string)
  UnmatchedParen
  EmptyInput

// メイン評価関数
// Koka: 効果型 <exn, div> が自動推論される
// Reml: with 節で明示的に効果を記述可能
pub fun eval(source: string) : <exn, div> value
  val tokens = tokenize(source)
  match parse-expr(tokens)
    Error(err) -> throw("パースエラー: " ++ err)
    Ok((expr, rest)) ->
      if rest.is-nil then
        val env = default-env()
        eval-expr(expr, env)
      else
        throw("末尾に未消費トークンがあります")

// トークン化: 括弧の前後に空白を挿入して分割
fun tokenize(source: string) : list<string>
  source
    .replace-all("(", " ( ")
    .replace-all(")", " ) ")
    .split(" ")
    .filter(fn(s) !s.is-empty)

// 式のパース
fun parse-expr(tokens: list<string>) : result<parse-error, (expr, list<string>)>
  match tokens
    Nil -> Error(EmptyInput)
    Cons(token, rest) -> parse-token(token, rest)

// トークンごとのパース
fun parse-token(token: string, rest: list<string>) : result<parse-error, (expr, list<string>)>
  if token == "(" then
    parse-list(rest, [])
  elif token == ")" then
    Error(UnmatchedParen)
  else
    match parse-float(token)
      Just(n) -> Ok((Number(n), rest))
      Nothing -> Ok((Symbol(token), rest))

// リストのパース（再帰的に蓄積）
fun parse-list(tokens: list<string>, acc: list<expr>) : result<parse-error, (expr, list<string>)>
  match tokens
    Nil -> Error(UnmatchedParen)
    Cons(token, rest) ->
      if token == ")" then
        Ok((List(acc.reverse), rest))
      else
        match parse-token(token, rest)
          Error(err) -> Error(err)
          Ok((expr, next)) -> parse-list(next, Cons(expr, acc))

// 式の評価
// Koka: exn と div 効果を持つ
// Reml: Except と Div 効果に相当
fun eval-expr(expr: expr, env: env) : <exn, div> value
  match expr
    Number(n) -> VNumber(n)
    Symbol(name) ->
      match env.lookup(name)
        Just(value) -> value
        Nothing -> throw("未定義シンボル: " ++ name)
    List(items) -> eval-list(items, env)

// リスト（関数適用）の評価
fun eval-list(items: list<expr>, env: env) : <exn, div> value
  match items
    Nil -> throw("空のリストは評価できません")
    Cons(head, rest) ->
      val callee = eval-expr(head, env)
      val args = evaluate-args(rest, env)
      apply(callee, args)

// 引数リストの評価
fun evaluate-args(exprs: list<expr>, env: env) : <exn, div> list<value>
  exprs.map(fn(e) eval-expr(e, env))

// 関数適用
fun apply(callee: value, args: list<value>) : <exn, div> value
  match callee
    VBuiltin(fun) -> fun(args)
    VLambda(params, body, lambda-env) -> apply-lambda(params, body, lambda-env, args)
    VNumber(_) -> throw("数値を関数としては適用できません")

// ラムダ適用
fun apply-lambda(params: list<string>, body: expr, lambda-env: env, args: list<value>) : <exn, div> value
  if params.length != args.length then
    throw("引数の数が一致しません")
  else
    val new-env = extend-env(lambda-env, params.zip(args))
    eval-expr(body, new-env)

// デフォルト環境: 基本的な算術演算
fun default-env() : env
  [ ("+", VBuiltin(builtin-numeric(fn(a, b) a + b)))
  , ("-", VBuiltin(builtin-numeric(fn(a, b) a - b)))
  , ("*", VBuiltin(builtin-numeric(fn(a, b) a * b)))
  , ("/", VBuiltin(builtin-numeric(fn(a, b) a / b)))
  ]

// 数値演算用ビルトイン関数
fun builtin-numeric(op: (float64, float64) -> float64) : list<value> -> <exn> value
  fn(args)
    match args
      [VNumber(lhs), VNumber(rhs)] -> VNumber(op(lhs, rhs))
      _ -> throw("数値演算は 2 引数の数値のみ対応します")

// 環境の拡張
fun extend-env(env: env, bindings: list<(string, value)>) : env
  bindings ++ env

// 環境からの値の検索
fun lookup(env: env, name: string) : maybe<value>
  match env
    Nil -> Nothing
    Cons((k, v), rest) ->
      if k == name then Just(v) else rest.lookup(name)

// 利用例
// eval("(+ 40 2)") => VNumber(42.0)
// eval("(* (+ 1 2) (- 5 3))") => VNumber(6.0)

// Reml との比較メモ:
// 1. Koka は効果型 <exn, div> が関数シグネチャに現れる
//    Reml は with 節で明示可能だが、推論も可能
// 2. Koka は Result 型ではなく例外効果 exn を使う傾向
//    Reml は Result を推奨し、効果は明示的なハンドラーで処理
// 3. Koka の構文: パターンマッチは indent-sensitive
//    Reml: end キーワードで明示的に閉じる
// 4. 両言語とも型推論が強力で、多くの型注釈を省略可能