(* test_monomorphize_poc.ml — 辞書経路とモノモルフィック経路の共存検証
 *
 * Core_ir.Monomorphize_poc パスが辞書呼び出しを具象ラッパーへ正しく置換し、
 * 両経路の差分レポートを出力することを確認する。
 *)

open Core_ir.Ir

(* ========= パス設定 ========= *)

let project_root =
  match Sys.getenv_opt "DUNE_SOURCEROOT" with
  | Some root -> root
  | None -> Filename.dirname Sys.argv.(0)

let resolve path = Filename.concat project_root path
let integration_dir = resolve "tests/integration"
let source_file = Filename.concat integration_dir "test_typeclass_e2e.reml"

let artifacts_dir =
  (* LLVM ゴールデンテストと同じ _actual ディレクトリを再利用 *)
  resolve "tests/llvm-ir/golden/_actual"

let diff_report_path =
  Filename.concat artifacts_dir "monomorphize_poc_core_ir.diff"

let diff_golden_path =
  resolve "tests/llvm-ir/golden/typeclass_monomorph.diff.golden"

(* ========= ヘルパー ========= *)

let ensure_dir dir =
  if not (Sys.file_exists dir) then Unix.mkdir dir 0o755

let read_file path =
  In_channel.with_open_text path In_channel.input_all

let parse_tast () =
  let source = read_file source_file in
  let lexbuf = Lexing.from_string source in
  lexbuf.Lexing.lex_curr_p <-
    { lexbuf.Lexing.lex_curr_p with Lexing.pos_fname = source_file };
  match Parser_driver.parse lexbuf with
  | Error diag ->
      let message =
        Printf.sprintf "Parse error: %s" (Diagnostic.to_string diag)
      in
      failwith message
  | Ok ast -> (
      match Type_inference.infer_compilation_unit ast with
      | Error type_err ->
          let diag =
            Type_error.to_diagnostic_with_source source source_file type_err
          in
          let message =
            Printf.sprintf "Type error: %s" (Diagnostic.to_string diag)
          in
          failwith message
      | Ok tast -> tast)

let desugar ~mode tast =
  let desugared = Core_ir.Desugar.desugar_compilation_unit tast in
  Core_ir.Monomorphize_poc.apply ~mode desugared

(* ========= Core IR 解析 ========= *)

let rec collect_expr acc expr =
  let acc =
    match expr.expr_kind with
    | DictMethodCall (dict_expr, method_name, args) ->
        let acc = method_name :: acc in
        let acc = collect_expr acc dict_expr in
        List.fold_left collect_expr acc args
    | App (fn, args) ->
        let acc = collect_expr acc fn in
        List.fold_left collect_expr acc args
    | Let (_, bound, body) ->
        let acc = collect_expr acc bound in
        collect_expr acc body
    | If (cond, then_e, else_e) ->
        let acc = collect_expr acc cond in
        let acc = collect_expr acc then_e in
        collect_expr acc else_e
    | Match (scrut, cases) ->
        let acc = collect_expr acc scrut in
        List.fold_left
          (fun acc case ->
            let acc = collect_expr acc case.case_body in
            match case.case_guard with
            | None -> acc
            | Some guard -> collect_expr acc guard)
          acc cases
    | Primitive (_, args) ->
        List.fold_left collect_expr acc args
    | TupleAccess (e, _) | RecordAccess (e, _) | ADTProject (e, _) ->
        collect_expr acc e
    | ArrayAccess (arr, idx) ->
        let acc = collect_expr acc arr in
        collect_expr acc idx
    | ADTConstruct (_, fields) ->
        List.fold_left collect_expr acc fields
    | AssignMutable (_, rhs) -> collect_expr acc rhs
    | Loop loop_info ->
        let acc =
          match loop_info.loop_kind with
          | WhileLoop cond -> collect_expr acc cond
          | ForLoop info ->
              let acc =
                List.fold_left
                  (fun acc (_, e) -> collect_expr acc e)
                  acc info.for_init
              in
              let acc =
                List.fold_left
                  (fun acc (_, e) -> collect_expr acc e)
                  acc info.for_step
              in
              let acc = collect_expr acc info.for_source in
              acc
          | InfiniteLoop -> acc
        in
        collect_expr acc loop_info.loop_body
    | Closure _ | Literal _ | Var _ | DictLookup _ | DictConstruct _
    | CapabilityCheck _ ->
        acc
  in
  acc

let string_ends_with ~suffix s =
  let len_s = String.length s and len_suffix = String.length suffix in
  len_suffix <= len_s
  && String.sub s (len_s - len_suffix) len_suffix = suffix

let string_starts_with ~prefix s =
  let len_s = String.length s and len_prefix = String.length prefix in
  len_prefix <= len_s
  && String.sub s 0 len_prefix = prefix

let collect_from_stmt acc = function
  | Assign (_, expr) | Return expr | ExprStmt expr ->
      collect_expr acc expr
  | Store (_, expr) -> collect_expr acc expr
  | Alloca _ -> acc
  | Branch (cond, _, _) -> collect_expr acc cond
  | Jump _ | Phi _ -> acc
  | EffectMarker { effect_expr; _ } -> (
      match effect_expr with
      | None -> acc
      | Some expr -> collect_expr acc expr)

