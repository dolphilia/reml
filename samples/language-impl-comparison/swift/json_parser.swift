// JSON パーサー (Swift 実装)
// JSON 構文を解析して汎用値型に変換する

import Foundation

// JSON 値型
enum JsonValue {
  case jNull
  case jBool(Bool)
  case jNumber(Double)
  case jString(String)
  case jArray([JsonValue])
  case jObject([String: JsonValue])
}

// トークン型
enum Token {
  case lBrace
  case rBrace
  case lBracket
  case rBracket
  case colon
  case comma
  case stringLiteral(String)
  case numberLiteral(Double)
  case boolLiteral(Bool)
  case nullLiteral
}

// パース状態
struct ParseState {
  let tokens: [Token]
}

// パースエラー
enum ParseError: Error {
  case unexpectedEOF
  case unexpectedToken(expected: String, found: Token)
}

// トークン化
func tokenize(_ source: String) -> [Token] {
  func loop(index: Int, acc: [Token]) -> [Token] {
    guard index < source.count else {
      return acc.reversed()
    }

    let ch = source[source.index(source.startIndex, offsetBy: index)]

    switch ch {
    case " ", "\n", "\t", "\r":
      return loop(index: index + 1, acc: acc)
    case "{":
      return loop(index: index + 1, acc: [.lBrace] + acc)
    case "}":
      return loop(index: index + 1, acc: [.rBrace] + acc)
    case "[":
      return loop(index: index + 1, acc: [.lBracket] + acc)
    case "]":
      return loop(index: index + 1, acc: [.rBracket] + acc)
    case ":":
      return loop(index: index + 1, acc: [.colon] + acc)
    case ",":
      return loop(index: index + 1, acc: [.comma] + acc)
    case "t":
      let startIdx = source.index(source.startIndex, offsetBy: index)
      let endIdx = source.index(startIdx, offsetBy: 4, limitedBy: source.endIndex) ?? source.endIndex
      if source[startIdx..<endIdx] == "true" {
        return loop(index: index + 4, acc: [.boolLiteral(true)] + acc)
      }
      return loop(index: index + 1, acc: acc)
    case "f":
      let startIdx = source.index(source.startIndex, offsetBy: index)
      let endIdx = source.index(startIdx, offsetBy: 5, limitedBy: source.endIndex) ?? source.endIndex
      if source[startIdx..<endIdx] == "false" {
        return loop(index: index + 5, acc: [.boolLiteral(false)] + acc)
      }
      return loop(index: index + 1, acc: acc)
    case "n":
      let startIdx = source.index(source.startIndex, offsetBy: index)
      let endIdx = source.index(startIdx, offsetBy: 4, limitedBy: source.endIndex) ?? source.endIndex
      if source[startIdx..<endIdx] == "null" {
        return loop(index: index + 4, acc: [.nullLiteral] + acc)
      }
      return loop(index: index + 1, acc: acc)
    case "\"":
      let startIdx = source.index(source.startIndex, offsetBy: index + 1)
      if let endIdx = source[startIdx...].firstIndex(of: "\"") {
        let str = String(source[startIdx..<endIdx])
        let nextIndex = source.distance(from: source.startIndex, to: endIdx) + 1
        return loop(index: nextIndex, acc: [.stringLiteral(str)] + acc)
      }
      return loop(index: index + 1, acc: acc)
    default:
      // 数値の読み取り (簡易実装)
      var endIndex = index
      while endIndex < source.count {
        let ch = source[source.index(source.startIndex, offsetBy: endIndex)]
        if ch.isNumber || ch == "." || ch == "-" {
          endIndex += 1
        } else {
          break
        }
      }
      let startIdx = source.index(source.startIndex, offsetBy: index)
      let endIdx = source.index(source.startIndex, offsetBy: endIndex)
      let numStr = String(source[startIdx..<endIdx])
      if let num = Double(numStr) {
        return loop(index: endIndex, acc: [.numberLiteral(num)] + acc)
      }
      return loop(index: index + 1, acc: acc)
    }
  }
  return loop(index: 0, acc: [])
}

