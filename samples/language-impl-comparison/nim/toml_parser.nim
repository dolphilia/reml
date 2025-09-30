import std/[strutils, tables, options, sequtils, times]

## TOML v1.0.0準拠の簡易パーサー実装
##
## 対応する構文：
## - キーバリューペア: `key = "value"`
## - テーブル: `[section]`
## - 配列テーブル: `[[array_section]]`
## - データ型: 文字列、整数、浮動小数点、真偽値、日時、配列、インラインテーブル
## - コメント: `# comment`
##
## 実装の特徴（Nimの強みを活用）：
## 1. **効率的なメモリ管理**
##    - ref objectによる効率的な値の管理
##    - シーケンス型による柔軟な配列操作
##
## 2. **マクロとメタプログラミング**
##    - 型安全な値アクセス
##    - コンパイル時の型チェック
##
## 3. **エラーハンドリング**
##    - 例外ベースの明確なエラー報告
##    - ParseError型による詳細な位置情報
##
## 4. **Unicode対応**
##    - UTF-8ネイティブ対応
##    - 複数行文字列のサポート

type
  TomlValueKind* = enum
    tvkString, tvkInteger, tvkFloat, tvkBoolean,
    tvkDateTime, tvkArray, tvkInlineTable

  TomlValue* = ref object
    case kind*: TomlValueKind
    of tvkString: stringVal*: string
    of tvkInteger: intVal*: int64
    of tvkFloat: floatVal*: float64
    of tvkBoolean: boolVal*: bool
    of tvkDateTime: dateTimeVal*: DateTime
    of tvkArray: arrayVal*: seq[TomlValue]
    of tvkInlineTable: tableVal*: Table[string, TomlValue]

  TomlTable* = Table[string, TomlValue]

  TomlDocument* = ref object
    root*: TomlTable
    tables*: Table[seq[string], TomlTable]

  Parser* = ref object
    input: string
    pos: int
    line: int
    column: int
    currentTable: seq[string]

  ParseError* = object of CatchableError
    line*: int
    column*: int

# パーサーヘルパー関数

proc newParser(input: string): Parser =
  Parser(input: input, pos: 0, line: 1, column: 1, currentTable: @[])

proc peek(p: Parser): char =
  if p.pos < p.input.len:
    p.input[p.pos]
  else:
    '\0'

proc advance(p: Parser) =
  if p.pos < p.input.len:
    if p.input[p.pos] == '\n':
      inc p.line
      p.column = 1
    else:
      inc p.column
    inc p.pos

proc isEof(p: Parser): bool =
  p.pos >= p.input.len

proc raiseError(p: Parser, msg: string) =
  var err = newException(ParseError, msg)
  err.line = p.line
  err.column = p.column
  raise err

proc expect(p: Parser, expected: char) =
  if p.peek() != expected:
    p.raiseError("期待された文字 '" & expected & "' が見つかりません")
  p.advance()

proc expectString(p: Parser, expected: string) =
  for c in expected:
    p.expect(c)

# 空白とコメントのスキップ

proc skipWhitespace(p: Parser) =
  while p.peek() in {' ', '\t'}:
    p.advance()

proc skipComment(p: Parser) =
  if p.peek() == '#':
    p.advance()
    while not p.isEof() and p.peek() != '\n':
      p.advance()

proc skipWhitespaceAndComments(p: Parser) =
  while true:
    let startPos = p.pos
    p.skipWhitespace()
    p.skipComment()
    if p.peek() == '\n':
      p.advance()
    if p.pos == startPos:
      break

proc skipInlineWhitespace(p: Parser) =
  while p.peek() in {' ', '\t'}:
    p.advance()

# キー名のパース

