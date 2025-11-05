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
open Ast
open Typed_ast

(* ========== テストヘルパー ========== *)

let reset_types () = TypeVarGen.reset ()

let parse_string = Test_support.parse_string

let _parse_expr_string src =
  match parse_string src with
  | Result.Ok cu -> (
      (* 最初の式文を取得 *)
      match cu.decls with
      | [ { decl_kind = LetDecl (_, _, expr); _ } ] -> Some expr
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
  | Result.Ok _ -> ()
  | Result.Error e ->
      let error_msg = Type_error.string_of_error e in
      failwith (Printf.sprintf "%s\nError: %s" msg error_msg)

let assert_type_eq expected actual msg =
  if not (Types.type_equal expected actual) then
    failwith
      (Printf.sprintf "%s\n  Expected: %s\n  Actual:   %s" msg
         (Types.string_of_ty expected)
         (Types.string_of_ty actual))

(* ========== 基本パターンテスト ========== *)

let test_basic_patterns () =
  Printf.printf "Basic Pattern Tests:\n";

  (* 変数パターン *)
  run_test "infer_pattern: PatVar" (fun () ->
      let env = initial_env in
      let pat =
        {
          pat_kind = PatVar { name = "x"; span = dummy_span };
          pat_span = dummy_span;
        }
      in
      let expected_ty = ty_i64 in
      let result = infer_pattern env pat expected_ty in
      assert_ok result "Variable pattern should succeed";
      match result with
      | Ok (tpat, _) ->
          assert_type_eq expected_ty tpat.tpat_ty "Variable pattern type"
      | Error _ -> failwith "Should not reach here");

  (* ワイルドカードパターン *)
  run_test "infer_pattern: PatWildcard" (fun () ->
      let env = initial_env in
      let pat = { pat_kind = PatWildcard; pat_span = dummy_span } in
      let expected_ty = ty_i64 in
      let result = infer_pattern env pat expected_ty in
      assert_ok result "Wildcard pattern should succeed";
      match result with
      | Ok (tpat, _) ->
          assert_type_eq expected_ty tpat.tpat_ty "Wildcard pattern type"
      | Error _ -> failwith "Should not reach here");

  (* リテラルパターン *)
  run_test "infer_pattern: PatLiteral (Int)" (fun () ->
      let env = initial_env in
      let pat =
        { pat_kind = PatLiteral (Int ("42", Base10)); pat_span = dummy_span }
      in
      let expected_ty = ty_i64 in
      let result = infer_pattern env pat expected_ty in
      assert_ok result "Literal pattern should succeed";
      match result with
      | Ok (tpat, _) ->
          assert_type_eq expected_ty tpat.tpat_ty "Literal pattern type"
      | Error _ -> failwith "Should not reach here")

(* ========== タプルパターンテスト ========== *)

let test_tuple_patterns () =
  Printf.printf "\nTuple Pattern Tests:\n";

  (* 単純なタプルパターン *)
  run_test "infer_pattern: PatTuple (x, y)" (fun () ->
      let env = initial_env in
      let pat =
        {
          pat_kind =
            PatTuple
              [
                {
                  pat_kind = PatVar { name = "x"; span = dummy_span };
                  pat_span = dummy_span;
                };
                {
                  pat_kind = PatVar { name = "y"; span = dummy_span };
                  pat_span = dummy_span;
                };
              ];
          pat_span = dummy_span;
        }
      in
      let expected_ty = TTuple [ ty_i64; ty_f64 ] in
      let result = infer_pattern env pat expected_ty in
      assert_ok result "Tuple pattern should succeed";
      match result with
      | Ok (tpat, _) ->
          assert_type_eq expected_ty tpat.tpat_ty "Tuple pattern type"
      | Error _ -> failwith "Should not reach here")

(* ========== コンストラクタパターンテスト ========== *)

let test_constructor_patterns () =
  Printf.printf "\nConstructor Pattern Tests:\n";

  (* Some(x) パターン *)
  run_test "infer_pattern: Some(x)" (fun () ->
      let env = initial_env in
      let pat =
        {
          pat_kind =
            PatConstructor
              ( { name = "Some"; span = dummy_span },
                [
                  {
                    pat_kind = PatVar { name = "x"; span = dummy_span };
                    pat_span = dummy_span;
                  };
                ] );
          pat_span = dummy_span;
        }
      in
      let expected_ty = ty_option ty_i64 in
      let result = infer_pattern env pat expected_ty in
      assert_ok result "Some(x) pattern should succeed";
      match result with
      | Ok (tpat, _) ->
          assert_type_eq expected_ty tpat.tpat_ty "Some(x) pattern type"
      | Error _ -> failwith "Should not reach here");

  (* None パターン *)
  run_test "infer_pattern: None" (fun () ->
      let env = initial_env in
      let pat =
        {
          pat_kind = PatConstructor ({ name = "None"; span = dummy_span }, []);
          pat_span = dummy_span;
        }
      in
      let expected_ty = ty_option ty_i64 in
      let result = infer_pattern env pat expected_ty in
      assert_ok result "None pattern should succeed";
      match result with
      | Ok (tpat, _) ->
          assert_type_eq expected_ty tpat.tpat_ty "None pattern type"
      | Error _ -> failwith "Should not reach here")

(* ========== ネストパターンテスト ========== *)

let test_nested_patterns () =
  Printf.printf "\nNested Pattern Tests:\n";

  (* Some(Some(x)) パターン *)
  run_test "infer_pattern: Some(Some(x))" (fun () ->
      let env = initial_env in
      let pat =
        {
          pat_kind =
            PatConstructor
              ( { name = "Some"; span = dummy_span },
                [
                  {
                    pat_kind =
                      PatConstructor
                        ( { name = "Some"; span = dummy_span },
                          [
                            {
                              pat_kind =
                                PatVar { name = "x"; span = dummy_span };
                              pat_span = dummy_span;
                            };
                          ] );
                    pat_span = dummy_span;
                  };
                ] );
          pat_span = dummy_span;
        }
      in
      let expected_ty = ty_option (ty_option ty_i64) in
      let result = infer_pattern env pat expected_ty in
      assert_ok result "Some(Some(x)) pattern should succeed";
      match result with
      | Ok (tpat, _) ->
          assert_type_eq expected_ty tpat.tpat_ty "Some(Some(x)) pattern type"
      | Error _ -> failwith "Should not reach here")

(* ========== match式テスト ========== *)

let test_match_expressions () =
  Printf.printf "\nMatch Expression Tests:\n";

  (* 単純なmatch式: Option<i64> *)
  run_test "infer_expr: match Some(42) with ..." (fun () ->
      let env = initial_env in

      (* Some(42) *)
      let scrutinee =
        {
          expr_kind =
            Call
              ( {
                  expr_kind = Var { name = "Some"; span = dummy_span };
                  expr_span = dummy_span;
                },
                [
                  PosArg
                    {
                      expr_kind = Literal (Int ("42", Base10));
                      expr_span = dummy_span;
                    };
                ] );
          expr_span = dummy_span;
        }
      in

      (* match arms *)
      let arms =
        [
          (* Some(x) -> x *)
          {
            arm_pattern =
              {
                pat_kind =
                  PatConstructor
                    ( { name = "Some"; span = dummy_span },
                      [
                        {
                          pat_kind = PatVar { name = "x"; span = dummy_span };
                          pat_span = dummy_span;
                        };
                      ] );
                pat_span = dummy_span;
              };
            arm_guard = None;
            arm_body =
              {
                expr_kind = Var { name = "x"; span = dummy_span };
                expr_span = dummy_span;
              };
            arm_span = dummy_span;
          };
          (* None -> 0 *)
          {
            arm_pattern =
              {
                pat_kind =
                  PatConstructor ({ name = "None"; span = dummy_span }, []);
                pat_span = dummy_span;
              };
            arm_guard = None;
            arm_body =
              {
                expr_kind = Literal (Int ("0", Base10));
                expr_span = dummy_span;
              };
            arm_span = dummy_span;
          };
        ]
      in

      let match_expr =
        { expr_kind = Match (scrutinee, arms); expr_span = dummy_span }
      in

      let result = infer_expr env match_expr in
      assert_ok result "Match expression should succeed";
      match result with
      | Ok (_, ty, _, _) ->
          assert_type_eq ty_i64 ty "Match expression result type"
      | Error _ -> failwith "Should not reach here");

  (* ネストしたmatch: Option<Option<i64>> *)
  run_test "infer_expr: nested match Some(Some(x))" (fun () ->
      let env = initial_env in

      (* スクラティニー: 型変数にする *)
      let scrutinee =
        {
          expr_kind = Var { name = "nested_opt"; span = dummy_span };
          expr_span = dummy_span;
        }
      in

      (* nested_opt を環境に追加 *)
      let env =
        extend "nested_opt"
          (scheme_to_constrained (mono_scheme (ty_option (ty_option ty_i64))))
          env
      in

      let arms =
        [
          (* Some(Some(x)) -> x *)
          {
            arm_pattern =
              {
                pat_kind =
                  PatConstructor
                    ( { name = "Some"; span = dummy_span },
                      [
                        {
                          pat_kind =
                            PatConstructor
                              ( { name = "Some"; span = dummy_span },
                                [
                                  {
                                    pat_kind =
                                      PatVar { name = "x"; span = dummy_span };
                                    pat_span = dummy_span;
                                  };
                                ] );
                          pat_span = dummy_span;
                        };
                      ] );
                pat_span = dummy_span;
              };
            arm_guard = None;
            arm_body =
              {
                expr_kind = Var { name = "x"; span = dummy_span };
                expr_span = dummy_span;
              };
            arm_span = dummy_span;
          };
          (* _ -> 0 *)
          {
            arm_pattern = { pat_kind = PatWildcard; pat_span = dummy_span };
            arm_guard = None;
            arm_body =
              {
                expr_kind = Literal (Int ("0", Base10));
                expr_span = dummy_span;
              };
            arm_span = dummy_span;
          };
        ]
      in

      let match_expr =
        { expr_kind = Match (scrutinee, arms); expr_span = dummy_span }
      in

      let result = infer_expr env match_expr in
      assert_ok result "Nested match expression should succeed";
      match result with
      | Ok (_, ty, _, _) ->
          assert_type_eq ty_i64 ty "Nested match expression result type"
      | Error _ -> failwith "Should not reach here")

(* ========== ブロック式テスト ========== *)

let test_block_expressions () =
  Printf.printf "\nBlock Expression Tests:\n";

  (* 空のブロック *)
  run_test "infer_expr: empty block {}" (fun () ->
      let env = initial_env in
      let block_expr = { expr_kind = Block []; expr_span = dummy_span } in
      let result = infer_expr env block_expr in
      assert_ok result "Empty block should succeed";
      match result with
      | Ok (_, ty, _, _) -> assert_type_eq ty_unit ty "Empty block type"
      | Error _ -> failwith "Should not reach here");

  (* 式のみのブロック *)
  run_test "infer_expr: block with single expr { 42 }" (fun () ->
      let env = initial_env in
      let block_expr =
        {
          expr_kind =
            Block
              [
                ExprStmt
                  {
                    expr_kind = Literal (Int ("42", Base10));
                    expr_span = dummy_span;
                  };
              ];
          expr_span = dummy_span;
        }
      in
      let result = infer_expr env block_expr in
      assert_ok result "Block with single expr should succeed";
      match result with
      | Ok (_, ty, _, _) ->
          assert_type_eq ty_i64 ty "Block with single expr type"
      | Error _ -> failwith "Should not reach here");

  (* let束縛を含むブロック（簡易版：二項演算を使わない） *)
  run_test "infer_expr: block with let binding { let x = 1; x }" (fun () ->
      let env = initial_env in
      (* let x = 1 *)
      let let_decl =
        {
          decl_attrs = [];
          decl_vis = Private;
          decl_kind =
            LetDecl
              ( {
                  pat_kind = PatVar { name = "x"; span = dummy_span };
                  pat_span = dummy_span;
                },
                None,
                {
                  expr_kind = Literal (Int ("1", Base10));
                  expr_span = dummy_span;
                } );
          decl_span = dummy_span;
        }
      in
      (* x *)
      let x_expr =
        {
          expr_kind = Var { name = "x"; span = dummy_span };
          expr_span = dummy_span;
        }
      in
      let block_expr =
        {
          expr_kind = Block [ DeclStmt let_decl; ExprStmt x_expr ];
          expr_span = dummy_span;
        }
      in
      let result = infer_expr env block_expr in
      assert_ok result "Block with let binding should succeed";
      match result with
      | Ok (_, ty, _, _) ->
          assert_type_eq ty_i64 ty "Block with let binding type"
      | Error _ -> failwith "Should not reach here");

  (* 複数の文を含むブロック *)
  run_test "infer_expr: block with multiple statements" (fun () ->
      let env = initial_env in
      (* let x = 1 *)
      let let_x =
        {
          decl_attrs = [];
          decl_vis = Private;
          decl_kind =
            LetDecl
              ( {
                  pat_kind = PatVar { name = "x"; span = dummy_span };
                  pat_span = dummy_span;
                },
                None,
                {
                  expr_kind = Literal (Int ("1", Base10));
                  expr_span = dummy_span;
                } );
          decl_span = dummy_span;
        }
      in
      (* let y = 2 *)
      let let_y =
        {
          decl_attrs = [];
          decl_vis = Private;
          decl_kind =
            LetDecl
              ( {
                  pat_kind = PatVar { name = "y"; span = dummy_span };
                  pat_span = dummy_span;
                },
                None,
                {
                  expr_kind = Literal (Int ("2", Base10));
                  expr_span = dummy_span;
                } );
          decl_span = dummy_span;
        }
      in
      (* x (最後の式) *)
      let x_expr =
        {
          expr_kind = Var { name = "x"; span = dummy_span };
          expr_span = dummy_span;
        }
      in
      let block_expr =
        {
          expr_kind = Block [ DeclStmt let_x; DeclStmt let_y; ExprStmt x_expr ];
          expr_span = dummy_span;
        }
      in
      let result = infer_expr env block_expr in
      assert_ok result "Block with multiple statements should succeed";
      match result with
      | Ok (_, ty, _, _) ->
          assert_type_eq ty_i64 ty "Block with multiple statements type"
      | Error _ -> failwith "Should not reach here");

  (* ブロックの最後が宣言文 → Unit型 *)
  run_test "infer_expr: block ending with decl -> Unit" (fun () ->
      let env = initial_env in
      let let_decl =
        {
          decl_attrs = [];
          decl_vis = Private;
          decl_kind =
            LetDecl
              ( {
                  pat_kind = PatVar { name = "x"; span = dummy_span };
                  pat_span = dummy_span;
                },
                None,
                {
                  expr_kind = Literal (Int ("42", Base10));
                  expr_span = dummy_span;
                } );
          decl_span = dummy_span;
        }
      in
      let block_expr =
        { expr_kind = Block [ DeclStmt let_decl ]; expr_span = dummy_span }
      in
      let result = infer_expr env block_expr in
      assert_ok result "Block ending with decl should succeed";
      match result with
      | Ok (_, ty, _, _) ->
          assert_type_eq ty_unit ty "Block ending with decl type"
      | Error _ -> failwith "Should not reach here");

  (* ネストしたブロック *)
  run_test "infer_expr: nested blocks" (fun () ->
      let env = initial_env in
      (* 内側のブロック: { 42 } *)
      let inner_block =
        {
          expr_kind =
            Block
              [
                ExprStmt
                  {
                    expr_kind = Literal (Int ("42", Base10));
                    expr_span = dummy_span;
                  };
              ];
          expr_span = dummy_span;
        }
      in
      (* 外側のブロック: { { 42 } } *)
      let outer_block =
        { expr_kind = Block [ ExprStmt inner_block ]; expr_span = dummy_span }
      in
      let result = infer_expr env outer_block in
      assert_ok result "Nested blocks should succeed";
      match result with
      | Ok (_, ty, _, _) -> assert_type_eq ty_i64 ty "Nested blocks type"
      | Error _ -> failwith "Should not reach here")

(* ========== 関数宣言テスト ========== *)

let test_function_declarations () =
  Printf.printf "\nFunction Declaration Tests:\n";

  (* 単純な関数宣言（式本体）: x + y *)
  run_test "infer_decl: fn add(x: i64, y: i64) -> i64 = x + y" (fun () ->
      let env = initial_env in
      (* x + y を構築 *)
      let add_expr =
        {
          expr_kind =
            Binary
              ( Add,
                {
                  expr_kind = Var { name = "x"; span = dummy_span };
                  expr_span = dummy_span;
                },
                {
                  expr_kind = Var { name = "y"; span = dummy_span };
                  expr_span = dummy_span;
                } );
          expr_span = dummy_span;
        }
      in
      let fn_decl =
        {
          decl_attrs = [];
          decl_vis = Private;
          decl_kind =
            FnDecl
              {
                fn_name = { name = "add"; span = dummy_span };
                fn_generic_params = [];
                fn_params =
                  [
                    {
                      pat =
                        {
                          pat_kind = PatVar { name = "x"; span = dummy_span };
                          pat_span = dummy_span;
                        };
                      ty =
                        Some
                          {
                            ty_kind =
                              TyIdent { name = "i64"; span = dummy_span };
                            ty_span = dummy_span;
                          };
                      default = None;
                      param_span = dummy_span;
                    };
                    {
                      pat =
                        {
                          pat_kind = PatVar { name = "y"; span = dummy_span };
                          pat_span = dummy_span;
                        };
                      ty =
                        Some
                          {
                            ty_kind =
                              TyIdent { name = "i64"; span = dummy_span };
                            ty_span = dummy_span;
                          };
                      default = None;
                      param_span = dummy_span;
                    };
                  ];
                fn_ret_type =
                  Some
                    {
                      ty_kind = TyIdent { name = "i64"; span = dummy_span };
                      ty_span = dummy_span;
                    };
                fn_where_clause = [];
                fn_effect_profile = None;
                fn_body = FnExpr add_expr;
              };
          decl_span = dummy_span;
        }
      in
      let result = infer_decl env fn_decl in
      assert_ok result "Function declaration should succeed";
      match result with
      | Ok (tdecl, _, _) ->
          (* 関数型: i64 -> i64 -> i64 *)
          let expected_ty = ty_arrow ty_i64 (ty_arrow ty_i64 ty_i64) in
          assert_type_eq expected_ty tdecl.tdecl_scheme.body "Function type"
      | Error _ -> failwith "Should not reach here");

  (* 型注釈なしのパラメータ *)
  run_test "infer_decl: fn inferred_add(x, y) = x + y" (fun () ->
      let env = initial_env in
      let add_expr =
        {
          expr_kind =
            Binary
              ( Add,
                {
                  expr_kind = Var { name = "x"; span = dummy_span };
                  expr_span = dummy_span;
                },
                {
                  expr_kind = Var { name = "y"; span = dummy_span };
                  expr_span = dummy_span;
                } );
          expr_span = dummy_span;
        }
      in
      let fn_decl =
        {
          decl_attrs = [];
          decl_vis = Private;
          decl_kind =
            FnDecl
              {
                fn_name = { name = "inferred_add"; span = dummy_span };
                fn_generic_params = [];
                fn_params =
                  [
                    {
                      pat =
                        {
                          pat_kind = PatVar { name = "x"; span = dummy_span };
                          pat_span = dummy_span;
                        };
                      ty = None;
                      default = None;
                      param_span = dummy_span;
                    };
                    {
                      pat =
                        {
                          pat_kind = PatVar { name = "y"; span = dummy_span };
                          pat_span = dummy_span;
                        };
                      ty = None;
                      default = None;
                      param_span = dummy_span;
                    };
                  ];
                fn_ret_type = None;
                fn_where_clause = [];
                fn_effect_profile = None;
                fn_body = FnExpr add_expr;
              };
          decl_span = dummy_span;
        }
      in
      let result = infer_decl env fn_decl in
      assert_ok result "Inferred add function should succeed";
      match result with
      | Ok (tdecl, _, _) -> (
          match tdecl.tdecl_kind with
          | Typed_ast.TFnDecl tfn ->
              List.iteri
                (fun idx param ->
                  let msg =
                    Printf.sprintf "Parameter %d type should default to i64"
                      (idx + 1)
                  in
                  assert_type_eq ty_i64 param.tty msg;
                  assert_type_eq ty_i64 param.tpat.tpat_ty msg)
                tfn.tfn_params;
              let expected_ty = ty_arrow ty_i64 (ty_arrow ty_i64 ty_i64) in
              assert_type_eq expected_ty tdecl.tdecl_scheme.body
                "Inferred add function type"
          | _ -> failwith "Expected a function declaration")
      | Error _ -> failwith "Should not reach here");

  (* 型注釈なしのパラメータ *)
  run_test "infer_decl: fn identity(x) = x" (fun () ->
      let env = initial_env in
      let fn_decl =
        {
          decl_attrs = [];
          decl_vis = Private;
          decl_kind =
            FnDecl
              {
                fn_name = { name = "identity"; span = dummy_span };
                fn_generic_params = [];
                fn_params =
                  [
                    {
                      pat =
                        {
                          pat_kind = PatVar { name = "x"; span = dummy_span };
                          pat_span = dummy_span;
                        };
                      ty = None;
                      default = None;
                      param_span = dummy_span;
                    };
                  ];
                fn_ret_type = None;
                fn_where_clause = [];
                fn_effect_profile = None;
                fn_body =
                  FnExpr
                    {
                      expr_kind = Var { name = "x"; span = dummy_span };
                      expr_span = dummy_span;
                    };
              };
          decl_span = dummy_span;
        }
      in
      let result = infer_decl env fn_decl in
      assert_ok result "Identity function should succeed";
      match result with
      | Ok (tdecl, _, _) ->
          (* 型変数が一般化される: ∀t. t -> t *)
          (* 量化変数が1つあることを確認 *)
          if List.length tdecl.tdecl_scheme.quantified < 1 then
            failwith
              "Identity function should have at least one quantified variable"
      | Error _ -> failwith "Should not reach here");

  (* ブロック本体の関数 *)
  run_test "infer_decl: fn const_forty_two() -> i64 { 42 }" (fun () ->
      let env = initial_env in
      let fn_decl =
        {
          decl_attrs = [];
          decl_vis = Private;
          decl_kind =
            FnDecl
              {
                fn_name = { name = "const_forty_two"; span = dummy_span };
                fn_generic_params = [];
                fn_params = [];
                fn_ret_type =
                  Some
                    {
                      ty_kind = TyIdent { name = "i64"; span = dummy_span };
                      ty_span = dummy_span;
                    };
                fn_where_clause = [];
                fn_effect_profile = None;
                fn_body =
                  FnBlock
                    [
                      ExprStmt
                        {
                          expr_kind = Literal (Int ("42", Base10));
                          expr_span = dummy_span;
                        };
                    ];
              };
          decl_span = dummy_span;
        }
      in
      let result = infer_decl env fn_decl in
      assert_ok result "Const function should succeed";
      match result with
      | Ok (tdecl, _, _) ->
          (* 関数型: () -> i64 (パラメータなし) *)
          assert_type_eq ty_i64 tdecl.tdecl_scheme.body "Const function type"
      | Error _ -> failwith "Should not reach here");

  (* 再帰関数: if n <= 1 then 1 else n * fact(n - 1) *)
  run_test
    "infer_decl: fn fact(n: i64) -> i64 = if n <= 1 then 1 else n * fact(n - 1)"
    (fun () ->
      let env = initial_env in
      (* n <= 1 *)
      let cond_expr =
        {
          expr_kind =
            Binary
              ( Le,
                {
                  expr_kind = Var { name = "n"; span = dummy_span };
                  expr_span = dummy_span;
                },
                {
                  expr_kind = Literal (Int ("1", Base10));
                  expr_span = dummy_span;
                } );
          expr_span = dummy_span;
        }
      in
      (* n - 1 *)
      let n_minus_1 =
        {
          expr_kind =
            Binary
              ( Sub,
                {
                  expr_kind = Var { name = "n"; span = dummy_span };
                  expr_span = dummy_span;
                },
                {
                  expr_kind = Literal (Int ("1", Base10));
                  expr_span = dummy_span;
                } );
          expr_span = dummy_span;
        }
      in
      (* fact(n - 1) *)
      let fact_call =
        {
          expr_kind =
            Call
              ( {
                  expr_kind = Var { name = "fact"; span = dummy_span };
                  expr_span = dummy_span;
                },
                [ PosArg n_minus_1 ] );
          expr_span = dummy_span;
        }
      in
      (* n * fact(n - 1) *)
      let else_expr =
        {
          expr_kind =
            Binary
              ( Mul,
                {
                  expr_kind = Var { name = "n"; span = dummy_span };
                  expr_span = dummy_span;
                },
                fact_call );
          expr_span = dummy_span;
        }
      in
      (* if n <= 1 then 1 else n * fact(n - 1) *)
      let fact_body =
        {
          expr_kind =
            If
              ( cond_expr,
                {
                  expr_kind = Literal (Int ("1", Base10));
                  expr_span = dummy_span;
                },
                Some else_expr );
          expr_span = dummy_span;
        }
      in
      let fn_decl =
        {
          decl_attrs = [];
          decl_vis = Private;
          decl_kind =
            FnDecl
              {
                fn_name = { name = "fact"; span = dummy_span };
                fn_generic_params = [];
                fn_params =
                  [
                    {
                      pat =
                        {
                          pat_kind = PatVar { name = "n"; span = dummy_span };
                          pat_span = dummy_span;
                        };
                      ty =
                        Some
                          {
                            ty_kind =
                              TyIdent { name = "i64"; span = dummy_span };
                            ty_span = dummy_span;
                          };
                      default = None;
                      param_span = dummy_span;
                    };
                  ];
                fn_ret_type =
                  Some
                    {
                      ty_kind = TyIdent { name = "i64"; span = dummy_span };
                      ty_span = dummy_span;
                    };
                fn_where_clause = [];
                fn_effect_profile = None;
                fn_body = FnExpr fact_body;
              };
          decl_span = dummy_span;
        }
      in
      let result = infer_decl env fn_decl in
      assert_ok result "Recursive factorial function should succeed";
      match result with
      | Ok (tdecl, _, _) ->
          (* 関数型: i64 -> i64 *)
          let expected_ty = ty_arrow ty_i64 ty_i64 in
          assert_type_eq expected_ty tdecl.tdecl_scheme.body
            "Factorial function type"
      | Error _ -> failwith "Should not reach here");

  (* 複数文を含むブロック本体 *)
  run_test "infer_decl: fn multi_stmt() -> i64 { let x = 1; x }" (fun () ->
      let env = initial_env in
      let fn_decl =
        {
          decl_attrs = [];
          decl_vis = Private;
          decl_kind =
            FnDecl
              {
                fn_name = { name = "multi_stmt"; span = dummy_span };
                fn_generic_params = [];
                fn_params = [];
                fn_ret_type =
                  Some
                    {
                      ty_kind = TyIdent { name = "i64"; span = dummy_span };
                      ty_span = dummy_span;
                    };
                fn_where_clause = [];
                fn_effect_profile = None;
                fn_body =
                  FnBlock
                    [
                      DeclStmt
                        {
                          decl_attrs = [];
                          decl_vis = Private;
                          decl_kind =
                            LetDecl
                              ( {
                                  pat_kind =
                                    PatVar { name = "x"; span = dummy_span };
                                  pat_span = dummy_span;
                                },
                                None,
                                {
                                  expr_kind = Literal (Int ("1", Base10));
                                  expr_span = dummy_span;
                                } );
                          decl_span = dummy_span;
                        };
                      ExprStmt
                        {
                          expr_kind = Var { name = "x"; span = dummy_span };
                          expr_span = dummy_span;
                        };
                    ];
              };
          decl_span = dummy_span;
        }
      in
      let result = infer_decl env fn_decl in
      assert_ok result "Multi-statement function should succeed";
      match result with
      | Ok (tdecl, _, _) ->
          assert_type_eq ty_i64 tdecl.tdecl_scheme.body
            "Multi-statement function type"
      | Error _ -> failwith "Should not reach here")

(* ========== 二項演算テスト ========== *)

let test_binary_operations () =
  Printf.printf "\nBinary Operation Tests:\n";

  (* 算術演算: 1 + 2 *)
  run_test "infer_expr: 1 + 2" (fun () ->
      let env = initial_env in
      let expr =
        {
          expr_kind =
            Binary
              ( Add,
                {
                  expr_kind = Literal (Int ("1", Base10));
                  expr_span = dummy_span;
                },
                {
                  expr_kind = Literal (Int ("2", Base10));
                  expr_span = dummy_span;
                } );
          expr_span = dummy_span;
        }
      in
      let result = infer_expr env expr in
      assert_ok result "Arithmetic addition should succeed";
      match result with
      | Ok (_, ty, _, _) -> assert_type_eq ty_i64 ty "Addition result type"
      | Error _ -> failwith "Should not reach here");

  (* 算術演算: 3.0 * 2.0 *)
  run_test "infer_expr: 3.0 * 2.0" (fun () ->
      let env = initial_env in
      let expr =
        {
          expr_kind =
            Binary
              ( Mul,
                { expr_kind = Literal (Float "3.0"); expr_span = dummy_span },
                { expr_kind = Literal (Float "2.0"); expr_span = dummy_span } );
          expr_span = dummy_span;
        }
      in
      let result = infer_expr env expr in
      assert_ok result "Arithmetic multiplication should succeed";
      match result with
      | Ok (_, ty, _, _) ->
          assert_type_eq ty_f64 ty "Multiplication result type"
      | Error _ -> failwith "Should not reach here");

  (* 比較演算: 5 == 5 *)
  run_test "infer_expr: 5 == 5" (fun () ->
      let env = initial_env in
      let expr =
        {
          expr_kind =
            Binary
              ( Eq,
                {
                  expr_kind = Literal (Int ("5", Base10));
                  expr_span = dummy_span;
                },
                {
                  expr_kind = Literal (Int ("5", Base10));
                  expr_span = dummy_span;
                } );
          expr_span = dummy_span;
        }
      in
      let result = infer_expr env expr in
      assert_ok result "Equality comparison should succeed";
      match result with
      | Ok (_, ty, _, _) -> assert_type_eq ty_bool ty "Equality result type"
      | Error _ -> failwith "Should not reach here");

  (* 論理演算: true && false *)
  run_test "infer_expr: true && false" (fun () ->
      let env = initial_env in
      let expr =
        {
          expr_kind =
            Binary
              ( And,
                { expr_kind = Literal (Bool true); expr_span = dummy_span },
                { expr_kind = Literal (Bool false); expr_span = dummy_span } );
          expr_span = dummy_span;
        }
      in
      let result = infer_expr env expr in
      assert_ok result "Logical AND should succeed";
      match result with
      | Ok (_, ty, _, _) -> assert_type_eq ty_bool ty "Logical AND result type"
      | Error _ -> failwith "Should not reach here");

  (* 混合演算: (1 + 2) * 3 *)
  run_test "infer_expr: (1 + 2) * 3" (fun () ->
      let env = initial_env in
      let inner_expr =
        {
          expr_kind =
            Binary
              ( Add,
                {
                  expr_kind = Literal (Int ("1", Base10));
                  expr_span = dummy_span;
                },
                {
                  expr_kind = Literal (Int ("2", Base10));
                  expr_span = dummy_span;
                } );
          expr_span = dummy_span;
        }
      in
      let outer_expr =
        {
          expr_kind =
            Binary
              ( Mul,
                inner_expr,
                {
                  expr_kind = Literal (Int ("3", Base10));
                  expr_span = dummy_span;
                } );
          expr_span = dummy_span;
        }
      in
      let result = infer_expr env outer_expr in
      assert_ok result "Nested arithmetic should succeed";
      match result with
      | Ok (_, ty, _, _) ->
          assert_type_eq ty_i64 ty "Nested arithmetic result type"
      | Error _ -> failwith "Should not reach here");

  (* パイプ演算: 42 |> identity *)
  run_test "infer_expr: 42 |> identity" (fun () ->
      let env = initial_env in
      (* identity関数を環境に追加: ∀a. a -> a *)
      let tv_a = TypeVarGen.fresh (Some "a") in
      let identity_ty = ty_arrow (Types.TVar tv_a) (Types.TVar tv_a) in
      let env_with_identity =
        extend "identity"
          (scheme_to_constrained { quantified = [ tv_a ]; body = identity_ty })
          env
      in

      let expr =
        {
          expr_kind =
            Binary
              ( PipeOp,
                {
                  expr_kind = Literal (Int ("42", Base10));
                  expr_span = dummy_span;
                },
                {
                  expr_kind = Var { name = "identity"; span = dummy_span };
                  expr_span = dummy_span;
                } );
          expr_span = dummy_span;
        }
      in
      let result = infer_expr env_with_identity expr in
      assert_ok result "Pipe operation should succeed";
      match result with
      | Ok (_, ty, _, _) -> assert_type_eq ty_i64 ty "Pipe result type"
      | Error _ -> failwith "Should not reach here")

(* ========== 複合リテラルテスト ========== *)

let test_composite_literals () =
  Printf.printf "\nComposite Literal Tests:\n";

  (* タプルリテラル: (1, "hello", true) *)
  run_test "infer_expr: tuple literal (1, \"hello\", true)" (fun () ->
      let env = initial_env in
      let expr =
        {
          expr_kind =
            Literal
              (Tuple
                 [
                   {
                     expr_kind = Literal (Int ("1", Base10));
                     expr_span = dummy_span;
                   };
                   {
                     expr_kind = Literal (String ("hello", Normal));
                     expr_span = dummy_span;
                   };
                   { expr_kind = Literal (Bool true); expr_span = dummy_span };
                 ]);
          expr_span = dummy_span;
        }
      in
      let result = infer_expr env expr in
      assert_ok result "Tuple literal should succeed";
      match result with
      | Ok (_, ty, _, _) ->
          let expected_ty = TTuple [ ty_i64; ty_string; ty_bool ] in
          assert_type_eq expected_ty ty "Tuple literal type"
      | Error _ -> failwith "Should not reach here");

  (* 空タプル: () *)
  run_test "infer_expr: empty tuple ()" (fun () ->
      let env = initial_env in
      let expr = { expr_kind = Literal (Tuple []); expr_span = dummy_span } in
      let result = infer_expr env expr in
      assert_ok result "Empty tuple should succeed";
      match result with
      | Ok (_, ty, _, _) -> assert_type_eq ty_unit ty "Empty tuple type"
      | Error _ -> failwith "Should not reach here");

  (* ネストしたタプル: ((1, 2), (3, 4)) *)
  run_test "infer_expr: nested tuple ((1, 2), (3, 4))" (fun () ->
      let env = initial_env in
      let expr =
        {
          expr_kind =
            Literal
              (Tuple
                 [
                   {
                     expr_kind =
                       Literal
                         (Tuple
                            [
                              {
                                expr_kind = Literal (Int ("1", Base10));
                                expr_span = dummy_span;
                              };
                              {
                                expr_kind = Literal (Int ("2", Base10));
                                expr_span = dummy_span;
                              };
                            ]);
                     expr_span = dummy_span;
                   };
                   {
                     expr_kind =
                       Literal
                         (Tuple
                            [
                              {
                                expr_kind = Literal (Int ("3", Base10));
                                expr_span = dummy_span;
                              };
                              {
                                expr_kind = Literal (Int ("4", Base10));
                                expr_span = dummy_span;
                              };
                            ]);
                     expr_span = dummy_span;
                   };
                 ]);
          expr_span = dummy_span;
        }
      in
      let result = infer_expr env expr in
      assert_ok result "Nested tuple should succeed";
      match result with
      | Ok (_, ty, _, _) ->
          let expected_ty =
            TTuple [ TTuple [ ty_i64; ty_i64 ]; TTuple [ ty_i64; ty_i64 ] ]
          in
          assert_type_eq expected_ty ty "Nested tuple type"
      | Error _ -> failwith "Should not reach here");

  (* レコードリテラル: { x: 42, y: "test" } *)
  run_test "infer_expr: record literal { x: 42, y: \"test\" }" (fun () ->
      let env = initial_env in
      let expr =
        {
          expr_kind =
            Literal
              (Record
                 [
                   ( { name = "x"; span = dummy_span },
                     {
                       expr_kind = Literal (Int ("42", Base10));
                       expr_span = dummy_span;
                     } );
                   ( { name = "y"; span = dummy_span },
                     {
                       expr_kind = Literal (String ("test", Normal));
                       expr_span = dummy_span;
                     } );
                 ]);
          expr_span = dummy_span;
        }
      in
      let result = infer_expr env expr in
      assert_ok result "Record literal should succeed";
      match result with
      | Ok (_, ty, _, _) ->
          let expected_ty = TRecord [ ("x", ty_i64); ("y", ty_string) ] in
          assert_type_eq expected_ty ty "Record literal type"
      | Error _ -> failwith "Should not reach here");

  (* レコードリテラル（フィールド順序確認）: { name: "Alice", age: 30 } *)
  run_test "infer_expr: record literal { name: \"Alice\", age: 30 }" (fun () ->
      let env = initial_env in
      let expr =
        {
          expr_kind =
            Literal
              (Record
                 [
                   ( { name = "name"; span = dummy_span },
                     {
                       expr_kind = Literal (String ("Alice", Normal));
                       expr_span = dummy_span;
                     } );
                   ( { name = "age"; span = dummy_span },
                     {
                       expr_kind = Literal (Int ("30", Base10));
                       expr_span = dummy_span;
                     } );
                 ]);
          expr_span = dummy_span;
        }
      in
      let result = infer_expr env expr in
      assert_ok result "Record literal with different fields should succeed";
      match result with
      | Ok (_, ty, _, _) ->
          let expected_ty = TRecord [ ("name", ty_string); ("age", ty_i64) ] in
          assert_type_eq expected_ty ty
            "Record literal type with different fields"
      | Error _ -> failwith "Should not reach here");

  (* ネストしたレコード: { outer: { inner: 42 } } *)
  run_test "infer_expr: nested record { outer: { inner: 42 } }" (fun () ->
      let env = initial_env in
      let expr =
        {
          expr_kind =
            Literal
              (Record
                 [
                   ( { name = "outer"; span = dummy_span },
                     {
                       expr_kind =
                         Literal
                           (Record
                              [
                                ( { name = "inner"; span = dummy_span },
                                  {
                                    expr_kind = Literal (Int ("42", Base10));
                                    expr_span = dummy_span;
                                  } );
                              ]);
                       expr_span = dummy_span;
                     } );
                 ]);
          expr_span = dummy_span;
        }
      in
      let result = infer_expr env expr in
      assert_ok result "Nested record should succeed";
      match result with
      | Ok (_, ty, _, _) ->
          let expected_ty =
            TRecord [ ("outer", TRecord [ ("inner", ty_i64) ]) ]
          in
          assert_type_eq expected_ty ty "Nested record type"
      | Error _ -> failwith "Should not reach here");

  (* タプルとレコードの混在: (1, { x: 2 }) *)
  run_test "infer_expr: tuple with record (1, { x: 2 })" (fun () ->
      let env = initial_env in
      let expr =
        {
          expr_kind =
            Literal
              (Tuple
                 [
                   {
                     expr_kind = Literal (Int ("1", Base10));
                     expr_span = dummy_span;
                   };
                   {
                     expr_kind =
                       Literal
                         (Record
                            [
                              ( { name = "x"; span = dummy_span },
                                {
                                  expr_kind = Literal (Int ("2", Base10));
                                  expr_span = dummy_span;
                                } );
                            ]);
                     expr_span = dummy_span;
                   };
                 ]);
          expr_span = dummy_span;
        }
      in
      let result = infer_expr env expr in
      assert_ok result "Tuple with record should succeed";
      match result with
      | Ok (_, ty, _, _) ->
          let expected_ty = TTuple [ ty_i64; TRecord [ ("x", ty_i64) ] ] in
          assert_type_eq expected_ty ty "Tuple with record type"
      | Error _ -> failwith "Should not reach here")

(* ========== 複合リテラル エラーケーステスト ========== *)

let test_composite_literal_errors () =
  Printf.printf "\nComposite Literal Error Tests:\n";

  (* 配列リテラルは未実装 *)
  run_test
    "infer_expr: array literal [1, 2, 3] should fail (not yet implemented)"
    (fun () ->
      let env = initial_env in
      let expr =
        {
          expr_kind =
            Literal
              (Array
                 [
                   {
                     expr_kind = Literal (Int ("1", Base10));
                     expr_span = dummy_span;
                   };
                   {
                     expr_kind = Literal (Int ("2", Base10));
                     expr_span = dummy_span;
                   };
                   {
                     expr_kind = Literal (Int ("3", Base10));
                     expr_span = dummy_span;
                   };
                 ]);
          expr_span = dummy_span;
        }
      in
      let result = infer_expr env expr in
      match result with
      | Ok _ -> failwith "Array literal should fail (not yet implemented)"
      | Error e ->
          (* エラーメッセージを確認 *)
          let error_msg = Type_error.string_of_error e in
          if not (String.length error_msg > 0) then
            failwith "Error message should not be empty")

(* ========== パターンマッチエラーテスト ========== *)

let test_pattern_match_errors () =
  Printf.printf "\nPattern Match Error Tests:\n";

  (* E7009: ConstructorArityMismatch - Some() *)
  run_test "error: ConstructorArityMismatch Some()" (fun () ->
      let env = initial_env in
      let pattern =
        {
          pat_kind =
            PatConstructor
              ({ name = "Some"; span = dummy_span }, [] (* 引数なし - エラー *));
          pat_span = dummy_span;
        }
      in
      let expected_ty = ty_option ty_i64 in
      let result = infer_pattern env pattern expected_ty in
      match result with
      | Error (ConstructorArityMismatch { constructor; expected; actual; _ }) ->
          if constructor = "Some" && expected = 1 && actual = 0 then ()
          else failwith "Constructor arity error details mismatch"
      | Error e ->
          let msg = Type_error.string_of_error e in
          failwith
            (Printf.sprintf "Expected ConstructorArityMismatch, got: %s" msg)
      | Ok _ -> failwith "Should fail with ConstructorArityMismatch");

  (* E7009: ConstructorArityMismatch - None(x) *)
  run_test "error: ConstructorArityMismatch None(x)" (fun () ->
      let env = initial_env in
      let pattern =
        {
          pat_kind =
            PatConstructor
              ( { name = "None"; span = dummy_span },
                [
                  {
                    pat_kind = PatVar { name = "x"; span = dummy_span };
                    pat_span = dummy_span;
                  };
                ] );
          pat_span = dummy_span;
        }
      in
      let expected_ty = ty_option ty_i64 in
      let result = infer_pattern env pattern expected_ty in
      match result with
      | Error (ConstructorArityMismatch { constructor; expected; actual; _ }) ->
          if constructor = "None" && expected = 0 && actual = 1 then ()
          else failwith "Constructor arity error details mismatch"
      | Error e ->
          let msg = Type_error.string_of_error e in
          failwith
            (Printf.sprintf "Expected ConstructorArityMismatch, got: %s" msg)
      | Ok _ -> failwith "Should fail with ConstructorArityMismatch");

  (* E7010: TupleArityMismatch - (x, y) vs (1, 2, 3) *)
  run_test "error: TupleArityMismatch (x, y) vs 3-tuple" (fun () ->
      let env = initial_env in
      let pattern =
        {
          pat_kind =
            PatTuple
              [
                {
                  pat_kind = PatVar { name = "x"; span = dummy_span };
                  pat_span = dummy_span;
                };
                {
                  pat_kind = PatVar { name = "y"; span = dummy_span };
                  pat_span = dummy_span;
                };
              ];
          pat_span = dummy_span;
        }
      in
      let expected_ty = TTuple [ ty_i64; ty_i64; ty_i64 ] in
      let result = infer_pattern env pattern expected_ty in
      match result with
      | Error (TupleArityMismatch { expected; actual; _ }) ->
          if expected = 3 && actual = 2 then ()
          else failwith "Tuple arity error details mismatch"
      | Error e ->
          let msg = Type_error.string_of_error e in
          failwith (Printf.sprintf "Expected TupleArityMismatch, got: %s" msg)
      | Ok _ -> failwith "Should fail with TupleArityMismatch");

  (* E7013: NotARecord - レコードパターンを非レコード型に適用 *)
  run_test "error: NotARecord { x } vs i64" (fun () ->
      let env = initial_env in
      let pattern =
        {
          pat_kind =
            PatRecord
              ( [
                  ( { name = "x"; span = dummy_span },
                    Some
                      {
                        pat_kind = PatVar { name = "x"; span = dummy_span };
                        pat_span = dummy_span;
                      } );
                ],
                false );
          pat_span = dummy_span;
        }
      in
      let expected_ty = ty_i64 in
      let result = infer_pattern env pattern expected_ty in
      match result with
      | Error (NotARecord (ty, _)) ->
          if Types.type_equal ty ty_i64 then ()
          else failwith "NotARecord error type mismatch"
      | Error e ->
          let msg = Type_error.string_of_error e in
          failwith (Printf.sprintf "Expected NotARecord, got: %s" msg)
      | Ok _ -> failwith "Should fail with NotARecord");

  (* E7015: EmptyMatch - 空のmatch式 *)
  run_test "error: EmptyMatch" (fun () ->
      (* 変数 x を環境に追加 *)
      let env =
        extend "x" (scheme_to_constrained (mono_scheme ty_i64)) initial_env
      in
      let match_expr =
        {
          expr_kind =
            Match
              ( {
                  expr_kind = Var { name = "x"; span = dummy_span };
                  expr_span = dummy_span;
                },
                [] (* アームなし *) );
          expr_span = dummy_span;
        }
      in
      let result = infer_expr env match_expr in
      match result with
      | Error (EmptyMatch _) -> ()
      | Error e ->
          let msg = Type_error.string_of_error e in
          failwith (Printf.sprintf "Expected EmptyMatch, got: %s" msg)
      | Ok _ -> failwith "Should fail with EmptyMatch");

  (* ネストしたパターンエラー: Some(None(x)) *)
  run_test "error: nested pattern Some(None(x))" (fun () ->
      let env = initial_env in
      let pattern =
        {
          pat_kind =
            PatConstructor
              ( { name = "Some"; span = dummy_span },
                [
                  {
                    pat_kind =
                      PatConstructor
                        ( { name = "None"; span = dummy_span },
                          [
                            {
                              pat_kind =
                                PatVar { name = "x"; span = dummy_span };
                              pat_span = dummy_span;
                            };
                          ] );
                    pat_span = dummy_span;
                  };
                ] );
          pat_span = dummy_span;
        }
      in
      let expected_ty = ty_option (ty_option ty_i64) in
      let result = infer_pattern env pattern expected_ty in
      match result with
      | Error (ConstructorArityMismatch { constructor; expected; actual; _ }) ->
          (* None は引数を取らないのでエラー *)
          if constructor = "None" && expected = 0 && actual = 1 then ()
          else failwith "Nested pattern error details mismatch"
      | Error e ->
          let msg = Type_error.string_of_error e in
          failwith
            (Printf.sprintf
               "Expected ConstructorArityMismatch for None, got: %s" msg)
      | Ok _ -> failwith "Should fail with ConstructorArityMismatch")

let test_mutable_bindings () =
  Printf.printf "\nMutable Binding Tests:\n";

  let make_literal value =
    { expr_kind = Literal (Int (value, Base10)); expr_span = dummy_span }
  in

  let make_pattern_x () =
    {
      pat_kind = PatVar { name = "x"; span = dummy_span };
      pat_span = dummy_span;
    }
  in

  run_test "var decl registers mutable binding" (fun () ->
      let env = initial_env in
      let decl =
        {
          decl_attrs = [];
          decl_vis = Private;
          decl_kind = VarDecl (make_pattern_x (), None, make_literal "0");
          decl_span = dummy_span;
        }
      in
      match infer_decl env decl with
      | Ok (_tdecl, new_env, _) -> (
          match lookup_mutability "x" new_env with
          | Some Mutable -> ()
          | _ -> failwith "'x' should be mutable after var declaration")
      | Error e ->
          let msg = Type_error.string_of_error e in
          failwith (Printf.sprintf "var declaration should succeed: %s" msg));

  run_test "assignment to mutable var succeeds" (fun () ->
      let env = initial_env in
      let decl =
        {
          decl_attrs = [];
          decl_vis = Private;
          decl_kind = VarDecl (make_pattern_x (), None, make_literal "0");
          decl_span = dummy_span;
        }
      in
      match infer_decl env decl with
      | Ok (_tdecl, new_env, _) -> (
          let assign_expr =
            {
              expr_kind =
                Assign
                  ( {
                      expr_kind = Var { name = "x"; span = dummy_span };
                      expr_span = dummy_span;
                    },
                    make_literal "1" );
              expr_span = dummy_span;
            }
          in
          let result = infer_expr new_env assign_expr in
          assert_ok result "Assignment to mutable var should succeed";
          match result with
          | Ok (_texpr, ty, _, _) ->
              assert_type_eq ty_unit ty "Assignment should return unit"
          | Error _ -> ())
      | Error e ->
          let msg = Type_error.string_of_error e in
          failwith (Printf.sprintf "var declaration should succeed: %s" msg));

  run_test "assignment to let binding fails" (fun () ->
      let env = initial_env in
      let let_decl =
        {
          decl_attrs = [];
          decl_vis = Private;
          decl_kind = LetDecl (make_pattern_x (), None, make_literal "0");
          decl_span = dummy_span;
        }
      in
      match infer_decl env let_decl with
      | Ok (_tdecl, new_env, _) -> (
          let assign_expr =
            {
              expr_kind =
                Assign
                  ( {
                      expr_kind = Var { name = "x"; span = dummy_span };
                      expr_span = dummy_span;
                    },
                    make_literal "1" );
              expr_span = dummy_span;
            }
          in
          match infer_expr new_env assign_expr with
          | Error (ImmutableBinding { name; _ }) ->
              if String.equal name "x" then ()
              else failwith "Immutable binding error should reference 'x'"
          | Error e ->
              let msg = Type_error.string_of_error e in
              failwith
                (Printf.sprintf "Expected immutable binding error, got %s" msg)
          | Ok _ -> failwith "Assignment to let binding should fail")
      | Error e ->
          let msg = Type_error.string_of_error e in
          failwith (Printf.sprintf "let declaration should succeed: %s" msg));

  run_test "assignment to literal fails" (fun () ->
      let env = initial_env in
      let assign_expr =
        {
          expr_kind = Assign (make_literal "0", make_literal "1");
          expr_span = dummy_span;
        }
      in
      match infer_expr env assign_expr with
      | Error (NotAssignable _) -> ()
      | Error e ->
          let msg = Type_error.string_of_error e in
          failwith (Printf.sprintf "Expected not-assignable error, got %s" msg)
      | Ok _ -> failwith "Assignment to literal should fail")

let test_trait_constraint_stage_metadata () =
  run_test "trait constraint stage metadata" (fun () ->
      let effect_stage : Type_error.trait_constraint_stage_extension =
        {
          required_stage = "beta";
          iterator_required = "at_least:beta";
          actual_stage = Some "experimental";
          capability = Some "core.iterator.collect";
          provider = Some "Core.Iter";
          manifest_path = Some "dsl/core.iter.toml";
          iterator_kind = Some "array_like";
          iterator_source = Some "typer";
          capability_metadata =
            Some (`Assoc [ ("verified_at", `String "2025-10-21T03:15:00Z") ]);
          residual =
            Some
              (`Assoc
                 [
                   ( "missing_ops",
                     `List [ `String "Iterator::next"; `String "Iterator::size_hint" ] );
                 ]);
          stage_trace = [];
        }
      in
      let error =
        Type_error.TraitConstraintFailure
          {
            trait_name = "Iterator";
            type_args =
              [
                Types.TCon (Types.TCUser "SampleStream");
                Types.TCon (Types.TCUser "SampleItem");
              ];
            reason = "Stage mismatch";
            span = dummy_span;
            effect_stage = Some effect_stage;
            typeclass_state = Typeclass_metadata.StageMismatch;
            typeclass_pending = [ "Iterator.size_hint" ];
            typeclass_generalized = [ "T" ];
          }
      in
      let diag = Type_error.to_diagnostic error in
      let expect_string key source =
        match source key with
        | Some (`String value) -> value
        | Some other ->
            failwith
              (Printf.sprintf "%s expected string, got %s" key
                 (Yojson.Basic.to_string other))
        | None -> failwith (Printf.sprintf "%s metadata missing" key)
      in
      let expect_string_list key source =
        match source key with
        | Some (`List items) ->
            List.map
              (function
                | `String value -> value
                | other ->
                    failwith
                      (Printf.sprintf "%s expected string list, got %s" key
                         (Yojson.Basic.to_string other)))
              items
        | Some other ->
            failwith
              (Printf.sprintf "%s expected list, got %s" key
                 (Yojson.Basic.to_string other))
        | None -> failwith (Printf.sprintf "%s metadata missing" key)
      in
      let audit_lookup key =
        Diagnostic.Extensions.get key diag.Diagnostic.audit_metadata
      in
      let event_domain = expect_string "event.domain" audit_lookup in
      let event_kind = expect_string "event.kind" audit_lookup in
      assert (String.equal event_domain "type");
      assert (String.equal event_kind "E7016");
      let capability_ids = expect_string_list "capability.ids" audit_lookup in
      assert (capability_ids = [ "core.iterator.collect" ]);
      let envelope_metadata = Audit_envelope.metadata diag.Diagnostic.audit in
      let envelope_lookup key =
        match List.assoc_opt key envelope_metadata with
        | Some value -> Some value
        | None -> None
      in
      let envelope_ids =
        expect_string_list "capability.ids" envelope_lookup
      in
      assert (envelope_ids = capability_ids);
      let capability_extension =
        match
          Diagnostic.Extensions.get "capability" diag.Diagnostic.extensions
        with
        | Some (`Assoc fields) -> fields
        | Some other ->
            failwith
              (Printf.sprintf "capability extension malformed: %s"
                 (Yojson.Basic.to_string other))
        | None -> failwith "capability extension missing"
      in
      let extension_ids =
        match List.assoc_opt "ids" capability_extension with
        | Some (`List items) ->
            List.map
              (function
                | `String value -> value
                | other ->
                    failwith
                      (Printf.sprintf "capability.ids extension invalid: %s"
                         (Yojson.Basic.to_string other)))
              items
        | _ -> failwith "capability.ids extension missing"
      in
      assert (extension_ids = capability_ids))

