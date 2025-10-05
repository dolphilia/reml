// Markdown風軽量マークアップパーサー - Swift実装
//
// Unicode処理の注意点：
// - SwiftのStringはUnicodeスカラー値のコレクション
// - count はCharacter（拡張書記素クラスター）数を返す
// - utf8.count でUTF-8バイト数を取得可能
// - unicodeScalars.count でUnicodeスカラー値（コードポイント）数を取得可能
// - Remlの3層モデル（Byte/Char/Grapheme）に近い柔軟性を持つ

import Foundation

// Markdown AST のインライン要素
enum Inline {
  case text(String)
  case strong([Inline])
  case emphasis([Inline])
  case code(String)
  case link([Inline], String)
  case lineBreak
}

// Markdown AST のブロック要素
enum Block {
  case heading(Int, [Inline])
  case paragraph([Inline])
  case unorderedList([[Inline]])
  case orderedList([[Inline]])
  case codeBlock(String?, String)
  case horizontalRule
}

typealias Document = [Block]

// パーサー状態
struct ParseState {
  let input: String
  var position: String.Index
}

// パーサーエラー
enum ParseError: Error {
  case eof
  case message(String)
}

// 現在位置の1文字を取得
func peekChar(_ state: ParseState) -> Character? {
  guard state.position < state.input.endIndex else {
    return nil
  }
  return state.input[state.position]
}

// 1文字を消費して進める
func advanceChar(_ state: inout ParseState) {
  if state.position < state.input.endIndex {
    state.position = state.input.index(after: state.position)
  }
}

// 固定文字列をマッチ
func matchString(_ state: inout ParseState, _ target: String) -> Bool {
  let endIndex = state.input.index(state.position, offsetBy: target.count, limitedBy: state.input.endIndex) ?? state.input.endIndex
  let substring = String(state.input[state.position..<endIndex])
  if substring == target {
    state.position = endIndex
    return true
  }
  return false
}

// 水平空白をスキップ
func skipHSpace(_ state: inout ParseState) {
  while let ch = peekChar(state), ch == " " || ch == "\t" {
    advanceChar(&state)
  }
}

// 空行をスキップ
func skipBlankLines(_ state: inout ParseState) {
  while let ch = peekChar(state), ch == "\n" {
    advanceChar(&state)
  }
}

// 行末まで読む
func readUntilEol(_ state: inout ParseState) -> String {
  var line = ""
  while let ch = peekChar(state), ch != "\n" {
    line.append(ch)
    advanceChar(&state)
  }
  return line
}

// 改行を消費
func consumeNewline(_ state: inout ParseState) {
  if let ch = peekChar(state), ch == "\n" {
    advanceChar(&state)
  }
}

// EOFチェック
func isEof(_ state: ParseState) -> Bool {
  state.position >= state.input.endIndex
}

// 見出し行のパース（`# Heading` 形式）
func parseHeading(_ state: inout ParseState) throws -> Block {
  skipHSpace(&state)

  // `#` の連続をカウント
  var level = 0
  while let ch = peekChar(state), ch == "#" {
    level += 1
    advanceChar(&state)
  }

  guard level > 0 && level <= 6 else {
    throw ParseError.message("見出しレベルは1-6の範囲内である必要があります")
  }

  skipHSpace(&state)
  let text = readUntilEol(&state)
  consumeNewline(&state)

  let inline: [Inline] = [.text(text.trimmingCharacters(in: .whitespaces))]
  return .heading(level, inline)
}

// 水平線のパース（`---`, `***`, `___`）
func parseHorizontalRule(_ state: inout ParseState) throws -> Block {
  skipHSpace(&state)
  let text = readUntilEol(&state)
  consumeNewline(&state)

  let trimmed = text.trimmingCharacters(in: .whitespaces)
  let isRule =
    (trimmed.allSatisfy { $0 == "-" } && trimmed.count >= 3) ||
    (trimmed.allSatisfy { $0 == "*" } && trimmed.count >= 3) ||
    (trimmed.allSatisfy { $0 == "_" } && trimmed.count >= 3)

  guard isRule else {
    throw ParseError.message("水平線として認識できません")
  }

  return .horizontalRule
}

// コードブロックのパース（```言語名）
func parseCodeBlock(_ state: inout ParseState) throws -> Block {
  guard matchString(&state, "```") else {
    throw ParseError.message("コードブロック開始が見つかりません")
  }

  let langLine = readUntilEol(&state)
  consumeNewline(&state)

  let lang: String? = {
    let trimmed = langLine.trimmingCharacters(in: .whitespaces)
    return trimmed.isEmpty ? nil : trimmed
  }()

  // コードブロック内容を ```閉じまで読む
  var codeLines: [String] = []
  while !matchString(&state, "```") {
    if isEof(state) {
      break
    }
    let line = readUntilEol(&state)
    consumeNewline(&state)
    codeLines.append(line)
  }

  consumeNewline(&state)

  let code = codeLines.joined(separator: "\n")
  return .codeBlock(lang, code)
}

