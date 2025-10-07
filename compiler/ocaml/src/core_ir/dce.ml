(* Core_ir.Dce — Dead Code Elimination Pass for Core IR (Phase 3)
 *
 * このファイルは Core IR の死コード削除（DCE）最適化パスを提供する。
 * 計画書: docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md §5
 *
 * 主要機能:
 * 1. 生存解析（変数定義・使用の追跡）
 * 2. 未使用束縛の削除
 * 3. 到達不能ブロックの除去
 * 4. 副作用を持つ式の保護
 *
 * 設計原則:
 * - 保守的な最適化（疑わしい場合は削除しない）
 * - 副作用を持つ式は常に保護
 * - 診断情報の完全保持（Span・メタデータ）
 * - メタデータタグ（preserve_for_diagnostics）に従う
 *)

open Ir

(* ========== 統計情報 ========== *)

(** DCE 統計 *)
type dce_stats = {
  mutable removed_bindings: int;     (** 削除された束縛の数 *)
  mutable removed_blocks: int;       (** 削除されたブロックの数 *)
  mutable removed_stmts: int;        (** 削除された文の数 *)
}

let create_stats () : dce_stats = {
  removed_bindings = 0;
  removed_blocks = 0;
  removed_stmts = 0;
}

let reset_stats (stats: dce_stats) : unit =
  stats.removed_bindings <- 0;
  stats.removed_blocks <- 0;
  stats.removed_stmts <- 0

(* ========== 生存解析 ========== *)

(** 変数使用集合 *)
module VarSet = Set.Make(struct
  type t = var_id
  let compare v1 v2 = Int.compare v1.vid v2.vid
end)

(** 式中で使用される変数を収集 *)
let rec collect_used_vars (e: expr) : VarSet.t =
  match e.expr_kind with
  | Literal _ -> VarSet.empty
  | Var var -> VarSet.singleton var
  | Primitive (_, args) ->
      List.fold_left (fun acc arg ->
        VarSet.union acc (collect_used_vars arg)
      ) VarSet.empty args
  | App (fn, args) ->
      let fn_vars = collect_used_vars fn in
      List.fold_left (fun acc arg ->
        VarSet.union acc (collect_used_vars arg)
      ) fn_vars args
  | If (cond, then_e, else_e) ->
      VarSet.union (collect_used_vars cond)
        (VarSet.union (collect_used_vars then_e) (collect_used_vars else_e))
  | Let (_, bound, body) ->
      VarSet.union (collect_used_vars bound) (collect_used_vars body)
  | Match (scrut, cases) ->
      let scrut_vars = collect_used_vars scrut in
      List.fold_left (fun acc case ->
        VarSet.union acc (collect_used_vars case.case_body)
      ) scrut_vars cases
  | TupleAccess (e1, _) -> collect_used_vars e1
  | RecordAccess (e1, _) -> collect_used_vars e1
  | ArrayAccess (e1, e2) ->
      VarSet.union (collect_used_vars e1) (collect_used_vars e2)
  | ADTConstruct (_, args) ->
      List.fold_left (fun acc arg ->
        VarSet.union acc (collect_used_vars arg)
      ) VarSet.empty args
  | ADTProject (e1, _) -> collect_used_vars e1
  | Closure _ | DictLookup _ | CapabilityCheck _ ->
      (* Phase 1 では簡易実装 *)
      VarSet.empty

(** 文中で使用される変数を収集 *)
let collect_stmt_used_vars (stmt: stmt) : VarSet.t =
  match stmt with
  | Assign (_, e) -> collect_used_vars e
  | ExprStmt e -> collect_used_vars e
  | Return e -> collect_used_vars e
  | Jump _ | Branch _ | Phi _ | EffectMarker _ -> VarSet.empty

(** 終端命令中で使用される変数を収集 *)
let collect_term_used_vars (term: terminator) : VarSet.t =
  match term with
  | TermReturn e -> collect_used_vars e
  | TermBranch (cond, _, _) -> collect_used_vars cond
  | TermJump _ | TermSwitch _ | TermUnreachable -> VarSet.empty

(** ブロック中で使用される変数を収集 *)
let collect_block_used_vars (block: block) : VarSet.t =
  let stmt_vars = List.fold_left (fun acc stmt ->
    VarSet.union acc (collect_stmt_used_vars stmt)
  ) VarSet.empty block.stmts in
  VarSet.union stmt_vars (collect_term_used_vars block.terminator)

(** 関数中で使用される変数を収集 *)
let collect_function_used_vars (fn: function_def) : VarSet.t =
  List.fold_left (fun acc block ->
    VarSet.union acc (collect_block_used_vars block)
  ) VarSet.empty fn.fn_blocks

(* ========== 副作用チェック ========== *)

