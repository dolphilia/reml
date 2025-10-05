import std/[strutils, options, sequtils]

# 正規表現エンジン：パース + 評価の両方を実装。
#
# 対応する正規表現構文（簡易版）：
# - リテラル: `abc`
# - 連結: `ab`
# - 選択: `a|b`
# - 繰り返し: `a*`, `a+`, `a?`, `a{2,5}`
# - グループ: `(abc)`
# - 文字クラス: `[a-z]`, `[^0-9]`, `\d`, `\w`, `\s`
# - アンカー: `^`, `$`
# - ドット: `.` (任意の1文字)

# 正規表現のAST
type
  PredefinedClass = enum
    pcDigit, pcWord, pcWhitespace
    pcNotDigit, pcNotWord, pcNotWhitespace

  CharSet = ref object
    case kind: CharSetKind
    of cskRange:
      rangeStart, rangeEnd: char
    of cskList:
      charList: seq[char]
    of cskPredefined:
      predefinedClass: PredefinedClass
    of cskNegated:
      negatedInner: CharSet
    of cskUnion:
      unionSets: seq[CharSet]

  CharSetKind = enum
    cskRange, cskList, cskPredefined, cskNegated, cskUnion

  RepeatKind = ref object
    case kind: RepeatKindType
    of rkZeroOrMore, rkOneOrMore, rkZeroOrOne:
      discard
    of rkExactly:
      exactCount: int
    of rkRange:
      minCount: int
      maxCount: Option[int]

  RepeatKindType = enum
    rkZeroOrMore, rkOneOrMore, rkZeroOrOne, rkExactly, rkRange

  AnchorKind = enum
    akStart, akEnd

  Regex = ref object
    case kind: RegexKind
    of rkLiteral:
      literal: string
    of rkCharClass:
      charClass: CharSet
    of rkDot:
      discard
    of rkConcat:
      concatTerms: seq[Regex]
    of rkAlternation:
      alternationTerms: seq[Regex]
    of rkRepeat:
      repeatInner: Regex
      repeatKind: RepeatKind
    of rkGroup:
      groupInner: Regex
    of rkAnchor:
      anchorKind: AnchorKind

  RegexKind = enum
    rkLiteral, rkCharClass, rkDot, rkConcat, rkAlternation
    rkRepeat, rkGroup, rkAnchor

  Parser[T] = proc(input: string): Option[(T, string)]

# パーサーコンビネーター
proc ok[T](value: T): Parser[T] =
  result = proc(input: string): Option[(T, string)] =
    some((value, input))

proc fail[T](message: string): Parser[T] =
  result = proc(input: string): Option[(T, string)] =
    none[(T, string)]()

proc bindP[A, B](p: Parser[A], f: proc(a: A): Parser[B]): Parser[B] =
  result = proc(input: string): Option[(B, string)] =
    let res = p(input)
    if res.isSome:
      let (value, rest) = res.get
      return f(value)(rest)
    else:
      return none[(B, string)]()

proc mapP[A, B](p: Parser[A], f: proc(a: A): B): Parser[B] =
  bindP(p, proc(a: A): Parser[B] = ok(f(a)))

proc choice[T](parsers: seq[Parser[T]]): Parser[T] =
  result = proc(input: string): Option[(T, string)] =
    for p in parsers:
      let res = p(input)
      if res.isSome:
        return res
    return none[(T, string)]()

proc manyP[T](p: Parser[T]): Parser[seq[T]] =
  result = proc(input: string): Option[(seq[T], string)] =
    var results: seq[T] = @[]
    var currentInput = input
    while true:
      let res = p(currentInput)
      if res.isSome:
        let (value, rest) = res.get
        results.add(value)
        currentInput = rest
      else:
        break
    return some((results, currentInput))

proc many1P[T](p: Parser[T]): Parser[seq[T]] =
  bindP(p, proc(first: T): Parser[seq[T]] =
    bindP(manyP(p), proc(rest: seq[T]): Parser[seq[T]] =
      ok(first & rest)
    )
  )

proc optionalP[T](p: Parser[T]): Parser[Option[T]] =
  result = proc(input: string): Option[(Option[T], string)] =
    let res = p(input)
    if res.isSome:
      let (value, rest) = res.get
      return some((some(value), rest))
    else:
      return some((none[T](), input))

