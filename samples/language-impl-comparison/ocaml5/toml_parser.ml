(* TOML風設定ファイルパーサー：OCaml 5効果システム版。

   対応する構文（TOML v1.0.0準拠の簡易版）：
   - キーバリューペア: `key = "value"`
   - テーブル: `[section]`
   - 配列テーブル: `[[array_section]]`
   - データ型: 文字列、整数、浮動小数点、真偽値、配列、インラインテーブル
   - コメント: `# comment`

   OCaml 5の特徴を活かした実装：
   - 効果システムによる型安全なパーサー状態管理
   - 代数的効果によるエラーハンドリング
   - 並行処理を想定した設計（効果ハンドラーの合成可能性）

   エラー品質の特徴：
   - ParseError効果による明確なエラー位置特定
   - バックトラック機能による複数候補の試行
   - 詳細なエラーメッセージ生成 *)

module StringMap = Map.Make(String)

(* TOML値の表現。 *)
type toml_value =
  | String of string
  | Integer of int
  | Float of float
  | Boolean of bool
  | Array of toml_value list
  | InlineTable of toml_value StringMap.t

(* TOMLテーブル。 *)
type toml_table = toml_value StringMap.t

(* TOMLドキュメント構造。 *)
type toml_document = {
  root: toml_table;
  tables: (string list * toml_table) list  (* セクション名パス → テーブル *)
}

(* パーサー効果の定義 *)
type _ Effect.t +=
  | ParseError : string -> 'a Effect.t
  | GetPos : int Effect.t
  | SetPos : int -> unit Effect.t
  | GetInput : string Effect.t

(* パーサー基本操作 *)
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

(* 空白・コメントのスキップ。 *)
let rec hspace () =
  match peek () with
  | Some (' ' | '\t') -> advance (); hspace ()
  | _ -> ()

let newline () =
  match peek () with
  | Some '\n' -> advance ()
  | Some '\r' ->
    advance ();
    (match peek () with
    | Some '\n' -> advance ()
    | _ -> ())
  | _ -> ()

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

(* 空白・コメント・改行をスキップ（lexeme用）。 *)
let rec whitespace () =
  match peek () with
  | Some (' ' | '\t' | '\n' | '\r') ->
    (match peek () with
    | Some '\n' | Some '\r' -> newline ()
    | _ -> advance ());
    whitespace ()
  | Some '#' ->
    comment ();
    (match peek () with
    | Some '\n' | Some '\r' -> newline ()
    | _ -> ());
    whitespace ()
  | _ -> ()

let lexeme parser =
  let result = parser () in
  whitespace ();
  result

(* キー名のパース（識別子または引用符付き文字列）。 *)
let parse_key () =
  match peek () with
  | Some '"' ->
    advance ();
    let rec parse_quoted acc =
      match peek () with
      | Some '"' -> advance (); acc
      | Some '\\' ->
        advance ();
        (match peek () with
        | Some c -> advance (); parse_quoted (acc ^ String.make 1 c)
        | None -> Effect.perform (ParseError "引用符が閉じられていません"))
      | Some c -> advance (); parse_quoted (acc ^ String.make 1 c)
      | None -> Effect.perform (ParseError "引用符が閉じられていません")
    in
    parse_quoted ""
  | Some (('a'..'z' | 'A'..'Z' | '0'..'9' | '-' | '_') as c) ->
    advance ();
    let rec parse_bare acc =
      match peek () with
      | Some (('a'..'z' | 'A'..'Z' | '0'..'9' | '-' | '_') as c) ->
        advance (); parse_bare (acc ^ String.make 1 c)
      | _ -> acc
    in
    parse_bare (String.make 1 c)
  | _ -> Effect.perform (ParseError "キー名が期待されます")

let key () = lexeme parse_key

(* ドットで区切られたキーパス（例：`section.subsection.key`）。 *)
let key_path () =
  let rec parse_path acc =
    let k = key () in
    let new_acc = k :: acc in
    match peek () with
    | Some '.' -> advance (); whitespace (); parse_path new_acc
    | _ -> List.rev new_acc
  in
  parse_path []

