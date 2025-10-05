// Markdown風軽量マークアップパーサー - F#実装
//
// Unicode処理の注意点：
// - F#のstringは.NETのString（UTF-16ベース）
// - String.length はUTF-16コードユニット数を返す（サロゲートペアに注意）
// - .NET Core 3.0以降はRune型でUnicodeコードポイント単位の操作が可能
// - Grapheme（書記素クラスター）処理にはSystem.Globalization.StringInfoが必要
// - Remlの3層モデルと比較すると、F#も明示的な区別が必要

module MarkdownParser

open System

/// Markdown AST のインライン要素
type Inline =
  | Text of string
  | Strong of Inline list
  | Emphasis of Inline list
  | Code of string
  | Link of text: Inline list * url: string
  | LineBreak

/// Markdown AST のブロック要素
type Block =
  | Heading of level: int * inline: Inline list
  | Paragraph of inline: Inline list
  | UnorderedList of items: Inline list list
  | OrderedList of items: Inline list list
  | CodeBlock of lang: string option * code: string
  | HorizontalRule

type Document = Block list

/// パーサー状態
type ParseState =
  { Input: string
    Position: int }

type ParseResult<'T> = Result<'T * ParseState, string>

/// 現在位置の1文字を取得
let peekChar (state: ParseState) : char option =
  if state.Position >= state.Input.Length then
    None
  else
    Some state.Input.[state.Position]

/// 1文字を消費して進める
let advanceChar (state: ParseState) : ParseState =
  { state with Position = state.Position + 1 }

/// 固定文字列をマッチ
let matchString (target: string) (state: ParseState) : ParseState option =
  let remaining = state.Input.Substring(state.Position)
  if remaining.StartsWith(target) then
    Some { state with Position = state.Position + target.Length }
  else
    None

/// 水平空白をスキップ
let rec skipHSpace (state: ParseState) : ParseState =
  match peekChar state with
  | Some c when c = ' ' || c = '\t' ->
      skipHSpace (advanceChar state)
  | _ -> state

/// 空行をスキップ
let rec skipBlankLines (state: ParseState) : ParseState =
  match peekChar state with
  | Some '\n' -> skipBlankLines (advanceChar state)
  | _ -> state

/// 行末まで読む
let readUntilEol (state: ParseState) : string * ParseState =
  let rec loop (pos: int) =
    if pos >= state.Input.Length then
      pos
    elif state.Input.[pos] = '\n' then
      pos
    else
      loop (pos + 1)

  let endPos = loop state.Position
  let text = state.Input.Substring(state.Position, endPos - state.Position)
  (text, { state with Position = endPos })

/// 改行を消費
let consumeNewline (state: ParseState) : ParseState =
  match peekChar state with
  | Some '\n' -> advanceChar state
  | _ -> state

/// EOFチェック
let isEof (state: ParseState) : bool =
  state.Position >= state.Input.Length

/// 見出し行のパース（`# Heading` 形式）
let parseHeading (state: ParseState) : ParseResult<Block> =
  let state1 = skipHSpace state

  // `#` の連続をカウント
  let rec countHashes (s: ParseState) (n: int) : int * ParseState =
    match peekChar s with
    | Some '#' -> countHashes (advanceChar s) (n + 1)
    | _ -> (n, s)

  let (level, state2) = countHashes state1 0

  if level = 0 || level > 6 then
    Error "見出しレベルは1-6の範囲内である必要があります"
  else
    let state3 = skipHSpace state2
    let (text, state4) = readUntilEol state3
    let state5 = consumeNewline state4
    let inline = [Text (text.Trim())]
    Ok (Heading (level, inline), state5)

/// 水平線のパース（`---`, `***`, `___`）
let parseHorizontalRule (state: ParseState) : ParseResult<Block> =
  let state1 = skipHSpace state
  let (text, state2) = readUntilEol state1
  let state3 = consumeNewline state2

  let trimmed = text.Trim()
  let isRule =
    (trimmed.ToCharArray() |> Array.forall ((=) '-') && trimmed.Length >= 3) ||
    (trimmed.ToCharArray() |> Array.forall ((=) '*') && trimmed.Length >= 3) ||
    (trimmed.ToCharArray() |> Array.forall ((=) '_') && trimmed.Length >= 3)

  if isRule then
    Ok (HorizontalRule, state3)
  else
    Error "水平線として認識できません"

/// コードブロックのパース（```言語名）
let parseCodeBlock (state: ParseState) : ParseResult<Block> =
  match matchString "```" state with
  | None -> Error "コードブロック開始が見つかりません"
  | Some state1 ->
      let (langLine, state2) = readUntilEol state1
      let state3 = consumeNewline state2

      let lang =
        let trimmed = langLine.Trim()
        if String.IsNullOrEmpty(trimmed) then None else Some trimmed

      // コードブロック内容を ```閉じまで読む
      let rec readCodeLines (s: ParseState) (lines: string list) : string list * ParseState =
        match matchString "```" s with
        | Some endState -> (List.rev lines, endState)
        | None ->
            if isEof s then
              (List.rev lines, s)
            else
              let (line, s2) = readUntilEol s
              let s3 = consumeNewline s2
              readCodeLines s3 (line :: lines)

      let (codeLines, state4) = readCodeLines state3 []
      let state5 = consumeNewline state4
      let code = String.Join("\n", codeLines)
      Ok (CodeBlock (lang, code), state5)

