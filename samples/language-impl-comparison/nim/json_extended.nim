## JSON拡張版：コメント・トレーリングカンマ対応。
##
## 標準JSONからの拡張点：
## 1. コメント対応（`//` 行コメント、`/* */` ブロックコメント）
## 2. トレーリングカンマ許可（配列・オブジェクトの最後の要素の後）
## 3. より詳細なエラーメッセージ
##
## 実用的な設定ファイル形式として：
## - `package.json` 風の設定ファイル
## - `.babelrc`, `.eslintrc` など開発ツールの設定
## - VS Code の `settings.json`

import tables, strutils

# 型定義

type
  JsonValueKind = enum
    jvkNull
    jvkBool
    jvkNumber
    jvkString
    jvkArray
    jvkObject

  JsonValue = ref object
    case kind: JsonValueKind
    of jvkNull: discard
    of jvkBool: boolVal: bool
    of jvkNumber: numVal: float
    of jvkString: strVal: string
    of jvkArray: arrayVal: seq[JsonValue]
    of jvkObject: objectVal: Table[string, JsonValue]

  ParseError = object
    msg: string

  State = object
    input: string
    pos: int

# コンストラクタ

proc newJNull(): JsonValue =
  JsonValue(kind: jvkNull)

proc newJBool(b: bool): JsonValue =
  JsonValue(kind: jvkBool, boolVal: b)

proc newJNumber(n: float): JsonValue =
  JsonValue(kind: jvkNumber, numVal: n)

proc newJString(s: string): JsonValue =
  JsonValue(kind: jvkString, strVal: s)

proc newJArray(items: seq[JsonValue]): JsonValue =
  JsonValue(kind: jvkArray, arrayVal: items)

proc newJObject(pairs: Table[string, JsonValue]): JsonValue =
  JsonValue(kind: jvkObject, objectVal: pairs)

# パース

proc skipWs(state: var State) =
  while state.pos < state.input.len and state.input[state.pos] in Whitespace:
    inc state.pos

proc skipLineComment(state: var State) =
  state.pos += 2  # "//" をスキップ
  while state.pos < state.input.len and state.input[state.pos] != '\n':
    inc state.pos
  if state.pos < state.input.len:
    inc state.pos  # '\n' をスキップ

proc skipBlockComment(state: var State): bool =
  state.pos += 2  # "/*" をスキップ
  while state.pos + 1 < state.input.len:
    if state.input[state.pos] == '*' and state.input[state.pos + 1] == '/':
      state.pos += 2
      return true
    inc state.pos
  return false

proc skipWhitespaceAndComments(state: var State): Result[void, ParseError] =
  while true:
    skipWs(state)
    if state.pos >= state.input.len:
      return ok()
    if state.pos + 1 < state.input.len:
      if state.input[state.pos..state.pos+1] == "//":
        skipLineComment(state)
      elif state.input[state.pos..state.pos+1] == "/*":
        if not skipBlockComment(state):
          return err(ParseError(msg: "ブロックコメントが閉じられていません"))
      else:
        return ok()
    else:
      return ok()

proc parseString(state: var State): Result[JsonValue, ParseError]

proc parseNumber(state: var State): Result[JsonValue, ParseError]

proc parseArray(state: var State): Result[JsonValue, ParseError]

proc parseObject(state: var State): Result[JsonValue, ParseError]

proc parseValue(state: var State): Result[JsonValue, ParseError] =
  ? skipWhitespaceAndComments(state)

  if state.pos >= state.input.len:
    return err(ParseError(msg: "予期しないEOF"))

  let remaining = state.input[state.pos..^1]

  if remaining.startsWith("null"):
    state.pos += 4
    return ok(newJNull())
  elif remaining.startsWith("true"):
    state.pos += 4
    return ok(newJBool(true))
  elif remaining.startsWith("false"):
    state.pos += 5
    return ok(newJBool(false))
  elif remaining[0] == '"':
    return parseString(state)
  elif remaining[0] == '[':
    return parseArray(state)
  elif remaining[0] == '{':
    return parseObject(state)
  else:
    return parseNumber(state)

proc parseString(state: var State): Result[JsonValue, ParseError] =
  inc state.pos  # '"' をスキップ
  var str = ""

  while state.pos < state.input.len:
    let ch = state.input[state.pos]
    if ch == '"':
      inc state.pos
      return ok(newJString(str))
    elif ch == '\\':
      if state.pos + 1 >= state.input.len:
        return err(ParseError(msg: "文字列が閉じられていません"))
      inc state.pos
      let escaped = state.input[state.pos]
      str.add(case escaped
        of 'n': '\n'
        of 't': '\t'
        of 'r': '\r'
        of '\\': '\\'
        of '"': '"'
        else: escaped
      )
      inc state.pos
    else:
      str.add(ch)
      inc state.pos

  return err(ParseError(msg: "文字列が閉じられていません"))

