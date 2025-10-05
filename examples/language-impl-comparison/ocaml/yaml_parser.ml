(* YAML風パーサー：インデント管理が重要な題材。

   対応する構文（簡易版）：
   - スカラー値: 文字列、数値、真偽値、null
   - リスト: `- item1`
   - マップ: `key: value`
   - ネストしたインデント構造

   インデント処理の特徴：
   - OCamlの関数型スタイルとパターンマッチを活用
   - エラー回復機能でインデントミスを報告しつつ継続 *)

module StringMap = Map.Make(String)

(* YAML値の表現。 *)
type yaml_value =
  | Scalar of string
  | List of yaml_value list
  | Map of yaml_value StringMap.t
  | Null

type document = yaml_value

(* パーサー型。 *)
type 'a parser = string -> int -> ('a * int) option

exception ParseError of string

(* パーサーヘルパー関数 *)

let peek input pos =
  if pos < String.length input then
    Some input.[pos]
  else
    None

let advance pos = pos + 1

let is_eof input pos = pos >= String.length input

let expect input pos expected =
  match peek input pos with
  | Some c when c = expected -> advance pos
  | _ -> raise (ParseError ("期待された文字 '" ^ String.make 1 expected ^ "' が見つかりません"))

let expect_string input pos expected =
  let len = String.length expected in
  if pos + len <= String.length input && String.sub input pos len = expected then
    pos + len
  else
    raise (ParseError ("期待された文字列 '" ^ expected ^ "' が見つかりません"))

(* 水平空白のみをスキップ（改行は含まない）。 *)
let rec hspace input pos =
  match peek input pos with
  | Some (' ' | '\t') -> hspace input (advance pos)
  | _ -> pos

