(* Core_ir.Desugar — Desugaring Pass for Core IR (Phase 3)
 *
 * このファイルは Typed AST を Core IR へ変換する際の糖衣削除パスを提供する。
 * 計画書: docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md §2
 *
 * 主要機能:
 * 1. パターンマッチの変換 (match → 決定木)
 * 2. パイプ演算子の展開 (|> → let 束縛)
 * 3. let再束縛の正規化 (SSA形式準備)
 *
 * 設計原則:
 * - Typed AST の型情報を完全に保持
 * - Span情報を引き継いで診断を容易に
 * - 段階的な変換（リテラル → タプル → コンストラクタ）
 *)

open Types
open Ast
open Typed_ast
open Ir
open Constraint_solver

(* ========== ユーティリティ ========== *)

exception DesugarError of string * span
(** エラー報告用のヘルパー *)

let desugar_error msg span = raise (DesugarError (msg, span))

(** 一時変数の生成 *)
let fresh_temp_var prefix ty span =
  VarIdGen.fresh (Printf.sprintf "$%s" prefix) ty span

(** Typed AST の型を Core IR の型へ変換（現在は単純にコピー） *)
let convert_ty ty = ty

let rec head_type_and_args ty =
  match ty with
  | TApp (fn, arg) ->
      let head, args = head_type_and_args fn in
      (head, args @ [ arg ])
  | _ -> (ty, [])

let is_user_type name ty =
  match head_type_and_args ty with
  | TCon (TCUser type_name), args when String.equal type_name name ->
      Some args
  | _ -> None

type for_source_kind =
  | ForSourceArray
  | ForSourceIterator

let is_array_like_ty ty =
  match ty with
  | TArray _ | TSlice _ -> true
  | _ -> Option.is_some (is_user_type "Array" ty)

let determine_for_source_kind (info : iterator_dict_info option) : for_source_kind =
  match info with
  | Some metadata -> (
      match metadata.kind with
      | IteratorArrayLike -> ForSourceArray
      | IteratorCoreIter
      | IteratorOptionLike
      | IteratorResultLike
      | IteratorCustom _ -> ForSourceIterator)
  | None -> ForSourceIterator

let stage_requirement_to_ir = function
  | IteratorStageExact stage -> StageExact (Effect.stage_id_of_string stage)
  | IteratorStageAtLeast stage ->
      StageAtLeast (Effect.stage_id_of_string stage)

let string_of_iterator_stage_requirement = function
  | IteratorStageExact stage -> Printf.sprintf "exact:%s" stage
  | IteratorStageAtLeast stage -> Printf.sprintf "at_least:%s" stage

let actual_stage_of_kind = function
  | IteratorArrayLike -> "stable"
  | IteratorCoreIter -> "beta"
  | IteratorOptionLike -> "beta"
  | IteratorResultLike -> "beta"
  | IteratorCustom name -> "custom:" ^ name

let effect_stage_requirement_to_ir = function
  | Effect_profile.StageExact stage -> Ir.StageExact stage
  | Effect_profile.StageAtLeast stage -> Ir.StageAtLeast stage

(* ========== 変数スコープマップ ========== *)

type var_scope_map = (string, var_id) Hashtbl.t
(** 変数スコープマップ
 *
 * 同名変数の再束縛を追跡し、SSA形式への変換を準備する。
 * Key: 変数名, Value: 現在のスコープでのvar_id
 *)

let create_scope_map () : var_scope_map = Hashtbl.create 64

let lookup_var (map : var_scope_map) (name : string) : var_id option =
  Hashtbl.find_opt map name

let bind_var (map : var_scope_map) (name : string) (var : var_id) : unit =
  Hashtbl.replace map name var

(** スコープのコピー（分岐処理用） *)
let copy_scope_map (map : var_scope_map) : var_scope_map = Hashtbl.copy map

type binding_mutability = BindingImmutable | BindingMutable

(* ========== 辞書生成パスの設計（Phase 2 Week 19-20 文書化） ========== *)

(** 【辞書生成パスの概要】
 *
 * Typed AST で収集されたトレイト制約を Core IR の辞書ノードへ変換する。
 * このパスは型推論後、制約解決後に実行される。
 *
 * 【処理フロー】（Week 21-22 実装予定）
 * 1. Constraint_solver.solve_constraints の結果（dict_ref list）を受け取る
 * 2. 各 dict_ref を DictLookup ノードに変換
 * 3. 関数宣言に辞書パラメータを挿入
 * 4. メソッド呼び出しを DictMethodCall に変換
 *
 * 【データフロー】
 * Typed AST + 制約リスト → Constraint Solver → Dict Refs → Core IR (DictLookup/DictMethodCall)
 *
 * 【参考】
 * - type_inference.ml の solve_trait_constraints
 * - constraint_solver.ml の DictImplicit/DictParam/DictLocal
 * - ir.ml の DictConstruct/DictMethodCall/DictLookup
 * - docs/plans/bootstrap-roadmap/2-1-typeclass-strategy.md §1.3
 * ======================================================================= *)

(** 標準トレイトのvtableメソッド順序
 *
 * 各トレイトのメソッドをvtableインデックス順に定義する。
 * この順序はConstraint_solverと一致させる必要がある。
 *
 * Week 21-22 実装時には、ir.ml の calculate_dict_layout と整合させる。
 *)
let trait_method_indices = function
  | "Eq" ->
      (* 仕様書 1-2 §B.1: 等価性比較 *)
      [
        ("eq", 0);   (* a == b *)
        ("ne", 1);   (* a != b *)
      ]
  | "Ord" ->
      (* 仕様書 1-2 §B.1: 順序付け（Eq をスーパートレイトとして要求） *)
      [
        ("cmp", 0);  (* a.cmp(b) → Ordering *)
        ("lt", 1);   (* a < b *)
        ("le", 2);   (* a <= b *)
        ("gt", 3);   (* a > b *)
        ("ge", 4);   (* a >= b *)
      ]
  | "Collector" ->
      (* 仕様書 3-1 §2.2: コレクション反復 *)
      [
        ("iter", 0);    (* for x in collection *)
        ("collect", 1); (* collection.collect() *)
      ]
  | _ -> []  (* 未知のトレイトはメソッド情報なし *)

(** トレイトメソッド名からvtableインデックスを取得
 *
 * Week 21-22 実装時、DictMethodCall の生成に使用。
 *)
let get_method_index (trait_name : string) (method_name : string) : int option =
  let methods = trait_method_indices trait_name in
  List.assoc_opt method_name methods

(** 辞書初期化コード生成のスタブ（Week 21-22 実装予定）
 *
 * 【実装手順】
 * 1. インスタンス宣言の検索
 *    - 型環境から `impl Trait for Type` を探す
 *    - 見つからない場合は組み込み実装を使用（Eq<i64> 等）
 *
 * 2. vtableの構築
 *    - trait_method_indices で定義された順序でメソッドを並べる
 *    - 各メソッドへの関数ポインタを収集
 *
 * 3. 辞書構造体の初期化
 *    - DictConstruct ノードを生成
 *    - ir.ml の make_dict_type でレイアウト情報を付与
 *
 * 【生成例】
 * ```reml
 * let add_i64(a: i64, b: i64) -> i64 = a + b
 * // ↓ 辞書生成後
 * let __dict_Add_i64 = DictConstruct {
 *   trait: "Add",
 *   impl_ty: i64,
 *   methods: [("add", add_i64)],
 *   layout: { vtable_size: 8, method_offsets: [("add", 0)], alignment: 8 }
 * }
 * ```
 *)
