// Markdown風軽量マークアップパーサー - Scala 3実装
//
// Unicode処理の注意点：
// - ScalaのStringはJavaのString（UTF-16ベース）
// - length はUTF-16コードユニット数を返す（サロゲートペアに注意）
// - codePointCount でUnicodeコードポイント数を取得可能
// - Grapheme（書記素クラスター）処理には外部ライブラリ（icu4j等）が必要
// - Remlの3層モデル（Byte/Char/Grapheme）と比較すると、Scala 3も明示的な区別が必要

package markdownParser

// Markdown AST のインライン要素
enum Inline:
  case Text(text: String)
  case Strong(inner: List[Inline])
  case Emphasis(inner: List[Inline])
  case Code(code: String)
  case Link(text: List[Inline], url: String)
  case LineBreak

// Markdown AST のブロック要素
enum Block:
  case Heading(level: Int, inline: List[Inline])
  case Paragraph(inline: List[Inline])
  case UnorderedList(items: List[List[Inline]])
  case OrderedList(items: List[List[Inline]])
  case CodeBlock(lang: Option[String], code: String)
  case HorizontalRule

type Document = List[Block]

// パーサー状態
case class ParseState(input: String, position: Int)

// パーサー例外
case class ParseError(message: String) extends Exception(message)

// 現在位置の1文字を取得
def peekChar(state: ParseState): Option[Char] =
  if state.position >= state.input.length then None
  else Some(state.input(state.position))

// 1文字を消費して進める
def advanceChar(state: ParseState): ParseState =
  state.copy(position = state.position + 1)

// 固定文字列をマッチ
def matchString(state: ParseState, target: String): Option[ParseState] =
  val remaining = state.input.substring(state.position)
  if remaining.startsWith(target) then
    Some(state.copy(position = state.position + target.length))
  else
    None

// 水平空白をスキップ
def skipHSpace(state: ParseState): ParseState =
  peekChar(state) match
    case Some(' ') | Some('\t') => skipHSpace(advanceChar(state))
    case _ => state

// 空行をスキップ
def skipBlankLines(state: ParseState): ParseState =
  peekChar(state) match
    case Some('\n') => skipBlankLines(advanceChar(state))
    case _ => state

// 行末まで読む
def readUntilEol(state: ParseState): (String, ParseState) =
  def loop(pos: Int, acc: StringBuilder): (String, ParseState) =
    if pos >= state.input.length then
      (acc.toString, state.copy(position = pos))
    else if state.input(pos) == '\n' then
      (acc.toString, state.copy(position = pos))
    else
      loop(pos + 1, acc.append(state.input(pos)))

  loop(state.position, new StringBuilder)

// 改行を消費
def consumeNewline(state: ParseState): ParseState =
  peekChar(state) match
    case Some('\n') => advanceChar(state)
    case _ => state

// EOFチェック
def isEof(state: ParseState): Boolean =
  state.position >= state.input.length

// 見出し行のパース（`# Heading` 形式）
def parseHeading(state: ParseState): (Block, ParseState) =
  val state1 = skipHSpace(state)

  // `#` の連続をカウント
  def countHashes(s: ParseState, n: Int): (Int, ParseState) =
    peekChar(s) match
      case Some('#') => countHashes(advanceChar(s), n + 1)
      case _ => (n, s)

  val (level, state2) = countHashes(state1, 0)

  if level == 0 || level > 6 then
    throw ParseError("見出しレベルは1-6の範囲内である必要があります")
  else
    val state3 = skipHSpace(state2)
    val (text, state4) = readUntilEol(state3)
    val state5 = consumeNewline(state4)
    val inline = List(Inline.Text(text.trim))
    (Block.Heading(level, inline), state5)

// 水平線のパース（`---`, `***`, `___`）
def parseHorizontalRule(state: ParseState): (Block, ParseState) =
  val state1 = skipHSpace(state)
  val (text, state2) = readUntilEol(state1)
  val state3 = consumeNewline(state2)

  val trimmed = text.trim
  val isRule =
    (trimmed.forall(_ == '-') && trimmed.length >= 3) ||
    (trimmed.forall(_ == '*') && trimmed.length >= 3) ||
    (trimmed.forall(_ == '_') && trimmed.length >= 3)

  if !isRule then
    throw ParseError("水平線として認識できません")
  else
    (Block.HorizontalRule, state3)

// コードブロックのパース（```言語名）
def parseCodeBlock(state: ParseState): (Block, ParseState) =
  matchString(state, "```") match
    case None => throw ParseError("コードブロック開始が見つかりません")
    case Some(state1) =>
      val (langLine, state2) = readUntilEol(state1)
      val state3 = consumeNewline(state2)

      val lang =
        val trimmed = langLine.trim
        if trimmed.isEmpty then None else Some(trimmed)

      // コードブロック内容を ```閉じまで読む
      def readCodeLines(s: ParseState, lines: List[String]): (List[String], ParseState) =
        matchString(s, "```") match
          case Some(endState) => (lines.reverse, endState)
          case None =>
            if isEof(s) then
              (lines.reverse, s)
            else
              val (line, s2) = readUntilEol(s)
              val s3 = consumeNewline(s2)
              readCodeLines(s3, line :: lines)

      val (codeLines, state4) = readCodeLines(state3, Nil)
      val state5 = consumeNewline(state4)

      val code = codeLines.mkString("\n")
      (Block.CodeBlock(lang, code), state5)

