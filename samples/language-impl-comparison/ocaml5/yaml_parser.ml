(* YAML風パーサー：インデント管理が重要な題材（OCaml 5効果版）。

   対応する構文（簡易版）：
   - スカラー値: 文字列、数値、真偽値、null
   - リスト: `- item1`
   - マップ: `key: value`
   - ネストしたインデント構造

   インデント処理の特徴：
   - OCaml 5の効果システムを活用したパーサー実装
   - エラー回復機能でインデントミスを報告しつつ継続 *)

module StringMap = Map.Make(String)

(* YAML値の表現。 *)
type yaml_value =
  | Scalar of string
  | List of yaml_value list
  | Map of yaml_value StringMap.t
  | Null

type document = yaml_value

(* パーサー効果の定義 *)
type _ Effect.t +=
  | ParseError : string -> 'a Effect.t
  | GetPos : int Effect.t
  | SetPos : int -> unit Effect.t
  | GetInput : string Effect.t

(* パーサーモナドの実装 *)
let peek () =
  let input = Effect.perform GetInput in
  let pos = Effect.perform GetPos in
  if pos < String.length input then
    Some input.[pos]
  else
    None

let advance () =
  let pos = Effect.perform GetPos in
  Effect.perform (SetPos (pos + 1))

let is_eof () =
  let input = Effect.perform GetInput in
  let pos = Effect.perform GetPos in
  pos >= String.length input

let expect expected =
  match peek () with
  | Some c when c = expected -> advance ()
  | _ -> Effect.perform (ParseError ("期待された文字 '" ^ String.make 1 expected ^ "' が見つかりません"))

let expect_string expected =
  let input = Effect.perform GetInput in
  let pos = Effect.perform GetPos in
  let len = String.length expected in
  if pos + len <= String.length input && String.sub input pos len = expected then
    Effect.perform (SetPos (pos + len))
  else
    Effect.perform (ParseError ("期待された文字列 '" ^ expected ^ "' が見つかりません"))

(* 水平空白のみをスキップ（改行は含まない）。 *)
let rec hspace () =
  match peek () with
  | Some (' ' | '\t') -> advance (); hspace ()
  | _ -> ()

(* 改行をスキップ。 *)
let newline () =
  match peek () with
  | Some '\n' -> advance ()
  | Some '\r' ->
    advance ();
    (match peek () with
    | Some '\n' -> advance ()
    | _ -> ())
  | _ -> ()

(* コメントのスキップ（`#` から行末まで）。 *)
let comment () =
  match peek () with
  | Some '#' ->
    advance ();
    let rec skip () =
      match peek () with
      | Some '\n' | None -> ()
      | _ -> advance (); skip ()
    in skip ()
  | _ -> ()

(* 空行またはコメント行をスキップ。 *)
let blank_or_comment () =
  hspace ();
  comment ();
  newline ()

(* 特定のインデントレベルを期待する。 *)
let expect_indent level =
  let rec count_spaces n =
    match peek () with
    | Some ' ' -> advance (); count_spaces (n + 1)
    | _ -> n
  in
  let spaces = count_spaces 0 in
  if spaces <> level then
    Effect.perform (ParseError ("インデント不一致: 期待 " ^ string_of_int level ^ ", 実際 " ^ string_of_int spaces))

(* 現在よりも深いインデントを検出。 *)
let deeper_indent current =
  let rec count_spaces n =
    match peek () with
    | Some ' ' -> advance (); count_spaces (n + 1)
    | _ -> n
  in
  let spaces = count_spaces 0 in
  if spaces <= current then
    Effect.perform (ParseError ("深いインデントが期待されます: 現在 " ^ string_of_int current ^ ", 実際 " ^ string_of_int spaces))
  else
    spaces

(* スカラー値のパース。 *)
let rec scalar_value () =
  let input = Effect.perform GetInput in
  let pos = Effect.perform GetPos in
  (* null *)
  if pos + 4 <= String.length input && String.sub input pos 4 = "null" then begin
    expect_string "null";
    Null
  end
  else if match peek () with Some '~' -> true | _ -> false then begin
    advance ();
    Null
  end
  (* 真偽値 *)
  else if pos + 4 <= String.length input && String.sub input pos 4 = "true" then begin
    expect_string "true";
    Scalar "true"
  end
  else if pos + 5 <= String.length input && String.sub input pos 5 = "false" then begin
    expect_string "false";
    Scalar "false"
  end
  (* 数値（簡易実装） *)
  else begin
    let rec parse_number acc =
      match peek () with
      | Some ('0'..'9' as c) -> advance (); parse_number (acc ^ String.make 1 c)
      | _ -> acc
    in
    let num_str = parse_number "" in
    if num_str <> "" then
      Scalar num_str
    else begin
      (* 文字列（引用符付き） *)
      if match peek () with Some '"' -> true | _ -> false then begin
        advance ();
        let rec parse_quoted acc =
          match peek () with
          | Some '"' -> advance (); Scalar acc
          | Some c -> advance (); parse_quoted (acc ^ String.make 1 c)
          | None -> Effect.perform (ParseError "引用符が閉じられていません")
        in
        parse_quoted ""
      end
      else begin
        (* 文字列（引用符なし：行末または `:` まで） *)
        let rec parse_unquoted acc =
          match peek () with
          | Some ('\n' | ':' | '#') -> Scalar (String.trim acc)
          | Some c -> advance (); parse_unquoted (acc ^ String.make 1 c)
          | None -> Scalar (String.trim acc)
        in
        parse_unquoted ""
      end
    end
  end

