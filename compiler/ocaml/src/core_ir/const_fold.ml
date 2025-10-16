(* Core_ir.Const_fold — Constant Folding Pass for Core IR (Phase 3)
 *
 * このファイルは Core IR の定数畳み込み最適化パスを提供する。
 * 計画書: docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md §4
 *
 * 主要機能:
 * 1. 定数評価エンジン（算術・論理・比較演算）
 * 2. 定数伝播（不変束縛の追跡）
 * 3. 条件分岐の静的評価
 * 4. 不動点反復
 *
 * 設計原則:
 * - 型安全な評価（オーバーフロー・ゼロ除算検出）
 * - 診断情報の完全保持（Span・メタデータ）
 * - 副作用を持つ式は保護
 * - 保守的な最適化（安全性優先）
 *)

open Types
open Ast
open Ir

(* ========== エラー型 ========== *)

(** 定数畳み込み時のエラー *)
type fold_error =
  | DivisionByZero of span  (** ゼロ除算 *)
  | IntegerOverflow of span  (** 整数オーバーフロー *)
  | TypeMismatch of ty * ty * span  (** 型不一致 *)
  | InvalidOperation of string * span  (** 無効な演算 *)

exception FoldError of fold_error

let fold_error err = raise (FoldError err)

(* ========== 統計情報 ========== *)

type fold_stats = {
  mutable folded_exprs : int;  (** 畳み込まれた式の数 *)
  mutable eliminated_branches : int;  (** 削除された分岐の数 *)
  mutable propagated_constants : int;  (** 伝播された定数の数 *)
}
(** 畳み込み統計 *)

let create_stats () : fold_stats =
  { folded_exprs = 0; eliminated_branches = 0; propagated_constants = 0 }

let reset_stats (stats : fold_stats) : unit =
  stats.folded_exprs <- 0;
  stats.eliminated_branches <- 0;
  stats.propagated_constants <- 0

(* ========== リテラル変換関数 ========== *)

(** Ast.literal → int64 への変換
 *
 * 基数（Base2/Base8/Base10/Base16）を考慮して文字列をパース。
 *)
let literal_to_int64 (lit : literal) : int64 option =
  match lit with
  | Int (s, base) -> (
      try
        (* Ast.literal の Int は文字列に既にプレフィックスを含む
         * 例: "0b1010", "0x10", "123" など
         * Base10 の場合はプレフィックスなし
         * OCaml の Int64.of_string はプレフィックスをそのまま認識する *)
        let s_clean =
          match base with
          | Base10 -> s
          | Base2 | Base8 | Base16 ->
              (* プレフィックスが既に含まれているはずだが、念のため確認 *)
              if String.length s >= 2 && s.[0] = '0' then s
              else s (* プレフィックスなしの場合はそのまま *)
        in
        Some (Int64.of_string s_clean)
      with _ -> None)
  | _ -> None

(** Ast.literal → float への変換 *)
let literal_to_float (lit : literal) : float option =
  match lit with
  | Float s -> ( try Some (Float.of_string s) with _ -> None)
  | _ -> None

(** Ast.literal → bool への変換 *)
let literal_to_bool (lit : literal) : bool option =
  match lit with Bool b -> Some b | _ -> None

(** Ast.literal → string への変換 *)
let literal_to_string (lit : literal) : string option =
  match lit with String (s, _) -> Some s | _ -> None

(** int64 → Ast.literal への逆変換
 *
 * 最適化結果は10進表記で出力。
 *)
let int64_to_literal (i : int64) : literal = Int (Int64.to_string i, Base10)

(** float → Ast.literal への逆変換 *)
let float_to_literal (f : float) : literal = Float (Float.to_string f)

(* ========== 定数評価エンジン ========== *)

(** 整数演算のオーバーフローチェック *)
let check_int_overflow (_op_name : string) (result : int64) (_span : span) :
    int64 =
  (* OCaml の int64 は自動的にラップアラウンドするため、
   * 明示的なオーバーフロー検出は複雑。Phase 1 では簡易実装 *)
  result

