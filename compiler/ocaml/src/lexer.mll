{
(* Lexer — Reml 字句解析器
 *
 * docs/spec/1-1-syntax.md §A に基づく字句解析を実装する。
 * Unicode XID 準拠、コメント処理、文字列エスケープなど。
 *)

open Token

module Trivia_profile = Parser_run_config.Lex.Trivia_profile
module Lex_record = Core_parse_lex_record
module Unicode_tables = Lexer_tables.Unicode_xid_tables

module Identifier_profile = struct
  type t =
    | Ascii_compat
    | Unicode

  let to_string = function Ascii_compat -> "ascii-compat" | Unicode -> "unicode"

  let current_ref = ref Unicode

  let set profile = current_ref := profile
  let current () = !current_ref

  let is_start code_point =
    match !current_ref with
    | Ascii_compat ->
        code_point < 0x80 && Unicode_tables.is_xid_start code_point
    | Unicode -> Unicode_tables.is_xid_start code_point

  let is_continue code_point =
    match !current_ref with
    | Ascii_compat ->
        code_point < 0x80 && Unicode_tables.is_xid_continue code_point
    | Unicode -> Unicode_tables.is_xid_continue code_point
end

type identifier_profile = Identifier_profile.t

let set_identifier_profile profile = Identifier_profile.set profile
let current_identifier_profile () = Identifier_profile.current ()
let identifier_profile_to_string profile = Identifier_profile.to_string profile

let current_trivia_profile_ref = ref Trivia_profile.strict_json

let set_trivia_profile profile = current_trivia_profile_ref := profile
let current_trivia_profile () = !current_trivia_profile_ref

let has_hash_inline () =
  let profile = !current_trivia_profile_ref in
  profile.Trivia_profile.hash_inline

let shebang_enabled () =
  let profile = !current_trivia_profile_ref in
  profile.Trivia_profile.shebang

let shebang_applicable lexbuf =
  shebang_enabled () && Lexing.lexeme_start lexbuf = 0

let block_nested_enabled () =
  let profile = !current_trivia_profile_ref in
  let rec find = function
    | [] -> false
    | pair :: rest ->
        if pair.Trivia_profile.start = "/*"
           && pair.Trivia_profile.stop = "*/"
        then pair.Trivia_profile.nested
        else find rest
  in
  find profile.Trivia_profile.block

exception Lexer_error of string * Ast.span

(* 位置情報の追跡 *)
let current_pos lexbuf =
  Lexing.lexeme_start lexbuf

let make_span lexbuf =
  let start = Lexing.lexeme_start lexbuf in
  let end_ = Lexing.lexeme_end lexbuf in
  { Ast.start; end_ }

(* 文字列バッファ (文字列リテラル解析用) *)
let string_buffer = Buffer.create 256

(* エスケープシーケンス処理 *)
let escape_char = function
  | 'n' -> '\n'
  | 't' -> '\t'
  | 'r' -> '\r'
  | '\\' -> '\\'
  | '"' -> '"'
  | '\'' -> '\''
  | '0' -> '\000'
  | c -> c  (* 未対応エスケープはそのまま *)

(* UTF-8 デコードユーティリティ *)
exception Invalid_utf8_sequence of int

let decode_utf8 text index =
  let length = String.length text in
  let byte idx =
    if idx >= length then raise (Invalid_utf8_sequence index)
    else Char.code text.[idx]
  in
  let ensure_continuation b =
    if b land 0xC0 <> 0x80 then raise (Invalid_utf8_sequence index)
  in
  let first = byte index in
  if first land 0x80 = 0 then (first, index + 1)
  else if first land 0xE0 = 0xC0 then (
    let b1 = byte (index + 1) in
    ensure_continuation b1;
    if first < 0xC2 then raise (Invalid_utf8_sequence index);
    let code =
      ((first land 0x1F) lsl 6) lor (b1 land 0x3F)
    in
    (code, index + 2))
  else if first land 0xF0 = 0xE0 then (
    let b1 = byte (index + 1) in
    let b2 = byte (index + 2) in
    ensure_continuation b1;
    ensure_continuation b2;
    if first = 0xE0 && b1 < 0xA0 then raise (Invalid_utf8_sequence index);
    if first = 0xED && b1 >= 0xA0 then raise (Invalid_utf8_sequence index);
    let code =
      ((first land 0x0F) lsl 12)
      lor ((b1 land 0x3F) lsl 6)
      lor (b2 land 0x3F)
    in
    (code, index + 3))
  else if first land 0xF8 = 0xF0 then (
    if first > 0xF4 then raise (Invalid_utf8_sequence index);
    let b1 = byte (index + 1) in
    let b2 = byte (index + 2) in
    let b3 = byte (index + 3) in
    ensure_continuation b1;
    ensure_continuation b2;
    ensure_continuation b3;
    if first = 0xF0 && b1 < 0x90 then raise (Invalid_utf8_sequence index);
    if first = 0xF4 && b1 > 0x8F then raise (Invalid_utf8_sequence index);
    let code =
      ((first land 0x07) lsl 18)
      lor ((b1 land 0x3F) lsl 12)
      lor ((b2 land 0x3F) lsl 6)
      lor (b3 land 0x3F)
    in
    if code > 0x10FFFF then raise (Invalid_utf8_sequence index);
    (code, index + 4))
  else raise (Invalid_utf8_sequence index)

type identifier_validation =
  | Identifier_valid
  | Identifier_invalid_start of int
  | Identifier_invalid_continue of int

let validate_identifier_bytes text =
  let length = String.length text in
  let decode index =
    if index >= length then None
    else
      let code_point, next = decode_utf8 text index in
      Some (code_point, next)
  in
  match decode 0 with
  | None -> Identifier_invalid_start 0
  | Some (first, next_index) ->
      if not (Identifier_profile.is_start first) then
        Identifier_invalid_start first
      else
        let rec loop idx =
          match decode idx with
          | None -> Identifier_valid
          | Some (code_point, next_idx) ->
              if Identifier_profile.is_continue code_point then loop next_idx
              else Identifier_invalid_continue code_point
        in
        loop next_index

let validate_identifier lexbuf identifier_text =
  try
    match validate_identifier_bytes identifier_text with
    | Identifier_valid -> ()
    | Identifier_invalid_start code_point ->
        let span = make_span lexbuf in
        let profile = identifier_profile_to_string (current_identifier_profile ()) in
        let message =
          Printf.sprintf
            "識別子の先頭に使用できないコードポイント U+%04X (profile=%s)"
            code_point profile
        in
        raise (Lexer_error (message, span))
    | Identifier_invalid_continue code_point ->
        let span = make_span lexbuf in
        let profile = identifier_profile_to_string (current_identifier_profile ()) in
        let message =
          Printf.sprintf
            "識別子に使用できないコードポイント U+%04X (profile=%s)"
            code_point profile
        in
        raise (Lexer_error (message, span))
  with Invalid_utf8_sequence _ ->
    let span = make_span lexbuf in
    raise (Lexer_error ("無効な UTF-8 シーケンスです", span))

}