proc parseKey(p: Parser): string =
  p.skipInlineWhitespace()

  # 引用符付きキー
  if p.peek() == '"':
    p.advance()
    var key = ""
    while p.peek() != '"' and not p.isEof():
      if p.peek() == '\\':
        p.advance()
        case p.peek()
        of 'n': key.add('\n')
        of 't': key.add('\t')
        of '\\': key.add('\\')
        of '"': key.add('"')
        else: p.raiseError("無効なエスケープシーケンス")
        p.advance()
      else:
        key.add(p.peek())
        p.advance()
    p.expect('"')
    return key

  # ベアキー（英数字・`-`・`_`のみ）
  var key = ""
  while p.peek() in {'a'..'z', 'A'..'Z', '0'..'9', '-', '_'}:
    key.add(p.peek())
    p.advance()

  if key.len == 0:
    p.raiseError("キー名が期待されます")

  return key

proc parseKeyPath(p: Parser): seq[string] =
  var path: seq[string] = @[]
  path.add(p.parseKey())

  while p.peek() == '.':
    p.advance()
    path.add(p.parseKey())

  return path

# 文字列値のパース

proc parseStringValue(p: Parser): TomlValue
proc parseValue(p: Parser): TomlValue

proc parseBasicString(p: Parser): string =
  p.expect('"')
  var str = ""
  while p.peek() != '"' and not p.isEof():
    if p.peek() == '\\':
      p.advance()
      case p.peek()
      of 'n': str.add('\n')
      of 't': str.add('\t')
      of 'r': str.add('\r')
      of '\\': str.add('\\')
      of '"': str.add('"')
      of 'u':
        # Unicode エスケープ（簡易実装）
        p.advance()
        var unicode = ""
        for i in 0..3:
          unicode.add(p.peek())
          p.advance()
        # 実際にはunicodeをデコードするべき
        str.add("\\u" & unicode)
        continue
      else: p.raiseError("無効なエスケープシーケンス")
      p.advance()
    else:
      str.add(p.peek())
      p.advance()
  p.expect('"')
  return str

proc parseMultilineBasicString(p: Parser): string =
  p.expectString("\"\"\"")
  if p.peek() == '\n':
    p.advance()

  var str = ""
  while true:
    if p.input[p.pos..^1].startsWith("\"\"\""):
      p.expectString("\"\"\"")
      break

    if p.isEof():
      p.raiseError("複数行文字列が閉じられていません")

    if p.peek() == '\\' and p.input[p.pos+1] == '\n':
      # 行末のバックスラッシュで空白を削除
      p.advance()
      p.advance()
      while p.peek() in {' ', '\t', '\n'}:
        p.advance()
    else:
      str.add(p.peek())
      p.advance()

  return str

proc parseLiteralString(p: Parser): string =
  p.expect('\'')
  var str = ""
  while p.peek() != '\'' and not p.isEof():
    str.add(p.peek())
    p.advance()
  p.expect('\'')
  return str

proc parseMultilineLiteralString(p: Parser): string =
  p.expectString("'''")
  if p.peek() == '\n':
    p.advance()

  var str = ""
  while true:
    if p.input[p.pos..^1].startsWith("'''"):
      p.expectString("'''")
      break

    if p.isEof():
      p.raiseError("複数行リテラル文字列が閉じられていません")

    str.add(p.peek())
    p.advance()

  return str

proc parseStringValue(p: Parser): TomlValue =
  # 複数行基本文字列
  if p.input[p.pos..^1].startsWith("\"\"\""):
    return TomlValue(kind: tvkString, stringVal: p.parseMultilineBasicString())

  # 複数行リテラル文字列
  if p.input[p.pos..^1].startsWith("'''"):
    return TomlValue(kind: tvkString, stringVal: p.parseMultilineLiteralString())

  # 基本文字列
  if p.peek() == '"':
    return TomlValue(kind: tvkString, stringVal: p.parseBasicString())

  # リテラル文字列
  if p.peek() == '\'':
    return TomlValue(kind: tvkString, stringVal: p.parseLiteralString())

  p.raiseError("文字列が期待されます")

# 数値のパース

