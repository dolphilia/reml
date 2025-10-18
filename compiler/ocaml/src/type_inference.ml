(* Type_inference — Type Inference Engine for Reml (Phase 2)
 *
 * このファイルはHindley-Milner型推論エンジンの実装を提供する。
 * 仕様書 1-2 §C（型推論）に従い、制約ベース推論を実装する。
 *
 * 設計原則:
 * - 制約収集と制約解決の分離
 * - let多相の一般化とインスタンス化
 * - 型注釈の統合
 *)

open Types
open Type_env
open Constraint
open Constraint_solver
open Type_error
open Ast
open Typed_ast
module Ffi = Ffi_contract

type config = { effect_context : Type_inference_effect.runtime_stage }

let make_config ?effect_context () =
  {
    effect_context =
      (match effect_context with
      | Some ctx -> ctx
      | None -> Type_inference_effect.runtime_stage_default);
  }

let default_config = make_config ()
let current_config : config ref = ref default_config

(* ========== 効果解析ヘルパー ========== *)

module Effect_analysis = struct
  open Effect_profile
  module StringSet = Set.Make (String)

  let normalize_effect_name name = String.lowercase_ascii (String.trim name)
  let span_is_dummy (span : Ast.span) = span.start <= 0 && span.end_ <= 0

  let add_tag tags name span =
    let normalized = normalize_effect_name name in
    if
      List.exists
        (fun (tag : Effect_profile.tag) ->
          String.equal (normalize_effect_name tag.effect_name) normalized)
        tags
    then tags
    else tags @ [ { effect_name = name; effect_span = span } ]

  let rec collect_expr tags (expr : typed_expr) =
    match expr.texpr_kind with
    | TLiteral _ -> tags
    | TVar _ -> tags
    | TModulePath _ -> tags
    | TCall (fn_expr, args) ->
        let tags = collect_expr tags fn_expr in
        let tags = List.fold_left collect_arg tags args in
        let callee_name =
          match fn_expr.texpr_kind with
          | TVar (id, _) -> Some id.name
          | TModulePath (_, id) -> Some id.name
          | _ -> None
        in
        let tags =
          match callee_name with
          | Some name when String.equal (normalize_effect_name name) "panic" ->
              add_tag tags "panic" expr.texpr_span
          | _ -> tags
        in
        tags
    | TLambda (params, _, body) ->
        let tags =
          List.fold_left
            (fun acc param ->
              match param.tdefault with
              | Some default -> collect_expr acc default
              | None -> acc)
            tags params
        in
        collect_expr tags body
    | TPipe (lhs, rhs) | TBinary (_, lhs, rhs) ->
        collect_expr (collect_expr tags lhs) rhs
    | TUnary (_, operand)
    | TFieldAccess (operand, _)
    | TTupleAccess (operand, _)
    | TPropagate operand
    | TUnsafe operand ->
        collect_expr tags operand
    | TIndex (lhs, rhs) | TAssign (lhs, rhs) ->
        collect_expr (collect_expr tags lhs) rhs
    | TIf (cond, then_branch, else_branch) -> (
        let tags = collect_expr tags cond in
        let tags = collect_expr tags then_branch in
        match else_branch with
        | Some expr -> collect_expr tags expr
        | None -> tags)
    | TMatch (scrutinee, arms) ->
        let tags = collect_expr tags scrutinee in
        List.fold_left collect_match_arm tags arms
    | TWhile (cond, body) -> collect_expr (collect_expr tags cond) body
    | TFor (_, iterable, body, _, _) ->
        collect_expr (collect_expr tags iterable) body
    | TLoop body -> collect_expr tags body
    | TContinue -> tags
    | TBlock stmts -> collect_block tags stmts
    | TReturn expr_opt -> (
        match expr_opt with None -> tags | Some expr -> collect_expr tags expr)
    | TDefer expr -> collect_expr tags expr

  and collect_arg tags = function
    | TPosArg expr -> collect_expr tags expr
    | TNamedArg (_, expr) -> collect_expr tags expr

  and collect_match_arm tags arm =
    let tags =
      match arm.tarm_guard with
      | Some guard -> collect_expr tags guard
      | None -> tags
    in
    collect_expr tags arm.tarm_body

  and collect_block tags stmts = List.fold_left collect_stmt tags stmts

  and collect_stmt tags = function
    | TDeclStmt decl -> collect_decl tags decl
    | TExprStmt expr -> collect_expr tags expr
    | TAssignStmt (lhs, rhs) -> collect_expr (collect_expr tags lhs) rhs
    | TDeferStmt expr -> collect_expr tags expr

  and collect_decl tags decl =
    match decl.tdecl_kind with
    | TLetDecl (_, expr) | TVarDecl (_, expr) -> collect_expr tags expr
    | TFnDecl _ -> tags
    | _ -> tags

  let collect_from_fn_body (body : typed_fn_body) =
    match body with
    | TFnExpr expr -> collect_expr [] expr
    | TFnBlock stmts -> collect_block [] stmts

  let merge_usage_into_profile ~(fallback_span : Ast.span) (profile : profile)
      (residual_tags : tag list) =
    let effect_set =
      List.fold_left
        (fun acc tag -> add_residual tag acc)
        profile.effect_set residual_tags
    in
    let declared_names =
      List.fold_left
        (fun acc tag ->
          StringSet.add (normalize_effect_name tag.effect_name) acc)
        StringSet.empty effect_set.declared
    in
    let residual_leaks =
      effect_set.residual
      |> List.filter (fun tag ->
             let name = normalize_effect_name tag.effect_name in
             not (StringSet.mem name declared_names))
      |> List.map (fun tag ->
             let leak_origin =
               if span_is_dummy tag.effect_span then fallback_span
               else tag.effect_span
             in
             {
               leaked_tag =
                 {
                   effect_name = tag.effect_name;
                   effect_span = tag.effect_span;
                 };
               leak_origin;
             })
    in
    let diagnostic_payload =
      {
        invalid_attributes = profile.diagnostic_payload.invalid_attributes;
        residual_leaks;
      }
    in
    let profile = { profile with effect_set; diagnostic_payload } in
    (profile, residual_leaks)
end

(* ========== グローバル状態: impl レジストリ ========== *)

(** impl 宣言のグローバルレジストリ
 *
 * Phase 2 Week 23-24: モジュールレベルのrefで保持し、
 * 関数シグネチャの変更を最小化する。
 *
 * 型推論エンジン全体でこのレジストリを共有し、
 * impl 宣言の登録と制約解決での参照を実現する。
 *)
let global_impl_registry : Impl_registry.impl_registry ref =
  ref (Impl_registry.empty ())

type ffi_bridge_snapshot = {
  normalized : Ffi.normalized_contract;
  param_types : Types.ty list;
  return_type : Types.ty;
}

let ffi_bridge_snapshots : ffi_bridge_snapshot list ref = ref []

let reset_ffi_bridge_snapshots () = ffi_bridge_snapshots := []

let record_ffi_bridge_snapshot (snapshot : ffi_bridge_snapshot) =
  ffi_bridge_snapshots := snapshot :: !ffi_bridge_snapshots

let current_ffi_bridge_snapshots () = List.rev !ffi_bridge_snapshots

(** レジストリのリセット（テスト用） *)
let reset_impl_registry () =
  global_impl_registry := Impl_registry.empty ();
  Monomorph_registry.reset ();
  reset_effect_constraints ();
  reset_ffi_bridge_snapshots ()

(** レジストリの取得 *)
let get_impl_registry () = !global_impl_registry

(** impl 情報をレジストリに登録 *)
let register_impl (impl_info : Impl_registry.impl_info) : unit =
  global_impl_registry := Impl_registry.register impl_info !global_impl_registry

(** モノモルフィゼーション PoC 用に解決済みインスタンスを記録 *)
let record_monomorph_instances (registry : Impl_registry.impl_registry)
    (dict_refs : dict_ref list) =
  List.iter
    (function
      | DictImplicit (trait_name, ty_args) ->
          let impl_ty =
            match ty_args with head :: _ -> head | [] -> ty_unit
          in
          let constraint_ =
            {
              trait_name;
              type_args = ty_args;
              constraint_span = Ast.dummy_span;
            }
          in
          let methods =
            match Impl_registry.find_matching_impls constraint_ registry with
            | impl :: _ when impl.methods <> [] -> impl.methods
            | _ ->
                Type_env.Monomorph_registry.builtin_methods trait_name impl_ty
          in
          Type_env.Monomorph_registry.record
            Type_env.Monomorph_registry.
              { trait_name; type_args = ty_args; methods }
      | DictParam _ | DictLocal _ -> ())
    dict_refs

(* ========== 型注釈の変換 ========== *)

(** AST型注釈をTypes.tyに変換
 *
 * Phase 2では基本型のみサポート（型クラスは後半で実装）
 *)
let rec convert_type_annot (tannot : type_annot) : ty =
  match tannot.ty_kind with
  | TyIdent id -> (
      (* 組み込み型の識別 *)
      match id.name with
      (* 整数型 *)
      | "i8" -> ty_i8
      | "i16" -> ty_i16
      | "i32" -> ty_i32
      | "i64" -> ty_i64
      | "isize" -> ty_isize
      | "u8" -> ty_u8
      | "u16" -> ty_u16
      | "u32" -> ty_u32
      | "u64" -> ty_u64
      | "usize" -> ty_usize
      (* 浮動小数型 *)
      | "f32" -> ty_f64
      | "f64" -> ty_f64
      (* 基本型 *)
      | "Bool" -> ty_bool
      | "Char" -> ty_char
      | "String" -> ty_string
      | "()" -> ty_unit
      | "Never" -> ty_never
      (* ユーザ定義型 *)
      | name -> TCon (TCUser name))
  | TyApp (base_id, args) ->
      (* 型適用: Vec<T>, Option<T> など *)
      let base_ty = TCon (TCUser base_id.name) in
      List.fold_left
        (fun acc arg -> TApp (acc, convert_type_annot arg))
        base_ty args
  | TyTuple tys ->
      (* タプル型 *)
      TTuple (List.map convert_type_annot tys)
  | TyRecord fields ->
      (* レコード型 *)
      TRecord
        (List.map
           (fun (field_id, field_ty) ->
             (field_id.name, convert_type_annot field_ty))
           fields)
  | TyFn (arg_tys, ret_ty) ->
      (* 関数型: (A, B) -> C を A -> B -> C に変換 *)
      List.fold_right
        (fun arg_ty acc -> TArrow (convert_type_annot arg_ty, acc))
        arg_tys
        (convert_type_annot ret_ty)

let check_extern_bridge_contract ~(block_target : string option)
    (item : extern_item) : (Ffi.normalized_contract, type_error) result =
  let contract =
    Ffi.bridge_contract ?block_target
      ~extern_name:item.extern_sig.sig_name.name
      ~source_span:item.extern_sig.sig_name.span
      ~metadata:item.extern_metadata ()
  in
  let normalized = Ffi.normalize_contract contract in
  match normalized.extern_symbol with
  | None -> Error (ffi_contract_symbol_missing_error normalized)
  | Some _ ->
      if not (Ffi.ownership_supported normalized.ownership_kind) then
        Error (ffi_contract_ownership_mismatch_error normalized)
      else if not (Ffi.abi_supported normalized.abi_kind) then
        Error (ffi_contract_unsupported_abi_error normalized)
      else
        (match normalized.expected_abi with
        | Some expected when expected <> normalized.abi_kind ->
            Error (ffi_contract_unsupported_abi_error normalized)
        | _ -> Ok normalized)

(* ========== 一般化とインスタンス化 ========== *)

(** 型の一般化: generalize(env, τ)
 *
 * 仕様書 1-2 §C.1: let束縛で自由型変数を量化
 *)