(* 正規表現定義 *)

let whitespace = [' ' '\t']+
let newline = '\r'? '\n' | '\r'
let digit = ['0'-'9']
let hex_digit = ['0'-'9' 'a'-'f' 'A'-'F']
let oct_digit = ['0'-'7']
let bin_digit = ['0'-'1']

(* Unicode XID 準拠の識別子（UTF-8 シーケンスを許可し、識別子検証でフィルタ） *)
let utf8_cont = ['\128'-'\191']
let utf8_2 = ['\194'-'\223'] utf8_cont
let utf8_3 = ['\224'-'\239'] utf8_cont utf8_cont
let utf8_4 = ['\240'-'\244'] utf8_cont utf8_cont utf8_cont
let unicode_scalar = utf8_2 | utf8_3 | utf8_4
let xid_start = ['a'-'z' 'A'-'Z' '_'] | unicode_scalar
let xid_continue = ['a'-'z' 'A'-'Z' '0'-'9' '_'] | unicode_scalar
let ident = xid_start xid_continue*

(* 整数リテラル *)
let dec_int = digit (digit | '_')*
let bin_int = "0b" bin_digit (bin_digit | '_')*
let oct_int = "0o" oct_digit (oct_digit | '_')*
let hex_int = "0x" hex_digit (hex_digit | '_')*

