(* test_ffi_contract.ml — FFI 契約診断のゴールデンテスト *)

open Ast

let () = Unix.putenv "REMLC_FIXED_TIMESTAMP" "1970-01-01T00:00:00Z"

module Run_config = Parser_run_config

let project_root =
  match Sys.getenv_opt "DUNE_SOURCEROOT" with
  | Some root -> root
  | None -> Filename.dirname Sys.argv.(0)

let resolve path = Filename.concat project_root path
let golden_dir = resolve "tests/golden"

let ensure_actual_dir () =
  let actual_dir = Filename.concat golden_dir "_actual" in
  if not (Sys.file_exists actual_dir) then Unix.mkdir actual_dir 0o755;
  actual_dir

let write_snapshot name ext content =
  let actual_dir = ensure_actual_dir () in
  let path = Filename.concat actual_dir (name ^ ext) in
  Out_channel.with_open_text path (fun oc ->
      output_string oc content;
      if content <> "" && content.[String.length content - 1] <> '\n' then
        output_char oc '\n');
  path

let read_all path =
  if Sys.file_exists path then
    In_channel.with_open_text path In_channel.input_all
  else ""

let dummy_span = Ast.dummy_span
let ident ?(span = dummy_span) name = { name; span }

let make_type ?(span = dummy_span) name =
  { ty_kind = TyIdent (ident ~span name); ty_span = span }

let build_extern_decl span =
  let signature =
    {
      sig_name = ident ~span "ffi_demo";
      sig_params = [];
      sig_args = [];
      sig_ret = Some (make_type ~span "i32");
      sig_where = [];
      sig_effect_profile = None;
    }
  in
  let metadata =
    {
      extern_target = None;
      extern_calling_convention = Some "system_v";
      extern_link_name = Some "ffi_demo_c";
      extern_ownership = Some "borrowed";
      extern_invalid_attributes = [];
    }
  in
  let extern_item =
    { extern_attrs = []; extern_sig = signature; extern_metadata = metadata }
  in
  {
    extern_abi = "C";
    extern_block_target = Some "x86_64-pc-windows-msvc";
    extern_items = [ extern_item ];
  }

let build_compilation_unit span =
  let extern_decl = build_extern_decl span in
  let decl =
    {
      decl_attrs = [];
      decl_vis = Public;
      decl_kind = ExternDecl extern_decl;
      decl_span = span;
    }
  in
  { header = None; uses = []; decls = [ decl ] }

let compare_with_golden ~name ~json ~audit =
  let diag_golden =
    resolve
      (Filename.concat "tests/golden/diagnostics/ffi" (name ^ ".json.golden"))
  in
  let audit_golden =
    resolve (Filename.concat "tests/golden/audit" "ffi-bridge.jsonl.golden")
  in
  let expected_json = read_all diag_golden in
  let expected_audit = read_all audit_golden in
  let json_trim = String.trim json in
  let expected_json_trim = String.trim expected_json in
  if json_trim <> expected_json_trim then (
    let actual_path = write_snapshot name ".actual.json" json in
    Printf.eprintf "✗ %s: JSON ゴールデンとの差分を検出しました (期待: %s, 現在: %s)\n" name
      diag_golden actual_path;
    exit 1);
  let audit_trim = String.trim audit in
  let expected_audit_trim = String.trim expected_audit in
  if audit_trim <> expected_audit_trim then (
    let actual_path = write_snapshot name ".audit.actual.jsonl" audit in
    Printf.eprintf "✗ %s: 監査ゴールデンとの差分を検出しました (期待: %s, 現在: %s)\n" name
      audit_golden actual_path;
    exit 1);
  Printf.printf "✓ %s\n" name

