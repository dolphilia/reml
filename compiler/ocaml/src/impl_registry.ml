(* Impl_registry — Trait Implementation Registry for Reml Type System
 *
 * impl 宣言から得られるトレイト実装情報を管理する。
 *
 * Phase 2 Week 23-24: 制約ソルバーへの impl 宣言登録
 *)

open Types
open Ast

(* ========== データ構造の定義 ========== *)

(** impl 実装情報 *)
type impl_info = {
  trait_name : string;
  impl_type : ty;
  generic_params : type_var list;
  where_constraints : trait_constraint list;
  methods : (string * string) list;
  span : span;
}

(** impl 宣言レジストリ *)
type impl_registry = { impls : impl_info list }

(* ========== 基本操作 ========== *)

(** 空のレジストリを作成 *)
let empty () = { impls = [] }

(** impl 実装情報をレジストリに登録 *)
let register impl_info registry =
  { impls = impl_info :: registry.impls }

(** レジストリに登録されているすべての impl を取得 *)
let all_impls registry = registry.impls

(* ========== 型照合ユーティリティ ========== *)

(** 型変数の束縛を管理する環境 *)
type type_subst = (type_var * ty) list

(** 空の型代入 *)
let empty_subst : type_subst = []

(** 型代入を適用 *)
let rec apply_type_subst (subst : type_subst) (ty : ty) : ty =
  match ty with
  | TVar tv -> (
      match List.assoc_opt tv subst with Some t -> t | None -> TVar tv)
  | TArrow (t1, t2) ->
      TArrow (apply_type_subst subst t1, apply_type_subst subst t2)
  | TTuple tys -> TTuple (List.map (apply_type_subst subst) tys)
  | TRecord fields ->
      TRecord
        (List.map (fun (name, ty) -> (name, apply_type_subst subst ty)) fields)
  | TApp (t1, t2) ->
      TApp (apply_type_subst subst t1, apply_type_subst subst t2)
  | TArray t -> TArray (apply_type_subst subst t)
  | TSlice (t, n) -> TSlice (apply_type_subst subst t, n)
  | TCon _ | TUnit | TNever -> ty

(** 型変数を型に束縛
 *
 * 既存の束縛と矛盾しないかチェック
 *)
let bind_type_var (tv : type_var) (ty : ty) (subst : type_subst) :
    type_subst option =
  match List.assoc_opt tv subst with
  | Some existing_ty ->
      if type_equal existing_ty ty then Some subst
      else None (* 矛盾 *)
  | None -> Some ((tv, ty) :: subst)

(** 型の統一（unification）
 *
 * パターン型（ジェネリック型変数を含む）と具体型を照合し、
 * 型変数の束縛を計算する。
 *
 * 例:
 *   unify_types Vec<T> Vec<i64> empty_subst
 *   → Some [(T, i64)]
 *
 * 返り値:
 *   Some subst: 統一成功、型変数の束縛
 *   None: 統一失敗
 *)
let rec unify_types (pattern : ty) (concrete : ty) (subst : type_subst) :
    type_subst option =
  (* パターン側に既存の束縛があれば適用 *)
  let pattern = apply_type_subst subst pattern in

  match (pattern, concrete) with
  (* 型変数の束縛 *)
  | TVar tv, _ -> bind_type_var tv concrete subst
  (* 同一の型コンストラクタ *)
  | TCon c1, TCon c2 when type_const_equal c1 c2 -> Some subst
  | TUnit, TUnit -> Some subst
  | TNever, TNever -> Some subst
  (* 関数型 *)
  | TArrow (p1, p2), TArrow (c1, c2) -> (
      match unify_types p1 c1 subst with
      | Some subst' -> unify_types p2 c2 subst'
      | None -> None)
  (* タプル型 *)
  | TTuple ps, TTuple cs when List.length ps = List.length cs -> (
      try
        Some
          (List.fold_left2
             (fun subst p c ->
               match subst with
               | Some s -> unify_types p c s
               | None -> None)
             (Some subst) ps cs
          |> function
          | Some s -> s
          | None -> raise Exit)
      with Exit -> None)
  (* レコード型（フィールド名と型を照合） *)
  | TRecord p_fields, TRecord c_fields when List.length p_fields = List.length c_fields -> (
      (* フィールド名でソート後に照合 *)
      let sort_fields fields =
        List.sort (fun (n1, _) (n2, _) -> String.compare n1 n2) fields
      in
      let p_sorted = sort_fields p_fields in
      let c_sorted = sort_fields c_fields in
      try
        Some
          (List.fold_left2
             (fun subst (pn, pty) (cn, cty) ->
               match subst with
               | Some s when pn = cn -> unify_types pty cty s
               | _ -> None)
             (Some subst) p_sorted c_sorted
          |> function
          | Some s -> s
          | None -> raise Exit)
      with Exit -> None)
  (* 型適用（例: Vec<T>） *)
  | TApp (p1, p2), TApp (c1, c2) -> (
      match unify_types p1 c1 subst with
      | Some subst' -> unify_types p2 c2 subst'
      | None -> None)
  (* 配列型・スライス型 *)
  | TArray p, TArray c -> unify_types p c subst
  | TSlice (p, pn), TSlice (c, cn) when pn = cn -> unify_types p c subst
  (* 不一致 *)
  | _ -> None

