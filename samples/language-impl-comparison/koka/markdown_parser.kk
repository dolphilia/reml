// Markdown風軽量マークアップパーサー - Koka実装
//
// Unicode処理の注意点：
// - Kokaのstringは内部的にUTF-8
// - string.count はUnicodeコードポイント数を返す
// - Grapheme（書記素クラスター）単位の操作は標準ライブラリにない
// - Remlの3層モデル（Byte/Char/Grapheme）と比較すると、Kokaも明示的な区別が必要

module samples/language-impl-comparison/markdown-parser

import std/core

// Markdown AST のインライン要素
type inline
  IText(text: string)
  IStrong(inner: list<inline>)
  IEmphasis(inner: list<inline>)
  ICode(code: string)
  ILink(text: list<inline>, url: string)
  ILineBreak

// Markdown AST のブロック要素
type block
  Heading(level: int, inline: list<inline>)
  Paragraph(inline: list<inline>)
  UnorderedList(items: list<list<inline>>)
  OrderedList(items: list<list<inline>>)
  CodeBlock(lang: maybe<string>, code: string)
  HorizontalRule

alias document = list<block>

// パーサー状態
struct parse-state
  input: string
  position: int

alias parse-result<a> = result<string, (a, parse-state)>

// 現在位置の1文字を取得
fun peek-char(state: parse-state) : maybe<char>
  if state.position >= state.input.count then Nothing
  else Just(state.input.slice.drop(state.position).head-char.unjust)

// 1文字を消費して進める
fun advance-char(state: parse-state) : parse-state
  state(position = state.position + 1)

// 固定文字列をマッチ
fun match-string(state: parse-state, target: string) : maybe<parse-state>
  val remaining = state.input.slice.drop(state.position).string
  if remaining.starts-with(target).bool then
    Just(state(position = state.position + target.count))
  else
    Nothing

// 水平空白をスキップ
fun skip-hspace(state: parse-state) : parse-state
  match peek-char(state)
    Just(c) | c == ' ' || c == '\t' -> skip-hspace(advance-char(state))
    _ -> state

// 空行をスキップ
fun skip-blank-lines(state: parse-state) : parse-state
  match peek-char(state)
    Just('\n') -> skip-blank-lines(advance-char(state))
    _ -> state

// 行末まで読む
fun read-until-eol(state: parse-state) : (string, parse-state)
  fun loop(pos: int) : int
    if pos >= state.input.count then pos
    elif state.input.slice.drop(pos).head-char.unjust == '\n' then pos
    else loop(pos + 1)

  val end-pos = loop(state.position)
  val text = state.input.slice.drop(state.position).take(end-pos - state.position).string
  (text, state(position = end-pos))

// 改行を消費
fun consume-newline(state: parse-state) : parse-state
  match peek-char(state)
    Just('\n') -> advance-char(state)
    _ -> state

// EOFチェック
fun is-eof(state: parse-state) : bool
  state.position >= state.input.count

// 見出し行のパース（`# Heading` 形式）
fun parse-heading(state: parse-state) : parse-result<block>
  val state1 = skip-hspace(state)

  // `#` の連続をカウント
  fun count-hashes(s: parse-state, n: int) : (int, parse-state)
    match peek-char(s)
      Just('#') -> count-hashes(advance-char(s), n + 1)
      _ -> (n, s)

  val (level, state2) = count-hashes(state1, 0)

  if level == 0 || level > 6 then
    Error("見出しレベルは1-6の範囲内である必要があります")
  else
    val state3 = skip-hspace(state2)
    val (text, state4) = read-until-eol(state3)
    val state5 = consume-newline(state4)
    val inline = [IText(text.trim)]
    Ok((Heading(level, inline), state5))

// 水平線のパース（`---`, `***`, `___`）
fun parse-horizontal-rule(state: parse-state) : parse-result<block>
  val state1 = skip-hspace(state)
  val (text, state2) = read-until-eol(state1)
  val state3 = consume-newline(state2)

  val trimmed = text.trim
  val is-rule =
    (trimmed.list.all(fn(c) c == '-') && trimmed.count >= 3) ||
    (trimmed.list.all(fn(c) c == '*') && trimmed.count >= 3) ||
    (trimmed.list.all(fn(c) c == '_') && trimmed.count >= 3)

  if is-rule then
    Ok((HorizontalRule, state3))
  else
    Error("水平線として認識できません")

// コードブロックのパース（```言語名）
fun parse-code-block(state: parse-state) : parse-result<block>
  match match-string(state, "```")
    Nothing -> Error("コードブロック開始が見つかりません")
    Just(state1) ->
      val (lang-line, state2) = read-until-eol(state1)
      val state3 = consume-newline(state2)

      val lang =
        val trimmed = lang-line.trim
        if trimmed.is-empty then Nothing else Just(trimmed)

      // コードブロック内容を ```閉じまで読む
      fun read-code-lines(s: parse-state, lines: list<string>) : (list<string>, parse-state)
        match match-string(s, "```")
          Just(end-state) -> (lines.reverse, end-state)
          Nothing ->
            if is-eof(s) then
              (lines.reverse, s)
            else
              val (line, s2) = read-until-eol(s)
              val s3 = consume-newline(s2)
              read-code-lines(s3, Cons(line, lines))

      val (code-lines, state4) = read-code-lines(state3, [])
      val state5 = consume-newline(state4)
      val code = code-lines.join("\n")
      Ok((CodeBlock(lang, code), state5))

