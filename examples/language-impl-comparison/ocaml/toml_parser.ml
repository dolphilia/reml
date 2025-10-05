(* TOML風パーサー：パーサーコンビネーター的なアプローチでTOML v1.0.0の簡易版を実装。

   対応する構文：
   - キーバリューペア: `key = "value"`
   - テーブル: `[section]`
   - 配列テーブル: `[[array_section]]`
   - データ型: 文字列、整数、浮動小数点、真偽値、配列、インラインテーブル
   - コメント: `# comment`

   OCamlの特徴を活かした実装：
   - バリアント型による明示的な型表現
   - パターンマッチによる構造的なパース処理
   - モジュールシステムでの名前空間管理
   - 例外による明確なエラーハンドリング *)

module StringMap = Map.Make(String)

(* TOML値の表現。 *)
type toml_value =
  | String of string
  | Integer of int
  | Float of float
  | Boolean of bool
  | Array of toml_value list
  | InlineTable of toml_value StringMap.t

(* TOMLテーブルの表現。 *)
type toml_table = toml_value StringMap.t

(* TOMLドキュメント全体の構造。 *)
type toml_document = {
  root: toml_table;
  tables: (string list * toml_table) list  (* セクション名パス → テーブル *)
}

(* パーサー型。 *)
type 'a parser = string -> int -> ('a * int) option

exception ParseError of string * int  (* エラーメッセージと位置 *)

(* === パーサーヘルパー関数 === *)

let peek input pos =
  if pos < String.length input then
    Some input.[pos]
  else
    None

let advance pos = pos + 1

let is_eof input pos = pos >= String.length input

let is_whitespace = function
  | ' ' | '\t' -> true
  | _ -> false

let is_newline = function
  | '\n' | '\r' -> true
  | _ -> false

let is_alpha = function
  | 'a'..'z' | 'A'..'Z' -> true
  | _ -> false

let is_digit = function
  | '0'..'9' -> true
  | _ -> false

let is_alphanumeric c =
  is_alpha c || is_digit c

let is_bare_key_char = function
  | 'a'..'z' | 'A'..'Z' | '0'..'9' | '-' | '_' -> true
  | _ -> false

(* 水平空白をスキップ。 *)
let rec skip_whitespace input pos =
  match peek input pos with
  | Some c when is_whitespace c -> skip_whitespace input (advance pos)
  | _ -> pos

