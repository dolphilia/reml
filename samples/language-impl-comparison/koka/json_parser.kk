// JSON パーサー - Koka 版
// Reml との比較ポイント: Result 型の利用、再帰的パターンマッチ

module samples/language-impl-comparison/json-parser

import std/core

// JSON 構文を解析して汎用値型に変換する
type json-value
  JNull
  JBool(flag: bool)
  JNumber(x: float64)
  JString(text: string)
  JArray(items: list<json-value>)
  JObject(pairs: list<(string, json-value)>)

// トークン型
type token
  LBrace
  RBrace
  LBracket
  RBracket
  Colon
  Comma
  StringLiteral(text: string)
  NumberLiteral(x: float64)
  BoolLiteral(flag: bool)
  NullLiteral

// パースエラー
type parse-error
  UnexpectedEOF
  UnexpectedToken(expected: string, found: string)

// パース状態
alias parse-state = list<token>

// メイン: JSON 文字列をパースして値を返す
pub fun parse-json(source: string) : exn json-value
  val tokens = tokenize(source)
  match parse-value(tokens)
    Error(err) -> throw(format-error(err))
    Ok((value, rest)) ->
      if rest.is-nil then
        value
      else
        throw("末尾に未消費トークンがあります")

// トークン化（簡易実装）
fun tokenize(source: string) : list<token>
  // 実装簡略化のため、空白で分割したシンプルなトークナイザー
  // 実用段階では文字列内の空白や記号を正しく処理する
  val cleaned = source
    .replace-all("{", " { ")
    .replace-all("}", " } ")
    .replace-all("[", " [ ")
    .replace-all("]", " ] ")
    .replace-all(":", " : ")
    .replace-all(",", " , ")

  cleaned.split(" ").filter(fn(s) !s.is-empty).map(classify-token)

// 文字列をトークンに分類
fun classify-token(s: string) : token
  if s == "{" then LBrace
  elif s == "}" then RBrace
  elif s == "[" then LBracket
  elif s == "]" then RBracket
  elif s == ":" then Colon
  elif s == "," then Comma
  elif s == "null" then NullLiteral
  elif s == "true" then BoolLiteral(True)
  elif s == "false" then BoolLiteral(False)
  elif s.starts-with("\"") then StringLiteral(s.trim-quotes)
  else
    match parse-float(s)
      Just(n) -> NumberLiteral(n)
      Nothing -> StringLiteral(s) // フォールバック

// 値のパース
fun parse-value(tokens: parse-state) : result<parse-error, (json-value, parse-state)>
  match tokens
    Nil -> Error(UnexpectedEOF)
    Cons(token, rest) ->
      match token
        NullLiteral -> Ok((JNull, rest))
        BoolLiteral(flag) -> Ok((JBool(flag), rest))
        NumberLiteral(num) -> Ok((JNumber(num), rest))
        StringLiteral(text) -> Ok((JString(text), rest))
        LBracket -> parse-array(rest)
        LBrace -> parse-object(rest)
        _ -> Error(UnexpectedToken("値", format-token(token)))

// 配列のパース
fun parse-array(tokens: parse-state) : result<parse-error, (json-value, parse-state)>
  match tokens
    Cons(RBracket, rest) -> Ok((JArray([]), rest))
    _ -> parse-array-items(tokens, [])

// 配列要素の再帰的パース
fun parse-array-items(tokens: parse-state, acc: list<json-value>) : result<parse-error, (json-value, parse-state)>
  match parse-value(tokens)
    Error(err) -> Error(err)
    Ok((value, rest)) ->
      val items = acc ++ [value]
      match rest
        Cons(Comma, next) -> parse-array-items(next, items)
        Cons(RBracket, next) -> Ok((JArray(items), next))
        Cons(other, _) -> Error(UnexpectedToken("]", format-token(other)))
        Nil -> Error(UnexpectedEOF)

// オブジェクトのパース
fun parse-object(tokens: parse-state) : result<parse-error, (json-value, parse-state)>
  match tokens
    Cons(RBrace, rest) -> Ok((JObject([]), rest))
    _ -> parse-object-pairs(tokens, [])

// オブジェクトのキー・値ペアの再帰的パース
fun parse-object-pairs(tokens: parse-state, acc: list<(string, json-value)>) : result<parse-error, (json-value, parse-state)>
  match expect-string(tokens)
    Error(err) -> Error(err)
    Ok((key, after-key)) ->
      match expect-colon(after-key)
        Error(err) -> Error(err)
        Ok(after-colon) ->
          match parse-value(after-colon)
            Error(err) -> Error(err)
            Ok((value, after-value)) ->
              val pairs = acc ++ [(key, value)]
              match after-value
                Cons(Comma, next) -> parse-object-pairs(next, pairs)
                Cons(RBrace, next) -> Ok((JObject(pairs), next))
                Cons(other, _) -> Error(UnexpectedToken("}", format-token(other)))
                Nil -> Error(UnexpectedEOF)

// 文字列トークンを期待
fun expect-string(tokens: parse-state) : result<parse-error, (string, parse-state)>
  match tokens
    Cons(StringLiteral(text), rest) -> Ok((text, rest))
    Cons(other, _) -> Error(UnexpectedToken("文字列", format-token(other)))
    Nil -> Error(UnexpectedEOF)

// コロントークンを期待
fun expect-colon(tokens: parse-state) : result<parse-error, parse-state>
  match tokens
    Cons(Colon, rest) -> Ok(rest)
    Cons(other, _) -> Error(UnexpectedToken(":", format-token(other)))
    Nil -> Error(UnexpectedEOF)

// トークンを文字列に整形
fun format-token(token: token) : string
  match token
    LBrace -> "{"
    RBrace -> "}"
    LBracket -> "["
    RBracket -> "]"
    Colon -> ":"
    Comma -> ","
    StringLiteral(s) -> "\"" ++ s ++ "\""
    NumberLiteral(n) -> show(n)
    BoolLiteral(b) -> show(b)
    NullLiteral -> "null"

// エラーメッセージの整形
fun format-error(err: parse-error) : string
  match err
    UnexpectedEOF -> "予期しない入力の終端"
    UnexpectedToken(expected, found) -> "期待: " ++ expected ++ ", 実際: " ++ found

// 引用符の除去
fun trim-quotes(s: string) : string
  if s.count >= 2 then
    s.slice.drop(1).take(s.count - 2).string
  else
    s

// 利用例
// parse-json("{\"key\": 123}") => JObject([("key", JNumber(123.0))])
// parse-json("[1, 2, 3]") => JArray([JNumber(1.0), JNumber(2.0), JNumber(3.0)])

// Reml との比較メモ:
// 1. Koka: Result<parse-error, T> で明示的エラーハンドリング
//    Reml: Result<T, ParseError> で同等の型を使用
// 2. Koka: 関数型スタイルで状態を渡していく
//    Reml: var による変数の再代入も可能（loop構文併用）
// 3. Koka: パターンマッチが主な制御構造
//    Reml: パターンマッチ + while/loop も利用可能
// 4. 両言語とも代数的データ型でJSON値を表現し、型安全性を確保