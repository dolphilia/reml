(* Type_env — Type Environment for Reml Type Inference (Phase 2)
 *
 * このファイルは型環境（Type Environment）の実装を提供する。
 * 型環境は識別子から型スキームへのマッピングを管理し、スコープのネストをサポートする。
 *
 * 設計原則:
 * - 識別子 → 型スキームのマッピング
 * - スコープのネスト（親環境への参照）
 * - 初期環境の提供（組み込み型、Core.Prelude）
 *)

open Types

type mutability = Immutable | Mutable
type binding = { scheme : constrained_scheme; mutability : mutability }

(* ========== 型環境の定義 ========== *)

(** 初期環境専用の型変数生成
 *
 * `TypeVarGen.reset ()` がテストで頻繁に呼ばれるため、初期環境で使用する
 * 型変数は通常のカウンタとは独立させ、ID 衝突を避ける。
 * ここでは負の ID を割り当て、実装側の生成する非負 ID と住み分ける。
 *)
let fresh_builtin_var =
  let counter = ref (-1) in
  fun name ->
    let id = !counter in
    decr counter;
    { tv_id = id; tv_name = Some name }

type env = { bindings : (string * binding) list; parent : env option }
(** 型環境
 *
 * bindings: 現在のスコープの束縛
 * parent: 親スコープの環境（Noneはトップレベル）
 *)

(* ========== 基本操作 ========== *)

(** 空の型環境 *)
let empty = { bindings = []; parent = None }

(** 識別子と型スキームを環境に追加
 *
 * 同名の束縛がある場合は上書き（シャドーイング）
 *)
let extend ?(mutability = Immutable) name scheme env =
  let binding = { scheme; mutability } in
  { env with bindings = (name, binding) :: env.bindings }

(** 識別子の型スキームを検索
 *
 * 現在のスコープで見つからない場合は親スコープを再帰的に探索
 *)
let rec lookup_binding name env =
  match List.assoc_opt name env.bindings with
  | Some binding -> Some binding
  | None -> (
      match env.parent with
      | Some parent -> lookup_binding name parent
      | None -> None)

let lookup name env =
  match lookup_binding name env with
  | Some binding -> Some binding.scheme
  | None -> None

let lookup_mutability name env =
  match lookup_binding name env with
  | Some binding -> Some binding.mutability
  | None -> None

let is_mutable name env =
  match lookup_mutability name env with Some Mutable -> true | _ -> false

(** 新しいスコープに入る
 *
 * 現在の環境を親として持つ新しい空の環境を作成
 *)
let enter_scope env = { bindings = []; parent = Some env }

(** スコープから出る
 *
 * 親環境を返す（トップレベルの場合はエラー）
 *)
let exit_scope env =
  match env.parent with
  | Some parent -> parent
  | None -> failwith "Cannot exit top-level scope"

(** 環境内のすべての束縛を取得（デバッグ用） *)
let bindings env =
  List.map (fun (name, binding) -> (name, binding.scheme)) env.bindings

let bindings_with_mut env = env.bindings

(* ========== 初期環境の構築 ========== *)

(** 組み込み型の初期環境
 *
 * 仕様書 1-2 §A.1/A.2 および 3-1 §2.1 に従う
 *)
let initial_env =
  (* 空環境から開始 *)
  let env = empty in

  (* 組み込み型定数は型システムに組み込まれているため、
   * ここでは Core.Prelude の型とユーザ定義型のみを登録 *)

  (* Option<T> のコンストラクタ
   * Some : ∀a. a -> Option<a>
   * None : ∀a. Option<a>
   *)
  let a_some = fresh_builtin_var "Option.a" in
  let env =
    extend "Some"
      (scheme_to_constrained
         {
           quantified = [ a_some ];
           body = TArrow (TVar a_some, ty_option (TVar a_some));
         })
      env
  in

  let a_none = fresh_builtin_var "Option.a" in
  let env =
    extend "None"
      (scheme_to_constrained
         { quantified = [ a_none ]; body = ty_option (TVar a_none) })
      env
  in

  (* Result<T, E> のコンストラクタ
   * Ok : ∀a, e. a -> Result<a, e>
   * Err : ∀a, e. e -> Result<a, e>
   *)
  let a_ok = fresh_builtin_var "Result.a" in
  let e_ok = fresh_builtin_var "Result.e" in
  let env =
    extend "Ok"
      (scheme_to_constrained
         {
           quantified = [ a_ok; e_ok ];
           body = TArrow (TVar a_ok, ty_result (TVar a_ok) (TVar e_ok));
         })
      env
  in

  let a_err = fresh_builtin_var "Result.a" in
  let e_err = fresh_builtin_var "Result.e" in
  let env =
    extend "Err"
      (scheme_to_constrained
         {
           quantified = [ a_err; e_err ];
           body = TArrow (TVar e_err, ty_result (TVar a_err) (TVar e_err));
         })
      env
  in

  (* Never 型（空集合、到達不能を表現）
   * 仕様書 3-1 §2.1: Never = Result<Never, Never>
   *)
  let env =
    extend "Never"
      (scheme_to_constrained { quantified = []; body = ty_never })
      env
  in

  env