let generalize (env : env) (ty : ty) : constrained_scheme =
  let env_vars = ftv_env env in
  let ty_vars = ftv_ty ty in
  (* 環境に出現しない自由変数を量化 *)
  let quantified =
    List.filter
      (fun tv ->
        not (List.exists (fun env_tv -> env_tv.tv_id = tv.tv_id) env_vars))
      ty_vars
  in
  { quantified; constraints = []; body = ty }

(** 型スキームのインスタンス化: instantiate(scheme)
 *
 * 量化変数を新鮮な型変数で置き換え
 *)
let instantiate (scheme : constrained_scheme) : ty =
  if scheme.quantified = [] then scheme.body
  else
    (* 量化変数 → 新鮮な型変数のマッピングを作成 *)
    let subst =
      List.map
        (fun qtv ->
          let fresh_var = TypeVarGen.fresh None in
          (qtv, Types.TVar fresh_var))
        scheme.quantified
    in
    apply_subst subst scheme.body

(** 型付きパラメータに代入を適用するヘルパー
 *
 * 関数宣言やラムダ式の最終型が確定したあとで呼び出し、
 * パラメータの型情報（パターン含む）を最新化する。
 *)
let apply_subst_to_tparam (subst : substitution) (tparam : typed_param) :
    typed_param =
  let rec normalize_ty ty =
    let ty' = apply_subst subst ty in
    if type_equal ty ty' then ty' else normalize_ty ty'
  in
  let updated_ty = normalize_ty tparam.tty in
  let updated_bindings =
    List.map
      (fun (name, ty) -> (name, normalize_ty ty))
      tparam.tpat.tpat_bindings
  in
  let updated_pattern =
    {
      tparam.tpat with
      tpat_ty = normalize_ty tparam.tpat.tpat_ty;
      tpat_bindings = updated_bindings;
    }
  in
  { tparam with tty = updated_ty; tpat = updated_pattern }

let resolve_params ?(force_numeric_default = false) (subst : substitution)
    (_param_env : env) (params : typed_param list) : typed_param list =
  List.map
    (fun param ->
      let param_subst = apply_subst_to_tparam subst param in
      match param_subst.tpat.tpat_kind with
      | TPatVar _id ->
          let concrete_ty =
            match (param_subst.tty, force_numeric_default) with
            | TVar _, true -> ty_i64
            | ty, _ -> ty
          in
          let updated_bindings =
            List.map
              (fun (name, _) -> (name, concrete_ty))
              param_subst.tpat.tpat_bindings
          in
          {
            param_subst with
            tty = concrete_ty;
            tpat =
              {
                param_subst.tpat with
                tpat_ty = concrete_ty;
                tpat_bindings = updated_bindings;
              };
          }
      | _ -> param_subst)
    params

(* ========== 型推論エンジン ========== *)

type infer_result = typed_expr * ty * substitution * trait_constraint list
(** 推論結果: 型付き式、推論された型、代入、トレイト制約のリスト
 *
 * Phase 2 Week 21-22: 制約リストを追加し、型クラスサポートの基盤を整備
 * - 型付き式: 型推論の結果生成される型付きAST
 * - 推論された型: 式の型
 * - 代入: 型変数への代入（単一化の結果）
 * - トレイト制約: 式の評価に必要なトレイト実装の制約
 *)

(** let* 演算子（Result モナド） *)
let ( let* ) = Result.bind

(** 文脈依存の単一化ヘルパー
 *
 * 汎用的な `UnificationFailure` を、利用箇所に応じた専用エラーへ
 * 変換するための補助関数群。technical-debt.md §7 で指摘された
 * 「型エラー生成順序の問題」への対処として導入する。
 *)

let is_function_type = function TArrow _ -> true | _ -> false

