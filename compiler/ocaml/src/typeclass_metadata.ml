open Types

module Json = Yojson.Basic

type resolution_state =
  | Resolved
  | Unresolved
  | Ambiguous
  | StageMismatch
  | UnresolvedTypeVar
  | Cyclic
  | Pending
  | Error of string

type stage_metadata = {
  stage_required :
    Constraint_solver.iterator_stage_requirement option;
      (** 要求される Stage （Iterator 系のみ） *)
  stage_actual : string option;
      (** Capability Registry が報告した Stage *)
  stage_capability : string option;
      (** 関連する Capability ID *)
  stage_provider : string option;
      (** Stage 情報を提供した主体（未実装の場合は None） *)
  stage_manifest_path : string option;
      (** Capability マニフェストのパス（未実装の場合は None） *)
  stage_iterator_kind : Constraint_solver.iterator_dict_kind option;
      (** Iterator 辞書の種別 *)
  stage_iterator_source : string option;
      (** Iterator のソース（型名など） *)
  stage_trace : Effect_profile.stage_trace option;
      (** Stage 判定のトレース情報 *)
}

type summary = {
  constraint_ : trait_constraint;
  span : Ast.span;
  resolution_state : resolution_state;
  dict_ref : Constraint_solver.dict_ref option;
  candidates : Constraint_solver.dict_ref list;
  pending : string list;
  generalized_typevars : string list;
  graph_export : string option;
  stage_info : stage_metadata option;
}

let make_summary ?dict_ref ?(candidates = []) ?(pending = [])
    ?(generalized_typevars = []) ?graph_export ?stage_info ~constraint_
    ~resolution_state () : summary =
  {
    constraint_;
    span = constraint_.constraint_span;
    resolution_state;
    dict_ref;
    candidates;
    pending;
    generalized_typevars;
    graph_export;
    stage_info;
  }

let resolution_state_to_string = function
  | Resolved -> "resolved"
  | Unresolved -> "unresolved"
  | Ambiguous -> "ambiguous"
  | StageMismatch -> "stage_mismatch"
  | UnresolvedTypeVar -> "unresolved_typevar"
  | Cyclic -> "cyclic"
  | Pending -> "pending"
  | Error _ -> "error"

let resolution_state_detail = function Error detail -> Some detail | _ -> None

let json_list_of_strings lst =
  `List (List.map (fun s -> `String s) lst)

let json_list_of_types tys =
  `List (List.map (fun ty -> `String (string_of_ty ty)) tys)

let string_of_stage_requirement
    (req : Constraint_solver.iterator_stage_requirement) =
  match req with
  | Constraint_solver.IteratorStageExact stage -> stage
  | Constraint_solver.IteratorStageAtLeast stage ->
      Printf.sprintf "at_least:%s" stage

let string_of_iterator_kind (kind : Constraint_solver.iterator_dict_kind) =
  match kind with
  | Constraint_solver.IteratorArrayLike -> "array_like"
  | Constraint_solver.IteratorCoreIter -> "core_iter"
  | Constraint_solver.IteratorOptionLike -> "option_like"
  | Constraint_solver.IteratorResultLike -> "result_like"
  | Constraint_solver.IteratorCustom name -> Printf.sprintf "custom:%s" name

let stage_json_of (stage : stage_metadata) =
  let as_json opt =
    match opt with
    | Some value when String.trim value <> "" -> `String value
    | _ -> `Null
  in
  let required_json =
    match stage.stage_required with
    | Some req -> `String (string_of_stage_requirement req)
    | None -> `Null
  in
  let iterator_kind_json =
    match stage.stage_iterator_kind with
    | Some kind -> `String (string_of_iterator_kind kind)
    | None -> `Null
  in
  let trace_json =
    match stage.stage_trace with
    | Some trace -> Effect_profile.stage_trace_to_json trace
    | None -> `Null
  in
  `Assoc
    [
      ("required", required_json);
      ("actual", as_json stage.stage_actual);
      ("capability", as_json stage.stage_capability);
      ("provider", as_json stage.stage_provider);
      ("manifest_path", as_json stage.stage_manifest_path);
      ("iterator_kind", iterator_kind_json);
      ("iterator_source", as_json stage.stage_iterator_source);
      ("trace", trace_json);
    ]