(* 改行をスキップ。 *)
let newline input pos =
  match peek input pos with
  | Some '\n' -> advance pos
  | Some '\r' ->
    let pos' = advance pos in
    (match peek input pos' with
    | Some '\n' -> advance pos'
    | _ -> pos')
  | _ -> pos

(* コメントのスキップ（`#` から行末まで）。 *)
let comment input pos =
  match peek input pos with
  | Some '#' ->
    let rec skip pos =
      match peek input pos with
      | Some '\n' | None -> pos
      | _ -> skip (advance pos)
    in skip (advance pos)
  | _ -> pos

(* 空行またはコメント行をスキップ。 *)
let blank_or_comment input pos =
  let pos = hspace input pos in
  let pos = comment input pos in
  newline input pos

(* 特定のインデントレベルを期待する。 *)
let expect_indent input pos level =
  let rec count_spaces pos n =
    match peek input pos with
    | Some ' ' -> count_spaces (advance pos) (n + 1)
    | _ -> (pos, n)
  in
  let (pos', spaces) = count_spaces pos 0 in
  if spaces = level then
    pos'
  else
    raise (ParseError ("インデント不一致: 期待 " ^ string_of_int level ^ ", 実際 " ^ string_of_int spaces))

(* 現在よりも深いインデントを検出。 *)
let deeper_indent input pos current =
  let rec count_spaces pos n =
    match peek input pos with
    | Some ' ' -> count_spaces (advance pos) (n + 1)
    | _ -> (pos, n)
  in
  let (pos', spaces) = count_spaces pos 0 in
  if spaces > current then
    (pos', spaces)
  else
    raise (ParseError ("深いインデントが期待されます: 現在 " ^ string_of_int current ^ ", 実際 " ^ string_of_int spaces))

(* スカラー値のパース（前方宣言）。 *)
let scalar_value input pos = None

(* YAML値のパース（前方宣言）。 *)
let rec parse_value input pos indent = None

(* スカラー値のパース実装。 *)
let scalar_value input pos =
  (* null *)
  if pos + 4 <= String.length input && String.sub input pos 4 = "null" then
    Some (Null, pos + 4)
  else if pos < String.length input && input.[pos] = '~' then
    Some (Null, advance pos)
  (* 真偽値 *)
  else if pos + 4 <= String.length input && String.sub input pos 4 = "true" then
    Some (Scalar "true", pos + 4)
  else if pos + 5 <= String.length input && String.sub input pos 5 = "false" then
    Some (Scalar "false", pos + 5)
  (* 数値（簡易実装） *)
  else
    let rec parse_number pos acc =
      match peek input pos with
      | Some ('0'..'9' as c) -> parse_number (advance pos) (acc ^ String.make 1 c)
      | _ -> if acc = "" then None else Some (Scalar acc, pos)
    in
    match parse_number pos "" with
    | Some result -> Some result
    | None ->
      (* 文字列（引用符付き） *)
      if pos < String.length input && input.[pos] = '"' then
        let rec parse_quoted pos acc =
          match peek input pos with
          | Some '"' -> Some (Scalar acc, advance pos)
          | Some c -> parse_quoted (advance pos) (acc ^ String.make 1 c)
          | None -> raise (ParseError "引用符が閉じられていません")
        in
        parse_quoted (advance pos) ""
      else
        (* 文字列（引用符なし：行末または `:` まで） *)
        let rec parse_unquoted pos acc =
          match peek input pos with
          | Some ('\n' | ':' | '#') -> Some (Scalar (String.trim acc), pos)
          | Some c -> parse_unquoted (advance pos) (acc ^ String.make 1 c)
          | None -> Some (Scalar (String.trim acc), pos)
        in
        parse_unquoted pos ""

(* リスト項目のパース（`- value` 形式）。 *)
and parse_list_item input pos indent =
  let pos = expect_indent input pos indent in
  let pos = expect input pos '-' in
  let pos = hspace input pos in
  parse_value input pos (indent + 2)

(* リスト全体のパース。 *)
and parse_list input pos indent =
  let rec parse_items pos acc =
    try
      match parse_list_item input pos indent with
      | Some (item, pos') ->
        let pos'' = (match peek input pos' with Some '\n' -> newline input pos' | _ -> pos') in
        parse_items pos'' (item :: acc)
      | None -> (List.rev acc, pos)
    with ParseError _ -> (List.rev acc, pos)
  in
  let (items, pos') = parse_items pos [] in
  if items = [] then
    None
  else
    Some (List items, pos')

(* マップのキーバリューペアのパース（`key: value` 形式）。 *)
and parse_map_entry input pos indent =
  let pos = expect_indent input pos indent in
  let rec parse_key pos acc =
    match peek input pos with
    | Some (':' | '\n') -> (String.trim acc, pos)
    | Some c -> parse_key (advance pos) (acc ^ String.make 1 c)
    | None -> (String.trim acc, pos)
  in
  let (key, pos) = parse_key pos "" in
  let pos = expect input pos ':' in
  let pos = hspace input pos in
  (* 同じ行に値があるか、次の行にネストされているか *)
  let (value, pos') =
    match peek input pos with
    | Some '\n' ->
      let pos' = newline input pos in
      (match parse_value input pos' (indent + 2) with
      | Some (v, p) -> (v, p)
      | None -> raise (ParseError "値が期待されます"))
    | _ ->
      (match parse_value input pos indent with
      | Some (v, p) -> (v, p)
      | None -> raise (ParseError "値が期待されます"))
  in
  Some ((key, value), pos')

(* マップ全体のパース。 *)
and parse_map input pos indent =
  let rec parse_entries pos acc =
    try
      match parse_map_entry input pos indent with
      | Some ((key, value), pos') ->
        let pos'' = (match peek input pos' with Some '\n' -> newline input pos' | _ -> pos') in
        parse_entries pos'' ((key, value) :: acc)
      | None -> (List.rev acc, pos)
    with ParseError _ -> (List.rev acc, pos)
  in
  let (entries, pos') = parse_entries pos [] in
  if entries = [] then
    None
  else
    let map = List.fold_left (fun m (k, v) -> StringMap.add k v m) StringMap.empty entries in
    Some (Map map, pos')

(* YAML値のパース（再帰的）実装。 *)
and parse_value input pos indent =
  match parse_list input pos indent with
  | Some result -> Some result
  | None ->
    match parse_map input pos indent with
    | Some result -> Some result
    | None -> scalar_value input pos

(* ドキュメント全体のパース。 *)
let document input =
  let rec skip_blanks pos =
    if is_eof input pos then pos
    else
      try
        let pos' = blank_or_comment input pos in
        if pos' = pos then pos else skip_blanks pos'
      with ParseError _ -> pos
  in
  let pos = skip_blanks 0 in
  match parse_value input pos 0 with
  | Some (doc, pos') ->
    let pos'' = skip_blanks pos' in
    if is_eof input pos'' then
      Some doc
    else
      raise (ParseError "ドキュメントの終端が期待されます")
  | None -> None

(* パブリックAPI：YAML文字列をパース。 *)
let parse_yaml input =
  try
    document input
  with ParseError msg ->
    print_endline ("パースエラー: " ^ msg);
    None

(* 簡易的なレンダリング（検証用）。 *)
let render_to_string doc =
  let rec render_value value indent =
    let indent_str = String.make indent ' ' in
    match value with
    | Scalar s -> s
    | Null -> "null"
    | List items ->
      String.concat "\n" (List.map (fun item -> indent_str ^ "- " ^ render_value item (indent + 2)) items)
    | Map entries ->
      let entries_list = StringMap.bindings entries in
      String.concat "\n" (List.map (fun (key, val_) ->
        match val_ with
        | Scalar _ | Null -> indent_str ^ key ^ ": " ^ render_value val_ 0
        | _ -> indent_str ^ key ^ ":\n" ^ render_value val_ (indent + 2)
      ) entries_list)
  in
  render_value doc 0

(* テスト例。 *)
let test_examples () =
  let examples = [
    ("simple_scalar", "hello");
    ("simple_list", "- item1\n- item2\n- item3");
    ("simple_map", "key1: value1\nkey2: value2");
    ("nested_map", "parent:\n  child1: value1\n  child2: value2");
    ("nested_list", "items:\n  - item1\n  - item2");
    ("mixed", "name: John\nage: 30\nhobbies:\n  - reading\n  - coding")
  ] in
  List.iter (fun (name, yaml_str) ->
    print_endline ("--- " ^ name ^ " ---");
    match parse_yaml yaml_str with
    | Some doc ->
      print_endline "パース成功:";
      print_endline (render_to_string doc)
    | None ->
      print_endline "パースエラー"
  ) examples

(* インデント処理の課題と解決策：

   1. **インデントレベルの追跡**
      - パーサー引数としてインデントレベルを渡す
      - OCamlの純粋関数型スタイルでパーサー状態を管理

   2. **エラー回復**
      - optionとexceptionでバックトラックを制御
      - ParseError例外で分かりやすいエラーメッセージを提供

   3. **空白の扱い**
      - hspaceで水平空白のみをスキップ（改行は構文の一部）
      - newlineでCR/LF/CRLFを正規化

   Remlとの比較：

   - **OCamlの利点**:
     - 強力なパターンマッチング
     - 効率的なコンパイル済みコード

   - **OCamlの課題**:
     - パーサーコンビネーターライブラリがRemlほど充実していない
     - 手動のバックトラック管理が煩雑

   - **Remlの利点**:
     - 字句レイヤの柔軟性により、インデント処理が自然に表現できる
     - cut/commitによるエラー品質の向上
     - recoverによる部分的なパース継続が可能 *)