module CI = Core_ir
module Audit = Audit_envelope
module Ffi_stub = Ffi_stub_builder
module Json = Yojson.Basic

type case = {
  label : string;
  target : string;
  ir_golden : string;
  audit_golden : string;
}

let project_root =
  match Sys.getenv_opt "DUNE_SOURCEROOT" with
  | Some root -> root
  | None -> Filename.dirname Sys.argv.(0)

let resolve path = Filename.concat project_root path

let read_file path =
  In_channel.with_open_text path In_channel.input_all

let ensure_dir path =
  if not (Sys.file_exists path) then Unix.mkdir path 0o755

let actual_dir = resolve "tests/golden/_actual"

let write_actual name suffix contents =
  ensure_dir actual_dir;
  let path = Filename.concat actual_dir (name ^ suffix) in
  Out_channel.with_open_text path (fun oc ->
      output_string oc contents;
      if contents <> "" && contents.[String.length contents - 1] <> '\n' then
        output_char oc '\n');
  path

let normalize_ir_line line =
  if String.starts_with ~prefix:";" line then Some line
  else if String.starts_with ~prefix:"source_filename" line then None
  else if String.trim line = "" then Some ""
  else Some line

let normalize_ir ir =
  ir |> String.split_on_char '\n'
  |> List.filter_map normalize_ir_line
  |> String.concat "\n" |> String.trim

let compile_case ~source_path ~target =
  let source = read_file source_path in
  let ast =
    match Parser_driver.parse_string ~filename:source_path source with
    | Ok ast -> ast
    | Error diag ->
        Printf.eprintf "✗ %s: 構文解析に失敗しました\n%s\n" source_path
          (Diagnostic.to_string diag);
        exit 1
  in
  match Type_inference.infer_compilation_unit ast with
  | Error err ->
      let diag =
        Type_error.to_diagnostic_with_source source source_path err
      in
      Printf.eprintf "✗ %s: 型推論に失敗しました\n%s\n" source_path
        (Diagnostic.to_string diag);
      exit 1
  | Ok tast ->
      let desugared = CI.Desugar.desugar_compilation_unit tast in
      let core_ir =
        CI.Monomorphize_poc.apply ~mode:CI.Monomorphize_poc.UseDictionary
          desugared
      in
      let opt_config =
        CI.Pipeline.
          {
            opt_level = CI.Pipeline.O1;
            enable_const_fold = true;
            enable_dce = true;
            max_iterations = 10;
            verbose = false;
            emit_intermediate = false;
          }
      in
      let optimized_ir, _stats =
        CI.Pipeline.optimize_module ~config:opt_config core_ir
      in
      let snapshots = Type_inference.current_ffi_bridge_snapshots () in
      let stub_plans =
        List.mapi
          (fun _ snapshot ->
            let normalized = Type_inference.ffi_snapshot_normalized snapshot in
            Ffi_stub.make_stub_plan
              ~param_types:
                (Type_inference.ffi_snapshot_param_types snapshot)
              ~return_type:
                (Type_inference.ffi_snapshot_return_type snapshot)
              normalized.contract)
          snapshots
      in
      let llvm_module =
        Codegen.codegen_module ~target_name:target ~stub_plans optimized_ir
      in
      (llvm_module, snapshots)

let audit_lines_of_snapshots snapshots =
  let events =
    snapshots
    |> List.map (fun snapshot ->
           let normalized = Type_inference.ffi_snapshot_normalized snapshot in
           let target =
             match normalized.Ffi_contract.target with
             | Some t -> t
             | None -> ""
           in
           let metadata =
             Ffi_contract.bridge_audit_metadata ~status:"ok" normalized
           in
           let event =
             Audit.make ~timestamp:"1970-01-01T00:00:00Z"
               ~category:"ffi.bridge" ~metadata ()
           in
           (target, Audit.to_json event))
    |> List.sort (fun (a, _) (b, _) -> compare a b)
  in
  events
  |> List.map (fun (_, json) -> Json.to_string json)
  |> String.concat "\n" |> fun s -> s ^ "\n"

let compare_with_golden case =
  let sample_path = resolve "tests/samples/ffi/cli-callconv-sample.reml" in
  let llvm_module, snapshots =
    compile_case ~source_path:sample_path ~target:case.target
  in
  let ir_actual = Llvm.string_of_llmodule llvm_module |> normalize_ir in
  let ir_golden_path = resolve case.ir_golden in
  if not (Sys.file_exists ir_golden_path) then (
    let actual_path = write_actual case.label ".ll.actual" ir_actual in
    Printf.eprintf "✗ %s: IR ゴールデン %s が存在しません。%s を参照してください。\n"
      case.label ir_golden_path actual_path;
    exit 1);
  let ir_expected =
    read_file ir_golden_path |> normalize_ir
  in
  if String.compare ir_actual ir_expected <> 0 then (
    let actual_path = write_actual case.label ".ll.actual" ir_actual in
    Printf.eprintf
      "✗ %s: IR ゴールデンと一致しません。期待値: %s, 実際: %s\n"
      case.label ir_golden_path actual_path;
    exit 1);

  let audit_actual = audit_lines_of_snapshots snapshots in
  let audit_golden_path = resolve case.audit_golden in
  if not (Sys.file_exists audit_golden_path) then (
    let actual_path = write_actual case.label ".audit.actual.jsonl" audit_actual in
    Printf.eprintf
      "✗ %s: 監査ゴールデン %s が存在しません。%s を参照してください。\n"
      case.label audit_golden_path actual_path;
    exit 1);
  let audit_expected = read_file audit_golden_path in
  if String.trim audit_expected <> String.trim audit_actual then (
    let actual_path =
      write_actual case.label ".audit.actual.jsonl" audit_actual
    in
    Printf.eprintf
      "✗ %s: 監査ゴールデンと一致しません。期待値: %s, 実際: %s\n"
      case.label audit_golden_path actual_path;
    exit 1);

  Printf.printf "✓ %s (target=%s)\n%!" case.label case.target

let () =
  let cases =
    [
      {
        label = "cli-linux";
        target = "x86_64-linux";
        ir_golden = "tests/golden/ffi/cli-linux.ll.golden";
        audit_golden =
          "tests/golden/audit/cli-ffi-bridge-linux.jsonl.golden";
      };
      {
        label = "cli-windows";
        target = "x86_64-pc-windows-msvc";
        ir_golden = "tests/golden/ffi/cli-windows.ll.golden";
        audit_golden =
          "tests/golden/audit/cli-ffi-bridge-windows.jsonl.golden";
      };
      {
        label = "cli-macos";
        target = "arm64-apple-darwin";
        ir_golden = "tests/golden/ffi/cli-macos.ll.golden";
        audit_golden =
          "tests/golden/audit/cli-ffi-bridge-macos.jsonl.golden";
      };
    ]
  in
  List.iter compare_with_golden cases;
  Printf.printf
    "CLI FFI call convention snapshots verified for %d targets.\n%!"
    (List.length cases)
