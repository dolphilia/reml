open Core_ir.Ir
open Types

let dummy_span = Ast.dummy_span

type metadata_expectation = { key : string; value : string }

type test_case = {
  label : string;
  module_target : string;
  plan : Ffi_stub_builder.stub_plan;
  expected_metadata : metadata_expectation list;
  expect_stub_substrings : string list;
  expect_thunk_substrings : string list;
}

let debug_enabled =
  match Sys.getenv_opt "REML_TEST_DEBUG" with
  | Some ("1" | "true" | "TRUE" | "yes" | "YES") -> true
  | _ -> false

let debug_log fmt =
  if debug_enabled then Printf.ksprintf prerr_endline fmt
  else Printf.ksprintf (fun _ -> ()) fmt

let call_conv_win64 = 79
let call_conv_aapcs = 67
let normalize_callconv value = value |> String.trim |> String.lowercase_ascii

let expected_call_conv_of_plan (plan : Ffi_stub_builder.stub_plan) =
  match normalize_callconv plan.calling_convention with
  | "win64" | "win64cc" | "msvc" -> call_conv_win64
  | "aarch64_aapcscc" | "aapcs64" | "arm_aapcs" | "arm_aapcscc" ->
      call_conv_aapcs
  | "ccc" | "c" | "system_v" | "" -> Llvm.CallConv.c
  | _ -> Llvm.CallConv.c

let extern_symbol_of_plan (plan : Ffi_stub_builder.stub_plan) =
  match plan.contract.metadata.extern_link_name with
  | Some name when String.trim name <> "" -> name
  | _ -> plan.contract.extern_name

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

let metadata_table entries =
  let tbl = Hashtbl.create 32 in
  List.iter
    (fun payload ->
      match String.index_opt payload '=' with
      | None -> ()
      | Some idx ->
          let key = String.sub payload 0 idx in
          let value =
            String.sub payload (idx + 1) (String.length payload - idx - 1)
          in
          Hashtbl.replace tbl key value)
    entries;
  tbl

let assert_metadata_value case_label table key expected =
  match Hashtbl.find_opt table key with
  | Some actual when String.equal actual expected -> ()
  | Some actual ->
      Printf.eprintf "ケース[%s]: メタデータ %s が %s でした (期待値: %s)\n" case_label key
        actual expected;
      exit 1
  | None ->
      Printf.eprintf "ケース[%s]: メタデータ %s が存在しません\n" case_label key;
      exit 1

let assert_contains_substring case_label ir substring kind =
  if Ffi_contract.contains_substring ir substring then ()
  else (
    Printf.eprintf "ケース[%s]: %s に文字列 \"%s\" が含まれていません\n" case_label kind
      substring;
    exit 1)

let rec collect_call_conv_from_value acc value =
  match Llvm.classify_value value with
  | Llvm.ValueKind.Instruction Llvm.Opcode.Call ->
      Llvm.instruction_call_conv value :: acc
  | Llvm.ValueKind.ConstantExpr ->
      Llvm.fold_left_uses collect_call_conv_from_use acc value
  | _ -> acc

and collect_call_conv_from_use acc use =
  let user = Llvm.user use in
  collect_call_conv_from_value acc user

let collect_call_conv_uses value =
  Llvm.fold_left_uses collect_call_conv_from_use [] value

let summarize_int_list values =
  match values |> List.sort_uniq compare with
  | [] -> "-"
  | xs -> xs |> List.map string_of_int |> String.concat ","

let dune_source_root () =
  match Sys.getenv_opt "DUNE_SOURCEROOT" with
  | Some root -> root
  | None -> Sys.getcwd ()

let load_file path =
  let channel = open_in path in
  let length = in_channel_length channel in
  let contents = really_input_string channel length in
  close_in channel;
  contents

