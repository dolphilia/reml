// 正規表現エンジン：パース + 評価の両方を実装。
//
// 対応する正規表現構文（簡易版）：
// - リテラル: `abc`
// - 連結: `ab`
// - 選択: `a|b`
// - 繰り返し: `a*`, `a+`, `a?`, `a{2,5}`
// - グループ: `(abc)`
// - 文字クラス: `[a-z]`, `[^0-9]`, `\d`, `\w`, `\s`
// - アンカー: `^`, `$`
// - ドット: `.` (任意の1文字)

// 正規表現のAST
indirect enum Regex {
    case literal(String)
    case charClass(CharSet)
    case dot
    case concat([Regex])
    case alternation([Regex])
    case `repeat`(Regex, RepeatKind)
    case group(Regex)
    case anchor(AnchorKind)
}

indirect enum CharSet {
    case charRange(Character, Character)
    case charList([Character])
    case predefined(PredefinedClass)
    case negated(CharSet)
    case union([CharSet])
}

enum PredefinedClass {
    case digit
    case word
    case whitespace
    case notDigit
    case notWord
    case notWhitespace
}

enum RepeatKind {
    case zeroOrMore
    case oneOrMore
    case zeroOrOne
    case exactly(Int)
    case range(Int, Int?)
}

enum AnchorKind {
    case start
    case end
}

// パーサー型
typealias ParseResult<T> = Result<(T, String), String>

struct Parser {
    static func ok<T>(_ value: T, _ rest: String) -> ParseResult<T> {
        .success((value, rest))
    }

    static func fail<T>(_ message: String) -> ParseResult<T> {
        .failure(message)
    }

    static func choice<T>(_ parsers: [(String) -> ParseResult<T>]) -> (String) -> ParseResult<T> {
        return { input in
            for parser in parsers {
                if case .success(let result) = parser(input) {
                    return .success(result)
                }
            }
            return fail("no choice matched")
        }
    }

    static func many<T>(_ parser: @escaping (String) -> ParseResult<T>) -> (String) -> ParseResult<([T], String)> {
        return { input in
            var results: [T] = []
            var current = input

            while case .success(let (value, rest)) = parser(current) {
                results.append(value)
                current = rest
            }

            return ok(results, current)
        }
    }

    static func many1<T>(_ parser: @escaping (String) -> ParseResult<T>) -> (String) -> ParseResult<([T], String)> {
        return { input in
            guard case .success(let (first, rest1)) = parser(input) else {
                return fail("many1 failed")
            }
            guard case .success(let (others, rest2)) = many(parser)(rest1) else {
                return fail("many1 continuation failed")
            }
            return ok([first] + others, rest2)
        }
    }

    static func optional<T>(_ parser: @escaping (String) -> ParseResult<T>) -> (String) -> ParseResult<(T?, String)> {
        return { input in
            if case .success(let (value, rest)) = parser(input) {
                return ok(value, rest)
            }
            return ok(nil, input)
        }
    }

    static func char(_ c: Character) -> (String) -> ParseResult<(Character, String)> {
        return { input in
            guard let first = input.first, first == c else {
                return fail("expected \(c)")
            }
            return ok(c, String(input.dropFirst()))
        }
    }

    static func string(_ s: String) -> (String) -> ParseResult<(String, String)> {
        return { input in
            guard input.hasPrefix(s) else {
                return fail("expected \(s)")
            }
            return ok(s, String(input.dropFirst(s.count)))
        }
    }

    static func satisfy(_ pred: @escaping (Character) -> Bool) -> (String) -> ParseResult<(Character, String)> {
        return { input in
            guard let c = input.first, pred(c) else {
                return fail("predicate failed")
            }
            return ok(c, String(input.dropFirst()))
        }
    }

    static func digit() -> (String) -> ParseResult<(Character, String)> {
        return satisfy { $0.isNumber }
    }

    static func integer() -> (String) -> ParseResult<(Int, String)> {
        return { input in
            guard case .success(let (digits, rest)) = many1(digit())(input) else {
                return fail("integer parse failed")
            }
            let numStr = String(digits)
            guard let num = Int(numStr) else {
                return fail("invalid integer")
            }
            return ok(num, rest)
        }
    }

