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
open Constraint_solver
open Core_ir.Ir
open Core_ir.Desugar
module CFG = Core_ir.Cfg

(* ========== テストユーティリティ ========== *)

let dummy_span = { start = 0; end_ = 0 }

let make_typed_expr kind ty =
  {
    texpr_kind = kind;
    texpr_ty = ty;
    texpr_span = dummy_span;
    texpr_dict_refs = [];
  }

let test_desugar_loop_continue () =
  Printf.printf "test_desugar_loop_continue ... ";
  Core_ir.Ir.VarIdGen.reset ();
  Core_ir.Ir.LabelGen.reset ();

  let i_var = Core_ir.Ir.VarIdGen.fresh ~mutable_:true "i" ty_i64 dummy_span in
  let i_ref = Core_ir.Ir.make_expr (Var i_var) ty_i64 dummy_span in
  let one_expr =
    Core_ir.Ir.make_expr (Literal (Ast.Int ("1", Ast.Base10))) ty_i64 dummy_span
  in
  let add_expr =
    Core_ir.Ir.make_expr
      (Primitive (PrimAdd, [ i_ref; one_expr ]))
      ty_i64 dummy_span
  in
  let assign_expr =
    Core_ir.Ir.make_expr (AssignMutable (i_var, add_expr)) ty_unit dummy_span
  in
  let continue_expr = Core_ir.Ir.make_expr Continue ty_unit dummy_span in
  let seq_var = Core_ir.Ir.VarIdGen.fresh "$seq" ty_unit dummy_span in
  let body_expr =
    Core_ir.Ir.make_expr
      (Let (seq_var, assign_expr, continue_expr))
      ty_unit dummy_span
  in
  let cond_expr =
    Core_ir.Ir.make_expr (Literal (Ast.Bool true)) ty_bool dummy_span
  in
  let loop_expr =
    Core_ir.Desugar.make_loop_expr (WhileLoop cond_expr) body_expr dummy_span
      ty_unit
  in
  (match loop_expr.expr_kind with
  | Loop info -> (
      if not info.loop_contains_continue then
        failwith "loop_contains_continue が true ではありません";
      match info.loop_carried with
      | [ lc ] -> (
          let continue_src =
            List.find_opt
              (fun src -> src.ls_kind = LoopSourceContinue)
              lc.lc_sources
          in
          match continue_src with
          | Some src -> (
              match src.ls_expr.expr_kind with
              | Primitive
                  ( PrimAdd,
                    [ _; { expr_kind = Literal (Int ("1", Base10)); _ } ] ) ->
                  ()
              | _ -> failwith "continue 経路の ls_expr が想定した加算式ではありません")
          | None -> failwith "LoopSourceContinue が loop_carried に含まれていません")
      | _ -> failwith "loop_carried の要素数が期待と異なります")
  | _ -> failwith "Loop 式の生成に失敗しました");
  Printf.printf "OK\n"

(* ========== リテラル変換テスト ========== *)

let test_desugar_int_literal () =
  Printf.printf "test_desugar_int_literal ... ";
  let texpr = make_typed_expr (TLiteral (Int ("42", Base10))) ty_i64 in
  let map = create_scope_map () in
  let result = desugar_expr map texpr in

  match result.expr_kind with
  | Literal (Int ("42", Base10)) -> Printf.printf "OK\n"
  | _ -> failwith "整数リテラルの変換に失敗"

let test_desugar_bool_literal () =
  Printf.printf "test_desugar_bool_literal ... ";
  let texpr = make_typed_expr (TLiteral (Bool true)) ty_bool in
  let map = create_scope_map () in
  let result = desugar_expr map texpr in

  match result.expr_kind with
  | Literal (Bool true) -> Printf.printf "OK\n"
  | _ -> failwith "真偽値リテラルの変換に失敗"

let test_desugar_unit_literal () =
  Printf.printf "test_desugar_unit_literal ... ";
  let texpr = make_typed_expr (TLiteral Unit) ty_unit in
  let map = create_scope_map () in
  let result = desugar_expr map texpr in

  match result.expr_kind with
  | Literal Unit -> Printf.printf "OK\n"
  | _ -> failwith "Unitリテラルの変換に失敗"

(* ========== 変数参照テスト ========== *)

