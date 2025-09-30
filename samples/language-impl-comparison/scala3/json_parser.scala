// JSON パーサー (Scala 3 実装)
// JSON 構文を解析して汎用値型に変換する

package jsonParser

import scala.util.{Try, Success, Failure}

// JSON 値型
enum JsonValue:
  case JNull
  case JBool(value: Boolean)
  case JNumber(value: Double)
  case JString(value: String)
  case JArray(items: List[JsonValue])
  case JObject(fields: Map[String, JsonValue])

// トークン型
enum Token:
  case LBrace
  case RBrace
  case LBracket
  case RBracket
  case Colon
  case Comma
  case StringLiteral(value: String)
  case NumberLiteral(value: Double)
  case BoolLiteral(value: Boolean)
  case NullLiteral

// パース状態
case class ParseState(tokens: List[Token])

// パースエラー
enum ParseError:
  case UnexpectedEOF
  case UnexpectedToken(expected: String, found: Token)

// トークン化
def tokenize(source: String): List[Token] =
  def loop(index: Int, acc: List[Token]): List[Token] =
    if index >= source.length then
      acc.reverse
    else
      source(index) match
        case ' ' | '\n' | '\t' | '\r' => loop(index + 1, acc)
        case '{' => loop(index + 1, Token.LBrace :: acc)
        case '}' => loop(index + 1, Token.RBrace :: acc)
        case '[' => loop(index + 1, Token.LBracket :: acc)
        case ']' => loop(index + 1, Token.RBracket :: acc)
        case ':' => loop(index + 1, Token.Colon :: acc)
        case ',' => loop(index + 1, Token.Comma :: acc)
        case 't' =>
          if source.substring(index, index + 4) == "true" then
            loop(index + 4, Token.BoolLiteral(true) :: acc)
          else
            loop(index + 1, acc)
        case 'f' =>
          if source.substring(index, index + 5) == "false" then
            loop(index + 5, Token.BoolLiteral(false) :: acc)
          else
            loop(index + 1, acc)
        case 'n' =>
          if source.substring(index, index + 4) == "null" then
            loop(index + 4, Token.NullLiteral :: acc)
          else
            loop(index + 1, acc)
        case '"' =>
          val endIndex = source.indexOf('"', index + 1)
          val str = source.substring(index + 1, endIndex)
          loop(endIndex + 1, Token.StringLiteral(str) :: acc)
        case _ =>
          // 数値の読み取り (簡易実装)
          var endIndex = index
          while endIndex < source.length &&
                (source(endIndex).isDigit || source(endIndex) == '.' || source(endIndex) == '-') do
            endIndex += 1
          val numStr = source.substring(index, endIndex)
          Try(numStr.toDouble) match
            case Success(num) => loop(endIndex, Token.NumberLiteral(num) :: acc)
            case Failure(_) => loop(index + 1, acc)
  loop(0, Nil)

// 値のパース
def parseValue(state: ParseState): Either[ParseError, (JsonValue, ParseState)] =
  state.tokens match
    case Nil => Left(ParseError.UnexpectedEOF)
    case token :: rest =>
      token match
        case Token.NullLiteral => Right((JsonValue.JNull, ParseState(rest)))
        case Token.BoolLiteral(flag) => Right((JsonValue.JBool(flag), ParseState(rest)))
        case Token.NumberLiteral(num) => Right((JsonValue.JNumber(num), ParseState(rest)))
        case Token.StringLiteral(text) => Right((JsonValue.JString(text), ParseState(rest)))
        case Token.LBracket => parseArray(ParseState(rest))
        case Token.LBrace => parseObject(ParseState(rest))
        case other => Left(ParseError.UnexpectedToken("値", other))

// 配列のパース
def parseArray(state: ParseState): Either[ParseError, (JsonValue, ParseState)] =
  state.tokens match
    case Token.RBracket :: rest => Right((JsonValue.JArray(Nil), ParseState(rest)))
    case _ =>
      def loop(current: ParseState, acc: List[JsonValue]): Either[ParseError, (JsonValue, ParseState)] =
        parseValue(current) match
          case Left(err) => Left(err)
          case Right((value, next)) =>
            val newAcc = acc :+ value
            next.tokens match
              case Token.Comma :: rest => loop(ParseState(rest), newAcc)
              case Token.RBracket :: rest => Right((JsonValue.JArray(newAcc), ParseState(rest)))
              case token :: _ => Left(ParseError.UnexpectedToken("]", token))
              case Nil => Left(ParseError.UnexpectedEOF)
      loop(state, Nil)

// オブジェクトのパース
def parseObject(state: ParseState): Either[ParseError, (JsonValue, ParseState)] =
  state.tokens match
    case Token.RBrace :: rest => Right((JsonValue.JObject(Map.empty), ParseState(rest)))
    case _ =>
      def loop(current: ParseState, acc: Map[String, JsonValue]): Either[ParseError, (JsonValue, ParseState)] =
        current.tokens match
          case Token.StringLiteral(key) :: Token.Colon :: rest =>
            parseValue(ParseState(rest)) match
              case Left(err) => Left(err)
              case Right((value, next)) =>
                val newAcc = acc + (key -> value)
                next.tokens match
                  case Token.Comma :: rest2 => loop(ParseState(rest2), newAcc)
                  case Token.RBrace :: rest2 => Right((JsonValue.JObject(newAcc), ParseState(rest2)))
                  case token :: _ => Left(ParseError.UnexpectedToken("}", token))
                  case Nil => Left(ParseError.UnexpectedEOF)
          case token :: _ => Left(ParseError.UnexpectedToken("文字列", token))
          case Nil => Left(ParseError.UnexpectedEOF)
      loop(state, Map.empty)

// メインパース関数
def parseJson(source: String): Either[String, JsonValue] =
  val tokens = tokenize(source)
  val state = ParseState(tokens)
  parseValue(state) match
    case Left(ParseError.UnexpectedEOF) => Left("予期しない入力終端")
    case Left(ParseError.UnexpectedToken(expected, found)) =>
      Left(s"期待: $expected, 実際: $found")
    case Right((value, rest)) =>
      if rest.tokens.isEmpty then
        Right(value)
      else
        Left("末尾に未消費トークンがあります")

// 利用例
// parseJson("""{"name": "Alice", "age": 30}""")
// => Right(JObject(Map("name" -> JString("Alice"), "age" -> JNumber(30.0))))