(* 文字列値のパース（基本文字列）。 *)
let parse_string_value () =
  match peek () with
  | Some '"' ->
    advance ();
    let rec parse_quoted acc =
      match peek () with
      | Some '"' -> advance (); String acc
      | Some '\\' ->
        advance ();
        (match peek () with
        | Some 'n' -> advance (); parse_quoted (acc ^ "\n")
        | Some 't' -> advance (); parse_quoted (acc ^ "\t")
        | Some 'r' -> advance (); parse_quoted (acc ^ "\r")
        | Some '\\' -> advance (); parse_quoted (acc ^ "\\")
        | Some '"' -> advance (); parse_quoted (acc ^ "\"")
        | Some c -> advance (); parse_quoted (acc ^ String.make 1 c)
        | None -> Effect.perform (ParseError "引用符が閉じられていません"))
      | Some c -> advance (); parse_quoted (acc ^ String.make 1 c)
      | None -> Effect.perform (ParseError "引用符が閉じられていません")
    in
    parse_quoted ""
  | _ -> Effect.perform (ParseError "文字列が期待されます")

let string_value () = lexeme parse_string_value

(* 整数値のパース。 *)
let parse_integer_value () =
  let sign = ref 1 in
  (match peek () with
  | Some '-' -> advance (); sign := -1
  | Some '+' -> advance ()
  | _ -> ());
  let rec parse_digits acc =
    match peek () with
    | Some ('0'..'9' as c) -> advance (); parse_digits (acc ^ String.make 1 c)
    | Some '_' -> advance (); parse_digits acc  (* 区切り文字を無視 *)
    | _ -> acc
  in
  let num_str = parse_digits "" in
  if num_str = "" then
    Effect.perform (ParseError "整数が期待されます")
  else
    Integer (!sign * int_of_string num_str)

let integer_value () = lexeme parse_integer_value

(* 浮動小数点値のパース。 *)
let parse_float_value () =
  let sign = ref 1 in
  (match peek () with
  | Some '-' -> advance (); sign := -1
  | Some '+' -> advance ()
  | _ -> ());
  let rec parse_digits acc =
    match peek () with
    | Some ('0'..'9' as c) -> advance (); parse_digits (acc ^ String.make 1 c)
    | Some '_' -> advance (); parse_digits acc
    | _ -> acc
  in
  let int_part = parse_digits "" in
  if int_part = "" then
    Effect.perform (ParseError "数値が期待されます");
  let frac_part =
    match peek () with
    | Some '.' ->
      advance ();
      "." ^ parse_digits ""
    | _ -> ""
  in
  let exp_part =
    match peek () with
    | Some ('e' | 'E') ->
      advance ();
      let exp_sign = match peek () with
        | Some '-' -> advance (); "-"
        | Some '+' -> advance (); "+"
        | _ -> ""
      in
      "e" ^ exp_sign ^ parse_digits ""
    | _ -> ""
  in
  let num_str = int_part ^ frac_part ^ exp_part in
  Float (float_of_string num_str *. float_of_int !sign)

let float_value () = lexeme parse_float_value

(* 真偽値のパース。 *)
let parse_boolean_value () =
  let input = Effect.perform GetInput in
  let pos = Effect.perform GetPos in
  if pos + 4 <= String.length input && String.sub input pos 4 = "true" then begin
    expect_string "true";
    Boolean true
  end
  else if pos + 5 <= String.length input && String.sub input pos 5 = "false" then begin
    expect_string "false";
    Boolean false
  end
  else
    Effect.perform (ParseError "真偽値が期待されます")

let boolean_value () = lexeme parse_boolean_value

(* 前方宣言用 *)
let toml_value_ref = ref (fun () -> Effect.perform (ParseError "未実装"))

