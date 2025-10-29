{
(* Lexer — Reml 字句解析器
 *
 * docs/spec/1-1-syntax.md §A に基づく字句解析を実装する。
 * Unicode XID 準拠、コメント処理、文字列エスケープなど。
 *)

open Token

module Trivia_profile = Parser_run_config.Lex.Trivia_profile

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

}

(* 正規表現定義 *)

let whitespace = [' ' '\t']+
let newline = '\r'? '\n' | '\r'
let digit = ['0'-'9']
let hex_digit = ['0'-'9' 'a'-'f' 'A'-'F']
let oct_digit = ['0'-'7']
let bin_digit = ['0'-'1']

(* Unicode XID 準拠の識別子
 * Phase 1 では簡易実装: ASCII + アンダースコアのみ
 * TODO: Phase 2 で Unicode XID 完全対応
 *)
let xid_start = ['a'-'z' 'A'-'Z' '_']
let xid_continue = ['a'-'z' 'A'-'Z' '0'-'9' '_']
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
  | whitespace              { token lexbuf }
  | newline                 { Lexing.new_line lexbuf; token lexbuf }

  (* コメント *)
  | "#!" [^ '\r' '\n']* {
      if shebang_applicable lexbuf then token lexbuf
      else
        let span = make_span lexbuf in
        raise (Lexer_error ("Unexpected character: " ^ String.make 1 '#', span))
    }
  | "#" [^ '\r' '\n']* {
      if has_hash_inline () then token lexbuf
      else
        let span = make_span lexbuf in
        raise (Lexer_error ("Unexpected character: " ^ String.make 1 '#', span))
    }
  | "//" [^ '\r' '\n']*     { token lexbuf }
  | "/*"                    { block_comment (block_nested_enabled ()) 1 lexbuf }

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
  | ident as s  { keyword_or_ident s }

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