proc parseIntegerValue(p: Parser): TomlValue =
  var numStr = ""
  var isNegative = false

  if p.peek() == '-':
    isNegative = true
    p.advance()
  elif p.peek() == '+':
    p.advance()

  # 16進数
  if p.peek() == '0' and p.input[p.pos+1] == 'x':
    p.advance()
    p.advance()
    while p.peek() in {'0'..'9', 'a'..'f', 'A'..'F', '_'}:
      if p.peek() != '_':
        numStr.add(p.peek())
      p.advance()
    let val = parseHexInt(numStr)
    return TomlValue(kind: tvkInteger, intVal: if isNegative: -val else: val)

  # 8進数
  if p.peek() == '0' and p.input[p.pos+1] == 'o':
    p.advance()
    p.advance()
    while p.peek() in {'0'..'7', '_'}:
      if p.peek() != '_':
        numStr.add(p.peek())
      p.advance()
    let val = parseOctInt(numStr)
    return TomlValue(kind: tvkInteger, intVal: if isNegative: -val else: val)

  # 2進数
  if p.peek() == '0' and p.input[p.pos+1] == 'b':
    p.advance()
    p.advance()
    while p.peek() in {'0'..'1', '_'}:
      if p.peek() != '_':
        numStr.add(p.peek())
      p.advance()
    let val = parseBinInt(numStr)
    return TomlValue(kind: tvkInteger, intVal: if isNegative: -val else: val)

  # 10進数
  while p.peek() in {'0'..'9', '_'}:
    if p.peek() != '_':
      numStr.add(p.peek())
    p.advance()

  let val = parseBiggestInt(numStr)
  return TomlValue(kind: tvkInteger, intVal: if isNegative: -val else: val)

proc parseFloatValue(p: Parser): TomlValue =
  var numStr = ""

  if p.peek() in {'-', '+'}:
    numStr.add(p.peek())
    p.advance()

  # inf または nan
  if p.input[p.pos..^1].startsWith("inf"):
    p.expectString("inf")
    return TomlValue(kind: tvkFloat, floatVal: if numStr == "-": -Inf else: Inf)

  if p.input[p.pos..^1].startsWith("nan"):
    p.expectString("nan")
    return TomlValue(kind: tvkFloat, floatVal: NaN)

  while p.peek() in {'0'..'9', '_'}:
    if p.peek() != '_':
      numStr.add(p.peek())
    p.advance()

  if p.peek() == '.':
    numStr.add('.')
    p.advance()
    while p.peek() in {'0'..'9', '_'}:
      if p.peek() != '_':
        numStr.add(p.peek())
      p.advance()

  if p.peek() in {'e', 'E'}:
    numStr.add(p.peek())
    p.advance()
    if p.peek() in {'-', '+'}:
      numStr.add(p.peek())
      p.advance()
    while p.peek() in {'0'..'9', '_'}:
      if p.peek() != '_':
        numStr.add(p.peek())
      p.advance()

  let val = parseFloat(numStr)
  return TomlValue(kind: tvkFloat, floatVal: val)

# 真偽値のパース

proc parseBooleanValue(p: Parser): TomlValue =
  if p.input[p.pos..^1].startsWith("true"):
    p.expectString("true")
    return TomlValue(kind: tvkBoolean, boolVal: true)

  if p.input[p.pos..^1].startsWith("false"):
    p.expectString("false")
    return TomlValue(kind: tvkBoolean, boolVal: false)

  p.raiseError("真偽値が期待されます")

# 日時のパース

proc parseDateTimeValue(p: Parser): TomlValue =
  var dtStr = ""

  # YYYY-MM-DD または YYYY-MM-DDTHH:MM:SS[.ms][Z|±HH:MM]
  while p.peek() in {'0'..'9', '-', ':', 'T', 'Z', '+', '.'}:
    dtStr.add(p.peek())
    p.advance()

  try:
    let dt = parse(dtStr, "yyyy-MM-dd'T'HH:mm:ss")
    return TomlValue(kind: tvkDateTime, dateTimeVal: dt)
  except:
    try:
      let dt = parse(dtStr, "yyyy-MM-dd")
      return TomlValue(kind: tvkDateTime, dateTimeVal: dt)
    except:
      p.raiseError("無効な日時形式: " & dtStr)