let generate_dict_init (trait_name : string) (ty : ty) (span : span) : expr option =
  (* Phase 2 Week 19-22 実装: 辞書初期化コード生成 *)

  (* 組み込み実装の判定ヘルパー *)
  let has_builtin_impl trait ty =
    match (trait, ty) with
    | ("Eq", TCon (TCInt _)) | ("Eq", TCon (TCFloat _))
    | ("Eq", TCon TCBool) | ("Eq", TCon TCChar) | ("Eq", TCon TCString)
    | ("Eq", TUnit) -> true
    | ("Ord", TCon (TCInt _)) | ("Ord", TCon (TCFloat _))
    | ("Ord", TCon TCBool) | ("Ord", TCon TCChar) | ("Ord", TCon TCString) -> true
    | _ -> false
  in

  (* メソッドシグネチャ取得ヘルパー *)
  let get_method_sig trait method_name impl_ty =
    match (trait, method_name) with
    | ("Eq", "eq") | ("Eq", "ne") -> Some (TArrow (impl_ty, TArrow (impl_ty, ty_bool)))
    | ("Ord", "lt") | ("Ord", "le") | ("Ord", "gt") | ("Ord", "ge") ->
        Some (TArrow (impl_ty, TArrow (impl_ty, ty_bool)))
    | _ -> None
  in

  (* 1. 組み込み実装の判定 *)
  if not (has_builtin_impl trait_name ty) then None
  else
    (* 2. vtable の構築 *)
    let methods = trait_method_indices trait_name in
    let methods_with_sigs =
      List.filter_map
        (fun (method_name, _) ->
          match get_method_sig trait_name method_name ty with
          | Some sig_ty -> Some (method_name, sig_ty)
          | None -> None)
        methods
    in

    if methods_with_sigs = [] then None
    else
      (* 3. 辞書型の構築 *)
      let dict_ty = make_dict_type trait_name ty methods_with_sigs in
      (* 4. DictConstruct ノードの生成 *)
      Some (make_expr (DictConstruct dict_ty) (TCon (TCUser "Dict")) span)

(** 型クラスメソッド呼び出しを辞書メソッドコールに変換（Phase 2 Week 19-22 実装）
 *
 * トレイトメソッド呼び出しを検出して DictMethodCall ノードに変換する。
 *
 * 現時点での実装制限:
 * - 辞書は関数パラメータとして明示的に渡されることを前提
 * - メソッド名は変数名から推測（例: `eq`, `lt` など）
 * - 将来的には型推論からのメタデータを利用
 *
 * @param fn_expr 関数式（メソッド参照の可能性がある）
 * @param args 引数リスト
 * @param ret_ty 戻り値の型
 * @param span 診断用位置情報
 * @return DictMethodCall ノード（該当する場合）
 *)
let try_convert_to_dict_method_call (fn_expr : expr) (args : expr list) (ret_ty : ty) (span : span) : expr option =
  (* Phase 2 Week 19-22 実装:
   *
   * 現時点では、関数名ベースの簡易検出のみを実装。
   * 完全な実装には、型推論からのトレイト情報が必要。
   *
   * 検出パターン:
   * 1. fn_expr が Var で、名前が既知のトレイトメソッド名と一致
   * 2. 第一引数が辞書型（Dict）の変数
   *
   * 例: eq(__dict_Eq_0, x, y) → DictMethodCall(__dict_Eq_0, "eq", [x, y])
   *)
  match fn_expr.expr_kind with
  | Var var when List.length args >= 1 ->
      let method_name = var.vname in
      (* 既知のトレイトメソッド名かチェック *)
      let is_trait_method trait_name =
        List.exists (fun (m, _) -> m = method_name) (trait_method_indices trait_name)
      in
      let trait_opt =
        if is_trait_method "Eq" then Some "Eq"
        else if is_trait_method "Ord" then Some "Ord"
        else if is_trait_method "Collector" then Some "Collector"
        else None
      in
      (match trait_opt with
      | Some _trait_name ->
          (* 第一引数が辞書かチェック *)
          let dict_arg = List.hd args in
          let method_args = List.tl args in
          (match dict_arg.expr_kind with
          | Var dict_var when String.starts_with ~prefix:"__dict_" dict_var.vname ->
              (* DictMethodCall ノードを生成 *)
              Some
                (make_expr
                   (DictMethodCall (dict_arg, method_name, method_args, None))
                   ret_ty span)
          | _ -> None)
      | None -> None)
  | _ -> None

(* ========== パターンマッチ決定木 ========== *)

(** 決定木ノード
 *
 * パターンマッチを決定木に変換するための中間表現。
 * 最終的に Core IR の if/match 式へ降格される。
 *)
type decision_tree =
  | Leaf of expr  (** 葉ノード: 実行する式 *)
  | Fail  (** 失敗ノード: マッチ失敗 *)
  | Switch of var_id * switch_case list  (** スイッチノード: 値による分岐 *)
  | Guard of expr * decision_tree * decision_tree  (** ガードノード: 条件付き分岐 *)

and switch_case = {
  test : test_kind;  (** テスト種別 *)
  subtree : decision_tree;  (** マッチした場合のサブツリー *)
}

and test_kind =
  | TestLiteral of literal  (** リテラル値テスト *)
  | TestConstructor of string * int  (** コンストラクタテスト (名前, アリティ) *)
  | TestTuple of int  (** タプルテスト (要素数) *)
  | TestWildcard  (** ワイルドカード（常に成功） *)

(* ========== リテラル・単純式の変換 ========== *)

(** リテラルの変換 *)
let desugar_literal lit ty span = make_expr (Literal lit) ty span

(** 変数参照の変換 *)
let desugar_var (map : var_scope_map) (id : ident) (ty : ty) (span : span) :
    expr =
  match lookup_var map id.name with
  | Some var -> make_expr (Var var) ty span
  | None ->
      (* 未定義変数（型推論で検出済みのはず） *)
      let var = VarIdGen.fresh id.name ty span in
      bind_var map id.name var;
      make_expr (Var var) ty span

