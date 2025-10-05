import Foundation

/// TOML風パーサー：TOML v1.0.0準拠の簡易版実装。
///
/// 対応する構文：
/// - キーバリューペア: `key = "value"`
/// - テーブル: `[section]`
/// - 配列テーブル: `[[array_section]]`
/// - データ型: 文字列、整数、浮動小数点、真偽値、日時、配列、インラインテーブル
/// - コメント: `# comment`
///
/// 実装の特徴：
/// - Swiftのenum associated valuesによる型安全なTomlValue表現
/// - Result型を活用したエラーハンドリング
/// - プロトコルによる拡張可能なパーサー設計
/// - Optionalによる明示的な値の有無の表現
/// - 再帰的なネスト構造への対応

// TOML値の表現。
indirect enum TomlValue: Equatable {
    case string(String)
    case integer(Int)
    case float(Double)
    case boolean(Bool)
    case dateTime(String)  // 簡易実装：ISO8601文字列
    case array([TomlValue])
    case inlineTable([String: TomlValue])
}

typealias TomlTable = [String: TomlValue]

struct TomlDocument: Equatable {
    var root: TomlTable
    var tables: [[String]: TomlTable]  // セクション名パス → テーブル
}

struct ParseError: Error, CustomStringConvertible {
    let message: String
    let position: Int

    var description: String {
        return "位置 \(position): \(message)"
    }
}

class Parser {
    let input: String
    var pos: String.Index
    var currentTable: [String]  // 現在のテーブルセクション

    init(input: String) {
        self.input = input
        self.pos = input.startIndex
        self.currentTable = []
    }

