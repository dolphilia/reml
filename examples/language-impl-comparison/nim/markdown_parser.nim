# Markdown風軽量マークアップパーサー - Nim実装
#
# Unicode処理の注意点：
# - Nimのstringはバイトシーケンス（UTF-8を想定）
# - len() はバイト数を返す
# - runeLen() や runesLen() でUnicodeコードポイント数を取得
# - Grapheme（書記素クラスター）単位の操作は標準ライブラリにない
# - Remlの3層モデル（Byte/Char/Grapheme）と比較すると、Nimも明示的な区別が必要

import std/[strutils, sequtils, options]

# Markdown AST のインライン要素
type
  Inline = ref object
    case kind: InlineKind
    of ikText: text: string
    of ikStrong: strongInner: seq[Inline]
    of ikEmphasis: emphasisInner: seq[Inline]
    of ikCode: code: string
    of ikLink:
      linkText: seq[Inline]
      url: string
    of ikLineBreak: discard

  InlineKind = enum
    ikText, ikStrong, ikEmphasis, ikCode, ikLink, ikLineBreak

# Markdown AST のブロック要素
type
  Block = ref object
    case kind: BlockKind
    of bkHeading:
      level: int
      headingInline: seq[Inline]
    of bkParagraph:
      paragraphInline: seq[Inline]
    of bkUnorderedList:
      ulItems: seq[seq[Inline]]
    of bkOrderedList:
      olItems: seq[seq[Inline]]
    of bkCodeBlock:
      lang: Option[string]
      codeContent: string
    of bkHorizontalRule: discard

  BlockKind = enum
    bkHeading, bkParagraph, bkUnorderedList, bkOrderedList, bkCodeBlock, bkHorizontalRule

  Document = seq[Block]

# パーサー状態
type
  ParseState = object
    input: string
    position: int

  ParseError = object of CatchableError

# コンストラクタヘルパー
proc newText(s: string): Inline =
  Inline(kind: ikText, text: s)

proc newHeading(lvl: int, inline: seq[Inline]): Block =
  Block(kind: bkHeading, level: lvl, headingInline: inline)

proc newParagraph(inline: seq[Inline]): Block =
  Block(kind: bkParagraph, paragraphInline: inline)

proc newUnorderedList(items: seq[seq[Inline]]): Block =
  Block(kind: bkUnorderedList, ulItems: items)

proc newCodeBlock(lang: Option[string], code: string): Block =
  Block(kind: bkCodeBlock, lang: lang, codeContent: code)

proc newHorizontalRule(): Block =
  Block(kind: bkHorizontalRule)

# === パーサーユーティリティ ===

proc peekChar(state: ParseState): Option[char] =
  if state.position >= state.input.len:
    return none(char)
  some(state.input[state.position])

proc advanceChar(state: var ParseState) =
  state.position += 1

proc matchString(state: var ParseState, target: string): bool =
  let remaining = state.input[state.position..^1]
  if remaining.startsWith(target):
    state.position += target.len
    return true
  false

proc skipHSpace(state: var ParseState) =
  while true:
    let ch = peekChar(state)
    if ch.isSome and (ch.get == ' ' or ch.get == '\t'):
      advanceChar(state)
    else:
      break

proc skipBlankLines(state: var ParseState) =
  while true:
    let ch = peekChar(state)
    if ch.isSome and ch.get == '\n':
      advanceChar(state)
    else:
      break

proc readUntilEol(state: var ParseState): string =
  var line = ""
  while true:
    let ch = peekChar(state)
    if ch.isNone or ch.get == '\n':
      break
    line.add(ch.get)
    advanceChar(state)
  line

proc consumeNewline(state: var ParseState) =
  let ch = peekChar(state)
  if ch.isSome and ch.get == '\n':
    advanceChar(state)

proc isEof(state: ParseState): bool =
  state.position >= state.input.len

# === パース関数 ===

