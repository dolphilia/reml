// JSON拡張版：コメント・トレーリングカンマ対応。
//
// 標準JSONからの拡張点：
// 1. コメント対応（`//` 行コメント、`/* */` ブロックコメント）
// 2. トレーリングカンマ許可（配列・オブジェクトの最後の要素の後）
// 3. より詳細なエラーメッセージ
//
// 実用的な設定ファイル形式として：
// - `package.json` 風の設定ファイル
// - `.babelrc`, `.eslintrc` など開発ツールの設定
// - VS Code の `settings.json`

module json_extended

import std/text/parse

// 型定義

type json-value
  JNull
  JBool(b: bool)
  JNumber(n: float64)
  JString(s: string)
  JArray(items: list<json-value>)
  JObject(pairs: list<(string, json-value)>)

type parse-error
  UnexpectedEOF
  InvalidValue(msg: string)
  UnclosedString
  UnclosedBlockComment
  ExpectedChar(ch: char)
  InvalidNumber(str: string)

// パース

fun parse(input: string): <exn> either<parse-error, json-value>
  match parse-eof({ skip-ws-and-comments(); parse-value() }, input.slice)
    ParseOk(value, _) -> Right(value)
    ParseError(msg, _) -> Left(InvalidValue(msg))

// 空白とコメントをスキップ

fun skip-ws-and-comments(): <parse> ()
  skip-whitespace()
  match peek-char()
    Just('/') ->
      match peek-char-at(1)
        Just('/') ->
          skip-line-comment()
          skip-ws-and-comments()
        Just('*') ->
          skip-block-comment()
          skip-ws-and-comments()
        _ -> ()
    _ -> ()

fun skip-line-comment(): <parse> ()
  char('/')
  char('/')
  many(fn() satisfy(fn(c) c != '\n'))
  optional(fn() char('\n'))
  ()

fun skip-block-comment(): <parse> ()
  char('/')
  char('*')
  skip-until-block-end()

fun skip-until-block-end(): <parse> ()
  match peek-char()
    Nothing -> fail("ブロックコメントが閉じられていません")
    Just('*') ->
      match peek-char-at(1)
        Just('/') ->
          char('*')
          char('/')
        _ ->
          advance()
          skip-until-block-end()
    _ ->
      advance()
      skip-until-block-end()

fun peek-char-at(offset: int): <parse> maybe<char>
  val saved = current-input()
  repeat(offset, fn() { advance(); () })
  val result = peek-char()
  set-input(saved)
  result

fun advance(): <parse> ()
  satisfy(fn(_) True)
  ()

fun current-input(): <parse> sslice
  match peek-char()
    Nothing -> "".slice
    Just(_) ->
      val saved = peek()
      saved

// 値のパース

fun parse-value(): <parse> json-value
  skip-ws-and-comments()
  match peek-char()
    Nothing -> fail("予期しないEOF")
    Just('n') ->
      pstring("null")
      JNull
    Just('t') ->
      pstring("true")
      JBool(True)
    Just('f') ->
      pstring("false")
      JBool(False)
    Just('"') -> parse-string()
    Just('[') -> parse-array()
    Just('{') -> parse-object()
    _ -> parse-number()

// 文字列リテラルのパース

fun parse-string(): <parse> json-value
  char('"')
  val str = build-string()
  char('"')
  JString(str)

fun build-string(): <parse> string
  match peek-char()
    Nothing -> fail("文字列が閉じられていません")
    Just('"') -> ""
    Just('\\') ->
      char('\\')
      val escaped = match peek-char()
        Just('n') -> { char('n'); "\n" }
        Just('t') -> { char('t'); "\t" }
        Just('r') -> { char('r'); "\r" }
        Just('\\') -> { char('\\'); "\\" }
        Just('"') -> { char('"'); "\"" }
        Just(c) -> { char(c); c.string }
        Nothing -> fail("文字列が閉じられていません")
      val rest = build-string()
      escaped ++ rest
    Just(c) ->
      char(c)
      val rest = build-string()
      c.string ++ rest

// 数値のパース

fun parse-number(): <parse> json-value
  val sign = optional-sign()
  val int-part = digits()
  val frac-part = optional-frac()
  val exp-part = optional-exp()
  val num-str = sign ++ int-part ++ frac-part ++ exp-part
  match parse-float64(num-str)
    Just(num) -> JNumber(num)
    Nothing -> fail("不正な数値: " ++ num-str)

fun optional-sign(): <parse> string
  match peek-char()
    Just('-') -> { char('-'); "-" }
    Just('+') -> { char('+'); "+" }
    _ -> ""

fun digits(): <parse> string
  many1(fn() satisfy(is-digit)).string

