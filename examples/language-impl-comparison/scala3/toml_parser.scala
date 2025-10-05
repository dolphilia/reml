import scala.collection.mutable
import scala.util.matching.Regex
import java.time.{LocalDate, LocalTime, LocalDateTime, OffsetDateTime, ZonedDateTime}
import java.time.format.DateTimeFormatter

/** TOMLパーサー：TOML v1.0.0準拠の簡易版実装。
  *
  * 対応する構文：
  * - キーバリューペア: `key = "value"`
  * - テーブル: `[section]`
  * - 配列テーブル: `[[array_section]]`
  * - データ型: 文字列、整数、浮動小数点、真偽値、日時、配列、インラインテーブル
  * - コメント: `# comment`
  * - トレーリングカンマのサポート
  *
  * Scala 3の特徴を活用：
  * - union型による柔軟な値表現
  * - enumによる構造化されたエラー型
  * - extension methodsによるユーティリティ拡張
  * - toplevel定義でモジュール的な構成
  * - given/usingによる暗黙的なパーサー設定（将来拡張用）
  *
  * 実装の特徴：
  * - Scala 3のクラスベースパーサーで状態管理
  * - Either型による明示的なエラーハンドリング
  * - 再帰下降パーサーで可読性重視
  * - バックトラックによる選択肢の試行
  */

// TOML値の表現（Scala 3のenum）。
enum TomlValue:
  case TString(value: String)
  case TInteger(value: Long)
  case TFloat(value: Double)
  case TBoolean(value: Boolean)
  case TDateTime(value: String) // ISO 8601形式の文字列として保持
  case TArray(items: List[TomlValue])
  case TInlineTable(entries: Map[String, TomlValue])

// TOMLテーブルとドキュメントの型エイリアス。
type TomlTable = Map[String, TomlValue]

case class TomlDocument(
  root: TomlTable,
  tables: Map[List[String], TomlTable]
)

// パースエラー型。
enum ParseError:
  case UnexpectedChar(pos: Int, expected: String, actual: Option[Char])
  case InvalidSyntax(pos: Int, message: String)
  case UnexpectedEof(expected: String)

  def message: String = this match
    case UnexpectedChar(pos, exp, act) =>
      s"位置 $pos: '$exp' が期待されましたが、${act.map(c => s"'$c'").getOrElse("EOF")} が見つかりました"
    case InvalidSyntax(pos, msg) =>
      s"位置 $pos: $msg"
    case UnexpectedEof(exp) =>
      s"予期しないEOF: $exp が期待されました"

