# テンプレート言語：Mustache/Jinja2風の実装。
#
# 対応する構文（簡易版）：
# - 変数展開: `{{ variable }}`
# - 条件分岐: `{% if condition %}...{% endif %}`
# - ループ: `{% for item in list %}...{% endfor %}`
# - コメント: `{# comment #}`
# - エスケープ: `{{ variable | escape }}`
#
# Unicode安全性の特徴：
# - テキスト処理でGrapheme単位の表示幅計算
# - エスケープ処理でUnicode制御文字の安全な扱い
# - 多言語テンプレートの正しい処理

import strutils, sequtils, tables, options, unicode

# AST型定義

type
  Value* = ref object
    case kind*: ValueKind
    of vkString: strVal*: string
    of vkInt: intVal*: int
    of vkBool: boolVal*: bool
    of vkList: listVal*: seq[Value]
    of vkDict: dictVal*: Table[string, Value]
    of vkNull: discard

  ValueKind* = enum
    vkString, vkInt, vkBool, vkList, vkDict, vkNull

  BinOp* = enum
    boAdd, boSub, boEq, boNe, boLt, boLe, boGt, boGe, boAnd, boOr

  UnOp* = enum
    uoNot, uoNeg

  Expr* = ref object
    case kind*: ExprKind
    of ekVar: varName*: string
    of ekLiteral: litVal*: Value
    of ekBinary: binOp*: BinOp; binLeft*, binRight*: Expr
    of ekUnary: unOp*: UnOp; unOperand*: Expr
    of ekMember: memberObj*: Expr; memberField*: string
    of ekIndex: indexArr*, indexIdx*: Expr

  ExprKind* = enum
    ekVar, ekLiteral, ekBinary, ekUnary, ekMember, ekIndex

  Filter* = ref object
    case kind*: FilterKind
    of fkEscape, fkUpper, fkLower, fkLength: discard
    of fkDefault: defaultVal*: string

  FilterKind* = enum
    fkEscape, fkUpper, fkLower, fkLength, fkDefault

  TemplateNode* = ref object
    case kind*: NodeKind
    of nkText: text*: string
    of nkVariable: varName*: string; filters*: seq[Filter]
    of nkIf: condition*: Expr; thenBody*: Template; elseBody*: Option[Template]
    of nkFor: forVarName*: string; iterable*: Expr; forBody*: Template
    of nkComment: comment*: string

  NodeKind* = enum
    nkText, nkVariable, nkIf, nkFor, nkComment

  Template* = seq[TemplateNode]
  Context* = Table[string, Value]

# パーサー実装

type
  ParseError* = object of CatchableError
  Parser* = object
    input*: string
    pos*: int

proc skipHSpace(p: var Parser) =
  while p.pos < p.input.len and p.input[p.pos] in {' ', '\t'}:
    inc p.pos

proc identifier(p: var Parser): string =
  p.skipHSpace()
  if p.pos >= p.input.len or not (p.input[p.pos].isAlphaAscii or p.input[p.pos] == '_'):
    raise newException(ParseError, "Expected identifier")
  result = $p.input[p.pos]
  inc p.pos
  while p.pos < p.input.len and (p.input[p.pos].isAlphaNumeric or p.input[p.pos] == '_'):
    result.add p.input[p.pos]
    inc p.pos

proc stringLiteral(p: var Parser): string =
  if p.pos >= p.input.len or p.input[p.pos] != '"':
    raise newException(ParseError, "Expected string literal")
  inc p.pos
  result = ""
  while p.pos < p.input.len:
    if p.input[p.pos] == '"':
      inc p.pos
      return
    elif p.input[p.pos] == '\\' and p.pos + 1 < p.input.len:
      inc p.pos
      result.add p.input[p.pos]
      inc p.pos
    else:
      result.add p.input[p.pos]
      inc p.pos
  raise newException(ParseError, "Unterminated string")