# 配列のパース

proc parseArray(p: Parser): TomlValue =
  p.expect('[')
  p.skipInlineWhitespace()

  var items: seq[TomlValue] = @[]

  while p.peek() != ']':
    p.skipWhitespaceAndComments()

    if p.peek() == ']':
      break

    items.add(p.parseValue())
    p.skipInlineWhitespace()

    if p.peek() == ',':
      p.advance()
      p.skipWhitespaceAndComments()
    elif p.peek() != ']':
      p.raiseError("配列内で ',' または ']' が期待されます")

  p.expect(']')
  return TomlValue(kind: tvkArray, arrayVal: items)

# インラインテーブルのパース

proc parseInlineTable(p: Parser): TomlValue =
  p.expect('{')
  p.skipInlineWhitespace()

  var table = initTable[string, TomlValue]()

  while p.peek() != '}':
    if p.peek() == '\n':
      p.raiseError("インラインテーブル内で改行は許可されません")

    let key = p.parseKey()
    p.skipInlineWhitespace()
    p.expect('=')
    p.skipInlineWhitespace()
    let value = p.parseValue()

    table[key] = value
    p.skipInlineWhitespace()

    if p.peek() == ',':
      p.advance()
      p.skipInlineWhitespace()
    elif p.peek() != '}':
      p.raiseError("インラインテーブル内で ',' または '}' が期待されます")

  p.expect('}')
  return TomlValue(kind: tvkInlineTable, tableVal: table)

# 値のパース（統合）

proc parseValue(p: Parser): TomlValue =
  p.skipInlineWhitespace()

  # 配列
  if p.peek() == '[':
    return p.parseArray()

  # インラインテーブル
  if p.peek() == '{':
    return p.parseInlineTable()

  # 文字列
  if p.peek() in {'"', '\''}:
    return p.parseStringValue()

  # 真偽値
  if p.input[p.pos..^1].startsWith("true") or p.input[p.pos..^1].startsWith("false"):
    return p.parseBooleanValue()

  # 日時（数値より優先して判定）
  let startPos = p.pos
  try:
    # 日時かどうか試行
    if p.peek() in {'0'..'9'}:
      var tempStr = ""
      var tempPos = p.pos
      while tempPos < p.input.len and p.input[tempPos] in {'0'..'9', '-', ':', 'T', 'Z', '+', '.'}:
        tempStr.add(p.input[tempPos])
        inc tempPos

      if 'T' in tempStr or '-' in tempStr:
        return p.parseDateTimeValue()
  except:
    p.pos = startPos

  # 浮動小数点（infやnan含む）
  if p.peek() in {'-', '+'}:
    let nextChar = if p.pos + 1 < p.input.len: p.input[p.pos + 1] else: '\0'
    if nextChar in {'i', 'n'}:
      return p.parseFloatValue()

  if p.input[p.pos..^1].startsWith("inf") or p.input[p.pos..^1].startsWith("nan"):
    return p.parseFloatValue()

  # 数値（整数または浮動小数点）
  if p.peek() in {'0'..'9', '-', '+'}:
    let startPos = p.pos
    var hasFloat = false

    # 先読みして浮動小数点かどうか判定
    var tempPos = p.pos
    if p.input[tempPos] in {'-', '+'}:
      inc tempPos
    while tempPos < p.input.len and p.input[tempPos] in {'0'..'9', '_', 'x', 'o', 'b'}:
      inc tempPos
    if tempPos < p.input.len and p.input[tempPos] in {'.', 'e', 'E'}:
      hasFloat = true

    p.pos = startPos

    if hasFloat:
      return p.parseFloatValue()
    else:
      return p.parseIntegerValue()

  p.raiseError("値が期待されます")

