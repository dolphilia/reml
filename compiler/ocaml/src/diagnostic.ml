(* diagnostic.ml — 構文診断モデル（Phase 1 簡易版） *)

type severity = Error | Warning | Note

type expectation =
  | Token of string
  | Keyword of string
  | Rule of string
  | Eof
  | Custom of string

type location = {
  filename : string;
  line : int;
  column : int;
  offset : int;
}

type span = {
  start_pos : location;
  end_pos : location;
}

type t = {
  severity : severity;
  code : string option;
  message : string;
  span : span;
  expected : expectation list;
  notes : string list;
}

let severity_label = function
  | Error -> "エラー"
  | Warning -> "警告"
  | Note -> "注記"

let location_of_pos (pos : Lexing.position) : location =
  let column = pos.pos_cnum - pos.pos_bol + 1 in
  {
    filename = if pos.pos_fname = "" then "<入力>" else pos.pos_fname;
    line = pos.pos_lnum;
    column;
    offset = pos.pos_cnum;
  }

let span_of_positions start_pos end_pos =
  { start_pos = location_of_pos start_pos; end_pos = location_of_pos end_pos }

let make ?(severity = Error) ?code ?(expected = []) ?(notes = []) ~message ~start_pos ~end_pos () =
  {
    severity;
    code;
    message;
    span = span_of_positions start_pos end_pos;
    expected;
    notes;
  }

let of_lexer_error ~message ~start_pos ~end_pos =
  make ~message ~start_pos ~end_pos ()

let of_parser_error ~message ~start_pos ~end_pos ~expected =
  make ~message ~expected ~start_pos ~end_pos ()

let string_of_expectation = function
  | Token s -> Printf.sprintf "トークン '%s'" s
  | Keyword s -> Printf.sprintf "キーワード '%s'" s
  | Rule s -> Printf.sprintf "構文 '%s'" s
  | Eof -> "入力終端"
  | Custom s -> s

let format_location loc = Printf.sprintf "%s:%d:%d" loc.filename loc.line loc.column

let to_string diag =
  let loc = format_location diag.span.start_pos in
  let header = Printf.sprintf "%s: %s: %s" loc (severity_label diag.severity) diag.message in
  let expected_str =
    match diag.expected with
    | [] -> None
    | items ->
        let body = items |> List.map string_of_expectation |> String.concat ", " in
        Some ("期待される入力: " ^ body)
  in
  let notes_str =
    match diag.notes with
    | [] -> []
    | notes -> ["補足:" ^ String.concat " / " notes]
  in
  let parts =
    header :: (match expected_str with None -> [] | Some s -> [s]) @ notes_str
  in
  String.concat "\n" parts

