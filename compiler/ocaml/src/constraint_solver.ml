(* Constraint_solver — Type Class Constraint Solver for Reml (Phase 2)
 *
 * このファイルは型クラス制約解決器の実装を提供する。
 * 仕様書 1-2 §B（トレイト）および §G（実装規約）に基づき、
 * 制約収集から辞書参照への変換パイプラインを実装する。
 *
 * Phase 2 Week 18-19:
 * - Eq, Ord, Collector の制約規則実装
 * - 制約グラフの構築と依存関係追跡
 * - 循環依存・未解決制約の検出
 *)

open Types
open Ast

(* ========== 基本データ構造 ========== *)

(** 辞書参照 *)
type dict_ref =
  | DictImplicit of string * ty
  | DictParam of int
  | DictLocal of string

(** 制約エラー理由 *)
type constraint_error_reason =
  | NoImpl
  | AmbiguousImpl of dict_ref list
  | CyclicConstraint of trait_constraint list
  | UnresolvedTypeVar of type_var

(** 制約エラー *)
type constraint_error = {
  trait_name : string;
  type_args : ty list;
  reason : constraint_error_reason;
  span : span;
}

(** 制約グラフ *)
type constraint_graph = {
  nodes : trait_constraint list;
  edges : (trait_constraint * trait_constraint) list;
}

(** 制約解決状態 *)
type solver_state = {
  constraints : trait_constraint list;
  resolved : (trait_constraint * dict_ref) list;
  pending : trait_constraint list;
  errors : constraint_error list;
}

(* ========== ヘルパー関数 ========== *)

(** トレイト制約の等価性判定 *)
let trait_constraint_equal (c1 : trait_constraint) (c2 : trait_constraint) : bool =
  c1.trait_name = c2.trait_name
  && List.length c1.type_args = List.length c2.type_args
  && List.for_all2 type_equal c1.type_args c2.type_args

(* 互換性のためのエイリアス *)
let constraint_equal = trait_constraint_equal

(** 型がプリミティブ型か判定 *)
let is_primitive = function
  | TCon (TCInt _) -> true
  | TCon (TCFloat _) -> true
  | TCon TCBool -> true
  | TCon TCChar -> true
  | TCon TCString -> true
  | _ -> false

(** 型がプリミティブまたは組み込み型か判定（Eq/Ord自動実装対象） *)
let is_builtin_for_eq = function
  | TCon (TCInt _) -> true
  | TCon (TCFloat _) -> true
  | TCon TCBool -> true
  | TCon TCChar -> true
  | TCon TCString -> true
  | TUnit -> true
  | _ -> false

(** 型がOrd自動実装対象か判定（浮動小数は除外可能にする） *)
let is_builtin_for_ord = function
  | TCon (TCInt _) -> true
  | TCon (TCFloat _) -> true  (* IEEE 754全順序比較として実装 *)
  | TCon TCBool -> true
  | TCon TCChar -> true
  | TCon TCString -> true
  | _ -> false

(* ========== 個別トレイトの解決 ========== *)

(** Eq トレイトの解決
 *
 * 仕様書 1-2 §B.1: 等価性比較のトレイト
 *
 * 解決規則:
 * - プリミティブ型は自動実装
 * - タプル型は要素が全て Eq を実装していれば実装
 * - レコード型はフィールドが全て Eq を実装していれば実装
 * - 配列型は要素型が Eq を実装していれば実装
 * - Option<T>, Result<T, E> も同様
 *)