(* TODO(TYPE-001/Step3): 値制限テストの実装
   strict / legacy 両モードで以下を検証する予定:
   - let + 純粋ラムダは Value_form.is_immediate + value_restriction_mode=Strict のとき量化変数を保持する
   - var + 純粋ラムダは常に単相化され、Value_restriction.evaluate から Monomorphic が返る
   - let + unsafe / mut / ffi タグを含む式は Strict で単相固定、Legacy では従来挙動をゴールデン比較する
   `value_restriction_mode` の切替と `Value_form` ヘルパ参照のサンプルは
   docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-001-proposal.md Step3 を参照。 *)

(* ========== 効果行統合テスト (TYPE-002 Sprint B) ========== *)

let effect_row_fixture_source =
  {|
effect Console : io {
  operation log : String -> Unit
}

@allows_effects(Console)
fn handled_demo(msg: String) = {
  handle perform Console.log(msg) with handler ConsoleHandler {
    operation log(value) {
      ()
    }
  }
}
|}

let filter_effect_decls ast =
  {
    ast with
    Ast.decls =
      List.filter
        (fun decl ->
          match decl.Ast.decl_kind with Ast.EffectDecl _ -> false | _ -> true)
        ast.Ast.decls;
  }

let parse_and_infer_dual_write source =
  Diagnostic.reset_audit_sequence ();
  let base_config =
    Parser_run_config.Legacy.bridge { require_eof = true; legacy_result = true }
  in
  let config = Parser_run_config.set_experimental_effects base_config true in
  let parse_result = Parser_driver.run_string ~config source in
  match Parser_driver.parse_result_to_legacy parse_result with
  | Result.Error diag ->
      failwith (Printf.sprintf "Parse error: %s" (Diagnostic.to_string diag))
  | Result.Ok ast -> (
      Type_inference.reset_impl_registry ();
      let filtered_ast = filter_effect_decls ast in
      let typer_config = make_config ~type_row_mode:Type_row_dual_write () in
      match infer_compilation_unit ~config:typer_config filtered_ast with
      | Result.Ok tast -> tast
      | Result.Error err ->
          failwith
            (Printf.sprintf "Type error: %s"
               (Type_error.string_of_error err)))

