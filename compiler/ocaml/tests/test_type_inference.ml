(* 型推論テストスイート
 *
 * Phase 2 Week 4: パターンマッチの型推論テスト
 *
 * テスト対象:
 * - 基本パターン（変数、ワイルドカード、リテラル）
 * - タプルパターン
 * - コンストラクタパターン（Option、Result）
 * - ネストパターン（2層、3層）
 * - ガード付きパターン
 * - レコードパターン
 * - エラーケース
 *)

open Types
open Type_env
open Type_inference
open Parser_driver
open Ast

(* ========== テストヘルパー ========== *)

let reset_types () =
  TypeVarGen.reset ()

let _parse_expr_string src =
  match parse_string src with
  | Result.Ok cu ->
      (* 最初の式文を取得 *)
      (match cu.decls with
       | [{ decl_kind = LetDecl (_, _, expr); _ }] -> Some expr
       | _ -> None)
  | Result.Error _ -> None

let test_name = ref ""

let run_test name f =
  test_name := name;
  reset_types ();
  try
    f ();
    Printf.printf "✓ %s\n" name
  with
  | Failure msg ->
      Printf.printf "✗ %s: %s\n" name msg;
      exit 1
  | e ->
      Printf.printf "✗ %s: %s\n" name (Printexc.to_string e);
      exit 1

let assert_ok result msg =
  match result with
  | Ok _ -> ()
  | Error e ->
      let error_msg = Type_error.string_of_error e in
      failwith (Printf.sprintf "%s\nError: %s" msg error_msg)

let assert_type_eq expected actual msg =
  if not (Types.type_equal expected actual) then
    failwith (Printf.sprintf "%s\n  Expected: %s\n  Actual:   %s"
      msg (Types.string_of_ty expected) (Types.string_of_ty actual))

(* ========== 基本パターンテスト ========== *)

let test_basic_patterns () =
  Printf.printf "Basic Pattern Tests:\n";

  (* 変数パターン *)
  run_test "infer_pattern: PatVar" (fun () ->
    let env = initial_env in
    let pat = {
      pat_kind = PatVar { name = "x"; span = dummy_span };
      pat_span = dummy_span;
    } in
    let expected_ty = ty_i64 in
    let result = infer_pattern env pat expected_ty in
    assert_ok result "Variable pattern should succeed";
    match result with
    | Ok (tpat, _) ->
        assert_type_eq expected_ty tpat.tpat_ty "Variable pattern type"
    | Error _ -> failwith "Should not reach here"
  );

  (* ワイルドカードパターン *)
  run_test "infer_pattern: PatWildcard" (fun () ->
    let env = initial_env in
    let pat = {
      pat_kind = PatWildcard;
      pat_span = dummy_span;
    } in
    let expected_ty = ty_i64 in
    let result = infer_pattern env pat expected_ty in
    assert_ok result "Wildcard pattern should succeed";
    match result with
    | Ok (tpat, _) ->
        assert_type_eq expected_ty tpat.tpat_ty "Wildcard pattern type"
    | Error _ -> failwith "Should not reach here"
  );

  (* リテラルパターン *)
  run_test "infer_pattern: PatLiteral (Int)" (fun () ->
    let env = initial_env in
    let pat = {
      pat_kind = PatLiteral (Int ("42", Base10));
      pat_span = dummy_span;
    } in
    let expected_ty = ty_i64 in
    let result = infer_pattern env pat expected_ty in
    assert_ok result "Literal pattern should succeed";
    match result with
    | Ok (tpat, _) ->
        assert_type_eq expected_ty tpat.tpat_ty "Literal pattern type"
    | Error _ -> failwith "Should not reach here"
  )

(* ========== タプルパターンテスト ========== *)

let test_tuple_patterns () =
  Printf.printf "\nTuple Pattern Tests:\n";

  (* 単純なタプルパターン *)
  run_test "infer_pattern: PatTuple (x, y)" (fun () ->
    let env = initial_env in
    let pat = {
      pat_kind = PatTuple [
        { pat_kind = PatVar { name = "x"; span = dummy_span }; pat_span = dummy_span };
        { pat_kind = PatVar { name = "y"; span = dummy_span }; pat_span = dummy_span };
      ];
      pat_span = dummy_span;
    } in
    let expected_ty = TTuple [ty_i64; ty_f64] in
    let result = infer_pattern env pat expected_ty in
    assert_ok result "Tuple pattern should succeed";
    match result with
    | Ok (tpat, _) ->
        assert_type_eq expected_ty tpat.tpat_ty "Tuple pattern type"
    | Error _ -> failwith "Should not reach here"
  )

