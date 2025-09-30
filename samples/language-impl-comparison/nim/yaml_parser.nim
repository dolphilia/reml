import std/[strutils, tables, options, sequtils]

## YAML風パーサー：インデント管理が重要な題材。
##
## 対応する構文（簡易版）：
## - スカラー値: 文字列、数値、真偽値、null
## - リスト: `- item1`
## - マップ: `key: value`
## - ネストしたインデント構造
##
## インデント処理の特徴：
## - Nimのマクロとパーサーコンビネーターを活用
## - エラー回復機能でインデントミスを報告しつつ継続

type
  YamlValue* = ref object
    case kind*: YamlValueKind
    of yvkScalar: scalarVal*: string
    of yvkList: listVal*: seq[YamlValue]
    of yvkMap: mapVal*: Table[string, YamlValue]
    of yvkNull: discard

  YamlValueKind* = enum
    yvkScalar, yvkList, yvkMap, yvkNull

  Document* = YamlValue

  Parser* = ref object
    input: string
    pos: int

  ParseError* = object of CatchableError

# パーサーヘルパー関数

proc newParser(input: string): Parser =
  Parser(input: input, pos: 0)

proc peek(p: Parser): char =
  if p.pos < p.input.len:
    p.input[p.pos]
  else:
    '\0'

proc advance(p: Parser) =
  if p.pos < p.input.len:
    inc p.pos

proc isEof(p: Parser): bool =
  p.pos >= p.input.len

proc expect(p: Parser, expected: char) =
  if p.peek() != expected:
    raise newException(ParseError, "期待された文字 '" & expected & "' が見つかりません")
  p.advance()

proc expectString(p: Parser, expected: string) =
  for c in expected:
    p.expect(c)

# 水平空白のみをスキップ（改行は含まない）。
proc hspace(p: Parser) =
  while p.peek() in {' ', '\t'}:
    p.advance()

# 改行をスキップ。
proc newline(p: Parser) =
  if p.peek() == '\n':
    p.advance()
  elif p.peek() == '\r':
    p.advance()
    if p.peek() == '\n':
      p.advance()

# コメントのスキップ（`#` から行末まで）。
proc comment(p: Parser) =
  if p.peek() == '#':
    p.advance()
    while not p.isEof() and p.peek() != '\n':
      p.advance()

# 空行またはコメント行をスキップ。
proc blankOrComment(p: Parser) =
  p.hspace()
  p.comment()
  p.newline()

# 特定のインデントレベルを期待する。
proc expectIndent(p: Parser, level: int) =
  var spaces = 0
  while p.peek() == ' ':
    inc spaces
    p.advance()

  if spaces != level:
    raise newException(ParseError, "インデント不一致: 期待 " & $level & ", 実際 " & $spaces)

# 現在よりも深いインデントを検出。
proc deeperIndent(p: Parser, current: int): int =
  var spaces = 0
  while p.peek() == ' ':
    inc spaces
    p.advance()

  if spaces <= current:
    raise newException(ParseError, "深いインデントが期待されます: 現在 " & $current & ", 実際 " & $spaces)

  return spaces

# スカラー値のパース。
proc scalarValue(p: Parser): YamlValue

# YAML値のパース（前方宣言）。
proc parseValue(p: Parser, indent: int): YamlValue

# スカラー値のパース実装。
proc scalarValue(p: Parser): YamlValue =
  # null
  if p.input[p.pos..^1].startsWith("null"):
    p.expectString("null")
    return YamlValue(kind: yvkNull)

  if p.peek() == '~':
    p.advance()
    return YamlValue(kind: yvkNull)

  # 真偽値
  if p.input[p.pos..^1].startsWith("true"):
    p.expectString("true")
    return YamlValue(kind: yvkScalar, scalarVal: "true")

  if p.input[p.pos..^1].startsWith("false"):
    p.expectString("false")
    return YamlValue(kind: yvkScalar, scalarVal: "false")

  # 数値（簡易実装）
  var numStr = ""
  while p.peek() in {'0'..'9'}:
    numStr.add(p.peek())
    p.advance()

  if numStr.len > 0:
    return YamlValue(kind: yvkScalar, scalarVal: numStr)

  # 文字列（引用符付き）
  if p.peek() == '"':
    p.advance()
    var str = ""
    while p.peek() != '"' and not p.isEof():
      str.add(p.peek())
      p.advance()
    p.expect('"')
    return YamlValue(kind: yvkScalar, scalarVal: str)

  # 文字列（引用符なし：行末または `:` まで）
  var str = ""
  while not p.isEof() and p.peek() notin {'\n', ':', '#'}:
    str.add(p.peek())
    p.advance()

  return YamlValue(kind: yvkScalar, scalarVal: str.strip())

# リスト項目のパース（`- value` 形式）。
proc parseListItem(p: Parser, indent: int): YamlValue =
  p.expectIndent(indent)
  p.expect('-')
  p.hspace()
  return p.parseValue(indent + 2)