(* 改行をスキップ。 *)
let skip_newline input pos =
  match peek input pos with
  | Some '\n' -> advance pos
  | Some '\r' ->
    let pos' = advance pos in
    (match peek input pos' with
    | Some '\n' -> advance pos'
    | _ -> pos')
  | _ -> pos

(* コメントをスキップ（`#` から行末まで）。 *)
let skip_comment input pos =
  match peek input pos with
  | Some '#' ->
    let rec skip_until_newline p =
      match peek input p with
      | Some '\n' | Some '\r' | None -> p
      | _ -> skip_until_newline (advance p)
    in
    skip_until_newline (advance pos)
  | _ -> pos

(* 空白、コメント、改行をスキップ。 *)
let rec skip_space_and_comments input pos =
  let pos' = skip_whitespace input pos in
  let pos'' = skip_comment input pos' in
  if pos'' > pos' then
    skip_space_and_comments input pos''
  else if pos' < String.length input && is_newline input.[pos'] then
    skip_space_and_comments input (skip_newline input pos')
  else
    pos'

(* 特定の文字を期待。 *)
let expect_char input pos expected =
  match peek input pos with
  | Some c when c = expected -> advance pos
  | _ -> raise (ParseError ("期待された文字 '" ^ String.make 1 expected ^ "' が見つかりません", pos))

(* 特定の文字列を期待。 *)
let expect_string input pos expected =
  let len = String.length expected in
  if pos + len <= String.length input && String.sub input pos len = expected then
    pos + len
  else
    raise (ParseError ("期待された文字列 '" ^ expected ^ "' が見つかりません", pos))

(* === キー名のパース === *)

(* ベアキー（英数字・`-`・`_`のみ）。 *)
let parse_bare_key input pos =
  let rec collect p acc =
    match peek input p with
    | Some c when is_bare_key_char c ->
      collect (advance p) (acc ^ String.make 1 c)
    | _ -> (acc, p)
  in
  match peek input pos with
  | Some c when is_bare_key_char c ->
    let (key, pos') = collect pos "" in
    if key = "" then None else Some (key, pos')
  | _ -> None

(* 引用符付きキー（基本文字列）。 *)
let parse_quoted_key input pos =
  if pos < String.length input && input.[pos] = '"' then
    let rec collect p acc =
      match peek input p with
      | Some '"' -> Some (acc, advance p)
      | Some '\\' ->
        (match peek input (advance p) with
        | Some c -> collect (advance (advance p)) (acc ^ String.make 1 c)
        | None -> raise (ParseError ("エスケープシーケンスが不完全です", p)))
      | Some c -> collect (advance p) (acc ^ String.make 1 c)
      | None -> raise (ParseError ("引用符が閉じられていません", p))
    in
    collect (advance pos) ""
  else
    None

(* キー名のパース（ベアキーまたは引用符付き）。 *)
let parse_key input pos =
  let pos' = skip_whitespace input pos in
  match parse_quoted_key input pos' with
  | Some result -> Some result
  | None -> parse_bare_key input pos'

(* ドット区切りキーパス（例：`section.subsection.key`）。 *)
let parse_key_path input pos =
  let rec parse_rest p acc =
    let p' = skip_whitespace input p in
    match peek input p' with
    | Some '.' ->
      let p'' = skip_whitespace input (advance p') in
      (match parse_key input p'' with
      | Some (key, p''') ->
        parse_rest p''' (key :: acc)
      | None -> (List.rev acc, p))
    | _ -> (List.rev acc, p')
  in
  match parse_key input pos with
  | Some (first_key, pos') ->
    let (rest_keys, final_pos) = parse_rest pos' [first_key] in
    Some (rest_keys, final_pos)
  | None -> None

(* === TOML値のパース === *)

(* 文字列値のパース（基本文字列のみ実装）。 *)
let parse_string_value input pos =
  if pos < String.length input && input.[pos] = '"' then
    let rec collect p acc =
      match peek input p with
      | Some '"' -> Some (String acc, advance p)
      | Some '\\' ->
        (match peek input (advance p) with
        | Some 'n' -> collect (advance (advance p)) (acc ^ "\n")
        | Some 't' -> collect (advance (advance p)) (acc ^ "\t")
        | Some '\\' -> collect (advance (advance p)) (acc ^ "\\")
        | Some '"' -> collect (advance (advance p)) (acc ^ "\"")
        | Some c -> collect (advance (advance p)) (acc ^ String.make 1 c)
        | None -> raise (ParseError ("エスケープシーケンスが不完全です", p)))
      | Some c -> collect (advance p) (acc ^ String.make 1 c)
      | None -> raise (ParseError ("引用符が閉じられていません", p))
    in
    collect (advance pos) ""
  else
    None

(* 整数値のパース。 *)
let parse_integer_value input pos =
  let is_sign = function '+' | '-' -> true | _ -> false in
  let start_pos = pos in
  let (has_sign, pos') =
    match peek input pos with
    | Some c when is_sign c -> (true, advance pos)
    | _ -> (false, pos)
  in
  let rec collect p acc =
    match peek input p with
    | Some c when is_digit c -> collect (advance p) (acc ^ String.make 1 c)
    | Some '_' -> collect (advance p) acc  (* アンダースコアは無視 *)
    | _ -> (acc, p)
  in
  let (digits, final_pos) = collect pos' "" in
  if digits = "" then
    None
  else
    let sign_str = if has_sign then String.sub input start_pos 1 else "" in
    let num_str = sign_str ^ digits in
    try
      Some (Integer (int_of_string num_str), final_pos)
    with Failure _ ->
      raise (ParseError ("整数値の変換に失敗しました: " ^ num_str, start_pos))

(* 浮動小数点値のパース。 *)
let parse_float_value input pos =
  let is_sign = function '+' | '-' -> true | _ -> false in
  let start_pos = pos in
  let (has_sign, pos') =
    match peek input pos with
    | Some c when is_sign c -> (true, advance pos)
    | _ -> (false, pos)
  in
  let rec collect_number p acc has_dot =
    match peek input p with
    | Some c when is_digit c -> collect_number (advance p) (acc ^ String.make 1 c) has_dot
    | Some '_' -> collect_number (advance p) acc has_dot  (* アンダースコアは無視 *)
    | Some '.' when not has_dot -> collect_number (advance p) (acc ^ ".") true
    | Some ('e' | 'E') ->
      let acc' = acc ^ "e" in
      let p' = advance p in
      (match peek input p' with
      | Some ('+' | '-' as s) -> collect_number (advance p') (acc' ^ String.make 1 s) has_dot
      | _ -> collect_number p' acc' has_dot)
    | _ -> (acc, p, has_dot)
  in
  let (num_str, final_pos, has_dot) = collect_number pos' "" false in
  if num_str = "" || not has_dot then
    None
  else
    let sign_str = if has_sign then String.sub input start_pos 1 else "" in
    let full_str = sign_str ^ num_str in
    try
      Some (Float (float_of_string full_str), final_pos)
    with Failure _ ->
      raise (ParseError ("浮動小数点値の変換に失敗しました: " ^ full_str, start_pos))

(* 真偽値のパース。 *)
let parse_boolean_value input pos =
  if pos + 4 <= String.length input && String.sub input pos 4 = "true" then
    Some (Boolean true, pos + 4)
  else if pos + 5 <= String.length input && String.sub input pos 5 = "false" then
    Some (Boolean false, pos + 5)
  else
    None

(* 配列のパース（前方宣言）。 *)
let rec parse_array_value input pos =
  if pos < String.length input && input.[pos] = '[' then
    let pos' = skip_space_and_comments input (advance pos) in
    let rec collect p acc =
      let p' = skip_space_and_comments input p in
      match peek input p' with
      | Some ']' -> (List.rev acc, advance p')
      | _ ->
        (match parse_toml_value input p' with
        | Some (value, p'') ->
          let p''' = skip_space_and_comments input p'' in
          (match peek input p''' with
          | Some ',' ->
            collect (skip_space_and_comments input (advance p''')) (value :: acc)
          | Some ']' -> (List.rev (value :: acc), advance p''')
          | _ -> raise (ParseError ("配列内で ',' または ']' が期待されます", p''')))
        | None -> raise (ParseError ("配列の値が期待されます", p')))
    in
    let (values, final_pos) = collect pos' [] in
    Some (Array values, final_pos)
  else
    None

(* インラインテーブルのパース。 *)
and parse_inline_table input pos =
  if pos < String.length input && input.[pos] = '{' then
    let pos' = skip_space_and_comments input (advance pos) in
    let rec collect p acc =
      let p' = skip_space_and_comments input p in
      match peek input p' with
      | Some '}' -> (acc, advance p')
      | _ ->
        (match parse_key input p' with
        | Some (key, p'') ->
          let p''' = skip_space_and_comments input p'' in
          let p'''' = expect_char input p''' '=' in
          let p''''' = skip_space_and_comments input p'''' in
          (match parse_toml_value input p''''' with
          | Some (value, p'''''') ->
            let new_map = StringMap.add key value acc in
            let p'''''''= skip_space_and_comments input p'''''' in
            (match peek input p''''''' with
            | Some ',' ->
              collect (skip_space_and_comments input (advance p''''''')) new_map
            | Some '}' -> (new_map, advance p''''''')
            | _ -> raise (ParseError ("インラインテーブル内で ',' または '}' が期待されます", p''''''')))
          | None -> raise (ParseError ("インラインテーブルの値が期待されます", p''''')))
        | None -> raise (ParseError ("インラインテーブルのキーが期待されます", p')))
    in
    let (table, final_pos) = collect pos' StringMap.empty in
    Some (InlineTable table, final_pos)
  else
    None

(* TOML値のパース（再帰的）。 *)
and parse_toml_value input pos =
  let pos' = skip_whitespace input pos in
  (* 試行順序が重要：浮動小数点を整数より前に試す *)
  match parse_string_value input pos' with
  | Some result -> Some result
  | None ->
    match parse_boolean_value input pos' with
    | Some result -> Some result
    | None ->
      match parse_float_value input pos' with
      | Some result -> Some result
      | None ->
        match parse_integer_value input pos' with
        | Some result -> Some result
        | None ->
          match parse_array_value input pos' with
          | Some result -> Some result
          | None -> parse_inline_table input pos'

(* === ドキュメント要素のパース === *)

(* キーバリューペアのパース（`key = value`）。 *)
let parse_key_value_pair input pos =
  match parse_key_path input pos with
  | Some (path, pos') ->
    let pos'' = skip_whitespace input pos' in
    let pos''' = expect_char input pos'' '=' in
    let pos'''' = skip_space_and_comments input pos''' in
    (match parse_toml_value input pos'''' with
    | Some (value, final_pos) -> Some ((path, value), final_pos)
    | None -> raise (ParseError ("値が期待されます", pos'''')))
  | None -> None

(* テーブルヘッダーのパース（`[section.subsection]`）。 *)
let parse_table_header input pos =
  if pos < String.length input && input.[pos] = '[' then
    let pos' = advance pos in
    (* 配列テーブルではないことを確認 *)
    if pos' < String.length input && input.[pos'] = '[' then
      None
    else
      let pos'' = skip_whitespace input pos' in
      (match parse_key_path input pos'' with
      | Some (path, pos''') ->
        let pos'''' = skip_whitespace input pos''' in
        let final_pos = expect_char input pos'''' ']' in
        Some (path, final_pos)
      | None -> raise (ParseError ("テーブル名が期待されます", pos'')))
  else
    None

(* 配列テーブルヘッダーのパース（`[[array_section]]`）。 *)
let parse_array_table_header input pos =
  if pos + 1 < String.length input && input.[pos] = '[' && input.[pos + 1] = '[' then
    let pos' = advance (advance pos) in
    let pos'' = skip_whitespace input pos' in
    (match parse_key_path input pos'' with
    | Some (path, pos''') ->
      let pos'''' = skip_whitespace input pos''' in
      let pos''''' = expect_char input pos'''' ']' in
      let final_pos = expect_char input pos''''' ']' in
      Some (path, final_pos)
    | None -> raise (ParseError ("配列テーブル名が期待されます", pos'')))
  else
    None

(* ドキュメント要素の型。 *)
type document_element =
  | KeyValue of string list * toml_value
  | Table of string list
  | ArrayTable of string list

(* ドキュメント要素のパース。 *)
let parse_document_element input pos =
  let pos' = skip_space_and_comments input pos in
  if is_eof input pos' then
    None
  else
    (* 配列テーブルを先に試す（`[[` を `[` より先に判定）*)
    match parse_array_table_header input pos' with
    | Some (path, pos'') -> Some (ArrayTable path, pos'')
    | None ->
      match parse_table_header input pos' with
      | Some (path, pos'') -> Some (Table path, pos'')
      | None ->
        match parse_key_value_pair input pos' with
        | Some ((path, value), pos'') -> Some (KeyValue (path, value), pos'')
        | None -> None

(* === ネストしたキーパスに値を挿入する補助関数 === *)

let rec insert_nested table path value =
  match path with
  | [] -> table
  | [key] -> StringMap.add key value table
  | key :: rest ->
    let nested =
      match StringMap.find_opt key table with
      | Some (InlineTable t) -> t
      | _ -> StringMap.empty
    in
    let updated_nested = insert_nested nested rest value in
    StringMap.add key (InlineTable updated_nested) table

(* === ドキュメント全体のパース === *)

let parse_document input =
  let rec collect_elements pos acc =
    match parse_document_element input pos with
    | Some (elem, pos') ->
      let pos'' = skip_space_and_comments input pos' in
      collect_elements pos'' (elem :: acc)
    | None ->
      let pos' = skip_space_and_comments input pos in
      if is_eof input pos' then
        List.rev acc
      else
        raise (ParseError ("予期しない文字が見つかりました", pos'))
  in
  let elements = collect_elements 0 [] in

  (* 要素をグループ化してドキュメント構造を構築 *)
  let rec build_structure elems current_table_path root tables =
    match elems with
    | [] -> { root = root; tables = tables }
    | Table path :: rest ->
      (* テーブルを初期化（まだ存在しない場合） *)
      let tables' =
        if List.mem_assoc path tables then tables
        else (path, StringMap.empty) :: tables
      in
      build_structure rest path root tables'
    | ArrayTable path :: rest ->
      (* 簡易実装では通常テーブルと同じ扱い *)
      let tables' =
        if List.mem_assoc path tables then tables
        else (path, StringMap.empty) :: tables
      in
      build_structure rest path root tables'
    | KeyValue (path, value) :: rest ->
      if current_table_path = [] then
        (* ルートテーブルに追加 - 単一キーの場合は直接追加 *)
        let new_root =
          match path with
          | [key] -> StringMap.add key value root
          | _ -> insert_nested root path value
        in
        build_structure rest current_table_path new_root tables
      else
        (* 現在のテーブルに追加 *)
        let current_table =
          match List.assoc_opt current_table_path tables with
          | Some t -> t
          | None -> StringMap.empty
        in
        let updated_table =
          match path with
          | [key] -> StringMap.add key value current_table
          | _ -> insert_nested current_table path value
        in
        let new_tables =
          (current_table_path, updated_table) ::
          (List.filter (fun (p, _) -> p <> current_table_path) tables)
        in
        build_structure rest current_table_path root new_tables
  in
  build_structure elements [] StringMap.empty []

(* === パブリックAPI === *)

let parse input =
  try
    Some (parse_document input)
  with ParseError (msg, pos) ->
    print_endline ("パースエラー (位置 " ^ string_of_int pos ^ "): " ^ msg);
    None

(* === 簡易的なレンダリング（検証用） === *)

let render_to_string doc =
  let rec render_value = function
    | String s -> "\"" ^ String.escaped s ^ "\""
    | Integer n -> string_of_int n
    | Float f -> string_of_float f
    | Boolean b -> if b then "true" else "false"
    | Array items ->
      let items_str = String.concat ", " (List.map render_value items) in
      "[" ^ items_str ^ "]"
    | InlineTable entries ->
      let entries_list = StringMap.bindings entries in
      let entries_str = String.concat ", "
        (List.map (fun (k, v) -> k ^ " = " ^ render_value v) entries_list) in
      "{ " ^ entries_str ^ " }"
  in
  let rec render_table_entries table prefix =
    let entries = StringMap.bindings table in
    String.concat "" (List.map (fun (key, value) ->
      let full_key = if prefix = "" then key else prefix ^ "." ^ key in
      match value with
      | InlineTable nested ->
        render_table_entries nested full_key
      | _ -> full_key ^ " = " ^ render_value value ^ "\n"
    ) entries)
  in
  let root_str = render_table_entries doc.root "" in
  let tables_str = String.concat "\n" (List.map (fun (path, table) ->
    let section_name = String.concat "." path in
    "\n[" ^ section_name ^ "]\n" ^ render_table_entries table ""
  ) doc.tables) in
  root_str ^ tables_str

(* === テスト例 === *)

let test_toml () =
  let example_toml = {|# Reml パッケージ設定

[package]
name = "my_project"
version = "0.1.0"
authors = ["Author Name"]

[dependencies]
core = "1.0"

[dev-dependencies]
test_framework = "0.5"

[[plugins]]
name = "system"
version = "1.0"

[[plugins]]
name = "memory"
version = "1.0"
|} in

  print_endline "--- reml.toml 風設定のパース ---";
  match parse example_toml with
  | Some doc ->
    print_endline "パース成功:";
    print_endline (render_to_string doc)
  | None ->
    print_endline "パースエラー"

let test_examples () =
  let examples = [
    ("simple_key_value", {|key = "value"|});
    ("integer", {|number = 42|});
    ("float", {|pi = 3.14159|});
    ("boolean", {|enabled = true|});
    ("array", {|items = [1, 2, 3, "four"]|});
    ("inline_table", {|point = { x = 1, y = 2 }|});
    ("table", {|[section]
key1 = "value1"
key2 = 123|});
    ("nested_keys", {|parent.child.grandchild = "nested value"|});
    ("with_comments", {|# コメント
key = "value"  # インラインコメント|});
  ] in
  List.iter (fun (name, toml_str) ->
    print_endline ("--- " ^ name ^ " ---");
    match parse toml_str with
    | Some doc ->
      print_endline "パース成功:";
      print_endline (render_to_string doc)
    | None ->
      print_endline "パースエラー"
  ) examples

(* OCaml実装の特徴と課題：

   1. **型安全性**
      - バリアント型により値の種類を明示的に表現
      - パターンマッチで網羅性チェック
      - コンパイル時に多くのエラーを検出

   2. **パーサーの構造**
      - 関数型スタイルで状態を明示的に管理（位置をパラメータで渡す）
      - optionとexceptionでエラーハンドリングを分離
      - 再帰的なパース処理が自然に表現できる

   3. **モジュールシステム**
      - StringMapで効率的なテーブル管理
      - 名前空間の明確な分離

   4. **課題**
      - パーサーコンビネーターライブラリが標準でない
      - 手動のバックトラック管理が必要
      - エラーメッセージの質の維持に工夫が必要

   Remlとの比較：

   - **OCamlの利点**:
     - 成熟したコンパイラと最適化
     - 効率的なコンパイル済みコード
     - 強力な型システム

   - **Remlの利点**:
     - パーサーコンビネーターが標準ライブラリで充実
     - cut/commitによる明確なエラー位置特定
     - recoverによる部分的なパース継続
     - 期待集合による有用な診断メッセージ

   パフォーマンス考察：
   - OCamlはコンパイル言語として高速
   - ただし手動実装はライブラリより最適化が難しい
   - Remlは最適化されたパーサーコンビネーターを提供 *)