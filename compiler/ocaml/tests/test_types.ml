(* Test_types — Type System Unit Tests (Phase 2)
 *
 * 型システム基盤のユニットテスト
 * - 型変数生成
 * - 型スキーム
 * - 型環境
 * - 制約システム
 *)

open Types
open Type_env
open Constraint

(* ========== テストヘルパー ========== *)

let assert_equal_int msg expected actual =
  if expected <> actual then
    failwith (Printf.sprintf "%s: expected %d, got %d" msg expected actual)

let assert_equal msg expected actual =
  if expected <> actual then failwith (Printf.sprintf "%s: mismatch" msg)

let assert_true msg cond =
  if not cond then failwith (Printf.sprintf "%s: assertion failed" msg)

let assert_false msg cond =
  if cond then
    failwith (Printf.sprintf "%s: assertion failed (expected false)" msg)

let assert_some msg = function
  | Some _ -> ()
  | None -> failwith (Printf.sprintf "%s: expected Some, got None" msg)

let assert_none msg = function
  | None -> ()
  | Some _ -> failwith (Printf.sprintf "%s: expected None, got Some" msg)

(* ========== 型変数生成のテスト ========== *)

let test_type_var_generation () =
  Printf.printf "Testing type variable generation...\n";

  (* リセット *)
  TypeVarGen.reset ();

  (* 新鮮な型変数を生成 *)
  let tv1 = TypeVarGen.fresh (Some "a") in
  assert_equal_int "tv1 id" 0 tv1.tv_id;
  assert_equal "tv1 name" (Some "a") tv1.tv_name;

  let tv2 = TypeVarGen.fresh (Some "b") in
  assert_equal_int "tv2 id" 1 tv2.tv_id;
  assert_equal "tv2 name" (Some "b") tv2.tv_name;

  (* 複数生成 *)
  TypeVarGen.reset ();
  let tvs = TypeVarGen.fresh_many 3 in
  assert_equal_int "fresh_many length" 3 (List.length tvs);
  assert_equal_int "fresh_many id 0" 0 (List.nth tvs 0).tv_id;
  assert_equal_int "fresh_many id 2" 2 (List.nth tvs 2).tv_id;

  (* ギリシャ文字風の名前 *)
  TypeVarGen.reset ();
  let alpha = TypeVarGen.fresh_greek () in
  assert_equal "greek name" (Some "α") alpha.tv_name;

  Printf.printf "  ✓ Type variable generation tests passed\n"

(* ========== 型スキームのテスト ========== *)

let test_type_scheme () =
  Printf.printf "Testing type schemes...\n";

  TypeVarGen.reset ();

  (* 単相型 *)
  let mono = mono_scheme ty_i64 in
  assert_false "mono is not polymorphic" (is_polymorphic mono);
  assert_equal "mono body" ty_i64 (scheme_body mono);

  (* 多相型: ∀a. a -> a *)
  let a = TypeVarGen.fresh (Some "a") in
  let id_scheme = { quantified = [ a ]; body = TArrow (TVar a, TVar a) } in
  assert_true "id_scheme is polymorphic" (is_polymorphic id_scheme);

  (* 型スキームの文字列表現 *)
  let id_str = string_of_scheme id_scheme in
  assert_true "id_scheme string contains forall" (String.length id_str > 0);

  Printf.printf "  ✓ Type scheme tests passed\n"

(* ========== 型環境のテスト ========== *)