    func currentPosition() -> Int {
        return input.distance(from: input.startIndex, to: pos)
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
            throw ParseError(
                message: "期待された文字 '\(expected)' が見つかりません",
                position: currentPosition()
            )
        }
        advance()
    }

    func expectString(_ expected: String) throws {
        for c in expected {
            try expect(c)
        }
    }

    /// 水平空白のスキップ。
    func hspace() {
        while let c = peek(), c == " " || c == "\t" {
            advance()
        }
    }

    /// 改行のスキップ。
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
            while let c = peek(), c != "\n" && c != "\r" {
                advance()
            }
        }
    }

    /// 空白・コメントのスキップ（改行を含む）。
    func skipWhitespaceAndComments() {
        while !isEof() {
            hspace()
            if peek() == "#" {
                comment()
            }
            if peek() == "\n" || peek() == "\r" {
                newline()
            } else {
                break
            }
        }
    }

    /// キー名のパース（ベアキーまたは引用符付き文字列）。
    func parseKey() throws -> String {
        hspace()

        // 引用符付きキー
        if peek() == "\"" {
            return try parseBasicString()
        }

        // ベアキー（英数字・`-`・`_`のみ）
        var key = ""
        while let c = peek(), c.isLetter || c.isNumber || c == "-" || c == "_" {
            key.append(c)
            advance()
        }

        guard !key.isEmpty else {
            throw ParseError(
                message: "キー名が空です",
                position: currentPosition()
            )
        }

        return key
    }

    /// ドットで区切られたキーパスのパース。
    func parseKeyPath() throws -> [String] {
        var path: [String] = []
        path.append(try parseKey())

        while peek() == "." {
            advance()
            path.append(try parseKey())
        }

        return path
    }

    /// 基本文字列のパース（`"..."`）。
    func parseBasicString() throws -> String {
        try expect("\"")

        var str = ""
        while let c = peek(), c != "\"" {
            if c == "\\" {
                advance()
                guard let escaped = peek() else {
                    throw ParseError(
                        message: "エスケープシーケンスが不完全です",
                        position: currentPosition()
                    )
                }

                // 簡易的なエスケープ処理
                switch escaped {
                case "n": str.append("\n")
                case "t": str.append("\t")
                case "r": str.append("\r")
                case "\\": str.append("\\")
                case "\"": str.append("\"")
                default: str.append(escaped)
                }
                advance()
            } else {
                str.append(c)
                advance()
            }
        }

        try expect("\"")
        return str
    }

    /// リテラル文字列のパース（`'...'`）。
    func parseLiteralString() throws -> String {
        try expect("'")

        var str = ""
        while let c = peek(), c != "'" {
            str.append(c)
            advance()
        }

        try expect("'")
        return str
    }

    /// 文字列値のパース。
    func parseStringValue() throws -> TomlValue {
        if peek() == "\"" {
            // 複数行基本文字列チェック
            let savedPos = pos
            advance()
            if peek() == "\"" {
                advance()
                if peek() == "\"" {
                    advance()
                    return try parseMultilineBasicString()
                }
            }
            pos = savedPos

            return .string(try parseBasicString())
        } else if peek() == "'" {
            // 複数行リテラル文字列チェック
            let savedPos = pos
            advance()
            if peek() == "'" {
                advance()
                if peek() == "'" {
                    advance()
                    return try parseMultilineLiteralString()
                }
            }
            pos = savedPos

            return .string(try parseLiteralString())
        }

        throw ParseError(
            message: "文字列が期待されます",
            position: currentPosition()
        )
    }

    /// 複数行基本文字列のパース（`"""..."""`）。
    func parseMultilineBasicString() throws -> TomlValue {
        var str = ""

        // 開始直後の改行はスキップ
        if peek() == "\n" {
            advance()
        } else if peek() == "\r" {
            advance()
            if peek() == "\n" {
                advance()
            }
        }

        var quoteCount = 0
        while quoteCount < 3 && !isEof() {
            if peek() == "\"" {
                quoteCount += 1
                advance()
            } else {
                for _ in 0..<quoteCount {
                    str.append("\"")
                }
                quoteCount = 0

                if let c = peek() {
                    str.append(c)
                    advance()
                }
            }
        }

        return .string(str)
    }

    /// 複数行リテラル文字列のパース（`'''...'''`）。
    func parseMultilineLiteralString() throws -> TomlValue {
        var str = ""

        // 開始直後の改行はスキップ
        if peek() == "\n" {
            advance()
        } else if peek() == "\r" {
            advance()
            if peek() == "\n" {
                advance()
            }
        }

        var quoteCount = 0
        while quoteCount < 3 && !isEof() {
            if peek() == "'" {
                quoteCount += 1
                advance()
            } else {
                for _ in 0..<quoteCount {
                    str.append("'")
                }
                quoteCount = 0

                if let c = peek() {
                    str.append(c)
                    advance()
                }
            }
        }

        return .string(str)
    }

    /// 整数値のパース。
    func parseInteger() throws -> TomlValue {
        var numStr = ""
        var isNegative = false

        if peek() == "-" {
            isNegative = true
            advance()
        } else if peek() == "+" {
            advance()
        }

        while let c = peek(), c.isNumber || c == "_" {
            if c != "_" {
                numStr.append(c)
            }
            advance()
        }

        guard let num = Int(numStr) else {
            throw ParseError(
                message: "整数のパースに失敗しました: \(numStr)",
                position: currentPosition()
            )
        }

        return .integer(isNegative ? -num : num)
    }

    /// 浮動小数点値のパース。
    func parseFloat() throws -> TomlValue {
        var numStr = ""

        if peek() == "-" || peek() == "+" {
            numStr.append(peek()!)
            advance()
        }

        while let c = peek(), c.isNumber || c == "_" || c == "." || c == "e" || c == "E" {
            if c != "_" {
                numStr.append(c)
            }
            advance()
        }

        guard let num = Double(numStr) else {
            throw ParseError(
                message: "浮動小数点数のパースに失敗しました: \(numStr)",
                position: currentPosition()
            )
        }

        return .float(num)
    }

    /// 真偽値のパース。
    func parseBoolean() throws -> TomlValue {
        let remaining = String(input[pos...])

        if remaining.hasPrefix("true") {
            try expectString("true")
            return .boolean(true)
        } else if remaining.hasPrefix("false") {
            try expectString("false")
            return .boolean(false)
        }

        throw ParseError(
            message: "真偽値が期待されます",
            position: currentPosition()
        )
    }

    /// 日時のパース（ISO 8601形式の簡易実装）。
    func parseDateTime() throws -> TomlValue {
        var dtStr = ""

        while let c = peek(), c.isNumber || c == "-" || c == ":" || c == "T" || c == "Z" || c == "." || c == "+" {
            dtStr.append(c)
            advance()
        }

        // 簡易検証：最低限の形式チェック
        guard dtStr.contains("T") || dtStr.contains("-") else {
            throw ParseError(
                message: "日時形式が無効です: \(dtStr)",
                position: currentPosition()
            )
        }

        return .dateTime(dtStr)
    }

    /// 配列のパース。
    func parseArray() throws -> TomlValue {
        try expect("[")
        skipWhitespaceAndComments()

        var items: [TomlValue] = []

        while peek() != "]" && !isEof() {
            let value = try parseValue()
            items.append(value)

            skipWhitespaceAndComments()

            if peek() == "," {
                advance()
                skipWhitespaceAndComments()
            } else if peek() != "]" {
                throw ParseError(
                    message: "',' または ']' が期待されます",
                    position: currentPosition()
                )
            }
        }

        try expect("]")
        return .array(items)
    }

    /// インラインテーブルのパース（`{ key = value, ... }`）。
    func parseInlineTable() throws -> TomlValue {
        try expect("{")
        hspace()

        var entries: [String: TomlValue] = [:]

        while peek() != "}" && !isEof() {
            let key = try parseKey()
            hspace()
            try expect("=")
            hspace()
            let value = try parseValue()

            entries[key] = value

            hspace()

            if peek() == "," {
                advance()
                hspace()
            } else if peek() != "}" {
                throw ParseError(
                    message: "',' または '}' が期待されます",
                    position: currentPosition()
                )
            }
        }

        try expect("}")
        return .inlineTable(entries)
    }

    /// TOML値のパース（再帰的）。
    func parseValue() throws -> TomlValue {
        skipWhitespaceAndComments()

        guard let c = peek() else {
            throw ParseError(
                message: "値が期待されます",
                position: currentPosition()
            )
        }

        // 文字列
        if c == "\"" || c == "'" {
            return try parseStringValue()
        }

        // 配列
        if c == "[" {
            return try parseArray()
        }

        // インラインテーブル
        if c == "{" {
            return try parseInlineTable()
        }

        // 真偽値
        let remaining = String(input[pos...])
        if remaining.hasPrefix("true") || remaining.hasPrefix("false") {
            return try parseBoolean()
        }

        // 数値または日時
        if c.isNumber || c == "-" || c == "+" {
            let savedPos = pos

            // 日時の試行
            do {
                return try parseDateTime()
            } catch {
                pos = savedPos
            }

            // 浮動小数点の試行
            var hasFloat = false
            var tempPos = pos
            while tempPos < input.endIndex {
                let ch = input[tempPos]
                if ch == "." || ch == "e" || ch == "E" {
                    hasFloat = true
                    break
                }
                if !ch.isNumber && ch != "_" && ch != "-" && ch != "+" {
                    break
                }
                tempPos = input.index(after: tempPos)
            }

            if hasFloat {
                return try parseFloat()
            } else {
                return try parseInteger()
            }
        }

        throw ParseError(
            message: "認識できない値です",
            position: currentPosition()
        )
    }

    /// キーバリューペアのパース（`key = value`）。
    func parseKeyValuePair() throws -> ([String], TomlValue) {
        let path = try parseKeyPath()
        hspace()
        try expect("=")
        let value = try parseValue()
        return (path, value)
    }

    /// テーブルヘッダーのパース（`[section.subsection]`）。
    func parseTableHeader() throws -> [String] {
        try expect("[")
        let path = try parseKeyPath()
        hspace()
        try expect("]")
        return path
    }

    /// 配列テーブルヘッダーのパース（`[[array_section]]`）。
    func parseArrayTableHeader() throws -> [String] {
        try expect("[")
        try expect("[")
        let path = try parseKeyPath()
        hspace()
        try expect("]")
        try expect("]")
        return path
    }

    /// ドキュメント要素の識別。
    enum DocumentElement {
        case keyValue([String], TomlValue)
        case table([String])
        case arrayTable([String])
    }

    /// ドキュメント全体のパース。
    func parseDocument() throws -> TomlDocument {
        var root: TomlTable = [:]
        var tables: [[String]: TomlTable] = [:]
        var currentTablePath: [String] = []

        skipWhitespaceAndComments()

        while !isEof() {
            hspace()

            if peek() == "[" {
                let savedPos = pos
                advance()

                if peek() == "[" {
                    // 配列テーブル
                    pos = savedPos
                    let path = try parseArrayTableHeader()
                    currentTablePath = path

                    if tables[path] == nil {
                        tables[path] = [:]
                    }
                } else {
                    // 通常のテーブル
                    pos = savedPos
                    let path = try parseTableHeader()
                    currentTablePath = path

                    if tables[path] == nil {
                        tables[path] = [:]
                    }
                }
            } else if !isEof() && peek() != "#" && peek() != "\n" && peek() != "\r" {
                // キーバリューペア
                let (path, value) = try parseKeyValuePair()

                if currentTablePath.isEmpty {
                    // ルートテーブルに追加
                    insertNested(&root, path: path, value: value)
                } else {
                    // 現在のテーブルに追加
                    if var table = tables[currentTablePath] {
                        insertNested(&table, path: path, value: value)
                        tables[currentTablePath] = table
                    }
                }
            }

            skipWhitespaceAndComments()
        }

        return TomlDocument(root: root, tables: tables)
    }

    /// ネストしたキーパスに値を挿入する補助関数。
    func insertNested(_ table: inout TomlTable, path: [String], value: TomlValue) {
        if path.count == 1 {
            table[path[0]] = value
        } else if path.count > 1 {
            let key = path[0]
            let rest = Array(path[1...])

            var nested: TomlTable
            if case .inlineTable(let existing) = table[key] {
                nested = existing
            } else {
                nested = [:]
            }

            insertNested(&nested, path: rest, value: value)
            table[key] = .inlineTable(nested)
        }
    }
}