(* ========== ユーティリティ関数 ========== *)

(** 環境に複数の束縛を一度に追加 *)
let extend_many bindings env =
  List.fold_left (fun env (name, scheme) -> extend name scheme env) env bindings

(** 環境の文字列表現（デバッグ用） *)
let rec string_of_env env =
  let bindings_str =
    List.map
      (fun (name, binding) ->
        let mut_label =
          match binding.mutability with
          | Mutable -> " (mutable)"
          | Immutable -> ""
        in
        Printf.sprintf "  %s : %s%s" name
          (string_of_constrained_scheme binding.scheme)
          mut_label)
      env.bindings
  in
  let current = String.concat "\n" bindings_str in
  match env.parent with
  | None -> current
  | Some parent -> current ^ "\n--- parent scope ---\n" ^ string_of_env parent

(** 環境内の識別子の存在確認 *)
let mem name env = Option.is_some (lookup name env)

(** 環境から識別子を取得（見つからない場合は例外） *)
let find name env =
  match lookup name env with
  | Some scheme -> scheme
  | None -> failwith (Printf.sprintf "Unbound variable: %s" name)

(* ========== トレイト制約用の拡張（Phase 2 後半で実装） ========== *)

(** トレイト制約を管理する環境拡張
 *
 * 現在は基本型のみのため、トレイト制約は Phase 2 後半で実装
 * 仕様書 1-2 §B でトレイト（型クラス風）の実装を扱う
 *)

(* TODO Phase 2 後半:
 * - トレイト定義の登録
 * - impl の登録と検索
 * - 制約解決のサポート
 *)

(* ========== モノモルフィゼーション PoC 用レジストリ ========== *)

(** 型クラス制約の解決結果を記録する PoC レジストリ *)
module Monomorph_registry = struct
  type trait_instance = {
    trait_name : string;
    type_args : ty list;
    methods : (string * string) list;
  }

  let registry : trait_instance list ref = ref []
  let reset () = registry := []

  let equal_instance lhs rhs =
    lhs.trait_name = rhs.trait_name
    && List.length lhs.type_args = List.length rhs.type_args
    && List.for_all2 type_equal lhs.type_args rhs.type_args

  let normalize_methods methods =
    List.fold_left
      (fun acc (name, impl) ->
        if List.exists (fun (existing, _) -> String.equal existing name) acc
        then acc
        else (name, impl) :: acc)
      [] methods

  let merge_methods lhs rhs =
    let merged = List.fold_left (fun acc item -> item :: acc) lhs rhs in
    normalize_methods merged

  let record (instance : trait_instance) =
    let instance =
      { instance with methods = normalize_methods instance.methods }
    in
    match
      List.find_opt (fun existing -> equal_instance existing instance) !registry
    with
    | Some existing ->
        let merged =
          {
            existing with
            methods = merge_methods existing.methods instance.methods;
          }
        in
        registry :=
          merged
          :: List.filter
               (fun candidate -> not (equal_instance candidate instance))
               !registry
    | None -> registry := instance :: !registry

  let all () = !registry

  let sanitize_type_name name =
    let buffer = Buffer.create (String.length name) in
    String.iter
      (fun ch ->
        match ch with
        | 'a' .. 'z' | 'A' .. 'Z' | '0' .. '9' -> Buffer.add_char buffer ch
        | _ -> Buffer.add_char buffer '_')
      name;
    Buffer.contents buffer

  let string_of_type_for_symbol ty = sanitize_type_name (string_of_ty ty)

  let builtin_methods trait ty =
    let ty_symbol = string_of_type_for_symbol ty in
    match trait with
    | "Eq" -> (
        match ty with
        | TCon (TCInt _) | TCon TCBool | TCon TCString ->
            [ ("eq", Printf.sprintf "__Eq_%s_eq" ty_symbol) ]
        | _ -> [])
    | "Ord" -> (
        match ty with
        | TCon (TCInt _) | TCon TCString ->
            [ ("cmp", Printf.sprintf "__Ord_%s_compare" ty_symbol) ]
        | _ -> [])
    | _ -> []
end