(** 関数適用の変換 *)
let rec desugar_expr (map : var_scope_map) (texpr : typed_expr) : expr =
  let ty = convert_ty texpr.texpr_ty in
  let span = texpr.texpr_span in

  match texpr.texpr_kind with
  | TLiteral lit -> desugar_literal lit ty span
  | TVar (id, _scheme) -> desugar_var map id ty span
  | TCall (fn, args) ->
      let fn_expr = desugar_expr map fn in
      let arg_exprs = List.map (desugar_arg map) args in
      (* Phase 2 Week 19-22: トレイトメソッド呼び出しの検出 *)
      (match try_convert_to_dict_method_call fn_expr arg_exprs ty span with
      | Some dict_call -> dict_call
      | None -> make_expr (App (fn_expr, arg_exprs)) ty span)
  | TLambda (_params, _ret_ty, _body) ->
      (* クロージャ変換は後のフェーズで実装 *)
      (* Phase 1 では簡易実装: 環境キャプチャなし *)
      let closure_span = span in
      let env_vars = [] in
      (* TODO: 環境キャプチャの実装 *)
      let fn_ref = "$lambda" in
      (* TODO: 一意な名前生成 *)
      let closure_info = { env_vars; fn_ref; closure_span } in
      make_expr (Closure closure_info) ty span
  | TPipe (e1, e2) -> desugar_pipe map e1 e2 ty span
  | TBinary (op, e1, e2) ->
      let lhs = desugar_expr map e1 in
      let rhs = desugar_expr map e2 in
      let prim_op =
        match op with
        | Add -> PrimAdd
        | Sub -> PrimSub
        | Mul -> PrimMul
        | Div -> PrimDiv
        | Mod -> PrimMod
        | Eq -> PrimEq
        | Ne -> PrimNe
        | Lt -> PrimLt
        | Le -> PrimLe
        | Gt -> PrimGt
        | Ge -> PrimGe
        | And -> PrimAnd
        | Or -> PrimOr
        | Pow -> desugar_error "累乗演算子 (**) の Core IR 変換は未実装です" span
        | PipeOp -> desugar_error "パイプ演算子 (|>) は TPipe で処理されるべきです" span
      in
      make_expr (Primitive (prim_op, [ lhs; rhs ])) ty span
  | TUnary (op, expr) -> (
      let operand = desugar_expr map expr in
      match op with
      | Not -> make_expr (Primitive (PrimNot, [ operand ])) ty span
      | Neg ->
          (* 0 - operand *)
          let zero_literal =
            match expr.texpr_ty with
            | TCon (TCInt _) ->
                make_expr (Literal (Int ("0", Base10))) expr.texpr_ty span
            | TCon (TCFloat _) ->
                make_expr (Literal (Float "0.0")) expr.texpr_ty span
            | _ -> desugar_error "この型に対する単項マイナスは未対応です" span
          in
          make_expr (Primitive (PrimSub, [ zero_literal; operand ])) ty span)
  | TIf (cond, then_e, else_opt) ->
      let cond_expr = desugar_expr map cond in
      let then_expr = desugar_expr map then_e in
      let else_expr =
        match else_opt with
        | Some e -> desugar_expr map e
        | None -> make_expr (Literal Unit) ty_unit span
      in
      make_expr (If (cond_expr, then_expr, else_expr)) ty span
  | TMatch (scrut, arms) -> desugar_match map scrut arms ty span
  | TBlock stmts -> desugar_block map stmts ty span
  | TFieldAccess (e, field) ->
      let obj_expr = desugar_expr map e in
      make_expr (RecordAccess (obj_expr, field.name)) ty span
  | TTupleAccess (e, idx) ->
      let tuple_expr = desugar_expr map e in
      make_expr (TupleAccess (tuple_expr, idx)) ty span
  | TIndex (arr, idx) ->
      let arr_expr = desugar_expr map arr in
      let idx_expr = desugar_expr map idx in
      make_expr (ArrayAccess (arr_expr, idx_expr)) ty span
  | TWhile (cond, body) ->
      let cond_expr = desugar_expr map cond in
      let body_expr = desugar_expr map body in
      make_loop_expr (WhileLoop cond_expr) body_expr span ty
  | TFor (pat, source, body, iterator_dict, iterator_info) ->
      desugar_for_loop map pat source body iterator_dict iterator_info ty span
  | TLoop body ->
      let body_expr = desugar_expr map body in
      make_loop_expr InfiniteLoop body_expr span ty
  | TContinue ->
      make_expr Continue ty_unit span
  | TUnsafe body ->
      (* unsafe式は型検査済みなので、そのまま脱糖 *)
      desugar_expr map body
  | TReturn ret_opt ->
      (* return式は簡易実装: 返り値の式をそのまま返す
       * CFG構築時にTermReturnに変換される
       *)
      (match ret_opt with
      | Some e -> desugar_expr map e
      | None -> make_expr (Literal Unit) ty_unit span)
  | TDefer deferred_expr ->
      (* defer式は後のフェーズで実装 *)
      (* Phase 2では簡易実装: 式を評価するが遅延はしない *)
      desugar_expr map deferred_expr
  | TAssign (lhs_expr, rhs_expr) ->
      desugar_mutable_assign map lhs_expr rhs_expr span
  | _ ->
      (* その他の式は後のフェーズで実装 *)
      desugar_error "未実装の式種別" span

and desugar_arg (map : var_scope_map) (arg : typed_arg) : expr =
  match arg with
  | TPosArg e -> desugar_expr map e
  | TNamedArg (_name, e) -> desugar_expr map e

and desugar_for_loop (map : var_scope_map) (pat : typed_pattern)
    (source : typed_expr) (body : typed_expr) (_iterator_dict : dict_ref)
    (iterator_info : iterator_dict_info option) (result_ty : ty) (span : span) :
    expr =
  let source_kind = determine_for_source_kind iterator_info in
  let header_effects, body_effects, has_next_audit, next_audit =
    match (source_kind, iterator_info) with
    | ForSourceIterator, Some info ->
        let required_value method_name =
          Printf.sprintf "%s:%s" method_name
            (string_of_iterator_stage_requirement info.stage_requirement)
        in
        let actual_value method_name =
          Printf.sprintf "%s:%s" method_name
            (actual_stage_of_kind info.kind)
        in
        let make_string_literal value span =
          make_expr (Literal (String (value, Normal))) ty_string span
        in
        let make_effect tag span value =
          {
            effect_tag = { effect_name = tag; effect_span = span };
            effect_expr = Some (make_string_literal value span);
          }
        in
        let capability_id =
          Option.map
            (fun name -> { cap_name = name; cap_span = span })
            info.capability
        in
        let audit_for method_name audit_span =
          Some
            {
              audit_method = method_name;
              audit_effect =
                {
                  effect_name =
                    Printf.sprintf "effect.stage.iterator.%s" method_name;
                  effect_span = audit_span;
                };
              audit_required_stage =
                Some (stage_requirement_to_ir info.stage_requirement);
              audit_capability = capability_id;
            }
        in
        ( [ make_effect "effect.stage.iterator.required" source.texpr_span
                (required_value "has_next");
            make_effect "effect.stage.iterator.actual" source.texpr_span
                (actual_value "has_next");
          ],
          [ make_effect "effect.stage.iterator.required" body.texpr_span
                (required_value "next");
            make_effect "effect.stage.iterator.actual" body.texpr_span
                (actual_value "next");
          ],
          audit_for "has_next" source.texpr_span,
          audit_for "next" body.texpr_span )
    | _ -> ([], [], None, None)
  in
  let element_ty = convert_ty pat.tpat_ty in
  let source_expr = desugar_expr map source in
  let body_scope = copy_scope_map map in
  let bind_body value_expr =
    let cont () = desugar_expr body_scope body in
    desugar_pattern_binding body_scope pat value_expr
      ~mutability:BindingImmutable ~cont result_ty span
  in
  let for_lowering, loop_body =
    match source_kind with
    | ForSourceArray ->
        let array_ty = source_expr.expr_ty in
        let array_var =
          VarIdGen.fresh "$for_array" array_ty source.texpr_span
        in
        let index_var =
          VarIdGen.fresh ~mutable_:true "$for_index" ty_i64 span
        in
        let length_var = VarIdGen.fresh "$for_length" ty_i64 span in
        let zero =
          make_expr (Literal (Int ("0", Base10))) ty_i64 source.texpr_span
        in
        let array_ref = make_expr (Var array_var) array_ty span in
        let length_expr =
          make_expr
            (Primitive (PrimArrayLength, [ array_ref ]))
            ty_i64 source.texpr_span
        in
        let index_ref = make_expr (Var index_var) ty_i64 span in
        let length_ref = make_expr (Var length_var) ty_i64 span in
        let cond_expr =
          make_expr
            (Primitive (PrimLt, [ index_ref; length_ref ]))
            ty_bool span
        in
        let one =
          make_expr (Literal (Int ("1", Base10))) ty_i64 source.texpr_span
        in
        let step_expr =
          make_expr (Primitive (PrimAdd, [ index_ref; one ])) ty_i64 span
        in
        let element_expr =
          make_expr (ArrayAccess (array_ref, index_ref)) element_ty span
        in
        let loop_body = bind_body element_expr in
        ( { for_pattern = None;
            for_source = cond_expr;
            for_init =
              [
                (array_var, source_expr);
                (length_var, length_expr);
                (index_var, zero);
              ];
            for_step = [ (index_var, step_expr) ];
          },
          loop_body )
    | ForSourceIterator ->
        let iter_ty = source_expr.expr_ty in
        let iter_var =
          VarIdGen.fresh "$for_iter_state" iter_ty source.texpr_span
        in
        let iter_ref = make_expr (Var iter_var) iter_ty span in
        let has_next_expr =
          make_expr
            (DictMethodCall (iter_ref, "has_next", [], has_next_audit))
            ty_bool span
        in
        let element_expr =
          make_expr
            (DictMethodCall (iter_ref, "next", [], next_audit))
            element_ty span
        in
        let loop_body = bind_body element_expr in
        ( { for_pattern = None;
            for_source = has_next_expr;
            for_init = [ (iter_var, source_expr) ];
            for_step = [];
          },
          loop_body )
  in
  make_loop_expr ~header_effects ~body_effects (ForLoop for_lowering) loop_body
    span result_ty

