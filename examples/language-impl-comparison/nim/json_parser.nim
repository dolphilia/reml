# JSON パーサー - Nim 版
# Reml との比較ポイント: パーサーコンビネーター vs NPeg、代数的データ型

import std/[tables, strutils, sequtils, options]

# JSON 構文を解析して汎用値型に変換する
type
  JsonValue = ref object
    case kind: JsonKind
    of jkNull: discard
    of jkBool: boolVal: bool
    of jkNumber: numVal: float
    of jkString: strVal: string
    of jkArray: items: seq[JsonValue]
    of jkObject: fields: Table[string, JsonValue]

  JsonKind = enum
    jkNull, jkBool, jkNumber, jkString, jkArray, jkObject

  Token = ref object
    case kind: TokenKind
    of tkLBrace, tkRBrace, tkLBracket, tkRBracket, tkColon, tkComma: discard
    of tkString: strVal: string
    of tkNumber: numVal: float
    of tkBool: boolVal: bool
    of tkNull: discard

  TokenKind = enum
    tkLBrace, tkRBrace, tkLBracket, tkRBracket, tkColon, tkComma,
    tkString, tkNumber, tkBool, tkNull

  ParseError = object of CatchableError

# === コンストラクタヘルパー ===

proc newJsonNull(): JsonValue =
  JsonValue(kind: jkNull)

proc newJsonBool(b: bool): JsonValue =
  JsonValue(kind: jkBool, boolVal: b)

proc newJsonNumber(n: float): JsonValue =
  JsonValue(kind: jkNumber, numVal: n)

proc newJsonString(s: string): JsonValue =
  JsonValue(kind: jkString, strVal: s)

proc newJsonArray(items: seq[JsonValue]): JsonValue =
  JsonValue(kind: jkArray, items: items)

proc newJsonObject(fields: Table[string, JsonValue]): JsonValue =
  JsonValue(kind: jkObject, fields: fields)

# === トークナイズ（簡易実装） ===

proc tokenize(source: string): seq[Token] =
  # 実装簡略化のため、空白で分割したシンプルなトークナイザー
  # 実用段階では文字列内の空白や記号を正しく処理する
  let cleaned = source
    .replace("{", " { ")
    .replace("}", " } ")
    .replace("[", " [ ")
    .replace("]", " ] ")
    .replace(":", " : ")
    .replace(",", " , ")

  for token in cleaned.splitWhitespace():
    if token == "{":
      result.add(Token(kind: tkLBrace))
    elif token == "}":
      result.add(Token(kind: tkRBrace))
    elif token == "[":
      result.add(Token(kind: tkLBracket))
    elif token == "]":
      result.add(Token(kind: tkRBracket))
    elif token == ":":
      result.add(Token(kind: tkColon))
    elif token == ",":
      result.add(Token(kind: tkComma))
    elif token == "null":
      result.add(Token(kind: tkNull))
    elif token == "true":
      result.add(Token(kind: tkBool, boolVal: true))
    elif token == "false":
      result.add(Token(kind: tkBool, boolVal: false))
    elif token.startsWith("\""):
      result.add(Token(kind: tkString, strVal: token.strip(chars = {'"'})))
    else:
      try:
        result.add(Token(kind: tkNumber, numVal: parseFloat(token)))
      except ValueError:
        result.add(Token(kind: tkString, strVal: token))

# === パース ===

proc parseValue(tokens: var seq[Token]): JsonValue

proc parseArray(tokens: var seq[Token]): JsonValue =
  var items: seq[JsonValue] = @[]

  if tokens.len > 0 and tokens[0].kind == tkRBracket:
    tokens.delete(0)
    return newJsonArray(items)

  while true:
    items.add(parseValue(tokens))

    if tokens.len == 0:
      raise newException(ParseError, "予期しない入力の終端")

    if tokens[0].kind == tkComma:
      tokens.delete(0)
    elif tokens[0].kind == tkRBracket:
      tokens.delete(0)
      return newJsonArray(items)
    else:
      raise newException(ParseError, "期待: ] or ,")

