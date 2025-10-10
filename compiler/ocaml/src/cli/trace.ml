(* Trace — コンパイルフェーズのトレース機能
 *
 * Phase 1-6 Week 15 の開発者体験整備タスクにおいて、
 * コンパイルフェーズの実行時間とメモリ使用量を追跡する機能を提供する。
 *
 * 使用方法:
 *   Trace.start_phase ~emit_log:opts.trace Parsing;
 *   let ast = parse_source lexbuf in
 *   Trace.end_phase ~emit_log:opts.trace Parsing;
 *
 * 出力例:
 *   [TRACE] Parsing started
 *   [TRACE] Parsing completed (0.012s, 512 bytes allocated)
 *   [TRACE] Total: 0.060s (2304 bytes allocated)
 *   [TRACE] Peak memory: 65536 bytes
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

(** フェーズ毎の統計情報 *)
type phase_metrics = {
  phase: phase;
  elapsed_seconds: float;
  allocated_bytes: int;
  time_ratio: float;
}

(** トレースサマリー構造体 *)
type summary = {
  phases: phase_metrics list;
  total_elapsed_seconds: float;
  total_allocated_bytes: int;
  peak_memory_bytes: int;
}

(** トレース情報の記録 *)
type trace_entry = {
  phase: phase;
  start_time: float;
  mutable end_time: float option;
  gc_stat_start: Gc.stat;
  mutable gc_stat_end: Gc.stat option;
  mutable allocated_bytes: int option;
}

(** グローバルトレース状態 *)
let trace_stack : trace_entry list ref = ref []
let trace_history : trace_entry list ref = ref []
let peak_memory_bytes : int ref = ref 0

let word_size_bytes = Sys.word_size / 8

let update_peak_memory gc_stat =
  let bytes =
    match word_size_bytes with
    | 0 -> 0
    | word_bytes ->
        let top_heap_words =
          try gc_stat.Gc.top_heap_words with
          | _ -> 0
        in
        top_heap_words * word_bytes
  in
  if bytes > !peak_memory_bytes then peak_memory_bytes := bytes

(** トレース機能が有効かどうか *)
let is_enabled () =
  !trace_stack <> [] || !trace_history <> []

(** トレース記録をリセット（テスト用） *)
let reset () =
  trace_stack := [];
  trace_history := [];
  peak_memory_bytes := 0

(** フェーズ開始を記録
 *
 * @param phase コンパイルフェーズ
 * @param emit_log ログ出力を行うかどうか
 *)
let start_phase ?(emit_log = true) phase =
  let entry = {
    phase;
    start_time = Unix.gettimeofday ();
    end_time = None;
    gc_stat_start = Gc.stat ();
    gc_stat_end = None;
    allocated_bytes = None;
  } in
  trace_stack := entry :: !trace_stack;
  if emit_log then
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
  let word_bytes =
    if word_size_bytes = 0 then 8 else word_size_bytes
  in
  int_of_float (words_allocated *. float_of_int word_bytes)

(** フェーズ終了を記録
 *
 * @param phase コンパイルフェーズ
 * @param emit_log ログ出力を行うかどうか
 *)
let end_phase ?(emit_log = true) phase =
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
      entry.allocated_bytes <- Some (compute_allocated entry.gc_stat_start gc_stat_end);
      trace_stack := rest;
      trace_history := entry :: !trace_history;

      update_peak_memory gc_stat_end;

      if emit_log then begin
        let elapsed = end_time -. entry.start_time in
        let allocated = match entry.allocated_bytes with Some bytes -> bytes | None -> 0 in
        Printf.eprintf "[TRACE] %s completed (%.3fs, %d bytes allocated)\n%!"
          (string_of_phase phase) elapsed allocated
      end

(** トレースサマリーを作成 *)
let summary () : summary =
  let entries =
    List.rev !trace_history
    |> List.filter_map (fun entry ->
         match entry.end_time, entry.gc_stat_end with
         | Some end_time, Some gc_stat_end ->
             let elapsed = end_time -. entry.start_time in
             let allocated =
               match entry.allocated_bytes with
               | Some bytes -> bytes
               | None -> compute_allocated entry.gc_stat_start gc_stat_end
             in
             Some (entry.phase, elapsed, allocated)
         | _ -> None)
  in
  let total_elapsed =
    List.fold_left (fun acc (_, elapsed, _) -> acc +. elapsed) 0.0 entries
  in
  let total_allocated =
    List.fold_left (fun acc (_, _, allocated) -> acc + allocated) 0 entries
  in
  let phases =
    List.map (fun (phase, elapsed, allocated) ->
      let ratio =
        if total_elapsed > 0.0 then elapsed /. total_elapsed else 0.0
      in
      { phase; elapsed_seconds = elapsed; allocated_bytes = allocated; time_ratio = ratio }
    ) entries
  in
  {
    phases;
    total_elapsed_seconds = total_elapsed;
    total_allocated_bytes = total_allocated;
    peak_memory_bytes = !peak_memory_bytes;
  }

(** トレースサマリーを出力
 *
 * 全フェーズの実行時間と合計を表示する。
 *)
let print_summary ?summary_data () =
  let summary = match summary_data with
    | Some s -> s
    | None -> summary ()
  in
  if summary.phases = [] then begin
    Printf.eprintf "[TRACE] No trace data available\n%!";
    ()
  end else begin
    Printf.eprintf "[TRACE] ===== Trace Summary =====\n%!";

    List.iter (fun { phase; elapsed_seconds; time_ratio; allocated_bytes } ->
      Printf.eprintf "[TRACE]   %s: %.3fs (%.1f%%, %d bytes)\n%!"
        (string_of_phase phase)
        elapsed_seconds
        (time_ratio *. 100.0)
        allocated_bytes
    ) summary.phases;

    Printf.eprintf "[TRACE] Total: %.3fs (%d bytes allocated)\n%!"
      summary.total_elapsed_seconds summary.total_allocated_bytes;
    if summary.peak_memory_bytes > 0 then
      Printf.eprintf "[TRACE] Peak memory: %d bytes\n%!" summary.peak_memory_bytes;
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
