(* Core_ir.Pipeline — Optimization Pipeline for Core IR (Phase 3)
 *
 * このファイルは Core IR の最適化パイプラインを提供する。
 * 計画書: docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md §6
 *
 * 主要機能:
 * 1. パス実行順序の管理（Desugar → CFG → ConstFold → DCE）
 * 2. 不動点反復（畳み込み→DCE→畳み込み...）
 * 3. 最適化レベルの設定（-O0, -O1）
 * 4. 統計収集とレポート
 *
 * 設計原則:
 * - 各パスの独立性を保つ
 * - 停止条件の明確化
 * - デバッグモードでの中間結果出力
 * - 性能測定と記録
 *)

open Ir
open Const_fold
open Dce

(* ========== 最適化レベル ========== *)

(** 最適化レベル *)
type opt_level = O0  (** 最適化なし *) | O1  (** 基本最適化（定数畳み込み + DCE） *)

(* ========== パイプライン設定 ========== *)

type pipeline_config = {
  opt_level : opt_level;  (** 最適化レベル *)
  enable_const_fold : bool;  (** 定数畳み込みを有効化 *)
  enable_dce : bool;  (** 死コード削除を有効化 *)
  max_iterations : int;  (** 不動点反復の最大回数 *)
  verbose : bool;  (** 詳細ログ出力 *)
  emit_intermediate : bool;  (** 中間結果を出力 *)
}
(** パイプライン設定 *)

(** デフォルト設定（O0: 最適化なし） *)
let config_o0 : pipeline_config =
  {
    opt_level = O0;
    enable_const_fold = false;
    enable_dce = false;
    max_iterations = 0;
    verbose = false;
    emit_intermediate = false;
  }

(** O1 設定（基本最適化） *)
let config_o1 : pipeline_config =
  {
    opt_level = O1;
    enable_const_fold = true;
    enable_dce = true;
    max_iterations = 5;
    verbose = false;
    emit_intermediate = false;
  }

(** 設定から最適化レベルを選択 *)
let config_from_level (level : opt_level) : pipeline_config =
  match level with O0 -> config_o0 | O1 -> config_o1

(* ========== 統計情報 ========== *)

type pipeline_stats = {
  mutable iterations : int;  (** 実行した反復回数 *)
  mutable total_const_fold_time : float;  (** 定数畳み込み総時間 (秒) *)
  mutable total_dce_time : float;  (** DCE 総時間 (秒) *)
  mutable total_folded_exprs : int;  (** 畳み込まれた式の総数 *)
  mutable total_removed_bindings : int;  (** 削除された束縛の総数 *)
  mutable total_removed_blocks : int;  (** 削除されたブロックの総数 *)
}
(** パイプライン統計 *)

let create_pipeline_stats () : pipeline_stats =
  {
    iterations = 0;
    total_const_fold_time = 0.0;
    total_dce_time = 0.0;
    total_folded_exprs = 0;
    total_removed_bindings = 0;
    total_removed_blocks = 0;
  }

(** 統計情報を表示 *)
let print_stats (stats : pipeline_stats) : unit =
  Printf.printf "=== Optimization Pipeline Statistics ===\n";
  Printf.printf "Iterations:            %d\n" stats.iterations;
  Printf.printf "Const Fold Time:       %.3f sec\n" stats.total_const_fold_time;
  Printf.printf "DCE Time:              %.3f sec\n" stats.total_dce_time;
  Printf.printf "Folded Expressions:    %d\n" stats.total_folded_exprs;
  Printf.printf "Removed Bindings:      %d\n" stats.total_removed_bindings;
  Printf.printf "Removed Blocks:        %d\n" stats.total_removed_blocks;
  Printf.printf "========================================\n%!"

(* ========== パイプライン実行 ========== *)

(** 関数が変化したかを判定 *)
let function_changed (f1 : function_def) (f2 : function_def) : bool =
  (* 簡易実装: ブロック数が変わったか、または物理的に異なるか *)
  List.length f1.fn_blocks <> List.length f2.fn_blocks || f1 != f2

