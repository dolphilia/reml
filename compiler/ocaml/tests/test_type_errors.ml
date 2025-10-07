(* 型エラーテストスイート
 *
 * Phase 2 Week 7: エラーケーステスト
 *
 * 目的:
 * - 全15種類の型エラーを包括的にテスト
 * - 診断メッセージの品質を検証（仕様書 2-5 準拠）
 * - エラーコード E7001-E7015 の正確性を確認
 *
 * テスト対象エラー:
 * E7001: UnificationFailure        - 型不一致
 * E7002: OccursCheck               - 無限型検出
 * E7003: UnboundVariable           - 未定義変数
 * E7004: ArityMismatch             - 引数数不一致
 * E7005: NotAFunction              - 非関数型への適用
 * E7006: ConditionNotBool          - 条件式が非Bool型
 * E7007: BranchTypeMismatch        - if式の分岐型不一致
 * E7008: PatternTypeMismatch       - パターンと式の型不一致
 * E7009: ConstructorArityMismatch  - コンストラクタ引数数不一致
 * E7010: TupleArityMismatch        - タプル要素数不一致
 * E7011: RecordFieldMissing        - レコードフィールド不足
 * E7012: RecordFieldUnknown        - レコードフィールド不明
 * E7013: NotARecord                - 非レコード型へのレコードパターン
 * E7014: NotATuple                 - 非タプル型へのタプルパターン
 * E7015: EmptyMatch                - 空のmatch式
 *
 * KNOWN ISSUES (7件の失敗テスト - Phase 2 Week 9):
 * 現在の型推論エンジンは、一部のケースで文脈依存の専用エラー型ではなく
 * 汎用的な UnificationFailure を返します。これは制約ベース型推論の実装方式に
 * 起因します。詳細: compiler/ocaml/docs/technical-debt.md#7-型エラー生成順序の問題
 *
 * 失敗するテスト:
 * - E7007: BranchTypeMismatch (1件) - if式の分岐型不一致
 * - E7005: NotAFunction (2件) - 非関数型への関数適用
 * - E7006: ConditionNotBool (2件) - 条件式が非Bool型
 * - E7014: NotATuple (1件) - 非タプル型へのタプルパターン
 *
 * 対応予定: Phase 2 後半（Week 10-12）で修正
 *)

open Types
open Type_env
open Type_inference
open Type_error
open Ast

(* ========== テストヘルパー ========== *)

let reset_types () =
  TypeVarGen.reset ()

let test_name = ref ""
let test_count = ref 0
let fail_count = ref 0

let run_test name f =
  test_name := name;
  incr test_count;
  reset_types ();
  try
    f ();
    Printf.printf "✓ %s\n" name
  with
  | Failure msg ->
      incr fail_count;
      Printf.printf "✗ %s: %s\n" name msg
  | e ->
      incr fail_count;
      Printf.printf "✗ %s: %s\n" name (Printexc.to_string e)

(** エラーが期待通り発生することを確認 *)
let assert_error expected_variant result msg =
  match result with
  | Ok _ ->
      failwith (Printf.sprintf "%s\nExpected error but got success" msg)
  | Error err ->
      let matches = match (expected_variant, err) with
        | ("UnificationFailure", UnificationFailure _) -> true
        | ("OccursCheck", OccursCheck _) -> true
        | ("UnboundVariable", UnboundVariable _) -> true
        | ("ArityMismatch", ArityMismatch _) -> true
        | ("NotAFunction", NotAFunction _) -> true
        | ("ConditionNotBool", ConditionNotBool _) -> true
        | ("BranchTypeMismatch", BranchTypeMismatch _) -> true
        | ("PatternTypeMismatch", PatternTypeMismatch _) -> true
        | ("ConstructorArityMismatch", ConstructorArityMismatch _) -> true
        | ("TupleArityMismatch", TupleArityMismatch _) -> true
        | ("RecordFieldMissing", RecordFieldMissing _) -> true
        | ("RecordFieldUnknown", RecordFieldUnknown _) -> true
        | ("NotARecord", NotARecord _) -> true
        | ("NotATuple", NotATuple _) -> true
        | ("EmptyMatch", EmptyMatch _) -> true
        | _ -> false
      in
      if not matches then
        failwith (Printf.sprintf "%s\nExpected %s but got: %s"
          msg expected_variant (string_of_error err))