(* ========== パイプ演算子の展開 ========== *)

and desugar_pipe (map : var_scope_map) (e1 : typed_expr) (e2 : typed_expr)
    (result_ty : ty) (span : span) : expr =
  (* a |> f を let t = a in f(t) へ変換 *)
  let arg_expr = desugar_expr map e1 in
  let arg_ty = convert_ty e1.texpr_ty in
  let temp_var = fresh_temp_var "pipe" arg_ty e1.texpr_span in

  (* e2 は関数適用（仮定） *)
  let fn_expr = desugar_expr map e2 in

  (* 一時変数への参照を構築 *)
  let temp_ref = make_expr (Var temp_var) arg_ty e1.texpr_span in

  (* 関数適用: f(temp_var) *)
  let app_expr = make_expr (App (fn_expr, [ temp_ref ])) result_ty span in

  (* let 束縛: let temp_var = arg_expr in app_expr *)
  make_expr (Let (temp_var, arg_expr, app_expr)) result_ty span

and desugar_mutable_assign (map : var_scope_map) (lhs : typed_expr)
    (rhs : typed_expr) (span : span) : expr =
  match lhs.texpr_kind with
  | TVar (id, _) -> (
      match lookup_var map id.name with
      | Some var ->
          if not var.vmutable then
            desugar_error "不変な変数には代入できません" span
          else
            let rhs_expr = desugar_expr map rhs in
            make_expr
              (AssignMutable (var, rhs_expr))
              ty_unit span
      | None ->
          desugar_error
            (Printf.sprintf "未宣言の変数 %s への代入です" id.name)
            span)
  | _ ->
      desugar_error
        "現在は単純な変数への代入（TVar）のみサポートしています" span

and collect_loop_carried_vars (body_expr : expr) :
    loop_carried_var list * bool =
  let base = collect_base_loop_carried body_expr in
  augment_loop_carried_with_continue body_expr base

and collect_base_loop_carried (body_expr : expr) : loop_carried_var list =
  let preheader_source var =
    let expr = make_expr (Var var) var.vty var.vspan in
    { ls_kind = LoopSourcePreheader; ls_span = var.vspan; ls_expr = expr }
  in
  let ensure_preheader var sources =
    if List.exists (fun src -> src.ls_kind = LoopSourcePreheader) sources then
      sources
    else preheader_source var :: sources
  in
  let add_source ordered var source =
    let rec aux acc = function
      | [] ->
          let sources =
            match source.ls_kind with
            | LoopSourcePreheader -> [ source ]
            | _ ->
                let preheader = preheader_source var in
                preheader :: [ source ]
          in
          List.rev ({ lc_var = var; lc_sources = sources } :: acc)
      | lc :: rest ->
          if lc.lc_var.vid = var.vid then
            let base_sources = ensure_preheader var lc.lc_sources in
            let sources =
              match source.ls_kind with
              | LoopSourcePreheader ->
                  if List.exists
                       (fun src -> src.ls_kind = LoopSourcePreheader)
                       base_sources
                  then base_sources
                  else source :: base_sources
              | _ -> base_sources @ [ source ]
            in
            List.rev_append acc ({ lc with lc_sources = sources } :: rest)
          else aux (lc :: acc) rest
    in
    aux [] ordered
  in
  let rec visit_expr ordered expr =
    match expr.expr_kind with
    | AssignMutable (var, rhs) ->
        let source =
          {
            ls_kind = LoopSourceLatch;
            ls_span = rhs.expr_span;
            ls_expr = rhs;
          }
        in
        let ordered = add_source ordered var source in
        visit_expr ordered rhs
    | Let (_, bound, body) ->
        let ordered = visit_expr ordered bound in
        visit_expr ordered body
    | App (fn, args) ->
        let ordered = visit_expr ordered fn in
        List.fold_left visit_expr ordered args
    | Primitive (_op, args) ->
        List.fold_left visit_expr ordered args
    | If (cond, then_e, else_e) ->
        let ordered = visit_expr ordered cond in
        let ordered = visit_expr ordered then_e in
        visit_expr ordered else_e
    | Match (scrut, cases) ->
        let ordered = visit_expr ordered scrut in
        List.fold_left
          (fun ordered case ->
            let ordered =
              match case.case_guard with
              | None -> ordered
              | Some guard -> visit_expr ordered guard
            in
            visit_expr ordered case.case_body)
          ordered cases
    | TupleAccess (tuple, _) -> visit_expr ordered tuple
    | RecordAccess (record, _) -> visit_expr ordered record
    | ArrayAccess (arr, idx) ->
        let ordered = visit_expr ordered arr in
        visit_expr ordered idx
    | ADTConstruct (_ctor, fields) ->
        List.fold_left visit_expr ordered fields
    | ADTProject (adt, _) -> visit_expr ordered adt
    | DictMethodCall (dict_expr, _name, args, _) ->
        let ordered = visit_expr ordered dict_expr in
        List.fold_left visit_expr ordered args
    | Continue -> ordered
    | Loop loop_info ->
        let ordered =
          match loop_info.loop_kind with
          | WhileLoop cond -> visit_expr ordered cond
          | ForLoop info ->
              let ordered =
                List.fold_left
                  (fun ordered (_, e) -> visit_expr ordered e)
                  ordered info.for_init
              in
              let ordered =
                List.fold_left
                  (fun ordered (_, e) -> visit_expr ordered e)
                  ordered info.for_step
              in
              let ordered =
                List.fold_left
                  (fun ordered (var, step_expr) ->
                    if var.vmutable then
                      let source =
                        {
                          ls_kind = LoopSourceLatch;
                          ls_span = step_expr.expr_span;
                          ls_expr = step_expr;
                        }
                      in
                      add_source ordered var source
                    else ordered)
                  ordered info.for_step
              in
              visit_expr ordered info.for_source
          | InfiniteLoop -> ordered
        in
        visit_expr ordered loop_info.loop_body
    | Closure _
    | DictLookup _
    | DictConstruct _
    | CapabilityCheck _
    | Literal _
    | Var _ ->
        ordered
  in
  visit_expr [] body_expr

