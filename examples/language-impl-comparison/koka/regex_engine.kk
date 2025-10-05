// 正規表現エンジン：パース + 評価の両方を実装。
//
// 対応する正規表現構文（簡易版）：
// - リテラル: `abc`
// - 連結: `ab`
// - 選択: `a|b`
// - 繰り返し: `a*`, `a+`, `a?`, `a{2,5}`
// - グループ: `(abc)`
// - 文字クラス: `[a-z]`, `[^0-9]`, `\d`, `\w`, `\s`
// - アンカー: `^`, `$`
// - ドット: `.` (任意の1文字)

import std/core
import std/text/parse

// 正規表現のAST
type regex
  Literal(s: string)
  CharClass(cs: charset)
  Dot
  Concat(terms: list<regex>)
  Alternation(alts: list<regex>)
  Repeat(inner: regex, kind: repeat-kind)
  Group(inner: regex)
  Anchor(kind: anchor-kind)

type charset
  CharRange(start: char, end: char)
  CharList(chars: list<char>)
  Predefined(cls: predefined-class)
  Negated(inner: charset)
  Union(sets: list<charset>)

type predefined-class
  Digit
  Word
  Whitespace
  NotDigit
  NotWord
  NotWhitespace

type repeat-kind
  ZeroOrMore
  OneOrMore
  ZeroOrOne
  Exactly(n: int)
  Range(min: int, max: maybe<int>)

type anchor-kind
  Start
  End

// パーサーコンビネーター
fun parse-regex(input: string): <parse,exn> regex
  input.slice.parse-eof(regex-expr)

fun regex-expr(): <parse> regex
  alternation-expr()

fun alternation-expr(): <parse> regex
  val alts = sep-by1(concat-expr, pstring("|"))
  match alts
    [single] -> single
    _ -> Alternation(alts)

fun concat-expr(): <parse> regex
  val terms = many1(postfix-term)
  match terms
    [single] -> single
    _ -> Concat(terms)

fun postfix-term(): <parse> regex
  val base = atom()
  match optional(repeat-suffix)
    Just(kind) -> Repeat(base, kind)
    Nothing -> base

fun atom(): <parse> regex
  choose([
    group-parser,
    anchor-start,
    anchor-end,
    dot-parser,
    char-class-parser,
    predefined-class-parser,
    escape-char-parser,
    literal-parser
  ])

fun group-parser(): <parse> regex
  pstring("(")
  val inner = regex-expr()
  pstring(")")
  Group(inner)

fun anchor-start(): <parse> regex
  pstring("^")
  Anchor(Start)

fun anchor-end(): <parse> regex
  pstring("$")
  Anchor(End)

fun dot-parser(): <parse> regex
  pstring(".")
  Dot

fun escape-char-parser(): <parse> regex
  pstring("\\")
  val c = satisfy(fn(ch) ch.is-one-of("ntr\\()[]{}*+?.|^$"))
  val result = match c
    'n' -> "\n"
    't' -> "\t"
    'r' -> "\r"
    _ -> c.string
  Literal(result)

fun predefined-class-parser(): <parse> regex
  pstring("\\")
  val cls = choose([
    { pchar('d'); Digit },
    { pchar('w'); Word },
    { pchar('s'); Whitespace },
    { pchar('D'); NotDigit },
    { pchar('W'); NotWord },
    { pchar('S'); NotWhitespace }
  ])
  CharClass(Predefined(cls))

fun char-class-parser(): <parse> regex
  pstring("[")
  val negated = optional(pstring("^"))
  val items = many1(char-class-item)
  pstring("]")
  val union-set = Union(items)
  val result = match negated
    Just(_) -> Negated(union-set)
    Nothing -> union-set
  CharClass(result)

fun char-class-item(): <parse> charset
  choose([
    char-range-parser,
    single-char-parser
  ])

fun char-range-parser(): <parse> charset
  val start = satisfy(fn(c) c != ']' && c != '-')
  match optional({ pstring("-"); satisfy(fn(c) c != ']') })
    Just(end) -> CharRange(start, end)
    Nothing -> CharList([start])

fun single-char-parser(): <parse> charset
  val c = satisfy(fn(ch) ch != ']')
  CharList([c])

fun literal-parser(): <parse> regex
  val c = satisfy(fn(ch) !ch.is-one-of("()[]{}*+?.|^$\\"))
  Literal(c.string)

fun repeat-suffix(): <parse> repeat-kind
  choose([
    { pstring("*"); ZeroOrMore },
    { pstring("+"); OneOrMore },
    { pstring("?"); ZeroOrOne },
    braced-repeat
  ])

fun braced-repeat(): <parse> repeat-kind
  pstring("{")
  val n = pint()
  val range-opt = optional({
    pstring(",")
    optional(pint)
  })
  pstring("}")
  match range-opt
    Nothing -> Exactly(n)
    Just(Nothing) -> Range(n, Nothing)
    Just(Just(m)) -> Range(n, Just(m))