(* ========== コンストラクタパターンテスト ========== *)

let test_constructor_patterns () =
  Printf.printf "\nConstructor Pattern Tests:\n";

  (* Some(x) パターン *)
  run_test "infer_pattern: Some(x)" (fun () ->
    let env = initial_env in
    let pat = {
      pat_kind = PatConstructor (
        { name = "Some"; span = dummy_span },
        [{ pat_kind = PatVar { name = "x"; span = dummy_span }; pat_span = dummy_span }]
      );
      pat_span = dummy_span;
    } in
    let expected_ty = ty_option ty_i64 in
    let result = infer_pattern env pat expected_ty in
    assert_ok result "Some(x) pattern should succeed";
    match result with
    | Ok (tpat, _) ->
        assert_type_eq expected_ty tpat.tpat_ty "Some(x) pattern type"
    | Error _ -> failwith "Should not reach here"
  );

  (* None パターン *)
  run_test "infer_pattern: None" (fun () ->
    let env = initial_env in
    let pat = {
      pat_kind = PatConstructor (
        { name = "None"; span = dummy_span },
        []
      );
      pat_span = dummy_span;
    } in
    let expected_ty = ty_option ty_i64 in
    let result = infer_pattern env pat expected_ty in
    assert_ok result "None pattern should succeed";
    match result with
    | Ok (tpat, _) ->
        assert_type_eq expected_ty tpat.tpat_ty "None pattern type"
    | Error _ -> failwith "Should not reach here"
  )

(* ========== ネストパターンテスト ========== *)

let test_nested_patterns () =
  Printf.printf "\nNested Pattern Tests:\n";

  (* Some(Some(x)) パターン *)
  run_test "infer_pattern: Some(Some(x))" (fun () ->
    let env = initial_env in
    let pat = {
      pat_kind = PatConstructor (
        { name = "Some"; span = dummy_span },
        [{
          pat_kind = PatConstructor (
            { name = "Some"; span = dummy_span },
            [{ pat_kind = PatVar { name = "x"; span = dummy_span }; pat_span = dummy_span }]
          );
          pat_span = dummy_span;
        }]
      );
      pat_span = dummy_span;
    } in
    let expected_ty = ty_option (ty_option ty_i64) in
    let result = infer_pattern env pat expected_ty in
    assert_ok result "Some(Some(x)) pattern should succeed";
    match result with
    | Ok (tpat, _) ->
        assert_type_eq expected_ty tpat.tpat_ty "Some(Some(x)) pattern type"
    | Error _ -> failwith "Should not reach here"
  )

(* ========== match式テスト ========== *)