(** 型照合の判定
 *
 * パターン型と具体型が照合可能かチェック
 *)
let type_matches (pattern : ty) (concrete : ty) : bool =
  match unify_types pattern concrete empty_subst with
  | Some _ -> true
  | None -> false

(* ========== impl 検索 ========== *)

(** 指定されたトレイト名と型に一致する impl を検索 *)
let lookup (trait_name : string) (impl_type : ty) (registry : impl_registry) :
    impl_info option =
  List.find_opt
    (fun impl_info ->
      impl_info.trait_name = trait_name
      && type_matches impl_info.impl_type impl_type)
    registry.impls

(** where 句制約の検証
 *
 * impl の where 句に指定された制約が満たされているかチェック。
 *
 * Phase 2 Week 23-24: 簡易実装
 *   - where 句が空なら常に成功
 *   - where 句がある場合は、暫定的に常に成功とする
 *   - Phase 2 後半で再帰的な制約解決を実装予定
 *
 * TODO Phase 2 後半:
 *   - where 句制約を再帰的に解決
 *   - 制約ソルバーとの連携
 *)
let check_where_constraints (where_constraints : trait_constraint list)
    (_subst : type_subst) : bool =
  (* 簡易実装: where 句が空なら成功 *)
  if where_constraints = [] then true
  else
    (* Phase 2 Week 23-24: where 句がある場合は暫定的に常に成功 *)
    (* TODO: 再帰的な制約解決を実装 *)
    true

(** トレイト制約に一致するすべての impl を検索 *)
let find_matching_impls (constraint_ : trait_constraint)
    (registry : impl_registry) : impl_info list =
  (* 制約から型を抽出（通常は1要素だが、複数型引数も考慮） *)
  let target_ty =
    match constraint_.type_args with
    | [ ty ] -> ty
    | _ ->
        (* 複数型引数の場合はタプルとして扱う（Phase 2 後半で拡張） *)
        TTuple constraint_.type_args
  in

  (* トレイト名と型が一致する impl を検索 *)
  List.filter
    (fun impl_info ->
      impl_info.trait_name = constraint_.trait_name
      &&
      match unify_types impl_info.impl_type target_ty empty_subst with
      | Some subst ->
          (* 型照合成功 → where 句制約も検証 *)
          check_where_constraints impl_info.where_constraints subst
      | None -> false)
    registry.impls

(* ========== デバッグ用ユーティリティ ========== *)

(** impl_info の文字列表現 *)
let string_of_impl_info (impl_info : impl_info) : string =
  (* ジェネリック型パラメータ *)
  let generics_str =
    if impl_info.generic_params = [] then ""
    else
      "<"
      ^ String.concat ", "
          (List.map
             (fun tv ->
               match tv.tv_name with
               | Some name -> name
               | None -> "?" ^ string_of_int tv.tv_id)
             impl_info.generic_params)
      ^ ">"
  in

  (* where 句 *)
  let where_str =
    if impl_info.where_constraints = [] then ""
    else
      " where "
      ^ String.concat ", "
          (List.map string_of_trait_constraint impl_info.where_constraints)
  in

  (* メソッド *)
  let methods_str =
    if impl_info.methods = [] then ""
    else
      " { "
      ^ String.concat ", "
          (List.map (fun (name, _impl_name) -> name) impl_info.methods)
      ^ " }"
  in

  Printf.sprintf "impl%s %s for %s%s%s" generics_str impl_info.trait_name
    (string_of_ty impl_info.impl_type)
    where_str methods_str

(** レジストリの文字列表現 *)
let string_of_registry (registry : impl_registry) : string =
  if registry.impls = [] then "Impl Registry: (empty)"
  else
    "Impl Registry:\n"
    ^ String.concat "\n"
        (List.map
           (fun impl_info -> "  " ^ string_of_impl_info impl_info)
           registry.impls)
