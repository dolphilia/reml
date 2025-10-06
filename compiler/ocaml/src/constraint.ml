(* Constraint — Type Constraint System for Reml (Phase 2)
 *
 * このファイルは型制約システムの基盤を提供する。
 * 仕様書 1-2 §C（型推論）に従い、制約ベースの型推論を実装する。
 *
 * 設計原則:
 * - 制約収集と制約解決の分離
 * - Unify(τ₁, τ₂) の表現
 * - エラー位置情報の保持
 *)

open Types
open Ast

(* ========== 制約の定義 ========== *)

(** 制約の種類 *)
type constraint_kind =
  | Unify of ty * ty                          (** τ₁ = τ₂ の単一化制約 *)

(** 制約（位置情報付き） *)
type constraint_ = {
  kind: constraint_kind;
  span: span;                                 (** エラー報告用の位置情報 *)
}

(** 制約集合 *)
type constraint_set = constraint_ list

(* ========== 制約の構築 ========== *)

(** 単一化制約を作成 *)
let unify_constraint t1 t2 span =
  { kind = Unify (t1, t2); span }

(** 空の制約集合 *)
let empty_constraints = []

(** 制約を追加 *)
let add_constraint c cs = c :: cs

(** 制約集合を結合 *)
let merge_constraints cs1 cs2 = cs1 @ cs2

(* ========== 代入（Substitution） ========== *)

(** 型代入: 型変数 → 型 のマッピング *)
type substitution = (type_var * ty) list

(** 空の代入 *)
let empty_subst = []

