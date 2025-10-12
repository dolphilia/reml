(* test_impl_registry.ml — Unit Tests for Impl_registry Module
 *
 * Phase 2 Week 23-24: 制約ソルバーへの impl 宣言登録
 *
 * テスト方針:
 * - TDD アプローチでレジストリ機能を検証
 * - 基本操作（登録・検索）のテスト
 * - 型照合ロジックの検証
 * - エッジケースのカバー
 *)

open Types
open Ast
open Impl_registry

(* ========== テストユーティリティ ========== *)

(** ダミーのspan *)
let dummy_span : span = { start = 0; end_ = 0 }

(** テスト用の型変数生成 *)
let fresh_tv =
  let counter = ref 0 in
  fun name ->
    let id = !counter in
    incr counter;
    { tv_id = id; tv_name = Some name }

(** 型変数 T *)
let tv_t = fresh_tv "T"

(* ========== テストケース ========== *)

(** Test 1: 空のレジストリ作成 *)
let test_empty_registry () =
  let registry = empty () in
  let impls = all_impls registry in
  assert (impls = []);
  print_endline "✓ Test 1: 空のレジストリ作成"

(** Test 2: ビルトイン型の impl 登録 *)
let test_register_builtin_impl () =
  let registry = empty () in

  (* impl Eq for i64 *)
  let impl_eq_i64 = {
    trait_name = "Eq";
    impl_type = TCon (TCInt I64);
    generic_params = [];
    where_constraints = [];
    methods = [("eq", "i64_eq"); ("ne", "i64_ne")];
    span = dummy_span;
  } in

  let registry = register impl_eq_i64 registry in
  let impls = all_impls registry in

  assert (List.length impls = 1);
  assert (List.hd impls = impl_eq_i64);
  print_endline "✓ Test 2: ビルトイン型の impl 登録"

(** Test 3: ビルトイン型の impl 検索 *)
let test_lookup_builtin_impl () =
  let registry = empty () in

  (* impl Eq for i64 *)
  let impl_eq_i64 = {
    trait_name = "Eq";
    impl_type = TCon (TCInt I64);
    generic_params = [];
    where_constraints = [];
    methods = [("eq", "i64_eq")];
    span = dummy_span;
  } in

  let registry = register impl_eq_i64 registry in

  (* 検索: Eq for i64 *)
  let result = lookup "Eq" (TCon (TCInt I64)) registry in
  assert (result = Some impl_eq_i64);

  (* 検索: Eq for String (存在しない) *)
  let result = lookup "Eq" (TCon TCString) registry in
  assert (result = None);

  (* 検索: Ord for i64 (トレイト名が異なる) *)
  let result = lookup "Ord" (TCon (TCInt I64)) registry in
  assert (result = None);

  print_endline "✓ Test 3: ビルトイン型の impl 検索"

(** Test 4: ジェネリック型の impl 登録 *)
let test_register_generic_impl () =
  let registry = empty () in

  (* impl<T> Eq for Vec<T> where T: Eq *)
  let impl_eq_vec = {
    trait_name = "Eq";
    impl_type = TApp (TCon (TCUser "Vec"), TVar tv_t);
    generic_params = [tv_t];
    where_constraints = [
      { trait_name = "Eq"; type_args = [TVar tv_t]; constraint_span = dummy_span }
    ];
    methods = [("eq", "Vec_eq")];
    span = dummy_span;
  } in

  let registry = register impl_eq_vec registry in
  let impls = all_impls registry in

  assert (List.length impls = 1);
  print_endline "✓ Test 4: ジェネリック型の impl 登録"