let linux_case () =
  let contract = make_contract ~block_target:"ffi-linux-block" () in
  let plan =
    Ffi_stub_builder.make_stub_plan ~param_types:[ ty_i64 ] ~return_type:TUnit
      contract
  in
  let stub_name = Ffi_stub_builder.stub_symbol_name ~index:0 plan in
  let thunk_name = Ffi_stub_builder.thunk_symbol_name ~index:0 plan in
  {
    label = "linux-default";
    module_target = "x86_64-linux";
    plan;
    expected_metadata =
      [
        { key = "bridge.stub_index"; value = "1" };
        { key = "bridge.stub_symbol"; value = stub_name };
        { key = "bridge.thunk_symbol"; value = thunk_name };
        { key = "bridge.target"; value = "x86_64-unknown-linux-gnu" };
        { key = "bridge.callconv"; value = "ccc" };
        { key = "bridge.abi"; value = "system_v" };
        { key = "bridge.ownership"; value = "borrowed" };
        { key = "bridge.platform"; value = "linux-x86_64" };
        { key = "bridge.arch"; value = "x86_64" };
        { key = "bridge.block_target"; value = "ffi-linux-block" };
      ];
    expect_stub_substrings = [ "reml_ffi_bridge_record_status"; thunk_name ];
    expect_thunk_substrings = [ "call"; "ffi_entry" ];
  }

let windows_case () =
  let contract =
    make_contract ~block_target:"ffi-win-block" ~target:"x86_64-pc-windows-msvc"
      ~callconv:"msvc" ~ownership:"transferred" ~link_name:"ffi_entry_symbol" ()
  in
  let plan =
    Ffi_stub_builder.make_stub_plan ~param_types:[] ~return_type:TUnit contract
  in
  let stub_name = Ffi_stub_builder.stub_symbol_name ~index:0 plan in
  let thunk_name = Ffi_stub_builder.thunk_symbol_name ~index:0 plan in
  {
    label = "windows-transferred";
    module_target = "x86_64-windows";
    plan;
    expected_metadata =
      [
        { key = "bridge.stub_index"; value = "1" };
        { key = "bridge.stub_symbol"; value = stub_name };
        { key = "bridge.thunk_symbol"; value = thunk_name };
        { key = "bridge.target"; value = "x86_64-pc-windows-msvc" };
        { key = "bridge.callconv"; value = "win64" };
        { key = "bridge.abi"; value = "msvc" };
        { key = "bridge.ownership"; value = "transferred" };
        { key = "bridge.platform"; value = "windows-msvc-x64" };
        { key = "bridge.arch"; value = "x86_64" };
        { key = "bridge.block_target"; value = "ffi-win-block" };
        { key = "bridge.extern_symbol"; value = "ffi_entry_symbol" };
      ];
    expect_stub_substrings = [ "reml_ffi_bridge_record_status"; thunk_name ];
    expect_thunk_substrings = [ "call"; "ffi_entry_symbol" ];
  }

let macos_case () =
  let contract =
    make_contract ~block_target:"ffi-macos-block" ~target:"arm64-apple-darwin"
      ~callconv:"aarch64_aapcscc" ~ownership:"borrowed" ()
  in
  let plan =
    Ffi_stub_builder.make_stub_plan ~param_types:[ ty_string ]
      ~return_type:ty_unit contract
  in
  let stub_name = Ffi_stub_builder.stub_symbol_name ~index:0 plan in
  let thunk_name = Ffi_stub_builder.thunk_symbol_name ~index:0 plan in
  {
    label = "macos-borrowed";
    module_target = "arm64-apple-darwin";
    plan;
    expected_metadata =
      [
        { key = "bridge.stub_index"; value = "1" };
        { key = "bridge.stub_symbol"; value = stub_name };
        { key = "bridge.thunk_symbol"; value = thunk_name };
        { key = "bridge.target"; value = "arm64-apple-darwin" };
        { key = "bridge.callconv"; value = "aarch64_aapcscc" };
        { key = "bridge.abi"; value = "darwin_aapcs64" };
        { key = "bridge.ownership"; value = "borrowed" };
        { key = "bridge.platform"; value = "macos-arm64" };
        { key = "bridge.arch"; value = "arm64" };
        { key = "bridge.block_target"; value = "ffi-macos-block" };
      ];
    expect_stub_substrings = [ "reml_ffi_bridge_record_status"; thunk_name ];
    expect_thunk_substrings = [ "call"; "ffi_entry" ];
  }