(** 型変数が代入に含まれるか確認 *)
let subst_mem tv subst =
  List.exists (fun (tv', _) -> tv.tv_id = tv'.tv_id) subst

(** 代入から型変数に対応する型を取得 *)
let subst_lookup tv subst =
  List.find_opt (fun (tv', _) -> tv.tv_id = tv'.tv_id) subst
  |> Option.map snd

(** 型への代入適用: subst(τ) *)
let rec apply_subst subst = function
  | TVar tv as t ->
      (match subst_lookup tv subst with
       | Some t' -> t'
       | None -> t)
  | TCon _ as t -> t
  | TApp (t1, t2) ->
      TApp (apply_subst subst t1, apply_subst subst t2)
  | TArrow (t1, t2) ->
      TArrow (apply_subst subst t1, apply_subst subst t2)
  | TTuple tys ->
      TTuple (List.map (apply_subst subst) tys)
  | TRecord fields ->
      TRecord (List.map (fun (name, ty) ->
        (name, apply_subst subst ty)
      ) fields)
  | TArray t -> TArray (apply_subst subst t)
  | TSlice (t, n) -> TSlice (apply_subst subst t, n)
  | TUnit -> TUnit
  | TNever -> TNever

(** 代入の合成: s1 ∘ s2
 *
 * (s1 ∘ s2)(τ) = s1(s2(τ))
 *)
let compose_subst s1 s2 =
  (* s2 の各マッピングに s1 を適用 *)
  let s2' = List.map (fun (tv, ty) -> (tv, apply_subst s1 ty)) s2 in
  (* s1 と s2' を結合（s1 が優先） *)
  s1 @ s2'

(** 型スキームへの代入適用
 *
 * 量化変数は代入から除外する
 *)
let apply_subst_scheme subst scheme =
  (* 量化変数を代入から除外 *)
  let subst' = List.filter (fun (tv, _) ->
    not (List.exists (fun qtv -> qtv.tv_id = tv.tv_id) scheme.quantified)
  ) subst in
  { scheme with body = apply_subst subst' scheme.body }

(** 型環境への代入適用 *)
let apply_subst_env subst env =
  Type_env.extend_many
    (List.map (fun (name, scheme) ->
      (name, apply_subst_scheme subst scheme)
    ) (Type_env.bindings env))
    Type_env.empty

(* ========== 自由型変数（Free Type Variables） ========== *)

(** 型に含まれる自由型変数を収集 *)
let rec ftv_ty = function
  | TVar tv -> [tv]
  | TCon _ -> []
  | TApp (t1, t2) -> ftv_ty t1 @ ftv_ty t2
  | TArrow (t1, t2) -> ftv_ty t1 @ ftv_ty t2
  | TTuple tys -> List.concat_map ftv_ty tys
  | TRecord fields -> List.concat_map (fun (_, ty) -> ftv_ty ty) fields
  | TArray t -> ftv_ty t
  | TSlice (t, _) -> ftv_ty t
  | TUnit -> []
  | TNever -> []

(** 型スキームに含まれる自由型変数を収集
 *
 * 量化変数は除外
 *)
let ftv_scheme scheme =
  let all_vars = ftv_ty scheme.body in
  List.filter (fun tv ->
    not (List.exists (fun qtv -> qtv.tv_id = tv.tv_id) scheme.quantified)
  ) all_vars

(** 型環境に含まれる自由型変数を収集 *)
let ftv_env env =
  List.concat_map (fun (_, scheme) ->
    ftv_scheme scheme
  ) (Type_env.bindings env)

(* ========== 制約解決（Phase 2 Week 3-4 で実装） ========== *)

(** 型エラーは type_error.ml で定義 *)
open Type_error

(** let* 演算子（Result モナド） *)
let (let*) = Result.bind

(** Occurs check: tv が ty に出現するか確認
 *
 * 無限型（例: α = α -> α）を検出
 *)
let rec occurs_check tv ty =
  match ty with
  | TVar tv' -> tv.tv_id = tv'.tv_id
  | TCon _ -> false
  | TApp (t1, t2) -> occurs_check tv t1 || occurs_check tv t2
  | TArrow (t1, t2) -> occurs_check tv t1 || occurs_check tv t2
  | TTuple tys -> List.exists (occurs_check tv) tys
  | TRecord fields -> List.exists (fun (_, t) -> occurs_check tv t) fields
  | TArray t -> occurs_check tv t
  | TSlice (t, _) -> occurs_check tv t
  | TUnit -> false
  | TNever -> false

(** 単一化アルゴリズム: unify(s, τ₁, τ₂)
 *
 * 仕様書 1-2 §G.1: 対称・逐次、occurs check あり
 *
 * Phase 2 Week 3-4 で詳細実装予定
 * 現在は基本的な構造のみ
 *)
let rec unify subst t1 t2 span =
  (* 代入を適用 *)
  let t1' = apply_subst subst t1 in
  let t2' = apply_subst subst t2 in

  match (t1', t2') with
  | (TVar tv1, TVar tv2) when tv1.tv_id = tv2.tv_id ->
      (* 同じ型変数 *)
      Ok subst

  | (TVar tv, t) | (t, TVar tv) ->
      (* Occurs check *)
      if occurs_check tv t then
        Error (occurs_check_error tv t span)
      else
        (* 新しい代入を追加 *)
        Ok ((tv, t) :: subst)

  | (TCon tc1, TCon tc2) when tc1 = tc2 ->
      (* 同じ型定数 *)
      Ok subst

  | (TApp (t11, t12), TApp (t21, t22)) ->
      (* 型適用: 両方の要素を単一化 *)
      let* s1 = unify subst t11 t21 span in
      unify s1 t12 t22 span

  | (TArrow (t11, t12), TArrow (t21, t22)) ->
      (* 関数型: 引数と返り値を単一化 *)
      let* s1 = unify subst t11 t21 span in
      unify s1 t12 t22 span

  | (TTuple tys1, TTuple tys2) when List.length tys1 = List.length tys2 ->
      (* タプル型: 各要素を単一化 *)
      unify_list subst (List.combine tys1 tys2) span

  | (TRecord fields1, TRecord fields2) ->
      (* レコード型: フィールドを単一化 *)
      unify_record subst fields1 fields2 span

  | (TArray t1, TArray t2) | (TSlice (t1, _), TSlice (t2, _)) ->
      (* 配列/スライス型: 要素型を単一化 *)
      unify subst t1 t2 span

  | (TUnit, TUnit) ->
      (* ユニット型 *)
      Ok subst

  | (TNever, _) | (_, TNever) ->
      (* Never型はどの型とも単一化可能 *)
      Ok subst

  | _ ->
      (* 型不一致 *)
      Error (unification_error t1' t2' span)

(** リストの単一化 *)
and unify_list subst pairs span =
  List.fold_left (fun acc (t1, t2) ->
    match acc with
    | Ok s -> unify s t1 t2 span
    | err -> err
  ) (Ok subst) pairs

(** レコード型の単一化 *)
and unify_record subst fields1 fields2 span =
  (* フィールド名でソート *)
  let sorted1 = List.sort (fun (n1, _) (n2, _) -> String.compare n1 n2) fields1 in
  let sorted2 = List.sort (fun (n1, _) (n2, _) -> String.compare n1 n2) fields2 in

  (* フィールド数と名前が一致するか確認 *)
  if List.length sorted1 <> List.length sorted2 then
    Error (unification_error (TRecord fields1) (TRecord fields2) span)
  else
    try
      let pairs = List.map2 (fun (n1, t1) (n2, t2) ->
        if n1 <> n2 then
          raise (Failure "Field name mismatch")
        else
          (t1, t2)
      ) sorted1 sorted2 in
      unify_list subst pairs span
    with Failure _ ->
      Error (unification_error (TRecord fields1) (TRecord fields2) span)

(** 制約解決: solve(constraints)
 *
 * Phase 2 Week 3-4 で詳細実装予定
 *)
let solve constraints =
  List.fold_left (fun acc c ->
    match acc with
    | Ok subst ->
        (match c.kind with
         | Unify (t1, t2) -> unify subst t1 t2 c.span)
    | err -> err
  ) (Ok empty_subst) constraints

(* ========== デバッグ用 ========== *)

(** 制約の文字列表現 *)
let string_of_constraint c =
  match c.kind with
  | Unify (t1, t2) ->
      Printf.sprintf "Unify(%s, %s) at %d:%d"
        (string_of_ty t1) (string_of_ty t2)
        c.span.start c.span.end_

(** 代入の文字列表現 *)
let string_of_subst subst =
  let bindings = List.map (fun (tv, ty) ->
    Printf.sprintf "%s := %s" (string_of_type_var tv) (string_of_ty ty)
  ) subst in
  "[" ^ String.concat ", " bindings ^ "]"

(* 型エラーの文字列表現は type_error.ml で定義 *)
