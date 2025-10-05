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

// 正規表現のAST
enum Regex:
  case Literal(s: String)
  case CharClass(cs: CharSet)
  case Dot
  case Concat(terms: List[Regex])
  case Alternation(alts: List[Regex])
  case Repeat(inner: Regex, kind: RepeatKind)
  case Group(inner: Regex)
  case Anchor(kind: AnchorKind)

enum CharSet:
  case CharRange(start: Char, end: Char)
  case CharList(chars: List[Char])
  case Predefined(cls: PredefinedClass)
  case Negated(inner: CharSet)
  case Union(sets: List[CharSet])

enum PredefinedClass:
  case Digit
  case Word
  case Whitespace
  case NotDigit
  case NotWord
  case NotWhitespace

enum RepeatKind:
  case ZeroOrMore
  case OneOrMore
  case ZeroOrOne
  case Exactly(n: Int)
  case Range(min: Int, max: Option[Int])

enum AnchorKind:
  case Start
  case End

// パーサー型
type ParseResult[T] = Either[String, (T, String)]

object Parser:
  def ok[T](value: T, rest: String): ParseResult[T] = Right((value, rest))
  def fail[T](msg: String): ParseResult[T] = Left(msg)

  def choice[T](parsers: List[String => ParseResult[T]])(input: String): ParseResult[T] =
    parsers match
      case Nil => fail("no choice matched")
      case p :: ps =>
        p(input) match
          case Right(result) => Right(result)
          case Left(_) => choice(ps)(input)

  def many[T](parser: String => ParseResult[T])(input: String): ParseResult[(List[T], String)] =
    def loop(acc: List[T], rest: String): (List[T], String) =
      parser(rest) match
        case Right((value, newRest)) => loop(value :: acc, newRest)
        case Left(_) => (acc.reverse, rest)
    val (values, rest) = loop(Nil, input)
    ok(values, rest)

  def many1[T](parser: String => ParseResult[T])(input: String): ParseResult[(List[T], String)] =
    for
      (first, rest1) <- parser(input)
      (others, rest2) <- many(parser)(rest1)
    yield (first :: others, rest2)

  def optional[T](parser: String => ParseResult[T])(input: String): ParseResult[(Option[T], String)] =
    parser(input) match
      case Right((value, rest)) => ok(Some(value), rest)
      case Left(_) => ok(None, input)

  def char(c: Char)(input: String): ParseResult[(Char, String)] =
    if input.nonEmpty && input.head == c then
      ok(c, input.tail)
    else
      fail(s"expected $c")

  def string(s: String)(input: String): ParseResult[(String, String)] =
    if input.startsWith(s) then
      ok(s, input.drop(s.length))
    else
      fail(s"expected $s")

  def satisfy(pred: Char => Boolean)(input: String): ParseResult[(Char, String)] =
    if input.nonEmpty && pred(input.head) then
      ok(input.head, input.tail)
    else
      fail("predicate failed")

  def digit(input: String): ParseResult[(Char, String)] =
    satisfy(_.isDigit)(input)

  def integer(input: String): ParseResult[(Int, String)] =
    for (digits, rest) <- many1(digit)(input)
    yield (digits.mkString.toInt, rest)

  def sepBy1[T, S](parser: String => ParseResult[T], sep: String => ParseResult[S])(
      input: String
  ): ParseResult[(List[T], String)] =
    for
      (first, rest1) <- parser(input)
      (others, rest2) <- many { inp =>
        for
          (_, r1) <- sep(inp)
          (value, r2) <- parser(r1)
        yield (value, r2)
      }(rest1)
    yield (first :: others, rest2)