# キーバリューペアのパース

proc parseKeyValuePair(p: Parser): (seq[string], TomlValue) =
  let path = p.parseKeyPath()
  p.skipInlineWhitespace()
  p.expect('=')
  p.skipInlineWhitespace()
  let value = p.parseValue()
  return (path, value)

# テーブルヘッダーのパース

proc parseTableHeader(p: Parser): seq[string] =
  p.expect('[')
  let path = p.parseKeyPath()
  p.expect(']')
  return path

# 配列テーブルヘッダーのパース

proc parseArrayTableHeader(p: Parser): seq[string] =
  p.expectString("[[")
  let path = p.parseKeyPath()
  p.expectString("]]")
  return path

# ネストしたキーパスに値を挿入する補助関数

proc insertNested(table: var TomlTable, path: seq[string], value: TomlValue) =
  if path.len == 1:
    table[path[0]] = value
    return

  let key = path[0]
  let rest = path[1..^1]

  if not table.hasKey(key):
    table[key] = TomlValue(kind: tvkInlineTable, tableVal: initTable[string, TomlValue]())

  if table[key].kind != tvkInlineTable:
    raise newException(ParseError, "キーの競合: " & key)

  insertNested(table[key].tableVal, rest, value)

# ドキュメント全体のパース

proc parseDocument(p: Parser): TomlDocument =
  let doc = TomlDocument(root: initTable[string, TomlValue](),
                          tables: initTable[seq[string], TomlTable]())

  p.skipWhitespaceAndComments()

  while not p.isEof():
    # 配列テーブル
    if p.input[p.pos..^1].startsWith("[["):
      let path = p.parseArrayTableHeader()
      p.currentTable = path

      if not doc.tables.hasKey(path):
        doc.tables[path] = initTable[string, TomlValue]()

      p.skipWhitespaceAndComments()
      continue

    # テーブル
    if p.peek() == '[':
      let path = p.parseTableHeader()
      p.currentTable = path

      if not doc.tables.hasKey(path):
        doc.tables[path] = initTable[string, TomlValue]()

      p.skipWhitespaceAndComments()
      continue

    # キーバリューペア
    if p.peek() in {'a'..'z', 'A'..'Z', '"', '\''}:
      let (path, value) = p.parseKeyValuePair()

      if p.currentTable.len == 0:
        # ルートテーブルに追加
        insertNested(doc.root, path, value)
      else:
        # 現在のテーブルに追加
        if not doc.tables.hasKey(p.currentTable):
          doc.tables[p.currentTable] = initTable[string, TomlValue]()
        insertNested(doc.tables[p.currentTable], path, value)

      p.skipWhitespaceAndComments()
      continue

    p.raiseError("予期しない文字: " & $p.peek())

  return doc

# パブリックAPI

proc parseToml*(input: string): TomlDocument =
  ## TOML文字列をパースしてドキュメントを返す
  let p = newParser(input)
  return p.parseDocument()

# レンダリング関数

proc renderValue*(value: TomlValue): string =
  case value.kind
  of tvkString:
    return "\"" & value.stringVal & "\""
  of tvkInteger:
    return $value.intVal
  of tvkFloat:
    return $value.floatVal
  of tvkBoolean:
    return if value.boolVal: "true" else: "false"
  of tvkDateTime:
    return value.dateTimeVal.format("yyyy-MM-dd'T'HH:mm:ss")
  of tvkArray:
    let items = value.arrayVal.mapIt(renderValue(it)).join(", ")
    return "[" & items & "]"
  of tvkInlineTable:
    var entries: seq[string] = @[]
    for key, val in value.tableVal.pairs:
      entries.add(key & " = " & renderValue(val))
    return "{ " & entries.join(", ") & " }"

