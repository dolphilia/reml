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

(* ========== ループコンテキスト ========== *)

type loop_context = {
  continue_label : label option;
  continue_target : label;
  latch_label : label;
  loop_span : span;
}
(** ループ降格時に必要なメタデータ
 *
 * continue 先のラベルや latch ラベルを保持して、ループ内の
 * Continue 式から適切なジャンプを生成できるようにする。
 *)

(* ========== CFG構築コンテキスト ========== *)

type cfg_builder = {
  mutable current_label : label option;  (** 現在のブロックラベル *)
  mutable current_stmts : stmt list;  (** 現在のブロックの命令列 *)
  mutable blocks : block list;  (** 完成したブロック *)
  mutable split_points : split_point list;  (** 検出された分岐点 *)
  mutable loop_stack : loop_context list;  (** ネストしているループのスタック *)
  value_env : (int, var_id) Hashtbl.t;  (** 可変変数の現在値 (var_id.vid → SSA値) *)
}
(** CFG構築状態 *)

(** CFG構築状態の初期化 *)
let create_cfg_builder () : cfg_builder =
  {
    current_label = None;
    current_stmts = [];
    blocks = [];
    split_points = [];
    loop_stack = [];
    value_env = Hashtbl.create 32;
  }

(** 新しいブロックを開始 *)
let start_block (builder : cfg_builder) (lbl : label) : unit =
  builder.current_label <- Some lbl;
  builder.current_stmts <- []

(** 現在のブロックに命令を追加 *)
let add_stmt (builder : cfg_builder) (stmt : stmt) : unit =
  builder.current_stmts <- builder.current_stmts @ [ stmt ]

let set_value_env (builder : cfg_builder) (var : var_id) (value : var_id) : unit
    =
  Hashtbl.replace builder.value_env var.vid value

let get_value_env (builder : cfg_builder) (var : var_id) : var_id option =
  Hashtbl.find_opt builder.value_env var.vid

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

let push_loop (builder : cfg_builder) (ctx : loop_context) : unit =
  builder.loop_stack <- ctx :: builder.loop_stack

let pop_loop (builder : cfg_builder) : unit =
  match builder.loop_stack with
  | _ :: rest -> builder.loop_stack <- rest
  | [] -> failwith "pop_loop called but loop_stack is empty"

let current_loop (builder : cfg_builder) : loop_context option =
  match builder.loop_stack with ctx :: _ -> Some ctx | [] -> None

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
      if var.vmutable then (
        add_stmt builder (Alloca var);
        add_stmt builder (Store (var, bound_ref)))
      else add_stmt builder (Assign (var, bound_ref));
      if var.vmutable then set_value_env builder var bound_var
      else set_value_env builder var var;
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
  | AssignMutable (var, rhs) ->
      let rhs_var = linearize_expr builder rhs in
      let rhs_ref = make_expr (Var rhs_var) rhs.expr_ty rhs.expr_span in
      add_stmt builder (Store (var, rhs_ref));
      let unit_var = VarIdGen.fresh "$unit" ty_unit expr.expr_span in
      let unit_expr = make_expr (Literal Unit) ty_unit expr.expr_span in
      add_stmt builder (Assign (unit_var, unit_expr));
      set_value_env builder var rhs_var;
      unit_var
  | Continue -> (
      match current_loop builder with
      | None ->
          failwith "Continue encountered outside of loop during CFG lowering"
      | Some loop_ctx ->
          let unit_var = VarIdGen.fresh "$continue" ty_unit expr.expr_span in
          let unit_expr = make_expr (Literal Unit) ty_unit expr.expr_span in
          add_stmt builder (Assign (unit_var, unit_expr));
          finish_block builder (TermJump loop_ctx.continue_target)
            expr.expr_span;
          let dead_label = LabelGen.fresh "continue_dead" in
          start_block builder dead_label;
          unit_var)
  | Loop loop_info ->
      linearize_loop builder loop_info expr.expr_ty expr.expr_span
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
      let result = VarIdGen.fresh "$dict_init" expr.expr_ty expr.expr_span in
      let dict_expr =
        make_expr (DictConstruct dict) expr.expr_ty expr.expr_span
      in
      add_stmt builder (Assign (result, dict_expr));
      result
  | DictMethodCall (dict_expr, method_name, args, audit) ->
      let dict_var = linearize_expr builder dict_expr in
      let arg_vars = List.map (linearize_expr builder) args in
      let dict_ref = make_expr (Var dict_var) dict_var.vty dict_var.vspan in
      let arg_exprs =
        List.map (fun v -> make_expr (Var v) v.vty v.vspan) arg_vars
      in
      let call_expr =
        make_expr
          (DictMethodCall (dict_ref, method_name, arg_exprs, audit))
          expr.expr_ty expr.expr_span
      in
      let result = VarIdGen.fresh "$dict_call" expr.expr_ty expr.expr_span in
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