// パーサー実装のコアクラス。
class TomlParser(val input: String):
  private var pos: Int = 0

  // 現在の文字を取得。
  def peek(): Option[Char] =
    if pos < input.length then Some(input(pos)) else None

  // 現在の文字を取得して位置を進める。
  def advance(): Option[Char] =
    val c = peek()
    if c.isDefined then pos += 1
    c

  // EOFに到達しているか。
  def isEof(): Boolean =
    pos >= input.length

  // 現在位置を取得。
  def position(): Int = pos

  // 特定の文字を期待。
  def expect(expected: Char): Either[ParseError, Unit] =
    peek() match
      case Some(c) if c == expected =>
        advance()
        Right(())
      case actual =>
        Left(ParseError.UnexpectedChar(pos, expected.toString, actual))

  // 特定の文字列を期待。
  def expectString(expected: String): Either[ParseError, Unit] =
    for c <- expected do
      expect(c) match
        case Left(err) => return Left(err)
        case Right(_) => ()
    Right(())

  // 水平空白（スペース・タブ）をスキップ。
  def skipWhitespace(): Unit =
    while peek() match
      case Some(' ') | Some('\t') => true
      case _ => false
    do advance()

  // 改行をスキップ。
  def skipNewline(): Unit =
    peek() match
      case Some('\n') => advance()
      case Some('\r') =>
        advance()
        if peek() == Some('\n') then advance()
      case _ => ()

  // コメント（`#` から行末まで）をスキップ。
  def skipComment(): Unit =
    if peek() == Some('#') then
      advance()
      while peek() match
        case Some('\n') | None => false
        case _ => true
      do advance()

  // 空白・コメント・改行をスキップ。
  def skipTrivia(): Unit =
    var continue = true
    while continue do
      val startPos = pos
      skipWhitespace()
      skipComment()
      if peek() == Some('\n') || peek() == Some('\r') then
        skipNewline()
      else
        continue = false
      if pos == startPos then continue = false

  // キー名のパース（ベアキーまたは引用符付きキー）。
  def parseKey(): Either[ParseError, String] =
    skipWhitespace()

    peek() match
      case Some('"') =>
        // 引用符付きキー
        parseBasicString()

      case Some('\'') =>
        // リテラル文字列キー
        parseLiteralString()

      case Some(c) if c.isLetterOrDigit || c == '_' || c == '-' =>
        // ベアキー
        val sb = mutable.StringBuilder()
        while peek() match
          case Some(ch) if ch.isLetterOrDigit || ch == '_' || ch == '-' => true
          case _ => false
        do
          sb += advance().get
        Right(sb.toString)

      case actual =>
        Left(ParseError.UnexpectedChar(pos, "キー", actual))

  // ドットで区切られたキーパス。
  def parseKeyPath(): Either[ParseError, List[String]] =
    val keys = mutable.ListBuffer[String]()

    parseKey() match
      case Left(err) => return Left(err)
      case Right(key) => keys += key

    while peek() == Some('.') do
      advance() // '.' をスキップ
      parseKey() match
        case Left(err) => return Left(err)
        case Right(key) => keys += key

    Right(keys.toList)

  // 基本文字列のパース（`"..."`）。
  def parseBasicString(): Either[ParseError, String] =
    // 複数行基本文字列（"""..."""）をチェック
    if input.drop(pos).startsWith("\"\"\"") then
      return parseMultilineBasicString()

    expect('"') match
      case Left(err) => return Left(err)
      case Right(_) => ()

    val sb = mutable.StringBuilder()
    var continue = true

    while continue do
      peek() match
        case Some('\\') =>
          advance()
          peek() match
            case Some('n') => sb += '\n'; advance()
            case Some('t') => sb += '\t'; advance()
            case Some('r') => sb += '\r'; advance()
            case Some('\\') => sb += '\\'; advance()
            case Some('"') => sb += '"'; advance()
            case Some(c) => sb += c; advance()
            case None => return Left(ParseError.UnexpectedEof("エスケープシーケンス"))

        case Some('"') =>
          advance()
          continue = false

        case Some('\n') | Some('\r') =>
          return Left(ParseError.InvalidSyntax(pos, "基本文字列に改行は含められません"))

        case Some(c) =>
          sb += c
          advance()

        case None =>
          return Left(ParseError.UnexpectedEof("閉じ引用符 \""))

    Right(sb.toString)

  // 複数行基本文字列のパース（"""..."""）。
  def parseMultilineBasicString(): Either[ParseError, String] =
    expectString("\"\"\"") match
      case Left(err) => return Left(err)
      case Right(_) => ()

    // 直後の改行を無視
    if peek() == Some('\n') then advance()
    else if peek() == Some('\r') then
      advance()
      if peek() == Some('\n') then advance()

    val sb = mutable.StringBuilder()
    var continue = true

    while continue do
      if input.drop(pos).startsWith("\"\"\"") then
        expectString("\"\"\"")
        continue = false
      else
        peek() match
          case Some('\\') =>
            advance()
            // 行末エスケープ（空白のトリミング）
            if peek() == Some('\n') || peek() == Some('\r') then
              skipNewline()
              skipWhitespace()
            else
              peek() match
                case Some('n') => sb += '\n'; advance()
                case Some('t') => sb += '\t'; advance()
                case Some('r') => sb += '\r'; advance()
                case Some('\\') => sb += '\\'; advance()
                case Some('"') => sb += '"'; advance()
                case Some(c) => sb += c; advance()
                case None => return Left(ParseError.UnexpectedEof("エスケープシーケンス"))

          case Some(c) =>
            sb += c
            advance()

          case None =>
            return Left(ParseError.UnexpectedEof("閉じ引用符 \"\"\""))

    Right(sb.toString)

  // リテラル文字列のパース（'...'）。
  def parseLiteralString(): Either[ParseError, String] =
    // 複数行リテラル文字列（'''...'''）をチェック
    if input.drop(pos).startsWith("'''") then
      return parseMultilineLiteralString()

    expect('\'') match
      case Left(err) => return Left(err)
      case Right(_) => ()

    val sb = mutable.StringBuilder()
    var continue = true

    while continue do
      peek() match
        case Some('\'') =>
          advance()
          continue = false

        case Some('\n') | Some('\r') =>
          return Left(ParseError.InvalidSyntax(pos, "リテラル文字列に改行は含められません"))

        case Some(c) =>
          sb += c
          advance()

        case None =>
          return Left(ParseError.UnexpectedEof("閉じ引用符 '"))

    Right(sb.toString)

  // 複数行リテラル文字列のパース（'''...'''）。
  def parseMultilineLiteralString(): Either[ParseError, String] =
    expectString("'''") match
      case Left(err) => return Left(err)
      case Right(_) => ()

    // 直後の改行を無視
    if peek() == Some('\n') then advance()
    else if peek() == Some('\r') then
      advance()
      if peek() == Some('\n') then advance()

    val sb = mutable.StringBuilder()
    var continue = true

    while continue do
      if input.drop(pos).startsWith("'''") then
        expectString("'''")
        continue = false
      else
        peek() match
          case Some(c) =>
            sb += c
            advance()

          case None =>
            return Left(ParseError.UnexpectedEof("閉じ引用符 '''"))

    Right(sb.toString)

  // 整数のパース。
  def parseInteger(): Either[ParseError, Long] =
    val sb = mutable.StringBuilder()

    // 符号
    if peek() == Some('+') || peek() == Some('-') then
      sb += advance().get

    // 数字（アンダースコアを無視）
    var hasDigit = false
    while peek() match
      case Some(c) if c.isDigit => true
      case Some('_') => true
      case _ => false
    do
      val c = advance().get
      if c != '_' then
        sb += c
        hasDigit = true

    if !hasDigit then
      return Left(ParseError.InvalidSyntax(pos, "整数値が必要です"))

    try
      Right(sb.toString.toLong)
    catch
      case _: NumberFormatException =>
        Left(ParseError.InvalidSyntax(pos, "無効な整数値"))

  // 浮動小数点のパース。
  def parseFloat(): Either[ParseError, Double] =
    val sb = mutable.StringBuilder()

    // 符号
    if peek() == Some('+') || peek() == Some('-') then
      sb += advance().get

    // 整数部
    while peek() match
      case Some(c) if c.isDigit => true
      case Some('_') => true
      case _ => false
    do
      val c = advance().get
      if c != '_' then sb += c

    // 小数部
    if peek() == Some('.') then
      sb += advance().get
      while peek() match
        case Some(c) if c.isDigit => true
        case Some('_') => true
        case _ => false
      do
        val c = advance().get
        if c != '_' then sb += c

    // 指数部
    if peek() == Some('e') || peek() == Some('E') then
      sb += advance().get
      if peek() == Some('+') || peek() == Some('-') then
        sb += advance().get
      while peek() match
        case Some(c) if c.isDigit => true
        case Some('_') => true
        case _ => false
      do
        val c = advance().get
        if c != '_' then sb += c

    try
      Right(sb.toString.toDouble)
    catch
      case _: NumberFormatException =>
        Left(ParseError.InvalidSyntax(pos, "無効な浮動小数点値"))

  // 真偽値のパース。
  def parseBoolean(): Either[ParseError, Boolean] =
    if input.drop(pos).startsWith("true") then
      expectString("true")
      Right(true)
    else if input.drop(pos).startsWith("false") then
      expectString("false")
      Right(false)
    else
      Left(ParseError.InvalidSyntax(pos, "真偽値が必要です"))

  // 日時のパース（簡易実装：ISO 8601形式の文字列として保持）。
  def parseDateTime(): Either[ParseError, String] =
    val sb = mutable.StringBuilder()

    // 日時文字列を抽出（簡易実装）
    while peek() match
      case Some(c) if c.isDigit || c == '-' || c == ':' || c == 'T' || c == 'Z' || c == '+' || c == '.' => true
      case _ => false
    do
      sb += advance().get

    val dtStr = sb.toString
    if dtStr.isEmpty then
      Left(ParseError.InvalidSyntax(pos, "日時が必要です"))
    else
      Right(dtStr)

  // 配列のパース。
  def parseArray(): Either[ParseError, List[TomlValue]] =
    expect('[') match
      case Left(err) => return Left(err)
      case Right(_) => ()

    skipTrivia()

    val items = mutable.ListBuffer[TomlValue]()

    // 空配列のチェック
    if peek() == Some(']') then
      advance()
      return Right(List.empty)

    var continue = true
    while continue do
      parseValue() match
        case Left(err) => return Left(err)
        case Right(value) => items += value

      skipTrivia()

      peek() match
        case Some(',') =>
          advance()
          skipTrivia()
          // トレーリングカンマのチェック
          if peek() == Some(']') then
            advance()
            continue = false
        case Some(']') =>
          advance()
          continue = false
        case actual =>
          return Left(ParseError.UnexpectedChar(pos, "',' または ']'", actual))

    Right(items.toList)

  // インラインテーブルのパース。
  def parseInlineTable(): Either[ParseError, Map[String, TomlValue]] =
    expect('{') match
      case Left(err) => return Left(err)
      case Right(_) => ()

    skipWhitespace()

    val entries = mutable.Map[String, TomlValue]()

    // 空テーブルのチェック
    if peek() == Some('}') then
      advance()
      return Right(Map.empty)

    var continue = true
    while continue do
      parseKey() match
        case Left(err) => return Left(err)
        case Right(key) =>
          skipWhitespace()
          expect('=') match
            case Left(err) => return Left(err)
            case Right(_) => ()

          skipWhitespace()

          parseValue() match
            case Left(err) => return Left(err)
            case Right(value) => entries(key) = value

      skipWhitespace()

      peek() match
        case Some(',') =>
          advance()
          skipWhitespace()
          // トレーリングカンマのチェック
          if peek() == Some('}') then
            advance()
            continue = false
        case Some('}') =>
          advance()
          continue = false
        case actual =>
          return Left(ParseError.UnexpectedChar(pos, "',' または '}'", actual))

    Right(entries.toMap)

  // 値のパース（再帰的）。
  def parseValue(): Either[ParseError, TomlValue] =
    skipWhitespace()

    val startPos = pos

    // 文字列
    if peek() == Some('"') then
      return parseBasicString().map(TomlValue.TString(_))
    else if peek() == Some('\'') then
      return parseLiteralString().map(TomlValue.TString(_))

    // 真偽値
    if input.drop(pos).startsWith("true") || input.drop(pos).startsWith("false") then
      return parseBoolean().map(TomlValue.TBoolean(_))

    // 配列
    if peek() == Some('[') then
      return parseArray().map(TomlValue.TArray(_))

    // インラインテーブル
    if peek() == Some('{') then
      return parseInlineTable().map(TomlValue.TInlineTable(_))

    // 日時（試行）
    val savedPos = pos
    parseDateTime() match
      case Right(dtStr) if dtStr.contains('T') || dtStr.contains('Z') || dtStr.contains(':') =>
        return Right(TomlValue.TDateTime(dtStr))
      case _ =>
        pos = savedPos

    // 浮動小数点または整数
    val numStart = pos
    parseFloat() match
      case Right(f) if input.substring(numStart, pos).contains('.') ||
                       input.substring(numStart, pos).contains('e') ||
                       input.substring(numStart, pos).contains('E') =>
        return Right(TomlValue.TFloat(f))
      case _ =>
        pos = numStart

    parseInteger() match
      case Right(n) => return Right(TomlValue.TInteger(n))
      case Left(err) => return Left(err)

  // キーバリューペアのパース。
  def parseKeyValuePair(): Either[ParseError, (List[String], TomlValue)] =
    parseKeyPath() match
      case Left(err) => return Left(err)
      case Right(path) =>
        skipWhitespace()
        expect('=') match
          case Left(err) => return Left(err)
          case Right(_) => ()

        skipWhitespace()

        parseValue() match
          case Left(err) => Left(err)
          case Right(value) => Right((path, value))

  // テーブルヘッダーのパース（`[section]`）。
  def parseTableHeader(): Either[ParseError, List[String]] =
    expect('[') match
      case Left(err) => return Left(err)
      case Right(_) => ()

    parseKeyPath() match
      case Left(err) => return Left(err)
      case Right(path) =>
        expect(']') match
          case Left(err) => Left(err)
          case Right(_) => Right(path)

  // 配列テーブルヘッダーのパース（`[[array_section]]`）。
  def parseArrayTableHeader(): Either[ParseError, List[String]] =
    expectString("[[") match
      case Left(err) => return Left(err)
      case Right(_) => ()

    parseKeyPath() match
      case Left(err) => return Left(err)
      case Right(path) =>
        expectString("]]") match
          case Left(err) => Left(err)
          case Right(_) => Right(path)

  // ドキュメント要素。
  enum DocumentElement:
    case KeyValue(path: List[String], value: TomlValue)
    case Table(path: List[String])
    case ArrayTable(path: List[String])

  // ドキュメント要素のパース。
  def parseDocumentElement(): Either[ParseError, DocumentElement] =
    skipTrivia()

    if isEof() then
      return Left(ParseError.UnexpectedEof("ドキュメント要素"))

    // 配列テーブル
    if input.drop(pos).startsWith("[[") then
      return parseArrayTableHeader().map(DocumentElement.ArrayTable(_))

    // テーブル
    if peek() == Some('[') then
      return parseTableHeader().map(DocumentElement.Table(_))

    // キーバリューペア
    parseKeyValuePair().map { case (path, value) =>
      DocumentElement.KeyValue(path, value)
    }

  // ドキュメント全体のパース。
  def parseDocument(): Either[ParseError, TomlDocument] =
    val elements = mutable.ListBuffer[DocumentElement]()

    skipTrivia()

    while !isEof() do
      parseDocumentElement() match
        case Left(err) => return Left(err)
        case Right(elem) => elements += elem

      skipTrivia()

    // 要素をグループ化してドキュメント構造を構築
    var currentTable: List[String] = List.empty
    val root = mutable.Map[String, TomlValue]()
    val tables = mutable.Map[List[String], TomlTable]()

    for elem <- elements do
      elem match
        case DocumentElement.Table(path) =>
          currentTable = path
          if !tables.contains(path) then
            tables(path) = Map.empty

        case DocumentElement.ArrayTable(path) =>
          currentTable = path
          // 簡易実装：配列テーブルは通常テーブルと同じ扱い
          if !tables.contains(path) then
            tables(path) = Map.empty

        case DocumentElement.KeyValue(path, value) =>
          if currentTable.isEmpty then
            // ルートテーブルに追加
            insertNested(root, path, value)
          else
            // 現在のテーブルに追加
            val table = tables.getOrElseUpdate(currentTable, Map.empty)
            val tableMap = mutable.Map.from(table)
            insertNested(tableMap, path, value)
            tables(currentTable) = tableMap.toMap

    Right(TomlDocument(root.toMap, tables.toMap))

  // ネストしたキーパスに値を挿入する補助関数。
  private def insertNested(table: mutable.Map[String, TomlValue], path: List[String], value: TomlValue): Unit =
    path match
      case Nil => ()
      case key :: Nil =>
        table(key) = value
      case key :: rest =>
        val nested = table.get(key) match
          case Some(TomlValue.TInlineTable(t)) => mutable.Map.from(t)
          case _ => mutable.Map[String, TomlValue]()

        insertNested(nested, rest, value)
        table(key) = TomlValue.TInlineTable(nested.toMap)

