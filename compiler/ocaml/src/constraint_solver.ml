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
 *
 * Phase 2 Week 23-24:
 * - Impl_registry との統合
 * - ユーザー定義 impl 宣言の検索対応
 *)

open Types
open Ast
module Profile = Effect_profile

(* ========== 効果制約テーブル ========== *)

module EffectConstraintTable = struct
  module StringMap = Map.Make (String)
  module StringSet = Set.Make (String)

  type entry = {
    symbol : string;
    effect_set : Profile.set;
    stage_requirement : Profile.stage_requirement;
    source_span : span;
    source_name : string option;
    resolved_stage : Profile.stage_id option;
    resolved_capability : string option;
    resolved_capabilities : Profile.capability_resolution list;
    stage_trace : Profile.stage_trace;
    diagnostic_payload : Profile.effect_diagnostic_payload;
  }

  type t = entry StringMap.t

  let empty () = StringMap.empty
  let normalize_symbol symbol = symbol |> String.trim |> String.lowercase_ascii

  let add_entry table entry =
    StringMap.add (normalize_symbol entry.symbol) entry table

  let add_effects table ~symbol ~effect_set ~stage_requirement ~source_span
      ?source_name ?resolved_stage ?resolved_capability
      ?(resolved_capabilities = [])
      ?(stage_trace = Profile.stage_trace_empty)
      ?(diagnostic_payload = Profile.empty_diagnostic_payload) () =
    let entry =
      {
        symbol;
        effect_set;
        stage_requirement;
        source_span;
        source_name;
        resolved_stage;
        resolved_capability;
        resolved_capabilities;
        stage_trace;
        diagnostic_payload;
      }
    in
    add_entry table entry

  let add_profile table ~symbol (profile : Profile.profile) =
    add_effects table ~symbol ~effect_set:profile.effect_set
      ~stage_requirement:profile.stage_requirement
      ~source_span:profile.source_span ?source_name:profile.source_name
      ?resolved_stage:profile.resolved_stage
      ?resolved_capability:profile.resolved_capability
      ~resolved_capabilities:profile.resolved_capabilities
      ~stage_trace:profile.stage_trace
      ~diagnostic_payload:profile.diagnostic_payload ()

  let merge ~into ~from = StringMap.union (fun _ _ rhs -> Some rhs) into from
  let resolve table ~symbol = StringMap.find_opt (normalize_symbol symbol) table

  let effect_set table ~symbol =
    resolve table ~symbol |> Option.map (fun entry -> entry.effect_set)

  let names_of_tags tags =
    List.fold_left
      (fun acc tag ->
        let name =
          tag.Profile.effect_name |> String.trim |> String.lowercase_ascii
        in
        StringSet.add name acc)
      StringSet.empty tags

  let effect_names (set : Profile.set) =
    let declared = names_of_tags set.Profile.declared in
    let residual = names_of_tags set.Profile.residual in
    StringSet.union declared residual

  let includes ~super ~sub =
    let super_names = effect_names super in
    let sub_names = effect_names sub in
    StringSet.for_all (fun name -> StringSet.mem name super_names) sub_names

  let to_list table =
    table |> StringMap.bindings |> List.map (fun (_, entry) -> entry)
end

let effect_constraints = ref (EffectConstraintTable.empty ())

let reset_effect_constraints () =
  effect_constraints := EffectConstraintTable.empty ()

let record_effect_profile ~symbol (profile : Profile.profile) =
  effect_constraints :=
    EffectConstraintTable.add_profile !effect_constraints ~symbol profile

let record_effect_set ~symbol ~effect_set ~stage_requirement ~source_span
    ?source_name ?(diagnostic_payload = Profile.empty_diagnostic_payload) () =
  effect_constraints :=
    EffectConstraintTable.add_effects !effect_constraints ~symbol ~effect_set
      ~stage_requirement ~source_span ?source_name ~diagnostic_payload ()

