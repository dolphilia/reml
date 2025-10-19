(* ffi_stub_builder.ml — FFI ブリッジ用スタブ生成スケルトン
 *
 * Phase 2-3 の計画（docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md）
 * に基づき、ターゲット別のスタブ生成に必要な最小限のデータ構造と
 * 判定ロジックを提供する。現時点では Typer が正規化した契約情報
 * (`Ffi_contract.bridge_contract`) を受け取り、コード生成と監査ログの
 * 双方で共有できるプラン情報を構築することを目的とする。
 *
 * このモジュールはまだ LLVM への lowering や C スタブの実生成は行わない。
 * 代わりに、ターゲットトリプル・呼出規約・所有権・ABI 情報を正規化し、
 * 監査ログで必要となるキー (`bridge.platform` など) を抽出する。
 *)

open Ffi_contract
open Types
open Ast

(* ========== 型定義 ========== *)

type platform = LinuxX86_64 | WindowsX64 | MacOSArm64

type stub_template = {
  platform : platform;
  default_target : string;
  default_call_conv : string;
  default_abi : abi_kind;
  audit_platform : string;
}

type stub_plan = {
  template : stub_template;
  target_triple : string;
  calling_convention : string;
  ownership : ownership_kind;
  abi : abi_kind;
  audit_tags : (string * string) list;
  param_types : ty list;
  return_type : ty;
  contract : bridge_contract;
}

(* ========== 内部ユーティリティ ========== *)

let linux_template =
  {
    platform = LinuxX86_64;
    default_target = "x86_64-unknown-linux-gnu";
    default_call_conv = "ccc";
    default_abi = AbiSystemV;
    audit_platform = "linux-x86_64";
  }

let windows_template =
  {
    platform = WindowsX64;
    default_target = "x86_64-pc-windows-msvc";
    default_call_conv = "win64";
    default_abi = AbiMsvc;
    audit_platform = "windows-msvc-x64";
  }

let macos_template =
  {
    platform = MacOSArm64;
    default_target = "arm64-apple-darwin";
    default_call_conv = "aarch64_aapcscc";
    default_abi = AbiAAPCS64;
    audit_platform = "macos-arm64";
  }

let known_templates = [ linux_template; windows_template; macos_template ]

let normalize_target (value : string) =
  value |> String.trim |> String.lowercase_ascii

let template_for_target target =
  let target = normalize_target target in
  let contains needle = contains_substring target needle in
  if contains "windows" || contains "msvc" then windows_template
  else if
    contains "darwin" || contains "apple" || contains "macos"
    || contains "aarch64"
  then macos_template
  else linux_template

let sanitize_option (value : string option) : string option =
  match value with
  | Some raw ->
      let trimmed = String.trim raw in
      if String.equal trimmed "" then None else Some trimmed
  | None -> None

let fallback_arch = function
  | LinuxX86_64 | WindowsX64 -> "x86_64"
  | MacOSArm64 -> "arm64"

let resolve_target (contract : bridge_contract) =
  let metadata_target = sanitize_option contract.metadata.extern_target in
  let block_target = sanitize_option contract.block_target in
  (* テンプレート選択用の候補値: extern_target -> block_target -> デフォルト *)
  let candidate =
    match metadata_target with
    | Some value -> value
    | None -> (
        match block_target with
        | Some value -> value
        | None -> linux_template.default_target)
  in
  let template = template_for_target candidate in
  (* 実際のターゲットトリプル: extern_target -> テンプレートのデフォルト
     block_targetはターゲット選択のヒントであり、ターゲットトリプルではない *)
  let effective_target =
    match metadata_target with
    | Some value -> value
    | None -> template.default_target
  in
  (template, effective_target)

let normalize_call_conv template metadata =
  match sanitize_option metadata.extern_calling_convention with
  | Some value -> (
      (* 呼出規約の別名を正規化 *)
      let normalized = String.lowercase_ascii value in
      match normalized with
      | "msvc" | "win64cc" -> "win64"
      | "aapcs64" | "arm_aapcs" | "arm_aapcscc" | "darwin" -> "aarch64_aapcscc"
      | "c" | "system_v" -> "ccc"
      | other -> other)
  | None -> template.default_call_conv

let normalize_ownership metadata =
  match ownership_kind_of_metadata metadata with
  | OwnershipUnspecified -> OwnershipBorrowed
  | ownership -> ownership

let normalize_abi template metadata =
  match abi_kind_of_metadata metadata with
  | AbiUnspecified -> template.default_abi
  | abi -> abi

let audit_tags_of_plan template target call_conv ownership abi =
  let abi_str = string_of_abi_kind abi in
  let ownership_str = string_of_ownership_kind ownership in
  let arch =
    match arch_of_target target with
    | Some v -> v
    | None -> fallback_arch template.platform
  in
  [
    ("bridge.platform", template.audit_platform);
    ("bridge.target", target);
    ("bridge.arch", arch);
    ("bridge.callconv", call_conv);
    ("bridge.abi", abi_str);
    ("bridge.ownership", ownership_str);
  ]

(* ========== 公開 API ========== *)

let make_stub_plan ~(param_types : ty list) ~(return_type : ty)
    (contract : bridge_contract) : stub_plan =
  let template, target = resolve_target contract in
  let call_conv = normalize_call_conv template contract.metadata in
  let ownership = normalize_ownership contract.metadata in
  let abi = normalize_abi template contract.metadata in
  let audit_tags = audit_tags_of_plan template target call_conv ownership abi in
  {
    template;
    target_triple = target;
    calling_convention = call_conv;
    ownership;
    abi;
    audit_tags;
    param_types;
    return_type;
    contract;
  }

let sanitize_symbol_component value =
  let buffer = Buffer.create (String.length value) in
  String.iter
    (fun ch ->
      let ch = Char.lowercase_ascii ch in
      if (ch >= 'a' && ch <= 'z') || (ch >= '0' && ch <= '9') || ch = '_' then
        Buffer.add_char buffer ch
      else Buffer.add_char buffer '_')
    value;
  let sanitized = Buffer.contents buffer in
  if String.equal sanitized "" then "ffi" else sanitized

let stub_symbol_name ~index (plan : stub_plan) =
  let base = sanitize_symbol_component plan.contract.extern_name in
  Printf.sprintf "__reml_stub_%s_%d" base (index + 1)

let thunk_symbol_name ~index (plan : stub_plan) =
  let source =
    match plan.contract.metadata.extern_link_name with
    | Some link -> sanitize_symbol_component link
    | None -> sanitize_symbol_component plan.contract.extern_name
  in
  Printf.sprintf "__reml_thunk_%s_%d" source (index + 1)