(** 単一パスの実行（定数畳み込み + DCE） *)
let run_single_pass (config : pipeline_config) (stats : pipeline_stats)
    (fn : function_def) : function_def =
  let fn_after_const_fold =
    if config.enable_const_fold then (
      let start_time = Unix.gettimeofday () in
      let fold_config =
        { Const_fold.max_iterations = 3; verbose = config.verbose }
      in
      let optimized, fold_stats =
        Const_fold.optimize_function ~config:fold_config fn
      in
      let end_time = Unix.gettimeofday () in
      stats.total_const_fold_time <-
        stats.total_const_fold_time +. (end_time -. start_time);
      stats.total_folded_exprs <-
        stats.total_folded_exprs + fold_stats.folded_exprs;
      if config.verbose then
        Printf.eprintf "[Pipeline] Const fold: %d exprs folded\n%!"
          fold_stats.folded_exprs;
      optimized)
    else fn
  in

  let fn_after_dce =
    if config.enable_dce then (
      let start_time = Unix.gettimeofday () in
      let optimized, dce_stats = Dce.optimize_function fn_after_const_fold in
      let end_time = Unix.gettimeofday () in
      stats.total_dce_time <- stats.total_dce_time +. (end_time -. start_time);
      stats.total_removed_bindings <-
        stats.total_removed_bindings + dce_stats.removed_bindings;
      stats.total_removed_blocks <-
        stats.total_removed_blocks + dce_stats.removed_blocks;
      if config.verbose then
        Printf.eprintf "[Pipeline] DCE: %d bindings, %d blocks removed\n%!"
          dce_stats.removed_bindings dce_stats.removed_blocks;
      optimized)
    else fn_after_const_fold
  in

  fn_after_dce

(** 不動点に達するまでパスを反復 *)
let run_to_fixpoint (config : pipeline_config) (stats : pipeline_stats)
    (fn : function_def) : function_def =
  let rec loop iteration fn_prev =
    if iteration >= config.max_iterations then (
      if config.verbose then
        Printf.eprintf "[Pipeline] Reached maximum iterations (%d)\n%!"
          config.max_iterations;
      fn_prev)
    else (
      stats.iterations <- stats.iterations + 1;
      let fn_next = run_single_pass config stats fn_prev in
      if function_changed fn_prev fn_next then (
        if config.verbose then
          Printf.eprintf "[Pipeline] Iteration %d: changes detected\n%!"
            iteration;
        loop (iteration + 1) fn_next)
      else (
        if config.verbose then
          Printf.eprintf "[Pipeline] Converged at iteration %d\n%!" iteration;
        fn_next))
  in
  loop 0 fn

(** 関数に対してパイプラインを実行 *)
let optimize_function ?(config = config_o1) (fn : function_def) :
    function_def * pipeline_stats =
  let stats = create_pipeline_stats () in
  let optimized =
    if config.opt_level = O0 then fn else run_to_fixpoint config stats fn
  in
  (optimized, stats)

(** モジュール全体に対してパイプラインを実行 *)
let optimize_module ?(config = config_o1) (m : module_def) :
    module_def * pipeline_stats =
  let stats = create_pipeline_stats () in
  let optimized_fns =
    List.map
      (fun fn ->
        let optimized, fn_stats = optimize_function ~config fn in
        stats.iterations <- stats.iterations + fn_stats.iterations;
        stats.total_const_fold_time <-
          stats.total_const_fold_time +. fn_stats.total_const_fold_time;
        stats.total_dce_time <- stats.total_dce_time +. fn_stats.total_dce_time;
        stats.total_folded_exprs <-
          stats.total_folded_exprs + fn_stats.total_folded_exprs;
        stats.total_removed_bindings <-
          stats.total_removed_bindings + fn_stats.total_removed_bindings;
        stats.total_removed_blocks <-
          stats.total_removed_blocks + fn_stats.total_removed_blocks;
        optimized)
      m.function_defs
  in
  ({ m with function_defs = optimized_fns }, stats)