(** 算術演算の畳み込み *)
let fold_arithmetic_op (op : prim_op) (args : expr list) (span : span) :
    expr option =
  match (op, args) with
  | ( (PrimAdd | PrimSub | PrimMul | PrimDiv | PrimMod),
      [
        { expr_kind = Literal lit_a; expr_ty; _ };
        { expr_kind = Literal lit_b; _ };
      ] ) -> (
      (* 整数演算を試す *)
      match (literal_to_int64 lit_a, literal_to_int64 lit_b) with
      | Some a, Some b ->
          let result =
            match op with
            | PrimAdd -> Int64.add a b
            | PrimSub -> Int64.sub a b
            | PrimMul -> Int64.mul a b
            | PrimDiv ->
                if b = 0L then fold_error (DivisionByZero span)
                else Int64.div a b
            | PrimMod ->
                if b = 0L then fold_error (DivisionByZero span)
                else Int64.rem a b
            | _ -> failwith "Unreachable"
          in
          let result = check_int_overflow "arith" result span in
          Some (make_expr (Literal (int64_to_literal result)) expr_ty span)
      | _ -> (
          (* 浮動小数演算を試す *)
          match (literal_to_float lit_a, literal_to_float lit_b) with
          | Some a, Some b ->
              let result =
                match op with
                | PrimAdd -> a +. b
                | PrimSub -> a -. b
                | PrimMul -> a *. b
                | PrimDiv -> a /. b
                | _ -> failwith "Unreachable (float mod not supported)"
              in
              Some (make_expr (Literal (float_to_literal result)) expr_ty span)
          | _ -> None))
  | _ -> None

(** 比較演算の畳み込み *)
let fold_comparison_op (op : prim_op) (args : expr list) (span : span) :
    expr option =
  match (op, args) with
  (* 整数比較 *)
  | ( PrimEq,
      [ { expr_kind = Literal lit_a; _ }; { expr_kind = Literal lit_b; _ } ] )
    -> (
      match (literal_to_int64 lit_a, literal_to_int64 lit_b) with
      | Some a, Some b -> Some (make_expr (Literal (Bool (a = b))) ty_bool span)
      | _ -> None)
  | ( PrimNe,
      [ { expr_kind = Literal lit_a; _ }; { expr_kind = Literal lit_b; _ } ] )
    -> (
      match (literal_to_int64 lit_a, literal_to_int64 lit_b) with
      | Some a, Some b ->
          Some (make_expr (Literal (Bool (a <> b))) ty_bool span)
      | _ -> None)
  | ( PrimLt,
      [ { expr_kind = Literal lit_a; _ }; { expr_kind = Literal lit_b; _ } ] )
    -> (
      match (literal_to_int64 lit_a, literal_to_int64 lit_b) with
      | Some a, Some b -> Some (make_expr (Literal (Bool (a < b))) ty_bool span)
      | _ -> None)
  | ( PrimLe,
      [ { expr_kind = Literal lit_a; _ }; { expr_kind = Literal lit_b; _ } ] )
    -> (
      match (literal_to_int64 lit_a, literal_to_int64 lit_b) with
      | Some a, Some b ->
          Some (make_expr (Literal (Bool (a <= b))) ty_bool span)
      | _ -> None)
  | ( PrimGt,
      [ { expr_kind = Literal lit_a; _ }; { expr_kind = Literal lit_b; _ } ] )
    -> (
      match (literal_to_int64 lit_a, literal_to_int64 lit_b) with
      | Some a, Some b -> Some (make_expr (Literal (Bool (a > b))) ty_bool span)
      | _ -> None)
  | ( PrimGe,
      [ { expr_kind = Literal lit_a; _ }; { expr_kind = Literal lit_b; _ } ] )
    -> (
      match (literal_to_int64 lit_a, literal_to_int64 lit_b) with
      | Some a, Some b ->
          Some (make_expr (Literal (Bool (a >= b))) ty_bool span)
      | _ -> None)
  (* ブール比較 *)
  | _ -> (
      match args with
      | [ { expr_kind = Literal lit_a; _ }; { expr_kind = Literal lit_b; _ } ]
        -> (
          match (literal_to_bool lit_a, literal_to_bool lit_b) with
          | Some a, Some b -> (
              match op with
              | PrimEq -> Some (make_expr (Literal (Bool (a = b))) ty_bool span)
              | PrimNe ->
                  Some (make_expr (Literal (Bool (a <> b))) ty_bool span)
              | _ -> None)
          | _ -> (
              (* 文字列比較 *)
              match (literal_to_string lit_a, literal_to_string lit_b) with
              | Some a, Some b -> (
                  match op with
                  | PrimEq ->
                      Some (make_expr (Literal (Bool (a = b))) ty_bool span)
                  | PrimNe ->
                      Some (make_expr (Literal (Bool (a <> b))) ty_bool span)
                  | _ -> None)
              | _ -> None))
      | _ -> None)