// 正規表現パーサー
object RegexParser:
  import Parser.*

  def parseRegex(input: String): Either[String, Regex] =
    regexExpr(input) match
      case Right((regex, "")) => Right(regex)
      case Right((_, rest)) => Left(s"unexpected input: $rest")
      case Left(err) => Left(err)

  def regexExpr(input: String): ParseResult[Regex] =
    alternationExpr(input)

  def alternationExpr(input: String): ParseResult[Regex] =
    for (alts, rest) <- sepBy1(concatExpr, string("|"))(input)
    yield
      (
        alts match
          case single :: Nil => single
          case _ => Regex.Alternation(alts)
        ,
        rest
      )

  def concatExpr(input: String): ParseResult[Regex] =
    for (terms, rest) <- many1(postfixTerm)(input)
    yield
      (
        terms match
          case single :: Nil => single
          case _ => Regex.Concat(terms)
        ,
        rest
      )

  def postfixTerm(input: String): ParseResult[Regex] =
    for
      (base, rest1) <- atom(input)
      (repeatOpt, rest2) <- optional(repeatSuffix)(rest1)
    yield
      (
        repeatOpt match
          case Some(kind) => Regex.Repeat(base, kind)
          case None => base
        ,
        rest2
      )

  def atom(input: String): ParseResult[Regex] =
    // 括弧グループ
    string("(")(input)
      .flatMap { case (_, rest1) =>
        regexExpr(rest1).flatMap { case (inner, rest2) =>
          string(")")(rest2).map { case (_, rest3) =>
            (Regex.Group(inner), rest3)
          }
        }
      }
      .orElse {
        // アンカー
        string("^")(input).map { case (_, rest) => (Regex.Anchor(AnchorKind.Start), rest) }
      }
      .orElse {
        string("$")(input).map { case (_, rest) => (Regex.Anchor(AnchorKind.End), rest) }
      }
      .orElse {
        // ドット
        string(".")(input).map { case (_, rest) => (Regex.Dot, rest) }
      }
      .orElse {
        // 文字クラス
        charClass(input)
      }
      .orElse {
        // 定義済みクラス
        predefinedClass(input)
      }
      .orElse {
        // エスケープ文字
        escapeChar(input)
      }
      .orElse {
        // 通常のリテラル
        satisfy(c =>
          c != '(' && c != ')' && c != '[' && c != ']' && c != '{' && c != '}' &&
            c != '*' && c != '+' && c != '?' && c != '.' && c != '|' &&
            c != '^' && c != '$' && c != '\\'
        )(input).map { case (c, rest) =>
          (Regex.Literal(c.toString), rest)
        }
      }

  def escapeChar(input: String): ParseResult[Regex] =
    for
      (_, rest1) <- string("\\")(input)
      (c, rest2) <- satisfy(ch =>
        ch == 'n' || ch == 't' || ch == 'r' || ch == '\\' ||
          ch == '(' || ch == ')' || ch == '[' || ch == ']' ||
          ch == '{' || ch == '}' || ch == '*' || ch == '+' ||
          ch == '?' || ch == '.' || ch == '|' || ch == '^' || ch == '$'
      )(rest1)
    yield
      val lit = c match
        case 'n' => "\n"
        case 't' => "\t"
        case 'r' => "\r"
        case _ => c.toString
      (Regex.Literal(lit), rest2)

  def predefinedClass(input: String): ParseResult[Regex] =
    for
      (_, rest1) <- string("\\")(input)
      result <- choice(
        List(
          char('d')(_).map { case (_, r) => (Regex.CharClass(CharSet.Predefined(PredefinedClass.Digit)), r) },
          char('w')(_).map { case (_, r) => (Regex.CharClass(CharSet.Predefined(PredefinedClass.Word)), r) },
          char('s')(_).map { case (_, r) => (Regex.CharClass(CharSet.Predefined(PredefinedClass.Whitespace)), r) },
          char('D')(_).map { case (_, r) => (Regex.CharClass(CharSet.Predefined(PredefinedClass.NotDigit)), r) },
          char('W')(_).map { case (_, r) => (Regex.CharClass(CharSet.Predefined(PredefinedClass.NotWord)), r) },
          char('S')(_).map { case (_, r) => (Regex.CharClass(CharSet.Predefined(PredefinedClass.NotWhitespace)), r) }
        )
      )(rest1)
    yield result

  def charClass(input: String): ParseResult[Regex] =
    for
      (_, rest1) <- string("[")(input)
      (negated, rest2) <- optional(string("^"))(rest1)
      (items, rest3) <- many1(charClassItem)(rest2)
      (_, rest4) <- string("]")(rest3)
    yield
      val unionSet = CharSet.Union(items)
      val cs = if negated.isDefined then CharSet.Negated(unionSet) else unionSet
      (Regex.CharClass(cs), rest4)

  def charClassItem(input: String): ParseResult[CharSet] =
    for
      (start, rest1) <- satisfy(c => c != ']' && c != '-')(input)
      (endOpt, rest2) <- optional { inp: String =>
        for
          (_, r1) <- string("-")(inp)
          (end, r2) <- satisfy(_ != ']')(r1)
        yield (end, r2)
      }(rest1)
    yield
      (
        endOpt match
          case Some(end) => CharSet.CharRange(start, end)
          case None => CharSet.CharList(List(start))
        ,
        rest2
      )

  def repeatSuffix(input: String): ParseResult[RepeatKind] =
    string("*")(input)
      .map { case (_, rest) => (RepeatKind.ZeroOrMore, rest) }
      .orElse {
        string("+")(input).map { case (_, rest) => (RepeatKind.OneOrMore, rest) }
      }
      .orElse {
        string("?")(input).map { case (_, rest) => (RepeatKind.ZeroOrOne, rest) }
      }
      .orElse {
        // {n,m} 形式
        for
          (_, rest1) <- string("{")(input)
          (n, rest2) <- integer(rest1)
          (rangeOpt, rest3) <- optional { inp: String =>
            for
              (_, r1) <- string(",")(inp)
              (mOpt, r2) <- optional(integer)(r1)
            yield (mOpt, r2)
          }(rest2)
          (_, rest4) <- string("}")(rest3)
        yield
          (
            rangeOpt match
              case None => RepeatKind.Exactly(n)
              case Some(None) => RepeatKind.Range(n, None)
              case Some(Some(m)) => RepeatKind.Range(n, Some(m))
            ,
            rest4
          )
      }