# リスト全体のパース。
proc parseList(p: Parser, indent: int): YamlValue =
  var items: seq[YamlValue] = @[]

  while true:
    try:
      let item = p.parseListItem(indent)
      items.add(item)

      if p.peek() == '\n':
        p.newline()
      else:
        break
    except ParseError:
      break

  if items.len == 0:
    raise newException(ParseError, "リストが空です")

  return YamlValue(kind: yvkList, listVal: items)

# マップのキーバリューペアのパース（`key: value` 形式）。
proc parseMapEntry(p: Parser, indent: int): (string, YamlValue) =
  p.expectIndent(indent)

  var key = ""
  while not p.isEof() and p.peek() notin {':', '\n'}:
    key.add(p.peek())
    p.advance()

  key = key.strip()
  p.expect(':')
  p.hspace()

  var value: YamlValue

  # 同じ行に値があるか、次の行にネストされているか
  if p.peek() == '\n':
    p.newline()
    value = p.parseValue(indent + 2)
  else:
    value = p.parseValue(indent)

  return (key, value)

# マップ全体のパース。
proc parseMap(p: Parser, indent: int): YamlValue =
  var entries = initTable[string, YamlValue]()

  while true:
    try:
      let (key, value) = p.parseMapEntry(indent)
      entries[key] = value

      if p.peek() == '\n':
        p.newline()
      else:
        break
    except ParseError:
      break

  if entries.len == 0:
    raise newException(ParseError, "マップが空です")

  return YamlValue(kind: yvkMap, mapVal: entries)

# YAML値のパース（再帰的）実装。
proc parseValue(p: Parser, indent: int): YamlValue =
  let startPos = p.pos

  # リストを試行
  try:
    return p.parseList(indent)
  except ParseError:
    p.pos = startPos

  # マップを試行
  try:
    return p.parseMap(indent)
  except ParseError:
    p.pos = startPos

  # スカラー
  return p.scalarValue()

# ドキュメント全体のパース。
proc document(p: Parser): Document =
  # 空行やコメントをスキップ
  while not p.isEof():
    let startPos = p.pos
    try:
      p.blankOrComment()
    except ParseError:
      p.pos = startPos
      break

  let doc = p.parseValue(0)

  # 末尾の空行やコメントをスキップ
  while not p.isEof():
    try:
      p.blankOrComment()
    except ParseError:
      break

  if not p.isEof():
    raise newException(ParseError, "ドキュメントの終端が期待されます")

  return doc

# パブリックAPI：YAML文字列をパース。
proc parseYaml*(input: string): Document =
  let p = newParser(input)
  return p.document()

# 簡易的なレンダリング（検証用）。
proc renderToString*(doc: Document): string =
  proc renderValue(value: YamlValue, indent: int): string =
    let indentStr = " ".repeat(indent)

    case value.kind
    of yvkScalar:
      return value.scalarVal
    of yvkNull:
      return "null"
    of yvkList:
      return value.listVal.mapIt(indentStr & "- " & renderValue(it, indent + 2)).join("\n")
    of yvkMap:
      var lines: seq[string] = @[]
      for key, val in value.mapVal.pairs:
        case val.kind
        of yvkScalar, yvkNull:
          lines.add(indentStr & key & ": " & renderValue(val, 0))
        else:
          lines.add(indentStr & key & ":\n" & renderValue(val, indent + 2))
      return lines.join("\n")

  return renderValue(doc, 0)

# テスト例。
proc testExamples*() =
  let examples = [
    ("simple_scalar", "hello"),
    ("simple_list", "- item1\n- item2\n- item3"),
    ("simple_map", "key1: value1\nkey2: value2"),
    ("nested_map", "parent:\n  child1: value1\n  child2: value2"),
    ("nested_list", "items:\n  - item1\n  - item2"),
    ("mixed", "name: John\nage: 30\nhobbies:\n  - reading\n  - coding")
  ]

  for (name, yamlStr) in examples:
    echo "--- ", name, " ---"
    try:
      let doc = parseYaml(yamlStr)
      echo "パース成功:"
      echo renderToString(doc)
    except ParseError as e:
      echo "パースエラー: ", e.msg

## インデント処理の課題と解決策：
##
## 1. **インデントレベルの追跡**
##    - パーサー引数としてインデントレベルを渡す
##    - Nimの参照型でパーサー状態を管理
##
## 2. **エラー回復**
##    - try/exceptでバックトラックを制御
##    - ParseError例外で分かりやすいエラーメッセージを提供
##
## 3. **空白の扱い**
##    - hspaceで水平空白のみをスキップ（改行は構文の一部）
##    - newlineでCR/LF/CRLFを正規化
##
## Remlとの比較：
##
## - **Nimの利点**:
##   - シンプルで読みやすい手書きパーサー
##   - 効率的なメモリ管理
##
## - **Nimの課題**:
##   - パーサーコンビネーターライブラリがRemlほど充実していない
##   - 手動のバックトラック管理が煩雑
##
## - **Remlの利点**:
##   - 字句レイヤの柔軟性により、インデント処理が自然に表現できる
##   - cut/commitによるエラー品質の向上
##   - recoverによる部分的なパース継続が可能