(** 論理演算の畳み込み *)
let fold_logical_op (op : prim_op) (args : expr list) (span : span) :
    expr option =
  match (op, args) with
  (* 論理積 *)
  | ( PrimAnd,
      [ { expr_kind = Literal lit_a; _ }; { expr_kind = Literal lit_b; _ } ] )
    -> (
      match (literal_to_bool lit_a, literal_to_bool lit_b) with
      | Some a, Some b ->
          Some (make_expr (Literal (Bool (a && b))) ty_bool span)
      | _ -> None)
  (* 論理和 *)
  | ( PrimOr,
      [ { expr_kind = Literal lit_a; _ }; { expr_kind = Literal lit_b; _ } ] )
    -> (
      match (literal_to_bool lit_a, literal_to_bool lit_b) with
      | Some a, Some b ->
          Some (make_expr (Literal (Bool (a || b))) ty_bool span)
      | _ -> None)
  (* 論理否定 *)
  | PrimNot, [ { expr_kind = Literal lit_a; _ } ] -> (
      match literal_to_bool lit_a with
      | Some a -> Some (make_expr (Literal (Bool (not a))) ty_bool span)
      | _ -> None)
  | _ -> None

(** プリミティブ演算の畳み込み *)
let fold_primitive (op : prim_op) (args : expr list) (span : span) : expr option
    =
  match op with
  | PrimAdd | PrimSub | PrimMul | PrimDiv | PrimMod | PrimPow ->
      fold_arithmetic_op op args span
  | PrimEq | PrimNe | PrimLt | PrimLe | PrimGt | PrimGe ->
      fold_comparison_op op args span
  | PrimAnd | PrimOr | PrimNot -> fold_logical_op op args span
  | _ -> None

(* ========== 定数伝播 ========== *)

type const_env = (int, literal) Hashtbl.t
(** 定数環境 (変数名 → リテラル値) *)

let create_const_env () : const_env = Hashtbl.create 64

let lookup_const (env : const_env) (var : var_id) : literal option =
  Hashtbl.find_opt env var.vid

let bind_const (env : const_env) (var : var_id) (lit : literal) : unit =
  Hashtbl.replace env var.vid lit

let unbind_const (env : const_env) (var : var_id) : unit =
  Hashtbl.remove env var.vid