// リスト項目のパース（簡易版：`-` または `*`）
def parseUnorderedList(state: ParseState): (Block, ParseState) =
  def parseItems(s: ParseState, items: List[List[Inline]]): (List[List[Inline]], ParseState) =
    val s1 = skipHSpace(s)
    peekChar(s1) match
      case Some('-') | Some('*') =>
        val s2 = advanceChar(s1)
        val s3 = skipHSpace(s2)
        val (text, s4) = readUntilEol(s3)
        val s5 = consumeNewline(s4)
        val inline = List(Inline.Text(text.trim))
        parseItems(s5, items :+ inline)
      case _ => (items, s)

  val (items, stateEnd) = parseItems(state, Nil)

  if items.isEmpty then
    throw ParseError("リスト項目が見つかりません")
  else
    (Block.UnorderedList(items), stateEnd)

// 段落のパース（簡易版：空行まで）
def parseParagraph(state: ParseState): (Block, ParseState) =
  def readLines(s: ParseState, lines: List[String]): (List[String], ParseState) =
    if isEof(s) then
      (lines.reverse, s)
    else
      peekChar(s) match
        case Some('\n') =>
          val s1 = advanceChar(s)
          peekChar(s1) match
            case Some('\n') => (lines.reverse, s1) // 空行で段落終了
            case _ => readLines(s1, "" :: lines)
        case Some(_) =>
          val (line, s2) = readUntilEol(s)
          val s3 = consumeNewline(s2)
          readLines(s3, line :: lines)
        case None => (lines.reverse, s)

  val (lines, stateEnd) = readLines(state, Nil)
  val text = lines.mkString(" ").trim
  val inline = List(Inline.Text(text))
  (Block.Paragraph(inline), stateEnd)

// ブロック要素のパース（優先順位付き試行）
def parseBlock(state: ParseState): (Block, ParseState) =
  val state1 = skipBlankLines(state)

  if isEof(state1) then
    throw ParseError("EOF")
  else
    val state2 = skipHSpace(state1)

    peekChar(state2) match
      case Some('#') => parseHeading(state2)
      case Some('`') =>
        matchString(state2, "```") match
          case Some(_) => parseCodeBlock(state2)
          case None => parseParagraph(state2)
      case Some('-') | Some('*') | Some('_') =>
        try
          parseHorizontalRule(state2)
        catch
          case _: ParseError => parseUnorderedList(state2)
      case Some(_) => parseParagraph(state2)
      case None => throw ParseError("EOF")

// ドキュメント全体のパース
def parseDocument(state: ParseState, blocks: List[Block]): Document =
  try
    val (block, newState) = parseBlock(state)
    parseDocument(newState, blocks :+ block)
  catch
    case ParseError("EOF") => blocks

// パブリックAPI：文字列からドキュメントをパース
def parse(input: String): Document =
  val initialState = ParseState(input, 0)
  parseDocument(initialState, Nil)

// 簡易的なレンダリング（検証用）
def renderInline(inlines: List[Inline]): String =
  inlines.map {
    case Inline.Text(s) => s
    case Inline.Strong(inner) => s"**${renderInline(inner)}**"
    case Inline.Emphasis(inner) => s"*${renderInline(inner)}*"
    case Inline.Code(s) => s"`$s`"
    case Inline.Link(text, url) => s"[${renderInline(text)}]($url)"
    case Inline.LineBreak => "\n"
  }.mkString

def renderBlock(block: Block): String =
  block match
    case Block.Heading(level, inline) =>
      val prefix = "#" * level
      s"$prefix ${renderInline(inline)}\n\n"
    case Block.Paragraph(inline) =>
      s"${renderInline(inline)}\n\n"
    case Block.UnorderedList(items) =>
      val itemsStr = items.map(item => s"- ${renderInline(item)}\n").mkString
      s"$itemsStr\n"
    case Block.OrderedList(items) =>
      val itemsStr = items.zipWithIndex.map { case (item, i) =>
        s"${i + 1}. ${renderInline(item)}\n"
      }.mkString
      s"$itemsStr\n"
    case Block.CodeBlock(lang, code) =>
      val langStr = lang.getOrElse("")
      s"```$langStr\n$code\n```\n\n"
    case Block.HorizontalRule =>
      "---\n\n"

def renderToString(doc: Document): String =
  doc.map(renderBlock).mkString

// Unicode 3層モデル比較：
//
// ScalaのStringはJavaのString（UTF-16ベース）で：
// - length はUTF-16コードユニット数を返す
// - サロゲートペア（例：絵文字「😀」）は2カウントされる
// - codePointCount でUnicodeコードポイント数を取得可能
// - Grapheme処理には icu4j 等の外部ライブラリが必要
//
// 例：
// val str = "🇯🇵"  // 国旗絵文字（2つのコードポイント、1つのgrapheme）
// str.length  // => 4 (UTF-16コードユニット数)
// str.codePointCount(0, str.length)  // => 2 (コードポイント数)
//
// Remlの3層モデル（Byte/Char/Grapheme）と比較すると、
// Scala 3も明示的な区別が必要で、標準APIだけでは絵文字や結合文字の扱いが複雑。
//
// Reml との比較メモ:
// 1. Scala 3: enum で代数的データ型を簡潔に表現
//    Reml: 型定義で直接 `type Inline = Text(string) | Strong(...) | ...` と記述
//    - 両言語とも現代的な構文で、パターンマッチが強力
// 2. Scala 3: immutableデータ構造とcopyメソッドで状態を管理
//    Reml: 関数型 + 手続き型のハイブリッドアプローチ
// 3. Scala 3: Either型やTry型でエラーハンドリング、例外も利用可能
//    Reml: Result型を標準で提供し、? 演算子で簡潔に記述
// 4. 両言語とも型推論が強力で、型安全性を確保