proc charP(c: char): Parser[char] =
  result = proc(input: string): Option[(char, string)] =
    if input.len > 0 and input[0] == c:
      return some((c, input[1..^1]))
    else:
      return none[(char, string)]()

proc stringP(s: string): Parser[string] =
  result = proc(input: string): Option[(string, string)] =
    if input.startsWith(s):
      return some((s, input[s.len..^1]))
    else:
      return none[(string, string)]()

proc satisfyP(pred: proc(c: char): bool): Parser[char] =
  result = proc(input: string): Option[(char, string)] =
    if input.len > 0 and pred(input[0]):
      return some((input[0], input[1..^1]))
    else:
      return none[(char, string)]()

proc digitP(): Parser[char] =
  satisfyP(proc(c: char): bool = c in '0'..'9')

proc integerP(): Parser[int] =
  mapP(many1P(digitP()), proc(digits: seq[char]): int =
    var num = 0
    for d in digits:
      num = num * 10 + (ord(d) - ord('0'))
    return num
  )

proc sepBy1P[T, S](p: Parser[T], sep: Parser[S]): Parser[seq[T]] =
  bindP(p, proc(first: T): Parser[seq[T]] =
    bindP(manyP(bindP(sep, proc(s: S): Parser[T] = p)), proc(rest: seq[T]): Parser[seq[T]] =
      ok(first & rest)
    )
  )

# 正規表現パーサー（前方宣言）
proc regexExpr(): Parser[Regex]
proc atomP(): Parser[Regex]

proc parseRegex(input: string): Option[Regex] =
  let res = regexExpr()(input)
  if res.isSome:
    let (regex, rest) = res.get
    if rest.len == 0:
      return some(regex)
  return none[Regex]()

proc alternationExpr(): Parser[Regex] =
  mapP(sepBy1P(concatExpr(), stringP("|")), proc(alts: seq[Regex]): Regex =
    if alts.len == 1:
      return alts[0]
    else:
      return Regex(kind: rkAlternation, alternationTerms: alts)
  )

proc concatExpr(): Parser[Regex] =
  mapP(many1P(postfixTerm()), proc(terms: seq[Regex]): Regex =
    if terms.len == 1:
      return terms[0]
    else:
      return Regex(kind: rkConcat, concatTerms: terms)
  )

proc postfixTerm(): Parser[Regex] =
  bindP(atomP(), proc(base: Regex): Parser[Regex] =
    mapP(optionalP(repeatSuffix()), proc(repeatOpt: Option[RepeatKind]): Regex =
      if repeatOpt.isSome:
        return Regex(kind: rkRepeat, repeatInner: base, repeatKind: repeatOpt.get)
      else:
        return base
    )
  )

proc literalP(): Parser[Regex] =
  mapP(satisfyP(proc(c: char): bool = c notin {'(', ')', '[', ']', '{', '}', '*', '+', '?', '.', '|', '^', '$', '\\'}),
    proc(c: char): Regex =
      Regex(kind: rkLiteral, literal: $c)
  )

proc escapeCharP(): Parser[Regex] =
  bindP(stringP("\\"), proc(s: string): Parser[Regex] =
    mapP(satisfyP(proc(c: char): bool = c in {'n', 't', 'r', '\\', '(', ')', '[', ']', '{', '}', '*', '+', '?', '.', '|', '^', '$'}),
      proc(c: char): Regex =
        let lit = case c
          of 'n': "\n"
          of 't': "\t"
          of 'r': "\r"
          else: $c
        Regex(kind: rkLiteral, literal: lit)
    )
  )

proc predefinedClassP(): Parser[Regex] =
  bindP(stringP("\\"), proc(s: string): Parser[Regex] =
    mapP(choice(@[
      mapP(charP('d'), proc(c: char): PredefinedClass = pcDigit),
      mapP(charP('w'), proc(c: char): PredefinedClass = pcWord),
      mapP(charP('s'), proc(c: char): PredefinedClass = pcWhitespace),
      mapP(charP('D'), proc(c: char): PredefinedClass = pcNotDigit),
      mapP(charP('W'), proc(c: char): PredefinedClass = pcNotWord),
      mapP(charP('S'), proc(c: char): PredefinedClass = pcNotWhitespace)
    ]), proc(cls: PredefinedClass): Regex =
      Regex(kind: rkCharClass, charClass: CharSet(kind: cskPredefined, predefinedClass: cls))
    )
  )