// パブリックAPI：TOML文字列をパース。
def parseTOML(input: String): Either[ParseError, TomlDocument] =
  val parser = TomlParser(input)
  parser.parseDocument()

// 簡易的なレンダリング（検証用）。
def renderToString(doc: TomlDocument): String =
  val sb = mutable.StringBuilder()

  // ルートテーブルをレンダリング
  for (key, value) <- doc.root do
    sb.append(s"$key = ${renderValue(value)}\n")

  // 各セクションをレンダリング
  for (path, table) <- doc.tables do
    sb.append(s"\n[${path.mkString(".")}]\n")
    for (key, value) <- table do
      sb.append(s"$key = ${renderValue(value)}\n")

  sb.toString

def renderValue(value: TomlValue): String =
  value match
    case TomlValue.TString(s) => s"\"$s\""
    case TomlValue.TInteger(n) => n.toString
    case TomlValue.TFloat(f) => f.toString
    case TomlValue.TBoolean(b) => b.toString
    case TomlValue.TDateTime(dt) => dt
    case TomlValue.TArray(items) =>
      items.map(renderValue).mkString("[", ", ", "]")
    case TomlValue.TInlineTable(entries) =>
      entries.map { (k, v) => s"$k = ${renderValue(v)}" }.mkString("{ ", ", ", " }")