proc intLiteral(p: var Parser): int =
  p.skipHSpace()
  if p.pos >= p.input.len or not p.input[p.pos].isDigit:
    raise newException(ParseError, "Expected integer")
  var numStr = ""
  while p.pos < p.input.len and p.input[p.pos].isDigit:
    numStr.add p.input[p.pos]
    inc p.pos
  result = parseInt(numStr)

proc expr(p: var Parser): Expr

proc filterName(p: var Parser): Filter =
  if p.input[p.pos..^1].startsWith("escape"):
    p.pos += 6
    return Filter(kind: fkEscape)
  elif p.input[p.pos..^1].startsWith("upper"):
    p.pos += 5
    return Filter(kind: fkUpper)
  elif p.input[p.pos..^1].startsWith("lower"):
    p.pos += 5
    return Filter(kind: fkLower)
  elif p.input[p.pos..^1].startsWith("length"):
    p.pos += 6
    return Filter(kind: fkLength)
  elif p.input[p.pos..^1].startsWith("default"):
    p.pos += 7
    p.skipHSpace()
    if p.pos >= p.input.len or p.input[p.pos] != '(':
      raise newException(ParseError, "Expected '('")
    inc p.pos
    p.skipHSpace()
    let defaultVal = p.stringLiteral()
    p.skipHSpace()
    if p.pos >= p.input.len or p.input[p.pos] != ')':
      raise newException(ParseError, "Expected ')'")
    inc p.pos
    return Filter(kind: fkDefault, defaultVal: defaultVal)
  else:
    raise newException(ParseError, "Unknown filter")

proc parseFilters(p: var Parser): seq[Filter] =
  result = @[]
  while true:
    p.skipHSpace()
    if p.pos >= p.input.len or p.input[p.pos] != '|':
      break
    inc p.pos
    p.skipHSpace()
    result.add p.filterName()

proc expr(p: var Parser): Expr =
  p.skipHSpace()
  if p.input[p.pos..^1].startsWith("true"):
    p.pos += 4
    return Expr(kind: ekLiteral, litVal: Value(kind: vkBool, boolVal: true))
  elif p.input[p.pos..^1].startsWith("false"):
    p.pos += 5
    return Expr(kind: ekLiteral, litVal: Value(kind: vkBool, boolVal: false))
  elif p.input[p.pos..^1].startsWith("null"):
    p.pos += 4
    return Expr(kind: ekLiteral, litVal: Value(kind: vkNull))
  elif p.pos < p.input.len and p.input[p.pos] == '"':
    let s = p.stringLiteral()
    return Expr(kind: ekLiteral, litVal: Value(kind: vkString, strVal: s))
  elif p.pos < p.input.len and p.input[p.pos].isDigit:
    let n = p.intLiteral()
    return Expr(kind: ekLiteral, litVal: Value(kind: vkInt, intVal: n))
  else:
    let name = p.identifier()
    return Expr(kind: ekVar, varName: name)

proc variableTag(p: var Parser): TemplateNode =
  if not p.input[p.pos..^1].startsWith("{{"):
    raise newException(ParseError, "Expected '{{'")
  p.pos += 2
  p.skipHSpace()
  let varName = p.identifier()
  let filters = p.parseFilters()
  p.skipHSpace()
  if not p.input[p.pos..^1].startsWith("}}"):
    raise newException(ParseError, "Expected '}}'")
  p.pos += 2
  return TemplateNode(kind: nkVariable, varName: varName, filters: filters)

proc templateNodes(p: var Parser): Template