let () =
  Printf.printf "FFI Contract Diagnostics Golden\n";
  Printf.printf "================================\n";
  let sample_path =
    resolve "tests/golden/diagnostics/ffi/unsupported-abi.reml"
  in
  let sample_source = read_all sample_path in
  let span = { Ast.start = 0; end_ = String.length sample_source } in
  let cu = build_compilation_unit span in
  match Type_inference.infer_compilation_unit cu with
  | Ok _ ->
      Printf.eprintf "✗ unsupported_abi: 型推論が成功しました (エラーを期待)\n";
      exit 1
  | Error err ->
      (let timestamp = "1970-01-01T00:00:00Z" in
       let build_id = Diagnostic.compute_build_id ~timestamp () in
       let sequence = 0 in
       let workspace = Some "." in
       let change_set =
         Diagnostic.make_change_set_template ~origin:"cli" ~build_id ?workspace
           ?sequence:(Some sequence)
           ~items:
             [
               `Assoc
                 [
                   ("kind", `String "cli-command");
                   ("command", `String "remlc-tests");
                 ];
               `Assoc
                 [
                   ("kind", `String "input");
                   ( "path",
                     `String
                       "tests/golden/diagnostics/ffi/unsupported-abi.reml" );
                 ];
             ]
           ()
       in
       let audit_id = Printf.sprintf "cli/%s#%d" build_id sequence in
       let diag =
         Type_error.to_diagnostic_with_source ~available_names:[]
           sample_source sample_path err
       |> fun diag -> { diag with timestamp }
       |> Diagnostic.apply_audit_policy_metadata ~channel:"cli" ~build_id
            ~sequence ?workspace
       |> Diagnostic.set_audit_id audit_id
        |> Diagnostic.set_change_set change_set
        |> Diagnostic.set_audit_metadata "bridge.audit_pass_rate" (`Float 1.0)
        |> Diagnostic.set_audit_metadata "bridge.status" (`String "ok")
        |> Diagnostic.set_audit_metadata "cli"
             (`Assoc
                [
                  ("audit_id", `String audit_id);
                  ("change_set", change_set);
                ])
       |> Diagnostic.set_audit_metadata "schema"
             (`Assoc [ ("version", `String "1.1") ])
      in
       let diag =
         Diagnostic.with_parser_runconfig_metadata ~config:Run_config.default
           diag
       in
       let json =
         Cli.Json_formatter.diagnostic_to_json ~mode:Cli.Options.JsonPretty diag
       in
      let audit =
        match err with
        | Type_error.FfiContractSymbolMissing normalized
        | Type_error.FfiContractOwnershipMismatch normalized
        | Type_error.FfiContractUnsupportedAbi normalized ->
            let base_metadata =
              Ffi_contract.bridge_audit_metadata_pairs ~status:"ok" normalized
            in
            let bridge_payload =
              match
                List.find_opt
                  (fun (key, _) -> String.equal key "bridge")
                  base_metadata
              with
              | Some (_, json) -> json
              | None -> `Assoc []
            in
            let metadata_pairs =
              [
                ("audit_id", `String audit_id);
                ("change_set", change_set);
                ("cli.audit_id", `String audit_id);
                ("cli.change_set", change_set);
                ("audit.channel", `String "cli");
                ("audit.build_id", `String build_id);
                ("audit.sequence", `Int sequence);
                ("schema.version", `String Audit_envelope.schema_version);
                ("audit.timestamp", `String timestamp);
              ]
              @ base_metadata
            in
            let metadata_json = `Assoc metadata_pairs in
            let extensions_json = `Assoc [ ("bridge", bridge_payload) ] in
            let event_json =
              `Assoc
                [
                  ("timestamp", `String timestamp);
                  ("category", `String "ffi.bridge");
                  ("metadata", metadata_json);
                  ("extensions", extensions_json);
                  ("audit_id", `String audit_id);
                  ("change_set", change_set);
                ]
            in
            Yojson.Basic.to_string event_json ^ "\n"
        | _ -> ""
      in
       compare_with_golden ~name:"unsupported-abi" ~json ~audit);
      Printf.printf "================================\n";
      Printf.printf "FFI contract diagnostics golden completed\n"