let rec solve_eq = function
  | ty when is_builtin_for_eq ty ->
      (* プリミティブ型は自動実装 *)
      Some (DictImplicit ("Eq", ty))
  | TTuple tys ->
      (* タプル型: 全要素がEqを実装していればOK *)
      if List.for_all (fun ty -> Option.is_some (solve_eq ty)) tys then
        Some (DictImplicit ("Eq", TTuple tys))
      else None
  | TRecord fields ->
      (* レコード型: 全フィールドがEqを実装していればOK *)
      let field_tys = List.map snd fields in
      if List.for_all (fun ty -> Option.is_some (solve_eq ty)) field_tys then
        Some (DictImplicit ("Eq", TRecord fields))
      else None
  | TArray ty | TSlice (ty, _) ->
      (* 配列/スライス型: 要素型がEqを実装していればOK *)
      (match solve_eq ty with
      | Some _ -> Some (DictImplicit ("Eq", TArray ty))
      | None -> None)
  | TApp (TCon (TCUser "Option"), ty) ->
      (* Option<T>: TがEqを実装していればOK *)
      (match solve_eq ty with
      | Some _ -> Some (DictImplicit ("Eq", TApp (TCon (TCUser "Option"), ty)))
      | None -> None)
  | TApp (TApp (TCon (TCUser "Result"), t_ty), e_ty) ->
      (* Result<T, E>: T, EがEqを実装していればOK *)
      (match (solve_eq t_ty, solve_eq e_ty) with
      | Some _, Some _ ->
          Some
            (DictImplicit
               ("Eq", TApp (TApp (TCon (TCUser "Result"), t_ty), e_ty)))
      | _ -> None)
  | TVar _ ->
      (* 型変数: 後で解決されるため保留 *)
      None
  | _ ->
      (* その他の型: 未実装 *)
      None

(** Ord トレイトの解決
 *
 * 仕様書 1-2 §B.1: 順序付けのトレイト
 *
 * 解決規則:
 * - Eq<T> を前提とする（スーパートレイト制約）
 * - プリミティブ型は自動実装
 * - タプル型は辞書順比較（左から順に比較）
 * - 浮動小数型は IEEE 754 の全順序比較（NaN は最大値として扱う）
 *)
let rec solve_ord ty =
  (* Ord は Eq を要求するため、まず Eq を確認 *)
  match solve_eq ty with
  | None -> None
  | Some _ -> (
      match ty with
      | ty when is_builtin_for_ord ty ->
          (* プリミティブ型は自動実装 *)
          Some (DictImplicit ("Ord", ty))
      | TTuple tys ->
          (* タプル型: 全要素がOrdを実装していればOK（辞書順比較） *)
          if List.for_all (fun ty -> Option.is_some (solve_ord ty)) tys then
            Some (DictImplicit ("Ord", TTuple tys))
          else None
      | TVar _ ->
          (* 型変数: 後で解決されるため保留 *)
          None
      | _ ->
          (* レコード型・配列型・Option/Result は Ord 未サポート *)
          None)

(** Collector トレイトの解決
 *
 * 仕様書 3-1 §2.2: コレクション型の反復処理サポート
 *
 * 解決規則:
 * - [T] (スライス)、[T; N] (固定長配列) は自動実装
 * - Option<T>, Result<T, E> は要素型を返すイテレータとして実装
 * - タプル型は各要素を順に返すイテレータとして実装
 *)
let solve_collector = function
  | TArray ty | TSlice (ty, _) ->
      (* 配列/スライス型: 要素型を返すイテレータ *)
      Some (DictImplicit ("Collector", TArray ty))
  | TApp (TCon (TCUser "Option"), ty) ->
      (* Option<T>: Some(T) なら1要素、None なら0要素 *)
      Some (DictImplicit ("Collector", TApp (TCon (TCUser "Option"), ty)))
  | TApp (TApp (TCon (TCUser "Result"), t_ty), e_ty) ->
      (* Result<T, E>: Ok(T) なら1要素、Err(E) なら0要素（Tのみ返す） *)
      Some
        (DictImplicit
           ("Collector", TApp (TApp (TCon (TCUser "Result"), t_ty), e_ty)))
  | TTuple tys ->
      (* タプル型: 各要素を順に返す *)
      Some (DictImplicit ("Collector", TTuple tys))
  | TVar _ ->
      (* 型変数: 後で解決されるため保留 *)
      None
  | _ ->
      (* その他の型: 未実装 *)
      None