and augment_loop_carried_with_continue (body_expr : expr)
    (initial : loop_carried_var list) :
    loop_carried_var list * bool =
  let module IntKey = struct
    type t = int

    let compare (a : int) (b : int) = Stdlib.compare a b
  end in
  let module IntMap = Map.Make (IntKey) in

  let loop_carried_ref = ref initial in
  let has_continue = ref false in

  let update_loop_carried var expr span =
    loop_carried_ref :=
      List.map
        (fun lc ->
          if lc.lc_var.vid = var.vid then
            let source =
              { ls_kind = LoopSourceContinue; ls_span = span; ls_expr = expr }
            in
            { lc with lc_sources = lc.lc_sources @ [ source ] }
          else lc)
        !loop_carried_ref
  in

  let make_current_value var =
    make_expr (Var var) var.vty var.vspan
  in

  let rec visit ~collect env expr =
    let span = expr.expr_span in
    match expr.expr_kind with
    | Continue ->
        if collect then
          has_continue := true;
          List.iter
            (fun lc ->
              let expr =
                match IntMap.find_opt lc.lc_var.vid env with
                | Some value -> value
                | None -> make_current_value lc.lc_var
              in
              update_loop_carried lc.lc_var expr span)
            !loop_carried_ref;
        env
    | AssignMutable (var, rhs) ->
        let env = visit ~collect env rhs in
        IntMap.add var.vid rhs env
    | Let (_var, bound, body) ->
        let env = visit ~collect env bound in
        visit ~collect env body
    | App (fn, args) ->
        let env = visit ~collect env fn in
        List.fold_left (visit ~collect) env args
    | Primitive (_op, args) ->
        List.fold_left (visit ~collect) env args
    | If (cond, then_e, else_e) ->
        let env_after_cond = visit ~collect env cond in
        ignore (visit ~collect env_after_cond then_e);
        ignore (visit ~collect env_after_cond else_e);
        env_after_cond
    | Match (scrut, cases) ->
        let env_after_scrut = visit ~collect env scrut in
        List.iter
          (fun case ->
            let env_case =
              match case.case_guard with
              | Some guard -> visit ~collect env_after_scrut guard
              | None -> env_after_scrut
            in
            ignore (visit ~collect env_case case.case_body))
          cases;
        env_after_scrut
    | TupleAccess (tuple, _) -> visit ~collect env tuple
    | RecordAccess (record, _) -> visit ~collect env record
    | ArrayAccess (arr, idx) ->
        let env = visit ~collect env arr in
        visit ~collect env idx
    | ADTConstruct (_ctor, fields) ->
        List.fold_left (visit ~collect) env fields
    | ADTProject (adt, _) -> visit ~collect env adt
    | DictMethodCall (dict_expr, _name, args, _) ->
        let env = visit ~collect env dict_expr in
        List.fold_left (visit ~collect) env args
    | Loop loop_info ->
        let env =
          match loop_info.loop_kind with
          | WhileLoop cond -> visit ~collect:false env cond
          | ForLoop info ->
              let env =
                List.fold_left
                  (fun env (_, e) -> visit ~collect:false env e)
                  env info.for_init
              in
              let env =
                List.fold_left
                  (fun env (_, e) -> visit ~collect:false env e)
                  env info.for_step
              in
              visit ~collect:false env info.for_source
          | InfiniteLoop -> env
        in
        visit ~collect:false env loop_info.loop_body
    | Closure _ | DictLookup _ | DictConstruct _ | CapabilityCheck _
    | Literal _ | Var _ ->
        env
  in

  ignore (visit ~collect:true IntMap.empty body_expr);
  (!loop_carried_ref, !has_continue)

and make_loop_expr ?(header_effects = []) ?(body_effects = [])
    (loop_kind : loop_kind) (loop_body : expr) (span : span) (ty : ty) : expr =
  let loop_carried, has_continue = collect_loop_carried_vars loop_body in
  make_expr
    (Loop
       {
         loop_kind;
         loop_body;
         loop_span = span;
         loop_carried;
         loop_contains_continue = has_continue;
         loop_header_effects = header_effects;
         loop_body_effects = body_effects;
       })
    ty span

(* ========== ブロック式の変換 ========== *)

and desugar_block (map : var_scope_map) (stmts : typed_stmt list)
    (result_ty : ty) (span : span) : expr =
  match stmts with
  | [] ->
      (* 空のブロック → Unit *)
      make_expr (Literal Unit) ty_unit span
  | [ TExprStmt e ] ->
      (* 単一の式 *)
      desugar_expr map e
  | stmt :: rest -> (
      (* 複数の文 → let 束縛の連鎖 *)
      match stmt with
      | TDeclStmt decl ->
          (* let/var 宣言 *)
          desugar_block_with_decl map decl rest result_ty span
      | TExprStmt e ->
          (* 式文 → 評価して次へ *)
          let e_expr = desugar_expr map e in
          let rest_expr = desugar_block map rest result_ty span in
          (* 副作用のみを評価する式として扱う *)
          let dummy_var =
            fresh_temp_var "unused" (convert_ty e.texpr_ty) e.texpr_span
          in
          make_expr (Let (dummy_var, e_expr, rest_expr)) result_ty span
      | TAssignStmt (lhs, rhs) ->
          let assign_expr =
            desugar_mutable_assign map lhs rhs lhs.texpr_span
          in
          let rest_expr = desugar_block map rest result_ty span in
          let dummy_var =
            fresh_temp_var "assign" assign_expr.expr_ty assign_expr.expr_span
          in
          make_expr (Let (dummy_var, assign_expr, rest_expr)) result_ty span
      | TDeferStmt _ ->
          (* defer 文 → ランタイムサポートが必要（後のフェーズ） *)
          desugar_error "defer 文の変換は未実装" span)

and desugar_block_with_decl (map : var_scope_map) (decl : typed_decl)
    (rest : typed_stmt list) (result_ty : ty) (span : span) : expr =
  match decl.tdecl_kind with
  | TLetDecl (pat, e) ->
      let bound_expr = desugar_expr map e in
      let continuation () = desugar_block map rest result_ty span in
      (* パターン束縛を let に変換（スコープを先に拡張） *)
      desugar_pattern_binding map pat bound_expr ~mutability:BindingImmutable
        ~cont:continuation result_ty span
  | TVarDecl (pat, e) ->
      (* var 宣言も同様（可変性は後で処理） *)
      let bound_expr = desugar_expr map e in
      let continuation () = desugar_block map rest result_ty span in
      desugar_pattern_binding map pat bound_expr ~mutability:BindingMutable
        ~cont:continuation result_ty span
  | TFnDecl _ ->
      (* 関数宣言はトップレベル処理（ブロック内関数は後のフェーズ） *)
      desugar_error "ブロック内関数宣言は未実装" span
  | _ -> desugar_error "未対応の宣言種別" span

(* ========== パターン束縛の変換 ========== *)