let stage_metadata_pairs (stage : stage_metadata) =
  let add_if_some acc key opt =
    match opt with
    | Some value ->
        let trimmed = String.trim value in
        if String.equal trimmed "" then acc
        else (key, `String trimmed) :: acc
    | None -> acc
  in
  let acc =
    match stage.stage_required with
    | Some req ->
        let value = string_of_stage_requirement req in
        [
          ("effect.stage.iterator.required", `String value);
          ("effect.stage.required", `String value);
        ]
    | None -> []
  in
  let acc =
    match stage.stage_iterator_kind with
    | Some kind ->
        ("effect.stage.iterator.kind", `String (string_of_iterator_kind kind))
        :: acc
    | None -> acc
  in
  let acc =
    match stage.stage_trace with
    | Some trace ->
        ("effect.stage_trace", Effect_profile.stage_trace_to_json trace) :: acc
    | None -> acc
  in
  let acc = add_if_some acc "effect.stage.iterator.actual" stage.stage_actual in
  let acc = add_if_some acc "effect.stage.actual" stage.stage_actual in
  let acc =
    add_if_some acc "effect.stage.iterator.capability" stage.stage_capability
  in
  let acc = add_if_some acc "effect.capability" stage.stage_capability in
  let acc = add_if_some acc "effect.provider" stage.stage_provider in
  let acc = add_if_some acc "effect.manifest_path" stage.stage_manifest_path in
  let acc =
    match stage.stage_iterator_source with
    | Some src ->
        let trimmed = String.trim src in
        if String.equal trimmed "" then acc
        else
          ("effect.stage.iterator.source_detail", `String trimmed)
          :: ("effect.stage.iterator.source", `String trimmed) :: acc
    | None -> acc
  in
  List.rev acc

