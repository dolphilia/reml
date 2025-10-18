(* test_dict_gen.ml - 辞書生成パスのテスト (Phase 2 Week 19-22)
 *
 * このファイルは辞書生成パス（desugar.ml の generate_dict_init 等）の
 * ユニットテストを提供する。
 *)

open Core_ir.Ir
open Core_ir.Desugar
open Types

(* ========== テストヘルパー ========== *)

let dummy_span = Ast.{ start = 0; end_ = 0 }

(** テスト成功カウンタ *)
let test_count = ref 0

let success_count = ref 0

(** テスト実行ヘルパー *)
let run_test name f =
  incr test_count;
  try
    f ();
    incr success_count;
    Printf.printf "  ✓ %s\n" name
  with e -> Printf.printf "  ✗ %s: %s\n" name (Printexc.to_string e)

(** テストアサーション *)
let assert_some opt msg = match opt with Some v -> v | None -> failwith msg

let assert_none opt msg = match opt with None -> () | Some _ -> failwith msg

(* ========== 辞書初期化テスト ========== *)

let test_generate_dict_init_eq_i64 () =
  (* Eq<i64> の辞書初期化コードを生成 *)
  let result = generate_dict_init "Eq" ty_i64 dummy_span in
  let dict_expr = assert_some result "Eq<i64> の辞書生成に失敗" in

  (* 辞書式が DictConstruct ノードであることを確認 *)
  match dict_expr.expr_kind with
  | DictConstruct dict_ty ->
      (* トレイト名と実装型が正しいことを確認 *)
      assert (dict_ty.dict_trait = "Eq");
      assert (type_equal dict_ty.dict_impl_ty ty_i64);
      (* メソッドが2つあることを確認（eq, ne） *)
      assert (List.length dict_ty.dict_methods = 2);
      ()
  | _ -> failwith "DictConstruct ノードが生成されていない"

let test_generate_dict_init_ord_i64 () =
  (* Ord<i64> の辞書初期化コードを生成 *)
  let result = generate_dict_init "Ord" ty_i64 dummy_span in
  let dict_expr = assert_some result "Ord<i64> の辞書生成に失敗" in

  match dict_expr.expr_kind with
  | DictConstruct dict_ty ->
      assert (dict_ty.dict_trait = "Ord");
      assert (type_equal dict_ty.dict_impl_ty ty_i64);
      (* Ord のメソッド数を確認（lt, le, gt, ge の4つ） *)
      assert (List.length dict_ty.dict_methods = 4);
      ()
  | _ -> failwith "DictConstruct ノードが生成されていない"

let test_generate_dict_init_eq_string () =
  (* Eq<String> の辞書初期化コードを生成 *)
  let result = generate_dict_init "Eq" ty_string dummy_span in
  let dict_expr = assert_some result "Eq<String> の辞書生成に失敗" in

  match dict_expr.expr_kind with
  | DictConstruct dict_ty ->
      assert (dict_ty.dict_trait = "Eq");
      assert (type_equal dict_ty.dict_impl_ty ty_string);
      ()
  | _ -> failwith "DictConstruct ノードが生成されていない"

let test_generate_dict_init_unsupported () =
  (* サポートされていない型クラス/型の組み合わせでは None を返す *)
  let tv = TVar { tv_id = 0; tv_name = Some "T" } in
  let result = generate_dict_init "Eq" tv dummy_span in
  assert_none result "型変数に対する辞書は生成されないべき";

  let result2 = generate_dict_init "UnknownTrait" ty_i64 dummy_span in
  assert_none result2 "未知のトレイトに対する辞書は生成されないべき"

(* ========== 辞書パラメータ生成テスト ========== *)

let test_generate_dict_params_empty () =
  (* 制約がない場合は空リストを返す *)
  let fn_scope = create_scope_map () in
  let params = generate_dict_params fn_scope [] dummy_span in
  assert (List.length params = 0)

let test_generate_dict_params_single () =
  (* 単一の制約から辞書パラメータを生成 *)
  let fn_scope = create_scope_map () in
  let constraint_info =
    {
      Types.trait_name = "Eq";
      Types.type_args = [ ty_i64 ];
      Types.constraint_span = dummy_span;
    }
  in
  let params = generate_dict_params fn_scope [ constraint_info ] dummy_span in

  assert (List.length params = 1);
  let param = List.hd params in
  assert (String.starts_with ~prefix:"__dict_Eq" param.param_var.vname)

let test_generate_dict_params_multiple () =
  (* 複数の制約から辞書パラメータを生成 *)
  let fn_scope = create_scope_map () in
  let constraints =
    [
      {
        Types.trait_name = "Eq";
        Types.type_args = [ ty_i64 ];
        Types.constraint_span = dummy_span;
      };
      {
        Types.trait_name = "Ord";
        Types.type_args = [ ty_i64 ];
        Types.constraint_span = dummy_span;
      };
    ]
  in
  let params = generate_dict_params fn_scope constraints dummy_span in

  assert (List.length params = 2);
  (* パラメータ名が異なることを確認 *)
  let names = List.map (fun p -> p.param_var.vname) params in
  assert (List.nth names 0 <> List.nth names 1)

(* ========== vtable インデックステスト ========== *)

let test_trait_method_indices_eq () =
  let methods = trait_method_indices "Eq" in
  assert (List.length methods = 2);
  assert (List.assoc "eq" methods = 0);
  assert (List.assoc "ne" methods = 1)

let test_trait_method_indices_ord () =
  let methods = trait_method_indices "Ord" in
  assert (List.length methods = 5);
  assert (List.assoc "cmp" methods = 0);
  assert (List.assoc "lt" methods = 1);
  assert (List.assoc "le" methods = 2);
  assert (List.assoc "gt" methods = 3);
  assert (List.assoc "ge" methods = 4)

let test_get_method_index () =
  assert (get_method_index "Eq" "eq" = Some 0);
  assert (get_method_index "Eq" "ne" = Some 1);
  assert (get_method_index "Ord" "lt" = Some 1);
  assert (get_method_index "UnknownTrait" "method" = None);
  assert (get_method_index "Eq" "unknown_method" = None)

(* ========== メインテストランナー ========== *)

let () =
  Printf.printf "\n辞書生成パステスト\n";
  Printf.printf "==================\n\n";

  Printf.printf "--- 辞書初期化テスト ---\n";
  run_test "test_generate_dict_init_eq_i64" test_generate_dict_init_eq_i64;
  run_test "test_generate_dict_init_ord_i64" test_generate_dict_init_ord_i64;
  run_test "test_generate_dict_init_eq_string" test_generate_dict_init_eq_string;
  run_test "test_generate_dict_init_unsupported"
    test_generate_dict_init_unsupported;

  Printf.printf "\n--- 辞書パラメータ生成テスト ---\n";
  run_test "test_generate_dict_params_empty" test_generate_dict_params_empty;
  run_test "test_generate_dict_params_single" test_generate_dict_params_single;
  run_test "test_generate_dict_params_multiple"
    test_generate_dict_params_multiple;

  Printf.printf "\n--- vtable インデックステスト ---\n";
  run_test "test_trait_method_indices_eq" test_trait_method_indices_eq;
  run_test "test_trait_method_indices_ord" test_trait_method_indices_ord;
  run_test "test_get_method_index" test_get_method_index;

  Printf.printf "\n==================\n";
  if !success_count = !test_count then
    Printf.printf "✓ 全 %d 件のテストが成功しました！\n\n" !test_count
  else
    Printf.printf "✗ %d/%d 件のテストが失敗しました\n\n"
      (!test_count - !success_count)
      !test_count