(** Test 5: ジェネリック型の impl 検索（具体型で照合） *)
let test_lookup_generic_impl () =
  let registry = empty () in

  (* impl<T> Eq for Vec<T> *)
  let impl_eq_vec = {
    trait_name = "Eq";
    impl_type = TApp (TCon (TCUser "Vec"), TVar tv_t);
    generic_params = [tv_t];
    where_constraints = [];
    methods = [("eq", "Vec_eq")];
    span = dummy_span;
  } in

  let registry = register impl_eq_vec registry in

  (* 検索: Eq for Vec<i64> → 照合成功（T = i64） *)
  let result = lookup "Eq" (TApp (TCon (TCUser "Vec"), TCon (TCInt I64))) registry in
  assert (result <> None);

  (* 検索: Eq for Vec<String> → 照合成功（T = String） *)
  let result = lookup "Eq" (TApp (TCon (TCUser "Vec"), TCon TCString)) registry in
  assert (result <> None);

  (* 検索: Eq for i64 → 照合失敗 *)
  let result = lookup "Eq" (TCon (TCInt I64)) registry in
  assert (result = None);

  print_endline "✓ Test 5: ジェネリック型の impl 検索（具体型で照合）"

(** Test 6: 複数の impl 登録と検索 *)
let test_multiple_impls () =
  let registry = empty () in

  (* impl Eq for i64 *)
  let impl_eq_i64 = {
    trait_name = "Eq";
    impl_type = TCon (TCInt I64);
    generic_params = [];
    where_constraints = [];
    methods = [("eq", "i64_eq")];
    span = dummy_span;
  } in

  (* impl Eq for String *)
  let impl_eq_string = {
    trait_name = "Eq";
    impl_type = TCon TCString;
    generic_params = [];
    where_constraints = [];
    methods = [("eq", "String_eq")];
    span = dummy_span;
  } in

  (* impl Ord for i64 *)
  let impl_ord_i64 = {
    trait_name = "Ord";
    impl_type = TCon (TCInt I64);
    generic_params = [];
    where_constraints = [];
    methods = [("compare", "i64_compare")];
    span = dummy_span;
  } in

  let registry = register impl_eq_i64 registry in
  let registry = register impl_eq_string registry in
  let registry = register impl_ord_i64 registry in

  (* 検索: Eq for i64 *)
  let result = lookup "Eq" (TCon (TCInt I64)) registry in
  assert (result = Some impl_eq_i64);

  (* 検索: Eq for String *)
  let result = lookup "Eq" (TCon TCString) registry in
  assert (result = Some impl_eq_string);

  (* 検索: Ord for i64 *)
  let result = lookup "Ord" (TCon (TCInt I64)) registry in
  assert (result = Some impl_ord_i64);

  print_endline "✓ Test 6: 複数の impl 登録と検索"

(** Test 7: find_matching_impls - 単一マッチ *)
let test_find_matching_impls_single () =
  let registry = empty () in

  (* impl Eq for i64 *)
  let impl_eq_i64 = {
    trait_name = "Eq";
    impl_type = TCon (TCInt I64);
    generic_params = [];
    where_constraints = [];
    methods = [("eq", "i64_eq")];
    span = dummy_span;
  } in

  let registry = register impl_eq_i64 registry in

  (* 制約: Eq<i64> *)
  let constraint_ = {
    trait_name = "Eq";
    type_args = [TCon (TCInt I64)];
    constraint_span = dummy_span;
  } in

  let matches = find_matching_impls constraint_ registry in
  assert (List.length matches = 1);
  assert (List.hd matches = impl_eq_i64);

  print_endline "✓ Test 7: find_matching_impls - 単一マッチ"

(** Test 8: find_matching_impls - マッチなし *)
let test_find_matching_impls_none () =
  let registry = empty () in

  (* impl Eq for i64 *)
  let impl_eq_i64 = {
    trait_name = "Eq";
    impl_type = TCon (TCInt I64);
    generic_params = [];
    where_constraints = [];
    methods = [("eq", "i64_eq")];
    span = dummy_span;
  } in

  let registry = register impl_eq_i64 registry in

  (* 制約: Ord<i64> (トレイト名が異なる) *)
  let constraint_ = {
    trait_name = "Ord";
    type_args = [TCon (TCInt I64)];
    constraint_span = dummy_span;
  } in

  let matches = find_matching_impls constraint_ registry in
  assert (List.length matches = 0);

  print_endline "✓ Test 8: find_matching_impls - マッチなし"

