(* Effect_profile — Shared effect metadata utilities (Phase 2-2)
 *
 * Parser / Typer / Core IR / Runtime で共通利用する効果タグと Stage 要件の定義。
 * 仕様書 docs/spec/1-3-effects-safety.md および設計ノート
 * compiler/ocaml/docs/effect-system-design-note.md を参照。
 *)

open Ast

module Json = Yojson.Basic

(* ========== Stage 定義 ========== *)

type stage_id =
  | Experimental
  | Beta
  | Stable
  | Custom of string

let normalize_stage_name (value : string) =
  value |> String.trim |> String.lowercase_ascii

let stage_id_of_string (value : string) =
  match normalize_stage_name value with
  | "experimental" -> Experimental
  | "beta" -> Beta
  | "stable" -> Stable
  | _ -> Custom value

let stage_id_to_string = function
  | Experimental -> "experimental"
  | Beta -> "beta"
  | Stable -> "stable"
  | Custom value -> value

let stage_id_of_ident (id : ident) = stage_id_of_string id.name

type stage_requirement =
  | StageExact of stage_id
  | StageAtLeast of stage_id

let stage_requirement_to_string = function
  | StageExact stage -> stage_id_to_string stage
  | StageAtLeast stage -> "at_least:" ^ stage_id_to_string stage

let stage_requirement_of_annot = function
  | Ast.StageExact ident -> StageExact (stage_id_of_ident ident)
  | Ast.StageAtLeast ident -> StageAtLeast (stage_id_of_ident ident)

let compare_stage_id lhs rhs =
  let rank = function
    | Experimental -> 0
    | Beta -> 1
    | Stable -> 2
    | Custom _ -> 3
  in
  match (lhs, rhs) with
  | Custom a, Custom b ->
      String.compare (normalize_stage_name a) (normalize_stage_name b)
  | Custom _, _ -> 1
  | _, Custom _ -> -1
  | a, b -> compare (rank a) (rank b)

let stage_requirement_satisfied requirement actual =
  match requirement with
  | StageExact expected -> compare_stage_id expected actual = 0
  | StageAtLeast minimum -> compare_stage_id minimum actual <= 0

let default_stage_requirement = StageAtLeast Stable

(* ========== Stage トレース ========== *)

type stage_trace_step = {
  source : string;
  stage : string option;
  capability : string option;
  note : string option;
  file : string option;
  target : string option;
}

type stage_trace = stage_trace_step list

let stage_string_of_id_option = function
  | Some stage -> Some (stage_id_to_string stage)
  | None -> None

let make_stage_trace_step ?stage ?capability ?note ?file ?target source =
  { source; stage; capability; note; file; target }

let stage_trace_step_of_stage_id ?capability ?note ?file ?target source stage =
  make_stage_trace_step ?capability ?note ?file ?target source
    ~stage:(stage_id_to_string stage)

let stage_trace_step_of_stage_id_opt ?capability ?note ?file ?target source
    stage_opt =
  match stage_opt with
  | Some stage ->
      stage_trace_step_of_stage_id ?capability ?note ?file ?target source stage
  | None -> make_stage_trace_step ?capability ?note ?file ?target source

let append_stage_trace base step = base @ [ step ]

