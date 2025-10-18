(* Ffi_contract — FFI 契約検証のスケルトン
 *
 * Phase 2-3 で導入する FFI ブリッジ検証のための基本データ型と
 * JSON 変換ヘルパーを定義する。現時点では Typer から診断・監査に
 * 共通形式でメタデータを渡すための骨組みのみを提供する。
 *)

open Ast
module Json = Yojson.Basic

type ownership_kind =
  | OwnershipBorrowed
  | OwnershipTransferred
  | OwnershipReference
  | OwnershipManaged of string
  | OwnershipUnspecified

type abi_kind =
  | AbiSystemV
  | AbiMsvc
  | AbiAAPCS64
  | AbiCustom of string
  | AbiUnspecified

type bridge_contract = {
  extern_name : string;
  source_span : span;
  block_target : string option;
  metadata : extern_metadata;
}

type contract_issue_kind =
  | MissingTarget
  | TargetConflict of {
      block_target : string option;
      item_target : string option;
    }
  | UnsupportedAbi of string option
  | OwnershipMissing
  | OwnershipUnsupported of string
  | OwnershipConflict of { expected : string; actual : string }

type contract_issue = {
  issue : contract_issue_kind;
  contract : bridge_contract;
}

let trimmed_option (value : string option) =
  match value with
  | Some raw ->
      let trimmed = String.trim raw in
      if String.equal trimmed "" then None else Some trimmed
  | None -> None

let string_of_ownership_kind = function
  | OwnershipBorrowed -> "borrowed"
  | OwnershipTransferred -> "transferred"
  | OwnershipReference -> "reference"
  | OwnershipManaged label -> Printf.sprintf "custom:%s" (String.trim label)
  | OwnershipUnspecified -> "unspecified"

let string_of_abi_kind = function
  | AbiSystemV -> "system_v"
  | AbiMsvc -> "msvc"
  | AbiAAPCS64 -> "darwin_aapcs64"
  | AbiCustom label -> Printf.sprintf "custom:%s" (String.trim label)
  | AbiUnspecified -> "unspecified"

let normalize_identifier value = value |> String.trim |> String.lowercase_ascii

let ownership_kind_of_metadata (metadata : extern_metadata) : ownership_kind =
  match metadata.extern_ownership with
  | None -> OwnershipUnspecified
  | Some value -> (
      match normalize_identifier value with
      | "" -> OwnershipUnspecified
      | "borrowed" -> OwnershipBorrowed
      | "transferred" -> OwnershipTransferred
      | "reference" -> OwnershipReference
      | other -> OwnershipManaged other)

let abi_kind_of_metadata (metadata : extern_metadata) : abi_kind =
  match metadata.extern_calling_convention with
  | None -> AbiUnspecified
  | Some value -> (
      match normalize_identifier value with
      | "" -> AbiUnspecified
      | "ccc" | "system_v" -> AbiSystemV
      | "msvc" -> AbiMsvc
      | "aapcs64" | "darwin_aapcs64" -> AbiAAPCS64
      | other -> AbiCustom other)

let span_to_json (span : span) : Json.t =
  `Assoc [ ("start", `Int span.start); ("end", `Int span.end_) ]

let json_of_option (encoder : 'a -> Json.t) (value : 'a option) : Json.t =
  match value with Some v -> encoder v | None -> `Null

let json_of_string_option (value : string option) : Json.t =
  json_of_option (fun s -> `String s) value

let json_of_invalid_attribute (attr : extern_invalid_attribute) : Json.t =
  let reason =
    match attr.extern_reason with
    | ExternAttrUnknownKey key ->
        `Assoc [ ("kind", `String "unknown_key"); ("key", `String key) ]
    | ExternAttrMissingStringValue key ->
        `Assoc [ ("kind", `String "missing_value"); ("key", `String key) ]
    | ExternAttrDuplicateKey key ->
        `Assoc [ ("kind", `String "duplicate_key"); ("key", `String key) ]
  in
  `Assoc
    [
      ("name", `String attr.extern_attr.attr_name.name);
      ("span", span_to_json attr.extern_attr_span);
      ("reason", reason);
    ]

let extern_metadata_to_json (metadata : extern_metadata) : Json.t =
  `Assoc
    [
      ("target", json_of_string_option metadata.extern_target);
      ( "calling_convention",
        json_of_string_option metadata.extern_calling_convention );
      ("link_name", json_of_string_option metadata.extern_link_name);
      ("ownership", json_of_string_option metadata.extern_ownership);
      ( "invalid_attributes",
        `List
          (List.map json_of_invalid_attribute metadata.extern_invalid_attributes)
      );
    ]

let bridge_contract ?block_target ~extern_name ~source_span ~metadata () :
    bridge_contract =
  { extern_name; source_span; block_target; metadata }