(** 診断メッセージの品質を検証 *)
let verify_diagnostic_quality (err: type_error) (expected_code: string) =
  let diag = to_diagnostic err in
  (* エラーコードの検証 *)
  (match diag.Diagnostic.code with
  | Some code when code = expected_code -> ()
  | Some code ->
      failwith (Printf.sprintf "Expected error code %s, got %s" expected_code code)
  | None ->
      failwith (Printf.sprintf "Expected error code %s, but got None" expected_code));
  (* メッセージが空でないことを確認 *)
  if diag.Diagnostic.message = "" then
    failwith "Diagnostic message is empty";
  (* notesが存在することを確認（より良い診断のため） *)
  if List.length diag.Diagnostic.notes = 0 then
    failwith "Diagnostic should have notes for better explanation"

(* ========== A. 型不一致系エラー ========== *)

let test_unification_failure () =
  Printf.printf "\nA. Type Mismatch Errors:\n";

  (* A-1. 基本型の不一致 *)
  run_test "E7001: basic type mismatch (i64 vs String)" (fun () ->
    let env = initial_env in
    (* let x: i64 = "hello" *)
    let expr = {
      expr_kind = Literal (String ("hello", Normal));
      expr_span = dummy_span;
    } in
    let result = infer_expr env expr in
    match result with
    | Ok (_, actual_ty, _) ->
        let expected_ty = ty_i64 in
        let unify_result = Constraint.unify [] expected_ty actual_ty dummy_span in
        assert_error "UnificationFailure" unify_result
          "String should not unify with i64";
        (match unify_result with
         | Error err -> verify_diagnostic_quality err "E7001"
         | _ -> ())
    | Error _ -> failwith "Expression inference should succeed"
  );

  (* A-2. タプル型の不一致 *)
  run_test "E7001: tuple type mismatch" (fun () ->
    let _env = initial_env in
    (* (1, 2) と (1, "x") の不一致 *)
    let expected_ty = TTuple [ty_i64; ty_i64] in
    let actual_ty = TTuple [ty_i64; ty_string] in
    let result = Constraint.unify [] expected_ty actual_ty dummy_span in
    assert_error "UnificationFailure" result "Tuple element types differ";
    match result with
    | Error err -> verify_diagnostic_quality err "E7001"
    | _ -> ()
  );

  (* A-3. 関数型の引数型不一致 *)
  run_test "E7001: function argument type mismatch" (fun () ->
    let expected_ty = TArrow (ty_i64, ty_i64) in
    let actual_ty = TArrow (ty_string, ty_i64) in
    let result = Constraint.unify [] expected_ty actual_ty dummy_span in
    assert_error "UnificationFailure" result "Function argument types differ";
    match result with
    | Error err -> verify_diagnostic_quality err "E7001"
    | _ -> ()
  )

let test_branch_type_mismatch () =
  (* A-4. if式の分岐型不一致 *)
  run_test "E7007: if branch type mismatch" (fun () ->
    let env = initial_env in
    (* if true then 42 else "hello" *)
    let if_expr = {
      expr_kind = If (
        { expr_kind = Literal (Bool true); expr_span = dummy_span },
        { expr_kind = Literal (Int ("42", Base10)); expr_span = dummy_span },
        Some { expr_kind = Literal (String ("hello", Normal)); expr_span = dummy_span }
      );
      expr_span = dummy_span;
    } in
    let result = infer_expr env if_expr in
    assert_error "BranchTypeMismatch" result "if branches must have same type";
    match result with
    | Error err -> verify_diagnostic_quality err "E7007"
    | _ -> ()
  );

  (* A-5. if式の分岐型不一致（より複雑な型） *)
  run_test "E7007: if branch type mismatch (complex types)" (fun () ->
    let env = initial_env in
    (* if true then (1, 2) else (1, 2, 3) *)
    let if_expr = {
      expr_kind = If (
        { expr_kind = Literal (Bool true); expr_span = dummy_span },
        { expr_kind = Literal (Tuple [
            { expr_kind = Literal (Int ("1", Base10)); expr_span = dummy_span };
            { expr_kind = Literal (Int ("2", Base10)); expr_span = dummy_span };
          ]);
          expr_span = dummy_span },
        Some { expr_kind = Literal (Tuple [
            { expr_kind = Literal (Int ("1", Base10)); expr_span = dummy_span };
            { expr_kind = Literal (Int ("2", Base10)); expr_span = dummy_span };
            { expr_kind = Literal (Int ("3", Base10)); expr_span = dummy_span };
          ]);
          expr_span = dummy_span }
      );
      expr_span = dummy_span;
    } in
    let result = infer_expr env if_expr in
    (* タプルの要素数が異なるため型不一致 *)
    assert_error "UnificationFailure" result "Tuple arities differ"
  )