// リスト項目のパース（簡易版：`-` または `*`）
fun parse-unordered-list(state: parse-state) : parse-result<block>
  fun parse-items(s: parse-state, items: list<list<inline>>) : (list<list<inline>>, parse-state)
    val s1 = skip-hspace(s)
    match peek-char(s1)
      Just(c) | c == '-' || c == '*' ->
        val s2 = advance-char(s1)
        val s3 = skip-hspace(s2)
        val (text, s4) = read-until-eol(s3)
        val s5 = consume-newline(s4)
        val inline = [IText(text.trim)]
        parse-items(s5, items ++ [inline])
      _ -> (items, s)

  val (items, state-end) = parse-items(state, [])

  if items.is-nil then
    Error("リスト項目が見つかりません")
  else
    Ok((UnorderedList(items), state-end))

// 段落のパース（簡易版：空行まで）
fun parse-paragraph(state: parse-state) : parse-result<block>
  fun read-lines(s: parse-state, lines: list<string>) : (list<string>, parse-state)
    if is-eof(s) then
      (lines.reverse, s)
    else
      match peek-char(s)
        Just('\n') ->
          val s1 = advance-char(s)
          match peek-char(s1)
            Just('\n') -> (lines.reverse, s1) // 空行で段落終了
            _ -> read-lines(s1, Cons("", lines))
        Just(_) ->
          val (line, s2) = read-until-eol(s)
          val s3 = consume-newline(s2)
          read-lines(s3, Cons(line, lines))
        Nothing -> (lines.reverse, s)

  val (lines, state-end) = read-lines(state, [])
  val text = lines.join(" ").trim
  val inline = [IText(text)]
  Ok((Paragraph(inline), state-end))

// ブロック要素のパース（優先順位付き試行）
fun parse-block(state: parse-state) : parse-result<block>
  val state1 = skip-blank-lines(state)

  if is-eof(state1) then
    Error("EOF")
  else
    val state2 = skip-hspace(state1)

    match peek-char(state2)
      Just('#') -> parse-heading(state2)
      Just('`') ->
        match match-string(state2, "```")
          Just(_) -> parse-code-block(state2)
          Nothing -> parse-paragraph(state2)
      Just(c) | c == '-' || c == '*' || c == '_' ->
        match parse-horizontal-rule(state2)
          Ok(result) -> Ok(result)
          Error(_) -> parse-unordered-list(state2)
      Just(_) -> parse-paragraph(state2)
      Nothing -> Error("EOF")

// ドキュメント全体のパース
fun parse-document(state: parse-state, blocks: list<block>) : result<string, document>
  match parse-block(state)
    Ok((block, new-state)) ->
      parse-document(new-state, blocks ++ [block])
    Error("EOF") -> Ok(blocks)
    Error(msg) -> Error(msg)

// パブリックAPI：文字列からドキュメントをパース
pub fun parse(input: string) : exn document
  val initial-state = Parse-state(input, 0)
  match parse-document(initial-state, [])
    Ok(doc) -> doc
    Error(msg) -> throw(msg)

// 簡易的なレンダリング（検証用）
fun render-inline(inlines: list<inline>) : string
  inlines.map(fn(i)
    match i
      IText(s) -> s
      IStrong(inner) -> "**" ++ render-inline(inner) ++ "**"
      IEmphasis(inner) -> "*" ++ render-inline(inner) ++ "*"
      ICode(s) -> "`" ++ s ++ "`"
      ILink(text, url) -> "[" ++ render-inline(text) ++ "](" ++ url ++ ")"
      ILineBreak -> "\n"
  ).join("")

fun render-block(block: block) : string
  match block
    Heading(level, inline) ->
      val prefix = "#".repeat(level)
      prefix ++ " " ++ render-inline(inline) ++ "\n\n"
    Paragraph(inline) ->
      render-inline(inline) ++ "\n\n"
    UnorderedList(items) ->
      val items-str = items.map(fn(item) "- " ++ render-inline(item) ++ "\n").join("")
      items-str ++ "\n"
    OrderedList(items) ->
      val items-str = items.map-indexed(fn(i, item) show(i + 1) ++ ". " ++ render-inline(item) ++ "\n").join("")
      items-str ++ "\n"
    CodeBlock(lang, code) ->
      val lang-str = lang.default("")
      "```" ++ lang-str ++ "\n" ++ code ++ "\n```\n\n"
    HorizontalRule ->
      "---\n\n"

pub fun render-to-string(doc: document) : string
  doc.map(render-block).join("")

// Unicode 3層モデル比較：
//
// Kokaのstringは内部的にUTF-8で、以下の特性を持つ：
// - string.count はUnicodeコードポイント数を返す
// - Grapheme（書記素クラスター）単位の操作は標準ライブラリにない
// - 絵文字や結合文字の正確な扱いには追加のロジックが必要
//
// 例：
// val str = "🇯🇵"  // 国旗絵文字（2つのコードポイント、1つのgrapheme）
// str.count  // => 2 (コードポイント数)
//
// Remlの3層モデル（Byte/Char/Grapheme）と比較すると、
// Kokaも明示的な区別が必要で、標準APIだけでは絵文字や結合文字の扱いが複雑。
//
// Reml との比較メモ:
// 1. Koka: 効果システムが強力で、exn効果で例外を明示的に追跡
//    Reml: 同様の効果システムを持つが、より汎用的な代数的効果をサポート
// 2. Koka: Result型で明示的エラーハンドリング
//    Reml: Result型も使用可能で、エラー伝播の構文がより簡潔
// 3. Koka: 関数型スタイルが主体で、パターンマッチが強力
//    Reml: 関数型 + 手続き型のハイブリッドアプローチ
// 4. 両言語ともUnicode処理における明示性を重視