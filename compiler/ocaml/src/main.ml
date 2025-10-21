module EffectTable = Constraint_solver.EffectConstraintTable
module IteratorAudit = Core_ir.Iterator_audit
module Ffi = Ffi_contract
module Ffi_stub = Ffi_stub_builder

let iterator_audit_entries : (string, IteratorAudit.entry) Hashtbl.t =
  Hashtbl.create 32

let json_of_span (span : Ast.span) =
  `Assoc [ ("start", `Int span.start); ("end", `Int span.end_) ]

let json_of_tag tag =
  `Assoc
    [
      ("name", `String tag.Effect_profile.effect_name);
      ("span", json_of_span tag.effect_span);
    ]

let json_of_tag_list tags = `List (List.map json_of_tag tags)

let metadata_for_effect ?symbol ?source_name ~source_span ~stage_requirement
    ~resolved_stage ~resolved_capability ~effect_set ~stage_trace
    ~diagnostic_payload extra_fields : Audit_envelope.metadata =
  let symbol = match symbol with Some name -> name | None -> "<anonymous>" in
  let stage_required =
    Effect_profile.stage_requirement_to_string stage_requirement
  in
  let stage_actual =
    match resolved_stage with
    | Some stage -> `String (Effect_profile.stage_id_to_string stage)
    | None -> `Null
  in
  let capability_json =
    match resolved_capability with
    | Some value when String.trim value <> "" -> `String value
    | Some _ -> `Null
    | None -> `Null
  in
  let diagnostic_json =
    Effect_profile.effect_diagnostic_payload_to_json diagnostic_payload
  in
  let residual_leaks =
    diagnostic_payload.Effect_profile.residual_leaks
    |> List.map (fun leak -> `String leak.Effect_profile.leaked_tag.effect_name)
  in
  let base_fields =
    [
      ("symbol", `String symbol);
      ( "effect.source",
        match source_name with Some name -> `String name | None -> `Null );
      ("effect.source.span", json_of_span source_span);
      ("effect.stage.required", `String stage_required);
      ("effect.stage.actual", stage_actual);
      ("effect.stage.capability", capability_json);
      ("effects.declared", json_of_tag_list effect_set.Effect_profile.declared);
      ("effects.residual", json_of_tag_list effect_set.residual);
      ("effects.diagnostic_payload", diagnostic_json);
      ( "effect.residual.leak_count",
        `Int (List.length diagnostic_payload.residual_leaks) );
    ]
  in
  let fields =
    if residual_leaks = [] then base_fields
    else ("effect.residual.missing", `List residual_leaks) :: base_fields
  in
  let fields =
    if stage_trace = [] then fields
    else
      ("stage_trace", Effect_profile.stage_trace_to_json stage_trace) :: fields
  in
  List.rev_append extra_fields fields

let event_of_effect_entry ?audit_id ?change_set (entry : EffectTable.entry) =
  let metadata =
    metadata_for_effect ~symbol:entry.symbol ?source_name:entry.source_name
      ~source_span:entry.source_span ~stage_requirement:entry.stage_requirement
      ~resolved_stage:entry.resolved_stage
      ~resolved_capability:entry.resolved_capability
      ~effect_set:entry.effect_set ~stage_trace:entry.stage_trace
      ~diagnostic_payload:entry.diagnostic_payload []
  in
  Audit_envelope.make ?audit_id ?change_set ~category:"effect.stage"
    ~metadata_pairs:metadata ()

let event_of_profile ?audit_id ?change_set ?symbol
    (profile : Effect_profile.profile) =
  let metadata =
    metadata_for_effect ?symbol ?source_name:profile.source_name
      ~source_span:profile.source_span
      ~stage_requirement:profile.stage_requirement
      ~resolved_stage:profile.resolved_stage
      ~resolved_capability:profile.resolved_capability
      ~effect_set:profile.effect_set ~stage_trace:profile.stage_trace
      ~diagnostic_payload:profile.diagnostic_payload
      [ ("status", `String "error") ]
  in
  Audit_envelope.make ?audit_id ?change_set ~category:"effect.stage.error"
    ~metadata_pairs:metadata ()

let event_of_stage_mismatch ?audit_id ?change_set ~function_name ~required_stage
    ~actual_stage ~capability ~stage_trace () =
  let metadata =
    [
      ("symbol", `String function_name);
      ("effect.stage.required", `String required_stage);
      ("effect.stage.actual", `String actual_stage);
      ( "effect.stage.capability",
        match capability with Some cap -> `String cap | None -> `Null );
      ("status", `String "error");
    ]
  in
  let metadata =
    if stage_trace = [] then metadata
    else
      ("stage_trace", Effect_profile.stage_trace_to_json stage_trace)
      :: metadata
  in
  Audit_envelope.make ?audit_id ?change_set ~category:"effect.stage.error"
    ~metadata_pairs:metadata ()

let runtime_stage_event ?audit_id ?change_set
    (context : Type_inference_effect.runtime_stage) =
  let capabilities =
    context.Type_inference_effect.capability_stages
    |> List.map (fun (name, stage) ->
           `Assoc
             [
               ("name", `String name);
               ("stage", `String (Effect_profile.stage_id_to_string stage));
             ])
  in
  let metadata =
    [
      ( "effect.stage.default",
        `String (Effect_profile.stage_id_to_string context.default_stage) );
      ("effect.stage.capabilities", `List capabilities);
    ]
  in
  let has_source trace source =
    List.exists
      (fun (step : Effect_profile.stage_trace_step) ->
        String.equal step.source source)
      trace
  in
  let append_step trace source =
    let stage_id = context.Type_inference_effect.default_stage in
    let step = Effect_profile.stage_trace_step_of_stage_id source stage_id in
    trace @ [ step ]
  in
  let stage_trace =
    let base = context.Type_inference_effect.stage_trace in
    let with_typer =
      if has_source base "typer" then base else append_step base "typer"
    in
    if has_source with_typer "runtime" then with_typer
    else append_step with_typer "runtime"
  in
  let metadata =
    if stage_trace = [] then metadata
    else
      ("stage_trace", Effect_profile.stage_trace_to_json stage_trace)
      :: metadata
  in
  Audit_envelope.make ?audit_id ?change_set ~category:"effect.stage.runtime"
    ~metadata_pairs:metadata ()

let iterator_stage_event ?audit_id ?change_set runtime_context
    (entry : IteratorAudit.entry) =
  let capability_name = entry.IteratorAudit.capability in
  let actual_stage_id =
    Type_inference_effect.stage_for_capability runtime_context capability_name
  in
  let actual_stage = Effect_profile.stage_id_to_string actual_stage_id in
  let required_stage =
    match entry.IteratorAudit.required_stage with
    | Some requirement -> Effect_profile.stage_requirement_to_string requirement
    | None -> actual_stage
  in
  let stage_source =
    let rec find = function
      | [] -> None
      | (step : Effect_profile.stage_trace_step) :: rest -> (
          match step.stage with
          | Some stage when String.equal stage actual_stage -> Some step.source
          | _ -> find rest)
    in
    find runtime_context.Type_inference_effect.stage_trace
    |> Option.value ~default:"runtime"
  in
  let base_trace = runtime_context.Type_inference_effect.stage_trace in
  let has_source trace source =
    List.exists
      (fun (step : Effect_profile.stage_trace_step) ->
        String.equal step.source source)
      trace
  in
  let typer_step =
    match capability_name with
    | Some cap ->
        Effect_profile.stage_trace_step_of_stage_id ~capability:cap "typer"
          actual_stage_id
    | None ->
        Effect_profile.stage_trace_step_of_stage_id "typer" actual_stage_id
  in
  let runtime_step =
    match capability_name with
    | Some cap ->
        Effect_profile.stage_trace_step_of_stage_id ~capability:cap "runtime"
          actual_stage_id
    | None ->
        Effect_profile.stage_trace_step_of_stage_id "runtime" actual_stage_id
  in
  let trace_with_typer =
    if has_source base_trace "typer" then base_trace
    else
      match base_trace with
      | [] -> [ typer_step ]
      | first :: rest -> first :: typer_step :: rest
  in
  let stage_trace =
    if has_source trace_with_typer "runtime" then trace_with_typer
    else
      match trace_with_typer with
      | [] -> [ runtime_step ]
      | [ single ] -> [ single; runtime_step ]
      | first :: second :: rest when String.equal second.source "typer" ->
          first :: second :: runtime_step :: rest
      | first :: rest -> first :: runtime_step :: rest
  in
  let capability_json =
    match capability_name with
    | Some cap when cap <> "" -> `String cap
    | _ -> `Null
  in
  let iterator_kind_json =
    match entry.IteratorAudit.iterator_kind with
    | Some kind -> `String kind
    | None -> `Null
  in
  let iterator_source_json =
    match entry.IteratorAudit.iterator_source with
    | Some src -> `String src
    | None -> `Null
  in
  let metadata =
    [
      ("symbol", `String entry.IteratorAudit.function_name);
      ("effect.stage.required", `String required_stage);
      ("effect.stage.actual", `String actual_stage);
      ("effect.stage.source", `String "runtime");
      ("effect.capability", capability_json);
      ("effect.stage.iterator.required", `String required_stage);
      ("effect.stage.iterator.actual", `String actual_stage);
      ("effect.stage.iterator.capability", capability_json);
      ("effect.stage.iterator.kind", iterator_kind_json);
      ("effect.stage.iterator.source", `String stage_source);
      ("effect.stage.iterator.source_detail", iterator_source_json);
      ("effect.stage.iterator.method", `String entry.IteratorAudit.method_name);
      ( "audit.note",
        `String
          (Printf.sprintf "Iterator audit (%s.%s)"
             entry.IteratorAudit.function_name entry.IteratorAudit.method_name)
      );
      ("stage_trace", Effect_profile.stage_trace_to_json stage_trace);
    ]
  in
  Audit_envelope.make ?audit_id ?change_set ~category:"effect.stage"
    ~metadata_pairs:metadata ()

let iterator_audit_events ?audit_id ?change_set runtime_context =
  Hashtbl.fold
    (fun _ entry acc ->
      iterator_stage_event ?audit_id ?change_set runtime_context entry :: acc)
    iterator_audit_entries []

let ffi_bridge_events ?audit_id ?change_set () =
  Type_inference.current_ffi_bridge_snapshots ()
  |> List.map (fun snapshot ->
         Audit_envelope.make ?audit_id ?change_set ~category:"ffi.bridge"
           ~metadata_pairs:
             (Ffi.bridge_audit_metadata_pairs ~status:"ok"
                (Type_inference.ffi_snapshot_normalized snapshot))
           ())

let events_from_effect_constraints ?audit_id ?change_set () =
  Constraint_solver.current_effect_constraints ()
  |> EffectTable.to_list
  |> List.map (event_of_effect_entry ?audit_id ?change_set)

let event_of_type_error ?audit_id ?change_set err =
  match err with
  | Type_error.EffectResidualLeak { function_name; profile; _ } ->
      Some
        (event_of_profile ?audit_id ?change_set ?symbol:function_name profile)
  | Type_error.EffectStageMismatch
      {
        required_stage;
        actual_stage;
        function_name;
        capability;
        stage_trace;
        _;
      } ->
      let symbol =
        match function_name with Some name -> name | None -> "<anonymous>"
      in
      Some
        (event_of_stage_mismatch ?audit_id ?change_set
           ~function_name:symbol ~required_stage ~actual_stage ~capability
           ~stage_trace ())
  | Type_error.FfiContractSymbolMissing normalized
  | Type_error.FfiContractOwnershipMismatch normalized
  | Type_error.FfiContractUnsupportedAbi normalized ->
      Some
        (Audit_envelope.make ?audit_id ?change_set ~category:"ffi.bridge"
           ~metadata_pairs:
             (Ffi.bridge_audit_metadata_pairs ~status:"error" normalized)
           ())
  | _ -> None

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
          ~include_snippet:opts.Cli.Options.include_snippet
    | Cli.Options.Json ->
        Cli.Json_formatter.diagnostic_to_json ~mode:opts.Cli.Options.json_mode
          diag
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
  let audit_context = Cli.Audit_path_resolver.resolve opts in

  let audit_seed =
    let input_label =
      if opts.use_stdin then "<stdin>" else opts.input_file
    in
    String.concat "|"
      [
        input_label;
        opts.target;
        String.concat " " (Array.to_list Sys.argv);
      ]
  in
  let audit_id = Digest.(string audit_seed |> to_hex) in
  let requested_outputs =
    [
      ("emit_ast", opts.emit_ast);
      ("emit_tast", opts.emit_tast);
      ("emit_ir", opts.emit_ir);
      ("emit_bc", opts.emit_bc);
    ]
    |> List.filter_map (fun (name, enabled) ->
           if enabled then Some (`String name) else None)
  in
  let change_set_json =
    `Assoc
      [
        ("command", `String "remlc");
        ( "args",
          `List
            (Array.to_list Sys.argv |> List.map (fun arg -> `String arg)) );
        ( "input",
          `String (if opts.use_stdin then "<stdin>" else opts.input_file) );
        ("target", `String opts.target);
        ("outputs", `List requested_outputs);
      ]
  in
  let attach_audit diag =
    diag
    |> Diagnostic.set_audit_id audit_id
    |> Diagnostic.set_change_set change_set_json
  in

  (* ファイルを開いてソース文字列を読み込む *)
  let ic = open_in opts.input_file in
  let source = really_input_string ic (in_channel_length ic) in
  close_in ic;

  if opts.stats then Cli.Stats.set_input_size_bytes (String.length source);

  let collect_trace = opts.trace || opts.stats in
  let emit_trace_logs = opts.trace in
  let runtime_stage_context =
    Runtime_capability_resolver.resolve
      ~cli_override:opts.Cli.Options.effect_stage_override
      ~registry_path:opts.Cli.Options.runtime_capabilities_path
      ~target:(Some opts.target)
  in
  let type_config =
    Type_inference.make_config ~effect_context:runtime_stage_context ()
  in
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

        match Type_inference.infer_compilation_unit ~config:type_config ast with
        | Ok tast ->
            (* Phase 1-6 Week 15: 型推論完了 *)
            record_end TypeChecking;

            let audit_events =
              runtime_stage_event ~audit_id ~change_set:change_set_json
                runtime_stage_context
              :: (iterator_audit_events ~audit_id ~change_set:change_set_json
                    runtime_stage_context
                 @ ffi_bridge_events ~audit_id ~change_set:change_set_json ()
                 @ events_from_effect_constraints ~audit_id
                     ~change_set:change_set_json ())
            in
            Cli.Audit_persistence.append_events audit_context audit_events;

            (* Phase 2: Typed AST 出力 *)
            (if opts.emit_tast then
               let rendered = Typed_ast.string_of_typed_compilation_unit tast in
               Printf.printf "%s\n" rendered);

            (* Phase 3: LLVM IR 生成パイプライン *)
            if
              opts.emit_ir || opts.emit_bc || opts.verify_ir
              || opts.link_runtime
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
                  let () =
                    Core_ir.Iterator_audit.collect core_ir
                    |> List.iter (fun entry ->
                           let key = Core_ir.Iterator_audit.entry_key entry in
                           Hashtbl.replace iterator_audit_entries key entry)
                  in
                  record_end_if collect_metrics CoreIR;
                  record_start_if collect_metrics Optimization;
                  let optimized_ir, _stats =
                    Core_ir.Pipeline.optimize_module ~config:opt_config core_ir
                  in
                  record_end_if collect_metrics Optimization;
                  record_start_if collect_metrics CodeGen;
                  let stub_plans =
                    Type_inference.current_ffi_bridge_snapshots ()
                    |> List.map (fun snapshot ->
                           Ffi_stub.make_stub_plan
                             ~param_types:
                               (Type_inference.ffi_snapshot_param_types snapshot)
                             ~return_type:
                               (Type_inference.ffi_snapshot_return_type snapshot)
                             (Type_inference.ffi_snapshot_normalized snapshot)
                               .contract)
                  in
                  let llvm_module =
                    Codegen.codegen_module ~target_name:opts.target ~stub_plans
                      optimized_ir
                  in
                  record_end_if collect_metrics CodeGen;
                  (if opts.verify_ir then
                     match Verify.verify_llvm_ir llvm_module with
                     | Ok () ->
                         Printf.printf "%sLLVM IR verification passed.\n" prefix
                     | Error err ->
                         let diag = Verify.error_to_diagnostic err None in
                         print_diagnostic opts None diag;
                         ignore (Stdlib.exit 1));

                  let basename = get_basename opts.input_file in
                  let ll_file_opt = ref None in
                  if opts.emit_ir then (
                    let output_path = output_filename out_dir basename ".ll" in
                    Codegen.emit_llvm_ir llvm_module output_path;
                    Printf.printf "%sLLVM IR written to: %s\n" prefix
                      output_path;
                    ll_file_opt := Some output_path);

                  if opts.emit_bc then (
                    let output_path = output_filename out_dir basename ".bc" in
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
                    let output_exe = output_filename out_dir basename "" in
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
                      Diagnostic.(
                        Builder.create ~severity:Error
                          ~message:(Printf.sprintf "Core IR 変換エラー: %s" msg)
                          ~primary:diag_span ()
                        |> Builder.set_primary_code "E8001"
                        |> Builder.build)
                      |> attach_audit
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
                      Diagnostic.(
                        Builder.create ~severity:Error
                          ~message:(Printf.sprintf "LLVM IR 生成エラー: %s" msg)
                          ~primary:{ start_pos = dummy_loc; end_pos = dummy_loc }
                          ()
                        |> Builder.set_primary_code "E8002"
                        |> Builder.build)
                      |> attach_audit
                    in
                    print_diagnostic opts None diag;
                    exit 1
              in
              Cli.File_util.ensure_directory opts.out_dir;
              match opts.Cli.Options.typeclass_mode with
              | Cli.Options.TypeclassDictionary ->
                  run_backend ~mode:Core_ir.Monomorphize_poc.UseDictionary
                    ~mode_label:"dictionary" ~out_dir:opts.out_dir
                    ~collect_metrics:true
              | Cli.Options.TypeclassMonomorph ->
                  run_backend ~mode:Core_ir.Monomorphize_poc.UseMonomorph
                    ~mode_label:"monomorph" ~out_dir:opts.out_dir
                    ~collect_metrics:true
              | Cli.Options.TypeclassBoth ->
                  let dict_dir = Filename.concat opts.out_dir "dictionary" in
                  let mono_dir = Filename.concat opts.out_dir "monomorph" in
                  run_backend ~mode:Core_ir.Monomorphize_poc.UseDictionary
                    ~mode_label:"dictionary" ~out_dir:dict_dir
                    ~collect_metrics:true;
                  run_backend ~mode:Core_ir.Monomorphize_poc.UseMonomorph
                    ~mode_label:"monomorph" ~out_dir:mono_dir
                    ~collect_metrics:false)
        | Error type_err ->
            (* 型推論エラー *)
            let runtime_event =
              runtime_stage_event ~audit_id ~change_set:change_set_json
                runtime_stage_context
            in
            let constraint_events =
              events_from_effect_constraints ~audit_id
                ~change_set:change_set_json ()
            in
            let iterator_events =
              iterator_audit_events ~audit_id ~change_set:change_set_json
                runtime_stage_context
            in
            let events_with_error =
              match
                event_of_type_error ~audit_id ~change_set:change_set_json
                  type_err
              with
              | Some event -> event :: constraint_events
              | None -> constraint_events
            in
            Cli.Audit_persistence.append_events audit_context
              (runtime_event :: (iterator_events @ events_with_error));
            let diag =
              Type_error.to_diagnostic_with_source source opts.input_file
                type_err
              |> attach_audit
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

  let iterator_events =
    iterator_audit_events ~audit_id ~change_set:change_set_json
      runtime_stage_context
  in
  Cli.Audit_persistence.append_events audit_context
    (runtime_stage_event ~audit_id ~change_set:change_set_json
       runtime_stage_context
    :: iterator_events);

  let diag = attach_audit diag in
  print_diagnostic opts (Some source) diag;
  exit 1