/// パブリックAPI：TOML文字列をパース。
func parseToml(_ input: String) -> Result<TomlDocument, ParseError> {
    let parser = Parser(input: input)
    do {
        let doc = try parser.parseDocument()
        return .success(doc)
    } catch let error as ParseError {
        return .failure(error)
    } catch {
        return .failure(ParseError(
            message: "不明なエラー: \(error)",
            position: parser.currentPosition()
        ))
    }
}

/// 簡易的なレンダリング（検証用）。
func renderToString(_ doc: TomlDocument) -> String {
    var output = ""

    // ルートテーブルのレンダリング
    output += renderTable(doc.root, prefix: [])

    // 各セクションのレンダリング
    for (path, table) in doc.tables.sorted(by: { $0.key.joined(separator: ".") < $1.key.joined(separator: ".") }) {
        output += "\n[\(path.joined(separator: "."))]\n"
        output += renderTable(table, prefix: [])
    }

    return output
}

func renderTable(_ table: TomlTable, prefix: [String]) -> String {
    var output = ""

    for (key, value) in table.sorted(by: { $0.key < $1.key }) {
        let fullKey = prefix.isEmpty ? key : (prefix + [key]).joined(separator: ".")

        switch value {
        case .inlineTable(let nested):
            output += renderTable(nested, prefix: prefix + [key])
        default:
            output += "\(fullKey) = \(renderValue(value))\n"
        }
    }

    return output
}