let test_desugar_var () =
  Printf.printf "test_desugar_var ... ";
  let id = { name = "x"; span = dummy_span } in
  let scheme = scheme_to_constrained { quantified = []; body = ty_i64 } in
  let texpr = make_typed_expr (TVar (id, scheme)) ty_i64 in
  let map = create_scope_map () in
  let result = desugar_expr map texpr in

  match result.expr_kind with
  | Var var when var.vname = "x" && var.vty = ty_i64 -> Printf.printf "OK\n"
  | _ -> failwith "変数参照の変換に失敗"

(* ========== パイプ展開テスト ========== *)

let test_desugar_pipe_simple () =
  Printf.printf "test_desugar_pipe_simple ... ";
  (* 42 |> f の変換 *)
  let arg = make_typed_expr (TLiteral (Int ("42", Base10))) ty_i64 in
  let fn_id = { name = "f"; span = dummy_span } in
  let fn_scheme =
    scheme_to_constrained { quantified = []; body = TArrow (ty_i64, ty_i64) }
  in
  let fn =
    make_typed_expr (TVar (fn_id, fn_scheme)) (TArrow (ty_i64, ty_i64))
  in
  let pipe_expr = make_typed_expr (TPipe (arg, fn)) ty_i64 in

  let map = create_scope_map () in
  let result = desugar_expr map pipe_expr in

  (* Let ($pipe, 42, App(f, $pipe)) の形式を期待 *)
  match result.expr_kind with
  | Let (_temp_var, bound, body) -> (
      match (bound.expr_kind, body.expr_kind) with
      | Literal (Int ("42", Base10)), App (_fn_expr, [ _arg_ref ]) ->
          Printf.printf "OK\n"
      | _ -> failwith "パイプ展開の束縛式または本体が不正")
  | _ -> failwith "パイプ展開の変換に失敗"

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
  | If (cond_expr, then_expr, else_expr) -> (
      match (cond_expr.expr_kind, then_expr.expr_kind, else_expr.expr_kind) with
      | ( Literal (Bool true),
          Literal (Int ("1", Base10)),
          Literal (Int ("0", Base10)) ) ->
          Printf.printf "OK\n"
      | _ -> failwith "if式の各分岐が不正")
  | _ -> failwith "if式の変換に失敗"

(* ========== ブロック式変換テスト ========== *)

let test_desugar_block_single_expr () =
  Printf.printf "test_desugar_block_single_expr ... ";
  (* { 42 } の変換 *)
  let expr = make_typed_expr (TLiteral (Int ("42", Base10))) ty_i64 in
  let block_expr = make_typed_expr (TBlock [ TExprStmt expr ]) ty_i64 in

  let map = create_scope_map () in
  let result = desugar_expr map block_expr in

  match result.expr_kind with
  | Literal (Int ("42", Base10)) -> Printf.printf "OK\n"
  | _ -> failwith "単一式ブロックの変換に失敗"

let test_desugar_block_empty () =
  Printf.printf "test_desugar_block_empty ... ";
  (* {} の変換 → () *)
  let block_expr = make_typed_expr (TBlock []) ty_unit in

  let map = create_scope_map () in
  let result = desugar_expr map block_expr in

  match result.expr_kind with
  | Literal Unit -> Printf.printf "OK\n"
  | _ -> failwith "空ブロックの変換に失敗"

let terminator_signature blocks =
  let kind_of = function
    | TermReturn _ -> "return"
    | TermJump _ -> "jump"
    | TermBranch _ -> "branch"
    | TermSwitch _ -> "switch"
    | TermUnreachable -> "unreachable"
  in
  List.map (fun block -> kind_of block.terminator) blocks

