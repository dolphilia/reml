(* test_ffi_stub_builder.ml — FFI スタブプランの初期検証 *)

open Ast
open Ffi_contract
open Ffi_stub_builder

let test_count = ref 0
let pass_count = ref 0

let record_result name ok detail =
  test_count := !test_count + 1;
  if ok then (
    pass_count := !pass_count + 1;
    Printf.printf "✓ %s\n" name)
  else (
    Printf.printf "✗ %s\n" name;
    (match detail with Some msg -> Printf.printf "    %s\n" msg | None -> ()))

let assert_string name expected actual =
  let ok = String.equal expected actual in
  let detail =
    if ok then None
    else Some (Printf.sprintf "期待値: %s / 実際値: %s" expected actual)
  in
  record_result name ok detail

let assert_abi name expected actual =
  record_result name (expected = actual)
    (if expected = actual then None
     else
       Some
         (Printf.sprintf "期待値: %s / 実際値: %s"
            (string_of_abi_kind expected)
            (string_of_abi_kind actual)))

let assert_ownership name expected actual =
  record_result name (expected = actual)
    (if expected = actual then None
     else
       Some
         (Printf.sprintf "期待値: %s / 実際値: %s"
            (string_of_ownership_kind expected)
            (string_of_ownership_kind actual)))

let lookup_audit key tags =
  let rec aux = function
    | [] -> None
    | (k, v) :: rest -> if String.equal k key then Some v else aux rest
  in
  aux tags

let assert_audit name plan key expected =
  match lookup_audit key plan.audit_tags with
  | Some value -> assert_string (name ^ " — " ^ key) expected value
  | None ->
      record_result
        (name ^ " — " ^ key)
        false
        (Some "監査タグが存在しません")

let dummy_span = Ast.dummy_span

let make_metadata ?target ?callconv ?ownership () =
  {
    extern_target = target;
    extern_calling_convention = callconv;
    extern_link_name = None;
    extern_ownership = ownership;
    extern_invalid_attributes = [];
  }

let make_contract ?block_target ?target ?callconv ?ownership () =
  let metadata = make_metadata ?target ?callconv ?ownership () in
  bridge_contract ?block_target ~extern_name:"ffi_entry" ~source_span:dummy_span
    ~metadata ()

let test_linux_defaults () =
  let contract = make_contract () in
  let plan = make_stub_plan contract in
  assert_string "Linux デフォルトターゲット"
    "x86_64-unknown-linux-gnu"
    plan.target_triple;
  assert_string "Linux デフォルト呼出規約" "ccc" plan.calling_convention;
  assert_abi "Linux 既定 ABI" AbiSystemV plan.abi;
  assert_ownership "Linux 所有権既定値" OwnershipBorrowed plan.ownership;
  assert_audit "Linux 監査" plan "bridge.platform" "linux-x86_64";
  assert_audit "Linux 監査" plan "bridge.arch" "x86_64";
  assert_audit "Linux 監査" plan "bridge.callconv" "ccc";
  assert_audit "Linux 監査" plan "bridge.abi" "system_v";
  assert_audit "Linux 監査" plan "bridge.ownership" "borrowed"

let test_windows_plan () =
  let contract =
    make_contract
      ~target:"x86_64-pc-windows-msvc"
      ~ownership:"transferred" ()
  in
  let plan = make_stub_plan contract in
  assert_string "Windows ターゲット"
    "x86_64-pc-windows-msvc"
    plan.target_triple;
  assert_string "Windows 呼出規約" "win64" plan.calling_convention;
  assert_abi "Windows ABI" AbiMsvc plan.abi;
  assert_ownership "Windows 所有権" OwnershipTransferred plan.ownership;
  assert_audit "Windows 監査" plan "bridge.platform" "windows-msvc-x64";
  assert_audit "Windows 監査" plan "bridge.arch" "x86_64";
  assert_audit "Windows 監査" plan "bridge.callconv" "win64";
  assert_audit "Windows 監査" plan "bridge.ownership" "transferred"

let test_macos_plan () =
  let contract =
    make_contract
      ~target:"arm64-apple-darwin"
      ~ownership:"borrowed" ()
  in
  let plan = make_stub_plan contract in
  assert_string "macOS ターゲット" "arm64-apple-darwin" plan.target_triple;
  assert_string "macOS 呼出規約" "aarch64_aapcscc" plan.calling_convention;
  assert_abi "macOS ABI" AbiAAPCS64 plan.abi;
  assert_ownership "macOS 所有権" OwnershipBorrowed plan.ownership;
  assert_audit "macOS 監査" plan "bridge.platform" "macos-arm64";
  assert_audit "macOS 監査" plan "bridge.arch" "arm64";
  assert_audit "macOS 監査" plan "bridge.callconv" "aarch64_aapcscc";
  assert_audit "macOS 監査" plan "bridge.abi" "darwin_aapcs64"

let () =
  Printf.printf "===============================\n";
  Printf.printf "FFI スタブプラン初期テスト\n";
  Printf.printf "===============================\n";
  test_linux_defaults ();
  test_windows_plan ();
  test_macos_plan ();
  Printf.printf "\n===============================\n";
  Printf.printf "テスト結果: %d/%d 成功\n" !pass_count !test_count;
  Printf.printf "===============================\n";
  if !pass_count = !test_count then exit 0 else exit 1