let test_match_expressions () =
  Printf.printf "\nMatch Expression Tests:\n";

  (* 単純なmatch式: Option<i64> *)
  run_test "infer_expr: match Some(42) with ..." (fun () ->
    let env = initial_env in

    (* Some(42) *)
    let scrutinee = {
      expr_kind = Call (
        { expr_kind = Var { name = "Some"; span = dummy_span }; expr_span = dummy_span },
        [PosArg { expr_kind = Literal (Int ("42", Base10)); expr_span = dummy_span }]
      );
      expr_span = dummy_span;
    } in

    (* match arms *)
    let arms = [
      (* Some(x) -> x *)
      {
        arm_pattern = {
          pat_kind = PatConstructor (
            { name = "Some"; span = dummy_span },
            [{ pat_kind = PatVar { name = "x"; span = dummy_span }; pat_span = dummy_span }]
          );
          pat_span = dummy_span;
        };
        arm_guard = None;
        arm_body = { expr_kind = Var { name = "x"; span = dummy_span }; expr_span = dummy_span };
        arm_span = dummy_span;
      };
      (* None -> 0 *)
      {
        arm_pattern = {
          pat_kind = PatConstructor ({ name = "None"; span = dummy_span }, []);
          pat_span = dummy_span;
        };
        arm_guard = None;
        arm_body = { expr_kind = Literal (Int ("0", Base10)); expr_span = dummy_span };
        arm_span = dummy_span;
      };
    ] in

    let match_expr = {
      expr_kind = Match (scrutinee, arms);
      expr_span = dummy_span;
    } in

    let result = infer_expr env match_expr in
    assert_ok result "Match expression should succeed";
    match result with
    | Ok (_, ty, _) ->
        assert_type_eq ty_i64 ty "Match expression result type"
    | Error _ -> failwith "Should not reach here"
  );

  (* ネストしたmatch: Option<Option<i64>> *)
  run_test "infer_expr: nested match Some(Some(x))" (fun () ->
    let env = initial_env in

    (* スクラティニー: 型変数にする *)
    let scrutinee = {
      expr_kind = Var { name = "nested_opt"; span = dummy_span };
      expr_span = dummy_span;
    } in

    (* nested_opt を環境に追加 *)
    let env = extend "nested_opt"
      (mono_scheme (ty_option (ty_option ty_i64)))
      env in

    let arms = [
      (* Some(Some(x)) -> x *)
      {
        arm_pattern = {
          pat_kind = PatConstructor (
            { name = "Some"; span = dummy_span },
            [{
              pat_kind = PatConstructor (
                { name = "Some"; span = dummy_span },
                [{ pat_kind = PatVar { name = "x"; span = dummy_span }; pat_span = dummy_span }]
              );
              pat_span = dummy_span;
            }]
          );
          pat_span = dummy_span;
        };
        arm_guard = None;
        arm_body = { expr_kind = Var { name = "x"; span = dummy_span }; expr_span = dummy_span };
        arm_span = dummy_span;
      };
      (* _ -> 0 *)
      {
        arm_pattern = { pat_kind = PatWildcard; pat_span = dummy_span };
        arm_guard = None;
        arm_body = { expr_kind = Literal (Int ("0", Base10)); expr_span = dummy_span };
        arm_span = dummy_span;
      };
    ] in

    let match_expr = {
      expr_kind = Match (scrutinee, arms);
      expr_span = dummy_span;
    } in

    let result = infer_expr env match_expr in
    assert_ok result "Nested match expression should succeed";
    match result with
    | Ok (_, ty, _) ->
        assert_type_eq ty_i64 ty "Nested match expression result type"
    | Error _ -> failwith "Should not reach here"
  )

(* ========== ブロック式テスト ========== *)

