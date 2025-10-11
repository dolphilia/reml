(* Test_constraint_solver — Tests for Type Class Constraint Solver
 *
 * Phase 2 Week 18-19: 制約解決器の単体テスト
 *
 * テスト項目:
 * 1. プリミティブ型の制約解決（Eq<i64>, Ord<String> 等）
 * 2. 複合型の制約解決（Eq<(i64, String)>, Ord<Option<i64>> 等）
 * 3. 循環依存の検出
 * 4. 未解決制約のエラー報告
 *)

open Types
open Ast
open Constraint_solver

(* ========== テストヘルパー ========== *)

(** ダミーのspan（テスト用） *)
let dummy_span = { start = 0; end_ = 0 }

(** トレイト制約の構築ヘルパー *)
let make_constraint trait_name type_args =
  { trait_name; type_args; constraint_span = dummy_span }

(** テスト結果の検証 *)
let assert_some msg opt =
  match opt with
  | Some _ -> ()
  | None -> failwith ("Expected Some, got None: " ^ msg)

let assert_none msg opt =
  match opt with
  | Some _ -> failwith ("Expected None, got Some: " ^ msg)
  | None -> ()

let assert_ok msg result =
  match result with
  | Ok _ -> ()
  | Error _ -> failwith ("Expected Ok, got Error: " ^ msg)

let assert_error msg result =
  match result with
  | Ok _ -> failwith ("Expected Error, got Ok: " ^ msg)
  | Error _ -> ()

(* ========== 1. プリミティブ型の制約解決テスト ========== *)

(** Eq<i64> の解決 *)
let test_eq_primitive_i64 () =
  let result = solve_eq ty_i64 in
  assert_some "Eq<i64> should be resolved" result;
  print_endline "✓ test_eq_primitive_i64"

(** Eq<String> の解決 *)
let test_eq_primitive_string () =
  let result = solve_eq ty_string in
  assert_some "Eq<String> should be resolved" result;
  print_endline "✓ test_eq_primitive_string"

(** Eq<Bool> の解決 *)
let test_eq_primitive_bool () =
  let result = solve_eq ty_bool in
  assert_some "Eq<Bool> should be resolved" result;
  print_endline "✓ test_eq_primitive_bool"

(** Ord<i64> の解決 *)
let test_ord_primitive_i64 () =
  let result = solve_ord ty_i64 in
  assert_some "Ord<i64> should be resolved" result;
  print_endline "✓ test_ord_primitive_i64"

(** Ord<String> の解決 *)
let test_ord_primitive_string () =
  let result = solve_ord ty_string in
  assert_some "Ord<String> should be resolved" result;
  print_endline "✓ test_ord_primitive_string"

(** Ord<f64> の解決（IEEE 754全順序） *)
let test_ord_primitive_f64 () =
  let result = solve_ord ty_f64 in
  assert_some "Ord<f64> should be resolved (IEEE 754 total order)" result;
  print_endline "✓ test_ord_primitive_f64"

(** Collector<[i64]> の解決 *)
let test_collector_array () =
  let result = solve_collector (ty_array ty_i64) in
  assert_some "Collector<[i64]> should be resolved" result;
  print_endline "✓ test_collector_array"

(* ========== 2. 複合型の制約解決テスト ========== *)

(** Eq<(i64, String)> の解決 *)
let test_eq_tuple () =
  let ty = ty_tuple [ ty_i64; ty_string ] in
  let result = solve_eq ty in
  assert_some "Eq<(i64, String)> should be resolved" result;
  print_endline "✓ test_eq_tuple"

(** Eq<(i64, CustomType)> の解決失敗 *)
let test_eq_tuple_with_custom () =
  let custom_ty = TCon (TCUser "CustomType") in
  let ty = ty_tuple [ ty_i64; custom_ty ] in
  let result = solve_eq ty in
  assert_none "Eq<(i64, CustomType)> should fail" result;
  print_endline "✓ test_eq_tuple_with_custom"

(** Eq<{x: i64, y: String}> の解決 *)
let test_eq_record () =
  let ty = ty_record [ ("x", ty_i64); ("y", ty_string) ] in
  let result = solve_eq ty in
  assert_some "Eq<{x: i64, y: String}> should be resolved" result;
  print_endline "✓ test_eq_record"

(** Ord<(i64, String)> の解決 *)
let test_ord_tuple () =
  let ty = ty_tuple [ ty_i64; ty_string ] in
  let result = solve_ord ty in
  assert_some "Ord<(i64, String)> should be resolved" result;
  print_endline "✓ test_ord_tuple"

(** Eq<Option<i64>> の解決 *)
let test_eq_option () =
  let ty = ty_option ty_i64 in
  let result = solve_eq ty in
  assert_some "Eq<Option<i64>> should be resolved" result;
  print_endline "✓ test_eq_option"

(** Eq<Result<i64, String>> の解決 *)
let test_eq_result () =
  let ty = ty_result ty_i64 ty_string in
  let result = solve_eq ty in
  assert_some "Eq<Result<i64, String>> should be resolved" result;
  print_endline "✓ test_eq_result"

(** Collector<Option<i64>> の解決 *)
let test_collector_option () =
  let ty = ty_option ty_i64 in
  let result = solve_collector ty in
  assert_some "Collector<Option<i64>> should be resolved" result;
  print_endline "✓ test_collector_option"

(* ========== 3. 制約解決の統合テスト ========== *)

