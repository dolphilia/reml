(* Main — Reml コンパイラエントリーポイント (Phase 1-6)
 *
 * コマンドライン引数を解析し、パーサー、型推論、LLVM IR生成を実行する。
 * Phase 1 M1: --emit-ast オプション
 * Phase 2 M2: --emit-tast オプション
 * Phase 3 M3: --emit-ir, --verify-ir オプション
 * Phase 1-6: CLI オプション管理を Cli.Options モジュールに移行
 * Phase 1-6: 診断出力を Diagnostic_formatter / Json_formatter に移行
 *)

(** カラーモードを解決する *)
let resolve_color_mode opts =
  Cli.Color.resolve_color_mode ~requested:opts.Cli.Options.color
    ~is_tty:(Cli.Color.is_tty Unix.stderr)

(** 診断を出力する
 *
 * @param opts コマンドラインオプション
 * @param source ソースコード文字列（オプション）
 * @param diag 診断情報
 *)
let print_diagnostic opts source diag =
  let color_mode = resolve_color_mode opts in
  let output =
    match opts.Cli.Options.format with
    | Cli.Options.Text ->
        Cli.Diagnostic_formatter.format_diagnostic ~source ~diag ~color_mode
    | Cli.Options.Json -> Cli.Json_formatter.diagnostic_to_json diag
  in
  Printf.eprintf "%s\n" output

(** 出力ファイル名生成 *)
let output_filename out_dir basename suffix =
  Filename.concat out_dir (basename ^ suffix)

(** ベース名取得（拡張子除去） *)
let get_basename filepath =
  Filename.remove_extension (Filename.basename filepath)

(** ランタイムライブラリとリンクして実行可能ファイルを生成
 *
 * @param ll_file LLVM IR ファイルパス
 * @param runtime_lib ランタイムライブラリパス
 * @param output_exe 出力実行可能ファイルパス
 *)
let link_with_runtime ll_file runtime_lib output_exe =
  (* LLVM IR → オブジェクトファイル *)
  let obj_file = Filename.remove_extension ll_file ^ ".o" in
  let llc_cmd = Printf.sprintf "llc -filetype=obj %s -o %s" ll_file obj_file in

  Printf.printf "Compiling to object file: %s\n" obj_file;
  let llc_result = Sys.command llc_cmd in
  if llc_result <> 0 then (
    Printf.eprintf "Error: llc failed with exit code %d\n" llc_result;
    exit 1);

  (* オブジェクトファイル + ランタイム → 実行可能ファイル *)
  (* clang に標準ライブラリリンクを任せる（デフォルト動作） *)
  let link_cmd =
    Printf.sprintf "cc %s %s -o %s" obj_file runtime_lib output_exe
  in

  Printf.printf "Linking with runtime: %s\n" output_exe;
  let link_result = Sys.command link_cmd in
  if link_result <> 0 then (
    Printf.eprintf "Error: linking failed with exit code %d\n" link_result;
    exit 1);

  (* 一時オブジェクトファイルを削除 *)
  Sys.remove obj_file;

  Printf.printf "Executable created: %s\n" output_exe

