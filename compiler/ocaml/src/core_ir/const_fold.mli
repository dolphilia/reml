(* Core_ir.Const_fold — Constant Folding Pass Interface (Phase 3)
 *
 * このファイルは定数畳み込み最適化パスの公開インターフェースを定義する。
 *)

open Ir

(** 定数畳み込み時のエラー *)
type fold_error =
  | DivisionByZero of Ast.span         (** ゼロ除算 *)
  | IntegerOverflow of Ast.span        (** 整数オーバーフロー *)
  | TypeMismatch of Types.ty * Types.ty * Ast.span  (** 型不一致 *)
  | InvalidOperation of string * Ast.span  (** 無効な演算 *)

exception FoldError of fold_error

(** 畳み込み統計 *)
type fold_stats = {
  mutable folded_exprs: int;           (** 畳み込まれた式の数 *)
  mutable eliminated_branches: int;    (** 削除された分岐の数 *)
  mutable propagated_constants: int;   (** 伝播された定数の数 *)
}

(** 統計情報の初期化 *)
val create_stats : unit -> fold_stats

(** 統計情報のリセット *)
val reset_stats : fold_stats -> unit

(** 不動点反復の設定 *)
type fold_config = {
  max_iterations: int;     (** 最大反復回数 *)
  verbose: bool;           (** 詳細ログ出力 *)
}

(** デフォルト設定 *)
val default_config : fold_config

(** 関数に対して定数畳み込みを適用
 *
 * @param config 最適化設定（オプション）
 * @param fn 対象関数
 * @return (最適化後の関数, 統計情報)
 *)
val optimize_function : ?config:fold_config -> function_def -> function_def * fold_stats

(** モジュール全体に対して定数畳み込みを適用
 *
 * @param config 最適化設定（オプション）
 * @param m 対象モジュール
 * @return (最適化後のモジュール, 統計情報)
 *)
val optimize_module : ?config:fold_config -> module_def -> module_def * fold_stats