(** 単一制約の解決 *)
let test_solve_single_constraint () =
  let constraint_ = make_constraint "Eq" [ ty_i64 ] in
  let result = solve_constraints [ constraint_ ] in
  assert_ok "solve_constraints should succeed for Eq<i64>" result;
  print_endline "✓ test_solve_single_constraint"

(** 複数制約の解決 *)
let test_solve_multiple_constraints () =
  let constraints =
    [
      make_constraint "Eq" [ ty_i64 ];
      make_constraint "Ord" [ ty_string ];
      make_constraint "Collector" [ ty_array ty_i64 ];
    ]
  in
  let result = solve_constraints constraints in
  assert_ok "solve_constraints should succeed for multiple constraints" result;
  print_endline "✓ test_solve_multiple_constraints"

(** 解決失敗の検出 *)
let test_solve_failing_constraint () =
  let custom_ty = TCon (TCUser "CustomType") in
  let constraint_ = make_constraint "Eq" [ custom_ty ] in
  let result = solve_constraints [ constraint_ ] in
  assert_error "solve_constraints should fail for Eq<CustomType>" result;
  print_endline "✓ test_solve_failing_constraint"

(* ========== 4. 制約グラフのテスト ========== *)

(** 制約グラフの構築（単純なケース） *)
let test_build_constraint_graph_simple () =
  let constraints =
    [
      make_constraint "Eq" [ ty_i64 ];
      make_constraint "Ord" [ ty_i64 ];
    ]
  in
  let graph = build_constraint_graph constraints in
  (* Ord<i64> は Eq<i64> に依存する *)
  let has_dependency =
    List.exists
      (fun ((dep : Types.trait_constraint), _) ->
        dep.trait_name = "Eq" && List.length dep.type_args = 1
        && match List.hd dep.type_args with
           | ty when type_equal ty ty_i64 -> true
           | _ -> false)
      graph.edges
  in
  if has_dependency then
    print_endline "✓ test_build_constraint_graph_simple"
  else failwith "Expected Ord<i64> to depend on Eq<i64>"

(** 制約グラフの構築（再帰的な依存） *)
let test_build_constraint_graph_recursive () =
  let tuple_ty = ty_tuple [ ty_i64; ty_string ] in
  let constraint_ = make_constraint "Eq" [ tuple_ty ] in
  let graph = build_constraint_graph [ constraint_ ] in
  (* Eq<(i64, String)> は Eq<i64> と Eq<String> に依存する *)
  let has_i64_dep =
    List.exists
      (fun ((dep : Types.trait_constraint), _) ->
        dep.trait_name = "Eq" && List.length dep.type_args = 1
        && match List.hd dep.type_args with
           | ty when type_equal ty ty_i64 -> true
           | _ -> false)
      graph.edges
  in
  let has_string_dep =
    List.exists
      (fun ((dep : Types.trait_constraint), _) ->
        dep.trait_name = "Eq" && List.length dep.type_args = 1
        && match List.hd dep.type_args with
           | ty when type_equal ty ty_string -> true
           | _ -> false)
      graph.edges
  in
  if has_i64_dep && has_string_dep then
    print_endline "✓ test_build_constraint_graph_recursive"
  else failwith "Expected Eq<(i64, String)> to depend on Eq<i64> and Eq<String>"

(* ========== 5. デバッグ出力のテスト ========== *)

(** 辞書参照の文字列表現 *)
let test_string_of_dict_ref () =
  let dict_ref = DictImplicit ("Eq", ty_i64) in
  let str = string_of_dict_ref dict_ref in
  if String.length str > 0 then
    print_endline "✓ test_string_of_dict_ref"
  else failwith "string_of_dict_ref returned empty string"

(** 制約エラーの文字列表現 *)
let test_string_of_constraint_error () =
  let error =
    {
      trait_name = "Eq";
      type_args = [ TCon (TCUser "CustomType") ];
      reason = NoImpl;
      span = dummy_span;
    }
  in
  let str = string_of_constraint_error error in
  if String.length str > 0 then
    print_endline "✓ test_string_of_constraint_error"
  else failwith "string_of_constraint_error returned empty string"

(* ========== テスト実行 ========== *)

let () =
  print_endline "===== Constraint Solver Tests =====";
  print_endline "";
  print_endline "=== 1. Primitive Type Constraints ===";
  test_eq_primitive_i64 ();
  test_eq_primitive_string ();
  test_eq_primitive_bool ();
  test_ord_primitive_i64 ();
  test_ord_primitive_string ();
  test_ord_primitive_f64 ();
  test_collector_array ();
  print_endline "";
  print_endline "=== 2. Compound Type Constraints ===";
  test_eq_tuple ();
  test_eq_tuple_with_custom ();
  test_eq_record ();
  test_ord_tuple ();
  test_eq_option ();
  test_eq_result ();
  test_collector_option ();
  print_endline "";
  print_endline "=== 3. Constraint Solving Integration ===";
  test_solve_single_constraint ();
  test_solve_multiple_constraints ();
  test_solve_failing_constraint ();
  print_endline "";
  print_endline "=== 4. Constraint Graph Tests ===";
  test_build_constraint_graph_simple ();
  test_build_constraint_graph_recursive ();
  print_endline "";
  print_endline "=== 5. Debug Output Tests ===";
  test_string_of_dict_ref ();
  test_string_of_constraint_error ();
  print_endline "";
  print_endline "===== All Tests Passed! ====="