let find_fn_decl tast name =
  tast.Typed_ast.tcu_items
  |> List.find_map (fun decl ->
         match decl.tdecl_kind with
         | TFnDecl fn when String.equal fn.tfn_name.name name -> Some fn
         | _ -> None)

let normalize_effect_names names =
  names
  |> List.map (fun name -> name |> String.lowercase_ascii |> String.trim)

let effect_tag_names tags =
  tags
  |> List.map (fun tag ->
         tag.Effect_profile.effect_name |> String.lowercase_ascii |> String.trim)

let effect_set_to_list set =
  Types.Effect_name_set.fold (fun name acc -> name :: acc) set []
  |> List.sort_uniq String.compare

let test_type_effect_row_equivalence () =
  Printf.printf "\nEffect Row Integration Tests:\n";
  run_test "type_effect_row_equivalence_profile" (fun () ->
      let tast = parse_and_infer_dual_write effect_row_fixture_source in
      match find_fn_decl tast "handled_demo" with
      | None -> failwith "handled_demo が型付き AST から見つかりません"
      | Some fn ->
          let profile = fn.tfn_effect_profile in
          let row = fn.tfn_effect_row in
          let declared_profile =
            effect_tag_names profile.Effect_profile.effect_set.declared
          in
          let residual_profile =
            effect_tag_names profile.Effect_profile.effect_set.residual
          in
          let row_declared = normalize_effect_names row.declared in
          let row_residual = normalize_effect_names row.residual in
          if row_declared <> declared_profile then
            failwith
              (Printf.sprintf
                 "効果行 declared の内容が一致しません。\n  期待: [%s]\n  実際: [%s]"
                 (String.concat ", " declared_profile)
                 (String.concat ", " row_declared));
          if row_residual <> residual_profile then
            failwith
              (Printf.sprintf
                 "効果行 residual の内容が一致しません。\n  期待: [%s]\n  実際: [%s]"
                 (String.concat ", " residual_profile)
                 (String.concat ", " row_residual));
          let expected_canonical =
            List.fold_left
              (fun acc name -> Types.Effect_name_set.add name acc)
              Types.Effect_name_set.empty
              (declared_profile @ residual_profile)
          in
          if not (Types.Effect_name_set.equal expected_canonical row.canonical)
          then
            let expected_str =
              String.concat ", " (effect_set_to_list expected_canonical)
            in
            let actual_str =
              String.concat ", " (effect_set_to_list row.canonical)
            in
            failwith
              (Printf.sprintf
                 "効果行 canonical の内容が一致しません。\n  期待: [%s]\n  実際: [%s]"
                 expected_str actual_str))