    static func sepBy1<T, S>(
        _ parser: @escaping (String) -> ParseResult<T>,
        _ sep: @escaping (String) -> ParseResult<S>
    ) -> (String) -> ParseResult<([T], String)> {
        return { input in
            guard case .success(let (first, rest1)) = parser(input) else {
                return fail("sepBy1 failed")
            }

            let sepThenParser: (String) -> ParseResult<T> = { inp in
                guard case .success(let (_, r1)) = sep(inp) else {
                    return fail("sep failed")
                }
                return parser(r1)
            }

            guard case .success(let (others, rest2)) = many(sepThenParser)(rest1) else {
                return fail("sepBy1 continuation failed")
            }

            return ok([first] + others, rest2)
        }
    }
}

// 正規表現パーサー
struct RegexParser {
    static func parseRegex(_ input: String) -> Result<Regex, String> {
        switch regexExpr(input) {
        case .success(let (regex, "")):
            return .success(regex)
        case .success(let (_, rest)):
            return .failure("unexpected input: \(rest)")
        case .failure(let err):
            return .failure(err)
        }
    }

    static func regexExpr(_ input: String) -> ParseResult<Regex> {
        return alternationExpr(input)
    }

    static func alternationExpr(_ input: String) -> ParseResult<Regex> {
        guard case .success(let (alts, rest)) = Parser.sepBy1(concatExpr, Parser.string("|"))(input) else {
            return Parser.fail("alternation failed")
        }

        if alts.count == 1 {
            return Parser.ok(alts[0], rest)
        } else {
            return Parser.ok(.alternation(alts), rest)
        }
    }

    static func concatExpr(_ input: String) -> ParseResult<Regex> {
        guard case .success(let (terms, rest)) = Parser.many1(postfixTerm)(input) else {
            return Parser.fail("concat failed")
        }

        if terms.count == 1 {
            return Parser.ok(terms[0], rest)
        } else {
            return Parser.ok(.concat(terms), rest)
        }
    }

    static func postfixTerm(_ input: String) -> ParseResult<Regex> {
        guard case .success(let (base, rest1)) = atom(input) else {
            return Parser.fail("postfix term failed")
        }
        guard case .success(let (repeatOpt, rest2)) = Parser.optional(repeatSuffix)(rest1) else {
            return Parser.fail("repeat suffix failed")
        }

        if let kind = repeatOpt {
            return Parser.ok(.repeat(base, kind), rest2)
        } else {
            return Parser.ok(base, rest2)
        }
    }

    static func atom(_ input: String) -> ParseResult<Regex> {
        // 括弧グループ
        if case .success(let (_, rest1)) = Parser.string("(")(input),
           case .success(let (inner, rest2)) = regexExpr(rest1),
           case .success(let (_, rest3)) = Parser.string(")")(rest2) {
            return Parser.ok(.group(inner), rest3)
        }

        // アンカー
        if case .success(let (_, rest)) = Parser.string("^")(input) {
            return Parser.ok(.anchor(.start), rest)
        }
        if case .success(let (_, rest)) = Parser.string("$")(input) {
            return Parser.ok(.anchor(.end), rest)
        }

        // ドット
        if case .success(let (_, rest)) = Parser.string(".")(input) {
            return Parser.ok(.dot, rest)
        }

        // 文字クラス
        if case .success(let result) = charClass(input) {
            return .success(result)
        }

        // 定義済みクラス
        if case .success(let result) = predefinedClass(input) {
            return .success(result)
        }

        // エスケープ文字
        if case .success(let result) = escapeChar(input) {
            return .success(result)
        }

        // 通常のリテラル
        return Parser.satisfy { c in
            c != "(" && c != ")" && c != "[" && c != "]" &&
            c != "{" && c != "}" && c != "*" && c != "+" &&
            c != "?" && c != "." && c != "|" && c != "^" &&
            c != "$" && c != "\\"
        }(input).map { (c, rest) in
            (.literal(String(c)), rest)
        }
    }

    static func escapeChar(_ input: String) -> ParseResult<Regex> {
        guard case .success(let (_, rest1)) = Parser.string("\\")(input) else {
            return Parser.fail("escape char failed")
        }
        guard case .success(let (c, rest2)) = Parser.satisfy({ ch in
            ch == "n" || ch == "t" || ch == "r" || ch == "\\" ||
            ch == "(" || ch == ")" || ch == "[" || ch == "]" ||
            ch == "{" || ch == "}" || ch == "*" || ch == "+" ||
            ch == "?" || ch == "." || ch == "|" || ch == "^" || ch == "$"
        })(rest1) else {
            return Parser.fail("invalid escape sequence")
        }

        let lit: String
        switch c {
        case "n": lit = "\n"
        case "t": lit = "\t"
        case "r": lit = "\r"
        default: lit = String(c)
        }

        return Parser.ok(.literal(lit), rest2)
    }