let resolve_effect_profile ~symbol =
  EffectConstraintTable.resolve !effect_constraints ~symbol

let effect_set_for ~symbol =
  EffectConstraintTable.effect_set !effect_constraints ~symbol

let current_effect_constraints () = !effect_constraints
let effect_set_includes ~super ~sub = EffectConstraintTable.includes ~super ~sub

(* ========== 基本データ構造 ========== *)

(** 辞書参照 *)
type dict_ref =
  | DictImplicit of string * ty list
  | DictParam of int
  | DictLocal of string

type iterator_dict_kind =
  | IteratorArrayLike
  | IteratorCoreIter
  | IteratorOptionLike
  | IteratorResultLike
  | IteratorCustom of string

type iterator_stage_requirement =
  | IteratorStageExact of string
  | IteratorStageAtLeast of string

type iterator_dict_info = {
  dict_ref : dict_ref;
  source_ty : ty;
  element_ty : ty;
  kind : iterator_dict_kind;
  stage_requirement : iterator_stage_requirement;
  capability : string option;
  stage_actual : string;
}

(** 制約エラー理由 *)
type constraint_error_reason =
  | NoImpl
  | AmbiguousImpl of dict_ref list
  | CyclicConstraint of trait_constraint list
  | StageMismatch of {
      required : iterator_stage_requirement;
      actual : string option;
      capability : string option;
      iterator_kind : iterator_dict_kind option;
      iterator_source : string option;
      provider : string option;
      manifest_path : string option;
      stage_trace : Profile.stage_trace;
    }
  | UnresolvedTypeVar of type_var

type constraint_error = {
  trait_name : string;
  type_args : ty list;
  reason : constraint_error_reason;
  span : span;
}
(** 制約エラー *)

type constraint_graph = {
  nodes : trait_constraint list;
  edges : (trait_constraint * trait_constraint) list;
}
(** 制約グラフ *)

type solver_state = {
  constraints : trait_constraint list;
  resolved : (trait_constraint * dict_ref) list;
  pending : trait_constraint list;
  errors : constraint_error list;
}
(** 制約解決状態 *)

(* ========== ヘルパー関数 ========== *)

(** トレイト制約の等価性判定 *)
let trait_constraint_equal (c1 : trait_constraint) (c2 : trait_constraint) :
    bool =
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
  | TCon (TCFloat _) -> true (* IEEE 754全順序比較として実装 *)
  | TCon TCBool -> true
  | TCon TCChar -> true
  | TCon TCString -> true
  | _ -> false

let normalize_stage_name (stage : string) = String.lowercase_ascii stage

let stage_rank stage =
  match normalize_stage_name stage with
  | "stable" | "release" -> 3
  | "beta" -> 2
  | "alpha" -> 1
  | _ -> 0

let stage_requirement_satisfied requirement actual_stage =
  let actual_norm = normalize_stage_name actual_stage in
  match requirement with
  | IteratorStageExact stage ->
      String.equal (normalize_stage_name stage) actual_norm
  | IteratorStageAtLeast stage ->
      let required_rank = stage_rank stage in
      let actual_rank = stage_rank actual_norm in
      required_rank > 0 && actual_rank >= required_rank

let[@warning "-32"] string_of_stage_requirement = function
  | IteratorStageExact stage -> Printf.sprintf "exact:%s" stage
  | IteratorStageAtLeast stage -> Printf.sprintf "at_least:%s" stage

(** 型適用のヘッドと引数を抽出 *)
let rec head_type_and_args ty =
  match ty with
  | TApp (fn, arg) ->
      let head, args = head_type_and_args fn in
      (head, args @ [ arg ])
  | _ -> (ty, [])