/// リスト項目のパース（簡易版：`-` または `*`）
let parseUnorderedList (state: ParseState) : ParseResult<Block> =
  let rec parseItems (s: ParseState) (items: Inline list list) : Inline list list * ParseState =
    let s1 = skipHSpace s
    match peekChar s1 with
    | Some c when c = '-' || c = '*' ->
        let s2 = advanceChar s1
        let s3 = skipHSpace s2
        let (text, s4) = readUntilEol s3
        let s5 = consumeNewline s4
        let inline = [Text (text.Trim())]
        parseItems s5 (items @ [inline])
    | _ -> (items, s)

  let (items, stateEnd) = parseItems state []

  if List.isEmpty items then
    Error "リスト項目が見つかりません"
  else
    Ok (UnorderedList items, stateEnd)

/// 段落のパース（簡易版：空行まで）
let parseParagraph (state: ParseState) : ParseResult<Block> =
  let rec readLines (s: ParseState) (lines: string list) : string list * ParseState =
    if isEof s then
      (List.rev lines, s)
    else
      match peekChar s with
      | Some '\n' ->
          let s1 = advanceChar s
          match peekChar s1 with
          | Some '\n' -> (List.rev lines, s1) // 空行で段落終了
          | _ -> readLines s1 ("" :: lines)
      | Some _ ->
          let (line, s2) = readUntilEol s
          let s3 = consumeNewline s2
          readLines s3 (line :: lines)
      | None -> (List.rev lines, s)

  let (lines, stateEnd) = readLines state []
  let text = String.Join(" ", lines).Trim()
  let inline = [Text text]
  Ok (Paragraph inline, stateEnd)

/// ブロック要素のパース（優先順位付き試行）
let rec parseBlock (state: ParseState) : ParseResult<Block> =
  let state1 = skipBlankLines state

  if isEof state1 then
    Error "EOF"
  else
    let state2 = skipHSpace state1

    match peekChar state2 with
    | Some '#' -> parseHeading state2
    | Some '`' ->
        match matchString "```" state2 with
        | Some _ -> parseCodeBlock state2
        | None -> parseParagraph state2
    | Some c when c = '-' || c = '*' || c = '_' ->
        match parseHorizontalRule state2 with
        | Ok result -> Ok result
        | Error _ -> parseUnorderedList state2
    | Some _ -> parseParagraph state2
    | None -> Error "EOF"

/// ドキュメント全体のパース
let rec parseDocument (state: ParseState) (blocks: Block list) : Result<Document, string> =
  match parseBlock state with
  | Ok (block, newState) ->
      parseDocument newState (blocks @ [block])
  | Error "EOF" -> Ok blocks
  | Error msg -> Error msg

/// パブリックAPI：文字列からドキュメントをパース
let parse (input: string) : Result<Document, string> =
  let initialState = { Input = input; Position = 0 }
  parseDocument initialState []

/// 簡易的なレンダリング（検証用）
let rec renderInline (inlines: Inline list) : string =
  inlines
  |> List.map (fun i ->
      match i with
      | Text s -> s
      | Strong inner -> "**" + renderInline inner + "**"
      | Emphasis inner -> "*" + renderInline inner + "*"
      | Code s -> "`" + s + "`"
      | Link (text, url) -> "[" + renderInline text + "](" + url + ")"
      | LineBreak -> "\n"
  )
  |> String.concat ""

let renderBlock (block: Block) : string =
  match block with
  | Heading (level, inline) ->
      let prefix = String.replicate level "#"
      prefix + " " + renderInline inline + "\n\n"
  | Paragraph inline ->
      renderInline inline + "\n\n"
  | UnorderedList items ->
      let itemsStr =
        items
        |> List.map (fun item -> "- " + renderInline item + "\n")
        |> String.concat ""
      itemsStr + "\n"
  | OrderedList items ->
      let itemsStr =
        items
        |> List.mapi (fun i item -> $"{i + 1}. " + renderInline item + "\n")
        |> String.concat ""
      itemsStr + "\n"
  | CodeBlock (lang, code) ->
      let langStr = Option.defaultValue "" lang
      "```" + langStr + "\n" + code + "\n```\n\n"
  | HorizontalRule ->
      "---\n\n"

let renderToString (doc: Document) : string =
  doc
  |> List.map renderBlock
  |> String.concat ""

// Unicode 3層モデル比較：
//
// F#の string は .NET の System.String（UTF-16ベース）なので：
// - String.length はUTF-16コードユニット数を返す
// - サロゲートペア（例：絵文字「😀」）は2カウントされる
// - .NET Core 3.0以降は System.Text.Rune 型でコードポイント単位の操作が可能
// - Grapheme処理には System.Globalization.StringInfo が必要
//
// 例：
// let str = "🇯🇵"  // 国旗絵文字（2つのコードポイント、1つのgrapheme）
// str.Length  // => 4 (UTF-16コードユニット数)
// StringInfo(str).LengthInTextElements  // => 1 (grapheme数)
//
// Remlの3層モデル（Byte/Char/Grapheme）と比較すると、
// F#も明示的な区別が必要で、標準APIだけでは絵文字や結合文字の扱いが複雑。