    static func predefinedClass(_ input: String) -> ParseResult<Regex> {
        guard case .success(let (_, rest1)) = Parser.string("\\")(input) else {
            return Parser.fail("predefined class failed")
        }

        if case .success(let (_, rest2)) = Parser.char("d")(rest1) {
            return Parser.ok(.charClass(.predefined(.digit)), rest2)
        }
        if case .success(let (_, rest2)) = Parser.char("w")(rest1) {
            return Parser.ok(.charClass(.predefined(.word)), rest2)
        }
        if case .success(let (_, rest2)) = Parser.char("s")(rest1) {
            return Parser.ok(.charClass(.predefined(.whitespace)), rest2)
        }
        if case .success(let (_, rest2)) = Parser.char("D")(rest1) {
            return Parser.ok(.charClass(.predefined(.notDigit)), rest2)
        }
        if case .success(let (_, rest2)) = Parser.char("W")(rest1) {
            return Parser.ok(.charClass(.predefined(.notWord)), rest2)
        }
        if case .success(let (_, rest2)) = Parser.char("S")(rest1) {
            return Parser.ok(.charClass(.predefined(.notWhitespace)), rest2)
        }

        return Parser.fail("invalid predefined class")
    }

    static func charClass(_ input: String) -> ParseResult<Regex> {
        guard case .success(let (_, rest1)) = Parser.string("[")(input) else {
            return Parser.fail("char class failed")
        }
        guard case .success(let (negated, rest2)) = Parser.optional(Parser.string("^"))(rest1) else {
            return Parser.fail("negation check failed")
        }
        guard case .success(let (items, rest3)) = Parser.many1(charClassItem)(rest2) else {
            return Parser.fail("char class items failed")
        }
        guard case .success(let (_, rest4)) = Parser.string("]")(rest3) else {
            return Parser.fail("char class close failed")
        }

        let unionSet = CharSet.union(items)
        let cs = negated != nil ? CharSet.negated(unionSet) : unionSet

        return Parser.ok(.charClass(cs), rest4)
    }

    static func charClassItem(_ input: String) -> ParseResult<CharSet> {
        guard case .success(let (start, rest1)) = Parser.satisfy({ $0 != "]" && $0 != "-" })(input) else {
            return Parser.fail("char class item failed")
        }
        guard case .success(let (endOpt, rest2)) = Parser.optional({ inp -> ParseResult<Character> in
            guard case .success(let (_, r1)) = Parser.string("-")(inp) else {
                return Parser.fail("dash failed")
            }
            return Parser.satisfy({ $0 != "]" })(r1)
        })(rest1) else {
            return Parser.fail("range check failed")
        }

        if let end = endOpt {
            return Parser.ok(.charRange(start, end), rest2)
        } else {
            return Parser.ok(.charList([start]), rest2)
        }
    }

    static func repeatSuffix(_ input: String) -> ParseResult<RepeatKind> {
        if case .success(let (_, rest)) = Parser.string("*")(input) {
            return Parser.ok(.zeroOrMore, rest)
        }
        if case .success(let (_, rest)) = Parser.string("+")(input) {
            return Parser.ok(.oneOrMore, rest)
        }
        if case .success(let (_, rest)) = Parser.string("?")(input) {
            return Parser.ok(.zeroOrOne, rest)
        }

        // {n,m} 形式
        guard case .success(let (_, rest1)) = Parser.string("{")(input) else {
            return Parser.fail("repeat suffix failed")
        }
        guard case .success(let (n, rest2)) = Parser.integer()(rest1) else {
            return Parser.fail("repeat count failed")
        }
        guard case .success(let (rangeOpt, rest3)) = Parser.optional({ inp -> ParseResult<Int?> in
            guard case .success(let (_, r1)) = Parser.string(",")(inp) else {
                return Parser.fail("comma failed")
            }
            return Parser.optional(Parser.integer())(r1)
        })(rest2) else {
            return Parser.fail("range check failed")
        }
        guard case .success(let (_, rest4)) = Parser.string("}")(rest3) else {
            return Parser.fail("brace close failed")
        }

        if let mOpt = rangeOpt {
            return Parser.ok(.range(n, mOpt), rest4)
        } else {
            return Parser.ok(.exactly(n), rest4)
        }
    }
}