(** ユーザー定義型か判定し、ヘッド名と引数を返す *)
let as_user_type name ty =
  let head, args = head_type_and_args ty in
  match head with
  | TCon (TCUser type_name) when String.equal type_name name -> Some args
  | _ -> None

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
      Some (DictImplicit ("Eq", [ ty ]))
  | TTuple tys ->
      (* タプル型: 全要素がEqを実装していればOK *)
      if List.for_all (fun ty -> Option.is_some (solve_eq ty)) tys then
        Some (DictImplicit ("Eq", [ TTuple tys ]))
      else None
  | TRecord fields ->
      (* レコード型: 全フィールドがEqを実装していればOK *)
      let field_tys = List.map snd fields in
      if List.for_all (fun ty -> Option.is_some (solve_eq ty)) field_tys then
        Some (DictImplicit ("Eq", [ TRecord fields ]))
      else None
  | TArray ty | TSlice (ty, _) -> (
      (* 配列/スライス型: 要素型がEqを実装していればOK *)
      match solve_eq ty with
      | Some _ -> Some (DictImplicit ("Eq", [ TArray ty ]))
      | None -> None)
  | TApp (TCon (TCUser "Option"), ty) -> (
      (* Option<T>: TがEqを実装していればOK *)
      match solve_eq ty with
      | Some _ ->
          Some (DictImplicit ("Eq", [ TApp (TCon (TCUser "Option"), ty) ]))
      | None -> None)
  | TApp (TApp (TCon (TCUser "Result"), t_ty), e_ty) -> (
      (* Result<T, E>: T, EがEqを実装していればOK *)
      match (solve_eq t_ty, solve_eq e_ty) with
      | Some _, Some _ ->
          Some
            (DictImplicit
               ("Eq", [ TApp (TApp (TCon (TCUser "Result"), t_ty), e_ty) ]))
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
          Some (DictImplicit ("Ord", [ ty ]))
      | TTuple tys ->
          (* タプル型: 全要素がOrdを実装していればOK（辞書順比較） *)
          if List.for_all (fun ty -> Option.is_some (solve_ord ty)) tys then
            Some (DictImplicit ("Ord", [ TTuple tys ]))
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
      Some (DictImplicit ("Collector", [ TArray ty ]))
  | TApp (TCon (TCUser "Option"), ty) ->
      (* Option<T>: Some(T) なら1要素、None なら0要素 *)
      Some (DictImplicit ("Collector", [ TApp (TCon (TCUser "Option"), ty) ]))
  | TApp (TApp (TCon (TCUser "Result"), t_ty), e_ty) ->
      (* Result<T, E>: Ok(T) なら1要素、Err(E) なら0要素（Tのみ返す） *)
      Some
        (DictImplicit
           ("Collector", [ TApp (TApp (TCon (TCUser "Result"), t_ty), e_ty) ]))
  | TTuple tys ->
      (* タプル型: 各要素を順に返す *)
      Some (DictImplicit ("Collector", [ TTuple tys ]))
  | TVar _ ->
      (* 型変数: 後で解決されるため保留 *)
      None
  | _ ->
      (* その他の型: 未実装 *)
      None

(** Iterator トレイトの解決
 *
 * 仕様書 3-1 §3: Core.Iter による反復処理
 *
 * 解決規則:
 * - 配列/スライスは要素型を返すイテレータとして実装
 * - `Core.Iter.Iter<T>` は T を要素型としてそのまま返す
 * - それ以外はユーザー定義 impl を探索
 *)
let capability_for_kind = function
  | IteratorArrayLike -> Some "core.iter.array"
  | IteratorCoreIter -> Some "core.iter.core"
  | IteratorOptionLike -> Some "core.iter.option"
  | IteratorResultLike -> Some "core.iter.result"
  | IteratorCustom _ -> None

let stage_requirement_for_kind = function
  | IteratorArrayLike -> IteratorStageExact "stable"
  | IteratorCoreIter -> IteratorStageAtLeast "beta"
  | IteratorOptionLike -> IteratorStageAtLeast "beta"
  | IteratorResultLike -> IteratorStageAtLeast "beta"
  | IteratorCustom _ -> IteratorStageAtLeast "beta"

let stage_actual_for_kind = function
  | IteratorArrayLike -> "stable"
  | IteratorCoreIter -> "beta"
  | IteratorOptionLike -> "beta"
  | IteratorResultLike -> "beta"
  | IteratorCustom _ -> "unknown"

let string_of_iterator_kind = function
  | IteratorArrayLike -> "array_like"
  | IteratorCoreIter -> "core_iter"
  | IteratorOptionLike -> "option_like"
  | IteratorResultLike -> "result_like"
  | IteratorCustom name -> Printf.sprintf "custom:%s" name

let make_iterator_info kind source_ty element_ty =
  {
    dict_ref = DictImplicit ("Iterator", [ source_ty; element_ty ]);
    source_ty;
    element_ty;
    kind;
    stage_requirement = stage_requirement_for_kind kind;
    capability = capability_for_kind kind;
    stage_actual = stage_actual_for_kind kind;
  }

let solve_iterator (source_ty : ty) : iterator_dict_info option =
  match source_ty with
  | TArray ty -> Some (make_iterator_info IteratorArrayLike source_ty ty)
  | TSlice (ty, _) -> Some (make_iterator_info IteratorArrayLike source_ty ty)
  | _ -> (
      match as_user_type "Array" source_ty with
      | Some [ elem_ty ] ->
          Some (make_iterator_info IteratorArrayLike source_ty elem_ty)
      | _ -> (
          match as_user_type "Iter" source_ty with
          | Some [ elem_ty ] ->
              Some (make_iterator_info IteratorCoreIter source_ty elem_ty)
          | _ -> (
              match as_user_type "IteratorState" source_ty with
              | Some [ elem_ty ] ->
                  Some (make_iterator_info IteratorCoreIter source_ty elem_ty)
              | _ -> (
                  match as_user_type "Option" source_ty with
                  | Some [ elem_ty ] ->
                      Some
                        (make_iterator_info IteratorOptionLike source_ty elem_ty)
                  | _ -> (
                      match as_user_type "Result" source_ty with
                      | Some (ok_ty :: _err_ty :: _) ->
                          Some
                            (make_iterator_info IteratorResultLike source_ty
                               ok_ty)
                      | _ -> None)))))

(* ========== 制約解決のメインロジック ========== *)

(** 単一制約の解決を試みる
 *
 * Phase 2 Week 23-24 更新: レジストリパラメータを追加
 *
 * 解決戦略:
 * 1. 組み込み型の自動実装をチェック（solve_eq, solve_ord, solve_collector）
 * 2. レジストリからユーザー定義impl宣言を検索
 * 3. どちらも見つからない場合はNone
 *)
let try_solve_constraint (registry : Impl_registry.impl_registry)
    (c : trait_constraint) : dict_ref option =
  (* 組み込み型の自動実装を優先チェック *)
  let builtin_result =
    match c.trait_name with
    | "Eq" -> ( match c.type_args with [ ty ] -> solve_eq ty | _ -> None)
    | "Ord" -> ( match c.type_args with [ ty ] -> solve_ord ty | _ -> None)
    | "Collector" -> (
        match c.type_args with [ ty ] -> solve_collector ty | _ -> None)
    | "Iterator" -> (
        match c.type_args with
        | source_ty :: _ -> (
            match solve_iterator source_ty with
            | Some info -> Some info.dict_ref
            | None -> None)
        | _ -> None)
    | _ -> None
  in

  match builtin_result with
  | Some dict_ref -> Some dict_ref
  | None -> (
      (* 組み込み型で見つからない場合、レジストリから検索 *)
      let matching_impls = Impl_registry.find_matching_impls c registry in
      match matching_impls with
      | [] ->
          (* 一致するimplが見つからない *)
          None
      | [ impl_info ] ->
          (* 一意にimplが決定 *)
          (* Phase 2 Week 23-24: 簡易実装では最初に見つかったimplを使用 *)
          Some (DictImplicit (impl_info.trait_name, c.type_args))
      | _ ->
          (* 複数のimplが一致（曖昧性エラー）*)
          (* TODO: AmbiguousImpl エラーを返すべきだが、現在の戻り値型がoption *)
          (* Phase 2 後半でエラーハンドリングを改善 *)
          None)

(** 初期状態の作成 *)
let init_solver_state constraints =
  { constraints; resolved = []; pending = constraints; errors = [] }

(** 解決を1ステップ進める
 *
 * Phase 2 Week 23-24 更新: レジストリパラメータを追加
 *)
let step_solver (registry : Impl_registry.impl_registry) (state : solver_state)
    : solver_state =
  match state.pending with
  | [] ->
      (* 解決待ちがなければ何もしない *)
      state
  | c :: rest_pending -> (
      (* 先頭の制約を解決試行 *)
      match try_solve_constraint registry c with
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
          { state with errors = error :: state.errors; pending = rest_pending })

(** 解決が完了したか判定 *)
let is_solved state = state.pending = []

(* ========== 制約グラフの構築と解析 ========== *)

(** スーパートレイト依存関係の取得
 *
 * トレイト c が要求するスーパートレイトのリストを返す
 * 例: Ord<T> は Eq<T> を要求
 *)
let get_supertrait_dependencies (c : trait_constraint) : trait_constraint list =
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
let get_recursive_dependencies (c : trait_constraint) : trait_constraint list =
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

(** Tarjanアルゴリズムによる強連結成分検出
 *
 * 強連結成分（Strongly Connected Components, SCC）を検出し、
 * サイズ2以上のSCCを循環依存として返す。
 *
 * アルゴリズム:
 * 1. DFSで各ノードにindexとlowlinkを割り当て
 * 2. スタックを使ってSCCを識別
 * 3. lowlink == indexとなったノードがSCCのルート
 *
 * 参考: Robert Tarjan, "Depth-First Search and Linear Graph Algorithms" (1972)
 *)
let find_cycles graph =
  (* Tarjanアルゴリズムの状態 *)
  let index_counter = ref 0 in
  let stack = ref [] in
  let indices = Hashtbl.create 10 in
  let lowlinks = Hashtbl.create 10 in
  let on_stack = Hashtbl.create 10 in
  let sccs = ref [] in

  (* 制約のハッシュ値を生成（Hashtblのキーとして使用） *)
  let constraint_hash (c : trait_constraint) =
    Hashtbl.hash (c.trait_name, List.map (fun ty -> string_of_ty ty) c.type_args)
  in

  (* 制約が訪問済みか確認 *)
  let is_visited (c : trait_constraint) =
    Hashtbl.mem indices (constraint_hash c)
  in

  (* 隣接ノード（依存先）を取得 *)
  let get_neighbors (c : trait_constraint) : trait_constraint list =
    List.filter_map
      (fun (dep, target) ->
        if constraint_equal target c then Some dep else None)
      graph.edges
  in

  (* Tarjan DFS *)
  let rec strongconnect (node_v : trait_constraint) : unit =
    let v_hash = constraint_hash node_v in
    (* node_vにindexとlowlinkを割り当て *)
    Hashtbl.add indices v_hash !index_counter;
    Hashtbl.add lowlinks v_hash !index_counter;
    index_counter := !index_counter + 1;

    (* スタックにプッシュ *)
    stack := node_v :: !stack;
    Hashtbl.add on_stack v_hash true;

    (* 隣接ノードを探索 *)
    List.iter
      (fun node_w ->
        let w_hash = constraint_hash node_w in
        if not (is_visited node_w) then (
          (* node_wが未訪問ならDFS *)
          strongconnect node_w;
          (* lowlinkを更新 *)
          let v_lowlink = Hashtbl.find lowlinks v_hash in
          let w_lowlink = Hashtbl.find lowlinks w_hash in
          Hashtbl.replace lowlinks v_hash (min v_lowlink w_lowlink))
        else if Hashtbl.mem on_stack w_hash then
          (* node_wがスタック上にある（バックエッジ） *)
          let v_lowlink = Hashtbl.find lowlinks v_hash in
          let w_index = Hashtbl.find indices w_hash in
          Hashtbl.replace lowlinks v_hash (min v_lowlink w_index))
      (get_neighbors node_v);

    (* node_v がSCCのルートか確認 *)
    let v_index = Hashtbl.find indices v_hash in
    let v_lowlink = Hashtbl.find lowlinks v_hash in
    if v_lowlink = v_index then
      (* SCCを抽出 *)
      let rec pop_scc acc =
        match !stack with
        | [] -> acc
        | node_w :: rest ->
            stack := rest;
            Hashtbl.remove on_stack (constraint_hash node_w);
            if constraint_equal node_w node_v then node_w :: acc
            else pop_scc (node_w :: acc)
      in
      let scc = pop_scc [] in
      sccs := scc :: !sccs
  in

  (* 全ノードを探索 *)
  List.iter
    (fun node -> if not (is_visited node) then strongconnect node)
    graph.nodes;

  (* サイズ2以上のSCCを循環依存として返す *)
  List.filter (fun scc -> List.length scc >= 2) !sccs

(** Kahnアルゴリズムによるトポロジカルソート
 *
 * 制約グラフをトポロジカル順にソートする。
 * 循環依存がある場合はNoneを返す。
 *
 * アルゴリズム:
 * 1. 各ノードの入次数を計算
 * 2. 入次数0のノードをキューに追加
 * 3. キューからノードを取り出し、結果リストに追加
 * 4. 隣接ノードの入次数を減らし、0になったらキューに追加
 * 5. 全ノードが処理されたらSome、そうでなければNone
 *
 * 参考: Arthur B. Kahn, "Topological Sorting of Large Networks" (1962)
 *)
let topological_sort graph =
  (* 入次数を計算 *)
  let in_degrees = Hashtbl.create (List.length graph.nodes) in

  (* 制約のハッシュ値を生成 *)
  let constraint_hash (c : trait_constraint) =
    Hashtbl.hash (c.trait_name, List.map (fun ty -> string_of_ty ty) c.type_args)
  in

  (* 全ノードの入次数を0で初期化 *)
  List.iter
    (fun (node : trait_constraint) ->
      Hashtbl.add in_degrees (constraint_hash node) 0)
    graph.nodes;

  (* エッジから入次数を計算 *)
  List.iter
    (fun ((_dep : trait_constraint), (target : trait_constraint)) ->
      let target_hash = constraint_hash target in
      let current = Hashtbl.find in_degrees target_hash in
      Hashtbl.replace in_degrees target_hash (current + 1))
    graph.edges;

  (* 入次数0のノードをキューに追加 *)
  let queue = Queue.create () in
  List.iter
    (fun (node : trait_constraint) ->
      let node_hash = constraint_hash node in
      if Hashtbl.find in_degrees node_hash = 0 then Queue.add node queue)
    graph.nodes;

  (* トポロジカルソート *)
  let result = ref [] in
  let processed_count = ref 0 in

  while not (Queue.is_empty queue) do
    let node = Queue.take queue in
    result := node :: !result;
    processed_count := !processed_count + 1;

    (* 隣接ノード（依存元）を取得 *)
    let neighbors =
      List.filter_map
        (fun ((dep : trait_constraint), (target : trait_constraint)) ->
          if constraint_equal dep node then Some target else None)
        graph.edges
    in

    (* 隣接ノードの入次数を減らす *)
    List.iter
      (fun (neighbor : trait_constraint) ->
        let neighbor_hash = constraint_hash neighbor in
        let current = Hashtbl.find in_degrees neighbor_hash in
        let new_degree = current - 1 in
        Hashtbl.replace in_degrees neighbor_hash new_degree;
        if new_degree = 0 then Queue.add neighbor queue)
      neighbors
  done;

  (* 全ノードが処理されたか確認 *)
  if !processed_count = List.length graph.nodes then Some (List.rev !result)
  else None
(* 循環依存がある *)

(* ========== 制約解決のエントリポイント ========== *)

(** 制約解決のメインエントリポイント
 *
 * Phase 2 Week 20-21 更新: 循環依存検出を統合
 * Phase 2 Week 23-24 更新: レジストリパラメータを追加
 *
 * 制約グラフを構築し、循環依存がある場合はエラーを返す。
 * ユーザー定義impl宣言の検索にレジストリを使用する。
 *)
let solve_constraints (registry : Impl_registry.impl_registry)
    (constraints : trait_constraint list) :
    (dict_ref list, constraint_error list) result =
  (* Week 20-21 実装: 循環依存を事前検出 *)
  let graph = build_constraint_graph constraints in
  let cycles = find_cycles graph in

  (* 循環依存がある場合はエラーを返す *)
  if cycles <> [] then
    let first_cycle = List.hd cycles in
    let error =
      {
        trait_name = (List.hd first_cycle).trait_name;
        type_args = (List.hd first_cycle).type_args;
        reason = CyclicConstraint first_cycle;
        span = (List.hd first_cycle).constraint_span;
      }
    in
    Error [ error ]
  else
    (* 循環依存なし: 通常の解決フローへ *)
    let rec loop state =
      if is_solved state then
        (* 全て解決完了 *)
        if state.errors = [] then
          (* エラーなし: 辞書参照のリストを返す *)
          Ok (List.map snd state.resolved)
        else (* エラーあり: エラーリストを返す *)
          Error state.errors
      else (* まだ解決待ちがある: 1ステップ進めて再帰 *)
        loop (step_solver registry state)
    in
    loop (init_solver_state constraints)

(** Iterator 制約を辞書メタデータ付きで解決 *)
let solve_iterator_dict (registry : Impl_registry.impl_registry)
    (constraint_ : trait_constraint) :
    (iterator_dict_info, constraint_error) result =
  if not (String.equal constraint_.trait_name "Iterator") then
    Error
      {
        trait_name = constraint_.trait_name;
        type_args = constraint_.type_args;
        reason = NoImpl;
        span = constraint_.constraint_span;
      }
  else
    match constraint_.type_args with
    | source_ty :: item_ty :: _ -> (
        match solve_constraints registry [ constraint_ ] with
        | Ok [ dict_ref ] ->
            let base_info =
              match solve_iterator source_ty with
              | Some info -> { info with dict_ref }
              | None ->
                  let custom_name = Types.string_of_ty source_ty in
                  {
                    dict_ref;
                    source_ty;
                    element_ty = item_ty;
                    kind = IteratorCustom custom_name;
                    stage_requirement =
                      stage_requirement_for_kind (IteratorCustom custom_name);
                    capability = None;
                    stage_actual =
                      stage_actual_for_kind (IteratorCustom custom_name);
                  }
            in
            let element_ty =
              if type_equal base_info.element_ty item_ty then
                base_info.element_ty
              else item_ty
            in
            let iterator_info = { base_info with element_ty } in
            if
              stage_requirement_satisfied iterator_info.stage_requirement
                iterator_info.stage_actual
            then Ok iterator_info
            else
              let error =
                {
                  trait_name = constraint_.trait_name;
                  type_args = constraint_.type_args;
                  reason =
                    StageMismatch
                      {
                        required = iterator_info.stage_requirement;
                        actual = Some iterator_info.stage_actual;
                        capability = iterator_info.capability;
                        iterator_kind = Some iterator_info.kind;
                        iterator_source =
                          Some (Types.string_of_ty iterator_info.source_ty);
                        provider = None;
                        manifest_path = None;
                        stage_trace = Profile.stage_trace_empty;
                      };
                  span = constraint_.constraint_span;
                }
              in
              Error error
        | Ok [] ->
            Error
              {
                trait_name = constraint_.trait_name;
                type_args = constraint_.type_args;
                reason = NoImpl;
                span = constraint_.constraint_span;
              }
        | Ok (dict_ref :: _) ->
            (* 複数候補は曖昧性とみなす *)
            Error
              {
                trait_name = constraint_.trait_name;
                type_args = constraint_.type_args;
                reason = AmbiguousImpl [ dict_ref ];
                span = constraint_.constraint_span;
              }
        | Error (err :: _) -> Error err
        | Error [] ->
            Error
              {
                trait_name = constraint_.trait_name;
                type_args = constraint_.type_args;
                reason = NoImpl;
                span = constraint_.constraint_span;
              })
    | _ ->
        Error
          {
            trait_name = constraint_.trait_name;
            type_args = constraint_.type_args;
            reason = NoImpl;
            span = constraint_.constraint_span;
          }

(* ========== デバッグ用 ========== *)

(** トレイト制約の文字列表現 *)
let string_of_trait_constraint (c : trait_constraint) : string =
  Printf.sprintf "%s<%s>" c.trait_name
    (String.concat ", " (List.map string_of_ty c.type_args))

(** 辞書参照の文字列表現 *)
let string_of_dict_ref = function
  | DictImplicit (trait, tys) ->
      let ty_str = String.concat ", " (List.map string_of_ty tys) in
      Printf.sprintf "DictImplicit(%s, [%s])" trait ty_str
  | DictParam idx -> Printf.sprintf "DictParam(%d)" idx
  | DictLocal name -> Printf.sprintf "DictLocal(%s)" name

(** 制約エラー理由の文字列表現
 *
 * Week 20-21 更新: 循環依存のメッセージに循環パスを表示
 *)
let string_of_constraint_error_reason (reason : constraint_error_reason) :
    string =
  match reason with
  | NoImpl -> "NoImpl"
  | AmbiguousImpl dicts ->
      Printf.sprintf "AmbiguousImpl([%s])"
        (String.concat ", " (List.map string_of_dict_ref dicts))
  | CyclicConstraint (cs : trait_constraint list) ->
      (* 循環パスを矢印で表示: Ord<T> -> Eq<T> -> ... *)
      let cycle_path =
        String.concat " -> "
          (List.map
             (fun (c : trait_constraint) ->
               Printf.sprintf "%s<%s>" c.trait_name
                 (String.concat ", " (List.map string_of_ty c.type_args)))
             cs)
      in
      Printf.sprintf "CyclicConstraint: %s" cycle_path
  | StageMismatch
      { required; actual; capability; iterator_kind; iterator_source; _ } ->
      let required_str =
        match required with
        | IteratorStageExact stage -> Printf.sprintf "Exact(%s)" stage
        | IteratorStageAtLeast stage -> Printf.sprintf "AtLeast(%s)" stage
      in
      let capability_str =
        match capability with Some id -> id | None -> "<unspecified>"
      in
      let actual_str =
        match actual with Some value -> value | None -> "<unknown>"
      in
      let kind_str =
        match iterator_kind with
        | Some kind -> string_of_iterator_kind kind
        | None -> "<unspecified>"
      in
      let source_str =
        match iterator_source with Some src -> src | None -> "<unspecified>"
      in
      Printf.sprintf
        "StageMismatch(required=%s, actual=%s, capability=%s, kind=%s, \
         source=%s)"
        required_str actual_str capability_str kind_str source_str
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
           Printf.sprintf "(%s -> %s)"
             (string_of_trait_constraint c1)
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
           Printf.sprintf "(%s => %s)"
             (string_of_trait_constraint c)
             (string_of_dict_ref d))
         state.resolved)
  in
  let pending_str =
    String.concat ", " (List.map string_of_trait_constraint state.pending)
  in
  let errors_str =
    String.concat ", " (List.map string_of_constraint_error state.errors)
  in
  Printf.sprintf "SolverState { resolved: [%s], pending: [%s], errors: [%s] }"
    resolved_str pending_str errors_str