// 値のパース
func parseValue(_ state: ParseState) -> Result<(JsonValue, ParseState), ParseError> {
  guard !state.tokens.isEmpty else {
    return .failure(.unexpectedEOF)
  }
  let token = state.tokens[0]
  let rest = Array(state.tokens.dropFirst())

  switch token {
  case .nullLiteral:
    return .success((.jNull, ParseState(tokens: rest)))
  case .boolLiteral(let flag):
    return .success((.jBool(flag), ParseState(tokens: rest)))
  case .numberLiteral(let num):
    return .success((.jNumber(num), ParseState(tokens: rest)))
  case .stringLiteral(let text):
    return .success((.jString(text), ParseState(tokens: rest)))
  case .lBracket:
    return parseArray(ParseState(tokens: rest))
  case .lBrace:
    return parseObject(ParseState(tokens: rest))
  default:
    return .failure(.unexpectedToken(expected: "値", found: token))
  }
}

// 配列のパース
func parseArray(_ state: ParseState) -> Result<(JsonValue, ParseState), ParseError> {
  if !state.tokens.isEmpty && state.tokens[0] == .rBracket {
    let rest = Array(state.tokens.dropFirst())
    return .success((.jArray([]), ParseState(tokens: rest)))
  }

  func loop(current: ParseState, acc: [JsonValue]) -> Result<(JsonValue, ParseState), ParseError> {
    switch parseValue(current) {
    case .failure(let err):
      return .failure(err)
    case .success(let (value, next)):
      let newAcc = acc + [value]
      guard !next.tokens.isEmpty else {
        return .failure(.unexpectedEOF)
      }
      let token = next.tokens[0]
      let rest = Array(next.tokens.dropFirst())
      switch token {
      case .comma:
        return loop(current: ParseState(tokens: rest), acc: newAcc)
      case .rBracket:
        return .success((.jArray(newAcc), ParseState(tokens: rest)))
      default:
        return .failure(.unexpectedToken(expected: "]", found: token))
      }
    }
  }
  return loop(current: state, acc: [])
}

// オブジェクトのパース
func parseObject(_ state: ParseState) -> Result<(JsonValue, ParseState), ParseError> {
  if !state.tokens.isEmpty && state.tokens[0] == .rBrace {
    let rest = Array(state.tokens.dropFirst())
    return .success((.jObject([:]), ParseState(tokens: rest)))
  }

  func loop(current: ParseState, acc: [String: JsonValue]) -> Result<(JsonValue, ParseState), ParseError> {
    guard current.tokens.count >= 2 else {
      return .failure(.unexpectedEOF)
    }
    guard case .stringLiteral(let key) = current.tokens[0],
          case .colon = current.tokens[1] else {
      return .failure(.unexpectedToken(expected: "文字列:値", found: current.tokens[0]))
    }
    let rest = Array(current.tokens.dropFirst(2))

    switch parseValue(ParseState(tokens: rest)) {
    case .failure(let err):
      return .failure(err)
    case .success(let (value, next)):
      var newAcc = acc
      newAcc[key] = value
      guard !next.tokens.isEmpty else {
        return .failure(.unexpectedEOF)
      }
      let token = next.tokens[0]
      let rest2 = Array(next.tokens.dropFirst())
      switch token {
      case .comma:
        return loop(current: ParseState(tokens: rest2), acc: newAcc)
      case .rBrace:
        return .success((.jObject(newAcc), ParseState(tokens: rest2)))
      default:
        return .failure(.unexpectedToken(expected: "}", found: token))
      }
    }
  }
  return loop(current: state, acc: [:])
}

// メインパース関数
func parseJson(_ source: String) -> Result<JsonValue, String> {
  let tokens = tokenize(source)
  let state = ParseState(tokens: tokens)
  switch parseValue(state) {
  case .failure(.unexpectedEOF):
    return .failure("予期しない入力終端")
  case .failure(.unexpectedToken(let expected, _)):
    return .failure("期待: \(expected)")
  case .success(let (value, rest)):
    if rest.tokens.isEmpty {
      return .success(value)
    } else {
      return .failure("末尾に未消費トークンがあります")
    }
  }
}

// 利用例
// parseJson("""{"name": "Alice", "age": 30}""")
// => .success(.jObject(["name": .jString("Alice"), "age": .jNumber(30.0)]))