proc charClassItemP(): Parser[CharSet] =
  choice(@[
    # 範囲
    bindP(satisfyP(proc(c: char): bool = c != ']' and c != '-'), proc(start: char): Parser[CharSet] =
      mapP(optionalP(bindP(stringP("-"), proc(s: string): Parser[char] =
        satisfyP(proc(c: char): bool = c != ']')
      )), proc(endOpt: Option[char]): CharSet =
        if endOpt.isSome:
          CharSet(kind: cskRange, rangeStart: start, rangeEnd: endOpt.get)
        else:
          CharSet(kind: cskList, charList: @[start])
      )
    ),
    # 単一文字
    mapP(satisfyP(proc(c: char): bool = c != ']'), proc(c: char): CharSet =
      CharSet(kind: cskList, charList: @[c])
    )
  ])

proc charClassP(): Parser[Regex] =
  bindP(stringP("["), proc(s1: string): Parser[Regex] =
    bindP(optionalP(stringP("^")), proc(negated: Option[string]): Parser[Regex] =
      bindP(many1P(charClassItemP()), proc(items: seq[CharSet]): Parser[Regex] =
        bindP(stringP("]"), proc(s2: string): Parser[Regex] =
          let unionSet = CharSet(kind: cskUnion, unionSets: items)
          let cs = if negated.isSome:
            CharSet(kind: cskNegated, negatedInner: unionSet)
          else:
            unionSet
          ok(Regex(kind: rkCharClass, charClass: cs))
        )
      )
    )
  )

proc repeatSuffix(): Parser[RepeatKind] =
  choice(@[
    mapP(stringP("*"), proc(s: string): RepeatKind = RepeatKind(kind: rkZeroOrMore)),
    mapP(stringP("+"), proc(s: string): RepeatKind = RepeatKind(kind: rkOneOrMore)),
    mapP(stringP("?"), proc(s: string): RepeatKind = RepeatKind(kind: rkZeroOrOne)),
    # {n,m} 形式
    bindP(stringP("{"), proc(s1: string): Parser[RepeatKind] =
      bindP(integerP(), proc(n: int): Parser[RepeatKind] =
        bindP(optionalP(bindP(stringP(","), proc(s2: string): Parser[Option[int]] =
          optionalP(integerP())
        )), proc(rangeOpt: Option[Option[int]]): Parser[RepeatKind] =
          bindP(stringP("}"), proc(s3: string): Parser[RepeatKind] =
            if rangeOpt.isNone:
              ok(RepeatKind(kind: rkExactly, exactCount: n))
            elif rangeOpt.get.isNone:
              ok(RepeatKind(kind: rkRange, minCount: n, maxCount: none[int]()))
            else:
              ok(RepeatKind(kind: rkRange, minCount: n, maxCount: some(rangeOpt.get.get)))
          )
        )
      )
    )
  ])

proc atomP(): Parser[Regex] =
  choice(@[
    # 括弧グループ
    bindP(stringP("("), proc(s1: string): Parser[Regex] =
      bindP(regexExpr(), proc(inner: Regex): Parser[Regex] =
        bindP(stringP(")"), proc(s2: string): Parser[Regex] =
          ok(Regex(kind: rkGroup, groupInner: inner))
        )
      )
    ),
    # アンカー
    mapP(stringP("^"), proc(s: string): Regex = Regex(kind: rkAnchor, anchorKind: akStart)),
    mapP(stringP("$"), proc(s: string): Regex = Regex(kind: rkAnchor, anchorKind: akEnd)),
    # ドット
    mapP(stringP("."), proc(s: string): Regex = Regex(kind: rkDot)),
    # 文字クラス
    charClassP(),
    # 定義済みクラス
    predefinedClassP(),
    # エスケープ文字
    escapeCharP(),
    # 通常のリテラル
    literalP()
  ])

proc regexExpr(): Parser[Regex] =
  alternationExpr()

# マッチングエンジン
proc matchRegex(regex: Regex, text: string): bool =
  matchFromPos(regex, text, 0)