let build_for_loop_cfg source_kind =
  Core_ir.Ir.VarIdGen.reset ();
  Core_ir.Ir.LabelGen.reset ();
  let map = create_scope_map () in
  let pat_ident = { name = "item"; span = dummy_span } in
  let typed_pat =
    make_typed_pattern (TPatVar pat_ident) ty_i64
      [ ("item", ty_i64) ]
      dummy_span
  in
  let body_expr = make_typed_expr (TBlock []) ty_unit in
  let source_ident, source_ty =
    match source_kind with
    | `Array ->
        ( { name = "arr"; span = dummy_span },
          TApp (TCon (TCUser "Array"), ty_i64) )
    | `Iterator ->
        ( { name = "iter"; span = dummy_span },
          TApp (TCon (TCUser "Iterator"), ty_i64) )
  in
  let scheme = scheme_to_constrained { quantified = []; body = source_ty } in
  let source_expr = make_typed_expr (TVar (source_ident, scheme)) source_ty in
  let source_var =
    Core_ir.Ir.VarIdGen.fresh source_ident.name (convert_ty source_ty)
      dummy_span
  in
  bind_var map source_ident.name source_var;
  let iterator_dict =
    match source_kind with
    | `Array -> DictImplicit ("Iterator", [ source_ty; ty_i64 ])
    | `Iterator -> DictImplicit ("Iterator", [ source_ty; ty_i64 ])
  in
  let for_expr =
    make_typed_expr
      (TFor (typed_pat, source_expr, body_expr, iterator_dict, None))
      ty_unit
  in
  let core_expr = desugar_expr map for_expr in
  CFG.build_cfg_from_expr core_expr

let test_desugar_for_loop_cfg_equivalence () =
  Printf.printf "test_desugar_for_loop_cfg_equivalence ... ";
  let array_blocks = build_for_loop_cfg `Array in
  let iterator_blocks = build_for_loop_cfg `Iterator in
  let sig_array = terminator_signature array_blocks in
  let sig_iterator = terminator_signature iterator_blocks in
  if sig_array <> sig_iterator then failwith "配列版とiterator版でCFG終端種別の並びが一致しません";
  Printf.printf "OK\n"

(* ========== タプルパターン変換テスト ========== *)

let test_desugar_tuple_pattern_simple () =
  Printf.printf "test_desugar_tuple_pattern_simple ... ";
  (* let (x, y) = (1, 2) in x + y *)
  let x_id = { name = "x"; span = dummy_span } in
  let y_id = { name = "y"; span = dummy_span } in
  let x_pat =
    {
      tpat_kind = TPatVar x_id;
      tpat_ty = ty_i64;
      tpat_bindings = [ ("x", ty_i64) ];
      tpat_span = dummy_span;
    }
  in
  let y_pat =
    {
      tpat_kind = TPatVar y_id;
      tpat_ty = ty_i64;
      tpat_bindings = [ ("y", ty_i64) ];
      tpat_span = dummy_span;
    }
  in
  let tuple_pat =
    {
      tpat_kind = TPatTuple [ x_pat; y_pat ];
      tpat_ty = TTuple [ ty_i64; ty_i64 ];
      tpat_bindings = [ ("x", ty_i64); ("y", ty_i64) ];
      tpat_span = dummy_span;
    }
  in

  let bound = make_typed_expr (TLiteral (Int ("1", Base10))) ty_i64 in
  let rest = make_typed_expr (TLiteral (Int ("2", Base10))) ty_i64 in

  let map = create_scope_map () in
  let bound_ir = desugar_expr map bound in
  let rest_ir = desugar_expr map rest in
  let result =
    desugar_pattern_binding map tuple_pat bound_ir ~mutability:BindingImmutable
      ~cont:(fun () -> rest_ir)
      ty_i64 dummy_span
  in

  (* Let ($tuple, ..., Let ($tuple_elem0, ..., Let ($tuple_elem1, ..., rest))) の形式を期待 *)
  match result.expr_kind with
  | Let (_tuple_var, _bound, _inner) -> Printf.printf "OK\n"
  | _ -> failwith "タプルパターン変換に失敗"

