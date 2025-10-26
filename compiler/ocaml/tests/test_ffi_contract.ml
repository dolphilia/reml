(* test_ffi_contract.ml — FFI 契約診断のゴールデンテスト *)

open Ast

let () = Unix.putenv "REMLC_FIXED_TIMESTAMP" "1970-01-01T00:00:00Z"

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
      (let diag =
         Type_error.to_diagnostic_with_source ~available_names:[] sample_source
           sample_path err
        |> Diagnostic.set_audit_id "00000000-0000-0000-0000-000000000000"
        |> Diagnostic.set_change_set
              (`Assoc
                [
                  ("command", `String "remlc-tests");
                  ( "input",
                    `String
                      "tests/golden/diagnostics/ffi/unsupported-abi.reml" );
                ])
       in
      let diag = { diag with timestamp = "1970-01-01T00:00:00Z" } in
      let json =
        Cli.Json_formatter.diagnostic_to_json ~mode:Cli.Options.JsonPretty diag
      in
       let audit =
         match err with
         | Type_error.FfiContractSymbolMissing normalized
         | Type_error.FfiContractOwnershipMismatch normalized
         | Type_error.FfiContractUnsupportedAbi normalized ->
             let event =
               Audit_envelope.make ~timestamp:"1970-01-01T00:00:00Z"
                 ~category:"ffi.bridge"
                 ~audit_id:"00000000-0000-0000-0000-000000000000"
                 ~change_set:
                   (`Assoc
                     [
                       ("command", `String "remlc-tests");
                       ( "input",
                         `String
                           "tests/golden/diagnostics/ffi/unsupported-abi.reml" );
                     ])
                 ~metadata_pairs:
                   (Ffi_contract.bridge_audit_metadata_pairs ~status:"error"
                      normalized)
                 ()
             in
             Yojson.Basic.to_string (Audit_envelope.to_json event) ^ "\n"
         | _ -> ""
       in
       compare_with_golden ~name:"unsupported-abi" ~json ~audit);
      Printf.printf "================================\n";
      Printf.printf "FFI contract diagnostics golden completed\n"