let stage_trace_to_json (trace : stage_trace) =
  let step_to_json (step : stage_trace_step) =
    let fields = [ ("source", `String step.source) ] in
    let fields =
      match step.stage with
      | Some value -> ("stage", `String value) :: fields
      | None -> fields
    in
    let fields =
      match step.capability with
      | Some value -> ("capability", `String value) :: fields
      | None -> fields
    in
    let fields =
      match step.note with
      | Some value -> ("note", `String value) :: fields
      | None -> fields
    in
    let fields =
      match step.file with
      | Some value -> ("file", `String value) :: fields
      | None -> fields
    in
    let fields =
      match step.target with
      | Some value -> ("target", `String value) :: fields
      | None -> fields
    in
    `Assoc (List.rev fields)
  in
  `List (List.map step_to_json trace)

let stage_trace_empty : stage_trace = []

(* ========== 効果タグ・効果集合 ========== *)

type tag = {
  effect_name : string;
  effect_span : span;
}

type set = {
  declared : tag list;
  residual : tag list;
}

let empty_set = { declared = []; residual = [] }

let normalize_effect_name name = String.lowercase_ascii name

let contains_tag name tags =
  let name = normalize_effect_name name in
  List.exists
    (fun tag ->
      String.equal (normalize_effect_name tag.effect_name) name)
    tags

let append_unique tag tags =
  if contains_tag tag.effect_name tags then tags else tags @ [ tag ]

let add_declared tag set = { set with declared = append_unique tag set.declared }

let add_residual tag set = { set with residual = append_unique tag set.residual }

let tags_of_idents idents =
  List.map (fun ident -> { effect_name = ident.name; effect_span = ident.span }) idents

let set_of_ast_nodes ~declared ~residual =
  let initial = empty_set in
  let with_declared = List.fold_left (fun acc tag -> add_declared tag acc) initial declared in
  List.fold_left (fun acc tag -> add_residual tag acc) with_declared residual

(* ========== 効果属性診断ペイロード ========== *)

type invalid_attribute_reason =
  | UnknownAttributeKey of string
  | UnsupportedStageValue
  | UnsupportedCapabilityValue
  | UnknownEffectTag
  | MissingStageValue

type invalid_attribute = {
  attribute_name : string;
  attribute_display : string;
  attribute_span : span;
  invalid_span : span;
  key : string option;
  provided_json : Json.t option;
  provided_display : string option;
  reason : invalid_attribute_reason;
}

type residual_effect_leak = {
  leaked_tag : tag;
  leak_origin : span;
}

type effect_diagnostic_payload = {
  invalid_attributes : invalid_attribute list;
  residual_leaks : residual_effect_leak list;
}

let empty_diagnostic_payload =
  { invalid_attributes = []; residual_leaks = [] }

let normalize_key_name name = String.lowercase_ascii (String.trim name)

let string_of_relative_head = function
  | Ast.Self -> "self"
  | Super n ->
      if n <= 0 then "self"
      else String.concat "." (List.init n (fun _ -> "super"))
  | PlainIdent id -> id.name

let string_of_module_path = function
  | Ast.Root ids -> "::" ^ String.concat "." (List.map (fun id -> id.name) ids)
  | Relative (head, tail) ->
      let head_str = string_of_relative_head head in
      (match tail with
      | [] -> head_str
      | _ ->
          head_str ^ "." ^ String.concat "." (List.map (fun id -> id.name) tail))

let rec expr_to_display expr =
  match expr.expr_kind with
  | Literal (String (value, _)) -> Printf.sprintf "\"%s\"" value
  | Literal (Int (value, _)) -> value
  | Literal (Float value) -> value
  | Literal (Bool true) -> "true"
  | Literal (Bool false) -> "false"
  | Var id -> id.name
  | ModulePath (path, id) ->
      Printf.sprintf "%s.%s" (string_of_module_path path) id.name
  | Literal (Array elements) ->
      let items = List.map expr_to_display elements in
      Printf.sprintf "[%s]" (String.concat ", " items)
  | Literal (Tuple elements) ->
      let items = List.map expr_to_display elements in
      Printf.sprintf "(%s)" (String.concat ", " items)
  | Literal (Record fields) ->
      let items =
        fields
        |> List.map (fun (id, e) ->
               Printf.sprintf "%s = %s" id.name (expr_to_display e))
      in
      Printf.sprintf "{ %s }" (String.concat ", " items)
  | Binary (Eq, lhs, rhs) ->
      Printf.sprintf "%s = %s" (expr_to_display lhs) (expr_to_display rhs)
  | _ -> "<expr>"

let rec expr_to_json expr =
  match expr.expr_kind with
  | Literal (Int (value, _)) -> (
      match int_of_string_opt value with
      | Some v -> Some (`Int v)
      | None -> Some (`String value))
  | Literal (Float value) -> (
      match float_of_string_opt value with
      | Some v -> Some (`Float v)
      | None -> Some (`String value))
  | Literal (Bool b) -> Some (`Bool b)
  | Literal (String (value, _)) -> Some (`String value)
  | Literal (Array elements) ->
      let json_elems =
        List.map
          (fun elem ->
            match expr_to_json elem with
            | Some json -> json
            | None -> `String (expr_to_display elem))
          elements
      in
      Some (`List json_elems)
  | Literal (Record fields) ->
      let json_fields =
        List.map
          (fun (id, value) ->
            let json_value =
              match expr_to_json value with
              | Some json -> json
              | None -> `String (expr_to_display value)
            in
            (id.name, json_value))
          fields
      in
      Some (`Assoc json_fields)
  | _ -> None

let attribute_to_display (attr : attribute) =
  let args =
    match attr.attr_args with
    | [] -> ""
    | args ->
        let rendered = List.map expr_to_display args in
        Printf.sprintf "(%s)" (String.concat ", " rendered)
  in
  Printf.sprintf "@%s%s" attr.attr_name.name args

let invalid_attribute_of_ast (issue : effect_invalid_attribute) =
  let attr = issue.invalid_attribute in
  let attribute_display = attribute_to_display attr in
  match issue.invalid_reason with
  | EffectAttrUnknownKey ident ->
      let key = normalize_key_name ident.name in
      {
        attribute_name = attr.attr_name.name;
        attribute_display;
        attribute_span = attr.attr_span;
        invalid_span = issue.invalid_span;
        key = Some key;
        provided_json = None;
        provided_display = None;
        reason = UnknownAttributeKey key;
      }
  | EffectAttrUnsupportedStageValue value_opt ->
      let provided_json, provided_display =
        match value_opt with
        | Some expr -> (expr_to_json expr, Some (expr_to_display expr))
        | None -> (None, None)
      in
      {
        attribute_name = attr.attr_name.name;
        attribute_display;
        attribute_span = attr.attr_span;
        invalid_span = issue.invalid_span;
        key = Some "stage";
        provided_json;
        provided_display;
        reason = UnsupportedStageValue;
      }
  | EffectAttrUnsupportedCapabilityValue value_opt ->
      let provided_json, provided_display =
        match value_opt with
        | Some expr -> (expr_to_json expr, Some (expr_to_display expr))
        | None -> (None, None)
      in
      {
        attribute_name = attr.attr_name.name;
        attribute_display;
        attribute_span = attr.attr_span;
        invalid_span = issue.invalid_span;
        key = Some "capability";
        provided_json;
        provided_display;
        reason = UnsupportedCapabilityValue;
      }
  | EffectAttrUnknownEffectTag expr ->
      {
        attribute_name = attr.attr_name.name;
        attribute_display;
        attribute_span = attr.attr_span;
        invalid_span = issue.invalid_span;
        key = None;
        provided_json = expr_to_json expr;
        provided_display = Some (expr_to_display expr);
        reason = UnknownEffectTag;
      }
  | EffectAttrMissingStageValue ->
      {
        attribute_name = attr.attr_name.name;
        attribute_display;
        attribute_span = attr.attr_span;
        invalid_span = issue.invalid_span;
        key = Some "stage";
        provided_json = None;
        provided_display = None;
        reason = MissingStageValue;
      }

let payload_of_ast (node : effect_profile_node) =
  let invalid_attributes =
    List.map invalid_attribute_of_ast node.effect_invalid_attributes
  in
  { invalid_attributes; residual_leaks = [] }

let string_of_invalid_reason = function
  | UnknownAttributeKey _ -> "unknown_attribute_key"
  | UnsupportedStageValue -> "unsupported_stage_value"
  | UnsupportedCapabilityValue -> "unsupported_capability_value"
  | UnknownEffectTag -> "unknown_effect_tag"
  | MissingStageValue -> "missing_stage_value"

let invalid_attribute_to_json (item : invalid_attribute) =
  let fields =
    [
      ("attribute", `String item.attribute_display);
      ("attribute_name", `String item.attribute_name);
      ( "reason",
        `String (string_of_invalid_reason item.reason) );
      ( "key",
        match item.key with Some key -> `String key | None -> `Null );
      ( "provided",
        match item.provided_json with
        | Some json -> json
        | None -> `Null );
      ( "provided_display",
        match item.provided_display with
        | Some display -> `String display
        | None -> `Null );
    ]
  in
  `Assoc fields

let residual_effect_leak_to_json (leak : residual_effect_leak) =
  `Assoc
    [
      ("effect", `String leak.leaked_tag.effect_name);
      ( "span",
        `Assoc
          [
            ("start", `Int leak.leak_origin.start);
            ("end", `Int leak.leak_origin.end_);
          ] );
    ]

let effect_diagnostic_payload_to_json (payload : effect_diagnostic_payload) =
  let invalid_json =
    `List (List.map invalid_attribute_to_json payload.invalid_attributes)
  in
  let residual_json =
    `List (List.map residual_effect_leak_to_json payload.residual_leaks)
  in
  `Assoc [ ("invalid_attributes", invalid_json); ("residual_leaks", residual_json) ]

(* ========== Effect Profile ========== *)

type profile = {
  effect_set : set;
  stage_requirement : stage_requirement;
  source_span : span;
  source_name : string option;
  resolved_stage : stage_id option;
  resolved_capability : string option;
  stage_trace : stage_trace;
  diagnostic_payload : effect_diagnostic_payload;
}

let make_profile ?source_name ?resolved_stage ?resolved_capability
    ?(stage_trace = stage_trace_empty)
    ?(diagnostic_payload = empty_diagnostic_payload)
    ~stage_requirement ~effect_set ~span ()
    =
  {
    effect_set;
    stage_requirement;
    source_span = span;
    source_name;
    resolved_stage;
    resolved_capability;
    stage_trace;
    diagnostic_payload;
  }

let default_profile ?source_name ?(stage_trace = stage_trace_empty) ~span () =
  make_profile ?source_name ~stage_trace
    ~stage_requirement:default_stage_requirement ~effect_set:empty_set ~span ()

let profile_of_ast ?source_name ?(stage_trace = stage_trace_empty)
    (node : effect_profile_node) =
  let declared = tags_of_idents node.effect_declared in
  let residual =
    match node.effect_residual with
    | [] -> declared
    | entries -> tags_of_idents entries
  in
  let capability_name =
    match node.effect_capabilities with
    | cap :: _ -> Some cap.name
    | [] -> None
  in
  let effect_set = set_of_ast_nodes ~declared ~residual in
  let stage_requirement =
    match node.effect_stage with
    | Some annot -> stage_requirement_of_annot annot
    | None -> default_stage_requirement
  in
  let diagnostic_payload = payload_of_ast node in
  make_profile ?source_name ?resolved_capability:capability_name
    ~stage_requirement ~effect_set ~span:node.effect_span ~stage_trace
    ~diagnostic_payload ()
