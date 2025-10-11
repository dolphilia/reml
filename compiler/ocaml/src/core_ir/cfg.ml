(* Core_ir.Cfg — Control Flow Graph Construction (Phase 3)
 *
 * このファイルは Core IR 式から CFG (制御フローグラフ) を構築する。
 * 計画書: docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md §3
 *
 * 主要機能:
 * 1. 制御フロー分岐点の検出
 * 2. 基本ブロックの生成
 * 3. ラベル自動生成とリンク
 * 4. CFG整形性の検証
 *
 * 設計原則:
 * - 式ツリーを線形命令列へ平坦化
 * - 分岐/合流点でブロック分割
 * - SSA形式への変換を容易にする構造
 *)

open Types
open Ir

(* ========== 型エイリアス ========== *)

type span = Ast.span
type literal = Ast.literal

(* ========== 制御フロー分岐点 ========== *)

(** 分岐点の種別 *)
type split_kind =
  | SplitIfBranch of label * label  (** if式の分岐 (then_label, else_label) *)
  | SplitMatchCase of (literal * label) list * label
      (** match式のcase (cases, default) *)
  | SplitLoop of label  (** ループ開始点 *)
  | SplitMerge  (** 合流点 *)
  | SplitReturn  (** 関数リターン *)

type split_point = {
  split_label : label;  (** このポイントのラベル *)
  split_kind : split_kind;  (** 分岐種別 *)
  split_span : span;  (** ソースコード位置 *)
}
(** 制御フロー分岐点
 *
 * 式ツリーを走査して検出された分岐/合流点。
 *)

(* ========== 線形化された命令列 ========== *)

(** 線形命令
 *
 * 式ツリーを平坦化した命令列。
 * 基本ブロック生成の中間表現。
 *)
type linear_instr =
  | LStmt of stmt  (** 通常の文 *)
  | LSplitPoint of split_point  (** 分岐点マーカー *)
  | LLabel of label  (** ラベル定義 *)

(* ========== CFG構築コンテキスト ========== *)

type cfg_builder = {
  mutable current_label : label option;  (** 現在のブロックラベル *)
  mutable current_stmts : stmt list;  (** 現在のブロックの命令列 *)
  mutable blocks : block list;  (** 完成したブロック *)
  mutable split_points : split_point list;  (** 検出された分岐点 *)
}
(** CFG構築状態 *)

(** CFG構築状態の初期化 *)
let create_cfg_builder () : cfg_builder =
  { current_label = None; current_stmts = []; blocks = []; split_points = [] }

(** 新しいブロックを開始 *)
let start_block (builder : cfg_builder) (lbl : label) : unit =
  builder.current_label <- Some lbl;
  builder.current_stmts <- []

(** 現在のブロックに命令を追加 *)
let add_stmt (builder : cfg_builder) (stmt : stmt) : unit =
  builder.current_stmts <- builder.current_stmts @ [ stmt ]

(** 現在のブロックを終端命令で閉じる *)
let finish_block (builder : cfg_builder) (term : terminator) (span : span) :
    unit =
  match builder.current_label with
  | None ->
      (* ブロックが開始されていない（エラー） *)
      failwith "finish_block called without start_block"
  | Some lbl ->
      let blk = make_block lbl [] (List.rev builder.current_stmts) term span in
      builder.blocks <- builder.blocks @ [ blk ];
      builder.current_label <- None;
      builder.current_stmts <- []

(* ========== 式の線形化 ========== *)

(** 式を線形命令列に変換
 *
 * 式ツリーを深さ優先探索し、制御フロー分岐点を検出しながら
 * 線形命令列を生成する。
 *)
