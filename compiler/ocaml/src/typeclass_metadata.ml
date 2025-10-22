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

type summary = {
  constraint_ : trait_constraint;
  span : Ast.span;
  resolution_state : resolution_state;
  dict_ref : Constraint_solver.dict_ref option;
  candidates : Constraint_solver.dict_ref list;
  pending : string list;
  generalized_typevars : string list;
  graph_export : string option;
}

let make_summary ?dict_ref ?(candidates = []) ?(pending = [])
    ?(generalized_typevars = []) ?graph_export ~constraint_
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
  let detail =
    match resolution_state_detail summary.resolution_state with
    | Some text -> [ ("typeclass.resolution_state_detail", `String text) ]
    | None -> []
  in
  base @ dictionary_flat @ detail

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