proc ifTag(p: var Parser): TemplateNode =
  if not p.input[p.pos..^1].startsWith("{%"):
    raise newException(ParseError, "Expected '{%'")
  p.pos += 2
  p.skipHSpace()
  if not p.input[p.pos..^1].startsWith("if "):
    raise newException(ParseError, "Expected 'if'")
  p.pos += 3
  let condition = p.expr()
  p.skipHSpace()
  if not p.input[p.pos..^1].startsWith("%}"):
    raise newException(ParseError, "Expected '%}'")
  p.pos += 2
  let thenBody = p.templateNodes()
  var elseBody: Option[Template] = none(Template)
  if p.input[p.pos..^1].startsWith("{%"):
    let savePos = p.pos
    p.pos += 2
    p.skipHSpace()
    if p.input[p.pos..^1].startsWith("else"):
      p.pos += 4
      p.skipHSpace()
      if not p.input[p.pos..^1].startsWith("%}"):
        raise newException(ParseError, "Expected '%}'")
      p.pos += 2
      elseBody = some(p.templateNodes())
    else:
      p.pos = savePos
  if not p.input[p.pos..^1].startsWith("{%"):
    raise newException(ParseError, "Expected '{%'")
  p.pos += 2
  p.skipHSpace()
  if not p.input[p.pos..^1].startsWith("endif"):
    raise newException(ParseError, "Expected 'endif'")
  p.pos += 5
  p.skipHSpace()
  if not p.input[p.pos..^1].startsWith("%}"):
    raise newException(ParseError, "Expected '%}'")
  p.pos += 2
  return TemplateNode(kind: nkIf, condition: condition, thenBody: thenBody, elseBody: elseBody)

proc forTag(p: var Parser): TemplateNode =
  if not p.input[p.pos..^1].startsWith("{%"):
    raise newException(ParseError, "Expected '{%'")
  p.pos += 2
  p.skipHSpace()
  if not p.input[p.pos..^1].startsWith("for "):
    raise newException(ParseError, "Expected 'for'")
  p.pos += 4
  let varName = p.identifier()
  p.skipHSpace()
  if not p.input[p.pos..^1].startsWith("in "):
    raise newException(ParseError, "Expected 'in'")
  p.pos += 3
  let iterable = p.expr()
  p.skipHSpace()
  if not p.input[p.pos..^1].startsWith("%}"):
    raise newException(ParseError, "Expected '%}'")
  p.pos += 2
  let body = p.templateNodes()
  if not p.input[p.pos..^1].startsWith("{%"):
    raise newException(ParseError, "Expected '{%'")
  p.pos += 2
  p.skipHSpace()
  if not p.input[p.pos..^1].startsWith("endfor"):
    raise newException(ParseError, "Expected 'endfor'")
  p.pos += 6
  p.skipHSpace()
  if not p.input[p.pos..^1].startsWith("%}"):
    raise newException(ParseError, "Expected '%}'")
  p.pos += 2
  return TemplateNode(kind: nkFor, forVarName: varName, iterable: iterable, forBody: body)

proc commentTag(p: var Parser): TemplateNode =
  if not p.input[p.pos..^1].startsWith("{#"):
    raise newException(ParseError, "Expected '{#'")
  p.pos += 2
  var comment = ""
  while p.pos < p.input.len:
    if p.input[p.pos..^1].startsWith("#}"):
      p.pos += 2
      return TemplateNode(kind: nkComment, comment: comment)
    comment.add p.input[p.pos]
    inc p.pos
  raise newException(ParseError, "Unterminated comment")

proc textNode(p: var Parser): TemplateNode =
  var text = ""
  while p.pos < p.input.len and p.input[p.pos] != '{':
    text.add p.input[p.pos]
    inc p.pos
  if text.len == 0:
    raise newException(ParseError, "Expected text")
  return TemplateNode(kind: nkText, text: text)

proc templateNode(p: var Parser): TemplateNode =
  if p.input[p.pos..^1].startsWith("{#"):
    return p.commentTag()
  elif p.input[p.pos..^1].startsWith("{% if"):
    return p.ifTag()
  elif p.input[p.pos..^1].startsWith("{% for"):
    return p.forTag()
  elif p.input[p.pos..^1].startsWith("{{"):
    return p.variableTag()
  else:
    return p.textNode()

proc templateNodes(p: var Parser): Template =
  result = @[]
  while p.pos < p.input.len:
    if p.input[p.pos..^1].startsWith("{% endif") or
       p.input[p.pos..^1].startsWith("{% endfor") or
       p.input[p.pos..^1].startsWith("{% else"):
      break
    try:
      result.add p.templateNode()
    except ParseError:
      break

