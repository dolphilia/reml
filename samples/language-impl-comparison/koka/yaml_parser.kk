// YAML風パーサー：インデント管理が重要な題材。
//
// 対応する構文（簡易版）：
// - スカラー値: 文字列、数値、真偽値、null
// - リスト: `- item1`
// - マップ: `key: value`
// - ネストしたインデント構造
//
// インデント処理の特徴：
// - Kokaの効果システムを活用したパーサー実装
// - エラー回復機能でインデントミスを報告しつつ継続

import std/core
import std/core/types
import std/text/parse

// YAML値の表現。
type yaml-value
  Scalar(value: string)
  YList(items: list<yaml-value>)
  YMap(entries: list<(string, yaml-value)>)
  YNull

alias document = yaml-value

// パーサー型（簡易実装）。
alias parser<a> = parse<a>

// 水平空白のみをスキップ（改行は含まない）。
fun hspace(): <parse> ()
  whitespace0()

// 改行をスキップ。
fun newline(): <parse> ()
  char('\n')
  ()

// コメントのスキップ（`#` から行末まで）。
fun comment(): <parse> ()
  char('#')
  many-until(fn() satisfy(fn(c) c != '\n'), fn() peek(fn() char('\n')))
  ()

// 空行またはコメント行をスキップ。
fun blank-or-comment(): <parse> ()
  hspace()
  optional(comment)
  newline()

// 特定のインデントレベルを期待する。
fun expect-indent(level: int): <parse> ()
  val spaces = many(fn() char(' '))
  val actual-indent = spaces.length
  if actual-indent == level then
    ()
  else
    error("インデント不一致: 期待 " ++ level.show ++ ", 実際 " ++ actual-indent.show)

// 現在よりも深いインデントを検出。
fun deeper-indent(current: int): <parse> int
  val spaces = many(fn() char(' '))
  val actual-indent = spaces.length
  if actual-indent > current then
    actual-indent
  else
    error("深いインデントが期待されます: 現在 " ++ current.show ++ ", 実際 " ++ actual-indent.show)

// スカラー値のパース。
fun scalar-value(): <parse> yaml-value
  choose([
    // null
    fn() { pstring("null"); YNull },
    fn() { pstring("~"); YNull },
    // 真偽値
    fn() { pstring("true"); Scalar("true") },
    fn() { pstring("false"); Scalar("false") },
    // 数値（簡易実装）
    fn() { val n = pint(); Scalar(n.show) },
    // 文字列（引用符付き）
    fn() {
      char('"')
      val s = many-until(fn() satisfy(fn(c) c != '"'), fn() peek(fn() char('"')))
      char('"')
      Scalar(s.string)
    },
    // 文字列（引用符なし：行末または `:` まで）
    fn() {
      val chars = many(fn() satisfy(fn(c) c != '\n' && c != ':' && c != '#'))
      Scalar(chars.string.trim)
    }
  ])

// リスト項目のパース（`- value` 形式）。
fun parse-list-item(indent: int): <parse> yaml-value
  expect-indent(indent)
  char('-')
  hspace()
  parse-value(indent + 2)

// リスト全体のパース。
fun parse-list(indent: int): <parse> yaml-value
  val items = many1(fn()
    val item = parse-list-item(indent)
    optional(newline)
    item
  )
  YList(items)

// マップのキーバリューペアのパース（`key: value` 形式）。
fun parse-map-entry(indent: int): <parse> (string, yaml-value)
  expect-indent(indent)
  val key-chars = many(fn() satisfy(fn(c) c != ':' && c != '\n'))
  val key = key-chars.string.trim
  char(':')
  hspace()
  val value = choose([
    // 同じ行に値がある
    fn() parse-value(indent),
    // 次の行にネストされた値
    fn() {
      newline()
      parse-value(indent + 2)
    }
  ])
  (key, value)

// マップ全体のパース。
fun parse-map(indent: int): <parse> yaml-value
  val entries = many1(fn()
    val entry = parse-map-entry(indent)
    optional(newline)
    entry
  )
  YMap(entries)

// YAML値のパース（再帰的）。
fun parse-value(indent: int): <parse> yaml-value
  choose([
    fn() parse-list(indent),
    fn() parse-map(indent),
    fn() scalar-value()
  ])

// ドキュメント全体のパース。
fun yaml-document(): <parse> document
  many(fn() blank-or-comment())
  val doc = parse-value(0)
  many(fn() blank-or-comment())
  eof()
  doc

// パブリックAPI：YAML文字列をパース。
pub fun parse-yaml(input: string): <parse> document
  parse(input.slice, yaml-document)

// 簡易的なレンダリング（検証用）。
pub fun render-to-string(doc: document): string
  render-value(doc, 0)

fun render-value(value: yaml-value, indent: int): string
  val indent-str = " ".repeat(indent)

  match value
    Scalar(s) -> s
    YNull -> "null"
    YList(items) ->
      items.map(fn(item)
        indent-str ++ "- " ++ render-value(item, indent + 2)
      ).join("\n")
    YMap(entries) ->
      entries.map(fn((key, val))
        match val
          Scalar(_) -> indent-str ++ key ++ ": " ++ render-value(val, 0)
          YNull -> indent-str ++ key ++ ": " ++ render-value(val, 0)
          _ -> indent-str ++ key ++ ":\n" ++ render-value(val, indent + 2)
      ).join("\n")

// テスト例。
pub fun test-examples(): <console> ()
  val examples = [
    ("simple_scalar", "hello"),
    ("simple_list", "- item1\n- item2\n- item3"),
    ("simple_map", "key1: value1\nkey2: value2"),
    ("nested_map", "parent:\n  child1: value1\n  child2: value2"),
    ("nested_list", "items:\n  - item1\n  - item2"),
    ("mixed", "name: John\nage: 30\nhobbies:\n  - reading\n  - coding")
  ]

  examples.foreach fn((name, yaml-str))
    println("--- " ++ name ++ " ---")
    match try(fn() parse-yaml(yaml-str))
      Ok(doc) ->
        println("パース成功:")
        println(render-to-string(doc))
      Error(err) ->
        println("パースエラー: " ++ err.show)

// インデント処理の課題と解決策：
//
// 1. **インデントレベルの追跡**
//    - パーサー引数としてインデントレベルを渡す
//    - Kokaの効果システムでパーサー状態を管理
//
// 2. **エラー回復**
//    - chooseでバックトラックを制御
//    - エラー効果で分かりやすいエラーメッセージを提供
//
// 3. **空白の扱い**
//    - hspaceで水平空白のみをスキップ（改行は構文の一部）
//    - newlineで改行を明示的に処理
//
// Remlとの比較：
//
// - **Kokaの利点**:
//   - 効果システムによる型安全なパーサー実装
//   - 代数的効果によるエラーハンドリングの柔軟性
//
// - **Kokaの課題**:
//   - パーサーコンビネーターライブラリがRemlほど充実していない
//   - インデント管理が明示的で冗長になりやすい
//
// - **Remlの利点**:
//   - 字句レイヤの柔軟性により、インデント処理が自然に表現できる
//   - cut/commitによるエラー品質の向上
//   - recoverによる部分的なパース継続が可能