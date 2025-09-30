import scala.collection.mutable
import scala.util.{Try, Success, Failure}

/** YAML風パーサー：インデント管理が重要な題材。
  *
  * 対応する構文（簡易版）：
  * - スカラー値: 文字列、数値、真偽値、null
  * - リスト: `- item1`
  * - マップ: `key: value`
  * - ネストしたインデント構造
  *
  * インデント処理の特徴：
  * - Scala 3のパターンマッチングとEitherを活用したパーサー実装
  * - エラー回復機能でインデントミスを報告しつつ継続
  */

// YAML値の表現。
enum YamlValue:
  case Scalar(value: String)
  case YList(items: List[YamlValue])
  case YMap(entries: Map[String, YamlValue])
  case YNull

type Document = YamlValue

case class ParseError(message: String) extends Exception(message)

class Parser(val input: String):
  private var pos: Int = 0

  def peek(): Option[Char] =
    if pos < input.length then Some(input(pos)) else None

  def advance(): Unit =
    if pos < input.length then pos += 1

  def isEof(): Boolean =
    pos >= input.length

  def expect(expected: Char): Unit =
    if peek() != Some(expected) then
      throw ParseError(s"期待された文字 '$expected' が見つかりません")
    advance()

  def expectString(expected: String): Unit =
    for c <- expected do expect(c)

  /** 水平空白のみをスキップ（改行は含まない）。 */
  def hspace(): Unit =
    while peek() match
      case Some(' ') | Some('\t') => true
      case _ => false
    do advance()

  /** 改行をスキップ。 */
  def newline(): Unit =
    peek() match
      case Some('\n') => advance()
      case Some('\r') =>
        advance()
        if peek() == Some('\n') then advance()
      case _ => ()

  /** コメントのスキップ（`#` から行末まで）。 */
  def comment(): Unit =
    if peek() == Some('#') then
      advance()
      while peek() match
        case Some('\n') | None => false
        case _ => true
      do advance()

  /** 空行またはコメント行をスキップ。 */
  def blankOrComment(): Unit =
    hspace()
    comment()
    newline()

  /** 特定のインデントレベルを期待する。 */
  def expectIndent(level: Int): Unit =
    var spaces = 0
    while peek() == Some(' ') do
      spaces += 1
      advance()

    if spaces != level then
      throw ParseError(s"インデント不一致: 期待 $level, 実際 $spaces")

  /** 現在よりも深いインデントを検出。 */
  def deeperIndent(current: Int): Int =
    var spaces = 0
    while peek() == Some(' ') do
      spaces += 1
      advance()

    if spaces <= current then
      throw ParseError(s"深いインデントが期待されます: 現在 $current, 実際 $spaces")

    spaces

  /** スカラー値のパース。 */
  def scalarValue(): YamlValue =
    // null
    if input.drop(pos).startsWith("null") then
      expectString("null")
      return YamlValue.YNull

    if peek() == Some('~') then
      advance()
      return YamlValue.YNull

    // 真偽値
    if input.drop(pos).startsWith("true") then
      expectString("true")
      return YamlValue.Scalar("true")

    if input.drop(pos).startsWith("false") then
      expectString("false")
      return YamlValue.Scalar("false")

    // 数値（簡易実装）
    val numStr = mutable.StringBuilder()
    while peek() match
      case Some(c) if c.isDigit => true
      case _ => false
    do
      numStr += peek().get
      advance()

    if numStr.nonEmpty then
      return YamlValue.Scalar(numStr.toString)

    // 文字列（引用符付き）
    if peek() == Some('"') then
      advance()
      val str = mutable.StringBuilder()
      while peek() match
        case Some('"') => false
        case Some(c) => true
        case None => false
      do
        str += peek().get
        advance()
      expect('"')
      return YamlValue.Scalar(str.toString)

    // 文字列（引用符なし：行末または `:` まで）
    val str = mutable.StringBuilder()
    while peek() match
      case Some('\n') | Some(':') | Some('#') => false
      case Some(c) => true
      case None => false
    do
      str += peek().get
      advance()

    YamlValue.Scalar(str.toString.trim)

  /** リスト項目のパース（`- value` 形式）。 */
  def parseListItem(indent: Int): YamlValue =
    expectIndent(indent)
    expect('-')
    hspace()
    parseValue(indent + 2)

  /** リスト全体のパース。 */
  def parseList(indent: Int): YamlValue =
    val items = mutable.ListBuffer[YamlValue]()

    var continue = true
    while continue do
      val savedPos = pos
      try
        val item = parseListItem(indent)
        items += item
        if peek() == Some('\n') then newline()
        else continue = false
      catch
        case _: ParseError =>
          pos = savedPos
          continue = false

    if items.isEmpty then
      throw ParseError("リストが空です")

    YamlValue.YList(items.toList)

  /** マップのキーバリューペアのパース（`key: value` 形式）。 */
  def parseMapEntry(indent: Int): (String, YamlValue) =
    expectIndent(indent)

    val key = mutable.StringBuilder()
    while peek() match
      case Some(':') | Some('\n') => false
      case Some(c) => true
      case None => false
    do
      key += peek().get
      advance()

    val keyStr = key.toString.trim
    expect(':')
    hspace()

    // 同じ行に値があるか、次の行にネストされているか
    val value =
      if peek() == Some('\n') then
        newline()
        parseValue(indent + 2)
      else
        parseValue(indent)

    (keyStr, value)

  /** マップ全体のパース。 */
  def parseMap(indent: Int): YamlValue =
    val entries = mutable.Map[String, YamlValue]()

    var continue = true
    while continue do
      val savedPos = pos
      try
        val (key, value) = parseMapEntry(indent)
        entries(key) = value
        if peek() == Some('\n') then newline()
        else continue = false
      catch
        case _: ParseError =>
          pos = savedPos
          continue = false

    if entries.isEmpty then
      throw ParseError("マップが空です")

    YamlValue.YMap(entries.toMap)

  /** YAML値のパース（再帰的）。 */
  def parseValue(indent: Int): YamlValue =
    val savedPos = pos

    // リストを試行
    try
      return parseList(indent)
    catch
      case _: ParseError => pos = savedPos

    // マップを試行
    try
      return parseMap(indent)
    catch
      case _: ParseError => pos = savedPos

    // スカラー
    scalarValue()

  /** ドキュメント全体のパース。 */
  def document(): Document =
    // 空行やコメントをスキップ
    while !isEof() do
      val savedPos = pos
      try
        blankOrComment()
        if pos == savedPos then
          throw ParseError("進行なし")
      catch
        case _: ParseError =>
          pos = savedPos
          throw ParseError("break")

    val doc = parseValue(0)

    // 末尾の空行やコメントをスキップ
    while !isEof() do
      val savedPos = pos
      try
        blankOrComment()
        if pos == savedPos then
          throw ParseError("進行なし")
      catch
        case _: ParseError =>
          pos = savedPos
          throw ParseError("break")

    if !isEof() then
      throw ParseError("ドキュメントの終端が期待されます")

    doc

