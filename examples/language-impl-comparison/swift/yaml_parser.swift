import Foundation

/// YAML風パーサー：インデント管理が重要な題材。
///
/// 対応する構文（簡易版）：
/// - スカラー値: 文字列、数値、真偽値、null
/// - リスト: `- item1`
/// - マップ: `key: value`
/// - ネストしたインデント構造
///
/// インデント処理の特徴：
/// - SwiftのResultとOptionalを活用したパーサー実装
/// - エラー回復機能でインデントミスを報告しつつ継続

// YAML値の表現。
indirect enum YamlValue: Equatable {
    case scalar(String)
    case list([YamlValue])
    case map([String: YamlValue])
    case null
}

typealias Document = YamlValue

struct ParseError: Error, CustomStringConvertible {
    let message: String

    var description: String {
        return message
    }
}

class Parser {
    let input: String
    var pos: String.Index

    init(input: String) {
        self.input = input
        self.pos = input.startIndex
    }

    func peek() -> Character? {
        guard pos < input.endIndex else { return nil }
        return input[pos]
    }

    func advance() {
        if pos < input.endIndex {
            pos = input.index(after: pos)
        }
    }

    func isEof() -> Bool {
        return pos >= input.endIndex
    }

    func expect(_ expected: Character) throws {
        guard peek() == expected else {
            throw ParseError(message: "期待された文字 '\(expected)' が見つかりません")
        }
        advance()
    }

    func expectString(_ expected: String) throws {
        for c in expected {
            try expect(c)
        }
    }

    /// 水平空白のみをスキップ（改行は含まない）。
    func hspace() {
        while let c = peek(), c == " " || c == "\t" {
            advance()
        }
    }

    /// 改行をスキップ。
    func newline() {
        if peek() == "\n" {
            advance()
        } else if peek() == "\r" {
            advance()
            if peek() == "\n" {
                advance()
            }
        }
    }

    /// コメントのスキップ（`#` から行末まで）。
    func comment() {
        if peek() == "#" {
            advance()
            while let c = peek(), c != "\n" {
                advance()
            }
        }
    }

    /// 空行またはコメント行をスキップ。
    func blankOrComment() {
        hspace()
        comment()
        newline()
    }

    /// 特定のインデントレベルを期待する。
    func expectIndent(_ level: Int) throws {
        var spaces = 0
        while peek() == " " {
            spaces += 1
            advance()
        }

        guard spaces == level else {
            throw ParseError(message: "インデント不一致: 期待 \(level), 実際 \(spaces)")
        }
    }

    /// 現在よりも深いインデントを検出。
    func deeperIndent(_ current: Int) throws -> Int {
        var spaces = 0
        while peek() == " " {
            spaces += 1
            advance()
        }

        guard spaces > current else {
            throw ParseError(message: "深いインデントが期待されます: 現在 \(current), 実際 \(spaces)")
        }

        return spaces
    }

    /// スカラー値のパース。
    func scalarValue() throws -> YamlValue {
        let remaining = String(input[pos...])

        // null
        if remaining.hasPrefix("null") {
            try expectString("null")
            return .null
        }

        if peek() == "~" {
            advance()
            return .null
        }

        // 真偽値
        if remaining.hasPrefix("true") {
            try expectString("true")
            return .scalar("true")
        }

        if remaining.hasPrefix("false") {
            try expectString("false")
            return .scalar("false")
        }

        // 数値（簡易実装）
        var numStr = ""
        while let c = peek(), c.isNumber {
            numStr.append(c)
            advance()
        }

        if !numStr.isEmpty {
            return .scalar(numStr)
        }

        // 文字列（引用符付き）
        if peek() == "\"" {
            advance()
            var str = ""
            while let c = peek(), c != "\"" {
                str.append(c)
                advance()
            }
            try expect("\"")
            return .scalar(str)
        }

        // 文字列（引用符なし：行末または `:` まで）
        var str = ""
        while let c = peek(), c != "\n" && c != ":" && c != "#" {
            str.append(c)
            advance()
        }

        return .scalar(str.trimmingCharacters(in: .whitespaces))
    }

    /// リスト項目のパース（`- value` 形式）。
    func parseListItem(_ indent: Int) throws -> YamlValue {
        try expectIndent(indent)
        try expect("-")
        hspace()
        return try parseValue(indent + 2)
    }

    /// リスト全体のパース。
    func parseList(_ indent: Int) throws -> YamlValue {
        var items: [YamlValue] = []

        while true {
            let savedPos = pos
            do {
                let item = try parseListItem(indent)
                items.append(item)

                if peek() == "\n" {
                    newline()
                } else {
                    break
                }
            } catch {
                pos = savedPos
                break
            }
        }

        guard !items.isEmpty else {
            throw ParseError(message: "リストが空です")
        }

        return .list(items)
    }