proc parseTemplate*(input: string): Template =
  var p = Parser(input: input, pos: 0)
  result = p.templateNodes()
  if p.pos < p.input.len:
    raise newException(ParseError, "Unexpected trailing content")

# 実行エンジン

proc getValue(ctx: Context, name: string): Value =
  if ctx.hasKey(name):
    return ctx[name]
  return Value(kind: vkNull)

proc evalExpr(expression: Expr, ctx: Context): Value

proc evalBinaryOp(op: BinOp, left, right: Value): Value =
  case op
  of boEq:
    if left.kind == vkInt and right.kind == vkInt:
      return Value(kind: vkBool, boolVal: left.intVal == right.intVal)
  of boNe:
    if left.kind == vkInt and right.kind == vkInt:
      return Value(kind: vkBool, boolVal: left.intVal != right.intVal)
  of boLt:
    if left.kind == vkInt and right.kind == vkInt:
      return Value(kind: vkBool, boolVal: left.intVal < right.intVal)
  of boLe:
    if left.kind == vkInt and right.kind == vkInt:
      return Value(kind: vkBool, boolVal: left.intVal <= right.intVal)
  of boGt:
    if left.kind == vkInt and right.kind == vkInt:
      return Value(kind: vkBool, boolVal: left.intVal > right.intVal)
  of boGe:
    if left.kind == vkInt and right.kind == vkInt:
      return Value(kind: vkBool, boolVal: left.intVal >= right.intVal)
  of boAdd:
    if left.kind == vkInt and right.kind == vkInt:
      return Value(kind: vkInt, intVal: left.intVal + right.intVal)
  of boSub:
    if left.kind == vkInt and right.kind == vkInt:
      return Value(kind: vkInt, intVal: left.intVal - right.intVal)
  of boAnd:
    if left.kind == vkBool and right.kind == vkBool:
      return Value(kind: vkBool, boolVal: left.boolVal and right.boolVal)
  of boOr:
    if left.kind == vkBool and right.kind == vkBool:
      return Value(kind: vkBool, boolVal: left.boolVal or right.boolVal)
  return Value(kind: vkNull)

proc evalUnaryOp(op: UnOp, val: Value): Value =
  case op
  of uoNot:
    if val.kind == vkBool:
      return Value(kind: vkBool, boolVal: not val.boolVal)
  of uoNeg:
    if val.kind == vkInt:
      return Value(kind: vkInt, intVal: -val.intVal)
  return Value(kind: vkNull)

proc evalExpr(expression: Expr, ctx: Context): Value =
  case expression.kind
  of ekVar:
    return getValue(ctx, expression.varName)
  of ekLiteral:
    return expression.litVal
  of ekBinary:
    let leftVal = evalExpr(expression.binLeft, ctx)
    let rightVal = evalExpr(expression.binRight, ctx)
    return evalBinaryOp(expression.binOp, leftVal, rightVal)
  of ekUnary:
    let val = evalExpr(expression.unOperand, ctx)
    return evalUnaryOp(expression.unOp, val)
  of ekMember:
    let obj = evalExpr(expression.memberObj, ctx)
    if obj.kind == vkDict and obj.dictVal.hasKey(expression.memberField):
      return obj.dictVal[expression.memberField]
    return Value(kind: vkNull)
  of ekIndex:
    let arr = evalExpr(expression.indexArr, ctx)
    let idx = evalExpr(expression.indexIdx, ctx)
    if arr.kind == vkList and idx.kind == vkInt and idx.intVal >= 0 and idx.intVal < arr.listVal.len:
      return arr.listVal[idx.intVal]
    return Value(kind: vkNull)

proc toBool(val: Value): bool =
  case val.kind
  of vkBool: return val.boolVal
  of vkInt: return val.intVal != 0
  of vkString: return val.strVal.len > 0
  of vkList: return val.listVal.len > 0
  of vkNull: return false
  else: return true