func renderValue(_ value: TomlValue) -> String {
    switch value {
    case .string(let s):
        return "\"\(s)\""
    case .integer(let n):
        return "\(n)"
    case .float(let f):
        return "\(f)"
    case .boolean(let b):
        return b ? "true" : "false"
    case .dateTime(let dt):
        return dt
    case .array(let items):
        let itemsStr = items.map { renderValue($0) }.joined(separator: ", ")
        return "[\(itemsStr)]"
    case .inlineTable(let entries):
        let entriesStr = entries.map { (k, v) in "\(k) = \(renderValue(v))" }.joined(separator: ", ")
        return "{ \(entriesStr) }"
    }
}

/// テスト例：Reml風の設定。
func testRemlToml() {
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

    print("--- reml.toml 風設定のパース ---")
    switch parseToml(exampleToml) {
    case .success(let doc):
        print("パース成功:")
        print(renderToString(doc))
    case .failure(let err):
        print("パースエラー: \(err)")
    }
}

/// 他のテスト例。
func testExamples() {
    let examples: [(String, String)] = [
        ("simple_key_value", "key = \"value\""),
        ("integer", "port = 8080"),
        ("float", "pi = 3.14159"),
        ("boolean", "enabled = true"),
        ("array", "numbers = [1, 2, 3, 4, 5]"),
        ("inline_table", "server = { host = \"localhost\", port = 8080 }"),
        ("table", "[database]\nhost = \"localhost\"\nport = 5432"),
        ("nested", "[server.database]\nconnection = \"postgresql\"")
    ]

    for (name, tomlStr) in examples {
        print("\n--- \(name) ---")
        switch parseToml(tomlStr) {
        case .success(let doc):
            print("パース成功:")
            print(renderToString(doc))
        case .failure(let err):
            print("パースエラー: \(err)")
        }
    }
}

/// TOML処理の課題と解決策：
///
/// 1. **型の多様性**
///    - indirect enumでTomlValueを再帰的に定義
///    - associated valuesで各データ型を表現
///
/// 2. **エラーハンドリング**
///    - Result型で成功/失敗を明示
///    - ParseError型でエラー位置とメッセージを提供
///
/// 3. **ネスト構造の管理**
///    - ドットで区切られたキーパスを配列で管理
///    - 再帰的にネストしたテーブルを構築
///
/// 4. **複数行文字列**
///    - `"""..."""` と `'''...'''` に対応
///    - 開始直後の改行を正しくスキップ
///
/// Remlとの比較：
///
/// - **Swiftの利点**:
///   - 強力な型システムによる安全性
///   - Result型とOptionalによる明示的なエラーハンドリング
///   - モダンな構文で可読性が高い
///   - enum associated valuesによる柔軟なデータ表現
///
/// - **Swiftの課題**:
///   - パーサーコンビネーターが標準ライブラリにない
///   - 手動でバックトラックを実装する必要がある
///   - エラー回復機能の実装が複雑
///
/// - **Remlの利点**:
///   - パーサーコンビネーターが標準で提供される
///   - cut/commitによる高品質なエラーメッセージ
///   - recoverによる部分的なパース継続が容易
///   - 字句レイヤの柔軟性が高い
///
/// 実装の特徴：
///
/// - TOML v1.0.0の主要機能をサポート
/// - 基本データ型、配列、インラインテーブル、テーブル、配列テーブルに対応
/// - 複数行文字列とエスケープシーケンスをサポート
/// - アンダースコア区切りの数値に対応
/// - トレーリングカンマを許可
/// - コメントと空白を適切に処理