let unify_as_bool (subst : substitution) (ty : ty) (span : span) :
    (substitution, type_error) result =
  let ty' = apply_subst subst ty in
  if type_equal ty' ty_bool then Ok subst
  else
    match ty' with
    | TVar _ -> unify subst ty' ty_bool span
    | _ -> (
        match unify subst ty' ty_bool span with
        | Ok s -> Ok s
        | Error (UnificationFailure _) ->
            Error (condition_not_bool_error ty' span)
        | Error e -> Error e)

let unify_branch_types (subst : substitution) (then_ty : ty) (else_ty : ty)
    (span : span) : (substitution, type_error) result =
  let then' = apply_subst subst then_ty in
  let else' = apply_subst subst else_ty in
  match unify subst then' else' span with
  | Ok s -> Ok s
  | Error (UnificationFailure _) ->
      Error (branch_type_mismatch_error then' else' span)
  | Error e -> Error e

let unify_as_function (subst : substitution) (fn_ty : ty) (expected_fn_ty : ty)
    (span : span) : (substitution, type_error) result =
  let fn_ty' = apply_subst subst fn_ty in
  let expected' = apply_subst subst expected_fn_ty in
  match fn_ty' with
  | TArrow _ | TVar _ -> (
      match unify subst fn_ty' expected' span with
      | Ok s -> Ok s
      | Error (UnificationFailure (lhs, rhs, span')) ->
          let lhs_ty = apply_subst subst lhs in
          let rhs_ty = apply_subst subst rhs in
          if (not (is_function_type lhs_ty)) && is_function_type rhs_ty then
            Error (not_a_function_error lhs_ty span)
          else Error (UnificationFailure (lhs_ty, rhs_ty, span'))
      | Error e -> Error e)
  | _ -> Error (not_a_function_error fn_ty' span)

(* ========== 制約マージと収集ヘルパー（Phase 2 Week 19-22） ========== *)

(** 制約リストのマージ
 *
 * 複数の部分式から収集された制約を結合する。
 * Week 21-22: 複合式（Call, Let, If等）で制約を伝播する際に使用。
 *
 * @param cs1 第一の制約リスト
 * @param cs2 第二の制約リスト
 * @return マージされた制約リスト
 *)
let merge_constraints (cs1 : trait_constraint list)
    (cs2 : trait_constraint list) : trait_constraint list =
  cs1 @ cs2

(** 複数の制約リストをマージ
 *
 * リストのリストを平坦化する。
 *
 * @param css 制約リストのリスト
 * @return 全ての制約を含むリスト
 *)
let merge_constraints_many (css : trait_constraint list list) :
    trait_constraint list =
  List.concat css

(* ========== 制約収集ヘルパー（Phase 2 Week 19-20 準備） ========== *)

(** 演算子からトレイト名へのマッピング
 *
 * Week 21-22 実装時に使用予定。
 * 二項演算子に対応するトレイト名を返す。
 *
 * @param op 二項演算子
 * @return トレイト名（None の場合は組み込み演算）
 *)
let trait_name_of_binary_op (op : Ast.binary_op) : string option =
  match op with
  | Add -> Some "Add"
  | Sub -> Some "Sub"
  | Mul -> Some "Mul"
  | Div -> Some "Div"
  | Mod -> Some "Mod"
  | Pow -> Some "Pow"
  | Eq | Ne -> Some "Eq"
  | Lt | Le | Gt | Ge -> Some "Ord"
  | And | Or | PipeOp -> None (* 組み込み演算 *)

(** トレイト制約の生成（Week 21-22 実装予定）
 *
 * 演算子と型から trait_constraint を生成する。
 * 現在は未使用だが、将来の統合準備として定義。
 *
 * @param trait_name トレイト名
 * @param type_args 型引数リスト
 * @param span 制約が生じた位置
 * @return 生成されたトレイト制約
 *)
let make_trait_constraint (trait_name : string) (type_args : ty list)
    (span : span) : trait_constraint =
  { trait_name; type_args; constraint_span = span }

(** 二項演算子からトレイト制約を生成（Week 21-22 実装予定）
 *
 * 二項演算子の型推論時に呼び出し、対応するトレイト制約を生成する。
 * Week 21-22 実装時には infer_binary_op から呼び出される予定。
 *
 * @param op 二項演算子
 * @param ty1 左辺の型
 * @param ty2 右辺の型
 * @param span 演算子の位置
 * @return 生成されたトレイト制約のリスト
 *)
let collect_binary_op_constraints (op : Ast.binary_op) (ty1 : ty) (_ty2 : ty)
    (span : span) : trait_constraint list =
  match trait_name_of_binary_op op with
  | Some trait_name ->
      (* 例: Add<i64, i64, i64> の場合、現在は ty1 のみを型引数とする簡易実装 *)
      [ make_trait_constraint trait_name [ ty1 ] span ]
  | None ->
      (* 組み込み演算（&&, ||, |>）には制約なし *)
      []

(* ========== 制約解決の統合（Phase 2 Week 18-19） ========== *)

(** 制約解決エラーを型エラーに変換
 *
 * Constraint_solver のエラーを Type_error に変換して、
 * 統一的な診断システムで報告できるようにする
 *)
let constraint_error_to_type_error (err : Constraint_solver.constraint_error) :
    type_error =
  match err.reason with
  | Constraint_solver.NoImpl ->
      let reason = "この型に対するトレイト実装が見つかりません" in
      TraitConstraintFailure
        {
          trait_name = err.trait_name;
          type_args = err.type_args;
          reason;
          span = err.span;
          effect_stage = None;
        }
  | Constraint_solver.AmbiguousImpl dict_refs ->
      let candidates =
        List.map Constraint_solver.string_of_dict_ref dict_refs
      in
      AmbiguousTraitImpl
        {
          trait_name = err.trait_name;
          type_args = err.type_args;
          candidates;
          span = err.span;
        }
  | Constraint_solver.CyclicConstraint cycle ->
      let cycle_names =
        List.map
          (fun (c : trait_constraint) ->
            let type_args_str =
              String.concat ", " (List.map string_of_ty c.type_args)
            in
            Printf.sprintf "%s<%s>" c.trait_name type_args_str)
          cycle
      in
      CyclicTraitConstraint { cycle = cycle_names; span = err.span }
  | Constraint_solver.StageMismatch
      {
        required;
        actual;
        capability;
        iterator_kind;
        iterator_source;
        provider;
        manifest_path;
        stage_trace = _;
      } ->
      let required_desc, required_stage_value =
        match required with
        | Constraint_solver.IteratorStageExact stage ->
            (Printf.sprintf "Stage = %s" stage, stage)
        | Constraint_solver.IteratorStageAtLeast stage ->
            (Printf.sprintf "Stage >= %s" stage, stage)
      in
      let capability_label =
        match capability with Some id -> id | None -> "<未指定>"
      in
      let actual_stage_label =
        match actual with Some stage -> stage | None -> "<未検証>"
      in
      let reason =
        Printf.sprintf "Capability %s の Stage (%s) が要求条件 %s を満たしていません"
          capability_label actual_stage_label required_desc
      in
      let iterator_requirement =
        match required with
        | Constraint_solver.IteratorStageExact stage ->
            Printf.sprintf "exact:%s" stage
        | Constraint_solver.IteratorStageAtLeast stage ->
            Printf.sprintf "at_least:%s" stage
      in
      let iterator_kind_label =
        match iterator_kind with
        | Some Constraint_solver.IteratorArrayLike -> Some "array_like"
        | Some Constraint_solver.IteratorCoreIter -> Some "core_iter"
        | Some Constraint_solver.IteratorOptionLike -> Some "option_like"
        | Some Constraint_solver.IteratorResultLike -> Some "result_like"
        | Some (Constraint_solver.IteratorCustom name) ->
            Some (Printf.sprintf "custom:%s" name)
        | None -> None
      in
      let stage_extension =
        Some
          {
            required_stage = required_stage_value;
            iterator_required = iterator_requirement;
            actual_stage = actual;
            capability;
            provider;
            manifest_path;
            iterator_kind = iterator_kind_label;
            iterator_source;
            capability_metadata = None;
            residual = None;
            stage_trace = Effect_profile.stage_trace_empty;
          }
      in
      TraitConstraintFailure
        {
          trait_name = err.trait_name;
          type_args = err.type_args;
          reason;
          span = err.span;
          effect_stage = stage_extension;
        }
  | Constraint_solver.UnresolvedTypeVar tv ->
      let reason = Printf.sprintf "型変数 %s が未解決です" (string_of_type_var tv) in
      TraitConstraintFailure
        {
          trait_name = err.trait_name;
          type_args = err.type_args;
          reason;
          span = err.span;
          effect_stage = None;
        }

(** 制約リストを解決し、型エラーに変換
 *
 * Phase 2 Week 23-24 更新: レジストリパラメータを追加
 *
 * 制約解決器を呼び出し、成功時は辞書参照リストを、
 * 失敗時は型エラーを返す
 *)
let solve_trait_constraints (constraints : trait_constraint list) :
    (dict_ref list, type_error) result =
  let registry = get_impl_registry () in
  match Constraint_solver.solve_constraints registry constraints with
  | Ok dict_refs ->
      record_monomorph_instances registry dict_refs;
      Ok dict_refs
  | Error errors ->
      (* 複数のエラーがある場合は最初のエラーを返す *)
      Error (constraint_error_to_type_error (List.hd errors))

let solve_iterator_constraint (constraint_ : trait_constraint) :
    (iterator_dict_info, type_error) result =
  let registry = get_impl_registry () in
  match Constraint_solver.solve_iterator_dict registry constraint_ with
  | Ok info ->
      record_monomorph_instances registry [ info.dict_ref ];
      Ok info
  | Error err -> Error (constraint_error_to_type_error err)

let ensure_assignable env (texpr : typed_expr) span =
  match texpr.texpr_kind with
  | TVar (id, _) -> (
      match lookup_mutability id.name env with
      | Some Mutable -> Ok ()
      | Some Immutable -> Error (immutable_binding_error id.name span)
      | None -> Error (not_assignable_error span))
  | _ -> Error (not_assignable_error span)

(* ========== 推論コンテキスト（ループ深度追跡） ========== *)

type inference_ctx = { loop_depth : int }

let initial_ctx = { loop_depth = 0 }
let enter_loop ctx = { loop_depth = ctx.loop_depth + 1 }

(** 式の型推論: infer_expr(env, expr)
 *
 * Phase 2 Week 2-3: 基本的な式の推論を実装
 * Phase 2 Week 8: 複合リテラル（Tuple/Record）対応
 *)
let rec infer_expr ?(ctx = initial_ctx) (env : env) (expr : expr) :
    (infer_result, type_error) result =
  match expr.expr_kind with
  | Literal lit ->
      (* 複合リテラル（Tuple/Record）もサポート
       * 制約: リテラルは制約を生成しない（空リスト）
       *)
      let* ty, typed_lit, s = infer_literal ~ctx env lit expr.expr_span in
      let texpr = make_typed_expr (TLiteral typed_lit) ty expr.expr_span in
      Ok (texpr, ty, s, [])
  | Var id -> (
      (* 変数参照: 型環境から検索してインスタンス化
       * 制約: 変数自体は制約を生成しない（制約は変数の型スキームに含まれる）
       *)
      match lookup id.name env with
      | Some scheme ->
          let ty = instantiate scheme in
          let texpr =
            make_typed_expr (Typed_ast.TVar (id, scheme)) ty expr.expr_span
          in
          Ok (texpr, ty, empty_subst, [])
      | None -> Error (unbound_variable_error id.name expr.expr_span))
  | Call (fn_expr, args) ->
      (* 関数適用の型推論
       *
       * 1. 関数式を推論
       * 2. 引数を推論
       * 3. 関数型を構築して単一化
       * 4. 返り値型を返す
       * 制約: 関数と引数の制約をマージ
       *)
      let* tfn, fn_ty, s1, fn_constraints = infer_expr ~ctx env fn_expr in

      (* 引数を推論 *)
      let* targs, arg_tys, s2, arg_constraints =
        infer_args ~ctx (apply_subst_env s1 env) args s1
      in

      (* 返り値型用の新鮮な型変数 *)
      let ret_var = TypeVarGen.fresh None in
      let ret_ty = Types.TVar ret_var in

      (* 関数型を構築: arg1 -> arg2 -> ... -> ret *)
      let expected_fn_ty =
        List.fold_right (fun arg_ty acc -> TArrow (arg_ty, acc)) arg_tys ret_ty
      in

      (* 関数型と単一化 *)
      let* s3 = unify_as_function s2 fn_ty expected_fn_ty expr.expr_span in

      (* 返り値型に代入を適用 *)
      let final_ret_ty = apply_subst s3 ret_ty in
      let s_final = compose_subst s3 s2 in

      (* 制約をマージ *)
      let all_constraints = merge_constraints fn_constraints arg_constraints in

      (* 型付き式を構築 *)
      let texpr =
        make_typed_expr (TCall (tfn, targs)) final_ret_ty expr.expr_span
      in
      Ok (texpr, final_ret_ty, s_final, all_constraints)
  | Lambda (params, ret_ty_annot, body) ->
      (* ラムダ式の型推論
       *
       * 1. パラメータの型を決定（注釈あれば変換、なければ新鮮な型変数）
       * 2. 型環境にパラメータを追加
       * 3. 本体を推論
       * 4. 返り値型注釈があれば単一化
       * 5. 関数型を構築
       * 制約: 本体の制約を伝播
       *)
      (* パラメータの型推論 *)
      let* tparams, _param_tys, param_env, s1 =
        infer_params env params empty_subst
      in

      (* 本体式を推論 *)
      let env' = apply_subst_env s1 param_env in
      let* tbody, body_ty, s2, body_constraints = infer_expr ~ctx env' body in

      (* 返り値型注釈があれば単一化 *)
      let* final_body_ty, s3 =
        match ret_ty_annot with
        | Some annot ->
            let expected_ret_ty = convert_type_annot annot in
            let* s = unify s2 body_ty expected_ret_ty expr.expr_span in
            Ok (apply_subst s body_ty, s)
        | None -> Ok (body_ty, s2)
      in

      (* パラメータへ代入を適用 *)
      let tparams' = resolve_params s3 param_env tparams in
      let param_tys_resolved = List.map (fun p -> p.tty) tparams' in

      (* 関数型を構築: param1 -> param2 -> ... -> body_ty *)
      let fn_ty =
        List.fold_right
          (fun param_ty acc -> TArrow (param_ty, acc))
          param_tys_resolved final_body_ty
      in

      (* 型付き式を構築 *)
      let ret_ty_opt = Option.map convert_type_annot ret_ty_annot in
      let texpr =
        make_typed_expr
          (TLambda (tparams', ret_ty_opt, tbody))
          fn_ty expr.expr_span
      in
      Ok (texpr, fn_ty, s3, body_constraints)
  | Binary (op, e1, e2) ->
      (* 二項演算の型推論
       *
       * 仕様書 1-2 §C.5: 演算子はトレイトで解決
       * Phase 2 MVP: 基本演算子の組み込みトレイトのみ（i64, f64, Bool, String対応）
       *
       * 1. 左辺と右辺を推論
       * 2. 演算子に応じた型制約を生成
       * 3. 返り値型を決定
       * 制約: 左辺・右辺の制約をマージし、演算子から生成された制約を追加
       *)
      (* 左辺を推論 *)
      let* te1, ty1, s1, constraints1 = infer_expr ~ctx env e1 in

      (* 右辺を推論 *)
      let env' = apply_subst_env s1 env in
      let* te2, ty2, s2, constraints2 = infer_expr ~ctx env' e2 in

      (* 演算子に応じた型推論と制約生成（Week 20-21 完全実装） *)
      let s_combined = compose_subst s2 s1 in
      let* ret_ty, s3, op_constraints =
        infer_binary_op op ty1 ty2 s_combined e1.expr_span e2.expr_span
      in
      let s_final = compose_subst s3 s_combined in

      (* 全制約をマージ（左辺・右辺・演算子の制約を統合） *)
      let all_constraints =
        merge_constraints_many [ constraints1; constraints2; op_constraints ]
      in

      (* 型付き式を構築 *)
      let texpr =
        make_typed_expr (TBinary (op, te1, te2)) ret_ty expr.expr_span
      in
      Ok (texpr, ret_ty, s_final, all_constraints)
  | If (cond, then_e, else_e) ->
      (* if式の型推論
       *
       * 1. 条件式を推論してBool型と単一化
       * 2. then分岐を推論
       * 3. else分岐を推論してthen分岐と統一
       * 制約: 条件式、then分岐、else分岐の制約をマージ
       *)
      (* 条件式を推論 *)
      let* tcond, cond_ty, s1, cond_constraints = infer_expr ~ctx env cond in

      (* 条件式をBool型と単一化 *)
      let* s2 = unify_as_bool s1 cond_ty cond.expr_span in

      (* then分岐を推論 *)
      let env' = apply_subst_env s2 env in
      let* tthen, then_ty, s3, then_constraints = infer_expr ~ctx env' then_e in

      (* else分岐を推論 *)
      let* telse_opt, final_ty, s4, else_constraints =
        match else_e with
        | Some else_expr ->
            let env'' = apply_subst_env s3 env' in
            let* telse, else_ty, s, else_cs = infer_expr ~ctx env'' else_expr in
            (* then分岐とelse分岐の型を統一 *)
            let* s' =
              unify_branch_types s then_ty else_ty else_expr.expr_span
            in
            let unified_ty = apply_subst s' then_ty in
            Ok (Some telse, unified_ty, s', else_cs)
        | None ->
            (* else分岐がない場合、then分岐はUnit型でなければならない *)
            let* s = unify s3 then_ty ty_unit then_e.expr_span in
            Ok (None, ty_unit, s, [])
      in

      (* 全制約をマージ *)
      let all_constraints =
        merge_constraints_many
          [ cond_constraints; then_constraints; else_constraints ]
      in

      (* 型付き式を構築 *)
      let texpr =
        make_typed_expr (TIf (tcond, tthen, telse_opt)) final_ty expr.expr_span
      in
      Ok (texpr, final_ty, s4, all_constraints)
  | Match (scrutinee, arms) ->
      (* match式の型推論
       *
       * 1. スクラティニー（検査対象）式の型推論
       * 2. 各アームのパターン推論と型環境更新
       * 3. ガード条件の型推論（Bool型）
       * 4. 各アームのボディを推論
       * 5. 全アームの型を統一
       *)
      (* スクラティニー式を推論
       * 制約: スクラティニーと全アームの制約をマージ
       *)
      let* tscrutinee, scrutinee_ty, s1, scrutinee_constraints =
        infer_expr ~ctx env scrutinee
      in

      (* アームが空の場合はエラー *)
      if arms = [] then Error (EmptyMatch expr.expr_span)
      else
        (* 最初のアームを処理 *)
        let first_arm = List.hd arms in
        let* first_tarm, first_body_ty, s2, first_constraints =
          infer_match_arm ~ctx (apply_subst_env s1 env) first_arm scrutinee_ty
            s1
        in

        (* 残りのアームを処理して型を統一 *)
        let* rest_tarms, final_ty, s_final, arms_constraints =
          List.fold_left
            (fun acc arm ->
              match acc with
              | Error e -> Error e
              | Ok (tarms, unified_ty, s_acc, cs_acc) ->
                  let env' = apply_subst_env s_acc env in
                  let scrutinee_ty' = apply_subst s_acc scrutinee_ty in
                  let* tarm, arm_body_ty, s_new, arm_cs =
                    infer_match_arm ~ctx env' arm scrutinee_ty' s_acc
                  in

                  (* 型を統一 *)
                  let* s_unified =
                    unify_branch_types s_new unified_ty arm_body_ty arm.arm_span
                  in
                  let new_unified_ty = apply_subst s_unified unified_ty in
                  let merged_cs = merge_constraints cs_acc arm_cs in

                  Ok (tarms @ [ tarm ], new_unified_ty, s_unified, merged_cs))
            (Ok ([ first_tarm ], first_body_ty, s2, first_constraints))
            (List.tl arms)
        in

        (* 全制約をマージ *)
        let all_constraints =
          merge_constraints scrutinee_constraints arms_constraints
        in

        (* 型付き式を構築 *)
        let texpr =
          make_typed_expr
            (TMatch (tscrutinee, rest_tarms))
            final_ty expr.expr_span
        in
        Ok (texpr, final_ty, s_final, all_constraints)
  | Block stmts ->
      (* ブロック式: Phase 2 Week 5 で実装
       *
       * 仕様書 1-1 §C.6: ブロックの最後の式が値
       * - 空のブロック → Unit型
       * - 文のリストを順次処理し、型環境を更新
       * - 最後の要素が式文なら、その式の型がブロック全体の型
       * - 最後の要素が宣言文・代入文・defer文なら Unit型
       *)
      if stmts = [] then
        (* 空のブロック: 制約なし *)
        let texpr = make_typed_expr (TBlock []) ty_unit expr.expr_span in
        Ok (texpr, ty_unit, empty_subst, [])
      else
        (* 文を順次処理: 制約を収集 *)
        let* tstmts, final_ty, s_final, constraints =
          infer_stmts ~ctx env stmts empty_subst
        in
        let texpr = make_typed_expr (TBlock tstmts) final_ty expr.expr_span in
        Ok (texpr, final_ty, s_final, constraints)
  | While (cond, body) ->
      (* while式の型推論
       *
       * 1. 条件式を推論してBool型と単一化
       * 2. ボディを推論
       * 3. while式全体の型はUnit（ループは値を返さない）
       * 制約: 条件式とボディの制約をマージ
       *)
      (* 条件式を推論 *)
      let* tcond, cond_ty, s1, cond_constraints = infer_expr ~ctx env cond in

      (* 条件式をBool型と単一化 *)
      let* s2 = unify_as_bool s1 cond_ty cond.expr_span in

      (* ボディを推論 *)
      let env' = apply_subst_env s2 env in
      let loop_ctx = enter_loop ctx in
      let* tbody, _body_ty, s3, body_constraints =
        infer_expr ~ctx:loop_ctx env' body
      in

      (* 全制約をマージ *)
      let all_constraints =
        merge_constraints cond_constraints body_constraints
      in

      (* 型付き式を構築 (while式はUnit型を返す) *)
      let texpr =
        make_typed_expr (TWhile (tcond, tbody)) ty_unit expr.expr_span
      in
      Ok (texpr, ty_unit, s3, all_constraints)
  | For (pat, source, body) ->
      (* for式の型推論
       *
       * 1. ソース式（イテレート対象）を推論
       * 2. Iterator トレイト制約を生成
       * 3. パターン変数を型環境に追加してボディを推論
       * 4. Iterator 辞書を解決して型を確定
       * 5. for式全体の型はUnit
       * 制約: ソース式とボディの制約、Iterator 制約をマージ
       *)
      (* ソース式を推論 *)
      let* tsource, source_ty, s1, source_constraints =
        infer_expr ~ctx env source
      in
      let elem_ty = Types.TVar (TypeVarGen.fresh None) in
      (* パターンを推論（Iterator の要素型を期待） *)
      let env_after_source = apply_subst_env s1 env in
      let* tpat, pat_env = infer_pattern ~ctx env_after_source pat elem_ty in
      (* ボディを推論 *)
      let env_for_body = apply_subst_env s1 pat_env in
      let loop_ctx = enter_loop ctx in
      let* tbody, _body_ty, s3, body_constraints =
        infer_expr ~ctx:loop_ctx env_for_body body
      in
      (* 追加の代入を適用 *)
      let s_acc = compose_subst s3 s1 in
      let source_ty_resolved = apply_subst s_acc source_ty in
      let elem_ty_resolved = apply_subst s_acc elem_ty in
      let iterator_constraint =
        make_trait_constraint "Iterator"
          [ source_ty_resolved; elem_ty_resolved ]
          source.expr_span
      in
      (* Iterator 辞書を解決 *)
      let* iterator_info = solve_iterator_constraint iterator_constraint in
      (* 辞書から要素型を取得し、型変数と単一化する *)
      let* s_final =
        unify s_acc elem_ty iterator_info.element_ty source.expr_span
      in
      let source_ty_final = apply_subst s_final source_ty in
      let elem_ty_final = apply_subst s_final elem_ty in
      let iterator_constraint_final =
        {
          iterator_constraint with
          type_args = [ source_ty_final; elem_ty_final ];
        }
      in
      let iterator_dict =
        match iterator_info.dict_ref with
        | DictImplicit (trait, tys) ->
            DictImplicit (trait, List.map (apply_subst s_final) tys)
        | dict -> dict
      in
      let iterator_info_final =
        {
          iterator_info with
          dict_ref = iterator_dict;
          source_ty = apply_subst s_final iterator_info.source_ty;
          element_ty = apply_subst s_final iterator_info.element_ty;
        }
      in
      let all_constraints =
        merge_constraints_many
          [
            source_constraints; body_constraints; [ iterator_constraint_final ];
          ]
      in
      let texpr =
        make_typed_expr
          (TFor (tpat, tsource, tbody, iterator_dict, Some iterator_info_final))
          ty_unit expr.expr_span
      in
      Ok (texpr, ty_unit, s_final, all_constraints)
  | Loop body ->
      (* loop式の型推論
       *
       * 1. ボディを推論
       * 2. loop式全体の型はUnit（無限ループは値を返さない）
       * 制約: ボディの制約を伝播
       *)
      let loop_ctx = enter_loop ctx in
      let* tbody, _body_ty, s1, body_constraints =
        infer_expr ~ctx:loop_ctx env body
      in

      (* 型付き式を構築 (loop式はUnit型を返す) *)
      let texpr = make_typed_expr (TLoop tbody) ty_unit expr.expr_span in
      Ok (texpr, ty_unit, s1, body_constraints)
  | Continue ->
      (* continue式はUnit型を返し、ループ継続を示す制御フロー操作 *)
      if ctx.loop_depth = 0 then
        Error (continue_outside_loop_error expr.expr_span)
      else
        let texpr = make_typed_expr TContinue ty_unit expr.expr_span in
        Ok (texpr, ty_unit, empty_subst, [])
  | Unsafe body ->
      (* unsafe式の型推論
       *
       * 1. ボディを推論
       * 2. unsafe式全体の型はボディの型
       * 制約: ボディの制約を伝播
       *)
      let* tbody, body_ty, s1, body_constraints = infer_expr ~ctx env body in

      (* 型付き式を構築 *)
      let texpr = make_typed_expr (TUnsafe tbody) body_ty expr.expr_span in
      Ok (texpr, body_ty, s1, body_constraints)
  | Return ret_expr_opt ->
      (* return式の型推論
       *
       * 1. 返り値式があれば推論
       * 2. return式全体の型はUnit（制御フローを変更するため）
       * 制約: 返り値式の制約を伝播
       *)
      let* tret_expr_opt, s1, ret_constraints =
        match ret_expr_opt with
        | Some ret_expr ->
            let* tret_expr, _ret_ty, s, ret_cs = infer_expr ~ctx env ret_expr in
            Ok (Some tret_expr, s, ret_cs)
        | None -> Ok (None, empty_subst, [])
      in

      (* 型付き式を構築 (return式はUnit型を返す) *)
      let texpr =
        make_typed_expr (TReturn tret_expr_opt) ty_unit expr.expr_span
      in
      Ok (texpr, ty_unit, s1, ret_constraints)
  | Defer deferred_expr ->
      (* defer式の型推論
       *
       * 1. 遅延実行される式を推論
       * 2. defer式全体の型はUnit
       * 制約: 遅延実行式の制約を伝播
       *)
      let* tdeferred_expr, _deferred_ty, s1, deferred_constraints =
        infer_expr ~ctx env deferred_expr
      in

      (* 型付き式を構築 (defer式はUnit型を返す) *)
      let texpr =
        make_typed_expr (TDefer tdeferred_expr) ty_unit expr.expr_span
      in
      Ok (texpr, ty_unit, s1, deferred_constraints)
  | Assign (lhs, rhs) ->
      (* 代入式の型推論
       *
       * 1. 左辺（代入先）を推論
       * 2. 右辺（代入元）を推論
       * 3. 左辺と右辺の型を単一化
       * 4. 代入式全体の型はUnit
       * 制約: 左辺と右辺の制約をマージ
       *)
      let* tlhs, lhs_ty, s1, lhs_constraints = infer_expr ~ctx env lhs in
      let* () = ensure_assignable env tlhs lhs.expr_span in

      let env' = apply_subst_env s1 env in
      let* trhs, rhs_ty, s2, rhs_constraints = infer_expr ~ctx env' rhs in

      (* 左辺と右辺の型を単一化 *)
      let* s3 = unify s2 lhs_ty rhs_ty rhs.expr_span in

      (* 全制約をマージ *)
      let all_constraints = merge_constraints lhs_constraints rhs_constraints in

      (* 型付き式を構築 (代入式はUnit型を返す) *)
      let texpr =
        make_typed_expr (TAssign (tlhs, trhs)) ty_unit expr.expr_span
      in
      Ok (texpr, ty_unit, s3, all_constraints)
  | _ ->
      (* その他の式は Phase 2 で順次実装 *)
      failwith "Expression not yet implemented"

(** 引数リストの型推論
 *
 * 位置引数と名前付き引数の両方をサポート
 * Phase 2 Week 21-22: 制約リストを収集
 *)
and infer_args ?(ctx = initial_ctx) (env : env) (args : arg list)
    (subst : substitution) :
    ( typed_arg list * ty list * substitution * trait_constraint list,
      type_error )
    result =
  List.fold_left
    (fun acc arg ->
      match acc with
      | Error e -> Error e
      | Ok (targs, arg_tys, s, constraints) -> (
          let env' = apply_subst_env s env in
          match arg with
          | PosArg expr -> (
              match infer_expr ~ctx env' expr with
              | Ok (texpr, ty, s', expr_constraints) ->
                  let s'' = compose_subst s' s in
                  let all_constraints =
                    merge_constraints constraints expr_constraints
                  in
                  Ok
                    ( targs @ [ TPosArg texpr ],
                      arg_tys @ [ ty ],
                      s'',
                      all_constraints )
              | Error e -> Error e)
          | NamedArg (id, expr) -> (
              match infer_expr ~ctx env' expr with
              | Ok (texpr, ty, s', expr_constraints) ->
                  let s'' = compose_subst s' s in
                  let all_constraints =
                    merge_constraints constraints expr_constraints
                  in
                  Ok
                    ( targs @ [ TNamedArg (id, texpr) ],
                      arg_tys @ [ ty ],
                      s'',
                      all_constraints )
              | Error e -> Error e)))
    (Ok ([], [], subst, []))
    args

(** パラメータリストの型推論
 *
 * パラメータの型を決定し、型環境に追加
 *)
and infer_params (env : env) (params : param list) (subst : substitution) :
    (typed_param list * ty list * env * substitution, type_error) result =
  List.fold_left
    (fun acc param ->
      match acc with
      | Error e -> Error e
      | Ok (tparams, param_tys, param_env, s) ->
          (* パラメータの型を決定 *)
          let param_ty =
            match param.ty with
            | Some annot -> convert_type_annot annot
            | None -> Types.TVar (TypeVarGen.fresh None)
          in

          (* パターンから変数名を抽出（簡易版：変数パターンのみ）*)
          let param_name, param_id =
            match param.pat.pat_kind with
            | PatVar id -> (id.name, id)
            | _ -> failwith "Complex parameter patterns not yet implemented"
          in

          (* デフォルト値があれば推論（Phase 2後半で実装）*)
          let tdefault =
            match param.default with
            | Some _expr -> failwith "Default parameters not yet implemented"
            | None -> None
          in

          (* 型付きパターンを構築 *)
          let tpat =
            make_typed_pattern (TPatVar param_id) param_ty
              [ (param_name, param_ty) ]
              param.param_span
          in

          (* 型付きパラメータを構築 *)
          let tparam =
            { tpat; tty = param_ty; tdefault; tparam_span = param.param_span }
          in

          (* 型環境に追加 *)
          let param_env' =
            extend param_name
              (scheme_to_constrained (mono_scheme param_ty))
              param_env
          in

          Ok (tparams @ [ tparam ], param_tys @ [ param_ty ], param_env', s))
    (Ok ([], [], env, subst))
    params

(** タプル要素の型推論
 *
 * Phase 2 Week 8: タプルリテラルの各要素を推論
 * Phase 2 Week 21-22: 制約リストを収集
 *
 * @param env 型環境
 * @param exprs タプル要素の式リスト
 * @return (型付き式リスト, 型リスト, 代入, 制約リスト)
 *)
and infer_tuple_elements ?(ctx = initial_ctx) (env : env) (exprs : expr list) :
    ( typed_expr list * ty list * substitution * trait_constraint list,
      type_error )
    result =
  List.fold_left
    (fun acc expr ->
      match acc with
      | Error e -> Error e
      | Ok (typed_exprs, tys, s, constraints) ->
          let env' = apply_subst_env s env in
          let* texpr, ty, s', expr_constraints = infer_expr ~ctx env' expr in
          let s'' = compose_subst s' s in
          let all_constraints =
            merge_constraints constraints expr_constraints
          in
          Ok (typed_exprs @ [ texpr ], tys @ [ ty ], s'', all_constraints))
    (Ok ([], [], empty_subst, []))
    exprs

(** レコードフィールドの型推論
 *
 * Phase 2 Week 8: レコードリテラルの各フィールドを推論
 * Phase 2 Week 21-22: 制約リストを収集
 *
 * @param env 型環境
 * @param fields レコードフィールド (識別子, 式) のリスト
 * @return (型付きフィールド, 型フィールド, 代入, 制約リスト)
 *)
and infer_record_fields ?(ctx = initial_ctx) (env : env)
    (fields : (ident * expr) list) :
    ( (ident * typed_expr) list
      * (string * ty) list
      * substitution
      * trait_constraint list,
      type_error )
    result =
  List.fold_left
    (fun acc (field_id, expr) ->
      match acc with
      | Error e -> Error e
      | Ok (typed_fields, field_tys, s, constraints) ->
          let env' = apply_subst_env s env in
          let* texpr, ty, s', expr_constraints = infer_expr ~ctx env' expr in
          let s'' = compose_subst s' s in
          let all_constraints =
            merge_constraints constraints expr_constraints
          in
          Ok
            ( typed_fields @ [ (field_id, texpr) ],
              field_tys @ [ (field_id.name, ty) ],
              s'',
              all_constraints ))
    (Ok ([], [], empty_subst, []))
    fields

(** リテラルの型推論
 *
 * Phase 2 Week 8: 複合リテラル（Tuple/Record）の推論を追加
 *)
and infer_literal ?(ctx = initial_ctx) (env : env) (lit : literal) (span : span)
    : (ty * literal * substitution, type_error) result =
  match lit with
  | Int (_, _) ->
      (* Phase 2: デフォルト i64 *)
      Ok (ty_i64, lit, empty_subst)
  | Float _ ->
      (* Phase 2: デフォルト f64 *)
      Ok (ty_f64, lit, empty_subst)
  | Char _ -> Ok (ty_char, lit, empty_subst)
  | String (_, _) -> Ok (ty_string, lit, empty_subst)
  | Bool _ -> Ok (ty_bool, lit, empty_subst)
  | Unit -> Ok (ty_unit, lit, empty_subst)
  | Tuple exprs ->
      (* タプルリテラル: (1, "hello", true)
       *
       * 1. 各要素を推論
       * 2. タプル型を構築
       *)
      if exprs = [] then (* 空タプル: Unit型、制約なし *)
        Ok (ty_unit, Unit, empty_subst)
      else
        (* タプル要素を推論: 制約は現在リテラルでは使用しない（将来の拡張用） *)
        let* _typed_exprs, elem_tys, s, _constraints =
          infer_tuple_elements ~ctx env exprs
        in
        let tuple_ty = TTuple elem_tys in
        (* 型付きリテラルを構築（元のliteralを返す） *)
        Ok (tuple_ty, lit, s)
  | Record fields ->
      (* レコードリテラル: { x: 42, y: "test" }
       *
       * 1. 各フィールドの式を推論
       * 2. レコード型を構築（構造的）
       *)
      (* レコードフィールドを推論: 制約は現在リテラルでは使用しない（将来の拡張用） *)
      let* _typed_fields, field_tys, s, _constraints =
        infer_record_fields ~ctx env fields
      in
      let record_ty = TRecord field_tys in
      (* 型付きリテラルを構築（元のliteralを返す） *)
      Ok (record_ty, lit, s)
  | Array _ ->
      (* 配列リテラルは Phase 2 後半で実装 *)
      Error (type_error_with_message "Array literals not yet implemented" span)

(** パターンの型推論: infer_pattern(env, pat, expected_ty)
 *
 * パターンを推論し、束縛変数を型環境に追加する
 *
 * @param env 現在の型環境
 * @param pat パターン（AST）
 * @param expected_ty パターンの期待される型
 * @return (型付きパターン, 束縛変数を追加した型環境)
 *)
and infer_pattern ?(ctx = initial_ctx) (env : env) (pat : pattern)
    (expected_ty : ty) : (typed_pattern * env, type_error) result =
  match pat.pat_kind with
  | PatLiteral lit ->
      (* リテラルパターン: リテラルの型と expected_ty を単一化 *)
      let* lit_ty, _typed_lit, _ = infer_literal ~ctx env lit pat.pat_span in
      let* _subst = unify empty_subst lit_ty expected_ty pat.pat_span in
      let tpat =
        make_typed_pattern (TPatLiteral lit) expected_ty [] pat.pat_span
      in
      Ok (tpat, env)
  | PatVar id ->
      (* 変数パターン: 変数を環境に追加 *)
      let bindings = [ (id.name, expected_ty) ] in
      let env' =
        extend id.name (scheme_to_constrained (mono_scheme expected_ty)) env
      in
      let tpat =
        make_typed_pattern (TPatVar id) expected_ty bindings pat.pat_span
      in
      Ok (tpat, env')
  | PatWildcard ->
      (* ワイルドカードパターン: 任意の型にマッチ、束縛なし *)
      let tpat = make_typed_pattern TPatWildcard expected_ty [] pat.pat_span in
      Ok (tpat, env)
  | PatTuple pats -> (
      (* タプルパターン: タプル型を構築して単一化 *)
      (* expected_ty がタプル型でない場合はエラー *)
      match expected_ty with
      | TTuple expected_tys when List.length pats = List.length expected_tys ->
          (* 各要素パターンを推論 *)
          let* tpats, env', all_bindings =
            List.fold_left2
              (fun acc pat expected_elem_ty ->
                match acc with
                | Error e -> Error e
                | Ok (tpats, env_acc, bindings_acc) ->
                    let* tpat, env_new =
                      infer_pattern ~ctx env_acc pat expected_elem_ty
                    in
                    Ok
                      ( tpats @ [ tpat ],
                        env_new,
                        bindings_acc @ tpat.tpat_bindings ))
              (Ok ([], env, []))
              pats expected_tys
          in
          let tpat =
            make_typed_pattern (TPatTuple tpats) expected_ty all_bindings
              pat.pat_span
          in
          Ok (tpat, env')
      | TTuple expected_tys ->
          (* タプルの要素数が不一致 *)
          Error
            (TupleArityMismatch
               {
                 expected = List.length expected_tys;
                 actual = List.length pats;
                 span = pat.pat_span;
               })
      | _ -> (
          (* expected_ty がタプル型でない場合は新しいタプル型を作成して単一化 *)
          let elem_vars =
            List.map (fun _ -> Types.TVar (TypeVarGen.fresh None)) pats
          in
          let tuple_ty = TTuple elem_vars in
          match unify empty_subst expected_ty tuple_ty pat.pat_span with
          | Ok _ ->
              (* 再帰的に推論 *)
              infer_pattern ~ctx env pat tuple_ty
          | Error (UnificationFailure _) ->
              Error (NotATuple (expected_ty, pat.pat_span))
          | Error e -> Error e))
  | PatConstructor (id, arg_pats) -> (
      (* コンストラクタパターン: コンストラクタの型スキームを取得してインスタンス化 *)
      match lookup id.name env with
      | Some scheme ->
          (* 型スキームをインスタンス化 *)
          let ctor_ty = instantiate scheme in

          (* コンストラクタ型から引数型と結果型を抽出 *)
          let arg_tys, result_ty = extract_function_args ctor_ty in

          (* 引数の数が一致するか確認 *)
          if List.length arg_pats <> List.length arg_tys then
            Error
              (ConstructorArityMismatch
                 {
                   constructor = id.name;
                   expected = List.length arg_tys;
                   actual = List.length arg_pats;
                   span = pat.pat_span;
                 })
          else
            (* 結果型と expected_ty を単一化 *)
            let* _subst =
              unify empty_subst result_ty expected_ty pat.pat_span
            in

            (* 各引数パターンを推論 *)
            let* targ_pats, env', all_bindings =
              List.fold_left2
                (fun acc arg_pat arg_ty ->
                  match acc with
                  | Error e -> Error e
                  | Ok (tpats, env_acc, bindings_acc) ->
                      let* tpat, env_new =
                        infer_pattern ~ctx env_acc arg_pat arg_ty
                      in
                      Ok
                        ( tpats @ [ tpat ],
                          env_new,
                          bindings_acc @ tpat.tpat_bindings ))
                (Ok ([], env, []))
                arg_pats arg_tys
            in

            let tpat =
              make_typed_pattern
                (TPatConstructor (id, targ_pats))
                expected_ty all_bindings pat.pat_span
            in
            Ok (tpat, env')
      | None -> Error (unbound_variable_error id.name pat.pat_span))
  | PatRecord (fields, has_rest) -> (
      (* レコードパターン: Phase 2 では基本実装のみ *)
      match expected_ty with
      | TRecord expected_fields ->
          (* 各フィールドパターンを推論 *)
          let* tfield_pats, env', all_bindings =
            List.fold_left
              (fun acc (field_id, field_pat_opt) ->
                match acc with
                | Error e -> Error e
                | Ok (tfields, env_acc, bindings_acc) -> (
                    (* expected_fields からフィールド型を検索 *)
                    match List.assoc_opt field_id.name expected_fields with
                    | Some field_ty -> (
                        (* フィールドパターンがある場合は推論、ない場合は変数束縛 *)
                        match field_pat_opt with
                        | Some field_pat ->
                            let* tpat, env_new =
                              infer_pattern ~ctx env_acc field_pat field_ty
                            in
                            Ok
                              ( tfields @ [ (field_id, Some tpat) ],
                                env_new,
                                bindings_acc @ tpat.tpat_bindings )
                        | None ->
                            (* フィールド名を変数として束縛 *)
                            let bindings = [ (field_id.name, field_ty) ] in
                            let env_new =
                              extend field_id.name
                                (scheme_to_constrained (mono_scheme field_ty))
                                env_acc
                            in
                            let tpat =
                              make_typed_pattern (TPatVar field_id) field_ty
                                bindings pat.pat_span
                            in
                            Ok
                              ( tfields @ [ (field_id, Some tpat) ],
                                env_new,
                                bindings_acc @ bindings ))
                    | None ->
                        Error
                          (RecordFieldUnknown
                             { field = field_id.name; span = pat.pat_span })))
              (Ok ([], env, []))
              fields
          in

          (* rest (..) がない場合、全フィールドをカバーしているか確認 *)
          if not has_rest then
            let pattern_fields = List.map (fun (id, _) -> id.name) fields in
            let type_fields = List.map fst expected_fields in
            let missing_fields =
              List.filter (fun f -> not (List.mem f pattern_fields)) type_fields
            in
            if missing_fields <> [] then
              Error (RecordFieldMissing { missing_fields; span = pat.pat_span })
            else
              let tpat =
                make_typed_pattern
                  (TPatRecord (tfield_pats, has_rest))
                  expected_ty all_bindings pat.pat_span
              in
              Ok (tpat, env')
          else
            let tpat =
              make_typed_pattern
                (TPatRecord (tfield_pats, has_rest))
                expected_ty all_bindings pat.pat_span
            in
            Ok (tpat, env')
      | _ -> Error (NotARecord (expected_ty, pat.pat_span)))
  | PatGuard (inner_pat, guard_expr) ->
      (* ガード付きパターン: 内部パターンを推論後、ガード式を Bool 型として推論
       * 注: ガード式の制約は現在破棄される（パターンマッチは制約を生成しない）
       *)
      let* tinner_pat, env' = infer_pattern ~ctx env inner_pat expected_ty in

      (* ガード式を Bool 型として推論: 制約は破棄 *)
      let* tguard_expr, guard_ty, _, _guard_constraints =
        infer_expr ~ctx env' guard_expr
      in
      let* _ = unify_as_bool empty_subst guard_ty guard_expr.expr_span in

      let tpat =
        make_typed_pattern
          (TPatGuard (tinner_pat, tguard_expr))
          expected_ty tinner_pat.tpat_bindings (* 束縛は内部パターンから継承 *) pat.pat_span
      in
      Ok (tpat, env')

(** 関数型から引数型のリストと結果型を抽出
 *
 * TArrow (A, TArrow (B, C)) → ([A; B], C)
 *)
and extract_function_args (ty : ty) : ty list * ty =
  match ty with
  | TArrow (arg_ty, rest_ty) ->
      let args, result = extract_function_args rest_ty in
      (arg_ty :: args, result)
  | _ -> ([], ty)

(** impl itemの型推論
 *
 * Phase 2 Week 23: impl宣言内のアイテムを推論
 *
 * @param env 型環境（ジェネリック型パラメータを含む）
 * @param items impl itemのリスト
 * @return (型付きimpl item, 制約リスト)
 *)
and infer_impl_items ?(ctx = initial_ctx) (env : env) (items : impl_item list) :
    (impl_item list * trait_constraint list, type_error) result =
  (* 各アイテムを順次推論し、制約を収集 *)
  List.fold_left
    (fun acc item ->
      match acc with
      | Error e -> Error e
      | Ok (timpl_items, constraints) -> (
          match item with
          | ImplFn fn ->
              (* メソッド定義の型推論 *)
              let* _tfn, _new_env, fn_constraints =
                (* fnをdeclとしてラップして推論 *)
                let dummy_decl =
                  {
                    decl_attrs = [];
                    decl_vis = Private;
                    decl_kind = FnDecl fn;
                    decl_span = dummy_span;
                  }
                in
                infer_decl ~ctx ~config:!current_config env dummy_decl
              in
              (* impl itemは元のASTを保持（後でLLVMコード生成で使用） *)
              Ok
                ( timpl_items @ [ ImplFn fn ],
                  merge_constraints constraints fn_constraints )
          | ImplLet (pat, ty_annot, expr) ->
              (* let束縛の型推論（簡易版） *)
              let* texpr, expr_ty, _s, expr_constraints =
                infer_expr ~ctx env expr
              in
              (* 型注釈があれば検証（後で実装） *)
              let _ = (pat, ty_annot, texpr, expr_ty) in
              Ok
                ( timpl_items @ [ ImplLet (pat, ty_annot, expr) ],
                  merge_constraints constraints expr_constraints )))
    (Ok ([], []))
    items

(** 二項演算子の型推論
 *
 * Phase 2 MVP: 基本演算子の組み込みトレイトのみ（i64, f64, Bool, String対応）
 *
 * @param op 演算子
 * @param ty1 左辺の型
 * @param ty2 右辺の型
 * @param subst 現在の代入
 * @param span1 左辺のSpan
 * @param span2 右辺のSpan
 * @return (返り値型, 新しい代入)
 *)
and infer_binary_op (op : Ast.binary_op) (ty1 : ty) (ty2 : ty)
    (subst : substitution) (span1 : span) (span2 : span) :
    (ty * substitution * trait_constraint list, type_error) result =
  (* ========== Phase 2 Week 20-21: トレイト制約収集の実装 ==========
   *
   * 【実装完了】
   * 本関数は二項演算子の型推論を行い、対応するトレイト制約を収集する。
   * 制約は型推論結果とともに返され、後続の制約解決器で処理される。
   *
   * 【演算子とトレイト制約のマッピング】
   * | 演算子    | トレイト          | 型制約                 | 例                |
   * |----------|-------------------|----------------------|-------------------|
   * | +        | Add<T>            | T: 数値型 (i64, f64)  | 1 + 2 : i64      |
   * | -, *, /  | Sub/Mul/Div<T>    | 同上                  | 3.0 * 2.0 : f64  |
   * | %, ^     | Mod/Pow<T>        | 整数型 (i64)          | 10 % 3 : i64     |
   * | ==, !=   | Eq<T>             | T: 比較可能型         | "a" == "b" : Bool|
   * | <, <=, >, >= | Ord<T>        | T: 順序付け可能型     | 1 < 2 : Bool     |
   * | &&, \|\| | なし              | T = Bool (組み込み)   | true && false    |
   * | \|>      | なし              | f: T -> U (関数適用)  | x \|> f          |
   *
   * 【Week 20-21 実装内容】
   * 1. ✅ 演算子から対応するトレイト制約を生成（collect_binary_op_constraints）
   * 2. ✅ 制約リストを返り値に追加（3要素タプル）
   * 3. 🚧 関数宣言の generalize 時に制約を含める（Week 21-22）
   * 4. 🚧 Constraint_solver.solve_constraints を呼び出し（Week 21-22）
   * 5. 🚧 解決された辞書参照を Core IR の DictLookup に変換（Week 21-22）
   *
   * 【参考資料】
   * - constraint_solver.ml の solve_eq, solve_ord
   * - docs/spec/1-2-types-Inference.md §B（トレイト）
   * - docs/plans/bootstrap-roadmap/2-1-typeclass-strategy.md
   * ================================================================= *)
  match op with
  (* 算術演算子: + - * / % ^ *)
  | Add | Sub | Mul | Div | Mod | Pow -> (
      (* Week 20-21 実装: Add<ty1> 制約を収集
       * 例: 1 + 2 なら Add<i64> 制約を生成
       *
       * 仕様書 1-2 §C.5: 数値型（i64, f64）のみサポート
       * Phase 3 Week 17 改善: 型変数を単一化前に i64 へ解決して具体化 *)
      let ty1' = apply_subst subst ty1 in
      let ty2' = apply_subst subst ty2 in

      (* ステップ1: 型変数を数値型デフォルト（i64）へ解決 *)
      let resolve_numeric_default s ty =
        match apply_subst s ty with
        | TVar tv -> compose_subst [ (tv, ty_i64) ] s
        | _ -> s
      in
      let s_resolved =
        resolve_numeric_default (resolve_numeric_default subst ty1') ty2'
      in

      (* ステップ2: 解決後の型を再適用 *)
      let ty1'' = apply_subst s_resolved ty1' in
      let ty2'' = apply_subst s_resolved ty2' in

      (* ステップ3: 単一化（両方とも具体型になっているはず） *)
      let* s1 = unify s_resolved ty1'' ty2'' span2 in
      let unified_ty = apply_subst s1 ty1'' in

      (* ステップ4: 最終確認と制約生成 *)
      match unified_ty with
      | TCon (TCInt _) | TCon (TCFloat _) ->
          let constraints =
            collect_binary_op_constraints op unified_ty unified_ty span1
          in
          Ok (unified_ty, s1, constraints)
      | TVar tv ->
          (* ここに到達することは通常ないが、安全のため残す *)
          let s2 = compose_subst [ (tv, ty_i64) ] s1 in
          let constraints =
            collect_binary_op_constraints op ty_i64 ty_i64 span1
          in
          Ok (ty_i64, s2, constraints)
      | _ ->
          Error
            (type_error_with_message "算術演算子は数値型 (i64 / f64) にのみ適用できます" span1))
  (* 比較演算子: == != < <= > >= *)
  | Eq | Ne ->
      (* Week 20-21 実装: Eq<T> 制約を収集
       * 例: x == y なら Eq<typeof(x)> 制約を生成
       * 左辺と右辺を単一化し、返り値は Bool *)
      let ty1' = apply_subst subst ty1 in
      let ty2' = apply_subst subst ty2 in
      let* s1 = unify subst ty1' ty2' span2 in
      let unified_ty = apply_subst s1 ty1' in
      let constraints =
        collect_binary_op_constraints op unified_ty unified_ty span1
      in
      Ok (ty_bool, s1, constraints)
  | Lt | Le | Gt | Ge ->
      (* Week 20-21 実装: Ord<T> 制約を収集（Eq<T> も自動的に要求される）
       * 例: x < y なら Ord<typeof(x)> 制約を生成
       * 制約解決器が Ord→Eq のスーパートレイト依存を解決 *)
      (* 左辺と右辺を単一化し、返り値は Bool
       * Phase 3 Week 17 改善: 型変数を単一化前に i64 へ解決 *)
      let ty1' = apply_subst subst ty1 in
      let ty2' = apply_subst subst ty2 in

      (* ステップ1: 型変数を数値型デフォルト（i64）へ解決 *)
      let resolve_numeric_default s ty =
        match apply_subst s ty with
        | TVar tv -> compose_subst [ (tv, ty_i64) ] s
        | _ -> s
      in
      let s_resolved =
        resolve_numeric_default (resolve_numeric_default subst ty1') ty2'
      in

      (* ステップ2: 解決後の型を再適用 *)
      let ty1'' = apply_subst s_resolved ty1' in
      let ty2'' = apply_subst s_resolved ty2' in

      (* ステップ3: 単一化と制約生成 *)
      let* s1 = unify s_resolved ty1'' ty2'' span2 in
      let unified_ty = apply_subst s1 ty1'' in
      let constraints =
        collect_binary_op_constraints op unified_ty unified_ty span1
      in
      Ok (ty_bool, s1, constraints)
  (* 論理演算子: && || *)
  | And | Or ->
      (* 左辺と右辺をBool型と単一化（制約なし：組み込み型で解決済み） *)
      let ty1' = apply_subst subst ty1 in
      let ty2' = apply_subst subst ty2 in
      let* s1 = unify_as_bool subst ty1' span1 in
      let* s2 = unify_as_bool s1 ty2' span2 in
      Ok (ty_bool, s2, [])
  (* 制約なし *)
  (* パイプ演算子: |> *)
  | PipeOp ->
      (* x |> f は f(x) に等価（制約なし：関数適用のみ）
       * ty1 : A, ty2 : A -> B のとき、返り値は B
       *)
      let ty1' = apply_subst subst ty1 in
      let ret_var = TypeVarGen.fresh None in
      let ret_ty = Types.TVar ret_var in
      let expected_fn_ty = TArrow (ty1', ret_ty) in
      let* s1 = unify_as_function subst ty2 expected_fn_ty span2 in
      let final_ret_ty = apply_subst s1 ret_ty in
      Ok (final_ret_ty, s1, [])
(* 制約なし *)

(** match アームの型推論
 *
 * @param env 型環境
 * @param arm match アーム
 * @param scrutinee_ty スクラティニー式の型
 * @param subst 現在の代入
 * @return (型付きアーム, ボディの型, 新しい代入)
 *)
and infer_match_arm ?(ctx = initial_ctx) (env : env) (arm : match_arm)
    (scrutinee_ty : ty) (subst : substitution) :
    ( typed_match_arm * ty * substitution * trait_constraint list,
      type_error )
    result =
  (* パターンを推論 *)
  let* tpat, pat_env = infer_pattern ~ctx env arm.arm_pattern scrutinee_ty in

  (* ガード条件があれば推論
   * 制約: ガードとボディの制約をマージ
   *)
  let* tguard_opt, s1, guard_constraints =
    match arm.arm_guard with
    | Some guard_expr ->
        let* tguard, guard_ty, s, guard_cs =
          infer_expr ~ctx pat_env guard_expr
        in
        let* s' = unify_as_bool s guard_ty guard_expr.expr_span in
        Ok (Some tguard, s', guard_cs)
    | None -> Ok (None, subst, [])
  in

  (* ボディを推論 *)
  let env' = apply_subst_env s1 pat_env in
  let* tbody, body_ty, s2, body_constraints =
    infer_expr ~ctx env' arm.arm_body
  in

  (* 制約をマージ *)
  let all_constraints = merge_constraints guard_constraints body_constraints in

  (* 型付きアームを構築 *)
  let tarm =
    {
      tarm_pattern = tpat;
      tarm_guard = tguard_opt;
      tarm_body = tbody;
      tarm_span = arm.arm_span;
    }
  in

  Ok (tarm, body_ty, s2, all_constraints)

(** 関数本体の型推論: infer_fn_body(env, body)
 *
 * Phase 2 Week 5: FnExpr（式）とFnBlock（文のリスト）の両方に対応
 *
 * @param env 型環境（パラメータで拡張済み）
 * @param body 関数本体（AST）
 * @return (型付き関数本体, 本体の型, 代入)
 *)
and infer_fn_body ?(ctx = initial_ctx) (env : env) (body : fn_body) :
    ( typed_fn_body * ty * substitution * trait_constraint list,
      type_error )
    result =
  match body with
  | FnExpr expr ->
      (* 式の場合: 直接推論 *)
      let* texpr, ty, s, constraints = infer_expr ~ctx env expr in
      Ok (TFnExpr texpr, ty, s, constraints)
  | FnBlock stmts ->
      (* ブロックの場合: 文のリストを推論 *)
      let* tstmts, ty, s, constraints =
        infer_stmts ~ctx env stmts empty_subst
      in
      Ok (TFnBlock tstmts, ty, s, constraints)

(** 文リストの型推論: infer_stmts(env, stmts, subst)
 *
 * Phase 2 Week 5: ブロック式のための文リスト型推論
 *
 * @param env 現在の型環境
 * @param stmts 文のリスト
 * @param subst 現在の代入
 * @return (型付き文リスト, 最終型, 最終代入)
 *)
and infer_stmts ?(ctx = initial_ctx) (env : env) (stmts : stmt list)
    (subst : substitution) :
    ( typed_stmt list * ty * substitution * trait_constraint list,
      type_error )
    result =
  (* 最後の文を特別扱い
   * 制約: 全ての文の制約をマージ
   *)
  let rec process_stmts env stmts acc_tstmts subst acc_constraints =
    match stmts with
    | [] ->
        (* 空リスト: Unit型、制約なし *)
        Ok (List.rev acc_tstmts, ty_unit, subst, acc_constraints)
    | [ last_stmt ] -> (
        (* 最後の文: ExprStmtなら式の型、それ以外はUnit *)
        match last_stmt with
        | ExprStmt expr ->
            (* 最後の式文: 式の型がブロック全体の型 *)
            let env' = apply_subst_env subst env in
            let* texpr, expr_ty, s, expr_constraints =
              infer_expr ~ctx env' expr
            in
            let tstmt = TExprStmt texpr in
            let all_constraints =
              merge_constraints acc_constraints expr_constraints
            in
            Ok (List.rev (tstmt :: acc_tstmts), expr_ty, s, all_constraints)
        | _ ->
            (* 最後の文が宣言/代入/defer: Unit型 *)
            let* tstmt, _new_env, s, stmt_constraints =
              infer_stmt ~ctx env last_stmt subst
            in
            let all_constraints =
              merge_constraints acc_constraints stmt_constraints
            in
            Ok (List.rev (tstmt :: acc_tstmts), ty_unit, s, all_constraints))
    | stmt :: rest ->
        (* 中間の文: 処理して環境更新 *)
        let* tstmt, new_env, s, stmt_constraints =
          infer_stmt ~ctx env stmt subst
        in
        let merged_constraints =
          merge_constraints acc_constraints stmt_constraints
        in
        process_stmts new_env rest (tstmt :: acc_tstmts) s merged_constraints
  in
  process_stmts env stmts [] subst []

(** 文の型推論: infer_stmt(env, stmt, subst)
 *
 * Phase 2 Week 5: 文の型推論
 *
 * @param env 現在の型環境
 * @param stmt 文（AST）
 * @param subst 現在の代入
 * @return (型付き文, 新しい型環境, 新しい代入)
 *)
and infer_stmt ?(ctx = initial_ctx) (env : env) (stmt : stmt)
    (subst : substitution) :
    (typed_stmt * env * substitution * trait_constraint list, type_error) result
    =
  match stmt with
  | DeclStmt decl ->
      (* 宣言文: 型推論して型環境を更新
       * 制約: 宣言の式から生成される制約を伝播
       *)
      let env' = apply_subst_env subst env in
      let* tdecl, new_env, decl_constraints =
        infer_decl ~ctx ~config:!current_config env' decl
      in
      Ok (TDeclStmt tdecl, new_env, subst, decl_constraints)
  | ExprStmt expr ->
      (* 式文: 式を推論（型環境は変更なし）
       * 制約: 式の制約を伝播
       *)
      let env' = apply_subst_env subst env in
      let* texpr, _ty, s, expr_constraints = infer_expr ~ctx env' expr in
      Ok (TExprStmt texpr, env, s, expr_constraints)
  | AssignStmt (lhs, rhs) ->
      (* 代入文: 左辺と右辺を推論して型を統一
       *
       * 仕様書 1-1 §C.6: var 束縛の再代入 `:=` は Unit型を返す
       * 制約: 左辺と右辺の制約をマージ
       *)
      let env' = apply_subst_env subst env in
      let* tlhs, lhs_ty, s1, lhs_constraints = infer_expr ~ctx env' lhs in
      let* () = ensure_assignable env' tlhs lhs.expr_span in
      let env'' = apply_subst_env s1 env' in
      let* trhs, rhs_ty, s2, rhs_constraints = infer_expr ~ctx env'' rhs in
      (* 左辺と右辺の型を単一化 *)
      let lhs_ty' = apply_subst s2 lhs_ty in
      let* s3 = unify s2 lhs_ty' rhs_ty rhs.expr_span in
      let all_constraints = merge_constraints lhs_constraints rhs_constraints in
      Ok (TAssignStmt (tlhs, trhs), env, s3, all_constraints)
  | DeferStmt expr ->
      (* defer文: 式を推論（Unit型、型環境は変更なし）
       * 制約: 式の制約を伝播
       *)
      let env' = apply_subst_env subst env in
      let* texpr, _ty, s, expr_constraints = infer_expr ~ctx env' expr in
      Ok (TDeferStmt texpr, env, s, expr_constraints)

(** 宣言の型推論: infer_decl(env, decl)
 *
 * Phase 2 Week 3-4 で実装
 *)
and infer_decl ?(ctx = initial_ctx) ?config (env : env) (decl : decl) :
    (typed_decl * env * trait_constraint list, type_error) result =
  let config = match config with Some cfg -> cfg | None -> !current_config in
  match decl.decl_kind with
  | LetDecl (pat, ty_annot, expr) ->
      (* let束縛の型推論
       *
       * 1. 式を推論
       * 2. 型注釈があれば単一化
       * 3. パターンの型推論
       * 4. 一般化してスキームを生成
       * 5. 型環境に追加
       * 制約: 式の制約を伝播
       *)
      (* 式を推論 *)
      let* texpr, expr_ty, s1, expr_constraints = infer_expr ~ctx env expr in

      (* 型注釈があれば単一化 *)
      let* final_ty, s2 =
        match ty_annot with
        | Some annot ->
            let expected_ty = convert_type_annot annot in
            let* s = unify s1 expr_ty expected_ty expr.expr_span in
            Ok (apply_subst s expr_ty, s)
        | None -> Ok (expr_ty, s1)
      in

      (* パターンから変数名と識別子を抽出（簡易版：変数パターンのみ）*)
      let pat_name, pat_id =
        match pat.pat_kind with
        | PatVar id -> (id.name, id)
        | _ -> failwith "Complex let patterns not yet implemented"
      in

      (* 型付きパターンを構築 *)
      let tpat =
        make_typed_pattern (TPatVar pat_id) final_ty
          [ (pat_name, final_ty) ]
          pat.pat_span
      in

      (* 一般化してスキームを生成 *)
      let env' = apply_subst_env s2 env in
      let scheme = generalize env' final_ty in

      (* Phase 2 Week 18-19: 型クラス制約の解決
       *
       * 制約が空でない場合は解決を試みる。
       * 現時点では制約収集は未実装のため、常に空リストだが、
       * 将来的に制約収集が実装されたときに自動的に解決される。
       *)
      let* _dict_refs =
        if scheme.constraints = [] then Ok []
        else (* 制約を解決 *)
          solve_trait_constraints scheme.constraints
      in

      (* 型付き宣言を構築 *)
      let tdecl =
        make_typed_decl decl.decl_attrs decl.decl_vis
          (TLetDecl (tpat, texpr))
          scheme decl.decl_span
      in

      (* 型環境に追加 *)
      let new_env = extend pat_name scheme env' in

      Ok (tdecl, new_env, expr_constraints)
  | VarDecl (pat, ty_annot, expr) ->
      (* var束縛の型推論
       *
       * let束縛と同様に型推論を行うが、ミュータブルフラグを記録する
       * 1. 初期値式を推論
       * 2. 型注釈があれば単一化
       * 3. パターンの型推論（現在は変数パターンのみ想定）
       * 4. 型スキームを一般化
       * 5. 環境にミュータブルとして追加
       *)
      let* texpr, expr_ty, s1, expr_constraints = infer_expr ~ctx env expr in
      let* final_ty, s2 =
        match ty_annot with
        | Some annot ->
            let expected_ty = convert_type_annot annot in
            let* s = unify s1 expr_ty expected_ty expr.expr_span in
            Ok (apply_subst s expr_ty, s)
        | None -> Ok (expr_ty, s1)
      in

      let pat_name, pat_id =
        match pat.pat_kind with
        | PatVar id -> (id.name, id)
        | _ -> failwith "Complex var patterns not yet implemented"
      in

      let tpat =
        make_typed_pattern (TPatVar pat_id) final_ty
          [ (pat_name, final_ty) ]
          pat.pat_span
      in

      let env' = apply_subst_env s2 env in
      let scheme = generalize env' final_ty in

      let* _dict_refs =
        if scheme.constraints = [] then Ok []
        else solve_trait_constraints scheme.constraints
      in

      let tdecl =
        make_typed_decl decl.decl_attrs decl.decl_vis
          (TVarDecl (tpat, texpr))
          scheme decl.decl_span
      in

      let new_env = extend ~mutability:Mutable pat_name scheme env' in
      Ok (tdecl, new_env, expr_constraints)
  | FnDecl fn ->
      (* 関数宣言の型推論
       *
       * Phase 2 Week 5: 関数宣言の型推論を実装
       *
       * 1. ジェネリック型パラメータを型変数に変換
       * 2. パラメータの型推論
       * 3. 再帰関数のための暫定型を構築
       * 4. 関数本体の型推論
       * 5. 返り値型の検証
       * 6. 関数型の一般化
       *)

      (* 1. ジェネリック型パラメータを型変数に変換 *)
      let generic_bindings =
        List.map
          (fun id -> (id, TypeVarGen.fresh (Some id.name)))
          fn.fn_generic_params
      in

      (* 2. ジェネリック型を型環境に追加 *)
      let env_with_generics =
        List.fold_left
          (fun acc (id, tv) ->
            extend id.name
              (scheme_to_constrained (mono_scheme (Types.TVar tv)))
              acc)
          env generic_bindings
      in

      (* 3. パラメータの型推論 *)
      let* tparams, param_tys, param_env, _s1 =
        infer_params env_with_generics fn.fn_params empty_subst
      in

      (* 4. 再帰関数のための暫定型を構築 *)
      let temp_ret_var = TypeVarGen.fresh None in
      let temp_fn_ty =
        List.fold_right
          (fun param_ty acc -> TArrow (param_ty, acc))
          param_tys (Types.TVar temp_ret_var)
      in

      (* 5. 関数名を型環境に追加（再帰呼び出しに対応） *)
      let env_with_fn =
        extend fn.fn_name.name
          (scheme_to_constrained (mono_scheme temp_fn_ty))
          param_env
      in

      (* 6. 関数本体の型推論
       * 制約: 関数本体の制約を伝播
       *)
      let* tbody, body_ty, s2, body_constraints =
        infer_fn_body ~ctx env_with_fn fn.fn_body
      in

      (* 7. 返り値型注釈があれば単一化 *)
      let* final_ret_ty, s3 =
        match fn.fn_ret_type with
        | Some annot ->
            let expected_ret_ty = convert_type_annot annot in
            let* s = unify s2 body_ty expected_ret_ty decl.decl_span in
            Ok (apply_subst s body_ty, s)
        | None -> Ok (body_ty, s2)
      in

      (* 8b. パラメータへ代入を適用 *)
      let force_numeric = fn.fn_generic_params = [] in
      let tparams' =
        resolve_params ~force_numeric_default:force_numeric s3 param_env tparams
      in
      let param_tys_resolved = List.map (fun p -> p.tty) tparams' in

      (* 8. 最終的な関数型を構築 *)
      let fn_ty =
        List.fold_right
          (fun param_ty acc -> TArrow (param_ty, acc))
          param_tys_resolved final_ret_ty
      in

      (* 9. 一般化してスキームを生成 *)
      let env' = apply_subst_env s3 env in
      let scheme = generalize env' fn_ty in

      (* Phase 2 Week 18-19: 型クラス制約の解決
       *
       * 関数宣言でも制約を解決する。
       *)
      let* _dict_refs =
        if scheme.constraints = [] then Ok []
        else (* 制約を解決 *)
          solve_trait_constraints scheme.constraints
      in

      (* 10a. 効果プロファイルを解析 *)
      let residual_tags = Effect_analysis.collect_from_fn_body tbody in
      let* effect_profile_raw =
        Type_inference_effect.resolve_function_profile
          ~runtime_context:config.effect_context ~function_ident:fn.fn_name
          fn.fn_effect_profile
      in
      let effect_profile, _residual_leaks =
        Effect_analysis.merge_usage_into_profile
          ~fallback_span:effect_profile_raw.source_span effect_profile_raw
          residual_tags
      in
      record_effect_profile ~symbol:fn.fn_name.name effect_profile;

      (* 10b. 型付き関数宣言を構築 *)
      let tfn =
        {
          tfn_name = fn.fn_name;
          tfn_generic_params = generic_bindings;
          tfn_params = tparams';
          tfn_ret_type = final_ret_ty;
          tfn_where_clause = fn.fn_where_clause;
          tfn_effect_profile = effect_profile;
          tfn_body = tbody;
        }
      in

      let tdecl =
        make_typed_decl decl.decl_attrs decl.decl_vis (TFnDecl tfn) scheme
          decl.decl_span
      in

      (* 11. 型環境に追加 *)
      let new_env = extend fn.fn_name.name scheme env' in

      Ok (tdecl, new_env, body_constraints)
  | ExternDecl extern_decl ->
      let block_target = extern_decl.extern_block_target in
      let convert_signature signature =
        let param_tys =
          List.map
            (fun param ->
              match param.ty with
              | Some annot -> convert_type_annot annot
              | None -> Types.TVar (TypeVarGen.fresh None))
            signature.sig_args
        in
        let ret_ty =
          match signature.sig_ret with
          | Some annot -> convert_type_annot annot
          | None -> ty_unit
        in
        let fn_ty =
          List.fold_right (fun param_ty acc -> TArrow (param_ty, acc)) param_tys
            ret_ty
        in
        (fn_ty, param_tys, ret_ty)
      in
      let rec process_items env = function
        | [] -> Ok env
        | item :: rest -> (
            match check_extern_bridge_contract ~block_target item with
            | Error err -> Error err
            | Ok normalized ->
                let fn_ty, param_tys, ret_ty =
                  convert_signature item.extern_sig
                in
                record_ffi_bridge_snapshot
                  { normalized; param_types = param_tys; return_type = ret_ty };
                let scheme = generalize env fn_ty in
                let env' = extend item.extern_sig.sig_name.name scheme env in
                process_items env' rest)
      in
      let* final_env = process_items env extern_decl.extern_items in
      let tdecl =
        make_typed_decl decl.decl_attrs decl.decl_vis
          (TExternDecl extern_decl)
          (scheme_to_constrained (mono_scheme ty_unit)) decl.decl_span
      in
      Ok (tdecl, final_env, [])
  | ImplDecl impl ->
      (* impl宣言の型推論
       *
       * Phase 2 Week 23: impl宣言の型推論を実装
       * Phase 2 Week 23-24: レジストリへの登録を追加
       *
       * 1. ジェネリック型パラメータを型変数に変換
       * 2. impl対象型を変換
       * 3. トレイト情報があれば処理
       * 4. 各メソッドの型推論
       * 5. トレイト制約の検証（where句）
       * 6. impl情報をレジストリに登録 ← 新規追加（Week 23-24）
       *)

      (* 1. ジェネリック型パラメータを型変数に変換 *)
      let generic_bindings =
        List.map
          (fun id -> (id, TypeVarGen.fresh (Some id.name)))
          impl.impl_params
      in

      (* 型変数のリストを抽出 *)
      let generic_type_vars = List.map snd generic_bindings in

      (* 2. ジェネリック型を型環境に追加 *)
      let env_with_generics =
        List.fold_left
          (fun acc (id, tv) ->
            extend id.name
              (scheme_to_constrained (mono_scheme (Types.TVar tv)))
              acc)
          env generic_bindings
      in

      (* 3. impl対象型を変換 *)
      let impl_target_ty = convert_type_annot impl.impl_type in

      (* 4. トレイト情報があれば変換 *)
      let trait_name, where_constraints =
        match impl.impl_trait with
        | Some (trait_id, _trait_args) ->
            (* トレイト実装の場合 *)
            (* TODO: where句からトレイト制約を抽出（Phase 2 後半で実装） *)
            (trait_id.name, [])
        | None ->
            (* inherent impl の場合 *)
            ("(inherent)", [])
      in

      (* 5. 各impl itemを推論 *)
      let* _timpl_items, item_constraints =
        infer_impl_items env_with_generics impl.impl_items
      in

      (* 6. メソッド実装情報を抽出 *)
      let method_list =
        List.filter_map
          (fun item ->
            match item with
            | Ast.ImplFn fn ->
                (* メソッド名と実装関数名のペア *)
                (* Phase 2 Week 23-24: 簡易実装では関数名をそのまま使用 *)
                Some (fn.fn_name.name, trait_name ^ "_" ^ fn.fn_name.name)
            | _ -> None)
          impl.impl_items
      in

      (* 7. impl情報を構築してレジストリに登録 *)
      let impl_info : Impl_registry.impl_info =
        {
          trait_name;
          impl_type = impl_target_ty;
          generic_params = generic_type_vars;
          where_constraints;
          methods = method_list;
          span = decl.decl_span;
        }
      in
      register_impl impl_info;

      (* 8. impl宣言の型スキームを構築
       * impl宣言自体は値を持たないため、Unitスキーム
       *)
      let impl_scheme = mono_scheme ty_unit in

      (* 9. 型付きimpl宣言を構築 *)
      let tdecl =
        make_typed_decl decl.decl_attrs decl.decl_vis (TImplDecl impl)
          (scheme_to_constrained impl_scheme)
          decl.decl_span
      in

      (* impl宣言は環境を変更しない（メソッドは辞書経由で解決） *)
      Ok (tdecl, env, item_constraints)
  | _ ->
      (* その他の宣言
       * 制約: 現時点では空リスト
       *)
      failwith "Declaration not yet implemented"

(** コンパイル単位の型推論 *)
let infer_compilation_unit ?(config = default_config) (cu : compilation_unit) :
    (typed_compilation_unit, type_error) result =
  let detect_effect_residual_leak () =
    let table = current_effect_constraints () in
    let entries = EffectConstraintTable.to_list table in
    let rec aux = function
      | [] -> None
      | entry :: rest ->
          let leaks =
            entry.Constraint_solver.EffectConstraintTable.diagnostic_payload
              .Effect_profile.residual_leaks
          in
          if leaks <> [] then
            let profile =
              Effect_profile.make_profile ?source_name:entry.source_name
                ?resolved_stage:entry.resolved_stage
                ?resolved_capability:entry.resolved_capability
                ~stage_trace:entry.stage_trace
                ~diagnostic_payload:
                  entry
                    .Constraint_solver.EffectConstraintTable.diagnostic_payload
                ~stage_requirement:entry.stage_requirement
                ~effect_set:entry.effect_set ~span:entry.source_span ()
            in
            Some (entry.symbol, profile, leaks)
          else aux rest
    in
    aux entries
  in
  (* 初期型環境を作成 *)
  reset_effect_constraints ();
  Monomorph_registry.reset ();
  reset_ffi_bridge_snapshots ();
  current_config := config;
  let init_env = initial_env in

  (* 各宣言を順次推論し、型環境を更新
   * 制約: 現時点では制約を収集するが使用しない（Week 21-22 で統合予定）
   *)
  let rec infer_items env items acc_decls =
    match items with
    | [] -> Ok (List.rev acc_decls, env)
    | item :: rest -> (
        match infer_decl ~config env item with
        | Ok (typed_decl, new_env, _constraints) ->
            (* TODO Week 21-22: ここで制約を蓄積して最終的に解決 *)
            infer_items new_env rest (typed_decl :: acc_decls)
        | Error err -> Error err)
  in

  match infer_items init_env cu.decls [] with
  | Ok (typed_items, _final_env) -> (
      match detect_effect_residual_leak () with
      | Some (symbol, profile, leaks) ->
          let function_name =
            match profile.Effect_profile.source_name with
            | Some name -> Some name
            | None -> Some symbol
          in
          Error
            (Type_error.effect_residual_leak_error ~function_name ~profile
               ~leaks)
      | None ->
          Ok
            {
              tcu_module_header = cu.header;
              tcu_use_decls = cu.uses;
              tcu_items = typed_items;
            })
  | Error err -> Error err

(* ========== デバッグ用 ========== *)

(** 推論結果の文字列表現
 * Phase 2 Week 21-22: 制約リストを含む4要素タプルに対応
 *)
let string_of_infer_result (texpr, ty, subst, constraints) =
  let constraints_str =
    if constraints = [] then "no constraints"
    else String.concat ", " (List.map string_of_trait_constraint constraints)
  in
  Printf.sprintf "%s : %s [%s] {%s}"
    (string_of_typed_expr texpr)
    (string_of_ty ty) (string_of_subst subst) constraints_str
