/// JSON拡張版：コメント・トレーリングカンマ対応。
///
/// 標準JSONからの拡張点：
/// 1. コメント対応（`//` 行コメント、`/* */` ブロックコメント）
/// 2. トレーリングカンマ許可（配列・オブジェクトの最後の要素の後）
/// 3. より詳細なエラーメッセージ
///
/// 実用的な設定ファイル形式として：
/// - `package.json` 風の設定ファイル
/// - `.babelrc`, `.eslintrc` など開発ツールの設定
/// - VS Code の `settings.json`

import Foundation

// 型定義

enum JsonValue: Equatable {
    case null
    case bool(Bool)
    case number(Double)
    case string(String)
    case array([JsonValue])
    case object([String: JsonValue])
}

enum ParseError: Error {
    case unexpectedEOF
    case invalidValue(String)
    case unclosedString
    case unclosedBlockComment
    case expectedChar(Character)
    case invalidNumber(String)
}

class State {
    let input: String
    var pos: String.Index

    init(input: String) {
        self.input = input
        self.pos = input.startIndex
    }
}

// パース

func parse(_ input: String) -> Result<JsonValue, ParseError> {
    let state = State(input: input)

    do {
        try skipWhitespaceAndComments(state)
        let value = try parseValue(state)
        try skipWhitespaceAndComments(state)

        if state.pos >= state.input.endIndex {
            return .success(value)
        } else {
            return .failure(.invalidValue("入力の終端に到達していません"))
        }
    } catch let error as ParseError {
        return .failure(error)
    } catch {
        return .failure(.invalidValue("不明なエラー"))
    }
}

// 空白とコメントをスキップ

func skipWhitespaceAndComments(_ state: State) throws {
    while true {
        skipWs(state)
        if state.pos >= state.input.endIndex {
            return
        }

        let remaining = state.input[state.pos...]
        if remaining.hasPrefix("//") {
            skipLineComment(state)
        } else if remaining.hasPrefix("/*") {
            try skipBlockComment(state)
        } else {
            return
        }
    }
}

func skipWs(_ state: State) {
    while state.pos < state.input.endIndex {
        let ch = state.input[state.pos]
        if ch == " " || ch == "\n" || ch == "\t" || ch == "\r" {
            state.pos = state.input.index(after: state.pos)
        } else {
            break
        }
    }
}

func skipLineComment(_ state: State) {
    state.pos = state.input.index(state.pos, offsetBy: 2) // "//" をスキップ
    while state.pos < state.input.endIndex {
        if state.input[state.pos] == "\n" {
            state.pos = state.input.index(after: state.pos)
            break
        }
        state.pos = state.input.index(after: state.pos)
    }
}

func skipBlockComment(_ state: State) throws {
    state.pos = state.input.index(state.pos, offsetBy: 2) // "/*" をスキップ
    while state.pos < state.input.endIndex {
        let idx = state.input.index(after: state.pos)
        if idx < state.input.endIndex {
            let slice = state.input[state.pos..<state.input.index(state.pos, offsetBy: 2)]
            if slice == "*/" {
                state.pos = state.input.index(state.pos, offsetBy: 2)
                return
            }
        }
        state.pos = state.input.index(after: state.pos)
    }
    throw ParseError.unclosedBlockComment
}

// 値のパース

func parseValue(_ state: State) throws -> JsonValue {
    try skipWhitespaceAndComments(state)

    if state.pos >= state.input.endIndex {
        throw ParseError.unexpectedEOF
    }

    let remaining = state.input[state.pos...]

    if remaining.hasPrefix("null") {
        state.pos = state.input.index(state.pos, offsetBy: 4)
        return .null
    } else if remaining.hasPrefix("true") {
        state.pos = state.input.index(state.pos, offsetBy: 4)
        return .bool(true)
    } else if remaining.hasPrefix("false") {
        state.pos = state.input.index(state.pos, offsetBy: 5)
        return .bool(false)
    } else if remaining.first == "\"" {
        return try parseString(state)
    } else if remaining.first == "[" {
        return try parseArray(state)
    } else if remaining.first == "{" {
        return try parseObject(state)
    } else {
        return try parseNumber(state)
    }
}

// 文字列リテラルのパース

func parseString(_ state: State) throws -> JsonValue {
    state.pos = state.input.index(after: state.pos) // '"' をスキップ
    var result = ""

    while state.pos < state.input.endIndex {
        let ch = state.input[state.pos]
        if ch == "\"" {
            state.pos = state.input.index(after: state.pos)
            return .string(result)
        } else if ch == "\\" {
            state.pos = state.input.index(after: state.pos)
            if state.pos >= state.input.endIndex {
                throw ParseError.unclosedString
            }
            let escaped = state.input[state.pos]
            switch escaped {
            case "n": result.append("\n")
            case "t": result.append("\t")
            case "r": result.append("\r")
            case "\\": result.append("\\")
            case "\"": result.append("\"")
            default: result.append(escaped)
            }
            state.pos = state.input.index(after: state.pos)
        } else {
            result.append(ch)
            state.pos = state.input.index(after: state.pos)
        }
    }

    throw ParseError.unclosedString
}

