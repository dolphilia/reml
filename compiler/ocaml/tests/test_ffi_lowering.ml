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
      "bridge.stub_symbol=__reml_stub_ffi_entry_1";
      "bridge.thunk_symbol=__reml_thunk_ffi_entry_symbol_1";
      "bridge.target=x86_64-pc-windows-msvc";
      "bridge.callconv=win64";
      "bridge.abi=msvc";
      "bridge.ownership=transferred";
      "bridge.block_target=ffi-block-target";
      "bridge.extern_symbol=ffi_entry_symbol";
      "bridge.platform=windows-msvc-x64";
      "bridge.arch=x86_64";
    ]
  in
  List.iter (assert_contains "stub-metadata" entries) expect;

  (* スタブ関数を検証 *)
  let stub_name = Ffi_stub_builder.stub_symbol_name ~index:0 plan in
  let stub_fn =
    match Llvm.lookup_function stub_name llvm_module with
    | Some fn -> fn
    | None ->
        Printf.eprintf "Stub function %s is missing\n" stub_name;
        exit 1
  in
  if Llvm.call_conv stub_fn <> Llvm.CallConv.c then (
    Printf.eprintf "Stub %s uses unexpected call convention\n" stub_name;
    exit 1);
  let stub_ir = Llvm.string_of_llvalue stub_fn in
  if not
       (Ffi_contract.contains_substring stub_ir
          "reml_ffi_bridge_record_status")
  then (
    Printf.eprintf
      "Stub %s does not record bridge status as expected\n" stub_name;
    exit 1);
  if not
       (Ffi_contract.contains_substring stub_ir
          "__reml_thunk_ffi_entry_symbol_1")
  then (
    Printf.eprintf
      "Stub %s does not call the generated thunk function\n" stub_name;
    exit 1);

  (* サンク（thunk）関数を検証 *)
  let thunk_name = Ffi_stub_builder.thunk_symbol_name ~index:0 plan in
  let thunk_fn =
    match Llvm.lookup_function thunk_name llvm_module with
    | Some fn -> fn
    | None ->
        Printf.eprintf "Thunk function %s is missing\n" thunk_name;
        exit 1
  in
  if Llvm.call_conv thunk_fn <> Llvm.CallConv.x86_64_win64 then (
    Printf.eprintf "Thunk %s uses unexpected call convention\n" thunk_name;
    exit 1);
  let thunk_ir = Llvm.string_of_llvalue thunk_fn in
  if not
       (Ffi_contract.contains_substring thunk_ir
          "reml_ffi_bridge_record_status")
  then (
    Printf.eprintf
      "Thunk %s does not record bridge status as expected\n" thunk_name;
    exit 1);

  Printf.printf "FFI lowering metadata test passed.\n";
  exit 0
