(* effect_analysis_tests.ml — 効果タグ収集ロジックの単体テスト
 *
 * Phase 2-5 EFFECT-001 で追加した効果タグ検出を検証する。
 * 目標:
 * - 変数再代入で `mut` タグが付与されること
 * - Core.IO 由来の呼び出しで `io` タグが付与されること
 * - FFI ブリッジスナップショットに一致する呼び出しで `ffi` タグが付与されること
 *)

open Typed_ast

module Effect_analysis = Type_inference.Effect_analysis
module Ffi = Ffi_contract

let dummy_span = Ast.dummy_span

let ident name = { Ast.name = name; span = dummy_span }

let scheme ty = Types.scheme_to_constrained (Types.mono_scheme ty)

let var_expr name ty =
  make_typed_expr (TVar (ident name, scheme ty)) ty dummy_span

let literal_i64 value =
  let lit = Ast.Int (string_of_int value, Ast.Base10) in
  make_typed_expr (TLiteral lit) Types.ty_i64 dummy_span

let literal_string value =
  let lit = Ast.String (value, Ast.Normal) in
  make_typed_expr (TLiteral lit) Types.ty_string dummy_span

let add_expr lhs rhs =
  make_typed_expr (TBinary (Ast.Add, lhs, rhs)) lhs.texpr_ty dummy_span

let var_pattern name ty =
  make_typed_pattern (TPatVar (ident name)) ty [ (name, ty) ] dummy_span

let names_of_tags tags =
  tags
  |> List.map (fun tag -> String.lowercase_ascii tag.Effect_profile.effect_name)
  |> List.sort_uniq String.compare

let assert_has_tag tags expected =
  if not (List.mem expected (names_of_tags tags)) then
    failwith
      (Printf.sprintf "期待するタグ %s が付与されていません: [%s]" expected
         (String.concat ", " (names_of_tags tags)))

let assert_only_tags tags expected =
  let actual = names_of_tags tags in
  let missing =
    List.filter (fun name -> not (List.mem name actual)) expected
  in
  let extra = List.filter (fun name -> not (List.mem name expected)) actual in
  if missing <> [] || extra <> [] then
    failwith
      (Printf.sprintf
         "効果タグが一致しません。\n  期待: [%s]\n  実際: [%s]\n  欠落: [%s]\n  余分: [%s]"
         (String.concat ", " expected)
         (String.concat ", " actual)
         (String.concat ", " missing)
         (String.concat ", " extra))

let test_count = ref 0
let success_count = ref 0

let run_test name f =
  incr test_count;
  try
    f ();
    incr success_count;
    Printf.printf "✓ %s\n%!" name
  with exn ->
    Printf.eprintf "✗ %s: %s\n%!" name (Printexc.to_string exn);
    exit 1

let test_mut_tag () =
  let decl =
    TDeclStmt
      {
        tdecl_attrs = [];
        tdecl_vis = Ast.Private;
        tdecl_kind =
          TVarDecl (var_pattern "y" Types.ty_i64, var_expr "x" Types.ty_i64);
        tdecl_scheme = scheme Types.ty_i64;
        tdecl_span = dummy_span;
        tdecl_dict_refs = [];
      }
  in
  let assign =
    TAssignStmt
      ( var_expr "y" Types.ty_i64,
        add_expr (var_expr "y" Types.ty_i64) (literal_i64 1) )
  in
  let body = TFnBlock [ decl; assign; TExprStmt (var_expr "y" Types.ty_i64) ] in
  let tags = Effect_analysis.collect_from_fn_body body in
  assert_only_tags tags [ "mut" ]

let test_io_tag () =
  let module_path = Ast.Root [ ident "Core"; ident "IO" ] in
  let fn_ident = ident "write_line" in
  let fn_ty = Types.ty_arrow Types.ty_string Types.ty_unit in
  let fn_expr = make_typed_expr (TModulePath (module_path, fn_ident)) fn_ty dummy_span in
  let call =
    make_typed_expr
      (TCall (fn_expr, [ TPosArg (literal_string "hello") ]))
      Types.ty_unit dummy_span
  in
  let body = TFnBlock [ TExprStmt call ] in
  let tags = Effect_analysis.collect_from_fn_body body in
  assert_only_tags tags [ "io" ]

let test_ffi_tag () =
  Type_inference.reset_ffi_bridge_snapshots ();
  let metadata =
    {
      Ast.extern_target = Some "x86_64-unknown-linux-gnu";
      extern_calling_convention = Some "system_v";
      extern_link_name = Some "ffi_entry";
      extern_ownership = Some "borrowed";
      extern_invalid_attributes = [];
    }
  in
  let contract =
    Ffi.bridge_contract ~extern_name:"ffi_entry" ~source_span:dummy_span
      ~metadata ()
  in
  let normalized = Ffi.normalize_contract contract in
  Type_inference.record_ffi_bridge_snapshot
    { normalized; param_types = []; return_type = Types.ty_unit };
  let fn_ty = Types.ty_arrow Types.ty_unit Types.ty_unit in
  let call =
    make_typed_expr
      (TCall (var_expr "ffi_entry" fn_ty, []))
      Types.ty_unit dummy_span
  in
  let body = TFnBlock [ TExprStmt call ] in
  let tags = Effect_analysis.collect_from_fn_body body in
  assert_has_tag tags "ffi";
  Type_inference.reset_ffi_bridge_snapshots ()

let () =
  run_test "mut タグ検出" test_mut_tag;
  run_test "io タグ検出" test_io_tag;
  run_test "ffi タグ検出" test_ffi_tag;
  Printf.printf "合計 %d 件中 %d 件成功\n%!" !test_count !success_count