(* 配列のパース。 *)
let parse_array_value () =
  expect '[';
  whitespace ();
  let rec parse_items acc =
    if match peek () with Some ']' -> true | _ -> false then
      acc
    else begin
      let item = !toml_value_ref () in
      let new_acc = item :: acc in
      whitespace ();
      (match peek () with
      | Some ',' -> advance (); whitespace (); parse_items new_acc
      | Some ']' -> new_acc
      | _ -> Effect.perform (ParseError "配列の区切り ',' または終端 ']' が期待されます"))
    end
  in
  let items = parse_items [] in
  expect ']';
  Array (List.rev items)

let array_value () = lexeme parse_array_value

(* インラインテーブルのパース（`{ key = value, ... }`）。 *)
let parse_inline_table () =
  expect '{';
  whitespace ();
  let rec parse_entries acc =
    if match peek () with Some '}' -> true | _ -> false then
      acc
    else begin
      let k = key () in
      whitespace ();
      expect '=';
      whitespace ();
      let v = !toml_value_ref () in
      let new_acc = (k, v) :: acc in
      whitespace ();
      (match peek () with
      | Some ',' -> advance (); whitespace (); parse_entries new_acc
      | Some '}' -> new_acc
      | _ -> Effect.perform (ParseError "インラインテーブルの区切り ',' または終端 '}' が期待されます"))
    end
  in
  let entries = parse_entries [] in
  expect '}';
  let map = List.fold_left (fun m (k, v) -> StringMap.add k v m) StringMap.empty entries in
  InlineTable map

let inline_table () = lexeme parse_inline_table

(* TOML値のパース（再帰的）。 *)
let toml_value () =
  let saved_pos = Effect.perform GetPos in
  (* 文字列を試行 *)
  try
    string_value ()
  with Effect.Unhandled (ParseError _, _) ->
    Effect.perform (SetPos saved_pos);
    (* 真偽値を試行 *)
    try
      boolean_value ()
    with Effect.Unhandled (ParseError _, _) ->
      Effect.perform (SetPos saved_pos);
      (* 配列を試行 *)
      try
        array_value ()
      with Effect.Unhandled (ParseError _, _) ->
        Effect.perform (SetPos saved_pos);
        (* インラインテーブルを試行 *)
        try
          inline_table ()
        with Effect.Unhandled (ParseError _, _) ->
          Effect.perform (SetPos saved_pos);
          (* 浮動小数点を試行（整数より先） *)
          try
            let saved_pos2 = Effect.perform GetPos in
            let result = float_value () in
            (* 浮動小数点の後にドットがあるか確認 *)
            (match peek () with
            | Some '.' -> ()
            | _ -> ());
            result
          with Effect.Unhandled (ParseError _, _) ->
            Effect.perform (SetPos saved_pos);
            (* 整数を試行 *)
            integer_value ()

(* toml_value_ref に実装を設定 *)
let () = toml_value_ref := toml_value

(* キーバリューペアのパース（`key = value`）。 *)
let key_value_pair () =
  let path = key_path () in
  whitespace ();
  expect '=';
  whitespace ();
  let value = toml_value () in
  whitespace ();
  (path, value)

(* テーブルヘッダーのパース（`[section.subsection]`）。 *)
let table_header () =
  expect '[';
  whitespace ();
  let path = key_path () in
  whitespace ();
  expect ']';
  whitespace ();
  path

(* 配列テーブルヘッダーのパース（`[[array_section]]`）。 *)
let array_table_header () =
  expect '[';
  expect '[';
  whitespace ();
  let path = key_path () in
  whitespace ();
  expect ']';
  expect ']';
  whitespace ();
  path

(* ドキュメント要素（キーバリューペアまたはテーブル定義）。 *)
type document_element =
  | KeyValue of string list * toml_value
  | Table of string list
  | ArrayTable of string list