let rec linearize_expr (builder : cfg_builder) (expr : expr) : var_id =
  match expr.expr_kind with
  | Literal lit ->
      (* リテラル → 一時変数に代入 *)
      let temp = VarIdGen.fresh "$lit" expr.expr_ty expr.expr_span in
      let lit_expr = make_expr (Literal lit) expr.expr_ty expr.expr_span in
      add_stmt builder (Assign (temp, lit_expr));
      temp
  | Var var ->
      (* 変数参照 → そのまま返す *)
      var
  | Primitive (op, args) ->
      (* プリミティブ演算 → 引数を先に評価 *)
      let arg_vars = List.map (linearize_expr builder) args in
      let arg_exprs =
        List.map (fun v -> make_expr (Var v) v.vty v.vspan) arg_vars
      in
      let prim_expr =
        make_expr (Primitive (op, arg_exprs)) expr.expr_ty expr.expr_span
      in
      let result = VarIdGen.fresh "$prim" expr.expr_ty expr.expr_span in
      add_stmt builder (Assign (result, prim_expr));
      result
  | Let (var, bound, body) ->
      (* let束縛 → 束縛式を評価してから本体へ *)
      let bound_var = linearize_expr builder bound in
      let bound_ref = make_expr (Var bound_var) bound.expr_ty bound.expr_span in
      add_stmt builder (Assign (var, bound_ref));
      linearize_expr builder body
  | If (cond, then_e, else_e) ->
      linearize_if builder cond then_e else_e expr.expr_ty expr.expr_span
  | Match (scrut, cases) ->
      linearize_match builder scrut cases expr.expr_ty expr.expr_span
  | App (fn, args) ->
      (* 関数適用 → 引数評価 + 呼出 *)
      let fn_var = linearize_expr builder fn in
      let arg_vars = List.map (linearize_expr builder) args in
      let arg_exprs =
        List.map (fun v -> make_expr (Var v) v.vty v.vspan) arg_vars
      in
      let app_expr =
        make_expr
          (App (make_expr (Var fn_var) fn_var.vty fn_var.vspan, arg_exprs))
          expr.expr_ty expr.expr_span
      in
      let result = VarIdGen.fresh "$app" expr.expr_ty expr.expr_span in
      add_stmt builder (Assign (result, app_expr));
      result
  | TupleAccess (tuple, idx) ->
      (* タプル要素アクセス *)
      let tuple_var = linearize_expr builder tuple in
      let access_expr =
        make_expr
          (TupleAccess
             (make_expr (Var tuple_var) tuple_var.vty tuple_var.vspan, idx))
          expr.expr_ty expr.expr_span
      in
      let result = VarIdGen.fresh "$tuple_elem" expr.expr_ty expr.expr_span in
      add_stmt builder (Assign (result, access_expr));
      result
  | RecordAccess (record, field) ->
      (* レコードフィールドアクセス *)
      let record_var = linearize_expr builder record in
      let access_expr =
        make_expr
          (RecordAccess
             (make_expr (Var record_var) record_var.vty record_var.vspan, field))
          expr.expr_ty expr.expr_span
      in
      let result = VarIdGen.fresh "$field" expr.expr_ty expr.expr_span in
      add_stmt builder (Assign (result, access_expr));
      result
  | ArrayAccess (arr, idx) ->
      (* 配列インデックスアクセス *)
      let arr_var = linearize_expr builder arr in
      let idx_var = linearize_expr builder idx in
      let access_expr =
        make_expr
          (ArrayAccess
             ( make_expr (Var arr_var) arr_var.vty arr_var.vspan,
               make_expr (Var idx_var) idx_var.vty idx_var.vspan ))
          expr.expr_ty expr.expr_span
      in
      let result = VarIdGen.fresh "$arr_elem" expr.expr_ty expr.expr_span in
      add_stmt builder (Assign (result, access_expr));
      result
  | ADTConstruct (ctor, fields) ->
      (* ADT コンストラクタ *)
      let field_vars = List.map (linearize_expr builder) fields in
      let field_exprs =
        List.map (fun v -> make_expr (Var v) v.vty v.vspan) field_vars
      in
      let adt_expr =
        make_expr (ADTConstruct (ctor, field_exprs)) expr.expr_ty expr.expr_span
      in
      let result = VarIdGen.fresh "$adt" expr.expr_ty expr.expr_span in
      add_stmt builder (Assign (result, adt_expr));
      result
  | ADTProject (adt, field_idx) ->
      (* ADT フィールド射影 *)
      let adt_var = linearize_expr builder adt in
      let project_expr =
        make_expr
          (ADTProject
             (make_expr (Var adt_var) adt_var.vty adt_var.vspan, field_idx))
          expr.expr_ty expr.expr_span
      in
      let result = VarIdGen.fresh "$adt_field" expr.expr_ty expr.expr_span in
      add_stmt builder (Assign (result, project_expr));
      result
  | Closure _closure_info ->
      (* クロージャ生成 → 後のフェーズで実装 *)
      let result = VarIdGen.fresh "$closure" expr.expr_ty expr.expr_span in
      add_stmt builder (Assign (result, expr));
      result
  | DictLookup _dict_ref ->
      (* 辞書参照 → 後のフェーズで実装 *)
      let result = VarIdGen.fresh "$dict" expr.expr_ty expr.expr_span in
      add_stmt builder (Assign (result, expr));
      result
  | DictConstruct dict ->
      let result =
        VarIdGen.fresh "$dict_init" expr.expr_ty expr.expr_span
      in
      let dict_expr =
        make_expr (DictConstruct dict) expr.expr_ty expr.expr_span
      in
      add_stmt builder (Assign (result, dict_expr));
      result
  | DictMethodCall (dict_expr, method_name, args) ->
      let dict_var = linearize_expr builder dict_expr in
      let arg_vars = List.map (linearize_expr builder) args in
      let dict_ref =
        make_expr (Var dict_var) dict_var.vty dict_var.vspan
      in
      let arg_exprs =
        List.map (fun v -> make_expr (Var v) v.vty v.vspan) arg_vars
      in
      let call_expr =
        make_expr
          (DictMethodCall (dict_ref, method_name, arg_exprs))
          expr.expr_ty expr.expr_span
      in
      let result =
        VarIdGen.fresh "$dict_call" expr.expr_ty expr.expr_span
      in
      add_stmt builder (Assign (result, call_expr));
      result
  | CapabilityCheck _cap_id ->
      (* Capability チェック → 後のフェーズで実装 *)
      let result = VarIdGen.fresh "$cap" expr.expr_ty expr.expr_span in
      add_stmt builder (Assign (result, expr));
      result

