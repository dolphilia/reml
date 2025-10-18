open Core_ir.Ir

let dummy_span = Ast.dummy_span

let make_metadata ?target ?callconv ?ownership ?link_name () =
  {
    Ast.extern_target = target;
    extern_calling_convention = callconv;
    extern_link_name = link_name;
    extern_ownership = ownership;
    extern_invalid_attributes = [];
  }

let make_contract ?block_target ?target ?callconv ?ownership ?link_name () =
  let metadata = make_metadata ?target ?callconv ?ownership ?link_name () in
  Ffi_contract.bridge_contract ?block_target ~extern_name:"ffi_entry"
    ~source_span:dummy_span ~metadata ()

let build_plan () =
  make_contract
    ~block_target:"ffi-block-target"
    ~target:"x86_64-pc-windows-msvc"
    ~callconv:"msvc"
    ~ownership:"transferred"
    ~link_name:"ffi_entry_symbol" ()
  |> Ffi_stub_builder.make_stub_plan

let empty_module =
  {
    module_name = "FfiLowering";
    type_defs = [];
    global_defs = [];
    function_defs = [];
  }

let metadata_strings node =
  Llvm.get_mdnode_operands node
  |> Array.to_list
  |> List.filter_map Llvm.get_mdstring

let assert_contains label strings expected =
  if List.exists (fun value -> String.equal value expected) strings then ()
  else (
    Printf.eprintf "Missing metadata (%s): %s\n" label expected;
    exit 1)

let () =
  let plan = build_plan () in
  let llvm_module = Codegen.codegen_module ~stub_plans:[ plan ] empty_module in

  (* モジュールフラグを確認 *)
  let llctx = Llvm.module_context llvm_module in
  let flag_opt = Llvm.get_module_flag llvm_module "reml.bridge.version" in
  let flag_value =
    match flag_opt with
    | None ->
        Printf.eprintf "Missing reml.bridge.version flag\n";
        exit 1
    | Some md -> (
        let value = Llvm.metadata_as_value llctx md in
        match Llvm.int64_of_const value with
        | Some v -> v
        | None ->
            Printf.eprintf "Module flag is not an integer constant\n";
            exit 1)
  in
  if not (Int64.equal flag_value 1L) then (
    Printf.eprintf "Unexpected bridge flag value: %Ld\n" flag_value;
    exit 1);

  (* メタデータを抽出 *)
  let nodes =
    Llvm.get_named_metadata llvm_module "reml.bridge.stubs" |> Array.to_list
  in
  if List.length nodes <> 1 then (
    Printf.eprintf "Expected exactly one stub metadata node, found %d\n"
      (List.length nodes);
    exit 1);

  let entries = metadata_strings (List.hd nodes) in
  let expect =
    [
      "bridge.stub_index=1";
      "bridge.extern_name=ffi_entry";
      "bridge.target=x86_64-pc-windows-msvc";
      "bridge.callconv=win64";
      "bridge.abi=msvc";
      "bridge.ownership=transferred";
      "bridge.block_target=ffi-block-target";
      "bridge.extern_symbol=ffi_entry_symbol";
      "bridge.platform=windows-msvc-x64";
    ]
  in
  List.iter (assert_contains "stub-metadata" entries) expect;

  Printf.printf "FFI lowering metadata test passed.\n";
  exit 0