let test_desugar_tuple_pattern_nested () =
  Printf.printf "test_desugar_tuple_pattern_nested ... ";
  (* let ((a, b), c) = ((1, 2), 3) *)
  let a_id = { name = "a"; span = dummy_span } in
  let b_id = { name = "b"; span = dummy_span } in
  let c_id = { name = "c"; span = dummy_span } in

  let a_pat =
    {
      tpat_kind = TPatVar a_id;
      tpat_ty = ty_i64;
      tpat_bindings = [ ("a", ty_i64) ];
      tpat_span = dummy_span;
    }
  in
  let b_pat =
    {
      tpat_kind = TPatVar b_id;
      tpat_ty = ty_i64;
      tpat_bindings = [ ("b", ty_i64) ];
      tpat_span = dummy_span;
    }
  in
  let inner_tuple_pat =
    {
      tpat_kind = TPatTuple [ a_pat; b_pat ];
      tpat_ty = TTuple [ ty_i64; ty_i64 ];
      tpat_bindings = [ ("a", ty_i64); ("b", ty_i64) ];
      tpat_span = dummy_span;
    }
  in
  let c_pat =
    {
      tpat_kind = TPatVar c_id;
      tpat_ty = ty_i64;
      tpat_bindings = [ ("c", ty_i64) ];
      tpat_span = dummy_span;
    }
  in
  let outer_tuple_pat =
    {
      tpat_kind = TPatTuple [ inner_tuple_pat; c_pat ];
      tpat_ty = TTuple [ TTuple [ ty_i64; ty_i64 ]; ty_i64 ];
      tpat_bindings = [ ("a", ty_i64); ("b", ty_i64); ("c", ty_i64) ];
      tpat_span = dummy_span;
    }
  in

  let bound = make_typed_expr (TLiteral (Int ("1", Base10))) ty_i64 in
  let rest = make_typed_expr (TLiteral (Int ("2", Base10))) ty_i64 in

  let map = create_scope_map () in
  let bound_ir = desugar_expr map bound in
  let rest_ir = desugar_expr map rest in
  let result =
    desugar_pattern_binding map outer_tuple_pat bound_ir
      ~mutability:BindingImmutable
      ~cont:(fun () -> rest_ir)
      ty_i64 dummy_span
  in

  match result.expr_kind with
  | Let (_outer_tuple_var, _bound, _inner) -> Printf.printf "OK\n"
  | _ -> failwith "ネストタプルパターン変換に失敗"

let test_desugar_tuple_pattern_with_wildcard () =
  Printf.printf "test_desugar_tuple_pattern_with_wildcard ... ";
  (* let (x, _, z) = (1, 2, 3) *)
  let x_id = { name = "x"; span = dummy_span } in
  let z_id = { name = "z"; span = dummy_span } in

  let x_pat =
    {
      tpat_kind = TPatVar x_id;
      tpat_ty = ty_i64;
      tpat_bindings = [ ("x", ty_i64) ];
      tpat_span = dummy_span;
    }
  in
  let wild_pat =
    {
      tpat_kind = TPatWildcard;
      tpat_ty = ty_i64;
      tpat_bindings = [];
      tpat_span = dummy_span;
    }
  in
  let z_pat =
    {
      tpat_kind = TPatVar z_id;
      tpat_ty = ty_i64;
      tpat_bindings = [ ("z", ty_i64) ];
      tpat_span = dummy_span;
    }
  in
  let tuple_pat =
    {
      tpat_kind = TPatTuple [ x_pat; wild_pat; z_pat ];
      tpat_ty = TTuple [ ty_i64; ty_i64; ty_i64 ];
      tpat_bindings = [ ("x", ty_i64); ("z", ty_i64) ];
      tpat_span = dummy_span;
    }
  in

  let bound = make_typed_expr (TLiteral (Int ("1", Base10))) ty_i64 in
  let rest = make_typed_expr (TLiteral (Int ("2", Base10))) ty_i64 in

  let map = create_scope_map () in
  let bound_ir = desugar_expr map bound in
  let rest_ir = desugar_expr map rest in
  let result =
    desugar_pattern_binding map tuple_pat bound_ir ~mutability:BindingImmutable
      ~cont:(fun () -> rest_ir)
      ty_i64 dummy_span
  in

  match result.expr_kind with
  | Let (_tuple_var, _bound, _inner) -> Printf.printf "OK\n"
  | _ -> failwith "ワイルドカード混在タプルパターン変換に失敗"

(* ========== レコードパターン変換テスト ========== *)