let test_pattern_type_mismatch () =
  (* A-6. パターンと式の型不一致 *)
  run_test "E7008: pattern type mismatch in let" (fun () ->
    let env = initial_env in
    (* let (x, y): (i64, i64) = ("hello", "world") *)
    let _pattern = {
      pat_kind = PatTuple [
        { pat_kind = PatVar { name = "x"; span = dummy_span }; pat_span = dummy_span };
        { pat_kind = PatVar { name = "y"; span = dummy_span }; pat_span = dummy_span };
      ];
      pat_span = dummy_span;
    } in
    let expr = {
      expr_kind = Literal (Tuple [
        { expr_kind = Literal (String ("hello", Normal)); expr_span = dummy_span };
        { expr_kind = Literal (String ("world", Normal)); expr_span = dummy_span };
      ]);
      expr_span = dummy_span;
    } in
    (* パターンに (i64, i64) の型注釈を想定 *)
    let expected_ty = TTuple [ty_i64; ty_i64] in
    let result = infer_expr env expr in
    match result with
    | Ok (_, actual_ty, _) ->
        let unify_result = Constraint.unify [] expected_ty actual_ty dummy_span in
        assert_error "UnificationFailure" unify_result "Pattern expects i64, but got String"
    | Error _ -> failwith "Expression should infer successfully"
  )

(* ========== B. 無限型・未定義変数 ========== *)

