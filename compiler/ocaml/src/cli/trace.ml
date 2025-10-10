(* Trace — コンパイルフェーズのトレース機能
 *
 * Phase 1-6 Week 15 の開発者体験整備タスクにおいて、
 * コンパイルフェーズの実行時間とメモリ使用量を追跡する機能を提供する。
 *
 * 使用方法:
 *   if opts.trace then Trace.start_phase Parsing;
 *   let ast = parse_source lexbuf in
 *   if opts.trace then Trace.end_phase Parsing;
 *
 * 出力例:
 *   [TRACE] Parsing started
 *   [TRACE] Parsing completed (0.012s, 512 bytes allocated)
 *   [TRACE] Total: 0.060s
 *)

(** コンパイルフェーズの定義 *)
type phase =
  | Parsing       (** 字句解析・構文解析 *)
  | TypeChecking  (** 型推論 *)
  | CoreIR        (** Core IR 生成・糖衣削除 *)
  | Optimization  (** Core IR 最適化 *)
  | CodeGen       (** LLVM IR 生成 *)

(** フェーズ名を文字列に変換 *)
let string_of_phase = function
  | Parsing -> "Parsing"
  | TypeChecking -> "TypeChecking"
  | CoreIR -> "CoreIR"
  | Optimization -> "Optimization"
  | CodeGen -> "CodeGen"

(** トレース情報の記録 *)
type trace_entry = {
  phase: phase;
  start_time: float;
  mutable end_time: float option;
  gc_stat_start: Gc.stat;
  mutable gc_stat_end: Gc.stat option;
}

(** グローバルトレース状態 *)
let trace_stack : trace_entry list ref = ref []
let trace_history : trace_entry list ref = ref []

(** トレース機能が有効かどうか *)
let is_enabled () =
  !trace_stack <> [] || !trace_history <> []

(** トレース記録をリセット（テスト用） *)
let reset () =
  trace_stack := [];
  trace_history := []

(** フェーズ開始を記録
 *
 * @param phase コンパイルフェーズ
 *)
let start_phase phase =
  let entry = {
    phase;
    start_time = Unix.gettimeofday ();
    end_time = None;
    gc_stat_start = Gc.stat ();
    gc_stat_end = None;
  } in
  trace_stack := entry :: !trace_stack;
  Printf.eprintf "[TRACE] %s started\n%!" (string_of_phase phase)

(** メモリ使用量を計算（バイト単位）
 *
 * @param start 開始時のGC統計
 * @param end_ 終了時のGC統計
 * @return アロケーション量（バイト）
 *)
let compute_allocated start end_ =
  let words_start = start.Gc.minor_words +. start.Gc.major_words -. start.Gc.promoted_words in
  let words_end = end_.Gc.minor_words +. end_.Gc.major_words -. end_.Gc.promoted_words in
  let words_allocated = words_end -. words_start in
  (* 1 word = 8 bytes on 64-bit platforms *)
  int_of_float (words_allocated *. 8.0)

(** フェーズ終了を記録
 *
 * @param phase コンパイルフェーズ
 *)
let end_phase phase =
  match !trace_stack with
  | [] ->
      Printf.eprintf "[TRACE] Warning: end_phase called without matching start_phase\n%!"
  | entry :: rest ->
      if entry.phase <> phase then begin
        Printf.eprintf "[TRACE] Warning: phase mismatch (expected %s, got %s)\n%!"
          (string_of_phase entry.phase) (string_of_phase phase)
      end;
      let end_time = Unix.gettimeofday () in
      let gc_stat_end = Gc.stat () in
      entry.end_time <- Some end_time;
      entry.gc_stat_end <- Some gc_stat_end;
      trace_stack := rest;
      trace_history := entry :: !trace_history;

      let elapsed = end_time -. entry.start_time in
      let allocated = compute_allocated entry.gc_stat_start gc_stat_end in
      Printf.eprintf "[TRACE] %s completed (%.3fs, %d bytes allocated)\n%!"
        (string_of_phase phase) elapsed allocated

(** トレースサマリーを出力
 *
 * 全フェーズの実行時間と合計を表示する。
 *)
let print_summary () =
  if !trace_history = [] then begin
    Printf.eprintf "[TRACE] No trace data available\n%!";
    ()
  end else begin
    Printf.eprintf "[TRACE] ===== Trace Summary =====\n%!";

    (* 逆順（実行順）に表示 *)
    let entries = List.rev !trace_history in
    let total_time = ref 0.0 in
    let total_allocated = ref 0 in

    List.iter (fun entry ->
      match entry.end_time, entry.gc_stat_end with
      | Some end_time, Some gc_stat_end ->
          let elapsed = end_time -. entry.start_time in
          let allocated = compute_allocated entry.gc_stat_start gc_stat_end in
          total_time := !total_time +. elapsed;
          total_allocated := !total_allocated + allocated;
          Printf.eprintf "[TRACE]   %s: %.3fs (%d bytes)\n%!"
            (string_of_phase entry.phase) elapsed allocated
      | _ ->
          (* 未完了のエントリ（通常は発生しない） *)
          Printf.eprintf "[TRACE]   %s: incomplete\n%!"
            (string_of_phase entry.phase)
    ) entries;

    Printf.eprintf "[TRACE] Total: %.3fs (%d bytes allocated)\n%!"
      !total_time !total_allocated;
    Printf.eprintf "[TRACE] =======================\n%!"
  end

(** トレース履歴を取得（テスト用）
 *
 * @return トレース履歴のリスト
 *)
let get_history () = !trace_history

(** 特定フェーズの実行時間を取得（テスト用）
 *
 * @param phase コンパイルフェーズ
 * @return 実行時間（秒）、見つからない場合は None
 *)
let get_phase_time phase =
  let entry = List.find_opt (fun e -> e.phase = phase) !trace_history in
  match entry with
  | Some { end_time = Some end_time; start_time; _ } ->
      Some (end_time -. start_time)
  | _ -> None