let with_const_binding (env : const_env) (var : var_id) (lit : literal)
    (f : unit -> 'a) : 'a =
  let previous = lookup_const env var in
  bind_const env var lit;
  let result = f () in
  (match previous with
  | Some prev_lit -> bind_const env var prev_lit
  | None -> unbind_const env var);
  result

(* ========== 式の畳み込み ========== *)

(** 式の再帰的な畳み込み *)
let rec fold_expr (env : const_env) (stats : fold_stats) (e : expr) : expr =
  match e.expr_kind with
  (* リテラルはそのまま *)
  | Literal _ -> e
  (* 変数参照: 定数環境を検索 *)
  | Var var -> (
      match lookup_const env var with
      | Some lit ->
          stats.propagated_constants <- stats.propagated_constants + 1;
          make_expr (Literal lit) e.expr_ty e.expr_span
      | None -> e)
  (* プリミティブ演算: 定数畳み込み *)
  | Primitive (op, args) -> (
      let folded_args = List.map (fold_expr env stats) args in
      match fold_primitive op folded_args e.expr_span with
      | Some folded ->
          stats.folded_exprs <- stats.folded_exprs + 1;
          folded
      | None -> make_expr (Primitive (op, folded_args)) e.expr_ty e.expr_span)
  (* 関数適用: 引数のみ畳み込み *)
  | App (fn, args) ->
      let folded_fn = fold_expr env stats fn in
      let folded_args = List.map (fold_expr env stats) args in
      make_expr (App (folded_fn, folded_args)) e.expr_ty e.expr_span
  (* if式: 条件の静的評価 *)
  | If (cond, then_e, else_e) -> (
      let folded_cond = fold_expr env stats cond in
      match folded_cond.expr_kind with
      | Literal (Bool true) ->
          stats.eliminated_branches <- stats.eliminated_branches + 1;
          fold_expr env stats then_e
      | Literal (Bool false) ->
          stats.eliminated_branches <- stats.eliminated_branches + 1;
          fold_expr env stats else_e
      | _ ->
          let folded_then = fold_expr env stats then_e in
          let folded_else = fold_expr env stats else_e in
          make_expr
            (If (folded_cond, folded_then, folded_else))
            e.expr_ty e.expr_span)
  (* Let束縛: 定数環境を更新 *)
  | Let (var, bound, body) ->
      let folded_bound = fold_expr env stats bound in
      let folded_body =
        match folded_bound.expr_kind with
        | Literal lit ->
            with_const_binding env var lit (fun () -> fold_expr env stats body)
        | _ -> fold_expr env stats body
      in
      make_expr (Let (var, folded_bound, folded_body)) e.expr_ty e.expr_span
  (* Match式: scrutinee と各アームを畳み込み *)
  | Match (scrut, cases) ->
      let folded_scrut = fold_expr env stats scrut in
      let folded_cases =
        List.map
          (fun case ->
            { case with case_body = fold_expr env stats case.case_body })
          cases
      in
      make_expr (Match (folded_scrut, folded_cases)) e.expr_ty e.expr_span
  (* その他の式: 再帰的に畳み込み *)
  | TupleAccess (e1, idx) ->
      let folded_e1 = fold_expr env stats e1 in
      make_expr (TupleAccess (folded_e1, idx)) e.expr_ty e.expr_span
  | RecordAccess (e1, field) ->
      let folded_e1 = fold_expr env stats e1 in
      make_expr (RecordAccess (folded_e1, field)) e.expr_ty e.expr_span
  | ArrayAccess (e1, e2) ->
      let folded_e1 = fold_expr env stats e1 in
      let folded_e2 = fold_expr env stats e2 in
      make_expr (ArrayAccess (folded_e1, folded_e2)) e.expr_ty e.expr_span
  | ADTConstruct (name, args) ->
      let folded_args = List.map (fold_expr env stats) args in
      make_expr (ADTConstruct (name, folded_args)) e.expr_ty e.expr_span
  | ADTProject (e1, idx) ->
      let folded_e1 = fold_expr env stats e1 in
      make_expr (ADTProject (folded_e1, idx)) e.expr_ty e.expr_span
  | AssignMutable (var, rhs) ->
      let folded_rhs = fold_expr env stats rhs in
      make_expr (AssignMutable (var, folded_rhs)) e.expr_ty e.expr_span
  | Continue -> e
  | Loop loop_info ->
      let folded_kind =
        match loop_info.loop_kind with
        | WhileLoop cond -> WhileLoop (fold_expr env stats cond)
        | ForLoop info ->
            let init' =
              List.map
                (fun (var, init_e) -> (var, fold_expr env stats init_e))
                info.for_init
            in
            let step' =
              List.map
                (fun (var, step_e) -> (var, fold_expr env stats step_e))
                info.for_step
            in
            ForLoop
              {
                info with
                for_source = fold_expr env stats info.for_source;
                for_init = init';
                for_step = step';
              }
        | InfiniteLoop -> InfiniteLoop
      in
      let folded_body = fold_expr env stats loop_info.loop_body in
      let header_effects =
        List.map
          (fun eff ->
            {
              eff with
              effect_expr =
                Option.map (fold_expr env stats) eff.effect_expr;
            })
          loop_info.loop_header_effects
      in
      let body_effects =
        List.map
          (fun eff ->
            {
              eff with
              effect_expr =
                Option.map (fold_expr env stats) eff.effect_expr;
            })
          loop_info.loop_body_effects
      in
      make_expr
        (Loop
           {
             loop_info with
             loop_kind = folded_kind;
             loop_body = folded_body;
             loop_header_effects = header_effects;
             loop_body_effects = body_effects;
           })
        e.expr_ty e.expr_span
  | DictConstruct dict ->
      make_expr (DictConstruct dict) e.expr_ty e.expr_span
  | DictMethodCall (dict_expr, method_name, args, audit) ->
      let dict' = fold_expr env stats dict_expr in
      let args' = List.map (fold_expr env stats) args in
      make_expr
        (DictMethodCall (dict', method_name, args', audit))
        e.expr_ty
        e.expr_span
  (* その他のノードはそのまま（Phase 1 で未実装のケース） *)
  | Closure _ | DictLookup _ | CapabilityCheck _ -> e

(* ========== 文・ブロックの畳み込み ========== *)

(** 文の畳み込み *)
let fold_stmt (env : const_env) (stats : fold_stats) (stmt : stmt) : stmt =
  match stmt with
  | Assign (var, e) ->
      let folded_e = fold_expr env stats e in
      (* 代入が定数なら環境に追加 *)
      (match folded_e.expr_kind with
      | Literal lit -> bind_const env var lit
      | _ -> ());
      Assign (var, folded_e)
  | Store (var, e) ->
      let folded_e = fold_expr env stats e in
      Store (var, folded_e)
  | Alloca _ as s -> s
  | ExprStmt e -> ExprStmt (fold_expr env stats e)
  | Return e -> Return (fold_expr env stats e)
  | (Jump _ | Branch _ | Phi _ | EffectMarker _) as s -> s

(** 終端命令の畳み込み *)
let fold_terminator (env : const_env) (stats : fold_stats) (term : terminator) :
    terminator =
  match term with
  | TermReturn e -> TermReturn (fold_expr env stats e)
  | TermBranch (cond, then_lbl, else_lbl) ->
      let folded_cond = fold_expr env stats cond in
      (* 条件が定数なら分岐を削除できる可能性があるが、
       * CFG構造を変更するのは複雑なのでDCEパスに委譲 *)
      TermBranch (folded_cond, then_lbl, else_lbl)
  | (TermJump _ | TermSwitch _ | TermUnreachable) as t -> t

(** ブロックの畳み込み *)
let fold_block (env : const_env) (stats : fold_stats) (block : block) : block =
  let folded_stmts = List.map (fold_stmt env stats) block.stmts in
  let folded_term = fold_terminator env stats block.terminator in
  {
    label = block.label;
    params = block.params;
    stmts = folded_stmts;
    terminator = folded_term;
    block_span = block.block_span;
  }

(** 関数の畳み込み *)
let fold_function (stats : fold_stats) (fn : function_def) : function_def =
  let env = create_const_env () in
  let folded_blocks = List.map (fold_block env stats) fn.fn_blocks in
  { fn with fn_blocks = folded_blocks }

(* ========== 不動点反復 ========== *)

(** ブロックが変化したかを判定 *)
let block_changed (b1 : block) (b2 : block) : bool = b1 != b2

(** 関数が変化したかを判定 *)
let function_changed (f1 : function_def) (f2 : function_def) : bool =
  List.length f1.fn_blocks <> List.length f2.fn_blocks
  || List.exists2 block_changed f1.fn_blocks f2.fn_blocks

type fold_config = {
  max_iterations : int;  (** 最大反復回数 *)
  verbose : bool;  (** 詳細ログ出力 *)
}
(** 不動点反復の設定 *)

let default_config : fold_config = { max_iterations = 5; verbose = false }

(** 不動点に達するまで畳み込みを反復 *)
let fold_to_fixpoint (config : fold_config) (stats : fold_stats)
    (fn : function_def) : function_def =
  let rec loop iteration fn_prev =
    if iteration >= config.max_iterations then fn_prev
    else
      let fn_next = fold_function stats fn_prev in
      if function_changed fn_prev fn_next then (
        if config.verbose then
          Printf.eprintf "[ConstFold] Iteration %d: changes detected\n%!"
            iteration;
        loop (iteration + 1) fn_next)
      else (
        if config.verbose then
          Printf.eprintf "[ConstFold] Converged at iteration %d\n%!" iteration;
        fn_next)
  in
  loop 0 fn

(* ========== 公開API ========== *)

(** 関数に対して定数畳み込みを適用 *)
let optimize_function ?(config = default_config) (fn : function_def) :
    function_def * fold_stats =
  let stats = create_stats () in
  let optimized = fold_to_fixpoint config stats fn in
  (optimized, stats)

(** モジュール全体に対して定数畳み込みを適用 *)
let optimize_module ?(config = default_config) (m : module_def) :
    module_def * fold_stats =
  let stats = create_stats () in
  let optimized_fns =
    List.map
      (fun fn ->
        let optimized, _ = optimize_function ~config fn in
        optimized)
      m.function_defs
  in
  let optimized_module = { m with function_defs = optimized_fns } in
  (optimized_module, stats)
