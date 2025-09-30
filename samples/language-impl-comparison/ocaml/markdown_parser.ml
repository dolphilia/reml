(* Markdown風軽量マークアップパーサー - OCaml実装

   Unicode処理の注意点：
   - OCamlのstringはバイトシーケンス（Latin-1またはUTF-8を想定）
   - String.length はバイト数を返す
   - Ucharモジュールでコードポイント単位の操作が可能
   - Grapheme（書記素クラスター）処理には外部ライブラリ（uutf, uunf等）が必要
   - Remlの3層モデル（Byte/Char/Grapheme）と比較すると、OCamlも明示的な区別が必要
*)

(* Markdown AST のインライン要素 *)
type inline =
  | Text of string
  | Strong of inline list
  | Emphasis of inline list
  | Code of string
  | Link of inline list * string
  | LineBreak

(* Markdown AST のブロック要素 *)
type block =
  | Heading of int * inline list
  | Paragraph of inline list
  | UnorderedList of inline list list
  | OrderedList of inline list list
  | CodeBlock of string option * string
  | HorizontalRule

type document = block list

exception Parse_error of string

(* パーサー状態 *)
type state = {
  source : string;
  mutable index : int
}

(* 現在位置の1文字を取得 *)
let peek st =
  if st.index >= String.length st.source then None
  else Some st.source.[st.index]

(* 1文字を消費して進める *)
let advance st =
  st.index <- st.index + 1

(* 固定文字列をマッチ *)
let match_string st target =
  let target_len = String.length target in
  let remaining_len = String.length st.source - st.index in
  if remaining_len < target_len then false
  else
    let matched = String.sub st.source st.index target_len = target in
    if matched then st.index <- st.index + target_len;
    matched

(* 水平空白をスキップ *)
let rec skip_hspace st =
  match peek st with
  | Some (' ' | '\t') -> advance st; skip_hspace st
  | _ -> ()

(* 空行をスキップ *)
let rec skip_blank_lines st =
  match peek st with
  | Some '\n' -> advance st; skip_blank_lines st
  | _ -> ()

(* 行末まで読む *)
let read_until_eol st =
  let buf = Buffer.create 16 in
  let rec loop () =
    match peek st with
    | Some '\n' | None -> Buffer.contents buf
    | Some c -> Buffer.add_char buf c; advance st; loop ()
  in
  loop ()

(* 改行を消費 *)
let consume_newline st =
  match peek st with
  | Some '\n' -> advance st
  | _ -> ()

(* EOFチェック *)
let is_eof st =
  st.index >= String.length st.source

(* 見出し行のパース（`# Heading` 形式） *)
let parse_heading st =
  skip_hspace st;

  (* `#` の連続をカウント *)
  let rec count_hashes n =
    match peek st with
    | Some '#' -> advance st; count_hashes (n + 1)
    | _ -> n
  in

  let level = count_hashes 0 in

  if level = 0 || level > 6 then
    raise (Parse_error "見出しレベルは1-6の範囲内である必要があります")
  else begin
    skip_hspace st;
    let text = read_until_eol st in
    consume_newline st;
    let inline = [Text (String.trim text)] in
    Heading (level, inline)
  end

(* 水平線のパース（`---`, `***`, `___`） *)
let parse_horizontal_rule st =
  skip_hspace st;
  let text = read_until_eol st in
  consume_newline st;

  let trimmed = String.trim text in
  let is_rule =
    (String.length trimmed >= 3 &&
     String.for_all (fun c -> c = '-') trimmed) ||
    (String.length trimmed >= 3 &&
     String.for_all (fun c -> c = '*') trimmed) ||
    (String.length trimmed >= 3 &&
     String.for_all (fun c -> c = '_') trimmed)
  in

  if not is_rule then
    raise (Parse_error "水平線として認識できません")
  else
    HorizontalRule

