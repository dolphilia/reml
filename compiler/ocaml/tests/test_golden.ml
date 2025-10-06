(* test_golden.ml — ゴールデンテスト
 *
 * サンプルファイルを解析して、AST 出力のスナップショットと比較する。
 * 差分が検出された場合はテスト失敗。
 *)

open Ast

(* AST を読みやすい文字列に変換 *)

let rec string_of_ident i = i.name

let string_of_module_path = function
  | Root ids -> "::" ^ String.concat "." (List.map string_of_ident ids)
  | Relative (head, tail) ->
      let head_str = match head with
        | Self -> "self"
        | Super n -> String.concat "." (List.init n (fun _ -> "super"))
        | PlainIdent id -> string_of_ident id
      in
      if tail = [] then head_str
      else head_str ^ "." ^ String.concat "." (List.map string_of_ident tail)

let string_of_visibility = function
  | Public -> "pub "
  | Private -> ""

let rec string_of_type_annot ty =
  match ty.ty_kind with
  | TyIdent id -> string_of_ident id
  | TyApp (id, args) ->
      string_of_ident id ^ "<" ^ String.concat ", " (List.map string_of_type_annot args) ^ ">"
  | TyTuple tys ->
      "(" ^ String.concat ", " (List.map string_of_type_annot tys) ^ ")"
  | TyRecord fields ->
      "{ " ^ String.concat ", " (List.map (fun (id, ty) ->
        string_of_ident id ^ ": " ^ string_of_type_annot ty) fields) ^ " }"
  | TyFn (args, ret) ->
      String.concat " -> " (List.map string_of_type_annot (args @ [ret]))

let string_of_decl_kind = function
  | LetDecl (pat, ty, _) ->
      "let " ^ (match ty with
        | Some t -> ": " ^ string_of_type_annot t
        | None -> "")
  | VarDecl (pat, ty, _) ->
      "var " ^ (match ty with
        | Some t -> ": " ^ string_of_type_annot t
        | None -> "")
  | FnDecl fn ->
      "fn " ^ string_of_ident fn.fn_name ^
      (if fn.fn_generic_params = [] then "" else "<" ^ String.concat ", " (List.map string_of_ident fn.fn_generic_params) ^ ">") ^
      "(...)" ^
      (match fn.fn_ret_type with Some rt -> " -> " ^ string_of_type_annot rt | None -> "")
  | TypeDecl td -> (match td with
      | AliasDecl (name, _, _) -> "type alias " ^ string_of_ident name
      | NewtypeDecl (name, _, _) -> "type " ^ string_of_ident name ^ " = new ..."
      | SumDecl (name, _, _) -> "type " ^ string_of_ident name ^ " = ...")
  | TraitDecl tr -> "trait " ^ string_of_ident tr.trait_name
  | ImplDecl impl -> "impl ..."
  | ExternDecl ext -> "extern \"" ^ ext.extern_abi ^ "\""
  | EffectDecl eff -> "effect " ^ string_of_ident eff.effect_name
  | HandlerDecl h -> "handler " ^ string_of_ident h.handler_name
  | ConductorDecl c -> "conductor " ^ string_of_ident c.conductor_name

let string_of_use_tree = function
  | UsePath (path, alias) ->
      "use " ^ string_of_module_path path ^
      (match alias with Some a -> " as " ^ string_of_ident a | None -> "")
  | UseBrace (path, items) ->
      "use " ^ string_of_module_path path ^ ".{...}"

let string_of_ast cu =
  let lines = [] in

  (* モジュールヘッダ *)
  let lines = match cu.header with
    | Some h -> lines @ ["module " ^ string_of_module_path h.module_path]
    | None -> lines
  in

  (* use 宣言 *)
  let lines = lines @ (List.map (fun u ->
    (if u.use_pub then "pub " else "") ^ string_of_use_tree u.use_tree
  ) cu.uses) in

  (* 宣言 *)
  let lines = lines @ (List.map (fun d ->
    string_of_visibility d.decl_vis ^ string_of_decl_kind d.decl_kind
  ) cu.decls) in

  String.concat "\n" lines

(* ゴールデンファイルのパス *)

let golden_dir = "tests/golden"

let golden_path name =
  Filename.concat golden_dir (name ^ ".golden")

(* ゴールデンテスト実行 *)

let test_golden name input_file =
  (* 入力ファイルを解析 *)
  let ic = open_in input_file in
  let lexbuf = Lexing.from_channel ic in
  lexbuf.Lexing.lex_curr_p <- { lexbuf.Lexing.lex_curr_p with Lexing.pos_fname = input_file };

  let ast = try
    let cu = Parser.compilation_unit Lexer.token lexbuf in
    close_in ic;
    Some cu
  with
  | Parser.Error ->
      close_in ic;
      Printf.eprintf "Parse error in %s\n" input_file;
      None
  | Lexer.Lexer_error (msg, span) ->
      close_in ic;
      Printf.eprintf "Lexer error in %s: %s\n" input_file msg;
      None
  in

  match ast with
  | None ->
      Printf.printf "✗ %s: parse failed\n" name;
      exit 1
  | Some cu ->
      let actual = string_of_ast cu in
      let golden_file = golden_path name in

      (* ゴールデンファイルが存在するか確認 *)
      if Sys.file_exists golden_file then begin
        (* 既存のゴールデンファイルと比較 *)
        let expected = In_channel.with_open_text golden_file In_channel.input_all in
        let expected = String.trim expected in
        let actual = String.trim actual in

        if expected = actual then
          Printf.printf "✓ %s\n" name
        else begin
          Printf.printf "✗ %s: output differs from golden file\n" name;
          Printf.printf "Expected:\n%s\n\n" expected;
          Printf.printf "Actual:\n%s\n\n" actual;
          Printf.printf "To update golden file: cp actual.txt %s\n" golden_file;
          exit 1
        end
      end else begin
        (* ゴールデンファイルが存在しない場合は作成 *)
        let oc = open_out golden_file in
        output_string oc actual;
        output_char oc '\n';
        close_out oc;
        Printf.printf "✓ %s: created golden file\n" name
      end

(* テストケース *)

let () =
  Printf.printf "Running Golden Tests\n";
  Printf.printf "====================\n\n";

  (* golden ディレクトリが存在しない場合は作成 *)
  if not (Sys.file_exists golden_dir) then
    Unix.mkdir golden_dir 0o755;

  (* simple.reml テスト *)
  test_golden "simple" "tests/simple.reml";

  Printf.printf "\n";
  Printf.printf "====================\n";
  Printf.printf "All Golden tests passed!\n"