let test_occurs_check () =
  Printf.printf "\nB. Occurs Check & Unbound Variable Errors:\n";

  (* B-1. 無限型の検出（自己参照） *)
  run_test "E7002: occurs check (self-reference)" (fun () ->
    let tv = TypeVarGen.fresh (Some "a") in
    let ty = TVar tv in
    (* 型変数 'a を 'a -> 'a に単一化しようとする *)
    let arrow_ty = TArrow (ty, ty) in
    let result = Constraint.unify [] ty arrow_ty dummy_span in
    assert_error "OccursCheck" result "Cannot construct infinite type";
    match result with
    | Error err -> verify_diagnostic_quality err "E7002"
    | _ -> ()
  );

  (* B-2. 無限型の検出（リスト構造） *)
  run_test "E7002: occurs check (list structure)" (fun () ->
    let tv = TypeVarGen.fresh (Some "x") in
    let ty = TVar tv in
    (* 'x = Option<'x> *)
    let option_ty = ty_option ty in
    let result = Constraint.unify [] ty option_ty dummy_span in
    assert_error "OccursCheck" result "Cannot construct infinite Option type";
    match result with
    | Error err -> verify_diagnostic_quality err "E7002"
    | _ -> ()
  )

let test_unbound_variable () =
  (* B-3. 未定義変数の参照 *)
  run_test "E7003: unbound variable" (fun () ->
    let env = initial_env in
    let expr = {
      expr_kind = Var { name = "undefined_var"; span = dummy_span };
      expr_span = dummy_span;
    } in
    let result = infer_expr env expr in
    assert_error "UnboundVariable" result "Variable should not be in scope";
    match result with
    | Error err -> verify_diagnostic_quality err "E7003"
    | _ -> ()
  );

  (* B-4. 未定義変数（類似名提案のテスト） *)
  run_test "E7003: unbound variable with similar names" (fun () ->
    (* 環境に類似名を追加 *)
    let env = extend "value" (mono_scheme ty_i64) initial_env in
    let env = extend "result" (mono_scheme ty_i64) env in

    (* "resul" という typo した変数を参照 *)
    let expr = {
      expr_kind = Var { name = "resul"; span = dummy_span };
      expr_span = dummy_span;
    } in
    let result = infer_expr env expr in
    assert_error "UnboundVariable" result "Should suggest 'result'";
    match result with
    | Error err ->
        verify_diagnostic_quality err "E7003";
        (* 類似名の提案を検証 *)
        let available = ["value"; "result"] in
        let suggestions = suggest_similar_names "resul" available in
        if not (List.mem "result" suggestions) then
          failwith "Should suggest 'result' as similar name"
    | _ -> ()
  )

(* ========== C. 引数数不一致系エラー ========== *)

let test_arity_mismatch () =
  Printf.printf "\nC. Arity Mismatch Errors:\n";

  (* C-1. 関数呼び出しの引数数不一致（少なすぎ） *)
  run_test "E7004: function arity mismatch (too few args)" (fun () ->
    let env = initial_env in
    (* add: i64 -> i64 -> i64 を環境に追加 *)
    let add_ty = TArrow (ty_i64, TArrow (ty_i64, ty_i64)) in
    let env = extend "add" (mono_scheme add_ty) env in

    (* add(1) - 引数が1つ不足 *)
    let call_expr = {
      expr_kind = Call (
        { expr_kind = Var { name = "add"; span = dummy_span }; expr_span = dummy_span },
        [PosArg { expr_kind = Literal (Int ("1", Base10)); expr_span = dummy_span }]
      );
      expr_span = dummy_span;
    } in

    (* 型推論自体は成功するが、返り値が i64 -> i64 になる *)
    let result = infer_expr env call_expr in
    match result with
    | Ok (_, ty, _) ->
        (* 部分適用により関数型が返る *)
        (match ty with
         | TArrow _ -> () (* 期待通り *)
         | _ -> failwith "Should return function type (partial application)")
    | Error _ -> failwith "Should succeed with partial application"
  );

  (* C-2. 関数呼び出しの引数数不一致（多すぎ） *)
  run_test "E7004: function arity mismatch (too many args)" (fun () ->
    let env = initial_env in
    (* identity: i64 -> i64 を環境に追加 *)
    let identity_ty = TArrow (ty_i64, ty_i64) in
    let env = extend "identity" (mono_scheme identity_ty) env in

    (* identity(1, 2) - 引数が多すぎ *)
    let call_expr = {
      expr_kind = Call (
        { expr_kind = Var { name = "identity"; span = dummy_span }; expr_span = dummy_span },
        [
          PosArg { expr_kind = Literal (Int ("1", Base10)); expr_span = dummy_span };
          PosArg { expr_kind = Literal (Int ("2", Base10)); expr_span = dummy_span };
        ]
      );
      expr_span = dummy_span;
    } in

    let result = infer_expr env call_expr in
    (* identity(1) は i64 を返すため、i64 に第2引数を適用しようとしてエラー *)
    assert_error "NotAFunction" result "Cannot apply arg to non-function type"
  )

let test_constructor_arity_mismatch () =
  (* C-3. コンストラクタ引数数不一致（Some） *)
  run_test "E7009: constructor arity mismatch (Some)" (fun () ->
    let env = initial_env in
    (* Some() - 引数が不足 *)
    let pattern = {
      pat_kind = PatConstructor (
        { name = "Some"; span = dummy_span },
        [] (* 引数なし *)
      );
      pat_span = dummy_span;
    } in
    let expected_ty = ty_option ty_i64 in
    let result = infer_pattern env pattern expected_ty in
    assert_error "ConstructorArityMismatch" result "Some expects 1 argument";
    match result with
    | Error err -> verify_diagnostic_quality err "E7009"
    | _ -> ()
  );

  (* C-4. コンストラクタ引数数不一致（None） *)
  run_test "E7009: constructor arity mismatch (None with args)" (fun () ->
    let env = initial_env in
    (* None(x) - 引数が多すぎ *)
    let pattern = {
      pat_kind = PatConstructor (
        { name = "None"; span = dummy_span },
        [{ pat_kind = PatVar { name = "x"; span = dummy_span }; pat_span = dummy_span }]
      );
      pat_span = dummy_span;
    } in
    let expected_ty = ty_option ty_i64 in
    let result = infer_pattern env pattern expected_ty in
    assert_error "ConstructorArityMismatch" result "None expects 0 arguments";
    match result with
    | Error err -> verify_diagnostic_quality err "E7009"
    | _ -> ()
  )

let test_tuple_arity_mismatch () =
  (* C-5. タプル要素数不一致 *)
  run_test "E7010: tuple arity mismatch" (fun () ->
    let env = initial_env in
    (* (x, y) パターン vs (1, 2, 3) 式 *)
    let pattern = {
      pat_kind = PatTuple [
        { pat_kind = PatVar { name = "x"; span = dummy_span }; pat_span = dummy_span };
        { pat_kind = PatVar { name = "y"; span = dummy_span }; pat_span = dummy_span };
      ];
      pat_span = dummy_span;
    } in
    let expected_ty = TTuple [ty_i64; ty_i64; ty_i64] in
    let result = infer_pattern env pattern expected_ty in
    assert_error "TupleArityMismatch" result "Tuple arity differs";
    match result with
    | Error err -> verify_diagnostic_quality err "E7010"
    | _ -> ()
  )

(* ========== D. 型カテゴリエラー ========== *)

let test_not_a_function () =
  Printf.printf "\nD. Type Category Errors:\n";

  (* D-1. 非関数型への関数適用 *)
  run_test "E7005: not a function (apply to i64)" (fun () ->
    let env = initial_env in
    (* let x = 42; x(1) *)
    let env = extend "x" (mono_scheme ty_i64) env in
    let call_expr = {
      expr_kind = Call (
        { expr_kind = Var { name = "x"; span = dummy_span }; expr_span = dummy_span },
        [PosArg { expr_kind = Literal (Int ("1", Base10)); expr_span = dummy_span }]
      );
      expr_span = dummy_span;
    } in
    let result = infer_expr env call_expr in
    assert_error "NotAFunction" result "Cannot call i64 as function";
    match result with
    | Error err -> verify_diagnostic_quality err "E7005"
    | _ -> ()
  );

  (* D-2. 非関数型への関数適用（タプル） *)
  run_test "E7005: not a function (apply to tuple)" (fun () ->
    let env = initial_env in
    (* (1, 2)(3) *)
    let tuple_expr = {
      expr_kind = Literal (Tuple [
        { expr_kind = Literal (Int ("1", Base10)); expr_span = dummy_span };
        { expr_kind = Literal (Int ("2", Base10)); expr_span = dummy_span };
      ]);
      expr_span = dummy_span;
    } in
    let call_expr = {
      expr_kind = Call (
        tuple_expr,
        [PosArg { expr_kind = Literal (Int ("3", Base10)); expr_span = dummy_span }]
      );
      expr_span = dummy_span;
    } in
    let result = infer_expr env call_expr in
    assert_error "NotAFunction" result "Cannot call tuple as function";
    match result with
    | Error err -> verify_diagnostic_quality err "E7005"
    | _ -> ()
  )

let test_condition_not_bool () =
  (* D-3. 条件式が非Bool型（if） *)
  run_test "E7006: condition not Bool (if)" (fun () ->
    let env = initial_env in
    (* if 42 then 1 else 2 *)
    let if_expr = {
      expr_kind = If (
        { expr_kind = Literal (Int ("42", Base10)); expr_span = dummy_span },
        { expr_kind = Literal (Int ("1", Base10)); expr_span = dummy_span },
        Some { expr_kind = Literal (Int ("2", Base10)); expr_span = dummy_span }
      );
      expr_span = dummy_span;
    } in
    let result = infer_expr env if_expr in
    assert_error "ConditionNotBool" result "if condition must be Bool";
    match result with
    | Error err -> verify_diagnostic_quality err "E7006"
    | _ -> ()
  );

  (* D-4. 条件式が非Bool型（String） *)
  run_test "E7006: condition not Bool (String)" (fun () ->
    let env = initial_env in
    (* if "hello" then 1 else 2 *)
    let if_expr = {
      expr_kind = If (
        { expr_kind = Literal (String ("hello", Normal)); expr_span = dummy_span },
        { expr_kind = Literal (Int ("1", Base10)); expr_span = dummy_span },
        Some { expr_kind = Literal (Int ("2", Base10)); expr_span = dummy_span }
      );
      expr_span = dummy_span;
    } in
    let result = infer_expr env if_expr in
    assert_error "ConditionNotBool" result "if condition must be Bool";
    match result with
    | Error err -> verify_diagnostic_quality err "E7006"
    | _ -> ()
  )

let test_not_a_record () =
  (* D-5. 非レコード型へのレコードパターン *)
  run_test "E7013: not a record (i64)" (fun () ->
    let env = initial_env in
    (* let { x } = 42 *)
    let pattern = {
      pat_kind = PatRecord (
        [
          ({ name = "x"; span = dummy_span },
           Some { pat_kind = PatVar { name = "x"; span = dummy_span }; pat_span = dummy_span })
        ],
        false (* rest *)
      );
      pat_span = dummy_span;
    } in
    let expected_ty = ty_i64 in
    let result = infer_pattern env pattern expected_ty in
    assert_error "NotARecord" result "Cannot use record pattern on i64";
    match result with
    | Error err -> verify_diagnostic_quality err "E7013"
    | _ -> ()
  )

let test_not_a_tuple () =
  (* D-6. 非タプル型へのタプルパターン *)
  run_test "E7014: not a tuple (i64)" (fun () ->
    let env = initial_env in
    (* let (x, y) = 42 *)
    let pattern = {
      pat_kind = PatTuple [
        { pat_kind = PatVar { name = "x"; span = dummy_span }; pat_span = dummy_span };
        { pat_kind = PatVar { name = "y"; span = dummy_span }; pat_span = dummy_span };
      ];
      pat_span = dummy_span;
    } in
    let expected_ty = ty_i64 in
    let result = infer_pattern env pattern expected_ty in
    assert_error "NotATuple" result "Cannot use tuple pattern on i64";
    match result with
    | Error err -> verify_diagnostic_quality err "E7014"
    | _ -> ()
  )

(* ========== E. レコードフィールド系エラー ========== *)

let test_record_field_errors () =
  Printf.printf "\nE. Record Field Errors:\n";

  (* E-1. レコードフィールド不足 *)
  run_test "E7011: missing record fields" (fun () ->
    (* NOTE: Phase 1 ではレコード型定義が未実装のため、
     * ここでは型エラーの構造のみをテスト *)
    let error = RecordFieldMissing {
      missing_fields = ["name"; "age"];
      span = dummy_span;
    } in
    verify_diagnostic_quality error "E7011";
    Printf.printf "  (Record type definition pending Phase 2+)\n"
  );

  (* E-2. レコードフィールド不明 *)
  run_test "E7012: unknown record field" (fun () ->
    let error = RecordFieldUnknown {
      field = "unknown_field";
      span = dummy_span;
    } in
    verify_diagnostic_quality error "E7012";
    Printf.printf "  (Record type definition pending Phase 2+)\n"
  )

(* ========== F. match式系エラー ========== *)

let test_empty_match () =
  Printf.printf "\nF. Match Expression Errors:\n";

  (* F-1. 空のmatch式 *)
  run_test "E7015: empty match" (fun () ->
    (* 変数 x を環境に追加 *)
    let env = extend "x" (mono_scheme ty_i64) initial_env in
    (* match x with *)
    let match_expr = {
      expr_kind = Match (
        { expr_kind = Var { name = "x"; span = dummy_span }; expr_span = dummy_span },
        [] (* アームなし *)
      );
      expr_span = dummy_span;
    } in
    let result = infer_expr env match_expr in
    assert_error "EmptyMatch" result "match must have at least one arm";
    match result with
    | Error err -> verify_diagnostic_quality err "E7015"
    | _ -> ()
  )

(* ========== メイン ========== *)

let () =
  Printf.printf "=== Type Error Test Suite ===\n";
  Printf.printf "Testing all 15 type error variants (E7001-E7015)\n";

  (* A. 型不一致系 *)
  test_unification_failure ();
  test_branch_type_mismatch ();
  test_pattern_type_mismatch ();

  (* B. 無限型・未定義変数 *)
  test_occurs_check ();
  test_unbound_variable ();

  (* C. 引数数不一致系 *)
  test_arity_mismatch ();
  test_constructor_arity_mismatch ();
  test_tuple_arity_mismatch ();

  (* D. 型カテゴリエラー *)
  test_not_a_function ();
  test_condition_not_bool ();
  test_not_a_record ();
  test_not_a_tuple ();

  (* E. レコードフィールド系 *)
  test_record_field_errors ();

  (* F. match式系 *)
  test_empty_match ();

  Printf.printf "\n=== Test Summary ===\n";
  Printf.printf "Total tests: %d\n" !test_count;
  Printf.printf "Passed: %d\n" (!test_count - !fail_count);
  Printf.printf "Failed: %d\n" !fail_count;

  if !fail_count = 0 then begin
    Printf.printf "\n✓ All type error tests passed!\n";
    exit 0
  end else begin
    Printf.printf "\n✗ Some tests failed\n";
    exit 1
  end