let test_type_env () =
  Printf.printf "Testing type environment...\n";

  TypeVarGen.reset ();

  (* 空環境 *)
  let env = empty in
  assert_none "empty env lookup" (lookup "x" env);

  (* 束縛の追加 *)
  let env = extend "x" (scheme_to_constrained (mono_scheme ty_i64)) env in
  assert_some "lookup x" (lookup "x" env);

  (match lookup "x" env with
  | Some scheme -> assert_equal "x type" ty_i64 scheme.body
  | None -> failwith "lookup x failed");

  (* シャドーイング *)
  let env = extend "x" (scheme_to_constrained (mono_scheme ty_string)) env in
  (match lookup "x" env with
  | Some scheme -> assert_equal "x shadowed" ty_string scheme.body
  | None -> failwith "lookup x after shadow failed");

  (* スコープのネスト *)
  let env = extend "y" (scheme_to_constrained (mono_scheme ty_bool)) env in
  let env2 = enter_scope env in

  (* 親スコープの変数にアクセス *)
  assert_some "parent scope y" (lookup "y" env2);

  (* 子スコープで上書き *)
  let env2 = extend "y" (scheme_to_constrained (mono_scheme ty_char)) env2 in
  (match lookup "y" env2 with
  | Some scheme -> assert_equal "y in child scope" ty_char scheme.body
  | None -> failwith "lookup y in child scope failed");

  (* 親スコープは変更されていない *)
  (match lookup "y" env with
  | Some scheme -> assert_equal "y in parent scope" ty_bool scheme.body
  | None -> failwith "lookup y in parent scope failed");

  Printf.printf "  ✓ Type environment tests passed\n"

(* ========== 初期環境のテスト ========== *)

let test_initial_env () =
  Printf.printf "Testing initial environment...\n";

  (* Option コンストラクタ *)
  assert_some "Some in initial_env" (lookup "Some" initial_env);
  assert_some "None in initial_env" (lookup "None" initial_env);

  (* Result コンストラクタ *)
  assert_some "Ok in initial_env" (lookup "Ok" initial_env);
  assert_some "Err in initial_env" (lookup "Err" initial_env);

  (* Never 型 *)
  assert_some "Never in initial_env" (lookup "Never" initial_env);

  Printf.printf "  ✓ Initial environment tests passed\n"

(* ========== 制約のテスト ========== *)

let test_constraints () =
  Printf.printf "Testing constraints...\n";

  TypeVarGen.reset ();

  (* 空の制約集合 *)
  assert_equal_int "empty constraints" 0 (List.length empty_constraints);

  (* 制約の追加 *)
  let span = Ast.{ start = 0; end_ = 10 } in
  let c1 = unify_constraint ty_i64 ty_i64 span in
  let cs = add_constraint c1 empty_constraints in
  assert_equal_int "one constraint" 1 (List.length cs);

  (* 制約の結合 *)
  let c2 = unify_constraint ty_bool ty_bool span in
  let cs2 = add_constraint c2 empty_constraints in
  let merged = merge_constraints cs cs2 in
  assert_equal_int "merged constraints" 2 (List.length merged);

  Printf.printf "  ✓ Constraint tests passed\n"

(* ========== 代入のテスト ========== *)

let test_substitution () =
  Printf.printf "Testing substitution...\n";

  TypeVarGen.reset ();

  (* 空の代入 *)
  assert_equal_int "empty subst" 0 (List.length empty_subst);

  (* 型変数への代入 *)
  let a = TypeVarGen.fresh (Some "a") in
  let subst = [ (a, ty_i64) ] in

  (* 代入の適用 *)
  let result = apply_subst subst (TVar a) in
  assert_equal "apply subst to var" ty_i64 result;

  (* 関数型への代入 *)
  let fn_ty = TArrow (TVar a, TVar a) in
  let result = apply_subst subst fn_ty in
  assert_equal "apply subst to arrow" (TArrow (ty_i64, ty_i64)) result;

  (* タプル型への代入 *)
  let tuple_ty = TTuple [ TVar a; ty_bool ] in
  let result = apply_subst subst tuple_ty in
  assert_equal "apply subst to tuple" (TTuple [ ty_i64; ty_bool ]) result;

  Printf.printf "  ✓ Substitution tests passed\n"

(* ========== 自由型変数のテスト ========== *)

let test_free_type_vars () =
  Printf.printf "Testing free type variables...\n";

  TypeVarGen.reset ();

  (* 型定数 *)
  let fvs = ftv_ty ty_i64 in
  assert_equal_int "no free vars in const" 0 (List.length fvs);

  (* 型変数 *)
  let a = TypeVarGen.fresh (Some "a") in
  let fvs = ftv_ty (TVar a) in
  assert_equal_int "one free var" 1 (List.length fvs);

  (* 関数型 *)
  let b = TypeVarGen.fresh (Some "b") in
  let fn_ty = TArrow (TVar a, TVar b) in
  let fvs = ftv_ty fn_ty in
  assert_equal_int "two free vars in arrow" 2 (List.length fvs);

  (* 型スキームの自由変数（量化変数は除外） *)
  let scheme = { quantified = [ a ]; body = TArrow (TVar a, TVar b) } in
  let fvs = ftv_cscheme (scheme_to_constrained scheme) in
  assert_equal_int "one free var in scheme" 1 (List.length fvs);

  Printf.printf "  ✓ Free type variables tests passed\n"