let dictionary_json_of_ref (dict_ref : Constraint_solver.dict_ref) =
  let repr = Constraint_solver.string_of_dict_ref dict_ref in
  match dict_ref with
  | Constraint_solver.DictImplicit (trait, tys) ->
      let type_arg_strings = List.map string_of_ty tys in
      let json =
        `Assoc
          [
            ("kind", `String "implicit");
            ("identifier", `String trait);
            ("trait", `String trait);
            ("type_args", json_list_of_strings type_arg_strings);
            ("repr", `String repr);
          ]
      in
      let flat =
        [
          ("typeclass.dictionary.kind", `String "implicit");
          ("typeclass.dictionary.identifier", `String trait);
          ("typeclass.dictionary.trait", `String trait);
          ( "typeclass.dictionary.type_args",
            json_list_of_strings type_arg_strings );
          ("typeclass.dictionary.repr", `String repr);
        ]
      in
      (json, flat)
  | Constraint_solver.DictParam idx ->
      let identifier = string_of_int idx in
      let json =
        `Assoc
          [
            ("kind", `String "parameter");
            ("identifier", `String identifier);
            ("parameter_index", `Int idx);
            ("trait", `Null);
            ("type_args", `List []);
            ("repr", `String repr);
          ]
      in
      let flat =
        [
          ("typeclass.dictionary.kind", `String "parameter");
          ("typeclass.dictionary.identifier", `String identifier);
          ("typeclass.dictionary.trait", `Null);
          ("typeclass.dictionary.type_args", `List []);
          ("typeclass.dictionary.parameter_index", `Int idx);
          ("typeclass.dictionary.repr", `String repr);
        ]
      in
      (json, flat)
  | Constraint_solver.DictLocal name ->
      let json =
        `Assoc
          [
            ("kind", `String "local");
            ("identifier", `String name);
            ("trait", `Null);
            ("type_args", `List []);
            ("repr", `String repr);
          ]
      in
      let flat =
        [
          ("typeclass.dictionary.kind", `String "local");
          ("typeclass.dictionary.identifier", `String name);
          ("typeclass.dictionary.trait", `Null);
          ("typeclass.dictionary.type_args", `List []);
          ("typeclass.dictionary.repr", `String repr);
        ]
      in
      (json, flat)

let dictionary_json_none =
  let json =
    `Assoc
      [
        ("kind", `String "none");
        ("identifier", `Null);
        ("trait", `Null);
        ("type_args", `List []);
        ("repr", `Null);
      ]
  in
  let flat =
    [
      ("typeclass.dictionary.kind", `String "none");
      ("typeclass.dictionary.identifier", `Null);
      ("typeclass.dictionary.trait", `Null);
      ("typeclass.dictionary.type_args", `List []);
      ("typeclass.dictionary.repr", `Null);
    ]
  in
  (json, flat)

let dictionary_summary dict_ref_opt =
  match dict_ref_opt with
  | Some dict_ref -> dictionary_json_of_ref dict_ref
  | None -> dictionary_json_none

let candidates_json candidates =
  let json_items =
    List.map
      (fun dict_ref ->
        let json, _ = dictionary_json_of_ref dict_ref in
        json)
      candidates
  in
  `List json_items

let extension_json summary =
  let dictionary_json, _ = dictionary_summary summary.dict_ref in
  let stage_json =
    match summary.stage_info with
    | Some stage -> stage_json_of stage
    | None -> `Null
  in
  let graph_json =
    `Assoc
      [
        ( "export_dot",
          match summary.graph_export with
          | Some path when String.trim path <> "" -> `String path
          | _ -> `Null );
      ]
  in
  `Assoc
    [
      ("trait", `String summary.constraint_.trait_name);
      ( "type_args",
        json_list_of_types summary.constraint_.type_args );
      ("constraint", `String (string_of_trait_constraint summary.constraint_));
      ( "resolution_state",
        `String (resolution_state_to_string summary.resolution_state) );
      ("dictionary", dictionary_json);
      ("candidates", candidates_json summary.candidates);
      ("pending", json_list_of_strings summary.pending);
      ( "generalized_typevars",
        json_list_of_strings summary.generalized_typevars );
      ("graph", graph_json);
      ("stage", stage_json);
    ]

let extension_pairs summary =
  let dictionary_json, dictionary_flat = dictionary_summary summary.dict_ref in
  let base =
    [
      ("typeclass.trait", `String summary.constraint_.trait_name);
      ( "typeclass.type_args",
        json_list_of_types summary.constraint_.type_args );
      ( "typeclass.constraint",
        `String (string_of_trait_constraint summary.constraint_) );
      ( "typeclass.resolution_state",
        `String (resolution_state_to_string summary.resolution_state) );
      ("typeclass.dictionary", dictionary_json);
      ("typeclass.candidates", candidates_json summary.candidates);
      ("typeclass.pending", json_list_of_strings summary.pending);
      ( "typeclass.generalized_typevars",
        json_list_of_strings summary.generalized_typevars );
      ( "typeclass.graph.export_dot",
        match summary.graph_export with
        | Some path when String.trim path <> "" -> `String path
        | _ -> `Null );
      ("typeclass.span.start", `Int summary.span.start);
      ("typeclass.span.end", `Int summary.span.end_);
    ]
  in
  let stage_pairs =
    match summary.stage_info with
    | Some stage -> stage_metadata_pairs stage
    | None -> []
  in
  let detail =
    match resolution_state_detail summary.resolution_state with
    | Some text -> [ ("typeclass.resolution_state_detail", `String text) ]
    | None -> []
  in
  base @ dictionary_flat @ stage_pairs @ detail

let metadata_pairs summary =
  extension_pairs summary

let audit_category (summary : summary) =
  let suffix =
    match summary.resolution_state with
    | Resolved -> "resolved"
    | Unresolved -> "unresolved"
    | Ambiguous -> "ambiguous"
    | StageMismatch -> "stage_mismatch"
    | UnresolvedTypeVar -> "unresolved_typevar"
    | Cyclic -> "cyclic"
    | Pending -> "pending"
    | Error _ -> "error"
  in
  Printf.sprintf "typeclass.dictionary.%s" suffix