(** Test 9: find_matching_impls - ジェネリック型マッチ *)
let test_find_matching_impls_generic () =
  let registry = empty () in

  (* impl<T> Eq for Vec<T> *)
  let impl_eq_vec = {
    trait_name = "Eq";
    impl_type = TApp (TCon (TCUser "Vec"), TVar tv_t);
    generic_params = [tv_t];
    where_constraints = [];
    methods = [("eq", "Vec_eq")];
    span = dummy_span;
  } in

  let registry = register impl_eq_vec registry in

  (* 制約: Eq<Vec<i64>> *)
  let constraint_ = {
    trait_name = "Eq";
    type_args = [TApp (TCon (TCUser "Vec"), TCon (TCInt I64))];
    constraint_span = dummy_span;
  } in

  let matches = find_matching_impls constraint_ registry in
  assert (List.length matches = 1);

  print_endline "✓ Test 9: find_matching_impls - ジェネリック型マッチ"

(** Test 10: 曖昧な impl の検出 *)
let test_ambiguous_impls () =
  let registry = empty () in

  (* impl Eq for i64 (1つ目) *)
  let impl_eq_i64_1 = {
    trait_name = "Eq";
    impl_type = TCon (TCInt I64);
    generic_params = [];
    where_constraints = [];
    methods = [("eq", "i64_eq_1")];
    span = dummy_span;
  } in

  (* impl Eq for i64 (2つ目、重複) *)
  let impl_eq_i64_2 = {
    trait_name = "Eq";
    impl_type = TCon (TCInt I64);
    generic_params = [];
    where_constraints = [];
    methods = [("eq", "i64_eq_2")];
    span = dummy_span;
  } in

  let registry = register impl_eq_i64_1 registry in
  let registry = register impl_eq_i64_2 registry in

  (* 制約: Eq<i64> *)
  let constraint_ = {
    trait_name = "Eq";
    type_args = [TCon (TCInt I64)];
    constraint_span = dummy_span;
  } in

  let matches = find_matching_impls constraint_ registry in
  assert (List.length matches = 2);  (* 曖昧 *)

  print_endline "✓ Test 10: 曖昧な impl の検出"

(** Test 11: タプル型の impl *)
let test_tuple_type_impl () =
  let registry = empty () in

  (* impl Eq for (i64, String) *)
  let impl_eq_tuple = {
    trait_name = "Eq";
    impl_type = TTuple [TCon (TCInt I64); TCon TCString];
    generic_params = [];
    where_constraints = [];
    methods = [("eq", "tuple_eq")];
    span = dummy_span;
  } in

  let registry = register impl_eq_tuple registry in

  (* 検索: Eq for (i64, String) *)
  let result = lookup "Eq" (TTuple [TCon (TCInt I64); TCon TCString]) registry in
  assert (result = Some impl_eq_tuple);

  (* 検索: Eq for (String, i64) → 順序が異なるので失敗 *)
  let result = lookup "Eq" (TTuple [TCon TCString; TCon (TCInt I64)]) registry in
  assert (result = None);

  print_endline "✓ Test 11: タプル型の impl"

(** Test 12: レコード型の impl *)
let test_record_type_impl () =
  let registry = empty () in

  (* impl Eq for { x: i64, y: String } *)
  let impl_eq_record = {
    trait_name = "Eq";
    impl_type = TRecord [("x", TCon (TCInt I64)); ("y", TCon TCString)];
    generic_params = [];
    where_constraints = [];
    methods = [("eq", "record_eq")];
    span = dummy_span;
  } in

  let registry = register impl_eq_record registry in

  (* 検索: Eq for { x: i64, y: String } *)
  let result = lookup "Eq" (TRecord [("x", TCon (TCInt I64)); ("y", TCon TCString)]) registry in
  assert (result = Some impl_eq_record);

  (* 検索: Eq for { y: String, x: i64 } → フィールド順が異なるが照合成功（ソートされる） *)
  let result = lookup "Eq" (TRecord [("y", TCon TCString); ("x", TCon (TCInt I64))]) registry in
  assert (result = Some impl_eq_record);

  print_endline "✓ Test 12: レコード型の impl"