fun optional-frac(): <parse> string
  match peek-char()
    Just('.') ->
      char('.')
      val frac = digits()
      "." ++ frac
    _ -> ""

fun optional-exp(): <parse> string
  match peek-char()
    Just('e') ->
      char('e')
      val sign = optional-sign()
      val exp-digits = digits()
      "e" ++ sign ++ exp-digits
    Just('E') ->
      char('E')
      val sign = optional-sign()
      val exp-digits = digits()
      "E" ++ sign ++ exp-digits
    _ -> ""

// 配列のパース（トレーリングカンマ対応）

fun parse-array(): <parse> json-value
  char('[')
  skip-ws-and-comments()
  match peek-char()
    Just(']') ->
      char(']')
      JArray([])
    _ ->
      val items = parse-array-elements()
      char(']')
      JArray(items)

fun parse-array-elements(): <parse> list<json-value>
  val value = parse-value()
  skip-ws-and-comments()
  match peek-char()
    Just(',') ->
      char(',')
      skip-ws-and-comments()
      match peek-char()
        Just(']') -> [value]  // トレーリングカンマ
        _ ->
          val rest = parse-array-elements()
          Cons(value, rest)
    Just(']') -> [value]
    _ -> fail("配列要素の後には ',' または ']' が必要です")

// オブジェクトのパース（トレーリングカンマ対応）

fun parse-object(): <parse> json-value
  char('{')
  skip-ws-and-comments()
  match peek-char()
    Just('}') ->
      char('}')
      JObject([])
    _ ->
      val pairs = parse-object-pairs()
      char('}')
      JObject(pairs)

fun parse-object-pairs(): <parse> list<(string, json-value)>
  val key-value = parse-string()
  val key = match key-value
    JString(s) -> s
    _ -> fail("オブジェクトのキーは文字列である必要があります")
  skip-ws-and-comments()
  char(':')
  skip-ws-and-comments()
  val value = parse-value()
  skip-ws-and-comments()
  match peek-char()
    Just(',') ->
      char(',')
      skip-ws-and-comments()
      match peek-char()
        Just('}') -> [(key, value)]  // トレーリングカンマ
        _ ->
          val rest = parse-object-pairs()
          Cons((key, value), rest)
    Just('}') -> [(key, value)]
    _ -> fail("オブジェクト要素の後には ',' または '}' が必要です")

// レンダリング

fun render-to-string(value: json-value, indent-level: int): string
  val indent = repeat-string("  ", indent-level)
  val next-indent = repeat-string("  ", indent-level + 1)
  match value
    JNull -> "null"
    JBool(True) -> "true"
    JBool(False) -> "false"
    JNumber(num) -> num.show
    JString(str) -> "\"" ++ str ++ "\""
    JArray(items) ->
      if items.is-nil then
        "[]"
      else
        val items-str = items.map(fn(item)
          next-indent ++ render-to-string(item, indent-level + 1)
        ).join(",\n")
        "[\n" ++ items-str ++ "\n" ++ indent ++ "]"
    JObject(pairs) ->
      if pairs.is-nil then
        "{}"
      else
        val pairs-str = pairs.map(fn((key, val))
          next-indent ++ "\"" ++ key ++ "\": " ++ render-to-string(val, indent-level + 1)
        ).join(",\n")
        "{\n" ++ pairs-str ++ "\n" ++ indent ++ "}"

fun repeat-string(s: string, n: int): string
  if n <= 0 then "" else s ++ repeat-string(s, n - 1)

// テスト

fun test-extended-json(): <console,exn> ()
  val test-cases = [
    ("コメント対応",
"""
{
  // これは行コメント
  "name": "test",
  /* これは
     ブロックコメント */
  "version": "1.0"
}
"""),
    ("トレーリングカンマ",
"""
{
  "items": [
    1,
    2,
    3,
  ],
  "config": {
    "debug": true,
    "port": 8080,
  }
}
""")
  ]
  test-cases.foreach fn((name, json-str))
    println("--- " ++ name ++ " ---")
    match parse(json-str)
      Right(value) ->
        println("パース成功:")
        println(render-to-string(value, 0))
      Left(err) ->
        println("パースエラー: " ++ err.show-error)
    println("")

fun show-error(err: parse-error): string
  match err
    UnexpectedEOF -> "予期しないEOF"
    InvalidValue(msg) -> "不正な値: " ++ msg
    UnclosedString -> "文字列が閉じられていません"
    UnclosedBlockComment -> "ブロックコメントが閉じられていません"
    ExpectedChar(ch) -> "'" ++ ch.string ++ "' が必要です"
    InvalidNumber(str) -> "不正な数値: " ++ str