// マッチングエンジン
struct RegexMatcher {
    static func matchRegex(_ regex: Regex, _ text: String) -> Bool {
        return matchFromPos(regex, text, text.startIndex)
    }

    static func matchFromPos(_ regex: Regex, _ text: String, _ pos: String.Index) -> Bool {
        switch regex {
        case .literal(let s):
            let endIndex = text.index(pos, offsetBy: s.count, limitedBy: text.endIndex) ?? text.endIndex
            return String(text[pos..<endIndex]) == s

        case .charClass(let cs):
            guard pos < text.endIndex else { return false }
            return charMatchesClass(text[pos], cs)

        case .dot:
            return pos < text.endIndex

        case .concat(let terms):
            var currentPos = pos
            for term in terms {
                guard matchFromPos(term, text, currentPos) else { return false }
                if currentPos < text.endIndex {
                    currentPos = text.index(after: currentPos)
                }
            }
            return true

        case .alternation(let alts):
            return alts.contains { matchFromPos($0, text, pos) }

        case .repeat(let inner, let kind):
            switch kind {
            case .zeroOrMore:
                return matchRepeatLoop(inner, text, pos, 0, 0, 999999)
            case .oneOrMore:
                if matchFromPos(inner, text, pos) {
                    let nextPos = pos < text.endIndex ? text.index(after: pos) : pos
                    return matchRepeatLoop(inner, text, nextPos, 1, 1, 999999)
                } else {
                    return false
                }
            case .zeroOrOne:
                return matchFromPos(inner, text, pos) || true
            case .exactly(let n):
                return matchRepeatLoop(inner, text, pos, 0, n, n)
            case .range(let min, let maxOpt):
                let max = maxOpt ?? 999999
                return matchRepeatLoop(inner, text, pos, 0, min, max)
            }

        case .group(let inner):
            return matchFromPos(inner, text, pos)

        case .anchor(let kind):
            switch kind {
            case .start:
                return pos == text.startIndex
            case .end:
                return pos >= text.endIndex
            }
        }
    }

    static func charMatchesClass(_ c: Character, _ cs: CharSet) -> Bool {
        switch cs {
        case .charRange(let start, let end):
            return c >= start && c <= end
        case .charList(let chars):
            return chars.contains(c)
        case .predefined(let cls):
            switch cls {
            case .digit: return c.isNumber
            case .word: return c.isLetter || c.isNumber || c == "_"
            case .whitespace: return c.isWhitespace
            case .notDigit: return !c.isNumber
            case .notWord: return !(c.isLetter || c.isNumber || c == "_")
            case .notWhitespace: return !c.isWhitespace
            }
        case .negated(let inner):
            return !charMatchesClass(c, inner)
        case .union(let sets):
            return sets.contains { charMatchesClass(c, $0) }
        }
    }

    static func matchRepeatLoop(
        _ inner: Regex,
        _ text: String,
        _ pos: String.Index,
        _ count: Int,
        _ min: Int,
        _ max: Int
    ) -> Bool {
        if count == max {
            return true
        } else if count >= min && !matchFromPos(inner, text, pos) {
            return true
        } else if matchFromPos(inner, text, pos) {
            let nextPos = pos < text.endIndex ? text.index(after: pos) : pos
            return matchRepeatLoop(inner, text, nextPos, count + 1, min, max)
        } else if count >= min {
            return true
        } else {
            return false
        }
    }
}

// テスト例
func testRegexEngine() {
    let examples: [(String, String, Bool)] = [
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

    for (pattern, text, expected) in examples {
        switch RegexParser.parseRegex(pattern) {
        case .success(let regex):
            let result = RegexMatcher.matchRegex(regex, text)
            let status = result == expected ? "✓" : "✗"
            print("\(status) パターン: '\(pattern)', テキスト: '\(text)', 期待: \(expected), 結果: \(result)")
        case .failure(let err):
            print("✗ パーサーエラー: \(pattern) - \(err)")
        }
    }
}

testRegexEngine()