(* 前方宣言 *)
and parse_value indent = scalar_value ()

(* リスト項目のパース（`- value` 形式）。 *)
let parse_list_item indent =
  expect_indent indent;
  expect '-';
  hspace ();
  parse_value (indent + 2)

(* リスト全体のパース。 *)
let parse_list indent =
  let rec parse_items acc =
    let saved_pos = Effect.perform GetPos in
    try
      let item = parse_list_item indent in
      (match peek () with Some '\n' -> newline () | _ -> ());
      parse_items (item :: acc)
    with Effect.Unhandled (ParseError _, _) ->
      Effect.perform (SetPos saved_pos);
      List.rev acc
  in
  let items = parse_items [] in
  if items = [] then
    Effect.perform (ParseError "リストが空です")
  else
    List items

(* マップのキーバリューペアのパース（`key: value` 形式）。 *)
let parse_map_entry indent =
  expect_indent indent;
  let rec parse_key acc =
    match peek () with
    | Some (':' | '\n') -> String.trim acc
    | Some c -> advance (); parse_key (acc ^ String.make 1 c)
    | None -> String.trim acc
  in
  let key = parse_key "" in
  expect ':';
  hspace ();
  (* 同じ行に値があるか、次の行にネストされているか *)
  let value =
    match peek () with
    | Some '\n' ->
      newline ();
      parse_value (indent + 2)
    | _ -> parse_value indent
  in
  (key, value)

(* マップ全体のパース。 *)
let parse_map indent =
  let rec parse_entries acc =
    let saved_pos = Effect.perform GetPos in
    try
      let entry = parse_map_entry indent in
      (match peek () with Some '\n' -> newline () | _ -> ());
      parse_entries (entry :: acc)
    with Effect.Unhandled (ParseError _, _) ->
      Effect.perform (SetPos saved_pos);
      List.rev acc
  in
  let entries = parse_entries [] in
  if entries = [] then
    Effect.perform (ParseError "マップが空です")
  else
    let map = List.fold_left (fun m (k, v) -> StringMap.add k v m) StringMap.empty entries in
    Map map

(* YAML値のパース（再帰的）実装。 *)
let rec parse_value indent =
  let saved_pos = Effect.perform GetPos in
  try
    parse_list indent
  with Effect.Unhandled (ParseError _, _) ->
    Effect.perform (SetPos saved_pos);
    try
      parse_map indent
    with Effect.Unhandled (ParseError _, _) ->
      Effect.perform (SetPos saved_pos);
      scalar_value ()

(* ドキュメント全体のパース。 *)
let document () =
  let rec skip_blanks () =
    if is_eof () then ()
    else
      let saved_pos = Effect.perform GetPos in
      try
        blank_or_comment ();
        skip_blanks ()
      with Effect.Unhandled (ParseError _, _) ->
        Effect.perform (SetPos saved_pos)
  in
  skip_blanks ();
  let doc = parse_value 0 in
  skip_blanks ();
  if not (is_eof ()) then
    Effect.perform (ParseError "ドキュメントの終端が期待されます");
  doc

(* パーサー実行ハンドラー *)
let run_parser input =
  let state = ref 0 in
  match
    Effect.Deep.try_with document ()
      { effc = (fun (type a) (eff : a Effect.t) ->
          match eff with
          | GetInput -> Some (fun (k : (a, _) Effect.Deep.continuation) ->
              Effect.Deep.continue k input)
          | GetPos -> Some (fun k ->
              Effect.Deep.continue k !state)
          | SetPos pos -> Some (fun k ->
              state := pos;
              Effect.Deep.continue k ())
          | ParseError msg -> Some (fun k ->
              Error msg)
          | _ -> None) }
  with
  | doc -> Ok doc
  | exception Effect.Unhandled (ParseError msg, _) -> Error msg

(* パブリックAPI：YAML文字列をパース。 *)
let parse_yaml input =
  match run_parser input with
  | Ok doc -> Some doc
  | Error msg ->
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
      - OCaml 5の効果システムでパーサー状態を管理

   2. **エラー回復**
      - 効果ハンドラーでバックトラックを制御
      - ParseError効果で分かりやすいエラーメッセージを提供

   3. **空白の扱い**
      - hspaceで水平空白のみをスキップ（改行は構文の一部）
      - newlineでCR/LF/CRLFを正規化

   Remlとの比較：

   - **OCaml 5の利点**:
     - 効果システムによる型安全なパーサー実装
     - 代数的効果によるエラーハンドリングの柔軟性

   - **OCaml 5の課題**:
     - 効果システムがまだ実験的
     - パーサーコンビネーターライブラリがRemlほど充実していない

   - **Remlの利点**:
     - 字句レイヤの柔軟性により、インデント処理が自然に表現できる
     - cut/commitによるエラー品質の向上
     - recoverによる部分的なパース継続が可能 *)