let contract_to_audit_json (contract : bridge_contract) : Json.t =
  let ownership =
    match ownership_kind_of_metadata contract.metadata with
    | OwnershipBorrowed -> "borrowed"
    | OwnershipTransferred -> "transferred"
    | OwnershipReference -> "reference"
    | OwnershipManaged label -> Printf.sprintf "custom:%s" label
    | OwnershipUnspecified -> "unspecified"
  in
  let abi =
    match abi_kind_of_metadata contract.metadata with
    | AbiSystemV -> "system_v"
    | AbiMsvc -> "msvc"
    | AbiAAPCS64 -> "aapcs64"
    | AbiCustom label -> Printf.sprintf "custom:%s" label
    | AbiUnspecified -> "unspecified"
  in
  `Assoc
    [
      ("extern_name", `String contract.extern_name);
      ("source_span", span_to_json contract.source_span);
      ("block_target", json_of_string_option contract.block_target);
      ("metadata", extern_metadata_to_json contract.metadata);
      ("ownership_kind", `String ownership);
      ("abi_kind", `String abi);
    ]

let arch_of_target (target : string) : string option =
  match String.split_on_char '-' target with
  | arch :: _ when String.trim arch <> "" -> Some arch
  | _ -> None

let contains_substring haystack needle =
  let haystack = String.lowercase_ascii haystack in
  let needle = String.lowercase_ascii needle in
  let h_len = String.length haystack in
  let n_len = String.length needle in
  let rec search idx =
    if idx + n_len > h_len then false
    else if String.sub haystack idx n_len = needle then true
    else search (idx + 1)
  in
  if n_len = 0 then false else search 0

let expected_abi_for_target (target : string option) : abi_kind option =
  match target with
  | None -> None
  | Some value ->
      let lower = String.lowercase_ascii value in
      if contains_substring lower "windows" || contains_substring lower "msvc"
      then Some AbiMsvc
      else if
        contains_substring lower "darwin"
        || contains_substring lower "apple"
        || contains_substring lower "macos"
      then Some AbiAAPCS64
      else Some AbiSystemV

let ownership_supported = function
  | OwnershipBorrowed | OwnershipTransferred | OwnershipReference -> true
  | OwnershipManaged _ | OwnershipUnspecified -> false

let abi_supported = function
  | AbiSystemV | AbiMsvc | AbiAAPCS64 -> true
  | AbiCustom _ | AbiUnspecified -> false

let supported_ownership_labels = [ "borrowed"; "transferred"; "reference" ]
let supported_abi_labels = [ "system_v"; "msvc"; "darwin_aapcs64" ]

type normalized_contract = {
  contract : bridge_contract;
  target : string option;
  arch : string option;
  abi_kind : abi_kind;
  abi_label : string;
  abi_raw : string option;
  expected_abi : abi_kind option;
  ownership_kind : ownership_kind;
  ownership_label : string;
  ownership_raw : string option;
  extern_symbol : string option;
  link_name : string option;
}

let normalize_contract (contract : bridge_contract) : normalized_contract =
  let explicit_target = trimmed_option contract.metadata.extern_target in
  let target =
    match explicit_target with
    | Some value -> Some value
    | None -> trimmed_option contract.block_target
  in
  let arch = match target with Some t -> arch_of_target t | None -> None in
  let abi_kind = abi_kind_of_metadata contract.metadata in
  let abi_label = string_of_abi_kind abi_kind in
  let ownership_kind = ownership_kind_of_metadata contract.metadata in
  let ownership_label = string_of_ownership_kind ownership_kind in
  let link_name = trimmed_option contract.metadata.extern_link_name in
  let extern_symbol = link_name in
  {
    contract;
    target;
    arch;
    abi_kind;
    abi_label;
    abi_raw = trimmed_option contract.metadata.extern_calling_convention;
    expected_abi = expected_abi_for_target target;
    ownership_kind;
    ownership_label;
    ownership_raw = trimmed_option contract.metadata.extern_ownership;
    extern_symbol;
    link_name;
  }

let bridge_json_of_normalized ?status (normalized : normalized_contract) :
    Json.t =
  let option_string value = match value with Some v -> `String v | None -> `Null in
  let fields =
    [
      ("extern_name", `String normalized.contract.extern_name);
      ("target", option_string normalized.target);
      ("arch", option_string normalized.arch);
      ("abi", `String normalized.abi_label);
      ("ownership", `String normalized.ownership_label);
      ("extern_symbol", option_string normalized.extern_symbol);
    ]
  in
  let fields =
    match normalized.expected_abi with
    | Some expected ->
        ("expected_abi", `String (string_of_abi_kind expected)) :: fields
    | None -> fields
  in
  let fields =
    match normalized.link_name with
    | Some link -> ("link_name", `String link) :: fields
    | None -> fields
  in
  let fields =
    match normalized.contract.block_target with
    | Some block -> ("block_target", `String block) :: fields
    | None -> fields
  in
  let invalid_attrs = normalized.contract.metadata.extern_invalid_attributes in
  let fields =
    if invalid_attrs = [] then fields
    else
      ("invalid_attributes", `List (List.map json_of_invalid_attribute invalid_attrs))
      :: fields
  in
  let fields =
    match status with
    | Some value -> ("status", `String value) :: fields
    | None -> fields
  in
  `Assoc fields

let bridge_audit_metadata ?(status = "ok")
    (normalized : normalized_contract) : Json.t =
  `Assoc
    [
      ("bridge", bridge_json_of_normalized ~status normalized);
      ("source_span", span_to_json normalized.contract.source_span);
    ]
