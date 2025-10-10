(* Stats — コンパイル統計情報収集
 *
 * Phase 1-6 Week 15-16 の開発者体験整備タスクにおいて、
 * コンパイル過程で生成されたデータ量とフェーズ時間・メモリ統計を記録し、
 * パフォーマンス分析およびメトリクス出力に活用する。
 *
 * 使用方法:
 *   (* Lexer *)
 *   Stats.incr_token_count ();
 *
 *   (* 最後に統計を出力 *)
 *   if opts.stats then Stats.print_stats ();
 *
 * 出力例:
 *   [STATS] ===== Compilation Statistics =====
 *   [STATS] Tokens parsed: 245
 *   [STATS] AST nodes: 87
 *   [STATS] Unify calls: 152
 *   [STATS] Optimization passes: 3
 *   [STATS] LLVM instructions: 421
 *   [STATS] Phase timings:
 *   [STATS]   Parsing: 0.012s (20.0%, 512 bytes)
 *   [STATS]   TypeChecking: 0.048s (80.0%, 1024 bytes)
 *   [STATS] Total time: 0.060s
 *   [STATS] Total allocated: 1536 bytes
 *   [STATS] Peak memory: 65536 bytes
 *   [STATS] memory_peak_ratio: 6.4000
 *   [STATS] ====================================
 *)

(** フェーズタイミング情報（Trace.phase_metrics のエイリアス） *)
type phase_timing = Trace.phase_metrics

(** 統計情報カウンタ *)
type stats = {
  mutable token_count: int;          (** パースしたトークン数 *)
  mutable ast_node_count: int;       (** 生成したASTノード数 *)
  mutable unify_calls: int;          (** 型推論のunify呼び出し回数 *)
  mutable optimization_passes: int;  (** 最適化パスの適用回数 *)
  mutable llvm_instructions: int;    (** 生成したLLVM IR命令数 *)
  mutable phase_timings: phase_timing list;  (** フェーズ別統計（`--trace`/summary 共有） *)
  mutable total_elapsed_seconds: float;      (** フェーズ合計時間（秒） *)
  mutable total_allocated_bytes: int;        (** フェーズ合計アロケーション量（バイト） *)
  mutable peak_memory_bytes: int option;     (** 計測期間中のピークメモリ（バイト） *)
  mutable memory_peak_ratio: float option;   (** `peak_memory_bytes / input_size_bytes` *)
  mutable input_size_bytes: int option;      (** 処理した入力サイズ（バイト） *)
}

(** グローバル統計カウンタ *)
let global_stats : stats = {
  token_count = 0;
  ast_node_count = 0;
  unify_calls = 0;
  optimization_passes = 0;
  llvm_instructions = 0;
  phase_timings = [];
  total_elapsed_seconds = 0.0;
  total_allocated_bytes = 0;
  peak_memory_bytes = None;
  memory_peak_ratio = None;
  input_size_bytes = None;
}

(** memory_peak_ratio 再計算 *)
let recompute_memory_peak_ratio () =
  match global_stats.peak_memory_bytes, global_stats.input_size_bytes with
  | Some peak, Some size when size > 0 ->
      global_stats.memory_peak_ratio <- Some (float_of_int peak /. float_of_int size)
  | _ ->
      global_stats.memory_peak_ratio <- None

(** 統計カウンタをリセット（テスト用）
 *
 * すべてのカウンタを0にリセットする。
 *)
let reset () =
  global_stats.token_count <- 0;
  global_stats.ast_node_count <- 0;
  global_stats.unify_calls <- 0;
  global_stats.optimization_passes <- 0;
  global_stats.llvm_instructions <- 0;
  global_stats.phase_timings <- [];
  global_stats.total_elapsed_seconds <- 0.0;
  global_stats.total_allocated_bytes <- 0;
  global_stats.peak_memory_bytes <- None;
  global_stats.memory_peak_ratio <- None;
  global_stats.input_size_bytes <- None

(** トークン数をインクリメント
 *
 * Lexer がトークンを生成するたびに呼び出す。
 *)
let incr_token_count () =
  global_stats.token_count <- global_stats.token_count + 1

(** ASTノード数をインクリメント
 *
 * Parser がASTノードを生成するたびに呼び出す。
 *)
let incr_ast_node_count () =
  global_stats.ast_node_count <- global_stats.ast_node_count + 1

(** unify呼び出し回数をインクリメント
 *
 * Type_inference.unify が呼び出されるたびに呼び出す。
 *)
let incr_unify_calls () =
  global_stats.unify_calls <- global_stats.unify_calls + 1

(** 最適化パス適用回数をインクリメント
 *
 * Core_ir.Pipeline が最適化パスを適用するたびに呼び出す。
 *)
let incr_optimization_passes () =
  global_stats.optimization_passes <- global_stats.optimization_passes + 1

(** LLVM IR命令数をインクリメント
 *
 * Llvm_gen.Codegen がLLVM IR命令を生成するたびに呼び出す。
 *)
let incr_llvm_instructions () =
  global_stats.llvm_instructions <- global_stats.llvm_instructions + 1

(** 入力サイズを記録（`memory_peak_ratio` 計算用） *)
let set_input_size_bytes size =
  global_stats.input_size_bytes <- Some size;
  recompute_memory_peak_ratio ()

(** フェーズサマリー情報で統計を更新
 *
 * @param summary `Cli.Trace.summary` の結果
 *)