proc parseHeading(state: var ParseState): Block =
  skipHSpace(state)

  # `#` の連続をカウント
  var level = 0
  while true:
    let ch = peekChar(state)
    if ch.isSome and ch.get == '#':
      level += 1
      advanceChar(state)
    else:
      break

  if level == 0 or level > 6:
    raise newException(ParseError, "見出しレベルは1-6の範囲内である必要があります")

  skipHSpace(state)
  let text = readUntilEol(state)
  consumeNewline(state)

  let inline = @[newText(text.strip)]
  newHeading(level, inline)

proc parseHorizontalRule(state: var ParseState): Block =
  skipHSpace(state)
  let text = readUntilEol(state)
  consumeNewline(state)

  let trimmed = text.strip
  let isRule =
    (trimmed.allIt(it == '-') and trimmed.len >= 3) or
    (trimmed.allIt(it == '*') and trimmed.len >= 3) or
    (trimmed.allIt(it == '_') and trimmed.len >= 3)

  if not isRule:
    raise newException(ParseError, "水平線として認識できません")

  newHorizontalRule()

proc parseCodeBlock(state: var ParseState): Block =
  if not matchString(state, "```"):
    raise newException(ParseError, "コードブロック開始が見つかりません")

  let langLine = readUntilEol(state)
  consumeNewline(state)

  let lang =
    let trimmed = langLine.strip
    if trimmed.len == 0:
      none(string)
    else:
      some(trimmed)

  # コードブロック内容を ```閉じまで読む
  var codeLines: seq[string] = @[]
  while true:
    if matchString(state, "```"):
      break
    if isEof(state):
      break
    let line = readUntilEol(state)
    consumeNewline(state)
    codeLines.add(line)

  consumeNewline(state)

  let code = codeLines.join("\n")
  newCodeBlock(lang, code)

proc parseUnorderedList(state: var ParseState): Block =
  var items: seq[seq[Inline]] = @[]

  while true:
    skipHSpace(state)
    let ch = peekChar(state)
    if ch.isNone or (ch.get != '-' and ch.get != '*'):
      break

    advanceChar(state)
    skipHSpace(state)
    let text = readUntilEol(state)
    consumeNewline(state)

    let inline = @[newText(text.strip)]
    items.add(inline)

  if items.len == 0:
    raise newException(ParseError, "リスト項目が見つかりません")

  newUnorderedList(items)

proc parseParagraph(state: var ParseState): Block =
  var lines: seq[string] = @[]

  while not isEof(state):
    let ch = peekChar(state)
    if ch.isSome and ch.get == '\n':
      advanceChar(state)
      let ch2 = peekChar(state)
      if ch2.isSome and ch2.get == '\n':
        break  # 空行で段落終了
      lines.add("")
    else:
      let line = readUntilEol(state)
      consumeNewline(state)
      lines.add(line)

  let text = lines.join(" ").strip
  let inline = @[newText(text)]
  newParagraph(inline)

proc parseBlock(state: var ParseState): Block =
  skipBlankLines(state)

  if isEof(state):
    raise newException(ParseError, "EOF")

  skipHSpace(state)

  let ch = peekChar(state)
  if ch.isNone:
    raise newException(ParseError, "EOF")

  case ch.get
  of '#':
    parseHeading(state)
  of '`':
    if matchString(state, "```"):
      parseCodeBlock(state)
    else:
      parseParagraph(state)
  of '-', '*', '_':
    try:
      parseHorizontalRule(state)
    except ParseError:
      # 水平線でなければリストとして解析
      parseUnorderedList(state)
  else:
    parseParagraph(state)

proc parseDocument(state: var ParseState): Document =
  var blocks: seq[Block] = @[]

  while true:
    try:
      let block = parseBlock(state)
      blocks.add(block)
    except ParseError as e:
      if e.msg == "EOF":
        break
      else:
        raise

  blocks

