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

(* ========== 型環境の定義 ========== *)

(** 型環境
 *
 * bindings: 現在のスコープの束縛
 * parent: 親スコープの環境（Noneはトップレベル）
 *)
type env = {
  bindings: (string * type_scheme) list;
  parent: env option;
}

(* ========== 基本操作 ========== *)

(** 空の型環境 *)
let empty = { bindings = []; parent = None }

(** 識別子と型スキームを環境に追加
 *
 * 同名の束縛がある場合は上書き（シャドーイング）
 *)
let extend name scheme env =
  { env with bindings = (name, scheme) :: env.bindings }

(** 識別子の型スキームを検索
 *
 * 現在のスコープで見つからない場合は親スコープを再帰的に探索
 *)
let rec lookup name env =
  match List.assoc_opt name env.bindings with
  | Some scheme -> Some scheme
  | None ->
      match env.parent with
      | Some parent -> lookup name parent
      | None -> None

(** 新しいスコープに入る
 *
 * 現在の環境を親として持つ新しい空の環境を作成
 *)
let enter_scope env =
  { bindings = []; parent = Some env }

(** スコープから出る
 *
 * 親環境を返す（トップレベルの場合はエラー）
 *)
let exit_scope env =
  match env.parent with
  | Some parent -> parent
  | None -> failwith "Cannot exit top-level scope"

(** 環境内のすべての束縛を取得（デバッグ用） *)
let bindings env = env.bindings

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
  let a_some = TypeVarGen.fresh (Some "a") in
  let env = extend "Some" {
    quantified = [a_some];
    body = TArrow (TVar a_some, ty_option (TVar a_some))
  } env in

  let a_none = TypeVarGen.fresh (Some "a") in
  let env = extend "None" {
    quantified = [a_none];
    body = ty_option (TVar a_none)
  } env in

  (* Result<T, E> のコンストラクタ
   * Ok : ∀a, e. a -> Result<a, e>
   * Err : ∀a, e. e -> Result<a, e>
   *)
  let a_ok = TypeVarGen.fresh (Some "a") in
  let e_ok = TypeVarGen.fresh (Some "e") in
  let env = extend "Ok" {
    quantified = [a_ok; e_ok];
    body = TArrow (TVar a_ok, ty_result (TVar a_ok) (TVar e_ok))
  } env in

  let a_err = TypeVarGen.fresh (Some "a") in
  let e_err = TypeVarGen.fresh (Some "e") in
  let env = extend "Err" {
    quantified = [a_err; e_err];
    body = TArrow (TVar e_err, ty_result (TVar a_err) (TVar e_err))
  } env in

  (* Never 型（空集合、到達不能を表現）
   * 仕様書 3-1 §2.1: Never = Result<Never, Never>
   *)
  let env = extend "Never" {
    quantified = [];
    body = ty_never
  } env in

  env

(* ========== ユーティリティ関数 ========== *)

(** 環境に複数の束縛を一度に追加 *)
let extend_many bindings env =
  List.fold_left (fun env (name, scheme) ->
    extend name scheme env
  ) env bindings

(** 環境の文字列表現（デバッグ用） *)
let rec string_of_env env =
  let bindings_str = List.map (fun (name, scheme) ->
    Printf.sprintf "  %s : %s" name (string_of_scheme scheme)
  ) env.bindings in
  let current = String.concat "\n" bindings_str in
  match env.parent with
  | None -> current
  | Some parent ->
      current ^ "\n--- parent scope ---\n" ^ string_of_env parent

(** 環境内の識別子の存在確認 *)
let mem name env =
  Option.is_some (lookup name env)

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