let () =
  (* Phase 1-6: Cli.Options を使用したオプション解析 *)
  let opts =
    match Cli.Options.parse_args Sys.argv with
    | Ok opts -> opts
    | Error msg ->
        prerr_endline msg;
        exit 1
  in

  (* ファイルを開いてソース文字列を読み込む *)
  let ic = open_in opts.input_file in
  let source = really_input_string ic (in_channel_length ic) in
  close_in ic;

  if opts.stats then Cli.Stats.set_input_size_bytes (String.length source);

  let collect_trace = opts.trace || opts.stats in
  let emit_trace_logs = opts.trace in
  let record_start phase =
    if collect_trace then Cli.Trace.start_phase ~emit_log:emit_trace_logs phase
  in
  let record_end phase =
    if collect_trace then Cli.Trace.end_phase ~emit_log:emit_trace_logs phase
  in

  (* パース用にソース文字列から lexbuf を作成 *)
  let lexbuf = Lexing.from_string source in
  lexbuf.Lexing.lex_curr_p <-
    { lexbuf.Lexing.lex_curr_p with Lexing.pos_fname = opts.input_file };

  (* Phase 1-6 Week 15: トレース開始 *)
  record_start Parsing;

  match Parser_driver.parse lexbuf with
  | Ok ast ->
      (* Phase 1-6 Week 15: パース完了 *)
      record_end Parsing;

      (* Phase 1: AST 出力 *)
      (if opts.emit_ast then
         let rendered = Ast_printer.string_of_compilation_unit ast in
         Printf.printf "%s\n" rendered);

      (* Phase 2+: 型推論が必要な処理 *)
      if opts.emit_tast || opts.emit_ir || opts.emit_bc || opts.verify_ir then (
        (* Phase 1-6 Week 15: 型推論開始 *)
        record_start TypeChecking;

        match Type_inference.infer_compilation_unit ast with
        | Ok tast ->
            (* Phase 1-6 Week 15: 型推論完了 *)
            record_end TypeChecking;

            (* Phase 2: Typed AST 出力 *)
            (if opts.emit_tast then
               let rendered = Typed_ast.string_of_typed_compilation_unit tast in
               Printf.printf "%s\n" rendered);

            (* Phase 3: LLVM IR 生成パイプライン *)
            if
              opts.emit_ir || opts.emit_bc || opts.verify_ir || opts.link_runtime
            then (
              let opt_config =
                Core_ir.Pipeline.
                  {
                    opt_level = O1;
                    enable_const_fold = true;
                    enable_dce = true;
                    max_iterations = 10;
                    verbose = false;
                    emit_intermediate = false;
                  }
              in
              let run_backend ~mode ~mode_label ~out_dir ~collect_metrics =
                Cli.File_util.ensure_directory out_dir;
                let prefix =
                  if mode_label = "" then ""
                  else Printf.sprintf "[%s] " mode_label
                in
                let record_start_if enabled phase =
                  if enabled then record_start phase
                in
                let record_end_if enabled phase =
                  if enabled then record_end phase
                in
                try
                  record_start_if collect_metrics CoreIR;
                  let core_ir =
                    let desugared =
                      Core_ir.Desugar.desugar_compilation_unit tast
                    in
                    Core_ir.Monomorphize_poc.apply ~mode desugared
                  in
                  record_end_if collect_metrics CoreIR;
                  record_start_if collect_metrics Optimization;
                  let optimized_ir, _stats =
                    Core_ir.Pipeline.optimize_module ~config:opt_config core_ir
                  in
                  record_end_if collect_metrics Optimization;
                  record_start_if collect_metrics CodeGen;
                  let llvm_module =
                    Codegen.codegen_module ~target_name:opts.target optimized_ir
                  in
                  record_end_if collect_metrics CodeGen;
                  if opts.verify_ir then
                    match Verify.verify_llvm_ir llvm_module with
                    | Ok () ->
                        Printf.printf "%sLLVM IR verification passed.\n" prefix
                    | Error err ->
                        let diag = Verify.error_to_diagnostic err None in
                        print_diagnostic opts None diag;
                        exit 1;

                  let basename = get_basename opts.input_file in
                  let ll_file_opt = ref None in
                  if opts.emit_ir then (
                    let output_path =
                      output_filename out_dir basename ".ll"
                    in
                    Codegen.emit_llvm_ir llvm_module output_path;
                    Printf.printf "%sLLVM IR written to: %s\n" prefix output_path;
                    ll_file_opt := Some output_path);

                  if opts.emit_bc then (
                    let output_path =
                      output_filename out_dir basename ".bc"
                    in
                    Codegen.emit_llvm_bc llvm_module output_path;
                    Printf.printf "%sLLVM Bitcode written to: %s\n" prefix
                      output_path);

                  if opts.link_runtime then (
                    let ll_file =
                      match !ll_file_opt with
                      | Some path -> path
                      | None ->
                          let temp_path =
                            output_filename out_dir basename ".ll"
                          in
                          Codegen.emit_llvm_ir llvm_module temp_path;
                          temp_path
                    in
                    if not (Sys.file_exists opts.runtime_path) then (
                      Printf.eprintf "Error: runtime library not found: %s\n"
                        opts.runtime_path;
                      Printf.eprintf
                        "Please build the runtime first with: make -C \
                         runtime/native runtime\n";
                      exit 1);
                    let output_exe =
                      output_filename out_dir basename ""
                    in
                    Printf.printf "%sLinking artifact into: %s\n" prefix
                      output_exe;
                    link_with_runtime ll_file opts.runtime_path output_exe;
                    if !ll_file_opt = None && not opts.emit_ir then
                      Sys.remove ll_file)
                with
                | Core_ir.Desugar.DesugarError (msg, ast_span) ->
                    let diag_span =
                      Diagnostic.
                        {
                          start_pos =
                            {
                              filename = opts.input_file;
                              line = 0;
                              column = 0;
                              offset = ast_span.Ast.start;
                            };
                          end_pos =
                            {
                              filename = opts.input_file;
                              line = 0;
                              column = 0;
                              offset = ast_span.Ast.end_;
                            };
                        }
                    in
                    let diag =
                      Diagnostic.
                        {
                          severity = Error;
                          severity_hint = None;
                          domain = None;
                          code = Some "E8001";
                          message = Printf.sprintf "Core IR 変換エラー: %s" msg;
                          span = diag_span;
                          expected_summary = None;
                          notes = [];
                          fixits = [];
                        }
                    in
                    print_diagnostic opts (Some source) diag;
                    exit 1
                | Codegen.CodegenError msg ->
                    let dummy_loc =
                      Diagnostic.
                        {
                          filename = opts.input_file;
                          line = 0;
                          column = 0;
                          offset = 0;
                        }
                    in
                    let diag =
                      Diagnostic.
                        {
                          severity = Error;
                          severity_hint = None;
                          domain = None;
                          code = Some "E8002";
                          message = Printf.sprintf "LLVM IR 生成エラー: %s" msg;
                          span = { start_pos = dummy_loc; end_pos = dummy_loc };
                          expected_summary = None;
                          notes = [];
                          fixits = [];
                        }
                    in
                    print_diagnostic opts None diag;
                    exit 1
              in
              Cli.File_util.ensure_directory opts.out_dir;
              (match opts.Cli.Options.typeclass_mode with
              | Cli.Options.TypeclassDictionary ->
                  run_backend
                    ~mode:Core_ir.Monomorphize_poc.UseDictionary
                    ~mode_label:"dictionary" ~out_dir:opts.out_dir
                    ~collect_metrics:true
              | Cli.Options.TypeclassMonomorph ->
                  run_backend
                    ~mode:Core_ir.Monomorphize_poc.UseMonomorph
                    ~mode_label:"monomorph" ~out_dir:opts.out_dir
                    ~collect_metrics:true
              | Cli.Options.TypeclassBoth ->
                  let dict_dir =
                    Filename.concat opts.out_dir "dictionary"
                  in
                  let mono_dir =
                    Filename.concat opts.out_dir "monomorph"
                  in
                  run_backend
                    ~mode:Core_ir.Monomorphize_poc.UseDictionary
                    ~mode_label:"dictionary" ~out_dir:dict_dir
                    ~collect_metrics:true;
                  run_backend
                    ~mode:Core_ir.Monomorphize_poc.UseMonomorph
                    ~mode_label:"monomorph" ~out_dir:mono_dir
                    ~collect_metrics:false))
        | Error type_err ->
            (* 型推論エラー *)
            let diag =
              Type_error.to_diagnostic_with_source source opts.input_file
                type_err
            in
            print_diagnostic opts (Some source) diag;
            exit 1);

      (* Phase 1-6 Week 15: トレース・統計サマリー出力（正常終了時） *)
      let trace_summary =
        if collect_trace then Some (Cli.Trace.summary ()) else None
      in
      if opts.stats then (
        (match trace_summary with
        | Some summary -> Cli.Stats.update_trace_summary summary
        | None -> ());
        Cli.Stats.print_stats ());
      if opts.trace then Cli.Trace.print_summary ?summary_data:trace_summary ();

      (* Phase 1-6 Week 16: メトリクス出力（--metrics指定時） *)
      (match opts.metrics_path with
      | Some path -> (
          (* トレース情報を統計に統合（まだの場合） *)
          (match trace_summary with
          | Some summary when not opts.stats ->
              Cli.Stats.update_trace_summary summary
          | _ -> ());

          (* メトリクスをファイルに出力 *)
          let content =
            match opts.metrics_format with
            | Cli.Options.MetricsJson -> Cli.Stats.to_json ()
            | Cli.Options.MetricsCsv -> Cli.Stats.to_csv ()
          in
          try
            let oc = open_out path in
            output_string oc content;
            close_out oc;
            Printf.eprintf "[METRICS] Metrics written to: %s\n%!" path
          with Sys_error msg ->
            Printf.eprintf "[METRICS] Error writing metrics file: %s\n%!" msg;
            exit 1)
      | None -> ());

      exit 0
  | Error diag ->
      (* Phase 1-6 Week 15: パース失敗時はトレース終了 *)
      if collect_trace then
        Cli.Trace.end_phase ~emit_log:emit_trace_logs Parsing;

      print_diagnostic opts (Some source) diag;
      exit 1