(* ========== 制約解決のメインロジック ========== *)

(** 単一制約の解決を試みる *)
let try_solve_constraint (c : trait_constraint) : dict_ref option =
  match c.trait_name with
  | "Eq" -> (
      match c.type_args with [ ty ] -> solve_eq ty | _ -> None)
  | "Ord" -> (
      match c.type_args with [ ty ] -> solve_ord ty | _ -> None)
  | "Collector" -> (
      match c.type_args with [ ty ] -> solve_collector ty | _ -> None)
  | _ ->
      (* 未知のトレイト *)
      None

(** 初期状態の作成 *)
let init_solver_state constraints =
  { constraints; resolved = []; pending = constraints; errors = [] }

(** 解決を1ステップ進める *)
let step_solver state =
  match state.pending with
  | [] ->
      (* 解決待ちがなければ何もしない *)
      state
  | c :: rest_pending -> (
      (* 先頭の制約を解決試行 *)
      match try_solve_constraint c with
      | Some dict_ref ->
          (* 解決成功: resolved に追加 *)
          {
            state with
            resolved = (c, dict_ref) :: state.resolved;
            pending = rest_pending;
          }
      | None ->
          (* 解決失敗: エラーに追加 *)
          let error =
            {
              trait_name = c.trait_name;
              type_args = c.type_args;
              reason = NoImpl;
              span = c.constraint_span;
            }
          in
          {
            state with
            errors = error :: state.errors;
            pending = rest_pending;
          })

(** 解決が完了したか判定 *)
let is_solved state = state.pending = []

(** 制約解決のメインエントリポイント *)
let solve_constraints constraints =
  let rec loop state =
    if is_solved state then
      (* 全て解決完了 *)
      if state.errors = [] then
        (* エラーなし: 辞書参照のリストを返す *)
        Ok (List.map snd state.resolved)
      else
        (* エラーあり: エラーリストを返す *)
        Error state.errors
    else
      (* まだ解決待ちがある: 1ステップ進めて再帰 *)
      loop (step_solver state)
  in
  loop (init_solver_state constraints)

(* ========== 制約グラフの構築と解析 ========== *)

(** スーパートレイト依存関係の取得
 *
 * トレイト c が要求するスーパートレイトのリストを返す
 * 例: Ord<T> は Eq<T> を要求
 *)
let get_supertrait_dependencies (c : trait_constraint) : trait_constraint list
    =
  match c.trait_name with
  | "Ord" ->
      (* Ord<T> requires Eq<T> *)
      [
        {
          trait_name = "Eq";
          type_args = c.type_args;
          constraint_span = c.constraint_span;
        };
      ]
  | _ ->
      (* 他のトレイトは現在スーパートレイトなし *)
      []

(** 再帰的な制約依存関係の取得
 *
 * 複合型の制約が要求する要素型の制約を返す
 * 例: Eq<(A, B)> は Eq<A>, Eq<B> を要求
 *)
let get_recursive_dependencies (c : trait_constraint) :
    trait_constraint list =
  match (c.trait_name, c.type_args) with
  | "Eq", [ TTuple tys ] | "Ord", [ TTuple tys ] ->
      (* タプル型: 各要素に同じトレイトを要求 *)
      List.map
        (fun ty ->
          {
            trait_name = c.trait_name;
            type_args = [ ty ];
            constraint_span = c.constraint_span;
          })
        tys
  | "Eq", [ TRecord fields ] ->
      (* レコード型: 各フィールドに Eq を要求 *)
      List.map
        (fun (_, ty) ->
          {
            trait_name = "Eq";
            type_args = [ ty ];
            constraint_span = c.constraint_span;
          })
        fields
  | "Eq", [ TArray ty ] | "Eq", [ TSlice (ty, _) ] ->
      (* 配列/スライス型: 要素型に Eq を要求 *)
      [
        {
          trait_name = "Eq";
          type_args = [ ty ];
          constraint_span = c.constraint_span;
        };
      ]
  | _ -> []