let update_trace_summary (summary : Trace.summary) =
  global_stats.phase_timings <- summary.phases;
  global_stats.total_elapsed_seconds <- summary.total_elapsed_seconds;
  global_stats.total_allocated_bytes <- summary.total_allocated_bytes;
  global_stats.peak_memory_bytes <- Some summary.peak_memory_bytes;
  recompute_memory_peak_ratio ()

(** 統計情報を取得（テスト用）
 *
 * @return 現在の統計情報
 *)
let get_stats () = global_stats

(** 統計情報を出力
 *
 * 収集した統計情報を標準エラー出力に表示する。
 *)
let print_stats () =
  Printf.eprintf "[STATS] ===== Compilation Statistics =====\n%!";
  Printf.eprintf "[STATS] Tokens parsed: %d\n%!" global_stats.token_count;
  Printf.eprintf "[STATS] AST nodes: %d\n%!" global_stats.ast_node_count;
  Printf.eprintf "[STATS] Unify calls: %d\n%!" global_stats.unify_calls;
  Printf.eprintf "[STATS] Optimization passes: %d\n%!" global_stats.optimization_passes;
  Printf.eprintf "[STATS] LLVM instructions: %d\n%!" global_stats.llvm_instructions;
  if global_stats.phase_timings <> [] then begin
    Printf.eprintf "[STATS] Phase timings:\n%!";
    List.iter (fun Trace.{ phase; elapsed_seconds; time_ratio; allocated_bytes } ->
      Printf.eprintf "[STATS]   %s: %.3fs (%.1f%%, %d bytes)\n%!"
        (Trace.string_of_phase phase)
        elapsed_seconds
        (time_ratio *. 100.0)
        allocated_bytes
    ) global_stats.phase_timings;
    Printf.eprintf "[STATS] Total time: %.3fs\n%!" global_stats.total_elapsed_seconds;
    Printf.eprintf "[STATS] Total allocated: %d bytes\n%!" global_stats.total_allocated_bytes;

    (* フェーズ別ランキング出力（時間降順） *)
    Printf.eprintf "[STATS] Phase timings (ranked by time):\n%!";
    let ranked_phases =
      List.sort (fun Trace.{ elapsed_seconds = t1; _ } Trace.{ elapsed_seconds = t2; _ } ->
        compare t2 t1  (* 降順: 大きい方が先 *)
      ) global_stats.phase_timings
    in
    List.iteri (fun i Trace.{ phase; elapsed_seconds; time_ratio; allocated_bytes } ->
      Printf.eprintf "[STATS]   %d. %s: %.3fs (%.1f%%, %d bytes)\n%!"
        (i + 1)
        (Trace.string_of_phase phase)
        elapsed_seconds
        (time_ratio *. 100.0)
        allocated_bytes
    ) ranked_phases;
  end;
  (match global_stats.peak_memory_bytes with
  | Some peak -> Printf.eprintf "[STATS] Peak memory: %d bytes\n%!" peak
  | None -> ());
  (match global_stats.memory_peak_ratio with
  | Some ratio -> Printf.eprintf "[STATS] memory_peak_ratio: %.4f\n%!" ratio
  | None -> ());
  Printf.eprintf "[STATS] ====================================\n%!"

(** 統計情報をJSON形式で出力
 *
 * @return JSON文字列
 *)
let to_json () =
  let module Y = Yojson.Basic in
  let phase_list =
    `List (List.map (fun Trace.{ phase; elapsed_seconds; time_ratio; allocated_bytes } ->
        `Assoc [
          ("phase", `String (Trace.string_of_phase phase));
          ("elapsed_seconds", `Float elapsed_seconds);
          ("time_ratio", `Float time_ratio);
          ("allocated_bytes", `Int allocated_bytes);
        ]
      ) global_stats.phase_timings)
  in
  let assoc =
    [
      ("tokens_parsed", `Int global_stats.token_count);
      ("ast_nodes", `Int global_stats.ast_node_count);
      ("unify_calls", `Int global_stats.unify_calls);
      ("optimization_passes", `Int global_stats.optimization_passes);
      ("llvm_instructions", `Int global_stats.llvm_instructions);
      ("phase_timings", phase_list);
      ("total_elapsed_seconds", `Float global_stats.total_elapsed_seconds);
      ("total_allocated_bytes", `Int global_stats.total_allocated_bytes);
      ("peak_memory_bytes", match global_stats.peak_memory_bytes with Some v -> `Int v | None -> `Null);
      ("memory_peak_ratio", match global_stats.memory_peak_ratio with Some v -> `Float v | None -> `Null);
      ("input_size_bytes", match global_stats.input_size_bytes with Some v -> `Int v | None -> `Null);
    ]
  in
  Y.pretty_to_string (`Assoc assoc)

(** 統計情報をCSV形式で出力
 *
 * フェーズ別のタイミング情報をCSV形式で返す。
 * ヘッダー行とデータ行を含む。
 *
 * @return CSV文字列
 *)
let to_csv () =
  let buffer = Buffer.create 1024 in
  (* ヘッダー行 *)
  Buffer.add_string buffer "phase,elapsed_seconds,time_ratio,allocated_bytes\n";
  (* データ行 *)
  List.iter (fun Trace.{ phase; elapsed_seconds; time_ratio; allocated_bytes } ->
    Printf.bprintf buffer "%s,%.6f,%.6f,%d\n"
      (Trace.string_of_phase phase)
      elapsed_seconds
      time_ratio
      allocated_bytes
  ) global_stats.phase_timings;
  Buffer.contents buffer
