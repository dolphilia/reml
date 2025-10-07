(* Core_ir.Dce — Dead Code Elimination Pass Interface *)

open Ir

(** DCE 統計情報 *)
type dce_stats = {
  mutable removed_bindings: int;     (** 削除された束縛の数 *)
  mutable removed_blocks: int;       (** 削除されたブロックの数 *)
  mutable removed_stmts: int;        (** 削除された文の数 *)
}

(** 統計情報を作成 *)
val create_stats : unit -> dce_stats

(** 統計情報をリセット *)
val reset_stats : dce_stats -> unit

(** 関数に対して死コード削除を適用
 *
 * @param fn 最適化対象の関数
 * @return (最適化後の関数, 統計情報)
 *)
val optimize_function : function_def -> function_def * dce_stats

(** モジュール全体に対して死コード削除を適用
 *
 * @param m 最適化対象のモジュール
 * @return (最適化後のモジュール, 統計情報)
 *)
val optimize_module : module_def -> module_def * dce_stats
