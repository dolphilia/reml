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