let test_block_expressions () =
  Printf.printf "\nBlock Expression Tests:\n";

  (* 空のブロック *)
  run_test "infer_expr: empty block {}" (fun () ->
    let env = initial_env in
    let block_expr = {
      expr_kind = Block [];
      expr_span = dummy_span;
    } in
    let result = infer_expr env block_expr in
    assert_ok result "Empty block should succeed";
    match result with
    | Ok (_, ty, _) ->
        assert_type_eq ty_unit ty "Empty block type"
    | Error _ -> failwith "Should not reach here"
  );

  (* 式のみのブロック *)
  run_test "infer_expr: block with single expr { 42 }" (fun () ->
    let env = initial_env in
    let block_expr = {
      expr_kind = Block [
        ExprStmt { expr_kind = Literal (Int ("42", Base10)); expr_span = dummy_span }
      ];
      expr_span = dummy_span;
    } in
    let result = infer_expr env block_expr in
    assert_ok result "Block with single expr should succeed";
    match result with
    | Ok (_, ty, _) ->
        assert_type_eq ty_i64 ty "Block with single expr type"
    | Error _ -> failwith "Should not reach here"
  );

  (* let束縛を含むブロック（簡易版：二項演算を使わない） *)
  run_test "infer_expr: block with let binding { let x = 1; x }" (fun () ->
    let env = initial_env in
    (* let x = 1 *)
    let let_decl = {
      decl_attrs = [];
      decl_vis = Private;
      decl_kind = LetDecl (
        { pat_kind = PatVar { name = "x"; span = dummy_span }; pat_span = dummy_span },
        None,
        { expr_kind = Literal (Int ("1", Base10)); expr_span = dummy_span }
      );
      decl_span = dummy_span;
    } in
    (* x *)
    let x_expr = {
      expr_kind = Var { name = "x"; span = dummy_span };
      expr_span = dummy_span;
    } in
    let block_expr = {
      expr_kind = Block [
        DeclStmt let_decl;
        ExprStmt x_expr;
      ];
      expr_span = dummy_span;
    } in
    let result = infer_expr env block_expr in
    assert_ok result "Block with let binding should succeed";
    match result with
    | Ok (_, ty, _) ->
        assert_type_eq ty_i64 ty "Block with let binding type"
    | Error _ -> failwith "Should not reach here"
  );

  (* 複数の文を含むブロック *)
  run_test "infer_expr: block with multiple statements" (fun () ->
    let env = initial_env in
    (* let x = 1 *)
    let let_x = {
      decl_attrs = [];
      decl_vis = Private;
      decl_kind = LetDecl (
        { pat_kind = PatVar { name = "x"; span = dummy_span }; pat_span = dummy_span },
        None,
        { expr_kind = Literal (Int ("1", Base10)); expr_span = dummy_span }
      );
      decl_span = dummy_span;
    } in
    (* let y = 2 *)
    let let_y = {
      decl_attrs = [];
      decl_vis = Private;
      decl_kind = LetDecl (
        { pat_kind = PatVar { name = "y"; span = dummy_span }; pat_span = dummy_span },
        None,
        { expr_kind = Literal (Int ("2", Base10)); expr_span = dummy_span }
      );
      decl_span = dummy_span;
    } in
    (* x (最後の式) *)
    let x_expr = {
      expr_kind = Var { name = "x"; span = dummy_span };
      expr_span = dummy_span;
    } in
    let block_expr = {
      expr_kind = Block [
        DeclStmt let_x;
        DeclStmt let_y;
        ExprStmt x_expr;
      ];
      expr_span = dummy_span;
    } in
    let result = infer_expr env block_expr in
    assert_ok result "Block with multiple statements should succeed";
    match result with
    | Ok (_, ty, _) ->
        assert_type_eq ty_i64 ty "Block with multiple statements type"
    | Error _ -> failwith "Should not reach here"
  );

  (* ブロックの最後が宣言文 → Unit型 *)
  run_test "infer_expr: block ending with decl -> Unit" (fun () ->
    let env = initial_env in
    let let_decl = {
      decl_attrs = [];
      decl_vis = Private;
      decl_kind = LetDecl (
        { pat_kind = PatVar { name = "x"; span = dummy_span }; pat_span = dummy_span },
        None,
        { expr_kind = Literal (Int ("42", Base10)); expr_span = dummy_span }
      );
      decl_span = dummy_span;
    } in
    let block_expr = {
      expr_kind = Block [DeclStmt let_decl];
      expr_span = dummy_span;
    } in
    let result = infer_expr env block_expr in
    assert_ok result "Block ending with decl should succeed";
    match result with
    | Ok (_, ty, _) ->
        assert_type_eq ty_unit ty "Block ending with decl type"
    | Error _ -> failwith "Should not reach here"
  );

  (* ネストしたブロック *)
  run_test "infer_expr: nested blocks" (fun () ->
    let env = initial_env in
    (* 内側のブロック: { 42 } *)
    let inner_block = {
      expr_kind = Block [
        ExprStmt { expr_kind = Literal (Int ("42", Base10)); expr_span = dummy_span }
      ];
      expr_span = dummy_span;
    } in
    (* 外側のブロック: { { 42 } } *)
    let outer_block = {
      expr_kind = Block [ExprStmt inner_block];
      expr_span = dummy_span;
    } in
    let result = infer_expr env outer_block in
    assert_ok result "Nested blocks should succeed";
    match result with
    | Ok (_, ty, _) ->
        assert_type_eq ty_i64 ty "Nested blocks type"
    | Error _ -> failwith "Should not reach here"
  )

(* ========== メイン ========== *)

let () =
  test_basic_patterns ();
  test_tuple_patterns ();
  test_constructor_patterns ();
  test_nested_patterns ();
  test_match_expressions ();
  test_block_expressions ();
  Printf.printf "\nAll type inference tests passed! ✓\n"