let document_element () =
  let saved_pos = Effect.perform GetPos in
  (* 配列テーブルヘッダーを試行 *)
  try
    let path = array_table_header () in
    ArrayTable path
  with Effect.Unhandled (ParseError _, _) ->
    Effect.perform (SetPos saved_pos);
    (* テーブルヘッダーを試行 *)
    try
      let path = table_header () in
      Table path
    with Effect.Unhandled (ParseError _, _) ->
      Effect.perform (SetPos saved_pos);
      (* キーバリューペアを試行 *)
      let (path, value) = key_value_pair () in
      KeyValue (path, value)

(* ネストしたキーパスに値を挿入する補助関数。 *)
let rec insert_nested table path value =
  match path with
  | [] -> table
  | [key] -> StringMap.add key value table
  | key :: rest ->
    let nested =
      try
        match StringMap.find key table with
        | InlineTable t -> t
        | _ -> StringMap.empty
      with Not_found -> StringMap.empty
    in
    let updated_nested = insert_nested nested rest value in
    StringMap.add key (InlineTable updated_nested) table

(* ドキュメント全体のパース。 *)
let document () =
  whitespace ();
  let rec parse_elements acc =
    if is_eof () then
      List.rev acc
    else begin
      let saved_pos = Effect.perform GetPos in
      try
        let elem = document_element () in
        parse_elements (elem :: acc)
      with Effect.Unhandled (ParseError msg, _) ->
        (* エラーを報告しつつ次の行にスキップ（回復処理） *)
        print_endline ("警告: " ^ msg ^ " (位置 " ^ string_of_int saved_pos ^ ")");
        (* 次の改行までスキップ *)
        let rec skip_to_newline () =
          match peek () with
          | Some '\n' -> newline (); whitespace ()
          | None -> ()
          | _ -> advance (); skip_to_newline ()
        in
        skip_to_newline ();
        parse_elements acc
    end
  in
  let elements = parse_elements [] in
  (* 要素をグループ化してドキュメント構造を構築 *)
  let current_table = ref [] in
  let root = ref StringMap.empty in
  let tables = ref [] in
  List.iter (fun elem ->
    match elem with
    | Table path ->
      current_table := path;
      if not (List.mem_assoc path !tables) then
        tables := (path, StringMap.empty) :: !tables
    | ArrayTable path ->
      current_table := path;
      if not (List.mem_assoc path !tables) then
        tables := (path, StringMap.empty) :: !tables
    | KeyValue (path, value) ->
      if !current_table = [] then
        (* ルートテーブルに追加 *)
        root := insert_nested !root path value
      else begin
        (* 現在のテーブルに追加 *)
        let table = try List.assoc !current_table !tables with Not_found -> StringMap.empty in
        let updated_table = insert_nested table path value in
        tables := (!current_table, updated_table) :: List.remove_assoc !current_table !tables
      end
  ) elements;
  { root = !root; tables = List.rev !tables }

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

(* パブリックAPI：TOML文字列をパース。 *)
let parse_toml input =
  match run_parser input with
  | Ok doc -> Some doc
  | Error msg ->
    print_endline ("パースエラー: " ^ msg);
    None

(* 簡易的なレンダリング（検証用）。 *)
let render_to_string doc =
  let rec render_value value =
    match value with
    | String s -> "\"" ^ String.escaped s ^ "\""
    | Integer n -> string_of_int n
    | Float f -> string_of_float f
    | Boolean b -> if b then "true" else "false"
    | Array items ->
      let items_str = String.concat ", " (List.map render_value items) in
      "[" ^ items_str ^ "]"
    | InlineTable entries ->
      let entries_list = StringMap.bindings entries in
      let entries_str = String.concat ", " (List.map (fun (k, v) ->
        k ^ " = " ^ render_value v
      ) entries_list) in
      "{ " ^ entries_str ^ " }"
  in
  let render_table table prefix =
    let entries = StringMap.bindings table in
    String.concat "" (List.map (fun (key, value) ->
      let full_key = if prefix = [] then key else String.concat "." (prefix @ [key]) in
      match value with
      | InlineTable nested ->
        let nested_entries = StringMap.bindings nested in
        String.concat "" (List.map (fun (k, v) ->
          full_key ^ "." ^ k ^ " = " ^ render_value v ^ "\n"
        ) nested_entries)
      | _ -> full_key ^ " = " ^ render_value value ^ "\n"
    ) entries)
  in
  let root_str = render_table doc.root [] in
  let tables_str = String.concat "" (List.map (fun (path, table) ->
    "\n[" ^ String.concat "." path ^ "]\n" ^ render_table table []
  ) doc.tables) in
  root_str ^ tables_str