// ユーティリティ
fun is-one-of(c: char, chars: string): bool
  chars.list.any(fn(ch) ch == c)

// マッチングエンジン
fun match-regex(regex: regex, text: string): <div> bool
  match-from-pos(regex, text, 0)

fun match-from-pos(regex: regex, text: string, pos: int): <div> bool
  match regex
    Literal(s) ->
      text.slice.advance(pos).starts-with(s).bool

    CharClass(cs) ->
      match text.slice.advance(pos).next
        Just((c, _)) -> char-matches-class(c, cs)
        Nothing -> False

    Dot ->
      match text.slice.advance(pos).next
        Just(_) -> True
        Nothing -> False

    Concat(terms) ->
      match-concat(terms, text, pos)

    Alternation(alts) ->
      alts.any(fn(alt) match-from-pos(alt, text, pos))

    Repeat(inner, kind) ->
      match kind
        ZeroOrMore -> match-repeat-zero-or-more(inner, text, pos)
        OneOrMore -> match-repeat-one-or-more(inner, text, pos)
        ZeroOrOne -> match-repeat-zero-or-one(inner, text, pos)
        Exactly(n) -> match-repeat-exactly(inner, text, pos, n)
        Range(min-count, max-opt) -> match-repeat-range(inner, text, pos, min-count, max-opt)

    Group(inner) ->
      match-from-pos(inner, text, pos)

    Anchor(kind) ->
      match kind
        Start -> pos == 0
        End -> pos >= text.count

fun match-concat(terms: list<regex>, text: string, pos: int): <div> bool
  match terms
    [] -> True
    Cons(term, rest) ->
      if match-from-pos(term, text, pos) then
        match-concat(rest, text, pos + 1)
      else
        False

fun char-matches-class(c: char, cs: charset): bool
  match cs
    CharRange(start, end) ->
      c >= start && c <= end

    CharList(chars) ->
      chars.any(fn(ch) ch == c)

    Predefined(cls) ->
      match cls
        Digit -> c.is-digit
        Word -> c.is-alpha-num || c == '_'
        Whitespace -> c.is-white
        NotDigit -> !c.is-digit
        NotWord -> !(c.is-alpha-num || c == '_')
        NotWhitespace -> !c.is-white

    Negated(inner) ->
      !char-matches-class(c, inner)

    Union(sets) ->
      sets.any(fn(set) char-matches-class(c, set))

fun match-repeat-zero-or-more(inner: regex, text: string, pos: int): <div> bool
  match-repeat-loop(inner, text, pos, 0, 0, 999999)

fun match-repeat-one-or-more(inner: regex, text: string, pos: int): <div> bool
  if match-from-pos(inner, text, pos) then
    match-repeat-zero-or-more(inner, text, pos + 1)
  else
    False

fun match-repeat-zero-or-one(inner: regex, text: string, pos: int): <div> bool
  match-from-pos(inner, text, pos) || True

fun match-repeat-exactly(inner: regex, text: string, pos: int, n: int): <div> bool
  match-repeat-loop(inner, text, pos, 0, n, n)

fun match-repeat-range(inner: regex, text: string, pos: int, min-count: int, max-opt: maybe<int>): <div> bool
  val max-count = match max-opt
    Just(m) -> m
    Nothing -> 999999
  match-repeat-loop(inner, text, pos, 0, min-count, max-count)

fun match-repeat-loop(inner: regex, text: string, pos: int, count: int, min-count: int, max-count: int): <div> bool
  if count == max-count then
    True
  elif count >= min-count && !match-from-pos(inner, text, pos) then
    True
  elif match-from-pos(inner, text, pos) then
    match-repeat-loop(inner, text, pos + 1, count + 1, min-count, max-count)
  elif count >= min-count then
    True
  else
    False

// テスト例
fun test-examples(): <console,parse,exn,div> ()
  val examples = [
    ("a+", "aaa", True),
    ("a+", "b", False),
    ("[0-9]+", "123", True),
    ("[0-9]+", "abc", False),
    ("(abc)+", "abcabc", True),
    ("a|b", "a", True),
    ("a|b", "b", True),
    ("a|b", "c", False),
    ("^hello$", "hello", True),
    ("^hello$", "hello world", False)
  ]

  examples.foreach fn((pattern, text, expected))
    match try({ Just(parse-regex(pattern)) })
      Just(regex) ->
        val result = match-regex(regex, text)
        val status = if result == expected then "✓" else "✗"
        println(status ++ " パターン: '" ++ pattern ++ "', テキスト: '" ++ text ++ "', 期待: " ++ expected.show ++ ", 結果: " ++ result.show)
      Nothing ->
        println("✗ パーサーエラー: " ++ pattern)

fun main()
  test-examples()