// マッチングエンジン
object RegexMatcher:
  def matchRegex(regex: Regex, text: String): Boolean =
    matchFromPos(regex, text, 0)

  def matchFromPos(regex: Regex, text: String, pos: Int): Boolean =
    regex match
      case Regex.Literal(s) =>
        if pos + s.length <= text.length then
          text.substring(pos, pos + s.length) == s
        else false

      case Regex.CharClass(cs) =>
        if pos < text.length then charMatchesClass(text(pos), cs)
        else false

      case Regex.Dot =>
        pos < text.length

      case Regex.Concat(terms) =>
        var currentPos = pos
        terms.forall { term =>
          val matched = matchFromPos(term, text, currentPos)
          if matched then currentPos += 1
          matched
        }

      case Regex.Alternation(alts) =>
        alts.exists(alt => matchFromPos(alt, text, pos))

      case Regex.Repeat(inner, kind) =>
        kind match
          case RepeatKind.ZeroOrMore => matchRepeatLoop(inner, text, pos, 0, 0, 999999)
          case RepeatKind.OneOrMore =>
            if matchFromPos(inner, text, pos) then
              matchRepeatLoop(inner, text, pos + 1, 1, 1, 999999)
            else false
          case RepeatKind.ZeroOrOne => matchFromPos(inner, text, pos) || true
          case RepeatKind.Exactly(n) => matchRepeatLoop(inner, text, pos, 0, n, n)
          case RepeatKind.Range(min, maxOpt) =>
            val max = maxOpt.getOrElse(999999)
            matchRepeatLoop(inner, text, pos, 0, min, max)

      case Regex.Group(inner) =>
        matchFromPos(inner, text, pos)

      case Regex.Anchor(kind) =>
        kind match
          case AnchorKind.Start => pos == 0
          case AnchorKind.End => pos >= text.length

  def charMatchesClass(c: Char, cs: CharSet): Boolean =
    cs match
      case CharSet.CharRange(start, end) => c >= start && c <= end
      case CharSet.CharList(chars) => chars.contains(c)
      case CharSet.Predefined(cls) =>
        cls match
          case PredefinedClass.Digit => c.isDigit
          case PredefinedClass.Word => c.isLetterOrDigit || c == '_'
          case PredefinedClass.Whitespace => c.isWhitespace
          case PredefinedClass.NotDigit => !c.isDigit
          case PredefinedClass.NotWord => !(c.isLetterOrDigit || c == '_')
          case PredefinedClass.NotWhitespace => !c.isWhitespace
      case CharSet.Negated(inner) => !charMatchesClass(c, inner)
      case CharSet.Union(sets) => sets.exists(set => charMatchesClass(c, set))

  def matchRepeatLoop(
      inner: Regex,
      text: String,
      pos: Int,
      count: Int,
      min: Int,
      max: Int
  ): Boolean =
    if count == max then true
    else if count >= min && !matchFromPos(inner, text, pos) then true
    else if matchFromPos(inner, text, pos) then
      matchRepeatLoop(inner, text, pos + 1, count + 1, min, max)
    else if count >= min then true
    else false

// テスト例
@main def testRegexEngine(): Unit =
  val examples = List(
    ("a+", "aaa", true),
    ("a+", "b", false),
    ("[0-9]+", "123", true),
    ("[0-9]+", "abc", false),
    ("\\d{2,4}", "12", true),
    ("\\d{2,4}", "12345", true),
    ("(abc)+", "abcabc", true),
    ("a|b", "a", true),
    ("a|b", "b", true),
    ("a|b", "c", false),
    ("^hello$", "hello", true),
    ("^hello$", "hello world", false)
  )

  examples.foreach { case (pattern, text, expected) =>
    RegexParser.parseRegex(pattern) match
      case Right(regex) =>
        val result = RegexMatcher.matchRegex(regex, text)
        val status = if result == expected then "✓" else "✗"
        println(s"$status パターン: '$pattern', テキスト: '$text', 期待: $expected, 結果: $result")
      case Left(err) =>
        println(s"✗ パーサーエラー: $pattern - $err")
  }