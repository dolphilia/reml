(** JSON拡張版：コメント・トレーリングカンマ対応。

    標準JSONからの拡張点：
    1. コメント対応（[//] 行コメント、[/* */] ブロックコメント）
    2. トレーリングカンマ許可（配列・オブジェクトの最後の要素の後）
    3. より詳細なエラーメッセージ

    実用的な設定ファイル形式として：
    - [package.json] 風の設定ファイル
    - [.babelrc], [.eslintrc] など開発ツールの設定
    - VS Code の [settings.json]
*)

(* 型定義 *)

type json_value =
  | JNull
  | JBool of bool
  | JNumber of float
  | JString of string
  | JArray of json_value list
  | JObject of (string * json_value) list

type parse_error =
  | UnexpectedEOF
  | InvalidValue of string
  | UnclosedString
  | UnclosedBlockComment
  | ExpectedChar of char
  | InvalidNumber of string

type state = {
  input : string;
  pos : int;
}

(* パース *)

let skip_ws state =
  let len = String.length state.input in
  let rec loop pos =
    if pos >= len then pos
    else
      match state.input.[pos] with
      | ' ' | '\n' | '\t' | '\r' -> loop (pos + 1)
      | _ -> pos
  in
  { state with pos = loop state.pos }

let skip_line_comment state =
  let len = String.length state.input in
  let new_pos = state.pos + 2 in
  let rec loop pos =
    if pos >= len then pos
    else if state.input.[pos] = '\n' then pos + 1
    else loop (pos + 1)
  in
  { state with pos = loop new_pos }

let skip_block_comment state =
  let len = String.length state.input in
  let new_pos = state.pos + 2 in
  let rec loop pos =
    if pos + 1 >= len then None
    else if state.input.[pos] = '*' && state.input.[pos + 1] = '/' then Some (pos + 2)
    else loop (pos + 1)
  in
  match loop new_pos with
  | None -> Error UnclosedBlockComment
  | Some end_pos -> Ok { state with pos = end_pos }

let rec skip_whitespace_and_comments state =
  let state_after_ws = skip_ws state in
  if state_after_ws.pos >= String.length state_after_ws.input then
    Ok state_after_ws
  else if state_after_ws.pos + 1 < String.length state_after_ws.input then
    let ch1 = state_after_ws.input.[state_after_ws.pos] in
    let ch2 = state_after_ws.input.[state_after_ws.pos + 1] in
    if ch1 = '/' && ch2 = '/' then
      skip_whitespace_and_comments (skip_line_comment state_after_ws)
    else if ch1 = '/' && ch2 = '*' then
      match skip_block_comment state_after_ws with
      | Error e -> Error e
      | Ok st -> skip_whitespace_and_comments st
    else
      Ok state_after_ws
  else
    Ok state_after_ws

let parse_string state =
  let len = String.length state.input in
  let rec loop pos acc =
    if pos >= len then Error UnclosedString
    else
      match state.input.[pos] with
      | '"' -> Ok (JString acc, { state with pos = pos + 1 })
      | '\\' when pos + 1 < len ->
          let escaped = match state.input.[pos + 1] with
            | 'n' -> "\n"
            | 't' -> "\t"
            | 'r' -> "\r"
            | '\\' -> "\\"
            | '"' -> "\""
            | ch -> String.make 1 ch
          in
          loop (pos + 2) (acc ^ escaped)
      | ch -> loop (pos + 1) (acc ^ String.make 1 ch)
  in
  loop (state.pos + 1) ""

let parse_number state =
  let len = String.length state.input in
  let is_num_char ch =
    ch = '-' || ch = '+' || ch = '.' || ch = 'e' || ch = 'E' ||
    (ch >= '0' && ch <= '9')
  in
  let rec loop pos =
    if pos >= len then pos
    else if is_num_char state.input.[pos] then loop (pos + 1)
    else pos
  in
  let end_pos = loop state.pos in
  let num_str = String.sub state.input state.pos (end_pos - state.pos) in
  try
    let num = float_of_string num_str in
    Ok (JNumber num, { state with pos = end_pos })
  with Failure _ -> Error (InvalidNumber num_str)

let rec parse_value state =
  match skip_whitespace_and_comments state with
  | Error e -> Error e
  | Ok clean_state ->
      if clean_state.pos >= String.length clean_state.input then
        Error UnexpectedEOF
      else
        let remaining = String.sub clean_state.input clean_state.pos
          (String.length clean_state.input - clean_state.pos) in
        if String.length remaining >= 4 && String.sub remaining 0 4 = "null" then
          Ok (JNull, { clean_state with pos = clean_state.pos + 4 })
        else if String.length remaining >= 4 && String.sub remaining 0 4 = "true" then
          Ok (JBool true, { clean_state with pos = clean_state.pos + 4 })
        else if String.length remaining >= 5 && String.sub remaining 0 5 = "false" then
          Ok (JBool false, { clean_state with pos = clean_state.pos + 5 })
        else if remaining.[0] = '"' then
          parse_string clean_state
        else if remaining.[0] = '[' then
          parse_array clean_state
        else if remaining.[0] = '{' then
          parse_object clean_state
        else
          parse_number clean_state

and parse_array state =
  let state_after_bracket = { state with pos = state.pos + 1 } in
  match skip_whitespace_and_comments state_after_bracket with
  | Error e -> Error e
  | Ok clean_state ->
      if clean_state.pos < String.length clean_state.input &&
         clean_state.input.[clean_state.pos] = ']' then
        Ok (JArray [], { clean_state with pos = clean_state.pos + 1 })
      else
        parse_array_elements clean_state []

and parse_array_elements state acc =
  match parse_value state with
  | Error e -> Error e
  | Ok (value, state_after_value) ->
      match skip_whitespace_and_comments state_after_value with
      | Error e -> Error e
      | Ok clean_state ->
          let new_acc = acc @ [value] in
          if clean_state.pos >= String.length clean_state.input then
            Error UnexpectedEOF
          else
            match clean_state.input.[clean_state.pos] with
            | ',' ->
                let state_after_comma = { clean_state with pos = clean_state.pos + 1 } in
                (match skip_whitespace_and_comments state_after_comma with
                | Error e -> Error e
                | Ok state_after_ws ->
                    if state_after_ws.pos < String.length state_after_ws.input &&
                       state_after_ws.input.[state_after_ws.pos] = ']' then
                      (* トレーリングカンマ *)
                      Ok (JArray new_acc, { state_after_ws with pos = state_after_ws.pos + 1 })
                    else
                      parse_array_elements state_after_ws new_acc)
            | ']' ->
                Ok (JArray new_acc, { clean_state with pos = clean_state.pos + 1 })
            | _ ->
                Error (ExpectedChar ',')

and parse_object state =
  let state_after_brace = { state with pos = state.pos + 1 } in
  match skip_whitespace_and_comments state_after_brace with
  | Error e -> Error e
  | Ok clean_state ->
      if clean_state.pos < String.length clean_state.input &&
         clean_state.input.[clean_state.pos] = '}' then
        Ok (JObject [], { clean_state with pos = clean_state.pos + 1 })
      else
        parse_object_pairs clean_state []

and parse_object_pairs state acc =
  match parse_string state with
  | Error e -> Error e
  | Ok (JString key, state_after_key) ->
      (match skip_whitespace_and_comments state_after_key with
      | Error e -> Error e
      | Ok clean_state1 ->
          if clean_state1.pos >= String.length clean_state1.input ||
             clean_state1.input.[clean_state1.pos] <> ':' then
            Error (ExpectedChar ':')
          else
            let state_after_colon = { clean_state1 with pos = clean_state1.pos + 1 } in
            (match skip_whitespace_and_comments state_after_colon with
            | Error e -> Error e
            | Ok clean_state2 ->
                (match parse_value clean_state2 with
                | Error e -> Error e
                | Ok (value, state_after_value) ->
                    (match skip_whitespace_and_comments state_after_value with
                    | Error e -> Error e
                    | Ok clean_state3 ->
                        let new_acc = acc @ [(key, value)] in
                        if clean_state3.pos >= String.length clean_state3.input then
                          Error UnexpectedEOF
                        else
                          match clean_state3.input.[clean_state3.pos] with
                          | ',' ->
                              let state_after_comma = { clean_state3 with pos = clean_state3.pos + 1 } in
                              (match skip_whitespace_and_comments state_after_comma with
                              | Error e -> Error e
                              | Ok state_after_ws ->
                                  if state_after_ws.pos < String.length state_after_ws.input &&
                                     state_after_ws.input.[state_after_ws.pos] = '}' then
                                    (* トレーリングカンマ *)
                                    Ok (JObject new_acc, { state_after_ws with pos = state_after_ws.pos + 1 })
                                  else
                                    parse_object_pairs state_after_ws new_acc)
                          | '}' ->
                              Ok (JObject new_acc, { clean_state3 with pos = clean_state3.pos + 1 })
                          | _ ->
                              Error (ExpectedChar ',')))))
  | Ok _ -> Error (InvalidValue "オブジェクトのキーは文字列である必要があります")
  | Error e -> Error e

let parse input =
  let initial_state = { input; pos = 0 } in
  match skip_whitespace_and_comments initial_state with
  | Error e -> Error e
  | Ok st1 ->
      match parse_value st1 with
      | Error e -> Error e
      | Ok (value, st2) ->
          match skip_whitespace_and_comments st2 with
          | Error e -> Error e
          | Ok final_state ->
              if final_state.pos >= String.length final_state.input then
                Ok value
              else
                Error (InvalidValue "入力の終端に到達していません")

(* レンダリング *)

let rec render_to_string value indent_level =
  let indent = String.make (indent_level * 2) ' ' in
  let next_indent = String.make ((indent_level + 1) * 2) ' ' in
  match value with
  | JNull -> "null"
  | JBool true -> "true"
  | JBool false -> "false"
  | JNumber num -> string_of_float num
  | JString str -> "\"" ^ str ^ "\""
  | JArray items ->
      if items = [] then "[]"
      else
        let items_str =
          items
          |> List.map (fun item -> next_indent ^ render_to_string item (indent_level + 1))
          |> String.concat ",\n"
        in
        "[\n" ^ items_str ^ "\n" ^ indent ^ "]"
  | JObject pairs ->
      if pairs = [] then "{}"
      else
        let pairs_str =
          pairs
          |> List.map (fun (key, value) ->
            next_indent ^ "\"" ^ key ^ "\": " ^ render_to_string value (indent_level + 1))
          |> String.concat ",\n"
        in
        "{\n" ^ pairs_str ^ "\n" ^ indent ^ "}"

(* テスト *)

let test_extended_json () =
  let test_cases = [
    ("コメント対応", {|
{
  // これは行コメント
  "name": "test",
  /* これは
     ブロックコメント */
  "version": "1.0"
}
|});
    ("トレーリングカンマ", {|
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
|})
  ] in
  List.iter (fun (name, json_str) ->
    Printf.printf "--- %s ---\n" name;
    match parse json_str with
    | Ok value ->
        Printf.printf "パース成功:\n%s\n" (render_to_string value 0)
    | Error err ->
        Printf.printf "パースエラー: %s\n" (match err with
          | UnexpectedEOF -> "予期しないEOF"
          | InvalidValue msg -> "不正な値: " ^ msg
          | UnclosedString -> "文字列が閉じられていません"
          | UnclosedBlockComment -> "ブロックコメントが閉じられていません"
          | ExpectedChar ch -> "'" ^ String.make 1 ch ^ "' が必要です"
          | InvalidNumber str -> "不正な数値: " ^ str);
    Printf.printf "\n"
  ) test_cases