// リスト項目のパース（簡易版：`-` または `*`）
func parseUnorderedList(_ state: inout ParseState) throws -> Block {
  var items: [[Inline]] = []

  while true {
    skipHSpace(&state)
    guard let ch = peekChar(state), ch == "-" || ch == "*" else {
      break
    }

    advanceChar(&state)
    skipHSpace(&state)
    let text = readUntilEol(&state)
    consumeNewline(&state)

    let inline: [Inline] = [.text(text.trimmingCharacters(in: .whitespaces))]
    items.append(inline)
  }

  guard !items.isEmpty else {
    throw ParseError.message("リスト項目が見つかりません")
  }

  return .unorderedList(items)
}

// 段落のパース（簡易版：空行まで）
func parseParagraph(_ state: inout ParseState) throws -> Block {
  var lines: [String] = []

  while !isEof(state) {
    if let ch = peekChar(state), ch == "\n" {
      advanceChar(&state)
      if let ch2 = peekChar(state), ch2 == "\n" {
        break  // 空行で段落終了
      }
      lines.append("")
    } else {
      let line = readUntilEol(&state)
      consumeNewline(&state)
      lines.append(line)
    }
  }

  let text = lines.joined(separator: " ").trimmingCharacters(in: .whitespaces)
  let inline: [Inline] = [.text(text)]
  return .paragraph(inline)
}

// ブロック要素のパース（優先順位付き試行）
func parseBlock(_ state: inout ParseState) throws -> Block {
  skipBlankLines(&state)

  guard !isEof(state) else {
    throw ParseError.eof
  }

  skipHSpace(&state)

  guard let ch = peekChar(state) else {
    throw ParseError.eof
  }

  switch ch {
  case "#":
    return try parseHeading(&state)
  case "`":
    if matchString(&state, "```") {
      return try parseCodeBlock(&state)
    } else {
      return try parseParagraph(&state)
    }
  case "-", "*", "_":
    do {
      return try parseHorizontalRule(&state)
    } catch {
      return try parseUnorderedList(&state)
    }
  default:
    return try parseParagraph(&state)
  }
}

// ドキュメント全体のパース
func parseDocument(_ state: inout ParseState) throws -> Document {
  var blocks: [Block] = []

  while true {
    do {
      let block = try parseBlock(&state)
      blocks.append(block)
    } catch ParseError.eof {
      break
    }
  }

  return blocks
}

// パブリックAPI：文字列からドキュメントをパース
func parse(_ input: String) throws -> Document {
  var state = ParseState(input: input, position: input.startIndex)
  return try parseDocument(&state)
}

// 簡易的なレンダリング（検証用）
func renderInline(_ inlines: [Inline]) -> String {
  inlines.map { inline in
    switch inline {
    case .text(let s):
      return s
    case .strong(let inner):
      return "**\(renderInline(inner))**"
    case .emphasis(let inner):
      return "*\(renderInline(inner))*"
    case .code(let s):
      return "`\(s)`"
    case .link(let text, let url):
      return "[\(renderInline(text))](\(url))"
    case .lineBreak:
      return "\n"
    }
  }.joined()
}

func renderBlock(_ block: Block) -> String {
  switch block {
  case .heading(let level, let inline):
    let prefix = String(repeating: "#", count: level)
    return "\(prefix) \(renderInline(inline))\n\n"
  case .paragraph(let inline):
    return "\(renderInline(inline))\n\n"
  case .unorderedList(let items):
    let itemsStr = items.map { item in
      "- \(renderInline(item))\n"
    }.joined()
    return "\(itemsStr)\n"
  case .orderedList(let items):
    let itemsStr = items.enumerated().map { (i, item) in
      "\(i + 1). \(renderInline(item))\n"
    }.joined()
    return "\(itemsStr)\n"
  case .codeBlock(let lang, let code):
    let langStr = lang ?? ""
    return "```\(langStr)\n\(code)\n```\n\n"
  case .horizontalRule:
    return "---\n\n"
  }
}

func renderToString(_ doc: Document) -> String {
  doc.map(renderBlock).joined()
}

// Unicode 3層モデル比較：
//
// SwiftのStringはUnicodeスカラー値のコレクションで：
// - count はCharacter（拡張書記素クラスター）数を返す
// - utf8.count でUTF-8バイト数を取得可能
// - unicodeScalars.count でUnicodeスカラー値（コードポイント）数を取得可能
//
// 例：
// let str = "🇯🇵"  // 国旗絵文字（2つのコードポイント、1つのgrapheme）
// str.count  // => 1 (Character/Grapheme数)
// str.utf8.count  // => 8 (バイト数)
// str.unicodeScalars.count  // => 2 (コードポイント数)
//
// Remlの3層モデル（Byte/Char/Grapheme）に近い柔軟性を持ち、
// デフォルトでGrapheme単位の操作が可能なため、絵文字や結合文字の扱いが自然。
//
// Reml との比較メモ:
// 1. Swift: enum で代数的データ型を表現、associated valuesが強力
//    Reml: 型定義で直接 `type Inline = Text(string) | Strong(...) | ...` と記述
//    - 両言語とも代数的データ型とパターンマッチが強力
// 2. Swift: inout パラメータで状態を管理、値型のコピーセマンティクス
//    Reml: 関数型 + 手続き型のハイブリッドアプローチ
// 3. Swift: Result型とdo-catchでエラーハンドリング
//    Reml: Result型を標準で提供し、? 演算子で簡潔に記述
// 4. 両言語とも型推論が強力で、型安全性を確保
// 5. SwiftはUnicode処理においてRemlに近い明示性と柔軟性を持つ