(** if式の線形化 *)
and linearize_if (builder : cfg_builder) (cond : expr) (then_e : expr)
    (else_e : expr) (result_ty : ty) (span : span) : var_id =
  (* 条件式を評価 *)
  let cond_var = linearize_expr builder cond in

  (* ラベルを生成 *)
  let then_label = LabelGen.fresh "if_then" in
  let else_label = LabelGen.fresh "if_else" in
  let merge_label = LabelGen.fresh "if_merge" in

  (* 現在のブロックを分岐で終了 *)
  finish_block builder
    (TermBranch
       ( make_expr (Var cond_var) cond_var.vty cond_var.vspan,
         then_label,
         else_label ))
    span;

  (* then ブロック *)
  start_block builder then_label;
  let then_var = linearize_expr builder then_e in
  finish_block builder (TermJump merge_label) then_e.expr_span;

  (* else ブロック *)
  start_block builder else_label;
  let else_var = linearize_expr builder else_e in
  finish_block builder (TermJump merge_label) else_e.expr_span;

  (* merge ブロック *)
  start_block builder merge_label;
  let result_var = VarIdGen.fresh "$if_result" result_ty span in
  (* φノードを挿入 *)
  add_stmt builder
    (Phi (result_var, [ (then_label, then_var); (else_label, else_var) ]));
  result_var

(** match式の線形化 *)
and linearize_match (builder : cfg_builder) (scrut : expr) (cases : case list)
    (result_ty : ty) (span : span) : var_id =
  (* scrutinee を評価 *)
  let scrut_var = linearize_expr builder scrut in

  (* ラベルを生成 *)
  let case_labels = List.map (fun _ -> LabelGen.fresh "match_case") cases in
  let merge_label = LabelGen.fresh "match_merge" in
  let fail_label = LabelGen.fresh "match_fail" in

  (* switch 終端命令を構築 *)
  let switch_cases =
    List.map2
      (fun case lbl ->
        match case.case_pattern with
        | PLiteral lit -> (lit, lbl)
        | _ -> (Ast.Unit, lbl)
        (* デフォルトケース *))
      cases case_labels
  in

  finish_block builder
    (TermSwitch
       ( make_expr (Var scrut_var) scrut_var.vty scrut_var.vspan,
         switch_cases,
         fail_label ))
    span;

  (* 各 case ブロック *)
  let case_result_vars =
    List.map2
      (fun case lbl ->
        start_block builder lbl;
        let body_var = linearize_expr builder case.case_body in
        finish_block builder (TermJump merge_label) case.case_span;
        (lbl, body_var))
      cases case_labels
  in

  (* fail ブロック (match失敗) *)
  start_block builder fail_label;
  finish_block builder TermUnreachable span;

  (* merge ブロック *)
  start_block builder merge_label;
  let result_var = VarIdGen.fresh "$match_result" result_ty span in
  add_stmt builder (Phi (result_var, case_result_vars));
  result_var

(* ========== CFG構築エントリポイント ========== *)

(** 関数定義からCFGを構築
 *
 * @param fn 関数定義
 * @return 基本ブロックリスト
 *)
let build_cfg (fn : function_def) : block list =
  VarIdGen.reset ();
  LabelGen.reset ();

  let builder = create_cfg_builder () in
  let entry_label = LabelGen.fresh "entry" in

  (* エントリブロックを開始 *)
  start_block builder entry_label;

  (* TODO: 関数本体の線形化
   * 現在の関数定義は fn_blocks を持っているが、
   * まだ高レベルの式表現から変換されていない。
   * ここでは仮実装として空のブロックリストを返す。
   *)

  (* エントリブロックを終了 *)
  finish_block builder
    (TermReturn (make_expr (Literal Ast.Unit) ty_unit fn.fn_metadata.fn_span))
    fn.fn_metadata.fn_span;

  builder.blocks