let test_type_effect_row_unify_guard () =
  Printf.printf "\nEffect Row Unification Tests:\n";
  run_test "type_effect_row_equivalence_unify_guard" (fun () ->
      let console_effect = Types.effect_row_make ~declared:[ "console" ] () in
      let pure_fn = Types.ty_arrow Types.ty_unit Types.ty_unit in
      let console_fn =
        Types.ty_arrow ~effect:console_effect Types.ty_unit Types.ty_unit
      in
      (match Constraint.unify [] pure_fn console_fn Ast.dummy_span with
      | Ok _ ->
          failwith
            "effect 行が異なる関数型の単一化が成功しました（失敗すべき）"
      | Error _ -> ());
      match Constraint.unify [] console_fn console_fn Ast.dummy_span with
      | Ok _ -> ()
      | Error err ->
          let message = Type_error.string_of_error err in
          failwith
            (Printf.sprintf "同一効果行の単一化が失敗しました: %s" message))

(* ========== 効果構文 PoC テスト草案 (SYNTAX-003 S2) ==========
 *
 * - `perform Console.log("msg")` が型 `Unit` を返しつつ `Σ_after = {Console}` を保持するケースを追加予定。
 * - `handle perform Console.log("msg") with handler { effect Console.log(msg) -> resume (); return 0 }` が `Σ_after = ∅` となる成功ケースを追加予定。
 * - `handle perform Console.log("msg") with handler { return 0 }` のように捕捉が無い場合に `effects.contract.residual` 診断を比較する失敗ケースを追加予定。
 *
 * Phase 2-7 で効果構文ノードが Typed AST に追加された際、上記ケースを `run_test` ベースで具体化し、`Type_inference_effect` の `Σ_before/Σ_after` 記録と連携する。
 *)

(* ========== メイン ========== *)

let () =
  test_basic_patterns ();
  test_tuple_patterns ();
  test_constructor_patterns ();
  test_nested_patterns ();
  test_match_expressions ();
  test_block_expressions ();
  test_function_declarations ();
  test_binary_operations ();
  test_composite_literals ();
  test_composite_literal_errors ();
  test_pattern_match_errors ();
  test_mutable_bindings ();
  test_trait_constraint_stage_metadata ();
  test_type_effect_row_equivalence ();
  test_type_effect_row_unify_guard ();
  Printf.printf "\nAll type inference tests passed! ✓\n"