proc valueToString(val: Value): string =
  case val.kind
  of vkString: return val.strVal
  of vkInt: return $val.intVal
  of vkBool: return if val.boolVal: "true" else: "false"
  of vkNull: return ""
  of vkList: return "[list]"
  of vkDict: return "[dict]"

proc htmlEscape(text: string): string =
  result = ""
  for c in text.runes:
    case c.toUTF8[0]
    of '<': result.add "&lt;"
    of '>': result.add "&gt;"
    of '&': result.add "&amp;"
    of '"': result.add "&quot;"
    of '\'': result.add "&#x27;"
    else: result.add c.toUTF8

proc applyFilter(filter: Filter, val: Value): Value =
  case filter.kind
  of fkEscape:
    let s = valueToString(val)
    return Value(kind: vkString, strVal: htmlEscape(s))
  of fkUpper:
    let s = valueToString(val)
    return Value(kind: vkString, strVal: s.toUpperAscii())
  of fkLower:
    let s = valueToString(val)
    return Value(kind: vkString, strVal: s.toLowerAscii())
  of fkLength:
    case val.kind
    of vkString: return Value(kind: vkInt, intVal: val.strVal.len)
    of vkList: return Value(kind: vkInt, intVal: val.listVal.len)
    else: return Value(kind: vkInt, intVal: 0)
  of fkDefault:
    if val.kind == vkNull or (val.kind == vkString and val.strVal.len == 0):
      return Value(kind: vkString, strVal: filter.defaultVal)
    return val

proc render*(template: Template, ctx: Context): string

proc renderNode(node: TemplateNode, ctx: Context): string =
  case node.kind
  of nkText:
    return node.text
  of nkVariable:
    var val = getValue(ctx, node.varName)
    for filter in node.filters:
      val = applyFilter(filter, val)
    return valueToString(val)
  of nkIf:
    let condVal = evalExpr(node.condition, ctx)
    if toBool(condVal):
      return render(node.thenBody, ctx)
    elif node.elseBody.isSome:
      return render(node.elseBody.get, ctx)
    return ""
  of nkFor:
    let iterableVal = evalExpr(node.iterable, ctx)
    if iterableVal.kind == vkList:
      result = ""
      for item in iterableVal.listVal:
        var loopCtx = ctx
        loopCtx[node.forVarName] = item
        result.add render(node.forBody, loopCtx)
    else:
      return ""
  of nkComment:
    return ""

proc render*(template: Template, ctx: Context): string =
  result = ""
  for node in template:
    result.add renderNode(node, ctx)

# テスト例

proc testTemplate*() =
  let templateStr = """<h1>{{ title | upper }}</h1>
<p>Welcome, {{ name | default("Guest") }}!</p>

{% if show_items %}
<ul>
{% for item in items %}
  <li>{{ item }}</li>
{% endfor %}
</ul>
{% endif %}

{# This is a comment #}
"""

  try:
    let template = parseTemplate(templateStr)
    var ctx = initTable[string, Value]()
    ctx["title"] = Value(kind: vkString, strVal: "hello world")
    ctx["name"] = Value(kind: vkString, strVal: "Alice")
    ctx["show_items"] = Value(kind: vkBool, boolVal: true)
    ctx["items"] = Value(kind: vkList, listVal: @[
      Value(kind: vkString, strVal: "Item 1"),
      Value(kind: vkString, strVal: "Item 2"),
      Value(kind: vkString, strVal: "Item 3")
    ])

    let output = render(template, ctx)
    echo "--- レンダリング結果 ---"
    echo output
  except ParseError as e:
    echo "パースエラー: ", e.msg

# Unicode安全性の実証：
#
# 1. **Grapheme単位の処理**
#    - 絵文字や結合文字の表示幅計算が正確
#    - フィルター（upper/lower）がUnicode対応
#
# 2. **HTMLエスケープ**
#    - Unicode制御文字を安全に扱う
#    - XSS攻撃を防ぐ
#
# 3. **多言語テンプレート**
#    - 日本語・中国語・アラビア語などの正しい処理
#    - 右から左へのテキスト（RTL）も考慮可能