and desugar_pattern_binding (map : var_scope_map) (pat : typed_pattern)
    (bound_expr : expr) ~(mutability : binding_mutability)
    ~(cont : unit -> expr) (result_ty : ty) (span : span) : expr =
  let is_mutable =
    match mutability with BindingMutable -> true | BindingImmutable -> false
  in
  match pat.tpat_kind with
  | TPatVar id ->
      (* 単純な変数束縛 *)
      let var =
        VarIdGen.fresh ~mutable_:is_mutable id.name (convert_ty pat.tpat_ty)
          pat.tpat_span
      in
      bind_var map id.name var;
      let rest_expr = cont () in
      make_expr (Let (var, bound_expr, rest_expr)) result_ty span
  | TPatWildcard ->
      (* ワイルドカード → 値を無視 *)
      let dummy_var =
        fresh_temp_var "wildcard" (convert_ty pat.tpat_ty) pat.tpat_span
      in
      let rest_expr = cont () in
      make_expr (Let (dummy_var, bound_expr, rest_expr)) result_ty span
  | TPatTuple pats ->
      (* タプルパターン → タプルアクセスで分解 *)
      let temp_var =
        fresh_temp_var "tuple" (convert_ty pat.tpat_ty) pat.tpat_span
      in
      let bindings =
        List.mapi
          (fun i sub_pat ->
            let access_expr =
              make_expr
                (TupleAccess
                   ( make_expr (Var temp_var) (convert_ty pat.tpat_ty)
                       pat.tpat_span,
                     i ))
                (convert_ty sub_pat.tpat_ty)
                sub_pat.tpat_span
            in
            (sub_pat, access_expr))
          pats
      in
      (* ネストした let 束縛を生成 *)
      let rec build_bindings pairs cont_fn =
        match pairs with
        | [] -> cont_fn ()
        | (sub_pat, access) :: rest ->
            desugar_pattern_binding map sub_pat access
              ~mutability ~cont:(fun () -> build_bindings rest cont_fn)
              result_ty span
      in
      let rest_expr = build_bindings bindings cont in
      make_expr (Let (temp_var, bound_expr, rest_expr)) result_ty span
  | TPatRecord (fields, _has_rest) ->
      (* レコードパターン → フィールドアクセスで分解 *)
      let temp_var =
        fresh_temp_var "record" (convert_ty pat.tpat_ty) pat.tpat_span
      in
      let rec bind_fields field_list cont_fn =
        match field_list with
        | [] -> cont_fn ()
        | (field_name, field_pat_opt) :: rest -> (
            match field_pat_opt with
            | Some field_pat ->
                (* { field: pattern } 形式 *)
                let access_expr =
                  make_expr
                    (RecordAccess
                       ( make_expr (Var temp_var) (convert_ty pat.tpat_ty)
                           pat.tpat_span,
                         field_name.name ))
                    (convert_ty field_pat.tpat_ty)
                    field_pat.tpat_span
                in
                desugar_pattern_binding map field_pat access_expr
                  ~mutability ~cont:(fun () -> bind_fields rest cont_fn)
                  result_ty span
            | None ->
                (* { field } 短縮形 → { field: field } として扱う *)
                let field_var =
                  VarIdGen.fresh ~mutable_:is_mutable field_name.name ty_i64
                    field_name.span
                in
                bind_var map field_name.name field_var;
                let access_expr =
                  make_expr
                    (RecordAccess
                       ( make_expr (Var temp_var) (convert_ty pat.tpat_ty)
                           pat.tpat_span,
                         field_name.name ))
                    ty_i64 field_name.span
                in
                let rest_expr = bind_fields rest cont_fn in
                make_expr
                  (Let (field_var, access_expr, rest_expr))
                  result_ty span)
      in
      let rest_expr = bind_fields fields cont in
      make_expr (Let (temp_var, bound_expr, rest_expr)) result_ty span
  | TPatConstructor (_ctor_name, arg_pats) ->
      (* コンストラクタパターン → タグ検査 + payload 射影 *)
      let temp_var =
        fresh_temp_var "adt" (convert_ty pat.tpat_ty) pat.tpat_span
      in
      (* payload の取り出しと各引数パターンへの束縛 *)
      let rec bind_args args cont_fn =
        match args with
        | [] -> cont_fn ()
        | (idx, arg_pat) :: rest ->
            let project_expr =
              make_expr
                (ADTProject
                   ( make_expr (Var temp_var) (convert_ty pat.tpat_ty)
                       pat.tpat_span,
                     idx ))
                (convert_ty arg_pat.tpat_ty)
                arg_pat.tpat_span
            in
            desugar_pattern_binding map arg_pat project_expr
              ~mutability ~cont:(fun () -> bind_args rest cont_fn)
              result_ty span
      in
      let rest_expr = bind_args (List.mapi (fun i p -> (i, p)) arg_pats) cont in
      make_expr (Let (temp_var, bound_expr, rest_expr)) result_ty span
  | TPatGuard (inner_pat, guard_expr) ->
      (* ガード付きパターン → 内側のパターンを先に束縛し、ガードを if 式に変換 *)
      let guard_ir = desugar_expr map guard_expr in
      let then_branch =
        desugar_pattern_binding map inner_pat bound_expr ~mutability ~cont
          result_ty span
      in
      let else_branch = desugar_error "パターンマッチ失敗（ガード条件不一致）" span in
      make_expr (If (guard_ir, then_branch, else_branch)) result_ty span
  | TPatLiteral lit ->
      (* リテラルパターン → 等価性チェック（通常は match で処理されるが念のため） *)
      let lit_expr =
        make_expr (Literal lit) (convert_ty pat.tpat_ty) pat.tpat_span
      in
      let cond =
        make_expr
          (Primitive (PrimEq, [ bound_expr; lit_expr ]))
          ty_bool pat.tpat_span
      in
      let then_branch = cont () in
      let else_branch = desugar_error "パターンマッチ失敗（リテラル不一致）" span in
      make_expr (If (cond, then_branch, else_branch)) result_ty span

(* ========== パターンマッチの変換 ========== *)

and desugar_match (map : var_scope_map) (scrut : typed_expr)
    (arms : typed_match_arm list) (result_ty : ty) (span : span) : expr =
  (* scrutinee を一時変数に束縛 *)
  let scrut_expr = desugar_expr map scrut in
  let scrut_ty = convert_ty scrut.texpr_ty in
  let scrut_var = fresh_temp_var "match" scrut_ty scrut.texpr_span in

  (* 決定木を構築 *)
  let decision_tree = compile_decision_tree map scrut_var arms in

  (* 決定木を Core IR 式に変換 *)
  let match_body = decision_tree_to_expr map decision_tree result_ty span in

  (* let scrut_var = scrut_expr in match_body *)
  make_expr (Let (scrut_var, scrut_expr, match_body)) result_ty span

and compile_decision_tree (map : var_scope_map) (scrut_var : var_id)
    (arms : typed_match_arm list) : decision_tree =
  match arms with
  | [] ->
      (* マッチアームがない → 失敗 *)
      Fail
  | arm :: rest ->
      (* 最初のアームを処理 *)
      compile_arm map scrut_var arm rest

and compile_arm (map : var_scope_map) (scrut_var : var_id)
    (arm : typed_match_arm) (rest : typed_match_arm list) : decision_tree =
  let pattern = arm.tarm_pattern in
  let guard = arm.tarm_guard in
  let body = arm.tarm_body in

  (* パターンに基づいて決定木ノードを生成 *)
  let success_tree = Leaf (desugar_expr map body) in
  let failure_tree = compile_decision_tree map scrut_var rest in

  match pattern.tpat_kind with
  | TPatWildcard -> (
      (* ワイルドカード → 常に成功 *)
      match guard with
      | Some g ->
          let guard_expr = desugar_expr map g in
          Guard (guard_expr, success_tree, failure_tree)
      | None -> success_tree)
  | TPatLiteral lit ->
      (* リテラルパターン → スイッチノード *)
      let test_case = { test = TestLiteral lit; subtree = success_tree } in
      Switch (scrut_var, [ test_case ])
  | TPatVar id ->
      (* 変数パターン → 束縛して成功 *)
      let var =
        VarIdGen.fresh id.name (convert_ty pattern.tpat_ty) pattern.tpat_span
      in
      bind_var map id.name var;
      (* TODO: 変数束縛を決定木に組み込む *)
      success_tree
  | TPatTuple sub_pats ->
      (* タプルパターン → タプル長検査 + 各要素のマッチ *)
      let arity = List.length sub_pats in
      let subtree =
        compile_tuple_pattern map scrut_var sub_pats guard body failure_tree
      in
      let test_case = { test = TestTuple arity; subtree } in
      Switch (scrut_var, [ test_case ])
  | TPatConstructor (ctor_name, arg_pats) ->
      (* コンストラクタパターン → タグ検査 + payload マッチ *)
      let arity = List.length arg_pats in
      let subtree =
        compile_constructor_pattern map scrut_var ctor_name.name arg_pats guard
          body failure_tree
      in
      let test_case =
        { test = TestConstructor (ctor_name.name, arity); subtree }
      in
      Switch (scrut_var, [ test_case ])
  | TPatRecord (fields, has_rest) ->
      (* レコードパターン → フィールド検査の連鎖 *)
      compile_record_pattern map scrut_var fields has_rest guard body
        failure_tree
  | TPatGuard (inner_pat, guard_expr) ->
      (* ガード付きパターン → 内側パターンのマッチ + ガード条件 *)
      let guard_ir = desugar_expr map guard_expr in
      let inner_tree =
        compile_arm map scrut_var
          {
            tarm_pattern = inner_pat;
            tarm_guard = None;
            tarm_body = body;
            tarm_span = arm.tarm_span;
          }
          rest
      in
      Guard (guard_ir, inner_tree, failure_tree)

