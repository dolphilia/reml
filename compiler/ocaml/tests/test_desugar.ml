(* Test_desugar — 糖衣削除パスのテスト (Phase 3)
 *
 * Typed AST → Core IR への変換をテストする。
 *
 * テスト戦略:
 * - 各式種別の基本的な変換
 * - パターンマッチの決定木変換
 * - パイプ演算子の展開
 * - let再束縛の正規化
 *)

open Types
open Ast
open Typed_ast
open Core_ir.Ir
open Core_ir.Desugar

(* ========== テストユーティリティ ========== *)

let dummy_span = { start = 0; end_ = 0 }

let make_typed_expr kind ty =
  {
    texpr_kind = kind;
    texpr_ty = ty;
    texpr_span = dummy_span;
  }

(* ========== リテラル変換テスト ========== *)

let test_desugar_int_literal () =
  Printf.printf "test_desugar_int_literal ... ";
  let texpr = make_typed_expr (TLiteral (Int ("42", Base10))) ty_i64 in
  let map = create_scope_map () in
  let result = desugar_expr map texpr in

  match result.expr_kind with
  | Literal (Int ("42", Base10)) ->
      Printf.printf "OK\n"
  | _ ->
      failwith "整数リテラルの変換に失敗"

let test_desugar_bool_literal () =
  Printf.printf "test_desugar_bool_literal ... ";
  let texpr = make_typed_expr (TLiteral (Bool true)) ty_bool in
  let map = create_scope_map () in
  let result = desugar_expr map texpr in

  match result.expr_kind with
  | Literal (Bool true) ->
      Printf.printf "OK\n"
  | _ ->
      failwith "真偽値リテラルの変換に失敗"

let test_desugar_unit_literal () =
  Printf.printf "test_desugar_unit_literal ... ";
  let texpr = make_typed_expr (TLiteral Unit) ty_unit in
  let map = create_scope_map () in
  let result = desugar_expr map texpr in

  match result.expr_kind with
  | Literal Unit ->
      Printf.printf "OK\n"
  | _ ->
      failwith "Unitリテラルの変換に失敗"

(* ========== 変数参照テスト ========== *)

let test_desugar_var () =
  Printf.printf "test_desugar_var ... ";
  let id = { name = "x"; span = dummy_span } in
  let scheme = { quantified = []; body = ty_i64 } in
  let texpr = make_typed_expr (TVar (id, scheme)) ty_i64 in
  let map = create_scope_map () in
  let result = desugar_expr map texpr in

  match result.expr_kind with
  | Var var when var.vname = "x" && var.vty = ty_i64 ->
      Printf.printf "OK\n"
  | _ ->
      failwith "変数参照の変換に失敗"

(* ========== パイプ展開テスト ========== *)

let test_desugar_pipe_simple () =
  Printf.printf "test_desugar_pipe_simple ... ";
  (* 42 |> f の変換 *)
  let arg = make_typed_expr (TLiteral (Int ("42", Base10))) ty_i64 in
  let fn_id = { name = "f"; span = dummy_span } in
  let fn_scheme = { quantified = []; body = TArrow (ty_i64, ty_i64) } in
  let fn = make_typed_expr (TVar (fn_id, fn_scheme)) (TArrow (ty_i64, ty_i64)) in
  let pipe_expr = make_typed_expr (TPipe (arg, fn)) ty_i64 in

  let map = create_scope_map () in
  let result = desugar_expr map pipe_expr in

  (* Let ($pipe, 42, App(f, $pipe)) の形式を期待 *)
  match result.expr_kind with
  | Let (_temp_var, bound, body) ->
      begin match (bound.expr_kind, body.expr_kind) with
      | (Literal (Int ("42", Base10)), App (_fn_expr, [_arg_ref])) ->
          Printf.printf "OK\n"
      | _ ->
          failwith "パイプ展開の束縛式または本体が不正"
      end
  | _ ->
      failwith "パイプ展開の変換に失敗"

(* ========== if式変換テスト ========== *)

let test_desugar_if_then_else () =
  Printf.printf "test_desugar_if_then_else ... ";
  (* if true then 1 else 0 の変換 *)
  let cond = make_typed_expr (TLiteral (Bool true)) ty_bool in
  let then_e = make_typed_expr (TLiteral (Int ("1", Base10))) ty_i64 in
  let else_e = make_typed_expr (TLiteral (Int ("0", Base10))) ty_i64 in
  let if_expr = make_typed_expr (TIf (cond, then_e, Some else_e)) ty_i64 in

  let map = create_scope_map () in
  let result = desugar_expr map if_expr in

  match result.expr_kind with
  | If (cond_expr, then_expr, else_expr) ->
      begin match (cond_expr.expr_kind, then_expr.expr_kind, else_expr.expr_kind) with
      | (Literal (Bool true), Literal (Int ("1", Base10)), Literal (Int ("0", Base10))) ->
          Printf.printf "OK\n"
      | _ ->
          failwith "if式の各分岐が不正"
      end
  | _ ->
      failwith "if式の変換に失敗"

(* ========== ブロック式変換テスト ========== *)

let test_desugar_block_single_expr () =
  Printf.printf "test_desugar_block_single_expr ... ";
  (* { 42 } の変換 *)
  let expr = make_typed_expr (TLiteral (Int ("42", Base10))) ty_i64 in
  let block_expr = make_typed_expr (TBlock [TExprStmt expr]) ty_i64 in

  let map = create_scope_map () in
  let result = desugar_expr map block_expr in

  match result.expr_kind with
  | Literal (Int ("42", Base10)) ->
      Printf.printf "OK\n"
  | _ ->
      failwith "単一式ブロックの変換に失敗"

let test_desugar_block_empty () =
  Printf.printf "test_desugar_block_empty ... ";
  (* {} の変換 → () *)
  let block_expr = make_typed_expr (TBlock []) ty_unit in

  let map = create_scope_map () in
  let result = desugar_expr map block_expr in

  match result.expr_kind with
  | Literal Unit ->
      Printf.printf "OK\n"
  | _ ->
      failwith "空ブロックの変換に失敗"

(* ========== メインテストランナー ========== *)

let run_tests () =
  Printf.printf "\n=== 糖衣削除パステスト開始 ===\n\n";

  (* リテラル変換 *)
  Printf.printf "--- リテラル変換 ---\n";
  test_desugar_int_literal ();
  test_desugar_bool_literal ();
  test_desugar_unit_literal ();

  (* 変数参照 *)
  Printf.printf "\n--- 変数参照 ---\n";
  test_desugar_var ();

  (* パイプ演算子 *)
  Printf.printf "\n--- パイプ演算子 ---\n";
  test_desugar_pipe_simple ();

  (* if式 *)
  Printf.printf "\n--- if式 ---\n";
  test_desugar_if_then_else ();

  (* ブロック式 *)
  Printf.printf "\n--- ブロック式 ---\n";
  test_desugar_block_single_expr ();
  test_desugar_block_empty ();

  Printf.printf "\n=== 全テスト成功 ===\n\n"

let () = run_tests ()