proc parseNumber(state: var State): Result[JsonValue, ParseError] =
  var numStr = ""

  while state.pos < state.input.len:
    let ch = state.input[state.pos]
    if ch in {'-', '+', '.', 'e', 'E'} or ch in Digits:
      numStr.add(ch)
      inc state.pos
    else:
      break

  try:
    let num = parseFloat(numStr)
    return ok(newJNumber(num))
  except ValueError:
    return err(ParseError(msg: "不正な数値: " & numStr))

proc parseArray(state: var State): Result[JsonValue, ParseError] =
  inc state.pos  # '[' をスキップ
  ? skipWhitespaceAndComments(state)

  if state.pos < state.input.len and state.input[state.pos] == ']':
    inc state.pos
    return ok(newJArray(@[]))

  var items: seq[JsonValue] = @[]

  while true:
    let value = ? parseValue(state)
    items.add(value)
    ? skipWhitespaceAndComments(state)

    if state.pos >= state.input.len:
      return err(ParseError(msg: "予期しないEOF"))

    if state.input[state.pos] == ',':
      inc state.pos
      ? skipWhitespaceAndComments(state)

      # トレーリングカンマチェック
      if state.pos < state.input.len and state.input[state.pos] == ']':
        inc state.pos
        return ok(newJArray(items))
    elif state.input[state.pos] == ']':
      inc state.pos
      return ok(newJArray(items))
    else:
      return err(ParseError(msg: "配列要素の後には ',' または ']' が必要です"))

proc parseObject(state: var State): Result[JsonValue, ParseError] =
  inc state.pos  # '{' をスキップ
  ? skipWhitespaceAndComments(state)

  if state.pos < state.input.len and state.input[state.pos] == '}':
    inc state.pos
    return ok(newJObject(initTable[string, JsonValue]()))

  var pairs = initTable[string, JsonValue]()

  while true:
    let keyValue = ? parseString(state)
    if keyValue.kind != jvkString:
      return err(ParseError(msg: "オブジェクトのキーは文字列である必要があります"))
    let key = keyValue.strVal

    ? skipWhitespaceAndComments(state)

    if state.pos >= state.input.len or state.input[state.pos] != ':':
      return err(ParseError(msg: "':' が必要です"))
    inc state.pos

    ? skipWhitespaceAndComments(state)

    let value = ? parseValue(state)
    pairs[key] = value

    ? skipWhitespaceAndComments(state)

    if state.pos >= state.input.len:
      return err(ParseError(msg: "予期しないEOF"))

    if state.input[state.pos] == ',':
      inc state.pos
      ? skipWhitespaceAndComments(state)

      # トレーリングカンマチェック
      if state.pos < state.input.len and state.input[state.pos] == '}':
        inc state.pos
        return ok(newJObject(pairs))
    elif state.input[state.pos] == '}':
      inc state.pos
      return ok(newJObject(pairs))
    else:
      return err(ParseError(msg: "オブジェクト要素の後には ',' または '}' が必要です"))

proc parse*(input: string): Result[JsonValue, ParseError] =
  var state = State(input: input, pos: 0)

  ? skipWhitespaceAndComments(state)
  let value = ? parseValue(state)
  ? skipWhitespaceAndComments(state)

  if state.pos < state.input.len:
    return err(ParseError(msg: "入力の終端に到達していません"))

  return ok(value)

# レンダリング

proc renderToString*(value: JsonValue, indentLevel: int = 0): string =
  let indent = repeat("  ", indentLevel)
  let nextIndent = repeat("  ", indentLevel + 1)

  case value.kind
  of jvkNull:
    return "null"
  of jvkBool:
    return if value.boolVal: "true" else: "false"
  of jvkNumber:
    return $value.numVal
  of jvkString:
    return "\"" & value.strVal & "\""
  of jvkArray:
    if value.arrayVal.len == 0:
      return "[]"
    var items: seq[string] = @[]
    for item in value.arrayVal:
      items.add(nextIndent & renderToString(item, indentLevel + 1))
    return "[\n" & items.join(",\n") & "\n" & indent & "]"
  of jvkObject:
    if value.objectVal.len == 0:
      return "{}"
    var pairs: seq[string] = @[]
    for key, val in value.objectVal:
      pairs.add(nextIndent & "\"" & key & "\": " & renderToString(val, indentLevel + 1))
    return "{\n" & pairs.join(",\n") & "\n" & indent & "}"

# テスト

proc testExtendedJson*() =
  let testCases = @[
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
  ]

  for (name, jsonStr) in testCases:
    echo "--- ", name, " ---"
    let result = parse(jsonStr)
    if result.isOk:
      echo "パース成功:"
      echo renderToString(result.get(), 0)
    else:
      echo "パースエラー: ", result.error.msg
    echo ""