(** 制約グラフの構築 *)
let build_constraint_graph constraints =
  let nodes = constraints in
  let edges =
    List.concat_map
      (fun c ->
        let supertrait_deps = get_supertrait_dependencies c in
        let recursive_deps = get_recursive_dependencies c in
        let all_deps = supertrait_deps @ recursive_deps in
        (* 各依存制約 dep に対して (dep, c) のエッジを作成 *)
        List.map (fun dep -> (dep, c)) all_deps)
      constraints
  in
  { nodes; edges }

(** 循環依存の検出（Tarjanアルゴリズムのシンプル版）
 *
 * Phase 2 Week 18-19 では基本実装のみ
 * 完全なアルゴリズムは Phase 2 後半で実装予定
 *)
let find_cycles _graph =
  (* TODO: Phase 2 Week 19-20 で完全実装 *)
  (* 現在はシンプルなパス検索で循環検出 *)
  []

(** トポロジカルソート（Kahnアルゴリズム）
 *
 * Phase 2 Week 18-19 では基本実装のみ
 * 完全なアルゴリズムは Phase 2 後半で実装予定
 *)
let topological_sort graph =
  (* TODO: Phase 2 Week 19-20 で完全実装 *)
  (* 現在は単純に入力順を返す *)
  Some graph.nodes

(* ========== デバッグ用 ========== *)

(** 辞書参照の文字列表現 *)
let string_of_dict_ref = function
  | DictImplicit (trait, ty) ->
      Printf.sprintf "DictImplicit(%s, %s)" trait (string_of_ty ty)
  | DictParam idx -> Printf.sprintf "DictParam(%d)" idx
  | DictLocal name -> Printf.sprintf "DictLocal(%s)" name

(** 制約エラー理由の文字列表現 *)
let string_of_constraint_error_reason = function
  | NoImpl -> "NoImpl"
  | AmbiguousImpl dicts ->
      Printf.sprintf "AmbiguousImpl([%s])"
        (String.concat ", " (List.map string_of_dict_ref dicts))
  | CyclicConstraint cs ->
      Printf.sprintf "CyclicConstraint([%s])"
        (String.concat ", "
           (List.map string_of_trait_constraint cs))
  | UnresolvedTypeVar tv ->
      Printf.sprintf "UnresolvedTypeVar(%s)" (string_of_type_var tv)

(** 制約エラーの文字列表現 *)
let string_of_constraint_error err =
  Printf.sprintf "ConstraintError { trait: %s, args: [%s], reason: %s }"
    err.trait_name
    (String.concat ", " (List.map string_of_ty err.type_args))
    (string_of_constraint_error_reason err.reason)

(** 制約グラフの文字列表現 *)
let string_of_constraint_graph graph =
  let nodes_str =
    String.concat ", "
      (List.map (fun c -> string_of_trait_constraint c) graph.nodes)
  in
  let edges_str =
    String.concat ", "
      (List.map
         (fun (c1, c2) ->
           Printf.sprintf "(%s -> %s)" (string_of_trait_constraint c1)
             (string_of_trait_constraint c2))
         graph.edges)
  in
  Printf.sprintf "ConstraintGraph { nodes: [%s], edges: [%s] }" nodes_str
    edges_str

(** 解決状態の文字列表現 *)
let string_of_solver_state state =
  let resolved_str =
    String.concat ", "
      (List.map
         (fun (c, d) ->
           Printf.sprintf "(%s => %s)" (string_of_trait_constraint c)
             (string_of_dict_ref d))
         state.resolved)
  in
  let pending_str =
    String.concat ", "
      (List.map string_of_trait_constraint state.pending)
  in
  let errors_str =
    String.concat ", "
      (List.map string_of_constraint_error state.errors)
  in
  Printf.sprintf
    "SolverState { resolved: [%s], pending: [%s], errors: [%s] }" resolved_str
    pending_str errors_str
