/** JSON拡張版：コメント・トレーリングカンマ対応。
  *
  * 標準JSONからの拡張点：
  * 1. コメント対応（`//` 行コメント、`/* */` ブロックコメント）
  * 2. トレーリングカンマ許可（配列・オブジェクトの最後の要素の後）
  * 3. より詳細なエラーメッセージ
  *
  * 実用的な設定ファイル形式として：
  * - `package.json` 風の設定ファイル
  * - `.babelrc`, `.eslintrc` など開発ツールの設定
  * - VS Code の `settings.json`
  */

import scala.collection.mutable

// 型定義

enum JsonValue:
  case JNull
  case JBool(value: Boolean)
  case JNumber(value: Double)
  case JString(value: String)
  case JArray(items: List[JsonValue])
  case JObject(pairs: Map[String, JsonValue])

enum ParseError:
  case UnexpectedEOF
  case InvalidValue(msg: String)
  case UnclosedString
  case UnclosedBlockComment
  case ExpectedChar(ch: Char)
  case InvalidNumber(str: String)

case class State(input: String, var pos: Int)

// パース

def parse(input: String): Either[ParseError, JsonValue] =
  val state = State(input, 0)

  for
    _ <- skipWhitespaceAndComments(state)
    value <- parseValue(state)
    _ <- skipWhitespaceAndComments(state)
    result <- if state.pos >= state.input.length then
      Right(value)
    else
      Left(ParseError.InvalidValue("入力の終端に到達していません"))
  yield result

// 空白とコメントをスキップ

def skipWhitespaceAndComments(state: State): Either[ParseError, Unit] =
  def loop(): Either[ParseError, Unit] =
    skipWs(state)
    if state.pos >= state.input.length then
      Right(())
    else
      val remaining = state.input.substring(state.pos)
      if remaining.startsWith("//") then
        skipLineComment(state)
        loop()
      else if remaining.startsWith("/*") then
        skipBlockComment(state) match
          case Left(err) => Left(err)
          case Right(_) => loop()
      else
        Right(())
  loop()

def skipWs(state: State): Unit =
  while state.pos < state.input.length && " \n\t\r".contains(state.input(state.pos)) do
    state.pos += 1

def skipLineComment(state: State): Unit =
  state.pos += 2 // "//" をスキップ
  while state.pos < state.input.length && state.input(state.pos) != '\n' do
    state.pos += 1
  if state.pos < state.input.length then
    state.pos += 1

def skipBlockComment(state: State): Either[ParseError, Unit] =
  state.pos += 2 // "/*" をスキップ
  while state.pos + 1 < state.input.length do
    if state.input.substring(state.pos, state.pos + 2) == "*/" then
      state.pos += 2
      return Right(())
    state.pos += 1
  Left(ParseError.UnclosedBlockComment)

// 値のパース

def parseValue(state: State): Either[ParseError, JsonValue] =
  skipWhitespaceAndComments(state) match
    case Left(err) => Left(err)
    case Right(_) =>
      if state.pos >= state.input.length then
        Left(ParseError.UnexpectedEOF)
      else
        val remaining = state.input.substring(state.pos)
        if remaining.startsWith("null") then
          state.pos += 4
          Right(JsonValue.JNull)
        else if remaining.startsWith("true") then
          state.pos += 4
          Right(JsonValue.JBool(true))
        else if remaining.startsWith("false") then
          state.pos += 5
          Right(JsonValue.JBool(false))
        else if remaining.startsWith("\"") then
          parseString(state)
        else if remaining.startsWith("[") then
          parseArray(state)
        else if remaining.startsWith("{") then
          parseObject(state)
        else
          parseNumber(state)

// 文字列リテラルのパース

def parseString(state: State): Either[ParseError, JsonValue] =
  state.pos += 1 // '"' をスキップ
  val result = StringBuilder()

  while state.pos < state.input.length do
    state.input(state.pos) match
      case '"' =>
        state.pos += 1
        return Right(JsonValue.JString(result.toString))
      case '\\' if state.pos + 1 < state.input.length =>
        state.pos += 1
        val escaped = state.input(state.pos) match
          case 'n' => '\n'
          case 't' => '\t'
          case 'r' => '\r'
          case '\\' => '\\'
          case '"' => '"'
          case ch => ch
        result.append(escaped)
        state.pos += 1
      case ch =>
        result.append(ch)
        state.pos += 1

  Left(ParseError.UnclosedString)

// 数値のパース

def parseNumber(state: State): Either[ParseError, JsonValue] =
  val start = state.pos

  while state.pos < state.input.length do
    val ch = state.input(state.pos)
    if ch == '-' || ch == '+' || ch == '.' || ch == 'e' || ch == 'E' || ch.isDigit then
      state.pos += 1
    else
      ()
      return parseNumberHelper(state, start)

  parseNumberHelper(state, start)

def parseNumberHelper(state: State, start: Int): Either[ParseError, JsonValue] =
  val numStr = state.input.substring(start, state.pos)
  try
    Right(JsonValue.JNumber(numStr.toDouble))
  catch
    case _: NumberFormatException => Left(ParseError.InvalidNumber(numStr))

// 配列のパース（トレーリングカンマ対応）

def parseArray(state: State): Either[ParseError, JsonValue] =
  state.pos += 1 // '[' をスキップ
  skipWhitespaceAndComments(state) match
    case Left(err) => Left(err)
    case Right(_) =>
      if state.pos < state.input.length && state.input(state.pos) == ']' then
        state.pos += 1
        Right(JsonValue.JArray(List.empty))
      else
        parseArrayElements(state, List.empty)

def parseArrayElements(state: State, acc: List[JsonValue]): Either[ParseError, JsonValue] =
  parseValue(state) match
    case Left(err) => Left(err)
    case Right(value) =>
      skipWhitespaceAndComments(state) match
        case Left(err) => Left(err)
        case Right(_) =>
          val newAcc = acc :+ value
          if state.pos >= state.input.length then
            Left(ParseError.UnexpectedEOF)
          else
            state.input(state.pos) match
              case ',' =>
                state.pos += 1
                skipWhitespaceAndComments(state) match
                  case Left(err) => Left(err)
                  case Right(_) =>
                    if state.pos < state.input.length && state.input(state.pos) == ']' then
                      // トレーリングカンマ
                      state.pos += 1
                      Right(JsonValue.JArray(newAcc))
                    else
                      parseArrayElements(state, newAcc)
              case ']' =>
                state.pos += 1
                Right(JsonValue.JArray(newAcc))
              case _ =>
                Left(ParseError.ExpectedChar(','))

// オブジェクトのパース（トレーリングカンマ対応）

def parseObject(state: State): Either[ParseError, JsonValue] =
  state.pos += 1 // '{' をスキップ
  skipWhitespaceAndComments(state) match
    case Left(err) => Left(err)
    case Right(_) =>
      if state.pos < state.input.length && state.input(state.pos) == '}' then
        state.pos += 1
        Right(JsonValue.JObject(Map.empty))
      else
        parseObjectPairs(state, Map.empty)

def parseObjectPairs(state: State, acc: Map[String, JsonValue]): Either[ParseError, JsonValue] =
  parseString(state) match
    case Left(err) => Left(err)
    case Right(JsonValue.JString(key)) =>
      skipWhitespaceAndComments(state) match
        case Left(err) => Left(err)
        case Right(_) =>
          if state.pos >= state.input.length || state.input(state.pos) != ':' then
            Left(ParseError.ExpectedChar(':'))
          else
            state.pos += 1
            skipWhitespaceAndComments(state) match
              case Left(err) => Left(err)
              case Right(_) =>
                parseValue(state) match
                  case Left(err) => Left(err)
                  case Right(value) =>
                    skipWhitespaceAndComments(state) match
                      case Left(err) => Left(err)
                      case Right(_) =>
                        val newAcc = acc + (key -> value)
                        if state.pos >= state.input.length then
                          Left(ParseError.UnexpectedEOF)
                        else
                          state.input(state.pos) match
                            case ',' =>
                              state.pos += 1
                              skipWhitespaceAndComments(state) match
                                case Left(err) => Left(err)
                                case Right(_) =>
                                  if state.pos < state.input.length && state.input(state.pos) == '}' then
                                    // トレーリングカンマ
                                    state.pos += 1
                                    Right(JsonValue.JObject(newAcc))
                                  else
                                    parseObjectPairs(state, newAcc)
                            case '}' =>
                              state.pos += 1
                              Right(JsonValue.JObject(newAcc))
                            case _ =>
                              Left(ParseError.ExpectedChar(','))
    case Right(_) => Left(ParseError.InvalidValue("オブジェクトのキーは文字列である必要があります"))

// レンダリング

def renderToString(value: JsonValue, indentLevel: Int = 0): String =
  val indent = "  " * indentLevel
  val nextIndent = "  " * (indentLevel + 1)

  value match
    case JsonValue.JNull => "null"
    case JsonValue.JBool(true) => "true"
    case JsonValue.JBool(false) => "false"
    case JsonValue.JNumber(num) => num.toString
    case JsonValue.JString(str) => s""""$str""""
    case JsonValue.JArray(items) =>
      if items.isEmpty then
        "[]"
      else
        val itemsStr = items
          .map(item => s"$nextIndent${renderToString(item, indentLevel + 1)}")
          .mkString(",\n")
        s"[\n$itemsStr\n$indent]"
    case JsonValue.JObject(pairs) =>
      if pairs.isEmpty then
        "{}"
      else
        val pairsStr = pairs
          .map((key, value) => s"""$nextIndent"$key": ${renderToString(value, indentLevel + 1)}""")
          .mkString(",\n")
        s"{\n$pairsStr\n$indent}"

// テスト

def testExtendedJson(): Unit =
  val testCases = List(
    ("コメント対応", """
{
  // これは行コメント
  "name": "test",
  /* これは
     ブロックコメント */
  "version": "1.0"
}
"""),
    ("トレーリングカンマ", """
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
  )

  testCases.foreach { (name, jsonStr) =>
    println(s"--- $name ---")
    parse(jsonStr) match
      case Right(value) =>
        println("パース成功:")
        println(renderToString(value, 0))
      case Left(err) =>
        println(s"パースエラー: $err")
    println()
  }

@main def main(): Unit =
  testExtendedJson()