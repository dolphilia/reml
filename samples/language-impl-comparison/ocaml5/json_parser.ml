type json =
  | Null
  | Bool of bool
  | Number of float
  | String of string
  | Array of json list
  | Object of (string * json) list

exception Parse_error of string

type state = { source : string; mutable index : int }

let peek st =
  if st.index >= String.length st.source then None
  else Some st.source.[st.index]

let bump st =
  let c = peek st in
  st.index <- st.index + 1;
  c

let rec skip_ws st =
  match peek st with
  | Some (' ' | '\n' | '\r' | '\t') -> st.index <- st.index + 1; skip_ws st
  | _ -> ()

let expect st ch =
  match bump st with
  | Some c when c = ch -> ()
  | _ -> raise (Parse_error (Printf.sprintf "期待した文字 %c" ch))

let expect_literal st lit =
  String.iter (fun c -> expect st c) lit

let rec parse_value st =
  skip_ws st;
  match peek st with
  | Some 'n' -> expect_literal st "null"; Null
  | Some 't' -> expect_literal st "true"; Bool true
  | Some 'f' -> expect_literal st "false"; Bool false
  | Some '"' -> String (parse_string st)
  | Some '[' -> Array (parse_array st)
  | Some '{' -> Object (parse_object st)
  | Some ('-' | '0' .. '9') -> Number (parse_number st)
  | Some c -> raise (Parse_error (Printf.sprintf "想定外の文字 %c" c))
  | None -> raise (Parse_error "入力が途中で終了しました")

and parse_string st =
  expect st '"';
  let buf = Buffer.create 16 in
  let rec loop () =
    match bump st with
    | Some '"' -> Buffer.contents buf
    | Some '\\' ->
        begin match bump st with
        | Some '"' -> Buffer.add_char buf '"'
        | Some '\\' -> Buffer.add_char buf '\\'
        | Some '/' -> Buffer.add_char buf '/'
        | Some 'b' -> Buffer.add_char buf '\b'
        | Some 'f' -> Buffer.add_char buf '\012'
        | Some 'n' -> Buffer.add_char buf '\n'
        | Some 'r' -> Buffer.add_char buf '\r'
        | Some 't' -> Buffer.add_char buf '\t'
        | Some 'u' -> Buffer.add_string buf (parse_unicode st)
        | Some c -> raise (Parse_error (Printf.sprintf "不明なエスケープ %c" c))
        | None -> raise (Parse_error "エスケープが途中で終了しました")
        end; loop ()
    | Some c -> Buffer.add_char buf c; loop ()
    | None -> raise (Parse_error "文字列が閉じていません")
  in
  loop ()

and parse_unicode st =
  let code = ref 0 in
  for _ = 1 to 4 do
    match bump st with
    | Some c ->
        code := (!code lsl 4)
        + (match c with
          | '0' .. '9' -> Char.code c - Char.code '0'
          | 'a' .. 'f' -> 10 + Char.code c - Char.code 'a'
          | 'A' .. 'F' -> 10 + Char.code c - Char.code 'A'
          | _ -> raise (Parse_error "16 進数を期待しました"))
    | None -> raise (Parse_error "Unicode エスケープが途中で終了しました")
  done;
  let ch = Char.chr !code in
  String.make 1 ch

and parse_array st =
  expect st '[';
  skip_ws st;
  if peek st = Some ']' then (ignore (bump st); [])
  else
    let rec elements acc =
      let value = parse_value st in
      skip_ws st;
      match peek st with
      | Some ',' -> st.index <- st.index + 1; elements (value :: acc)
      | Some ']' -> st.index <- st.index + 1; List.rev (value :: acc)
      | Some _ -> raise (Parse_error "配列内の区切りが不正です")
      | None -> raise (Parse_error "配列が閉じていません")
    in
    elements []

and parse_object st =
  expect st '{';
  skip_ws st;
  if peek st = Some '}' then (ignore (bump st); [])
  else
    let rec members acc =
      let key = parse_string st in
      skip_ws st;
      expect st ':';
      let value = parse_value st in
      skip_ws st;
      match peek st with
      | Some ',' -> st.index <- st.index + 1; members ((key, value) :: acc)
      | Some '}' -> st.index <- st.index + 1; List.rev ((key, value) :: acc)
      | Some _ -> raise (Parse_error "オブジェクトの区切りが不正です")
      | None -> raise (Parse_error "オブジェクトが閉じていません")
    in
    members []

and parse_number st =
  let start = st.index in
  (match peek st with
  | Some '-' -> ignore (bump st)
  | _ -> ());
  let rec advance_digits () =
    match peek st with
    | Some ('0' .. '9') -> ignore (bump st); advance_digits ()
    | _ -> ()
  in
  advance_digits ();
  (match peek st with
  | Some '.' -> ignore (bump st); advance_digits ()
  | _ -> ());
  (match peek st with
  | Some ('e' | 'E') ->
      ignore (bump st);
      (match peek st with
      | Some ('+' | '-') -> ignore (bump st)
      | _ -> ());
      advance_digits ()
  | _ -> ());
  let literal = String.sub st.source start (st.index - start) in
  float_of_string literal

let parse source =
  let st = { source; index = 0 } in
  let value = parse_value st in
  skip_ws st;
  if st.index <> String.length source then raise (Parse_error "未消費文字が残っています")
  else value