# パブリックAPI：文字列からドキュメントをパース
proc parse*(input: string): Document =
  var state = ParseState(input: input, position: 0)
  parseDocument(state)

# === レンダリング ===

proc renderInline(inlines: seq[Inline]): string =
  result = ""
  for i in inlines:
    case i.kind
    of ikText:
      result.add(i.text)
    of ikStrong:
      result.add("**" & renderInline(i.strongInner) & "**")
    of ikEmphasis:
      result.add("*" & renderInline(i.emphasisInner) & "*")
    of ikCode:
      result.add("`" & i.code & "`")
    of ikLink:
      result.add("[" & renderInline(i.linkText) & "](" & i.url & ")")
    of ikLineBreak:
      result.add("\n")

proc renderBlock(block: Block): string =
  case block.kind
  of bkHeading:
    let prefix = "#".repeat(block.level)
    prefix & " " & renderInline(block.headingInline) & "\n\n"
  of bkParagraph:
    renderInline(block.paragraphInline) & "\n\n"
  of bkUnorderedList:
    var itemsStr = ""
    for item in block.ulItems:
      itemsStr.add("- " & renderInline(item) & "\n")
    itemsStr & "\n"
  of bkOrderedList:
    var itemsStr = ""
    for i, item in block.olItems:
      itemsStr.add($(i + 1) & ". " & renderInline(item) & "\n")
    itemsStr & "\n"
  of bkCodeBlock:
    let langStr = if block.lang.isSome: block.lang.get else: ""
    "```" & langStr & "\n" & block.codeContent & "\n```\n\n"
  of bkHorizontalRule:
    "---\n\n"

proc renderToString*(doc: Document): string =
  result = ""
  for block in doc:
    result.add(renderBlock(block))

# === テスト ===

when isMainModule:
  echo "=== Nim Markdown パーサー ==="

  let markdown = """
# 見出し1

これは段落です。

- リスト1
- リスト2

```nim
echo "Hello"
```

---
"""

  try:
    let doc = parse(markdown)
    echo "パース成功: ", doc.len, " ブロック"
    echo renderToString(doc)
  except ParseError as e:
    echo "パースエラー: ", e.msg

# === Unicode 3層モデル比較 ===
#
# Nimのstringはバイトシーケンス（UTF-8を想定）で：
# - len() はバイト数を返す
# - runesLen() でUnicodeコードポイント数を取得
# - Grapheme（書記素クラスター）単位の操作は標準ライブラリにない
#
# 例：
# let str = "🇯🇵"  # 国旗絵文字（2つのコードポイント、1つのgrapheme）
# str.len  # => 8 (バイト数)
# str.runesLen  # => 2 (コードポイント数)
#
# Remlの3層モデル（Byte/Char/Grapheme）と比較すると、
# Nimも明示的な区別が必要で、標準APIだけでは絵文字や結合文字の扱いが複雑。
#
# === Reml との比較メモ ===
#
# 1. **代数的データ型（ADT）**
#    Nim: object variant (case object) で ADT 風に記述
#    Reml: 型定義で直接 `type Inline = Text(string) | Strong(...) | ...` と記述
#    - Reml の方が構文が簡潔で、パターンマッチがより自然
#
# 2. **パーサーの実装アプローチ**
#    Nim: 手書き再帰下降パーサー、またはNPeg（PEGパーサーコンビネーター）
#    Reml: Core.Parse コンビネーターで宣言的に記述
#    - Reml はパーサーコンビネーターが標準ライブラリとして統合
#
# 3. **エラーハンドリング**
#    Nim: 例外機構（try-except）が主流、Option 型もサポート
#    Reml: Result<T, E> を標準で提供し、? 演算子で簡潔に記述
#    - Reml の方が関数型スタイルに統一
#
# 4. **性能**
#    Nim: C バックエンドにより、非常に高速
#    Reml: 実装次第だが、同等の性能を目指す
#    - Nim は既に成熟した高性能言語