// 数値のパース

func parseNumber(_ state: State) throws -> JsonValue {
    let start = state.pos

    while state.pos < state.input.endIndex {
        let ch = state.input[state.pos]
        if ch == "-" || ch == "+" || ch == "." || ch == "e" || ch == "E" || ch.isNumber {
            state.pos = state.input.index(after: state.pos)
        } else {
            break
        }
    }

    let numStr = String(state.input[start..<state.pos])
    guard let num = Double(numStr) else {
        throw ParseError.invalidNumber(numStr)
    }
    return .number(num)
}

// 配列のパース（トレーリングカンマ対応）

func parseArray(_ state: State) throws -> JsonValue {
    state.pos = state.input.index(after: state.pos) // '[' をスキップ
    try skipWhitespaceAndComments(state)

    if state.pos < state.input.endIndex && state.input[state.pos] == "]" {
        state.pos = state.input.index(after: state.pos)
        return .array([])
    }

    var items: [JsonValue] = []

    while true {
        let value = try parseValue(state)
        items.append(value)
        try skipWhitespaceAndComments(state)

        if state.pos >= state.input.endIndex {
            throw ParseError.unexpectedEOF
        }

        let ch = state.input[state.pos]
        if ch == "," {
            state.pos = state.input.index(after: state.pos)
            try skipWhitespaceAndComments(state)

            // トレーリングカンマチェック
            if state.pos < state.input.endIndex && state.input[state.pos] == "]" {
                state.pos = state.input.index(after: state.pos)
                return .array(items)
            }
        } else if ch == "]" {
            state.pos = state.input.index(after: state.pos)
            return .array(items)
        } else {
            throw ParseError.expectedChar(",")
        }
    }
}

// オブジェクトのパース（トレーリングカンマ対応）

func parseObject(_ state: State) throws -> JsonValue {
    state.pos = state.input.index(after: state.pos) // '{' をスキップ
    try skipWhitespaceAndComments(state)

    if state.pos < state.input.endIndex && state.input[state.pos] == "}" {
        state.pos = state.input.index(after: state.pos)
        return .object([:])
    }

    var pairs: [String: JsonValue] = [:]

    while true {
        let keyValue = try parseString(state)
        guard case .string(let key) = keyValue else {
            throw ParseError.invalidValue("オブジェクトのキーは文字列である必要があります")
        }

        try skipWhitespaceAndComments(state)

        if state.pos >= state.input.endIndex || state.input[state.pos] != ":" {
            throw ParseError.expectedChar(":")
        }
        state.pos = state.input.index(after: state.pos)

        try skipWhitespaceAndComments(state)

        let value = try parseValue(state)
        pairs[key] = value

        try skipWhitespaceAndComments(state)

        if state.pos >= state.input.endIndex {
            throw ParseError.unexpectedEOF
        }

        let ch = state.input[state.pos]
        if ch == "," {
            state.pos = state.input.index(after: state.pos)
            try skipWhitespaceAndComments(state)

            // トレーリングカンマチェック
            if state.pos < state.input.endIndex && state.input[state.pos] == "}" {
                state.pos = state.input.index(after: state.pos)
                return .object(pairs)
            }
        } else if ch == "}" {
            state.pos = state.input.index(after: state.pos)
            return .object(pairs)
        } else {
            throw ParseError.expectedChar(",")
        }
    }
}

// レンダリング

func renderToString(_ value: JsonValue, indentLevel: Int = 0) -> String {
    let indent = String(repeating: "  ", count: indentLevel)
    let nextIndent = String(repeating: "  ", count: indentLevel + 1)

    switch value {
    case .null:
        return "null"
    case .bool(true):
        return "true"
    case .bool(false):
        return "false"
    case .number(let num):
        return String(num)
    case .string(let str):
        return "\"\(str)\""
    case .array(let items):
        if items.isEmpty {
            return "[]"
        }
        let itemsStr = items
            .map { "\(nextIndent)\(renderToString($0, indentLevel: indentLevel + 1))" }
            .joined(separator: ",\n")
        return "[\n\(itemsStr)\n\(indent)]"
    case .object(let pairs):
        if pairs.isEmpty {
            return "{}"
        }
        let pairsStr = pairs
            .map { key, value in
                "\(nextIndent)\"\(key)\": \(renderToString(value, indentLevel: indentLevel + 1))"
            }
            .joined(separator: ",\n")
        return "{\n\(pairsStr)\n\(indent)}"
    }
}

// テスト

func testExtendedJson() {
    let testCases: [(String, String)] = [
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

    for (name, jsonStr) in testCases {
        print("--- \(name) ---")
        switch parse(jsonStr) {
        case .success(let value):
            print("パース成功:")
            print(renderToString(value, indentLevel: 0))
        case .failure(let err):
            print("パースエラー: \(err)")
        }
        print()
    }
}