(* コードブロックのパース（```言語名） *)
let parse_code_block st =
  if not (match_string st "```") then
    raise (Parse_error "コードブロック開始が見つかりません")
  else begin
    let lang_line = read_until_eol st in
    consume_newline st;

    let lang =
      let trimmed = String.trim lang_line in
      if String.length trimmed = 0 then None else Some trimmed
    in

    (* コードブロック内容を ```閉じまで読む *)
    let rec read_code_lines acc =
      if match_string st "```" then
        List.rev acc
      else if is_eof st then
        List.rev acc
      else begin
        let line = read_until_eol st in
        consume_newline st;
        read_code_lines (line :: acc)
      end
    in

    let code_lines = read_code_lines [] in
    consume_newline st;

    let code = String.concat "\n" code_lines in
    CodeBlock (lang, code)
  end

(* リスト項目のパース（簡易版：`-` または `*`） *)
let parse_unordered_list st =
  let rec parse_items acc =
    skip_hspace st;
    match peek st with
    | Some ('-' | '*') ->
        advance st;
        skip_hspace st;
        let text = read_until_eol st in
        consume_newline st;
        let inline = [Text (String.trim text)] in
        parse_items (inline :: acc)
    | _ -> List.rev acc
  in

  let items = parse_items [] in

  if items = [] then
    raise (Parse_error "リスト項目が見つかりません")
  else
    UnorderedList items

(* 段落のパース（簡易版：空行まで） *)
let parse_paragraph st =
  let rec read_lines acc =
    if is_eof st then
      List.rev acc
    else
      match peek st with
      | Some '\n' ->
          advance st;
          begin match peek st with
          | Some '\n' -> List.rev acc  (* 空行で段落終了 *)
          | _ -> read_lines ("" :: acc)
          end
      | Some _ ->
          let line = read_until_eol st in
          consume_newline st;
          read_lines (line :: acc)
      | None -> List.rev acc
  in

  let lines = read_lines [] in
  let text = String.concat " " lines |> String.trim in
  let inline = [Text text] in
  Paragraph inline

(* ブロック要素のパース（優先順位付き試行） *)
let rec parse_block st =
  skip_blank_lines st;

  if is_eof st then
    raise (Parse_error "EOF")
  else begin
    skip_hspace st;

    match peek st with
    | Some '#' -> parse_heading st
    | Some '`' ->
        if match_string st "```" then
          parse_code_block st
        else
          parse_paragraph st
    | Some ('-' | '*' | '_') ->
        begin try
          parse_horizontal_rule st
        with Parse_error _ ->
          parse_unordered_list st
        end
    | Some _ -> parse_paragraph st
    | None -> raise (Parse_error "EOF")
  end

(* ドキュメント全体のパース *)
let rec parse_document st acc =
  try
    let block = parse_block st in
    parse_document st (block :: acc)
  with Parse_error "EOF" ->
    List.rev acc

(* パブリックAPI：文字列からドキュメントをパース *)
let parse source =
  let st = { source; index = 0 } in
  parse_document st []

(* 簡易的なレンダリング（検証用） *)
let rec render_inline inlines =
  String.concat "" (List.map (fun i ->
    match i with
    | Text s -> s
    | Strong inner -> "**" ^ render_inline inner ^ "**"
    | Emphasis inner -> "*" ^ render_inline inner ^ "*"
    | Code s -> "`" ^ s ^ "`"
    | Link (text, url) -> "[" ^ render_inline text ^ "](" ^ url ^ ")"
    | LineBreak -> "\n"
  ) inlines)

let render_block block =
  match block with
  | Heading (level, inline) ->
      let prefix = String.make level '#' in
      prefix ^ " " ^ render_inline inline ^ "\n\n"
  | Paragraph inline ->
      render_inline inline ^ "\n\n"
  | UnorderedList items ->
      let items_str =
        String.concat "" (List.map (fun item ->
          "- " ^ render_inline item ^ "\n"
        ) items)
      in
      items_str ^ "\n"
  | OrderedList items ->
      let items_str =
        String.concat "" (List.mapi (fun i item ->
          string_of_int (i + 1) ^ ". " ^ render_inline item ^ "\n"
        ) items)
      in
      items_str ^ "\n"
  | CodeBlock (lang, code) ->
      let lang_str = match lang with Some l -> l | None -> "" in
      "```" ^ lang_str ^ "\n" ^ code ^ "\n```\n\n"
  | HorizontalRule ->
      "---\n\n"

let render_to_string doc =
  String.concat "" (List.map render_block doc)

(* Unicode 3層モデル比較：

   OCamlのstringはバイトシーケンス（Latin-1またはUTF-8を想定）で：
   - String.length はバイト数を返す
   - Uchar モジュールでUnicodeコードポイント単位の操作が可能
   - Grapheme（書記素クラスター）処理には uutf, uunf 等の外部ライブラリが必要

   例：
   let str = "🇯🇵"  (* 国旗絵文字（2つのコードポイント、1つのgrapheme） *)
   String.length str  (* => 8 (バイト数) *)

   Remlの3層モデル（Byte/Char/Grapheme）と比較すると、
   OCamlも明示的な区別が必要で、標準APIだけでは絵文字や結合文字の扱いが複雑。

   Reml との比較メモ:
   1. OCaml: 代数的データ型が強力で、パターンマッチが洗練されている
      Reml: 同様の代数的データ型を持ち、より現代的な構文
   2. OCaml: 参照（ref）やミュータブルフィールドで状態を管理
      Reml: 関数型 + 手続き型のハイブリッドアプローチ
   3. OCaml: 例外機構が主流、Result型も利用可能
      Reml: Result型を標準で提供し、? 演算子で簡潔に記述
   4. 両言語とも型推論が強力で、型安全性を確保
*)