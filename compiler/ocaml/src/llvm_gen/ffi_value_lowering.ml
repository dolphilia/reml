open Ffi_stub_builder
module FlagBehavior = Llvm.ModuleFlagBehavior

let bridge_flag_name = "reml.bridge.version"
let bridge_flag_value = 1
let named_metadata = "reml.bridge.stubs"

let sanitize_value (value : string) =
  value |> String.trim
  |> String.map (fun ch -> if ch = '\n' || ch = '\r' then ' ' else ch)

let ensure_bridge_flag llctx llmodule =
  match Llvm.get_module_flag llmodule bridge_flag_name with
  | Some _ -> ()
  | None ->
      (* モジュールフラグは整数定数のメタデータとして設定する *)
      let i32_ty = Llvm.i32_type llctx in
      let const_value = Llvm.const_int i32_ty bridge_flag_value in
      (* value_as_metadataで整数定数をメタデータに変換 *)
      let metadata = Llvm.value_as_metadata const_value in
      Llvm.add_module_flag llmodule FlagBehavior.Override bridge_flag_name
        metadata

let optional_field key = function
  | None -> []
  | Some value ->
      let trimmed = String.trim value in
      if String.equal trimmed "" then [] else [ (key, trimmed) ]

let span_fields (span : Ast.span) =
  let open Ast in
  [
    ("bridge.source_span.start", string_of_int span.start);
    ("bridge.source_span.end", string_of_int span.end_);
  ]

let ownership_label ownership = Ffi_contract.string_of_ownership_kind ownership
let abi_label abi = Ffi_contract.string_of_abi_kind abi

let metadata_fields index (plan : stub_plan) =
  let base_fields =
    [
      ("bridge.stub_index", string_of_int (index + 1));
      ("bridge.extern_name", plan.contract.extern_name);
      ("bridge.stub_symbol", Ffi_stub_builder.stub_symbol_name ~index plan);
      ("bridge.thunk_symbol", Ffi_stub_builder.thunk_symbol_name ~index plan);
      ("bridge.target", plan.target_triple);
      ("bridge.callconv", plan.calling_convention);
      ("bridge.abi", abi_label plan.abi);
      ("bridge.ownership", ownership_label plan.ownership);
    ]
    @ span_fields plan.contract.source_span
  in
  let return_fields =
    let ownership = ownership_label plan.ownership in
    let common =
      [
        ("bridge.return.ownership", ownership);
        ("bridge.return.wrap", "wrap_foreign_ptr");
      ]
    in
    let extras =
      match plan.ownership with
      | Ffi_contract.OwnershipBorrowed ->
          [
            ("bridge.return.release_handler", "none");
            ("bridge.return.rc_adjustment", "none");
          ]
      | Ffi_contract.OwnershipTransferred ->
          [
            ("bridge.return.release_handler", "dec_ref");
            ("bridge.return.rc_adjustment", "dec_ref");
          ]
      | Ffi_contract.OwnershipReference ->
          [
            ("bridge.return.release_handler", "none");
            ("bridge.return.rc_adjustment", "reference");
          ]
      | Ffi_contract.OwnershipManaged label ->
          let trimmed = String.trim label in
          [
            ("bridge.return.release_handler", Printf.sprintf "managed:%s" trimmed);
            ("bridge.return.rc_adjustment", "managed");
          ]
      | Ffi_contract.OwnershipUnspecified ->
          [
            ("bridge.return.release_handler", "unknown");
            ("bridge.return.rc_adjustment", "unknown");
          ]
    in
    common @ extras
  in
  let base_fields = base_fields @ return_fields in
  let optional_fields =
    optional_field "bridge.block_target" plan.contract.block_target
    @ optional_field "bridge.extern_symbol"
        plan.contract.metadata.extern_link_name
    @ optional_field "bridge.metadata.target"
        plan.contract.metadata.extern_target
  in
  let seen = Hashtbl.create 16 in
  let add_field acc (key, value) =
    let sanitized = sanitize_value value in
    if String.equal sanitized "" then acc
    else if Hashtbl.mem seen key then acc
    else (
      Hashtbl.add seen key ();
      acc @ [ (key, sanitized) ])
  in
  let acc = List.fold_left add_field [] base_fields in
  let acc = List.fold_left add_field acc optional_fields in
  List.fold_left add_field acc plan.audit_tags

let mdnode_of_fields llctx fields =
  let operands =
    fields
    |> List.map (fun (key, value) ->
           let payload = Printf.sprintf "%s=%s" key value in
           Llvm.mdstring llctx payload)
    |> Array.of_list
  in
  Llvm.mdnode llctx operands

let attach_stub_plan llctx llmodule index plan =
  let fields = metadata_fields index plan in
  match fields with
  | [] -> ()
  | _ ->
      let node = mdnode_of_fields llctx fields in
      Llvm.add_named_metadata_operand llmodule named_metadata node

let attach_stub_plans llctx llmodule plans =
  match plans with
  | [] -> ()
  | _ ->
      ensure_bridge_flag llctx llmodule;
      List.iteri (attach_stub_plan llctx llmodule) plans