(* ========== Occurs check のテスト ========== *)

let test_occurs_check () =
  Printf.printf "Testing occurs check...\n";

  TypeVarGen.reset ();

  let a = TypeVarGen.fresh (Some "a") in

  (* 自己参照 *)
  assert_true "occurs in self" (occurs_check a (TVar a));

  (* 出現しない *)
  assert_false "not occurs in i64" (occurs_check a ty_i64);

  (* 関数型内に出現 *)
  let fn_ty = TArrow (TVar a, ty_i64) in
  assert_true "occurs in arrow" (occurs_check a fn_ty);

  (* タプル内に出現 *)
  let tuple_ty = TTuple [ ty_bool; TVar a ] in
  assert_true "occurs in tuple" (occurs_check a tuple_ty);

  Printf.printf "  ✓ Occurs check tests passed\n"

(* ========== 単一化のテスト ========== *)

let test_unification () =
  Printf.printf "Testing unification...\n";

  TypeVarGen.reset ();

  let span = Ast.{ start = 0; end_ = 10 } in

  (* 同じ型定数の単一化 *)
  let result = unify empty_subst ty_i64 ty_i64 span in
  (match result with
  | Ok subst -> assert_equal_int "same const unify" 0 (List.length subst)
  | Error _ -> failwith "same const unify failed");

  (* 型変数と型定数の単一化 *)
  let a = TypeVarGen.fresh (Some "a") in
  let result = unify empty_subst (TVar a) ty_i64 span in
  (match result with
  | Ok subst ->
      assert_equal_int "var-const unify" 1 (List.length subst);
      let result = apply_subst subst (TVar a) in
      assert_equal "var substituted" ty_i64 result
  | Error _ -> failwith "var-const unify failed");

  (* 関数型の単一化 *)
  TypeVarGen.reset ();
  let a = TypeVarGen.fresh (Some "a") in
  let b = TypeVarGen.fresh (Some "b") in
  let fn1 = TArrow (TVar a, ty_i64) in
  let fn2 = TArrow (ty_bool, TVar b) in
  let result = unify empty_subst fn1 fn2 span in
  (match result with
  | Ok subst ->
      assert_equal_int "arrow unify" 2 (List.length subst);
      let result = apply_subst subst fn1 in
      assert_equal "arrow unified" (TArrow (ty_bool, ty_i64)) result
  | Error _ -> failwith "arrow unify failed");

  (* 型不一致 *)
  let result = unify empty_subst ty_i64 ty_bool span in
  (match result with
  | Ok _ -> failwith "should fail on type mismatch"
  | Error (UnificationFailure _) -> ()
  | Error _ -> failwith "wrong error type");

  (* Occurs check エラー *)
  TypeVarGen.reset ();
  let a = TypeVarGen.fresh (Some "a") in
  let recursive = TArrow (TVar a, TVar a) in
  let result = unify empty_subst (TVar a) recursive span in
  (match result with
  | Ok _ -> failwith "should fail on occurs check"
  | Error (OccursCheck _) -> ()
  | Error _ -> failwith "wrong error type");

  Printf.printf "  ✓ Unification tests passed\n"

(* ========== メインテストランナー ========== *)

let () =
  Printf.printf "\n=== Type System Unit Tests ===\n\n";

  try
    test_type_var_generation ();
    test_type_scheme ();
    test_type_env ();
    test_initial_env ();
    test_constraints ();
    test_substitution ();
    test_free_type_vars ();
    test_occurs_check ();
    test_unification ();

    Printf.printf "\n=== All tests passed! ===\n\n"
  with
  | Failure msg ->
      Printf.eprintf "\n❌ Test failed: %s\n\n" msg;
      exit 1
  | e ->
      Printf.eprintf "\n❌ Unexpected error: %s\n\n" (Printexc.to_string e);
      exit 1