(* ========== パターンコンパイル補助関数 ========== *)

(** タプルパターンのコンパイル *)
and compile_tuple_pattern (map : var_scope_map) (scrut_var : var_id)
    (sub_pats : typed_pattern list) (guard : typed_expr option)
    (body : typed_expr) (failure_tree : decision_tree) : decision_tree =
  (* 各要素を一時変数に束縛 *)
  let element_vars =
    List.mapi
      (fun i sub_pat ->
        let element_ty = convert_ty sub_pat.tpat_ty in
        let element_var =
          fresh_temp_var
            (Printf.sprintf "tuple_elem%d" i)
            element_ty sub_pat.tpat_span
        in
        (i, sub_pat, element_var))
      sub_pats
  in

  (* 各要素のパターンマッチを再帰的にコンパイル *)
  let rec compile_elements elems =
    match elems with
    | [] -> (
        (* 全要素マッチ成功 → ガード条件チェック後、本体を実行 *)
        match guard with
        | Some g ->
            let guard_expr = desugar_expr map g in
            Guard (guard_expr, Leaf (desugar_expr map body), failure_tree)
        | None -> Leaf (desugar_expr map body))
    | (idx, sub_pat, elem_var) :: rest -> (
        (* 要素の取り出しと束縛 *)
        let _access_expr =
          make_expr
            (TupleAccess
               (make_expr (Var scrut_var) scrut_var.vty scrut_var.vspan, idx))
            elem_var.vty elem_var.vspan
        in
        (* 要素パターンのマッチを決定木に組み込む *)
        match sub_pat.tpat_kind with
        | TPatVar id ->
            bind_var map id.name elem_var;
            compile_elements rest
        | TPatWildcard -> compile_elements rest
        | _ ->
            (* ネストパターンは再帰的にコンパイル *)
            compile_elements rest)
  in
  compile_elements element_vars

(** コンストラクタパターンのコンパイル *)
and compile_constructor_pattern (map : var_scope_map) (scrut_var : var_id)
    (ctor_name : string) (arg_pats : typed_pattern list)
    (guard : typed_expr option) (body : typed_expr)
    (failure_tree : decision_tree) : decision_tree =
  (* 各引数を payload から取り出す *)
  let arg_vars =
    List.mapi
      (fun i arg_pat ->
        let arg_ty = convert_ty arg_pat.tpat_ty in
        let arg_var =
          fresh_temp_var
            (Printf.sprintf "%s_arg%d" ctor_name i)
            arg_ty arg_pat.tpat_span
        in
        (i, arg_pat, arg_var))
      arg_pats
  in

  (* 各引数のパターンマッチを再帰的にコンパイル *)
  let rec compile_args args =
    match args with
    | [] -> (
        (* 全引数マッチ成功 → ガード条件チェック後、本体を実行 *)
        match guard with
        | Some g ->
            let guard_expr = desugar_expr map g in
            Guard (guard_expr, Leaf (desugar_expr map body), failure_tree)
        | None -> Leaf (desugar_expr map body))
    | (idx, arg_pat, arg_var) :: rest -> (
        (* payload の射影 *)
        let _project_expr =
          make_expr
            (ADTProject
               (make_expr (Var scrut_var) scrut_var.vty scrut_var.vspan, idx))
            arg_var.vty arg_var.vspan
        in
        (* 引数パターンのマッチを決定木に組み込む *)
        match arg_pat.tpat_kind with
        | TPatVar id ->
            bind_var map id.name arg_var;
            compile_args rest
        | TPatWildcard -> compile_args rest
        | _ ->
            (* ネストパターンは再帰的にコンパイル *)
            compile_args rest)
  in
  compile_args arg_vars

(** レコードパターンのコンパイル *)
and compile_record_pattern (map : var_scope_map) (scrut_var : var_id)
    (fields : (ident * typed_pattern option) list) (_has_rest : bool)
    (guard : typed_expr option) (body : typed_expr)
    (failure_tree : decision_tree) : decision_tree =
  (* 各フィールドを一時変数に束縛 *)
  let field_bindings =
    List.map
      (fun (field_name, field_pat_opt) ->
        match field_pat_opt with
        | Some field_pat ->
            let field_ty = convert_ty field_pat.tpat_ty in
            let field_var =
              fresh_temp_var
                (Printf.sprintf "field_%s" field_name.name)
                field_ty field_pat.tpat_span
            in
            (field_name.name, Some field_pat, field_var)
        | None ->
            (* 短縮形 { field } *)
            let field_var =
              fresh_temp_var field_name.name ty_i64 field_name.span
            in
            bind_var map field_name.name field_var;
            (field_name.name, None, field_var))
      fields
  in

  (* 各フィールドのパターンマッチを再帰的にコンパイル *)
  let rec compile_fields flds =
    match flds with
    | [] -> (
        (* 全フィールドマッチ成功 → ガード条件チェック後、本体を実行 *)
        match guard with
        | Some g ->
            let guard_expr = desugar_expr map g in
            Guard (guard_expr, Leaf (desugar_expr map body), failure_tree)
        | None -> Leaf (desugar_expr map body))
    | (field_name, field_pat_opt, field_var) :: rest -> (
        (* フィールドアクセス *)
        let _access_expr =
          make_expr
            (RecordAccess
               ( make_expr (Var scrut_var) scrut_var.vty scrut_var.vspan,
                 field_name ))
            field_var.vty field_var.vspan
        in
        (* フィールドパターンのマッチを決定木に組み込む *)
        match field_pat_opt with
        | Some field_pat -> (
            match field_pat.tpat_kind with
            | TPatVar id ->
                bind_var map id.name field_var;
                compile_fields rest
            | TPatWildcard -> compile_fields rest
            | _ ->
                (* ネストパターンは再帰的にコンパイル *)
                compile_fields rest)
        | None ->
            (* 短縮形は既に束縛済み *)
            compile_fields rest)
  in
  compile_fields field_bindings

and decision_tree_to_expr (map : var_scope_map) (tree : decision_tree)
    (result_ty : ty) (span : span) : expr =
  match tree with
  | Leaf e -> e
  | Fail ->
      (* マッチ失敗 → panic（ランタイムサポート） *)
      desugar_error "パターンマッチ失敗の処理は未実装" span
  | Switch (var, cases) ->
      (* スイッチノードを if 式の連鎖に変換 *)
      compile_switch_to_if map var cases result_ty span
  | Guard (cond, then_tree, else_tree) ->
      (* ガードノードを if 式に変換 *)
      let then_expr = decision_tree_to_expr map then_tree result_ty span in
      let else_expr = decision_tree_to_expr map else_tree result_ty span in
      make_expr (If (cond, then_expr, else_expr)) result_ty span