and linearize_loop (builder : cfg_builder) (info : loop_info) (result_ty : ty)
    (span : span) : var_id =
  let header_label = LabelGen.fresh "loop_header" in
  let body_label = LabelGen.fresh "loop_body" in
  let latch_label = LabelGen.fresh "loop_latch" in
  let exit_label = LabelGen.fresh "loop_exit" in

  let preheader_label =
    match builder.current_label with
    | Some lbl -> lbl
    | None -> failwith "linearize_loop called without an active preheader block"
  in

  let has_continue_source =
    info.loop_contains_continue
    || List.exists
         (fun { lc_sources; _ } ->
           List.exists
             (fun source -> source.ls_kind = LoopSourceContinue)
             lc_sources)
         info.loop_carried
  in
  let continue_label_opt =
    if has_continue_source then Some (LabelGen.fresh "loop_continue") else None
  in
  let continue_entries : (var_id * var_id ref * expr option) list ref =
    ref []
  in

  let loop_ctx =
    {
      continue_label = continue_label_opt;
      continue_target =
        (match continue_label_opt with Some lbl -> lbl | None -> latch_label);
      latch_label;
      loop_span = span;
    }
  in

  (* 事前初期化（for等） *)
  let () =
    match info.loop_kind with
    | ForLoop for_info ->
        List.iter
          (fun (var, init_expr) ->
            let init_var = linearize_expr builder init_expr in
            let init_value =
              make_expr (Var init_var) init_var.vty init_var.vspan
            in
            add_stmt builder (Assign (var, init_value));
            if var.vmutable then set_value_env builder var init_var
            else set_value_env builder var var)
          for_info.for_init
    | _ -> ()
  in

  (* preheader を header へジャンプさせて閉じる *)
  finish_block builder (TermJump header_label) span;

  (* header ブロック開始 *)
  start_block builder header_label;
  push_loop builder loop_ctx;

  let emit_effect_markers effects =
    List.iter
      (fun eff ->
        let effect_expr =
          match eff.effect_expr with
          | Some expr ->
              let var = linearize_expr builder expr in
              Some (make_expr (Var var) var.vty var.vspan)
          | None -> None
        in
        add_stmt builder (EffectMarker { eff with effect_expr }))
      effects
  in

  let phi_records =
    List.map
      (fun { lc_var; lc_sources } ->
        let init_value =
          match get_value_env builder lc_var with Some v -> v | None -> lc_var
        in
        let entries = ref [] in
        List.iter
          (fun source ->
            let kind = source.ls_kind in
            let label, expr_opt =
              match kind with
              | LoopSourcePreheader -> (preheader_label, Some source.ls_expr)
              | LoopSourceLatch -> (latch_label, Some source.ls_expr)
              | LoopSourceContinue -> (
                  match continue_label_opt with
                  | Some lbl -> (lbl, Some source.ls_expr)
                  | None ->
                      failwith
                        "LoopSourceContinue detected without allocated label")
            in
            let placeholder =
              match kind with
              | LoopSourcePreheader -> init_value
              | _ -> init_value
            in
            let value_ref =
              match List.find_opt (fun (k, _, _, _) -> k = kind) !entries with
              | Some (_, _, ref_value, _) ->
                  entries :=
                    List.map
                      (fun (k, lbl, vref, stored_expr) ->
                        if k = kind then (k, label, vref, expr_opt)
                        else (k, lbl, vref, stored_expr))
                      !entries;
                  ref_value
              | None ->
                  let vref = ref placeholder in
                  entries := !entries @ [ (kind, label, vref, expr_opt) ];
                  vref
            in
            if kind = LoopSourceContinue then
              let already_tracked =
                List.exists
                  (fun (tracked_var, tracked_ref, _) ->
                    tracked_var.vid = lc_var.vid && tracked_ref == value_ref)
                  !continue_entries
              in
              if not already_tracked then
                continue_entries :=
                  (lc_var, value_ref, expr_opt) :: !continue_entries)
          lc_sources;
        if
          not
            (List.exists
               (fun (kind, _, _, _) -> kind = LoopSourcePreheader)
               !entries)
        then
          entries :=
            (LoopSourcePreheader, preheader_label, ref init_value, None)
            :: !entries;
        let entries = !entries in
        let phi_var =
          VarIdGen.fresh
            (Printf.sprintf "%s_phi" lc_var.vname)
            lc_var.vty lc_var.vspan
        in
        let phi_stmt =
          Phi
            ( phi_var,
              List.map
                (fun (_, label, value_ref, _) -> (label, !value_ref))
                entries )
        in
        add_stmt builder phi_stmt;
        (if lc_var.vmutable then
           let phi_expr = make_expr (Var phi_var) phi_var.vty lc_var.vspan in
           add_stmt builder (Store (lc_var, phi_expr)));
        set_value_env builder lc_var phi_var;
        (lc_var, phi_var, entries))
      info.loop_carried
  in

  (* ループヘッダ効果を発火 *)
  emit_effect_markers info.loop_header_effects;

  (* 条件分岐の生成 *)
  (match info.loop_kind with
  | WhileLoop cond_expr ->
      let cond_var = linearize_expr builder cond_expr in
      finish_block builder
        (TermBranch
           ( make_expr (Var cond_var) cond_var.vty cond_var.vspan,
             body_label,
             exit_label ))
        cond_expr.expr_span
  | ForLoop for_info ->
      let cond_var = linearize_expr builder for_info.for_source in
      finish_block builder
        (TermBranch
           ( make_expr (Var cond_var) cond_var.vty cond_var.vspan,
             body_label,
             exit_label ))
        for_info.for_source.expr_span
  | InfiniteLoop -> finish_block builder (TermJump body_label) info.loop_span);

  (* 本体ブロック *)
  start_block builder body_label;
  emit_effect_markers info.loop_body_effects;
  let _body_result = linearize_expr builder info.loop_body in
  finish_block builder (TermJump latch_label) info.loop_span;

  (* latch ブロック *)
  start_block builder latch_label;

  (match info.loop_kind with
  | ForLoop for_info ->
      List.iter
        (fun (var, step_expr) ->
          let step_var = linearize_expr builder step_expr in
          let step_value =
            make_expr (Var step_var) step_var.vty step_var.vspan
          in
          add_stmt builder (Assign (var, step_value));
          if var.vmutable then set_value_env builder var step_var
          else set_value_env builder var var)
        for_info.for_step
  | _ -> ());

  finish_block builder (TermJump header_label) info.loop_span;

  let latch_values =
    let tbl = Hashtbl.create (List.length phi_records) in
    List.iter
      (fun (lc_var, _phi_var, _entries) ->
        let value =
          match get_value_env builder lc_var with Some v -> v | None -> lc_var
        in
        Hashtbl.replace tbl lc_var.vid value)
      phi_records;
    tbl
  in

  (match continue_label_opt with
  | Some continue_label ->
      start_block builder continue_label;
      let continue_items = List.rev !continue_entries in
      List.iter
        (fun (lc_var, value_ref, expr_opt) ->
          let continue_var =
            match expr_opt with
            | Some expr -> linearize_expr builder expr
            | None -> (
                match Hashtbl.find_opt latch_values lc_var.vid with
                | Some v -> v
                | None -> lc_var)
          in
          value_ref := continue_var;
          (if lc_var.vmutable then
             let store_expr =
               make_expr (Var continue_var) continue_var.vty continue_var.vspan
             in
             add_stmt builder (Store (lc_var, store_expr)));
          set_value_env builder lc_var continue_var)
        continue_items;
      finish_block builder (TermJump latch_label) info.loop_span
  | None -> ());

  List.iter
    (fun (lc_var, _phi_var, entries) ->
      List.iter
        (fun (kind, _label, value_ref, _) ->
          match kind with
          | LoopSourceLatch -> (
              match Hashtbl.find_opt latch_values lc_var.vid with
              | Some v -> value_ref := v
              | None -> ())
          | LoopSourcePreheader | LoopSourceContinue -> ())
        entries)
    phi_records;

  let phi_patch_entries =
    List.map
      (fun (_lc_var, phi_var, entries) ->
        let sources =
          List.map
            (fun (_kind, label, value_ref, _) -> (label, !value_ref))
            entries
        in
        (phi_var, sources))
      phi_records
  in

  let update_phi_in_block blk =
    if not (String.equal blk.label header_label) then blk
    else
      let stmts =
        List.map
          (fun stmt ->
            match stmt with
            | Phi (var, _) -> (
                match
                  List.find_opt
                    (fun (phi_var, _) -> phi_var.vid = var.vid)
                    phi_patch_entries
                with
                | Some (_phi_var, sources) -> Phi (var, sources)
                | None -> stmt)
            | other -> other)
          blk.stmts
      in
      { blk with stmts }
  in

  builder.blocks <- List.map update_phi_in_block builder.blocks;

  List.iter
    (fun (lc_var, phi_var, _entries) -> set_value_env builder lc_var phi_var)
    phi_records;

  pop_loop builder;

  (* exit ブロック開始（後続処理のため未収束） *)
  start_block builder exit_label;
  let loop_result = VarIdGen.fresh "$loop_result" result_ty span in
  let loop_value = make_expr (Literal Ast.Unit) result_ty span in
  add_stmt builder (Assign (loop_result, loop_value));
  loop_result

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