(* テスト例：reml.toml風の設定。 *)
let test_reml_toml () =
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
  print_endline "--- reml.toml風設定のパース ---";
  match parse_toml example_toml with
  | Some doc ->
    print_endline "パース成功:";
    print_endline (render_to_string doc)
  | None ->
    print_endline "パースエラー"

(* 基本機能のテスト例。 *)
let test_examples () =
  let examples = [
    ("simple_kv", {|key = "value"|});
    ("integer", {|count = 42|});
    ("float", {|pi = 3.14159|});
    ("boolean", {|enabled = true|});
    ("array", {|items = [1, 2, 3]|});
    ("inline_table", {|server = { host = "localhost", port = 8080 }|});
    ("table", {|[section]
key = "value"|});
    ("nested_key", {|parent.child.grandchild = "value"|});
    ("comment", {|# This is a comment
key = "value"  # inline comment|});
  ] in
  List.iter (fun (name, toml_str) ->
    print_endline ("--- " ^ name ^ " ---");
    match parse_toml toml_str with
    | Some doc ->
      print_endline "パース成功:";
      print_endline (render_to_string doc)
    | None ->
      print_endline "パースエラー"
  ) examples

(* OCaml 5効果システムの利点：

   1. **型安全なパーサー状態管理**
      - Effect.tを使った状態の明示的な管理
      - GetPos/SetPos効果による位置追跡
      - GetInput効果による入力アクセス

   2. **柔軟なエラーハンドリング**
      - ParseError効果による構造化されたエラー報告
      - 効果ハンドラーでのバックトラック実装
      - エラー回復処理の実装（警告を出しつつ継続）

   3. **合成可能な設計**
      - 効果ハンドラーの合成により、追加機能を容易に実装可能
      - 例：ログ記録、デバッグトレース、パフォーマンス計測

   4. **並行処理への拡張性**
      - 効果システムにより、将来的に並行パース処理を追加可能
      - ファイバーベースの並行実行との統合

   Remlとの比較：

   - **OCaml 5の利点**:
     - 効果システムによる型安全性
     - モジュールシステムによる構造化
     - 成熟したエコシステム

   - **OCaml 5の課題**:
     - 効果システムがまだ発展途上（実験的機能）
     - パーサーコンビネーターライブラリが限定的
     - エラーメッセージのカスタマイズが煩雑

   - **Remlの利点**:
     - パーサーコンビネーター第一の設計
     - cut/commit/recoverによる高品質なエラー報告
     - 字句レイヤの柔軟性
     - 期待集合による有用な診断メッセージ

   実装上の工夫：

   1. **バックトラック**: 保存した位置に戻ることで複数候補を試行
   2. **エラー回復**: 構文エラーを検出しても次の行から継続してパース
   3. **lexeme抽象化**: 空白・コメント処理を一元化
   4. **再帰的値**: toml_value_refによる相互再帰の実現

   TOMLパーサーとしての制限事項：

   - 日時型は未対応（拡張可能）
   - 複数行文字列は未対応（実装可能）
   - リテラル文字列（'...'）は未対応
   - Unicode escapeは部分的対応
   - 数値の進数表記（0x、0o、0b）は未対応

   これらの制限は、OCaml 5の効果システムの能力を示す実装例として
   簡略化したものです。完全なTOML v1.0.0対応には、上記機能の追加が必要です。 *)