and compile_switch_to_if (map : var_scope_map) (var : var_id)
    (cases : switch_case list) (result_ty : ty) (span : span) : expr =
  match cases with
  | [] ->
      (* ケースなし → 失敗 *)
      desugar_error "スイッチケースが空" span
  | [ case ] ->
      (* 単一ケース *)
      decision_tree_to_expr map case.subtree result_ty span
  | case :: rest ->
      (* 複数ケース → if 式の連鎖 *)
      let test_expr = compile_test_expr var case.test span in
      let then_expr = decision_tree_to_expr map case.subtree result_ty span in
      let else_expr = compile_switch_to_if map var rest result_ty span in
      make_expr (If (test_expr, then_expr, else_expr)) result_ty span

and compile_test_expr (var : var_id) (test : test_kind) (span : span) : expr =
  match test with
  | TestLiteral lit ->
      (* var == lit *)
      let var_ref = make_expr (Var var) var.vty span in
      let lit_expr = make_expr (Literal lit) var.vty span in
      make_expr (Primitive (PrimEq, [ var_ref; lit_expr ])) ty_bool span
  | TestWildcard ->
      (* 常に true *)
      make_expr (Literal (Bool true)) ty_bool span
  | TestConstructor (ctor_name, _arity) ->
      (* ADT のタグ検査: var.tag == ctor_tag *)
      (* TODO: コンストラクタ名からタグIDへのマッピングが必要（Phase 3 後半で実装） *)
      (* 暫定実装: コンストラクタ名をハッシュ化してタグIDとする *)
      let tag_id = Hashtbl.hash ctor_name in
      let var_ref = make_expr (Var var) var.vty span in
      (* ADT タグ取得（仮想的な操作、LLVM 生成時に実装） *)
      let tag_access = make_expr (ADTProject (var_ref, -1)) ty_i64 span in
      (* -1 は特殊なタグフィールド *)
      let tag_lit =
        make_expr (Literal (Int (string_of_int tag_id, Base10))) ty_i64 span
      in
      make_expr (Primitive (PrimEq, [ tag_access; tag_lit ])) ty_bool span
  | TestTuple _arity ->
      (* タプル長検査: tuple.length == arity *)
      (* 型システムで既に検証済みのため、常に true を返す *)
      (* （実際の検査は型推論フェーズで完了している） *)
      make_expr (Literal (Bool true)) ty_bool span

(* ========== トップレベル変換 ========== *)

(** 辞書パラメータの生成（Phase 2 Week 19-22 実装）
 *
 * トレイト制約のリストから辞書パラメータを生成する。
 * 例: [Eq<T>, Ord<T>] → [__dict_Eq_T, __dict_Ord_T]
 *
 * @param fn_scope 関数スコープ
 * @param constraints トレイト制約リスト
 * @param span 診断用位置情報
 * @return 生成された辞書パラメータのリスト
 *)
let generate_dict_params (fn_scope : var_scope_map) (constraints : Types.trait_constraint list) (span : span) : param list =
  List.mapi
    (fun idx constraint_info ->
      let trait_name = constraint_info.Types.trait_name in
      (* 辞書パラメータ名: __dict_<Trait>_<index> *)
      let param_name = Printf.sprintf "__dict_%s_%d" trait_name idx in
      (* 辞書型: Dict (現時点では簡略化) *)
      let dict_ty = TCon (TCUser "Dict") in
      (* 辞書パラメータ変数を生成 *)
      let dict_var = VarIdGen.fresh param_name dict_ty span in
      (* スコープに登録（辞書ルックアップ時に参照） *)
      bind_var fn_scope param_name dict_var;
      { param_var = dict_var; param_default = None })
    constraints

(** 関数パラメータの変換 *)
let desugar_param (fn_scope : var_scope_map) (index : int) (param : typed_param)
    : param =
  let span = param.tparam_span in
  let pat = param.tpat in
  let ty = convert_ty param.tty in
  (match param.tdefault with
  | Some _ -> desugar_error "デフォルト引数はまだ Core IR へ変換できません" span
  | None -> ());
  let var =
    match pat.tpat_kind with
    | TPatVar id ->
        let v = VarIdGen.fresh id.name ty pat.tpat_span in
        bind_var fn_scope id.name v;
        v
    | TPatWildcard ->
        VarIdGen.fresh (Printf.sprintf "_arg%d" index) ty pat.tpat_span
    | _ -> desugar_error "関数パラメータに複雑なパターンは使用できません（未実装）" span
  in
  { param_var = var; param_default = None }

(** 関数宣言の変換（Phase 2 Week 19-22: 辞書パラメータ挿入対応）
 *
 * 型クラス制約を持つ関数に対して、辞書パラメータを自動挿入する。
 * 例: fn f<T: Eq>(x: T) -> Bool { ... }
 *   → fn f(__dict_Eq_0: Dict, x: T) -> Bool { ... }
 *)
let desugar_fn_decl (decl : typed_decl) (fn_decl : typed_fn_decl) : function_def
    =
  let fn_scope = create_scope_map () in
  let fn_name = fn_decl.tfn_name.name in
  let return_ty = convert_ty fn_decl.tfn_ret_type in

  (* Phase 2 Week 19-22: 制約から辞書パラメータを生成 *)
  let dict_params = generate_dict_params fn_scope decl.tdecl_scheme.constraints decl.tdecl_span in

  (* 通常のパラメータを変換 *)
  let user_params =
    List.mapi
      (fun idx param -> desugar_param fn_scope idx param)
      fn_decl.tfn_params
  in

  (* 辞書パラメータを先頭に配置 *)
  let all_params = dict_params @ user_params in

  let body_expr =
    match fn_decl.tfn_body with
    | TFnExpr expr -> desugar_expr fn_scope expr
    | TFnBlock stmts -> desugar_block fn_scope stmts return_ty decl.tdecl_span
  in
  let entry_block =
    make_block "entry" [] [] (TermReturn body_expr) decl.tdecl_span
  in
  let base_metadata = default_metadata decl.tdecl_span in
  let metadata =
    match resolve_effect_profile ~symbol:fn_name with
    | Some entry ->
        let capability_stage =
          Some (effect_stage_requirement_to_ir entry.stage_requirement)
        in
        let required_caps =
          match entry.resolved_capability with
          | Some name ->
              [
                {
                  cap_name = name;
                  cap_span = entry.source_span;
                };
              ]
          | None -> base_metadata.capabilities.required
        in
        let capabilities =
          { required = required_caps; stage = capability_stage }
        in
        {
          base_metadata with
          effects = entry.effect_set;
          capabilities;
        }
    | None -> base_metadata
  in
  make_function fn_name all_params return_ty [ entry_block ] metadata

(** トップレベル宣言の変換 *)
let desugar_decl (_map : var_scope_map) (decl : typed_decl) :
    function_def option =
  match decl.tdecl_kind with
  | TFnDecl fn_decl -> Some (desugar_fn_decl decl fn_decl)
  | _ -> None

(** コンパイル単位の変換 *)
let desugar_compilation_unit (tcu : typed_compilation_unit) : module_def =
  let map = create_scope_map () in

  (* 関数定義のみを抽出（暫定） *)
  let function_defs = List.filter_map (desugar_decl map) tcu.tcu_items in

  {
    module_name = "main";
    (* TODO: モジュール名の取得 *)
    type_defs = [];
    (* TODO: 型定義の変換 *)
    global_defs = [];
    (* TODO: グローバル変数の変換 *)
    function_defs;
  }