(* 浮動小数リテラル *)
let exponent = ['e' 'E'] ['+' '-']? digit+
let frac_part = digit (digit | '_')*
let float_lit = dec_int '.' frac_part? exponent?
              | dec_int exponent

(* メイントークナイザ *)
rule token = parse
  | whitespace              {
      let start_pos = Lexing.lexeme_start_p lexbuf in
      let end_pos = Lexing.lexeme_end_p lexbuf in
      Lex_record.consume ~kind:Lex_record.Space ~start_pos ~end_pos ();
      token lexbuf
    }
  | newline                 {
      let start_pos = Lexing.lexeme_start_p lexbuf in
      let end_pos = Lexing.lexeme_end_p lexbuf in
      Lex_record.consume ~kind:Lex_record.Newline ~start_pos ~end_pos ();
      Lexing.new_line lexbuf;
      token lexbuf
    }

  (* コメント *)
  | "#!" [^ '\r' '\n']* {
      if shebang_applicable lexbuf then (
        let start_pos = Lexing.lexeme_start_p lexbuf in
        let end_pos = Lexing.lexeme_end_p lexbuf in
        Lex_record.consume ~kind:Lex_record.Shebang ~start_pos ~end_pos ();
        token lexbuf
      )
      else
        let span = make_span lexbuf in
        raise (Lexer_error ("Unexpected character: " ^ String.make 1 '#', span))
    }
  | "#" [^ '\r' '\n']* {
      if has_hash_inline () then (
        let start_pos = Lexing.lexeme_start_p lexbuf in
        let end_pos = Lexing.lexeme_end_p lexbuf in
        Lex_record.consume ~kind:Lex_record.Hash_inline ~start_pos ~end_pos ();
        token lexbuf
      )
      else
        let span = make_span lexbuf in
        raise (Lexer_error ("Unexpected character: " ^ String.make 1 '#', span))
    }
  | "//" [^ '\r' '\n']*     {
      let start_pos = Lexing.lexeme_start_p lexbuf in
      let end_pos = Lexing.lexeme_end_p lexbuf in
      Lex_record.consume ~kind:Lex_record.Line_comment ~start_pos ~end_pos ();
      token lexbuf
    }
  | "/*"                    {
      let start_pos = Lexing.lexeme_start_p lexbuf in
      let end_pos = Lexing.lexeme_end_p lexbuf in
      Lex_record.consume ~kind:Lex_record.Block_comment ~start_pos ~end_pos ();
      block_comment (block_nested_enabled ()) 1 lexbuf
    }

  (* 演算子・区切り (長いものから優先) *)
  | "|>"        { PIPE }
  | "~>"        { CHANNEL_PIPE }
  | "->"        { ARROW }
  | "=>"        { DARROW }
  | ":="        { COLONEQ }
  | "=="        { EQEQ }
  | "!="        { NE }
  | "<="        { LE }
  | ">="        { GE }
  | "&&"        { AND }
  | "||"        { OR }
  | '|'         { BAR }
  | ".."        { DOTDOT }
  | '.'         { DOT }
  | ','         { COMMA }
  | ';'         { SEMICOLON }
  | ':'         { COLON }
  | '@'         { AT }
  | '='         { EQ }
  | '('         { LPAREN }
  | ')'         { RPAREN }
  | '['         { LBRACKET }
  | ']'         { RBRACKET }
  | '{'         { LBRACE }
  | '}'         { RBRACE }
  | '+'         { PLUS }
  | '-'         { MINUS }
  | '*'         { STAR }
  | '/'         { SLASH }
  | '%'         { PERCENT }
  | '^'         { POW }
  | '<'         { LT }
  | '>'         { GT }
  | '!'         { NOT }
  | '?'         { QUESTION }

  (* 整数リテラル *)
  | bin_int as s  {
      let num = String.sub s 2 (String.length s - 2) in
      let clean = String.concat "" (String.split_on_char '_' num) in
      INT (clean, Ast.Base2)
    }
  | oct_int as s  {
      let num = String.sub s 2 (String.length s - 2) in
      let clean = String.concat "" (String.split_on_char '_' num) in
      INT (clean, Ast.Base8)
    }
  | hex_int as s  {
      let num = String.sub s 2 (String.length s - 2) in
      let clean = String.concat "" (String.split_on_char '_' num) in
      INT (clean, Ast.Base16)
    }
  | dec_int as s  {
      let clean = String.concat "" (String.split_on_char '_' s) in
      INT (clean, Ast.Base10)
    }

  (* 浮動小数リテラル *)
  | float_lit as s  {
      let clean = String.concat "" (String.split_on_char '_' s) in
      FLOAT clean
    }

  (* 文字リテラル *)
  | '\'' ([^ '\\' '\''] as c) '\''  { CHAR (String.make 1 c) }
  | '\'' '\\' (['n' 't' 'r' '\\' '\'' '"' '0'] as c) '\''  {
      CHAR (String.make 1 (escape_char c))
    }

  (* 文字列リテラル *)
  | '"'       { Buffer.clear string_buffer; string_normal lexbuf }
  | "r\""     { Buffer.clear string_buffer; string_raw lexbuf }
  | "\"\"\""  { Buffer.clear string_buffer; string_multiline lexbuf }

  (* 識別子とキーワード *)
  | '_'         { UNDERSCORE }
  | ident as s  {
      validate_identifier lexbuf s;
      keyword_or_ident s
    }

  (* EOF *)
  | eof       { EOF }

  (* エラー *)
  | _ as c    {
      let span = make_span lexbuf in
      raise (Lexer_error ("Unexpected character: " ^ String.make 1 c, span))
    }