(** 式が副作用を持つかを判定
 *
 * Phase 1 では保守的に実装:
 * - 関数呼び出しは副作用を持つと仮定
 * - プリミティブ演算は副作用を持たないと仮定
 *)
let rec has_side_effect (e: expr) : bool =
  match e.expr_kind with
  | Literal _ | Var _ -> false
  | Primitive _ -> false  (* プリミティブ演算は純粋と仮定 *)
  | App _ -> true         (* 関数呼び出しは副作用を持つと仮定 *)
  | If (cond, then_e, else_e) ->
      has_side_effect cond || has_side_effect then_e || has_side_effect else_e
  | Let (_, bound, body) ->
      has_side_effect bound || has_side_effect body
  | Match (scrut, cases) ->
      has_side_effect scrut || List.exists (fun case -> has_side_effect case.case_body) cases
  | TupleAccess (e1, _) | RecordAccess (e1, _) | ADTProject (e1, _) ->
      has_side_effect e1
  | ArrayAccess (e1, e2) ->
      has_side_effect e1 || has_side_effect e2
  | ADTConstruct (_, args) ->
      List.exists has_side_effect args
  | Closure _ | DictLookup _ | CapabilityCheck _ ->
      (* 保守的に副作用ありと判定 *)
      true

(** 文が副作用を持つかを判定 *)
let _has_stmt_side_effect (stmt: stmt) : bool =
  match stmt with
  | Assign (_, e) -> has_side_effect e
  | ExprStmt e -> has_side_effect e
  | Return e -> has_side_effect e
  | Jump _ | Branch _ | Phi _ | EffectMarker _ ->
      (* 制御フロー命令は副作用を持つと判定 *)
      true

(* ========== 式の最適化 ========== *)