    /// マップのキーバリューペアのパース（`key: value` 形式）。
    func parseMapEntry(_ indent: Int) throws -> (String, YamlValue) {
        try expectIndent(indent)

        var key = ""
        while let c = peek(), c != ":" && c != "\n" {
            key.append(c)
            advance()
        }

        let keyStr = key.trimmingCharacters(in: .whitespaces)
        try expect(":")
        hspace()

        // 同じ行に値があるか、次の行にネストされているか
        let value: YamlValue
        if peek() == "\n" {
            newline()
            value = try parseValue(indent + 2)
        } else {
            value = try parseValue(indent)
        }

        return (keyStr, value)
    }

    /// マップ全体のパース。
    func parseMap(_ indent: Int) throws -> YamlValue {
        var entries: [String: YamlValue] = [:]

        while true {
            let savedPos = pos
            do {
                let (key, value) = try parseMapEntry(indent)
                entries[key] = value

                if peek() == "\n" {
                    newline()
                } else {
                    break
                }
            } catch {
                pos = savedPos
                break
            }
        }

        guard !entries.isEmpty else {
            throw ParseError(message: "マップが空です")
        }

        return .map(entries)
    }

    /// YAML値のパース（再帰的）。
    func parseValue(_ indent: Int) throws -> YamlValue {
        let savedPos = pos

        // リストを試行
        do {
            return try parseList(indent)
        } catch {
            pos = savedPos
        }

        // マップを試行
        do {
            return try parseMap(indent)
        } catch {
            pos = savedPos
        }

        // スカラー
        return try scalarValue()
    }

    /// ドキュメント全体のパース。
    func document() throws -> Document {
        // 空行やコメントをスキップ
        while !isEof() {
            let savedPos = pos
            blankOrComment()
            if pos == savedPos {
                break
            }
        }

        let doc = try parseValue(0)

        // 末尾の空行やコメントをスキップ
        while !isEof() {
            let savedPos = pos
            blankOrComment()
            if pos == savedPos {
                break
            }
        }

        guard isEof() else {
            throw ParseError(message: "ドキュメントの終端が期待されます")
        }

        return doc
    }
}

/// パブリックAPI：YAML文字列をパース。
func parseYaml(_ input: String) -> Result<Document, ParseError> {
    let parser = Parser(input: input)
    do {
        let doc = try parser.document()
        return .success(doc)
    } catch let error as ParseError {
        return .failure(error)
    } catch {
        return .failure(ParseError(message: "不明なエラー: \(error)"))
    }
}

/// 簡易的なレンダリング（検証用）。
func renderToString(_ doc: Document) -> String {
    func renderValue(_ value: YamlValue, _ indent: Int) -> String {
        let indentStr = String(repeating: " ", count: indent)

        switch value {
        case .scalar(let s):
            return s
        case .null:
            return "null"
        case .list(let items):
            return items.map { item in
                "\(indentStr)- \(renderValue(item, indent + 2))"
            }.joined(separator: "\n")
        case .map(let entries):
            return entries.map { (key, val) in
                switch val {
                case .scalar, .null:
                    return "\(indentStr)\(key): \(renderValue(val, 0))"
                default:
                    return "\(indentStr)\(key):\n\(renderValue(val, indent + 2))"
                }
            }.joined(separator: "\n")
        }
    }

    return renderValue(doc, 0)
}

/// テスト例。
func testExamples() {
    let examples: [(String, String)] = [
        ("simple_scalar", "hello"),
        ("simple_list", "- item1\n- item2\n- item3"),
        ("simple_map", "key1: value1\nkey2: value2"),
        ("nested_map", "parent:\n  child1: value1\n  child2: value2"),
        ("nested_list", "items:\n  - item1\n  - item2"),
        ("mixed", "name: John\nage: 30\nhobbies:\n  - reading\n  - coding")
    ]

    for (name, yamlStr) in examples {
        print("--- \(name) ---")
        switch parseYaml(yamlStr) {
        case .success(let doc):
            print("パース成功:")
            print(renderToString(doc))
        case .failure(let err):
            print("パースエラー: \(err)")
        }
    }
}

/// インデント処理の課題と解決策：
///
/// 1. **インデントレベルの追跡**
///    - パーサー引数としてインデントレベルを渡す
///    - Swiftのクラスでパーサー状態を管理
///
/// 2. **エラー回復**
///    - do/catchでバックトラックを制御
///    - ParseError型で分かりやすいエラーメッセージを提供
///
/// 3. **空白の扱い**
///    - hspaceで水平空白のみをスキップ（改行は構文の一部）
///    - newlineでCR/LF/CRLFを正規化
///
/// Remlとの比較：
///
/// - **Swiftの利点**:
///   - 強力な型システムとResultによるエラーハンドリング
///   - 読みやすいモダンな構文
///
/// - **Swiftの課題**:
///   - パーサーコンビネーターライブラリがRemlほど充実していない
///   - 手動のバックトラック管理が煩雑
///
/// - **Remlの利点**:
///   - 字句レイヤの柔軟性により、インデント処理が自然に表現できる
///   - cut/commitによるエラー品質の向上
///   - recoverによる部分的なパース継続が可能