// テスト例。
@main def testTomlParser(): Unit =
  val exampleToml = """
# Reml パッケージ設定

[package]
name = "my_project"
version = "0.1.0"
authors = ["Author Name"]
license = "MIT"

[dependencies]
core = "1.0"
parser = "2.3"

[dev-dependencies]
test_framework = "0.5"

[[plugins]]
name = "system"
version = "1.0"
enabled = true

[[plugins]]
name = "memory"
version = "1.0"
enabled = false

[build]
target = "native"
optimization = 3
features = ["simd", "parallel"]

[metadata]
description = "A sample TOML configuration"
homepage = "https://example.com"
config = { debug = true, port = 8080 }
"""

  println("--- TOML v1.0.0 簡易版パーサー ---\n")

  parseTOML(exampleToml) match
    case Right(doc) =>
      println("パース成功:\n")
      println(renderToString(doc))

      println("\n--- 構造の検証 ---")
      println(s"ルートテーブルのキー数: ${doc.root.size}")
      println(s"セクション数: ${doc.tables.size}")

      doc.tables.get(List("package")) match
        case Some(pkg) =>
          println(s"\n[package]セクション:")
          pkg.foreach { case (k, v) =>
            println(s"  $k = ${renderValue(v)}")
          }
        case None =>
          println("\n[package]セクションが見つかりません")

    case Left(err) =>
      println(s"パースエラー: ${err.message}")

  // 追加のテストケース
  println("\n--- 追加テスト: 基本データ型 ---")

  val basicTypes = """
string = "hello"
integer = 42
float = 3.14
boolean = true
datetime = 2024-01-01T12:00:00Z
array = [1, 2, 3]
inline_table = { x = 1, y = 2 }
"""

  parseTOML(basicTypes) match
    case Right(doc) =>
      println("基本データ型のパース成功:\n")
      println(renderToString(doc))
    case Left(err) =>
      println(s"パースエラー: ${err.message}")