let collect_from_terminator acc = function
  | TermReturn expr -> collect_expr acc expr
  | TermBranch (cond, _, _) -> collect_expr acc cond
  | TermJump _ | TermUnreachable -> acc
  | TermSwitch (scrutinee, _cases, _default) ->
      collect_expr acc scrutinee

let collect_dict_calls module_def =
  List.fold_left
    (fun acc fn ->
      List.fold_left
        (fun acc block ->
          let acc =
            List.fold_left collect_from_stmt acc block.stmts
          in
          collect_from_terminator acc block.terminator)
        acc fn.fn_blocks)
    [] module_def.function_defs

let collect_wrappers module_def =
  List.filter_map
    (fun fn ->
      if string_ends_with ~suffix:"_mono" fn.fn_name then Some fn.fn_name
      else None)
    module_def.function_defs

let string_of_dict_calls calls =
  let table = Hashtbl.create 4 in
  List.iter
    (fun method_name ->
      let count = Hashtbl.find_opt table method_name |> Option.value ~default:0 in
      Hashtbl.replace table method_name (count + 1))
    calls;
  let pairs =
    Hashtbl.to_seq table |> List.of_seq
    |> List.map (fun (method_name, count) -> (method_name, count))
    |> List.sort (fun (lhs, _) (rhs, _) -> String.compare lhs rhs)
  in
  if pairs = [] then ""
  else
    pairs
    |> List.map (fun (method_name, count) ->
           Printf.sprintf "- %s : %d call(s)" method_name count)
    |> String.concat "\n"

let string_of_wrappers wrappers =
  let sorted = List.sort_uniq String.compare wrappers in
  let eq_wrappers, other_wrappers =
    List.partition (fun name -> string_starts_with ~prefix:"__Eq_" name) sorted
  in
  let eq_section =
    match eq_wrappers with
    | [] -> "（Eq 系ラッパーは検出されませんでした）"
    | _ ->
        eq_wrappers
        |> List.map (fun name -> Printf.sprintf "- %s" name)
        |> String.concat "\n"
  in
  let others_count = List.length other_wrappers in
  Printf.sprintf "%s\n- その他: %d 件" eq_section others_count

let write_diff_report ~dictionary_module ~monomorph_module =
  ensure_dir artifacts_dir;
  let dict_calls = collect_dict_calls dictionary_module in
  let wrappers = collect_wrappers monomorph_module in
  let dict_section =
    if dict_calls = [] then "辞書経路: DictMethodCall は検出されませんでした。"
    else
      Printf.sprintf "辞書経路の DictMethodCall 一覧:\n%s"
        (string_of_dict_calls dict_calls)
  in
  let wrapper_section =
    if wrappers = [] then "モノモルフィック経路: 生成ラッパー無し。"
    else
      Printf.sprintf "モノモルフィック経路で生成されたラッパー:\n%s"
        (string_of_wrappers wrappers)
  in
  let report =
    Printf.sprintf
      "Monomorphize PoC 差分サマリー\n=============================\n%s\n\n%s\n"
      dict_section wrapper_section
  in
  Out_channel.with_open_text diff_report_path (fun oc ->
      output_string oc report);
  report

let verify_diff_against_golden report =
  if not (Sys.file_exists diff_golden_path) then (
    Printf.eprintf "✗ ゴールデンファイルが存在しません: %s\n"
      diff_golden_path;
    exit 1);
  let expected = read_file diff_golden_path |> String.trim in
  let actual = String.trim report in
  if not (String.equal expected actual) then (
    Printf.eprintf "✗ 差分サマリーがゴールデンと一致しません。\n";
    Printf.eprintf "  ゴールデン: %s\n" diff_golden_path;
    Printf.eprintf "  実測: %s\n" diff_report_path;
    exit 1)

(* ========= テスト本体 ========= *)

let () =
  Printexc.record_backtrace true;

  if not (Sys.file_exists source_file) then (
    Printf.eprintf "エラー: ソースファイルが存在しません: %s\n" source_file;
    exit 1);

  let tast = parse_tast () in

  let dictionary_module =
    desugar ~mode:Core_ir.Monomorphize_poc.UseDictionary tast
  in
  let monomorph_module =
    desugar ~mode:Core_ir.Monomorphize_poc.UseMonomorph tast
  in

  let dict_calls = collect_dict_calls dictionary_module in
  if dict_calls = [] then (
    Printf.eprintf "✗ 辞書経路で DictMethodCall が生成されていません。\n";
    exit 1);

  let mono_calls = collect_dict_calls monomorph_module in
  if mono_calls <> [] then (
    Printf.eprintf "✗ モノモルフィック経路に DictMethodCall が残存しています。\n";
    exit 1);

  let wrappers = collect_wrappers monomorph_module in
  if wrappers = [] then (
    Printf.eprintf "✗ モノモルフィックラッパーが生成されていません。\n";
    exit 1);

  let report = write_diff_report ~dictionary_module ~monomorph_module in
  verify_diff_against_golden report;
  Printf.printf "✓ 辞書経路とモノモルフィック経路の共存テスト成功\n%!";
  Printf.printf "  差分レポート: %s\n" diff_report_path;
  Printf.printf "  --- 抜粋 ---\n%s\n" report