/** パブリックAPI：YAML文字列をパース。 */
def parseYaml(input: String): Either[ParseError, Document] =
  val parser = Parser(input)
  try
    Right(parser.document())
  catch
    case e: ParseError => Left(e)

/** 簡易的なレンダリング（検証用）。 */
def renderToString(doc: Document): String =
  def renderValue(value: YamlValue, indent: Int): String =
    val indentStr = " " * indent

    value match
      case YamlValue.Scalar(s) => s
      case YamlValue.YNull => "null"
      case YamlValue.YList(items) =>
        items.map(item => s"$indentStr- ${renderValue(item, indent + 2)}").mkString("\n")
      case YamlValue.YMap(entries) =>
        entries.map { (key, value) =>
          value match
            case YamlValue.Scalar(_) | YamlValue.YNull =>
              s"$indentStr$key: ${renderValue(value, 0)}"
            case _ =>
              s"$indentStr$key:\n${renderValue(value, indent + 2)}"
        }.mkString("\n")

  renderValue(doc, 0)

/** テスト例。 */
def testExamples(): Unit =
  val examples = List(
    ("simple_scalar", "hello"),
    ("simple_list", "- item1\n- item2\n- item3"),
    ("simple_map", "key1: value1\nkey2: value2"),
    ("nested_map", "parent:\n  child1: value1\n  child2: value2"),
    ("nested_list", "items:\n  - item1\n  - item2"),
    ("mixed", "name: John\nage: 30\nhobbies:\n  - reading\n  - coding")
  )

  for (name, yamlStr) <- examples do
    println(s"--- $name ---")
    parseYaml(yamlStr) match
      case Right(doc) =>
        println("パース成功:")
        println(renderToString(doc))
      case Left(err) =>
        println(s"パースエラー: ${err.message}")

/** インデント処理の課題と解決策：
  *
  * 1. **インデントレベルの追跡**
  *    - パーサー引数としてインデントレベルを渡す
  *    - Scala 3のクラスでパーサー状態を管理
  *
  * 2. **エラー回復**
  *    - try/catchでバックトラックを制御
  *    - ParseError例外で分かりやすいエラーメッセージを提供
  *
  * 3. **空白の扱い**
  *    - hspaceで水平空白のみをスキップ（改行は構文の一部）
  *    - newlineでCR/LF/CRLFを正規化
  *
  * Remlとの比較：
  *
  * - **Scala 3の利点**:
  *   - 強力なパターンマッチングとenum
  *   - 表現力の高い型システム
  *
  * - **Scala 3の課題**:
  *   - パーサーコンビネーターライブラリがRemlほど充実していない
  *   - 手動のバックトラック管理が煩雑
  *
  * - **Remlの利点**:
  *   - 字句レイヤの柔軟性により、インデント処理が自然に表現できる
  *   - cut/commitによるエラー品質の向上
  *   - recoverによる部分的なパース継続が可能
  */