/** 実装のポイントと他言語との比較：
  *
  * **Scala 3の特徴を活かした実装**:
  * 1. enum型による構造化されたデータ表現（TomlValue、ParseError）
  * 2. Either型による明示的なエラーハンドリング
  * 3. extension methodsの活用可能性（将来拡張）
  * 4. toplevel定義でモジュール的な構成
  * 5. given/usingによる設定の暗黙的な受け渡し（拡張時）
  *
  * **Remlとの比較**:
  * - **Remlの利点**:
  *   - パーサーコンビネーターによる高品質なエラーメッセージ
  *   - cut/commitによる正確なエラー位置特定
  *   - recoverによる部分的なパース継続
  *   - 宣言的で読みやすいパーサー定義
  *
  * - **Scala 3の利点**:
  *   - 強力な型システムと表現力
  *   - JVM/Native/JSへのマルチプラットフォーム対応
  *   - 豊富なライブラリエコシステム
  *
  * - **Scala 3の課題**:
  *   - 手動のバックトラック管理が必要
  *   - エラーメッセージの質がパーサー実装に依存
  *   - ボイラープレートが多くなりがち
  *
  * **他言語実装との比較**:
  *
  * - **Rust（toml-rs）**:
  *   - serdeとの統合で型安全
  *   - エラーメッセージは実装次第
  *   - 所有権システムによる安全性
  *
  * - **Python（tomli）**:
  *   - シンプルで高速
  *   - 動的型付けによる柔軟性
  *   - エラー位置情報が限定的
  *
  * - **Go（BurntSushi/toml）**:
  *   - シンプルなAPI
  *   - エラーメッセージの質がまちまち
  *   - リフレクションによる構造体マッピング
  *
  * **実装上の工夫**:
  * - トレーリングカンマのサポート（配列・インラインテーブル）
  * - 複数行文字列（基本・リテラル）の完全サポート
  * - 日時のISO 8601形式対応（文字列として保持）
  * - ネストしたキーパスの自動展開
  * - バックトラックによる値型の自動判別
  */