(** 式からCFGを構築 (テスト用)
 *
 * @param expr トップレベル式
 * @return 基本ブロックリスト
 *)
let build_cfg_from_expr (expr : expr) : block list =
  VarIdGen.reset ();
  LabelGen.reset ();

  let builder = create_cfg_builder () in
  let entry_label = LabelGen.fresh "entry" in

  (* エントリブロックを開始 *)
  start_block builder entry_label;

  (* 式を線形化 *)
  let result_var = linearize_expr builder expr in

  (* エントリブロックを return で終了 *)
  finish_block builder
    (TermReturn (make_expr (Var result_var) result_var.vty result_var.vspan))
    expr.expr_span;

  builder.blocks

(* ========== CFG検証 ========== *)

(** 到達不能ブロックの検出
 *
 * @param blocks 基本ブロックリスト
 * @return 到達不能ブロックのラベルリスト
 *)
let find_unreachable_blocks (blocks : block list) : label list =
  (* ラベルからブロックへのルックアップを事前計算 *)
  let block_table = Hashtbl.create (List.length blocks) in
  List.iter (fun blk -> Hashtbl.replace block_table blk.label blk) blocks;

  (* 到達可能ラベル集合 *)
  let reachable = Hashtbl.create (List.length blocks) in

  let rec mark_reachable lbl =
    match Hashtbl.find_opt block_table lbl with
    | None -> ()
    | Some _ when Hashtbl.mem reachable lbl -> ()
    | Some blk -> (
        Hashtbl.add reachable lbl ();
        match blk.terminator with
        | TermReturn _ | TermUnreachable -> ()
        | TermJump target -> mark_reachable target
        | TermBranch (_, then_lbl, else_lbl) ->
            mark_reachable then_lbl;
            mark_reachable else_lbl
        | TermSwitch (_, cases, default_lbl) ->
            List.iter (fun (_, lbl) -> mark_reachable lbl) cases;
            mark_reachable default_lbl)
  in

  (* エントリブロックから探索開始 *)
  (match blocks with [] -> () | entry :: _ -> mark_reachable entry.label);

  List.filter_map
    (fun blk ->
      if Hashtbl.mem reachable blk.label then None else Some blk.label)
    blocks

(** 無限ループの検出
 *
 * @param blocks 基本ブロックリスト
 * @return ループヘッダのラベルリスト
 *)
let find_infinite_loops (blocks : block list) : label list =
  (* 簡易実装: 自分自身へジャンプするブロックを検出 *)
  List.filter_map
    (fun blk ->
      match blk.terminator with
      | TermJump target when target = blk.label -> Some blk.label
      | _ -> None)
    blocks

(** CFG整形性の検証
 *
 * @param blocks 基本ブロックリスト
 * @return (is_valid, error_messages)
 *)
let validate_cfg (blocks : block list) : bool * string list =
  let errors = ref [] in

  (* 1. 空のブロックリストチェック *)
  if List.length blocks = 0 then errors := "CFGが空です" :: !errors;

  (* 2. ラベルの重複チェック *)
  let label_set = Hashtbl.create (List.length blocks) in
  List.iter
    (fun blk ->
      if Hashtbl.mem label_set blk.label then
        errors := Printf.sprintf "ラベル '%s' が重複しています" blk.label :: !errors
      else Hashtbl.add label_set blk.label ())
    blocks;

  (* 3. 未定義ラベルへのジャンプチェック *)
  List.iter
    (fun blk ->
      let check_label lbl =
        if not (Hashtbl.mem label_set lbl) then
          errors := Printf.sprintf "未定義のラベル '%s' への参照があります" lbl :: !errors
      in
      match blk.terminator with
      | TermReturn _ | TermUnreachable -> ()
      | TermJump target -> check_label target
      | TermBranch (_, then_lbl, else_lbl) ->
          check_label then_lbl;
          check_label else_lbl
      | TermSwitch (_, cases, default_lbl) ->
          List.iter (fun (_, lbl) -> check_label lbl) cases;
          check_label default_lbl)
    blocks;

  (* 4. 到達不能ブロック警告 *)
  let unreachable = find_unreachable_blocks blocks in
  if List.length unreachable > 0 then
    errors :=
      Printf.sprintf "到達不能ブロック: %s" (String.concat ", " unreachable) :: !errors;

  (* 5. 無限ループ警告 *)
  let loops = find_infinite_loops blocks in
  if List.length loops > 0 then
    errors :=
      Printf.sprintf "無限ループの可能性: %s" (String.concat ", " loops) :: !errors;

  (List.length !errors = 0, List.rev !errors)