let test_desugar_record_pattern_basic () =
  Printf.printf "test_desugar_record_pattern_basic ... ";
  (* let { x, y } = { x: 1, y: 2 } *)
  let x_id = { name = "x"; span = dummy_span } in
  let y_id = { name = "y"; span = dummy_span } in

  let record_pat =
    {
      tpat_kind = TPatRecord ([ (x_id, None); (y_id, None) ], false);
      tpat_ty = ty_i64;
      (* 仮の型 *)
      tpat_bindings = [ ("x", ty_i64); ("y", ty_i64) ];
      tpat_span = dummy_span;
    }
  in

  let bound = make_typed_expr (TLiteral (Int ("1", Base10))) ty_i64 in
  let rest = make_typed_expr (TLiteral (Int ("2", Base10))) ty_i64 in

  let map = create_scope_map () in
  let bound_ir = desugar_expr map bound in
  let rest_ir = desugar_expr map rest in
  let result =
    desugar_pattern_binding map record_pat bound_ir ~mutability:BindingImmutable
      ~cont:(fun () -> rest_ir)
      ty_i64 dummy_span
  in

  match result.expr_kind with
  | Let (_record_var, _bound, _inner) -> Printf.printf "OK\n"
  | _ -> failwith "レコードパターン変換に失敗"

let test_desugar_record_pattern_with_rest () =
  Printf.printf "test_desugar_record_pattern_with_rest ... ";
  (* let { x, .. } = { x: 1, y: 2, z: 3 } *)
  let x_id = { name = "x"; span = dummy_span } in

  let record_pat =
    {
      tpat_kind = TPatRecord ([ (x_id, None) ], true);
      tpat_ty = ty_i64;
      tpat_bindings = [ ("x", ty_i64) ];
      tpat_span = dummy_span;
    }
  in

  let bound = make_typed_expr (TLiteral (Int ("1", Base10))) ty_i64 in
  let rest = make_typed_expr (TLiteral (Int ("2", Base10))) ty_i64 in

  let map = create_scope_map () in
  let bound_ir = desugar_expr map bound in
  let rest_ir = desugar_expr map rest in
  let result =
    desugar_pattern_binding map record_pat bound_ir ~mutability:BindingImmutable
      ~cont:(fun () -> rest_ir)
      ty_i64 dummy_span
  in

  match result.expr_kind with
  | Let (_record_var, _bound, _inner) -> Printf.printf "OK\n"
  | _ -> failwith "rest付きレコードパターン変換に失敗"

let test_desugar_record_pattern_nested () =
  Printf.printf "test_desugar_record_pattern_nested ... ";
  (* let { inner: { value } } = { inner: { value: 42 } } *)
  let value_id = { name = "value"; span = dummy_span } in
  let inner_id = { name = "inner"; span = dummy_span } in

  let value_pat =
    {
      tpat_kind = TPatVar value_id;
      tpat_ty = ty_i64;
      tpat_bindings = [ ("value", ty_i64) ];
      tpat_span = dummy_span;
    }
  in
  let inner_record_pat =
    {
      tpat_kind = TPatRecord ([ (value_id, Some value_pat) ], false);
      tpat_ty = ty_i64;
      tpat_bindings = [ ("value", ty_i64) ];
      tpat_span = dummy_span;
    }
  in
  let outer_record_pat =
    {
      tpat_kind = TPatRecord ([ (inner_id, Some inner_record_pat) ], false);
      tpat_ty = ty_i64;
      tpat_bindings = [ ("value", ty_i64) ];
      tpat_span = dummy_span;
    }
  in

  let bound = make_typed_expr (TLiteral (Int ("42", Base10))) ty_i64 in
  let rest = make_typed_expr (TLiteral (Int ("0", Base10))) ty_i64 in

  let map = create_scope_map () in
  let bound_ir = desugar_expr map bound in
  let rest_ir = desugar_expr map rest in
  let result =
    desugar_pattern_binding map outer_record_pat bound_ir
      ~mutability:BindingImmutable
      ~cont:(fun () -> rest_ir)
      ty_i64 dummy_span
  in

  match result.expr_kind with
  | Let (_outer_record_var, _bound, _inner) -> Printf.printf "OK\n"
  | _ -> failwith "ネストレコードパターン変換に失敗"

(* ========== コンストラクタパターン変換テスト ========== *)