(** Test 13: 関数型の impl *)
let test_function_type_impl () =
  let registry = empty () in

  (* impl Eq for (i64 -> String) *)
  let impl_eq_fn = {
    trait_name = "Eq";
    impl_type = TArrow (TCon (TCInt I64), TCon TCString);
    generic_params = [];
    where_constraints = [];
    methods = [("eq", "fn_eq")];
    span = dummy_span;
  } in

  let registry = register impl_eq_fn registry in

  (* 検索: Eq for (i64 -> String) *)
  let result = lookup "Eq" (TArrow (TCon (TCInt I64), TCon TCString)) registry in
  assert (result = Some impl_eq_fn);

  print_endline "✓ Test 13: 関数型の impl"

(** Test 14: ネストしたジェネリック型 *)
let test_nested_generic_impl () =
  let registry = empty () in

  (* impl<T> Eq for Vec<Vec<T>> *)
  let impl_eq_nested = {
    trait_name = "Eq";
    impl_type = TApp (TCon (TCUser "Vec"), TApp (TCon (TCUser "Vec"), TVar tv_t));
    generic_params = [tv_t];
    where_constraints = [];
    methods = [("eq", "Vec_Vec_eq")];
    span = dummy_span;
  } in

  let registry = register impl_eq_nested registry in

  (* 検索: Eq for Vec<Vec<i64>> → 照合成功 *)
  let result = lookup "Eq"
    (TApp (TCon (TCUser "Vec"), TApp (TCon (TCUser "Vec"), TCon (TCInt I64))))
    registry in
  assert (result <> None);

  print_endline "✓ Test 14: ネストしたジェネリック型"

(** Test 15: string_of_impl_info のテスト *)
let test_string_of_impl_info () =
  (* impl<T> Eq for Vec<T> where T: Eq *)
  let impl_info = {
    trait_name = "Eq";
    impl_type = TApp (TCon (TCUser "Vec"), TVar tv_t);
    generic_params = [tv_t];
    where_constraints = [
      { trait_name = "Eq"; type_args = [TVar tv_t]; constraint_span = dummy_span }
    ];
    methods = [("eq", "Vec_eq"); ("ne", "Vec_ne")];
    span = dummy_span;
  } in

  let str = string_of_impl_info impl_info in
  (* 出力例: "impl<T> Eq for Vec<T> where Eq<T> { eq, ne }" *)
  assert (String.length str > 0);
  assert (String.contains str '<');  (* ジェネリック型パラメータ *)
  assert (String.contains str '{');  (* メソッド *)

  print_endline "✓ Test 15: string_of_impl_info のテスト"

(** Test 16: string_of_registry のテスト *)
let test_string_of_registry () =
  let registry = empty () in

  (* 空のレジストリ *)
  let str = string_of_registry registry in
  assert (String.length str > 0);

  (* impl を登録 *)
  let impl_eq_i64 = {
    trait_name = "Eq";
    impl_type = TCon (TCInt I64);
    generic_params = [];
    where_constraints = [];
    methods = [("eq", "i64_eq")];
    span = dummy_span;
  } in

  let registry = register impl_eq_i64 registry in
  let str = string_of_registry registry in
  assert (String.length str > 0);

  print_endline "✓ Test 16: string_of_registry のテスト"

(* ========== テストスイート実行 ========== *)

let run_tests () =
  print_endline "=== Impl_registry Tests ===";
  print_endline "";

  (* 基本操作 *)
  test_empty_registry ();
  test_register_builtin_impl ();
  test_lookup_builtin_impl ();

  (* ジェネリック型 *)
  test_register_generic_impl ();
  test_lookup_generic_impl ();

  (* 複数 impl *)
  test_multiple_impls ();

  (* find_matching_impls *)
  test_find_matching_impls_single ();
  test_find_matching_impls_none ();
  test_find_matching_impls_generic ();
  test_ambiguous_impls ();

  (* 複合型 *)
  test_tuple_type_impl ();
  test_record_type_impl ();
  test_function_type_impl ();
  test_nested_generic_impl ();

  (* デバッグ用関数 *)
  test_string_of_impl_info ();
  test_string_of_registry ();

  print_endline "";
  print_endline "All Impl_registry tests passed! (16/16)";
  print_endline ""

let () = run_tests ()