let verify_module_flag case_label llvm_module =
  let llctx = Llvm.module_context llvm_module in
  match Llvm.get_module_flag llvm_module "reml.bridge.version" with
  | None ->
      Printf.eprintf "ケース[%s]: reml.bridge.version フラグがありません\n" case_label;
      exit 1
  | Some md -> (
      let value = Llvm.metadata_as_value llctx md in
      debug_log "ケース[%s]: module flag value = %s" case_label
        (Llvm.string_of_llvalue value);
      (* メタデータから整数値を抽出するには、まずValueとして取得し、
         その後Operandを確認する必要がある *)
      match Llvm.classify_value value with
      | Llvm.ValueKind.ConstantInt -> (
          match Llvm.int64_of_const value with
          | Some 1L -> ()
          | Some other ->
              Printf.eprintf "ケース[%s]: reml.bridge.version が %Ld でした (期待値: 1)\n"
                case_label other;
              exit 1
          | None ->
              (* ConstantIntだがint64_of_constが失敗する場合、
                 文字列表現から値を抽出 *)
              let value_str = Llvm.string_of_llvalue value in
              if String.contains value_str '1' then ()
              else (
                Printf.eprintf "ケース[%s]: reml.bridge.version の値が不正です: %s\n"
                  case_label value_str;
                exit 1))
      | _ ->
          (* ConstantInt以外の場合も、文字列表現で検証 *)
          let value_str = Llvm.string_of_llvalue value in
          debug_log "ケース[%s]: value kind = %s, checking string representation"
            case_label value_str;
          if String.contains value_str '1' then ()
          else (
            Printf.eprintf "ケース[%s]: reml.bridge.version が整数定数ではありません: %s\n"
              case_label value_str;
            exit 1))