let test_desugar_constructor_pattern_some () =
  Printf.printf "test_desugar_constructor_pattern_some ... ";
  (* match opt with | Some(x) -> x *)
  let x_id = { name = "x"; span = dummy_span } in
  let some_id = { name = "Some"; span = dummy_span } in

  let x_pat =
    {
      tpat_kind = TPatVar x_id;
      tpat_ty = ty_i64;
      tpat_bindings = [ ("x", ty_i64) ];
      tpat_span = dummy_span;
    }
  in
  let some_pat =
    {
      tpat_kind = TPatConstructor (some_id, [ x_pat ]);
      tpat_ty = TApp (TCon (TCUser "Option"), ty_i64);
      tpat_bindings = [ ("x", ty_i64) ];
      tpat_span = dummy_span;
    }
  in

  let bound = make_typed_expr (TLiteral (Int ("42", Base10))) ty_i64 in
  let rest =
    make_typed_expr
      (TVar (x_id, scheme_to_constrained { quantified = []; body = ty_i64 }))
      ty_i64
  in

  let map = create_scope_map () in
  let bound_ir = desugar_expr map bound in
  let rest_ir = desugar_expr map rest in
  let result =
    desugar_pattern_binding map some_pat bound_ir ~mutability:BindingImmutable
      ~cont:(fun () -> rest_ir)
      ty_i64 dummy_span
  in

  match result.expr_kind with
  | Let (_adt_var, _bound, _inner) -> Printf.printf "OK\n"
  | _ -> failwith "Some(x)パターン変換に失敗"

let test_desugar_constructor_pattern_nested () =
  Printf.printf "test_desugar_constructor_pattern_nested ... ";
  (* match opt with | Some(Some(x)) -> x *)
  let x_id = { name = "x"; span = dummy_span } in
  let some_id = { name = "Some"; span = dummy_span } in

  let x_pat =
    {
      tpat_kind = TPatVar x_id;
      tpat_ty = ty_i64;
      tpat_bindings = [ ("x", ty_i64) ];
      tpat_span = dummy_span;
    }
  in
  let inner_some_pat =
    {
      tpat_kind = TPatConstructor (some_id, [ x_pat ]);
      tpat_ty = TApp (TCon (TCUser "Option"), ty_i64);
      tpat_bindings = [ ("x", ty_i64) ];
      tpat_span = dummy_span;
    }
  in
  let outer_some_pat =
    {
      tpat_kind = TPatConstructor (some_id, [ inner_some_pat ]);
      tpat_ty =
        TApp (TCon (TCUser "Option"), TApp (TCon (TCUser "Option"), ty_i64));
      tpat_bindings = [ ("x", ty_i64) ];
      tpat_span = dummy_span;
    }
  in

  let bound = make_typed_expr (TLiteral (Int ("42", Base10))) ty_i64 in
  let rest =
    make_typed_expr
      (TVar (x_id, scheme_to_constrained { quantified = []; body = ty_i64 }))
      ty_i64
  in

  let map = create_scope_map () in
  let bound_ir = desugar_expr map bound in
  let rest_ir = desugar_expr map rest in
  let result =
    desugar_pattern_binding map outer_some_pat bound_ir
      ~mutability:BindingImmutable
      ~cont:(fun () -> rest_ir)
      ty_i64 dummy_span
  in

  match result.expr_kind with
  | Let (_outer_adt_var, _bound, _inner) -> Printf.printf "OK\n"
  | _ -> failwith "Some(Some(x))パターン変換に失敗"

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

  (* ループ補助メタデータ *)
  Printf.printf "\n--- ループ continue メタデータ ---\n";
  test_desugar_loop_continue ();
  test_desugar_for_loop_cfg_equivalence ();

  (* タプルパターン *)
  Printf.printf "\n--- タプルパターン変換 ---\n";
  test_desugar_tuple_pattern_simple ();
  test_desugar_tuple_pattern_nested ();
  test_desugar_tuple_pattern_with_wildcard ();

  (* レコードパターン *)
  Printf.printf "\n--- レコードパターン変換 ---\n";
  test_desugar_record_pattern_basic ();
  test_desugar_record_pattern_with_rest ();
  test_desugar_record_pattern_nested ();

  (* コンストラクタパターン *)
  Printf.printf "\n--- コンストラクタパターン変換 ---\n";
  test_desugar_constructor_pattern_some ();
  test_desugar_constructor_pattern_nested ();

  Printf.printf "\n=== 全テスト成功 ===\n\n"

let () = run_tests ()