(* ブロックコメント (入れ子対応) *)
and block_comment nested depth = parse
  | "/*"   {
      if nested then block_comment nested (depth + 1) lexbuf
      else block_comment nested depth lexbuf
    }
  | "*/"   {
      if depth = 1 then token lexbuf
      else block_comment nested (depth - 1) lexbuf
    }
  | newline { Lexing.new_line lexbuf; block_comment nested depth lexbuf }
  | eof    {
      let span = make_span lexbuf in
      raise (Lexer_error ("Unterminated block comment", span))
    }
  | _      { block_comment nested depth lexbuf }

(* 通常文字列 (エスケープ処理あり) *)
and string_normal = parse
  | '"'    { STRING (Buffer.contents string_buffer, Ast.Normal) }
  | '\\' 'n'   { Buffer.add_char string_buffer '\n'; string_normal lexbuf }
  | '\\' 't'   { Buffer.add_char string_buffer '\t'; string_normal lexbuf }
  | '\\' 'r'   { Buffer.add_char string_buffer '\r'; string_normal lexbuf }
  | '\\' '\\' { Buffer.add_char string_buffer '\\'; string_normal lexbuf }
  | '\\' '"'   { Buffer.add_char string_buffer '"'; string_normal lexbuf }
  | '\\' '0'   { Buffer.add_char string_buffer '\000'; string_normal lexbuf }
  | newline {
      Lexing.new_line lexbuf;
      Buffer.add_char string_buffer '\n';
      string_normal lexbuf
    }
  | eof    {
      let span = make_span lexbuf in
      raise (Lexer_error ("Unterminated string", span))
    }
  | _ as c { Buffer.add_char string_buffer c; string_normal lexbuf }

(* 生文字列 (エスケープなし) *)
and string_raw = parse
  | '"'    { STRING (Buffer.contents string_buffer, Ast.Raw) }
  | newline {
      Lexing.new_line lexbuf;
      Buffer.add_char string_buffer '\n';
      string_raw lexbuf
    }
  | eof    {
      let span = make_span lexbuf in
      raise (Lexer_error ("Unterminated raw string", span))
    }
  | _ as c { Buffer.add_char string_buffer c; string_raw lexbuf }

(* 複数行文字列 *)
and string_multiline = parse
  | "\"\"\"" { STRING (Buffer.contents string_buffer, Ast.Multiline) }
  | newline {
      Lexing.new_line lexbuf;
      Buffer.add_char string_buffer '\n';
      string_multiline lexbuf
    }
  | eof    {
      let span = make_span lexbuf in
      raise (Lexer_error ("Unterminated multiline string", span))
    }
  | _ as c { Buffer.add_char string_buffer c; string_multiline lexbuf }

{
let read_token lexbuf =
  let token = token lexbuf in
  let start_pos = Lexing.lexeme_start_p lexbuf in
  let end_pos = Lexing.lexeme_end_p lexbuf in
  (token, start_pos, end_pos)
}