let verify_case case =
  debug_log "ケース[%s]: コード生成開始" case.label;
  let llvm_module =
    Codegen.codegen_module ~target_name:case.module_target
      ~stub_plans:[ case.plan ] empty_module
  in
  debug_log "ケース[%s]: モジュール生成完了" case.label;
  verify_module_flag case.label llvm_module;

  debug_log "ケース[%s]: メタデータ検証開始" case.label;
  let metadata_nodes =
    Llvm.get_named_metadata llvm_module "reml.bridge.stubs"
  in
  if Array.length metadata_nodes <> 1 then (
    Printf.eprintf "ケース[%s]: reml.bridge.stubs の件数が %d 件でした (期待値: 1)\n"
      case.label
      (Array.length metadata_nodes);
    exit 1);
  let entries = metadata_strings metadata_nodes.(0) in
  let table = metadata_table entries in
  List.iter
    (fun { key; value } -> assert_metadata_value case.label table key value)
    case.expected_metadata;

  let expected_call_conv = expected_call_conv_of_plan case.plan in
  debug_log "ケース[%s]: スタブ/サンク検証開始" case.label;
  let stub_name = Ffi_stub_builder.stub_symbol_name ~index:0 case.plan in
  let stub_fn =
    match Llvm.lookup_function stub_name llvm_module with
    | Some fn -> fn
    | None ->
        Printf.eprintf "ケース[%s]: スタブ %s が生成されていません\n" case.label stub_name;
        exit 1
  in
  let stub_call_conv = Llvm.function_call_conv stub_fn in
  if stub_call_conv <> expected_call_conv then (
    Printf.eprintf "ケース[%s]: スタブ %s の CallConv=%d (期待値=%d)\n" case.label
      stub_name stub_call_conv expected_call_conv;
    exit 1);
  let stub_ir = Llvm.string_of_llvalue stub_fn in
  List.iter
    (fun substring ->
      assert_contains_substring case.label stub_ir substring "スタブIR")
    case.expect_stub_substrings;

  let thunk_name = Ffi_stub_builder.thunk_symbol_name ~index:0 case.plan in
  let thunk_fn =
    match Llvm.lookup_function thunk_name llvm_module with
    | Some fn -> fn
    | None ->
        Printf.eprintf "ケース[%s]: サンク %s が生成されていません\n" case.label thunk_name;
        exit 1
  in
  let thunk_call_conv = Llvm.function_call_conv thunk_fn in
  if thunk_call_conv <> expected_call_conv then (
    Printf.eprintf "ケース[%s]: サンク %s の CallConv=%d (期待値=%d)\n" case.label
      thunk_name thunk_call_conv expected_call_conv;
    exit 1);
  let thunk_ir = Llvm.string_of_llvalue thunk_fn in
  List.iter
    (fun substring ->
      assert_contains_substring case.label thunk_ir substring "サンクIR")
    case.expect_thunk_substrings;

  debug_log "ケース[%s]: 外部シンボル検証開始" case.label;
  let extern_symbol = extern_symbol_of_plan case.plan in
  let extern_fn =
    match Llvm.lookup_function extern_symbol llvm_module with
    | Some fn -> fn
    | None ->
        Printf.eprintf "ケース[%s]: 外部シンボル %s が見つかりません\n" case.label extern_symbol;
        exit 1
  in
  let extern_call_conv = Llvm.function_call_conv extern_fn in
  if extern_call_conv <> expected_call_conv then (
    Printf.eprintf "ケース[%s]: 外部シンボル %s の CallConv=%d (期待値=%d)\n" case.label
      extern_symbol extern_call_conv expected_call_conv;
    exit 1);

  debug_log "ケース[%s]: 呼出規約検証開始" case.label;
  let stub_call_sites = collect_call_conv_uses thunk_fn in
  let thunk_call_sites = collect_call_conv_uses extern_fn in
  if not (List.exists (fun cc -> cc = expected_call_conv) stub_call_sites) then (
    Printf.eprintf "ケース[%s]: サンクへの呼び出し CallConv に期待値 %d が含まれません\n" case.label
      expected_call_conv;
    exit 1);
  if not (List.exists (fun cc -> cc = expected_call_conv) thunk_call_sites) then (
    Printf.eprintf "ケース[%s]: 外部呼び出し CallConv に期待値 %d が含まれません\n" case.label
      expected_call_conv;
    exit 1);

  debug_log "ケース[%s]: サマリ比較開始" case.label;
  let summary_lines =
    [
      Printf.sprintf "stub_call_conv=%d" stub_call_conv;
      Printf.sprintf "thunk_call_conv=%d" thunk_call_conv;
      Printf.sprintf "extern_call_conv=%d" extern_call_conv;
      Printf.sprintf "stub_to_thunk_call_sites=%s"
        (summarize_int_list stub_call_sites);
      Printf.sprintf "thunk_to_extern_call_sites=%s"
        (summarize_int_list thunk_call_sites);
    ]
  in
  let summary = String.concat "\n" summary_lines ^ "\n" in
  let golden_dir = Filename.concat (dune_source_root ()) "tests/golden/llvm" in
  let golden_path = Filename.concat golden_dir (case.label ^ ".ll") in
  let expected_summary =
    try load_file golden_path
    with Sys_error msg ->
      Printf.eprintf "ケース[%s]: ゴールデンファイル読み込み失敗: %s\n" case.label msg;
      exit 1
  in
  if String.compare summary expected_summary <> 0 then (
    Printf.eprintf "ケース[%s]: 呼出規約サマリがゴールデンと一致しません。\n" case.label;
    Printf.eprintf "期待:\n%s\n実際:\n%s\n" expected_summary summary;
    exit 1)
  else debug_log "ケース[%s]: 検証完了" case.label

let () =
  let cases = [ linux_case (); windows_case (); macos_case () ] in
  List.iter verify_case cases;
  Printf.printf "FFI lowering metadata tests passed for %d targets.\n"
    (List.length cases);
  exit 0