proc matchFromPos(regex: Regex, text: string, pos: int): bool =
  case regex.kind
  of rkLiteral:
    if pos + regex.literal.len <= text.len:
      return text[pos..<pos + regex.literal.len] == regex.literal
    else:
      return false

  of rkCharClass:
    if pos < text.len:
      return charMatchesClass(text[pos], regex.charClass)
    else:
      return false

  of rkDot:
    return pos < text.len

  of rkConcat:
    var currentPos = pos
    for term in regex.concatTerms:
      if matchFromPos(term, text, currentPos):
        inc currentPos
      else:
        return false
    return true

  of rkAlternation:
    for alt in regex.alternationTerms:
      if matchFromPos(alt, text, pos):
        return true
    return false

  of rkRepeat:
    case regex.repeatKind.kind
    of rkZeroOrMore:
      return matchRepeatLoop(regex.repeatInner, text, pos, 0, 0, 999999)
    of rkOneOrMore:
      if matchFromPos(regex.repeatInner, text, pos):
        return matchRepeatLoop(regex.repeatInner, text, pos + 1, 1, 1, 999999)
      else:
        return false
    of rkZeroOrOne:
      return matchFromPos(regex.repeatInner, text, pos) or true
    of rkExactly:
      let n = regex.repeatKind.exactCount
      return matchRepeatLoop(regex.repeatInner, text, pos, 0, n, n)
    of rkRange:
      let minC = regex.repeatKind.minCount
      let maxC = if regex.repeatKind.maxCount.isSome: regex.repeatKind.maxCount.get else: 999999
      return matchRepeatLoop(regex.repeatInner, text, pos, 0, minC, maxC)

  of rkGroup:
    return matchFromPos(regex.groupInner, text, pos)

  of rkAnchor:
    case regex.anchorKind
    of akStart:
      return pos == 0
    of akEnd:
      return pos >= text.len

proc charMatchesClass(c: char, cs: CharSet): bool =
  case cs.kind
  of cskRange:
    return c >= cs.rangeStart and c <= cs.rangeEnd
  of cskList:
    return c in cs.charList
  of cskPredefined:
    case cs.predefinedClass
    of pcDigit:
      return c in '0'..'9'
    of pcWord:
      return c.isAlphaNumeric or c == '_'
    of pcWhitespace:
      return c in {' ', '\t', '\n', '\r'}
    of pcNotDigit:
      return c notin '0'..'9'
    of pcNotWord:
      return not (c.isAlphaNumeric or c == '_')
    of pcNotWhitespace:
      return c notin {' ', '\t', '\n', '\r'}
  of cskNegated:
    return not charMatchesClass(c, cs.negatedInner)
  of cskUnion:
    for set in cs.unionSets:
      if charMatchesClass(c, set):
        return true
    return false

proc matchRepeatLoop(inner: Regex, text: string, pos: int, count: int, minCount: int, maxCount: int): bool =
  if count == maxCount:
    return true
  elif count >= minCount and not matchFromPos(inner, text, pos):
    return true
  elif matchFromPos(inner, text, pos):
    return matchRepeatLoop(inner, text, pos + 1, count + 1, minCount, maxCount)
  elif count >= minCount:
    return true
  else:
    return false

# テスト例
proc testExamples() =
  let examples = @[
    ("a+", "aaa", true),
    ("a+", "b", false),
    ("[0-9]+", "123", true),
    ("[0-9]+", "abc", false),
    ("\\d{2,4}", "12", true),
    ("\\d{2,4}", "12345", true),
    ("(abc)+", "abcabc", true),
    ("a|b", "a", true),
    ("a|b", "b", true),
    ("a|b", "c", false),
    ("^hello$", "hello", true),
    ("^hello$", "hello world", false)
  ]

  for (pattern, text, expected) in examples:
    let regexOpt = parseRegex(pattern)
    if regexOpt.isSome:
      let regex = regexOpt.get
      let result = matchRegex(regex, text)
      let status = if result == expected: "✓" else: "✗"
      echo status & " パターン: '" & pattern & "', テキスト: '" & text & "', 期待: " & $expected & ", 結果: " & $result
    else:
      echo "✗ パーサーエラー: " & pattern

# 実行
when isMainModule:
  testExamples()