(** Let 束縛の DCE *)
let rec dce_expr (used_vars: VarSet.t) (stats: dce_stats) (e: expr) : expr =
  match e.expr_kind with
  | Literal _ | Var _ | Primitive _ | Closure _ | DictLookup _ | CapabilityCheck _ ->
      e

  | App (fn, args) ->
      let fn' = dce_expr used_vars stats fn in
      let args' = List.map (dce_expr used_vars stats) args in
      make_expr (App (fn', args')) e.expr_ty e.expr_span

  | If (cond, then_e, else_e) ->
      let cond' = dce_expr used_vars stats cond in
      let then_e' = dce_expr used_vars stats then_e in
      let else_e' = dce_expr used_vars stats else_e in
      make_expr (If (cond', then_e', else_e')) e.expr_ty e.expr_span

  | Let (var, bound, body) ->
      let bound' = dce_expr used_vars stats bound in
      let body' = dce_expr used_vars stats body in
      (* 変数が使用されていない、かつ副作用がない場合は束縛を削除 *)
      if not (VarSet.mem var used_vars) && not (has_side_effect bound') then begin
        stats.removed_bindings <- stats.removed_bindings + 1;
        body'
      end else
        make_expr (Let (var, bound', body')) e.expr_ty e.expr_span

  | Match (scrut, cases) ->
      let scrut' = dce_expr used_vars stats scrut in
      let cases' = List.map (fun case ->
        { case with case_body = dce_expr used_vars stats case.case_body }
      ) cases in
      make_expr (Match (scrut', cases')) e.expr_ty e.expr_span

  | TupleAccess (e1, idx) ->
      let e1' = dce_expr used_vars stats e1 in
      make_expr (TupleAccess (e1', idx)) e.expr_ty e.expr_span

  | RecordAccess (e1, field) ->
      let e1' = dce_expr used_vars stats e1 in
      make_expr (RecordAccess (e1', field)) e.expr_ty e.expr_span

  | ArrayAccess (e1, e2) ->
      let e1' = dce_expr used_vars stats e1 in
      let e2' = dce_expr used_vars stats e2 in
      make_expr (ArrayAccess (e1', e2')) e.expr_ty e.expr_span

  | ADTConstruct (name, args) ->
      let args' = List.map (dce_expr used_vars stats) args in
      make_expr (ADTConstruct (name, args')) e.expr_ty e.expr_span

  | ADTProject (e1, idx) ->
      let e1' = dce_expr used_vars stats e1 in
      make_expr (ADTProject (e1', idx)) e.expr_ty e.expr_span

(* ========== 文・ブロックの最適化 ========== *)

(** 文の DCE *)
let dce_stmt (used_vars: VarSet.t) (stats: dce_stats) (stmt: stmt) : stmt option =
  match stmt with
  | Assign (var, e) ->
      let e' = dce_expr used_vars stats e in
      (* 変数が使用されていない、かつ副作用がない場合は代入を削除 *)
      if not (VarSet.mem var used_vars) && not (has_side_effect e') then begin
        stats.removed_stmts <- stats.removed_stmts + 1;
        None
      end else
        Some (Assign (var, e'))

  | ExprStmt e ->
      let e' = dce_expr used_vars stats e in
      (* 副作用がない式文は削除 *)
      if not (has_side_effect e') then begin
        stats.removed_stmts <- stats.removed_stmts + 1;
        None
      end else
        Some (ExprStmt e')

  | Return e ->
      Some (Return (dce_expr used_vars stats e))

  | Jump _ | Branch _ | Phi _ | EffectMarker _ as s ->
      Some s

(** 終端命令の DCE *)
let dce_terminator (used_vars: VarSet.t) (stats: dce_stats) (term: terminator) : terminator =
  match term with
  | TermReturn e -> TermReturn (dce_expr used_vars stats e)
  | TermBranch (cond, then_lbl, else_lbl) ->
      TermBranch (dce_expr used_vars stats cond, then_lbl, else_lbl)
  | TermJump _ | TermSwitch _ | TermUnreachable as t -> t

(** ブロックの DCE *)
let dce_block (used_vars: VarSet.t) (stats: dce_stats) (block: block) : block =
  let stmts' = List.filter_map (dce_stmt used_vars stats) block.stmts in
  let term' = dce_terminator used_vars stats block.terminator in
  {
    label = block.label;
    params = block.params;
    stmts = stmts';
    terminator = term';
    block_span = block.block_span;
  }

(* ========== 到達不能ブロックの削除 ========== *)

(** 到達可能なブロックラベルを収集 *)
let collect_reachable_labels (entry_label: label) (blocks: block list) : label list =
  let reachable = Hashtbl.create 32 in
  let block_map = Hashtbl.create 32 in
  List.iter (fun block -> Hashtbl.add block_map block.label block) blocks;

  let rec visit label =
    if not (Hashtbl.mem reachable label) then begin
      Hashtbl.add reachable label ();
      match Hashtbl.find_opt block_map label with
      | Some block ->
          begin match block.terminator with
          | TermJump target -> visit target
          | TermBranch (_, then_lbl, else_lbl) ->
              visit then_lbl;
              visit else_lbl
          | TermSwitch (_, cases, default) ->
              List.iter (fun (_, lbl) -> visit lbl) cases;
              visit default
          | TermReturn _ | TermUnreachable -> ()
          end
      | None -> ()
    end
  in
  visit entry_label;
  Hashtbl.fold (fun lbl () acc -> lbl :: acc) reachable []

(** 到達不能ブロックを削除 *)
let remove_unreachable_blocks (stats: dce_stats) (blocks: block list) : block list =
  if List.length blocks = 0 then
    blocks
  else
    let entry_label = (List.hd blocks).label in
    let reachable_labels = collect_reachable_labels entry_label blocks in
    let reachable_set = List.fold_left (fun acc lbl ->
      Hashtbl.add acc lbl ();
      acc
    ) (Hashtbl.create 32) reachable_labels in
    let reachable_blocks = List.filter (fun block ->
      Hashtbl.mem reachable_set block.label
    ) blocks in
    let removed_count = List.length blocks - List.length reachable_blocks in
    stats.removed_blocks <- stats.removed_blocks + removed_count;
    reachable_blocks

(* ========== 関数・モジュールの最適化 ========== *)

(** 関数に対して DCE を適用 *)
let optimize_function (fn: function_def) : function_def * dce_stats =
  let stats = create_stats () in

  (* 診断保護フラグをチェック *)
  if fn.fn_metadata.opt_flags.preserve_for_diagnostics then
    (fn, stats)
  else begin
    (* 1. 使用される変数を収集 *)
    let used_vars = collect_function_used_vars fn in

    (* 2. 各ブロックに DCE を適用 *)
    let optimized_blocks = List.map (dce_block used_vars stats) fn.fn_blocks in

    (* 3. 到達不能ブロックを削除 *)
    let final_blocks = remove_unreachable_blocks stats optimized_blocks in

    ({ fn with fn_blocks = final_blocks }, stats)
  end

(** モジュール全体に対して DCE を適用 *)
let optimize_module (m: module_def) : module_def * dce_stats =
  let stats = create_stats () in
  let optimized_fns = List.map (fun fn ->
    let optimized, fn_stats = optimize_function fn in
    stats.removed_bindings <- stats.removed_bindings + fn_stats.removed_bindings;
    stats.removed_blocks <- stats.removed_blocks + fn_stats.removed_blocks;
    stats.removed_stmts <- stats.removed_stmts + fn_stats.removed_stmts;
    optimized
  ) m.function_defs in
  ({ m with function_defs = optimized_fns }, stats)