proc parseObject(tokens: var seq[Token]): JsonValue =
  var fields = initTable[string, JsonValue]()

  if tokens.len > 0 and tokens[0].kind == tkRBrace:
    tokens.delete(0)
    return newJsonObject(fields)

  while true:
    # キーを取得
    if tokens.len == 0 or tokens[0].kind != tkString:
      raise newException(ParseError, "期待: 文字列キー")
    let key = tokens[0].strVal
    tokens.delete(0)

    # コロンを期待
    if tokens.len == 0 or tokens[0].kind != tkColon:
      raise newException(ParseError, "期待: :")
    tokens.delete(0)

    # 値をパース
    let value = parseValue(tokens)
    fields[key] = value

    if tokens.len == 0:
      raise newException(ParseError, "予期しない入力の終端")

    if tokens[0].kind == tkComma:
      tokens.delete(0)
    elif tokens[0].kind == tkRBrace:
      tokens.delete(0)
      return newJsonObject(fields)
    else:
      raise newException(ParseError, "期待: } or ,")

proc parseValue(tokens: var seq[Token]): JsonValue =
  if tokens.len == 0:
    raise newException(ParseError, "予期しない入力の終端")

  let token = tokens[0]
  tokens.delete(0)

  case token.kind
  of tkNull:
    return newJsonNull()
  of tkBool:
    return newJsonBool(token.boolVal)
  of tkNumber:
    return newJsonNumber(token.numVal)
  of tkString:
    return newJsonString(token.strVal)
  of tkLBracket:
    return parseArray(tokens)
  of tkLBrace:
    return parseObject(tokens)
  else:
    raise newException(ParseError, "予期しないトークン")

# === メイン パース関数 ===

proc parseJson*(source: string): JsonValue =
  var tokens = tokenize(source)
  let value = parseValue(tokens)

  if tokens.len > 0:
    raise newException(ParseError, "末尾に未消費トークンがあります")

  value

# === テスト ===

when isMainModule:
  echo "=== Nim JSON パーサー ==="

  try:
    let json1 = parseJson("""{"key": 123}""")
    echo "Parsed: ", json1.kind  # => jkObject

    let json2 = parseJson("[1, 2, 3]")
    echo "Parsed: ", json2.kind  # => jkArray
  except ParseError as e:
    echo "Parse error: ", e.msg

# === Reml との比較メモ ===

# 1. **代数的データ型（ADT）**
#    Nim: object variant (case object) で ADT 風に記述
#         `case kind: JsonKind` で型タグを明示的に持つ
#    Reml: 型定義で直接 `type JsonValue = JNull | JBool(bool) | ...` と記述
#         型タグは暗黙的に管理される
#    - Reml の方が構文が簡潔で、パターンマッチがより自然

# 2. **パーサーの実装アプローチ**
#    Nim: 手書き再帰下降パーサー、またはNPeg（PEGパーサーコンビネーター）
#         NPegを使えば、Remlに近い宣言的な記述が可能
#    Reml: Core.Parse コンビネーターで宣言的に記述
#         演算子優先度、cut/commit、エラー回復などが組み込み
#    - Reml はパーサーコンビネーターが標準ライブラリとして統合
#    - Nim は NPeg を使えば同等の機能が得られる

# 3. **エラーハンドリング**
#    Nim: 例外機構（try-except）が主流
#         Option 型もサポート
#    Reml: Result<T, E> を標準で提供し、? 演算子で簡潔に記述
#    - Reml の方が関数型スタイルに統一

# 4. **性能**
#    Nim: C バックエンドにより、非常に高速
#         コンパイル時にメモリレイアウトが最適化される
#    Reml: 実装次第だが、同等の性能を目指す
#    - Nim は既に成熟した高性能言語

# 5. **NPeg との比較**
#    Nim + NPeg を使った場合の JSON パーサー例:
#
#    let jsonParser = peg "json":
#      json <- S * (object | array | string | number | boolean | null) * S
#      object <- "{" * (pair * ("," * pair)*)? * "}"
#      pair <- string * ":" * json
#      array <- "[" * (json * ("," * json)*)? * "]"
#      ...
#
#    - NPeg は PEG（Parsing Expression Grammar）ベース
#    - Reml の Core.Parse は LL(*) をベースに、Packrat や左再帰もサポート
#    - どちらも宣言的なパーサー記述が可能

# **結論**:
# Nim は手書きパーサーでも十分高速で、NPeg を使えば宣言的な記述も可能。
# Reml はパーサーコンビネーターが標準ライブラリとして統合され、
# 言語実装に最適化されたエラーハンドリングと型システムを持つ。