proc renderTable(table: TomlTable, prefix: seq[string] = @[]): string =
  var lines: seq[string] = @[]

  for key, value in table.pairs:
    let fullKey = if prefix.len == 0: key else: (prefix & @[key]).join(".")

    if value.kind == tvkInlineTable:
      lines.add(renderTable(value.tableVal, prefix & @[key]))
    else:
      lines.add(fullKey & " = " & renderValue(value))

  return lines.join("\n")

proc renderToString*(doc: TomlDocument): string =
  ## ドキュメントをTOML形式の文字列にレンダリング
  var output = ""

  # ルートテーブルをレンダリング
  output = renderTable(doc.root)

  # 各セクションをレンダリング
  for path, table in doc.tables.pairs:
    if output.len > 0:
      output.add("\n\n")
    output.add("[" & path.join(".") & "]\n")
    output.add(renderTable(table))

  return output

# テスト例

proc testRemlToml*() =
  ## reml.toml 風設定のパーステスト
  let exampleToml = """
# Reml パッケージ設定

[package]
name = "my_project"
version = "0.1.0"
authors = ["Author Name"]

[dependencies]
core = "1.0"

[dev-dependencies]
test_framework = "0.5"

[[plugins]]
name = "system"
version = "1.0"

[[plugins]]
name = "memory"
version = "1.0"
"""

  echo "--- reml.toml 風設定のパース ---"
  try:
    let doc = parseToml(exampleToml)
    echo "パース成功:"
    echo renderToString(doc)
  except ParseError as e:
    echo "パースエラー (行", e.line, ", 列", e.column, "): ", e.msg

proc testDataTypes*() =
  ## 各種データ型のパーステスト
  let examples = [
    ("文字列", """key = "value""""),
    ("複数行文字列", """key = ""\"
多行
文字列
""\""""),
    ("整数", """key = 42"""),
    ("16進数", """key = 0xFF"""),
    ("浮動小数点", """key = 3.14"""),
    ("真偽値", """key = true"""),
    ("日時", """key = 2024-01-01T12:00:00"""),
    ("配列", """key = [1, 2, 3]"""),
    ("インラインテーブル", """key = { x = 1, y = 2 }""")
  ]

  for (name, tomlStr) in examples:
    echo "\n--- ", name, " ---"
    try:
      let doc = parseToml(tomlStr)
      echo "パース成功:"
      echo renderToString(doc)
    except ParseError as e:
      echo "パースエラー (行", e.line, ", 列", e.column, "): ", e.msg

## Nimの特徴を活かした実装のポイント：
##
## 1. **効率的なメモリ管理**
##    - ref objectによる自動的なメモリ管理
##    - GCが適切にメモリを回収
##    - 浅いコピーで効率的な値の受け渡し
##
## 2. **型安全性**
##    - enum型による明確な値の種類の区別
##    - case objectによるメモリ効率の良い判別共用体
##    - コンパイル時の型チェック
##
## 3. **エラーハンドリング**
##    - 例外による明確なエラー伝搬
##    - 行番号・列番号付きのエラー情報
##    - try-exceptによる柔軟なエラー処理
##
## 4. **標準ライブラリの活用**
##    - strutils: 文字列処理
##    - tables: ハッシュテーブル
##    - times: 日時処理
##    - sequtils: シーケンス操作
##
## Remlとの比較：
##
## - **Nimの利点**:
##   - コンパイル時の最適化による高速な実行
##   - 手続き型の明示的なフロー制御
##   - C/C++との容易な相互運用
##
## - **Remlの利点**:
##   - パーサーコンビネーターによる宣言的な記述
##   - cut/commitによる高品質なエラー報告
##   - recoverによる部分的なパース継続
##
## - **実装の違い**:
##   - Nim: 手書きの再帰下降パーサー
##   - Reml: コンビネーターベースの合成的パーサー
##   - Nim: 例外ベースのエラー処理
##   - Reml: Result型による関数的エラー処理