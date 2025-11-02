%{
(* Parser — Reml 構文解析器
 *
 * docs/spec/1-1-syntax.md に基づく構文解析を実装する。
 * Menhir で LR(1) パーサを生成。
 *)

open Ast
open Parser_flags

exception Experimental_effects_disabled of Lexing.position * Lexing.position

(* ヘルパー関数 *)

let make_span start_pos end_pos = {
  start = start_pos.Lexing.pos_cnum;
  end_ = end_pos.Lexing.pos_cnum;
}
let tuple_index_from_literal (value, base) =
  match base with
  | Base10 -> (
      try int_of_string value with
      | Failure _ -> failwith "tuple index must be decimal"
    )
  | _ -> failwith "tuple index must be decimal"

let make_qualified_ident parts span =
  make_ident (String.concat "." parts) span

let make_stage_ident span name =
  let normalized = String.lowercase_ascii (String.trim name) in
  make_ident normalized span

let make_effect_ident span name =
  let trimmed = name |> String.trim |> String.lowercase_ascii in
  make_ident trimmed span

let make_capability_ident span name =
  let normalized = name |> String.trim |> String.lowercase_ascii in
  make_ident normalized span

let string_of_relative_head = function
  | Self -> "self"
  | Super n ->
      if n <= 0 then "self"
      else String.concat "." (List.init n (fun _ -> "super"))
  | PlainIdent id -> id.name

let string_of_module_path = function
  | Root ids -> String.concat "." (List.map (fun id -> id.name) ids)
  | Relative (head, tail) ->
      let head_str = string_of_relative_head head in
      (match tail with
      | [] -> head_str
      | _ ->
          head_str ^ "." ^ String.concat "." (List.map (fun id -> id.name) tail))

type effect_attr_analysis = {
  tags : ident list;
  capabilities : ident list;
  invalids : effect_invalid_attribute list;
}

let empty_attr_analysis = { tags = []; capabilities = []; invalids = [] }

let append_attr_analysis lhs rhs =
  {
    tags = lhs.tags @ rhs.tags;
    capabilities = lhs.capabilities @ rhs.capabilities;
    invalids = lhs.invalids @ rhs.invalids;
  }

let make_invalid_attribute attr reason span =
  { invalid_attribute = attr; invalid_reason = reason; invalid_span = span }

let normalize_key key = String.lowercase_ascii (String.trim key)

let stage_ident_from_value expr =
  match expr.expr_kind with
  | Literal (String (value, _)) -> Some (make_stage_ident expr.expr_span value)
  | Var id -> Some id
  | ModulePath (_, id) -> Some id
  | _ -> None

let capability_ident_from_value expr =
  match expr.expr_kind with
  | Literal (String (value, _)) ->
      Some (make_capability_ident expr.expr_span value)
  | Var id -> Some (make_capability_ident id.span id.name)
  | ModulePath (path, id) ->
      let base = string_of_module_path path in
      let name =
        if String.equal base "" then id.name else base ^ "." ^ id.name
      in
      Some (make_capability_ident expr.expr_span name)
  | _ -> None

let default_effect_keys = [ "allows_effects"; "handles"; "effect"; "effects" ]

let key_matches allowed key =
  match allowed with
  | None -> List.exists (String.equal key) default_effect_keys
  | Some keys -> List.exists (String.equal key) keys

let key_from_expr expr =
  match expr.expr_kind with
  | Var id -> Some id
  | ModulePath (_, id) -> Some id
  | _ -> None

let rec collect_capabilities_from_expr ?(allowed_keys : string list option = None)
    attr expr =
  match expr.expr_kind with
  | Literal (Array elements) ->
      List.fold_left
        (fun acc element ->
          append_attr_analysis acc
            (collect_capabilities_from_expr ~allowed_keys:None attr element))
        empty_attr_analysis elements
  | Literal (Tuple elements) ->
      List.fold_left
        (fun acc element ->
          append_attr_analysis acc
            (collect_capabilities_from_expr ~allowed_keys:None attr element))
        empty_attr_analysis elements
  | Literal (Record fields) ->
      List.fold_left
        (fun acc (field_id, field_expr) ->
          let key = normalize_key field_id.name in
          if key_matches allowed_keys key then
            append_attr_analysis acc
              (collect_capabilities_from_expr ~allowed_keys:None attr field_expr)
          else
            append_attr_analysis acc
              {
                tags = [];
                capabilities = [];
                invalids =
                  [
                    make_invalid_attribute attr
                      (EffectAttrUnknownKey field_id)
                      field_id.span;
                  ];
              })
        empty_attr_analysis fields
  | Binary (Eq, lhs, rhs) -> (
      match key_from_expr lhs with
      | Some key_ident ->
          let key = normalize_key key_ident.name in
          if key_matches allowed_keys key then
            collect_capabilities_from_expr ~allowed_keys:None attr rhs
          else
            {
              tags = [];
              capabilities = [];
              invalids =
                [
                  make_invalid_attribute attr
                    (EffectAttrUnknownKey key_ident)
                    key_ident.span;
                ];
            }
      | None ->
          {
            tags = [];
            capabilities = [];
            invalids =
              [
                make_invalid_attribute attr
                  (EffectAttrUnsupportedCapabilityValue (Some lhs))
                  lhs.expr_span;
              ];
          })
  | _ -> (
      match capability_ident_from_value expr with
      | Some ident -> { tags = []; capabilities = [ ident ]; invalids = [] }
      | None ->
          {
            tags = [];
            capabilities = [];
            invalids =
              [
                make_invalid_attribute attr
                  (EffectAttrUnsupportedCapabilityValue (Some expr))
                  expr.expr_span;
              ];
          })

let stage_value_from_expr attr expr =
  match stage_ident_from_value expr with
  | Some ident -> (Some ident, [])
  | None ->
      ( None,
        [
          make_invalid_attribute attr
            (EffectAttrUnsupportedStageValue (Some expr))
            expr.expr_span;
        ] )

let split_effect_module_path path =
  match path with
  | Root ids -> (
      match List.rev ids with
      | [] -> failwith "effect path requires at least one identifier"
      | effect_ident :: prefix_rev -> (
          match List.rev prefix_rev with
          | [] -> (None, effect_ident)
          | prefix -> (Some (Root prefix), effect_ident)))
  | Relative (PlainIdent id, []) -> (None, id)
  | Relative (head, tail) -> (
      match List.rev tail with
      | [] -> (
          match head with
          | PlainIdent id -> (None, id)
          | _ -> failwith "effect path requires a concrete identifier")
      | effect_ident :: prefix_rev ->
          let prefix_tail = List.rev prefix_rev in
          let effect_path =
            match prefix_tail with
            | [] -> Some (Relative (head, []))
            | _ -> Some (Relative (head, prefix_tail))
          in
          (effect_path, effect_ident))

let make_effect_reference_from_call_path path span =
  let fail () =
    failwith "effect target must be written as Effect.operation"
  in
  let effect_path, effect_name, effect_operation =
    match path with
    | Root ids -> (
        match List.rev ids with
        | operation :: effect_name :: prefix_rev ->
            let effect_path =
              match List.rev prefix_rev with
              | [] -> None
              | prefix -> Some (Root prefix)
            in
            (effect_path, effect_name, operation)
        | _ -> fail ())
    | Relative (head, tail) -> (
        match List.rev tail with
        | [] -> fail ()
        | operation :: tail_rev ->
            let tail_without_operation = List.rev tail_rev in
            let effect_segments =
              match head with
              | PlainIdent id -> id :: tail_without_operation
              | _ -> tail_without_operation
            in
            (match List.rev effect_segments with
            | [] -> fail ()
            | effect_name :: prefix_rev ->
                let effect_path =
                  match head with
                  | PlainIdent _ -> (
                      match List.rev prefix_rev with
                      | [] -> None
                      | first :: rest ->
                          Some (Relative (PlainIdent first, rest)))
                  | _ ->
                      Some (Relative (head, List.rev prefix_rev))
                in
                (effect_path, effect_name, operation)))
  in
  {
    effect_path;
    effect_name;
    effect_operation;
    effect_span = span;
  }

let make_effect_call sugar effect_ref args =
  { effect_ref; effect_args = args; effect_sugar = sugar }

let analyze_stage_expr attr expr =
  match expr.expr_kind with
  | Literal (Record fields) ->
      List.fold_left
        (fun (stage_opt, capabilities, invalids) (field_id, field_expr) ->
          let key_name = normalize_key field_id.name in
          if String.equal key_name "stage" then
            let value_opt, new_invalids = stage_value_from_expr attr field_expr in
            let stage_opt =
              match stage_opt with
              | Some _ -> stage_opt
              | None ->
                  Option.map (fun ident -> (ident, field_expr.expr_span))
                    value_opt
            in
            (stage_opt, capabilities, invalids @ new_invalids)
          else if key_matches (Some [ "capability"; "capabilities" ]) key_name
          then
            let cap_analysis =
              collect_capabilities_from_expr ~allowed_keys:None attr field_expr
            in
            ( stage_opt,
              capabilities @ cap_analysis.capabilities,
              invalids @ cap_analysis.invalids )
          else
            ( stage_opt,
              capabilities,
              invalids
              @ [
                  make_invalid_attribute attr
                    (EffectAttrUnknownKey field_id)
                    field_id.span;
                ] ))
        (None, [], []) fields
  | Binary (Eq, lhs, rhs) -> (
      match key_from_expr lhs with
      | Some key_ident ->
          let key_name = normalize_key key_ident.name in
          if String.equal key_name "stage" then
            let value_opt, invalids = stage_value_from_expr attr rhs in
            let stage =
              Option.map (fun ident -> (ident, rhs.expr_span)) value_opt
            in
            (stage, [], invalids)
          else if key_matches (Some [ "capability"; "capabilities" ]) key_name
          then
            let cap_analysis =
              collect_capabilities_from_expr ~allowed_keys:None attr rhs
            in
            ( None,
              cap_analysis.capabilities,
              cap_analysis.invalids )
          else
            ( None,
              [],
              [
                make_invalid_attribute attr
                  (EffectAttrUnknownKey key_ident)
                  key_ident.span;
              ] )
      | None ->
          let invalid =
            make_invalid_attribute attr
              (EffectAttrUnsupportedStageValue (Some lhs))
              lhs.expr_span
          in
          (None, [], [ invalid ]))
  | _ ->
      let stage_opt, invalids = stage_value_from_expr attr expr in
      ( Option.map (fun ident -> (ident, expr.expr_span)) stage_opt,
        [],
        invalids )

let stage_requirement_from_attr attr constructor =
  match attr.attr_args with
  | [] ->
      ( None,
        [],
        [
          make_invalid_attribute attr EffectAttrMissingStageValue
            attr.attr_span;
        ] )
  | args ->
      let rec gather stage_opt capabilities acc_invalids = function
        | [] -> (stage_opt, capabilities, List.rev acc_invalids)
        | expr :: rest ->
            let stage_result, caps, invalids = analyze_stage_expr attr expr in
            let stage_opt =
              match (stage_opt, stage_result) with
              | Some _, _ -> stage_opt
              | None, Some (ident, span) -> Some (constructor ident, span)
              | None, None -> None
            in
            let capabilities = capabilities @ caps in
            gather stage_opt capabilities
              (List.rev_append invalids acc_invalids)
              rest
      in
      let stage_opt, capabilities, invalids =
        gather None [] [] args
      in
      (stage_opt, capabilities, invalids)

let stage_requirement_from_attrs attrs =
  let rec find capabilities acc_invalids = function
    | [] -> (None, capabilities, List.rev acc_invalids)
    | attr :: rest ->
        let attr_name = normalize_key attr.attr_name.name in
        let continue caps invalids =
          find caps (List.rev_append invalids acc_invalids) rest
        in
        (match attr_name with
        | "requires_capability" | "requires_capability_exact" ->
            let stage_opt, caps, invalids =
              stage_requirement_from_attr attr (fun id -> StageExact id)
            in
            (match stage_opt with
            | Some (req, span) ->
                (Some (req, span), capabilities @ caps,
                 List.rev_append invalids acc_invalids)
            | None -> continue (capabilities @ caps) invalids)
        | "requires_capability_at_least" ->
            let stage_opt, caps, invalids =
              stage_requirement_from_attr attr (fun id -> StageAtLeast id)
            in
            (match stage_opt with
            | Some (req, span) ->
                (Some (req, span), capabilities @ caps,
                 List.rev_append invalids acc_invalids)
            | None -> continue (capabilities @ caps) invalids)
        | "experimental" | "beta" | "stable" ->
            ( Some (StageExact attr.attr_name, attr.attr_span),
              capabilities,
              List.rev acc_invalids )
        | _ -> find capabilities acc_invalids rest)
  in
  match find [] [] attrs with
  | (Some _ as stage, caps, invalids) -> (stage, caps, invalids)
  | (None, caps, invalids) ->
      let default_attr =
        List.find_opt
          (fun attr ->
            let name = normalize_key attr.attr_name.name in
            name = "dsl_export" || name = "allows_effects")
          attrs
      in
      (match default_attr with
      | Some attr ->
          let stage_ident = make_stage_ident attr.attr_span "stable" in
          (Some (StageAtLeast stage_ident, attr.attr_span), caps, invalids)
      | None -> (None, caps, invalids))

let rec collect_effect_tags_from_expr ?(allowed_keys : string list option = None)
    attr expr =
  match expr.expr_kind with
  | Literal (Array elements) ->
      List.fold_left
        (fun acc element ->
          append_attr_analysis acc
            (collect_effect_tags_from_expr ~allowed_keys:None attr element))
        empty_attr_analysis elements
  | Literal (Tuple elements) ->
      List.fold_left
        (fun acc element ->
          append_attr_analysis acc
            (collect_effect_tags_from_expr ~allowed_keys:None attr element))
        empty_attr_analysis elements
  | Literal (Record fields) ->
      List.fold_left
        (fun acc (field_id, field_expr) ->
          let key = normalize_key field_id.name in
          if key_matches allowed_keys key then
            append_attr_analysis acc
              (collect_effect_tags_from_expr ~allowed_keys:None attr field_expr)
          else
            append_attr_analysis acc
              {
                tags = [];
                capabilities = [];
                invalids =
                  [
                    make_invalid_attribute attr
                      (EffectAttrUnknownKey field_id)
                      field_id.span;
                  ];
              })
        empty_attr_analysis fields
  | Binary (Eq, lhs, rhs) -> (
      match key_from_expr lhs with
      | Some key_ident ->
          let key = normalize_key key_ident.name in
          if key_matches allowed_keys key then
            collect_effect_tags_from_expr ~allowed_keys:None attr rhs
          else
            {
              tags = [];
              capabilities = [];
              invalids =
                [
                  make_invalid_attribute attr
                    (EffectAttrUnknownKey key_ident)
                    key_ident.span;
                ];
            }
      | None ->
          {
            tags = [];
            capabilities = [];
            invalids =
              [
                make_invalid_attribute attr
                  (EffectAttrUnknownEffectTag lhs)
                  lhs.expr_span;
              ];
          })
  | Var id ->
      {
        tags = [ make_effect_ident id.span id.name ];
        capabilities = [];
        invalids = [];
      }
  | ModulePath (_, id) ->
      {
        tags = [ make_effect_ident id.span id.name ];
        capabilities = [];
        invalids = [];
      }
  | Literal (String (value, _)) ->
      {
        tags = [ make_effect_ident expr.expr_span value ];
        capabilities = [];
        invalids = [];
      }
  | _ ->
      {
        tags = [];
        capabilities = [];
        invalids =
          [
            make_invalid_attribute attr
              (EffectAttrUnknownEffectTag expr)
              expr.expr_span;
          ];
      }

let collect_effect_tags_from_attrs attrs =
  let collect_args_with (allowed_keys : string list option) attr =
    List.fold_left
      (fun acc expr ->
        append_attr_analysis acc
          (collect_effect_tags_from_expr ~allowed_keys attr expr))
      empty_attr_analysis attr.attr_args
  in
  List.fold_left
    (fun acc attr ->
      let name = normalize_key attr.attr_name.name in
      let analysis =
        match name with
        | "dsl_export" -> collect_args_with (Some ["allows_effects"]) attr
        | "allows_effects" -> collect_args_with None attr
        | "handles" ->
            collect_args_with (Some ["handles"; "effect"; "effects"]) attr
        | _ -> empty_attr_analysis
      in
      append_attr_analysis acc analysis)
    empty_attr_analysis attrs

let merge_effect_tags existing tags =
  List.fold_left
    (fun acc tag ->
      if List.exists (fun current -> String.equal current.name tag.name) acc
      then acc
      else acc @ [tag])
    existing tags

let merge_capabilities existing caps =
  List.fold_left
    (fun acc cap ->
      if
        List.exists
          (fun current -> String.equal current.name cap.name)
          acc
      then acc
      else acc @ [ cap ])
    existing caps

let default_attr_span attrs =
  match attrs with
  | attr :: _ -> attr.attr_span
  | [] -> Ast.dummy_span

let apply_stage_to_profile attrs existing =
  let stage_info, capabilities, stage_invalids =
    stage_requirement_from_attrs attrs
  in
  let tag_analysis = collect_effect_tags_from_attrs attrs in
  let effect_tags = tag_analysis.tags in
  let collected_invalids = stage_invalids @ tag_analysis.invalids in
  if
    stage_info = None
    && effect_tags = []
    && capabilities = []
    && collected_invalids = []
  then
    existing
  else
    let stage_req, stage_span =
      match stage_info with
      | Some (req, span) -> (Some req, span)
      | None -> (None, default_attr_span attrs)
    in
    let base =
      match existing with
      | Some info -> info
      | None ->
          {
            effect_declared = [];
            effect_residual = [];
            effect_stage = None;
            effect_capabilities = [];
            effect_span = stage_span;
            effect_invalid_attributes = [];
          }
    in
    let base =
      match stage_req with
      | Some req ->
          { base with effect_stage = Some req; effect_span = stage_span }
      | None -> { base with effect_span = stage_span }
    in
    let updated_declared =
      merge_effect_tags base.effect_declared effect_tags
    in
    let updated_residual =
      if base.effect_residual = [] || base.effect_residual = base.effect_declared
      then updated_declared
      else merge_effect_tags base.effect_residual effect_tags
    in
    let updated_invalids =
      base.effect_invalid_attributes @ collected_invalids
    in
    let updated_capabilities =
      merge_capabilities base.effect_capabilities capabilities
    in
    Some
      {
        base with
        effect_declared = updated_declared;
        effect_residual = updated_residual;
        effect_capabilities = updated_capabilities;
        effect_invalid_attributes = updated_invalids;
      }

let apply_stage_to_fn attrs fn =
  { fn with fn_effect_profile = apply_stage_to_profile attrs fn.fn_effect_profile }

let apply_stage_to_signature attrs sig_ =
  { sig_ with sig_effect_profile = apply_stage_to_profile attrs sig_.sig_effect_profile }

let empty_extern_metadata : Ast.extern_metadata =
  {
    extern_target = None;
    extern_calling_convention = None;
    extern_link_name = None;
    extern_ownership = None;
    extern_invalid_attributes = [];
  }

let make_extern_invalid (attr : Ast.attribute)
    (reason : Ast.extern_invalid_attribute_reason) :
    Ast.extern_invalid_attribute =
  { extern_attr = attr; extern_reason = reason; extern_attr_span = attr.attr_span }

let add_extern_invalid meta invalid =
  {
    meta with
    extern_invalid_attributes =
      meta.extern_invalid_attributes @ [invalid];
  }

let add_extern_invalids meta invalids =
  List.fold_left add_extern_invalid meta invalids

let string_value_from_expr expr =
  match expr.expr_kind with
  | Literal (String (value, _)) -> Some value
  | Var id -> Some id.name
  | ModulePath (path, id) ->
      let base = string_of_module_path path in
      if String.equal base "" then Some id.name
      else Some (base ^ "." ^ id.name)
  | _ -> None

let extern_string_argument attr key =
  match attr.attr_args with
  | [] ->
      ( None,
        [
          make_extern_invalid attr
            (ExternAttrMissingStringValue key);
        ] )
  | expr :: _ -> (
      match string_value_from_expr expr with
      | Some value -> (Some value, [])
      | None ->
          ( None,
            [
              make_extern_invalid attr
                (ExternAttrMissingStringValue key);
            ] ))

let apply_string_field meta attr key getter setter =
  let value_opt, invalids = extern_string_argument attr key in
  let meta = add_extern_invalids meta invalids in
  match value_opt with
  | None -> meta
  | Some value ->
      if Option.is_some (getter meta) then
        add_extern_invalid meta
          (make_extern_invalid attr (ExternAttrDuplicateKey key))
      else setter meta value

let extern_metadata_from_attrs attrs =
  List.fold_left
    (fun acc attr ->
      let key = normalize_key attr.attr_name.name in
      match key with
      | "ffi_target" | "target" ->
          apply_string_field acc attr key
            (fun meta -> meta.extern_target)
            (fun meta value -> { meta with extern_target = Some value })
      | "ffi_calling_convention" | "callconv" ->
          apply_string_field acc attr key
            (fun meta -> meta.extern_calling_convention)
            (fun meta value ->
              { meta with extern_calling_convention = Some value })
      | "ffi_link_name" | "link_name" ->
          apply_string_field acc attr key
            (fun meta -> meta.extern_link_name)
            (fun meta value -> { meta with extern_link_name = Some value })
      | "ffi_ownership" | "ownership" ->
          apply_string_field acc attr key
            (fun meta -> meta.extern_ownership)
            (fun meta value -> { meta with extern_ownership = Some value })
      | _ ->
          add_extern_invalid acc
            (make_extern_invalid attr (ExternAttrUnknownKey key)))
    empty_extern_metadata
    attrs

let derive_extern_target items =
  match items with
  | [] -> None
  | item :: rest ->
      let target = item.extern_metadata.extern_target in
      let consistent =
        List.for_all
          (fun elem ->
            match (target, elem.extern_metadata.extern_target) with
            | None, Some _ -> false
            | Some _, None -> false
            | Some lhs, Some rhs -> String.equal lhs rhs
            | None, None -> true)
          rest
      in
      if consistent then target else None

%}

(* トークン定義 *)

(* キーワード *)
%token MODULE USE AS PUB SELF SUPER
%token LET VAR FN TYPE ALIAS NEW TRAIT IMPL EXTERN
%token EFFECT OPERATION HANDLER CONDUCTOR CHANNELS EXECUTION MONITORING
%token IF THEN ELSE MATCH WITH FOR IN WHILE LOOP RETURN DEFER UNSAFE
%token PERFORM DO HANDLE
%token WHERE
%token TRUE FALSE
%token BREAK CONTINUE

(* 演算子・区切り *)
%token PIPE CHANNEL_PIPE
%token DOT COMMA SEMICOLON COLON AT BAR EQ COLONEQ ARROW DARROW
%token LPAREN RPAREN LBRACKET RBRACKET LBRACE RBRACE
%token PLUS MINUS STAR SLASH PERCENT POW
%token EQEQ NE LT LE GT GE
%token AND OR NOT
%token QUESTION DOTDOT UNDERSCORE

(* リテラル *)
%token <string * Ast.int_base> INT
%token <string> FLOAT
%token <string> CHAR
%token <string * Ast.string_kind> STRING
%token <string> IDENT
%token <string> UPPER_IDENT

%token EOF

(* 優先順位と結合性 (仕様 §D.1 に準拠) *)
%left PIPE
%left OR
%left AND
%nonassoc EQEQ NE
%nonassoc LT LE GT GE
%left PLUS MINUS
%left STAR SLASH PERCENT
%right POW
%right UMINUS UNOT  (* 単項演算子 *)
%left DOT LPAREN LBRACKET QUESTION

(* match アーム境界のための特別な優先順位レベル *)
%nonassoc MATCH_ARM

(* 開始シンボル *)
%start <Ast.compilation_unit> compilation_unit

%%

(* ========== コンパイル単位 ========== *)

compilation_unit:
  | header = module_header_opt;
    uses = use_decl_list;
    decls = decl_list;
    EOF
    { { header; uses; decls } }

module_header_opt:
  | (* empty *) { None }
  | MODULE; path = module_path
    {
      let span = make_span $startpos $endpos in
      Some { module_path = path; header_span = span }
    }

use_decl_list:
  | (* empty *) { [] }
  | uses = use_decl_list; u = use_decl { uses @ [u] }

decl_list:
  | (* empty *) { [] }
  | decls = decl_list; d = decl { decls @ [d] }

(* ========== use 宣言 ========== *)

use_decl:
  | pub = pub_opt; USE; tree = use_tree
    {
      let span = make_span $startpos $endpos in
      { use_pub = pub; use_tree = tree; use_span = span }
    }

pub_opt:
  | (* empty *) { false }
  | PUB { true }

use_tree:
  | path = module_path; alias = use_alias_opt
    { UsePath (path, alias) }
  | prefix = use_brace_prefix; DOT; LBRACE; items = use_item_list; RBRACE
    { UseBrace (prefix, items) }

use_alias_opt:
  | (* empty *) { None }
  | AS; id = ident { Some id }

use_brace_prefix:
  | base = use_brace_base { base }
  | prefix = use_brace_prefix; DOT; id = ident
    {
      match prefix with
      | Root ids -> Root (ids @ [id])
      | Relative (head, tail) -> Relative (head, tail @ [id])
    }

use_brace_base:
  | COLON; COLON; ids = ident_list { Root ids }
  | SELF { Relative (Self, []) }
  | count = super_list { Relative (Super count, []) }
  | id = ident { Relative (PlainIdent id, []) }

use_item_list:
  | item = use_item { [item] }
  | items = use_item_list; COMMA; item = use_item { items @ [item] }

use_item:
  | name = ident; alias = use_alias_opt; nested = use_item_nested_opt
    {
      { item_name = name; item_alias = alias; item_nested = nested }
    }

use_item_nested_opt:
  | (* empty *) { None }
  | DOT; LBRACE; items = use_item_list; RBRACE { Some items }

(* ========== モジュールパス ========== *)

module_path:
  | COLON; COLON; ids = ident_list { Root ids }
  | head = relative_head; tail = relative_tail { Relative (head, tail) }

relative_head:
  | SELF { Self }
  | count = super_list { Super count }
  | id = ident { PlainIdent id }

super_list:
  | SUPER { 1 }
  | count = super_list; DOT; SUPER { count + 1 }

relative_tail:
  | (* empty *) { [] }
  | DOT; ids = ident_list { ids }

(* ========== 宣言 ========== *)

decl:
  | attrs = attribute_list; vis = visibility; kind = decl_kind
    {
      let span = make_span $startpos $endpos in
      let kind =
        match kind with
        | FnDecl fn -> FnDecl (apply_stage_to_fn attrs fn)
        | _ -> kind
      in
      { decl_attrs = attrs; decl_vis = vis; decl_kind = kind; decl_span = span }
    }

attribute_list:
  | (* empty *) { [] }
  | attrs = attribute_list; attr = attribute { attrs @ [attr] }

attribute:
  | AT; name = ident; args = attribute_args_opt
    {
      let span = make_span $startpos $endpos in
      { attr_name = name; attr_args = args; attr_span = span }
    }

attribute_args_opt:
  | (* empty *) { [] }
  | LPAREN; args = expr_list; RPAREN { args }

visibility:
  | (* empty *) { Private }
  | PUB { Public }

decl_kind:
  | LET; pat = pattern; ty = type_annot_opt; EQ; e = expr
    { LetDecl (pat, ty, e) }
  | VAR; pat = pattern; ty = type_annot_opt; EQ; e = expr
    { VarDecl (pat, ty, e) }
  | fn = fn_decl { FnDecl fn }
  | TYPE; decl = type_decl { TypeDecl decl }
  | TRAIT; decl = trait_decl { TraitDecl decl }
  | IMPL; decl = impl_decl { ImplDecl decl }
  | EXTERN; decl = extern_decl { ExternDecl decl }
  | EFFECT; decl = effect_decl { EffectDecl decl }
  | HANDLER; decl = handler_decl { HandlerDecl decl }

fn_decl:
  | FN; name = ident; generics = generic_params_opt; params = fn_params;
    ret = return_type_opt; where_clause = where_clause_opt;
    effects = effect_profile_opt; body = fn_body
    {
      {
        fn_name = name;
        fn_generic_params = generics;
        fn_params = params;
        fn_ret_type = ret;
        fn_where_clause = where_clause;
        fn_effect_profile = effects;
        fn_body = body;
      }
    }

fn_params:
  | LPAREN; RPAREN { [] }
  | LPAREN; ps = param_list; RPAREN { ps }

generic_params_opt:
  | (* empty *) { [] }
  | LT; params = generic_param_list; GT { params }

generic_param_list:
  | id = ident { [id] }
  | params = generic_param_list; COMMA; id = ident { params @ [id] }

where_clause_opt:
  | (* empty *) { [] }
  | WHERE; clauses = constraint_list { clauses }

constraint_list:
  | c = constraint_spec { [c] }
  | cs = constraint_list; COMMA; c = constraint_spec { cs @ [c] }

constraint_spec:
  | trait_id = ident; LT; args = type_arg_list; GT
    {
      let span = make_span $startpos $endpos in
      { constraint_trait = trait_id; constraint_types = args; constraint_span = span }
    }

type_arg_list:
  | ty = type_annot { [ty] }
  | tys = type_arg_list; COMMA; ty = type_annot { tys @ [ty] }

effect_profile_opt:
  | (* empty *) { None }
  | NOT; LBRACE; tags = effect_tag_list_opt; RBRACE
    {
      let span = make_span $startpos $endpos in
      Some
        {
          effect_declared = tags;
          effect_residual = tags;
          effect_stage = None;
          effect_capabilities = [];
          effect_span = span;
          effect_invalid_attributes = [];
        }
    }

effect_tag_list_opt:
  | (* empty *) { [] }
  | tags = effect_tag_list { tags }

effect_tag_list:
  | id = ident { [id] }
  | tags = effect_tag_list; COMMA; id = ident { tags @ [id] }

param_list:
  | p = param { [p] }
  | ps = param_list; COMMA; p = param { ps @ [p] }

param:
  | pat = pattern; ty = type_annot_opt; default = default_expr_opt
    {
      let span = make_span $startpos $endpos in
      { pat; ty; default; param_span = span }
    }

default_expr_opt:
  | (* empty *) { None }
  | EQ; e = expr { Some e }

return_type_opt:
  | (* empty *) { None }
  | ARROW; ty = type_annot { Some ty }

fn_body:
  | EQ; e = expr { FnExpr e }
  | block = block_stmt { FnBlock block }

(* ========== 型宣言 ========== *)

type_decl:
  | ALIAS; name = ident; generics = generic_params_opt; EQ; ty = type_annot
    { AliasDecl (name, generics, ty) }
  | name = ident; generics = generic_params_opt; EQ; NEW; ty = type_annot
    { NewtypeDecl (name, generics, ty) }
  | name = ident; generics = generic_params_opt; EQ; variants = sum_variant_list
    { SumDecl (name, generics, variants) }

sum_variant_list:
  | first = sum_variant { [first] }
  | BAR; first = sum_variant { [first] }
  | variants = sum_variant_list; BAR; v = sum_variant { variants @ [v] }

sum_variant:
  | name = ident; payload = variant_payload_opt
    {
      let span = make_span $startpos $endpos in
      { variant_name = name; variant_types = payload; variant_span = span }
    }

variant_payload_opt:
  | (* empty *) { [] }
  | LPAREN; args = type_arg_list_opt; RPAREN { args }

type_arg_list_opt:
  | (* empty *) { [] }
  | args = type_arg_list { args }

(* ========== トレイト宣言 ========== *)

trait_decl:
  | name = ident; generics = generic_params_opt; where_clause = where_clause_opt; body = trait_body
    { { trait_name = name; trait_params = generics; trait_where = where_clause; trait_items = body } }

trait_body:
  | LBRACE; items = trait_item_list; RBRACE { items }

trait_item_list:
  | (* empty *) { [] }
  | items = trait_item_list; item = trait_item { items @ [item] }

trait_item:
  | attrs = attribute_list; sig_ = fn_signature_only; default = trait_default_opt
    {
      {
        item_attrs = attrs;
        item_sig = apply_stage_to_signature attrs sig_;
        item_default = default;
      }
    }

fn_signature_only:
  | FN; name = ident; generics = generic_params_opt; params = fn_params;
    ret = return_type_opt; where_clause = where_clause_opt; effects = effect_profile_opt
    {
      {
        sig_name = name;
        sig_params = generics;
        sig_args = params;
        sig_ret = ret;
        sig_where = where_clause;
        sig_effect_profile = effects;
      }
    }

trait_default_opt:
  | (* empty *) { None }
  | EQ; e = expr { Some (FnExpr e) }
  | block = block_stmt { Some (FnBlock block) }

(* ========== impl 宣言 ========== *)

impl_decl:
  | generics = generic_params_opt; target = impl_target; where_clause = where_clause_opt; body = impl_body
    {
      let trait_ref, ty = target in
      { impl_params = generics; impl_trait = trait_ref; impl_type = ty; impl_where = where_clause; impl_items = body }
    }

impl_target:
  | trait_ref = trait_reference; FOR; ty = type_annot { (Some trait_ref, ty) }
  | ty = type_annot { (None, ty) }

trait_reference:
  | name = ident; args = generic_args_opt { (name, args) }

generic_args_opt:
  | (* empty *) { [] }
  | LT; args = type_arg_list; GT { args }

impl_body:
  | LBRACE; items = impl_item_list; RBRACE { items }

impl_item_list:
  | (* empty *) { [] }
  | items = impl_item_list; item = impl_item { items @ [item] }

impl_item:
  | attrs = attribute_list; fn = fn_decl
    { ImplFn (apply_stage_to_fn attrs fn) }
  | LET; pat = pattern; ty = type_annot_opt; EQ; e = expr
    { ImplLet (pat, ty, e) }
  | VAR; pat = pattern; ty = type_annot_opt; EQ; e = expr
    { ImplLet (pat, ty, e) }

(* ========== extern 宣言 ========== *)

extern_decl:
  | abi = extern_abi; body = extern_body
      {
        let target = derive_extern_target body in
        {
          extern_abi = abi;
          extern_block_target = target;
          extern_items = body;
        }
      }

extern_abi:
  | s = STRING { fst s }

extern_body:
  | sig_ = fn_signature_only; SEMICOLON
    {
      [
        {
          extern_attrs = [];
          extern_sig = sig_;
          extern_metadata = empty_extern_metadata;
        };
      ]
    }
  | LBRACE; items = extern_item_list; RBRACE { items }

extern_item_list:
  | (* empty *) { [] }
  | items = extern_item_list; item = extern_item { items @ [item] }

extern_item:
  | attrs = attribute_list; sig_ = fn_signature_only; SEMICOLON
    {
      let metadata = extern_metadata_from_attrs attrs in
      {
        extern_attrs = attrs;
        extern_sig = apply_stage_to_signature attrs sig_;
        extern_metadata = metadata;
      }
    }

(* ========== effect / handler 宣言 ========== *)

effect_decl:
  | name = ident; COLON; tag = ident; body = effect_body
    { { effect_name = name; effect_tag = tag; operations = body } }

effect_body:
  | LBRACE; ops = operation_list; RBRACE { ops }

operation_list:
  | (* empty *) { [] }
  | ops = operation_list; op = operation_decl { ops @ [op] }

operation_decl:
  | attrs = attribute_list; OPERATION; name = ident; COLON; ty = type_annot
    {
      let span = make_span $startpos $endpos in
      ignore attrs;
      { op_name = name; op_type = ty; op_span = span }
    }

handler_decl:
  | name = ident; body = handler_body
    {
      let span = make_span $startpos $endpos in
      { handler_name = name; handler_entries = body; handler_span = span }
    }

handler_body:
  | LBRACE; entries = handler_entry_list; RBRACE { entries }

handler_entry_list:
  | entry = handler_entry { [entry] }
  | entries = handler_entry_list; entry = handler_entry { entries @ [entry] }

handler_entry:
  | attrs = attribute_list; OPERATION; name = ident; LPAREN; params = handler_param_list_opt; RPAREN; block = handler_block
    {
      let stmts, _ = block in
      ignore attrs;
      let span = make_span $startpos $endpos in
      HandlerOperation {
        handler_op_name = name;
        handler_op_params = params;
        handler_op_body = stmts;
        handler_op_span = span;
      }
    }
  | attrs = attribute_list; RETURN; value = ident; block = handler_block
    {
      let stmts, _ = block in
      ignore attrs;
      let span = make_span $startpos $endpos in
      HandlerReturn {
        handler_return_name = value;
        handler_return_body = stmts;
        handler_return_span = span;
      }
    }

handler_param_list_opt:
  | (* empty *) { [] }
  | params = param_list { params }

handler_block:
  | LBRACE; stmts = stmt_list; RBRACE
    {
      let span = make_span $startpos $endpos in
      (stmts, span)
    }

(* ========== 式 ========== *)

expr:
  | e = expr_base { e }
  | e = pipe_expr { e }

expr_base:
  | e = postfix_expr { e }
  | e = perform_expr { e }
  | e = do_expr { e }
  | e = handle_expr { e }
  | e = binary_expr { e }
  | e = unary_expr { e }
  | e = if_expr { e }
  | e = lambda_expr { e }
  | e = match_expr { e }
  | e = while_expr { e }
  | e = for_expr { e }
  | e = loop_expr { e }
  | e = continue_expr { e }
  | e = return_expr { e }
  | e = defer_expr { e }
  | e = unsafe_expr { e }

primary_expr:
  | lit = literal
    {
      let span = make_span $startpos $endpos in
      make_expr (Literal lit) span
    }
  | id = ident
    {
      make_expr (Var id) id.span
    }
  | e = block_expr { e }
  | LPAREN; first = expr; COMMA; rest = tuple_expr_rest; RPAREN
    {
      let elements = first :: rest in
      let span = make_span $startpos $endpos in
      make_expr (Literal (Tuple elements)) span
    }
  | LPAREN; e = expr; RPAREN { e }

literal:
  | i = INT { Int (fst i, snd i) }
  | f = FLOAT { Float f }
  | c = CHAR { Char c }
  | s = STRING { String (fst s, snd s) }
  | TRUE { Bool true }
  | FALSE { Bool false }
  | LPAREN; RPAREN { Unit }
  | LBRACKET; elements = expr_list_opt; RBRACKET { Array elements }
  | LBRACE; fields = record_field_list_opt; RBRACE { Record fields }

(* 後置演算子（関数呼び出し、フィールドアクセス、インデックスなど）
 * Menhir は左再帰を処理できるので、postfix_expr を左再帰で構築 *)
postfix_expr:
  | e = primary_expr { e }
  | func = postfix_expr; LPAREN; args = arg_list_opt; RPAREN
    {
      let span = merge_span func.expr_span (make_span $endpos $endpos) in
      make_expr (Call (func, args)) span
    }
  | target = postfix_expr; DOT; field = ident
    {
      let span = make_span $startpos $endpos in
      make_expr (FieldAccess (target, field)) span
    }
  | target = postfix_expr; DOT; index_lit = INT
    {
      let index = tuple_index_from_literal index_lit in
      let span = make_span $startpos $endpos in
      make_expr (TupleAccess (target, index)) span
    }
  | target = postfix_expr; LBRACKET; idx = expr; RBRACKET
    {
      let span = make_span $startpos $endpos in
      make_expr (Index (target, idx)) span
    }
  | target = postfix_expr; QUESTION
    {
      let span = make_span $startpos $endpos in
      make_expr (Propagate target) span
    }

perform_expr:
  | PERFORM; target = effect_target; LPAREN; args = arg_list_opt; RPAREN
    {
      let span = make_span $startpos $endpos in
      if not (experimental_effects_enabled ()) then
        raise (Experimental_effects_disabled ($startpos, $endpos));
      let call = make_effect_call PerformKeyword target args in
      make_expr (PerformCall call) span
    }

do_expr:
  | DO; target = effect_target; LPAREN; args = arg_list_opt; RPAREN
    {
      let span = make_span $startpos $endpos in
      if not (experimental_effects_enabled ()) then
        raise (Experimental_effects_disabled ($startpos, $endpos));
      let call = make_effect_call DoKeyword target args in
      make_expr (PerformCall call) span
    }

handle_expr:
  | HANDLE; target = expr; WITH; handler = handler_literal_expr
    {
      let span = make_span $startpos $endpos in
      if not (experimental_effects_enabled ()) then
        raise (Experimental_effects_disabled ($startpos, $endpos));
      let node = { handle_target = target; handle_handler = handler } in
      make_expr (Handle node) span
    }

handler_literal_expr:
  | HANDLER; decl = handler_decl
    {
      let span = make_span $startpos $endpos in
      { decl with handler_span = span }
    }

effect_target:
  | path = module_path; DOT; op = ident
    {
      let span = make_span $startpos $endpos in
      make_effect_reference_from_path path op span
    }

arg_list_opt:
  | (* empty *) { [] }
  | args = arg_list { args }

arg_list:
  | arg = arg { [arg] }
  | args = arg_list; COMMA; arg = arg { args @ [arg] }

arg:
  | e = expr { PosArg e }
  | id = ident; EQ; e = expr { NamedArg (id, e) }

binary_expr:
  | lhs = expr; PLUS; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Add, lhs, rhs)) span
    }
  | lhs = expr; MINUS; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Sub, lhs, rhs)) span
    }
  | lhs = expr; STAR; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Mul, lhs, rhs)) span
    }
  | lhs = expr; SLASH; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Div, lhs, rhs)) span
    }
  | lhs = expr; PERCENT; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Mod, lhs, rhs)) span
    }
  | lhs = expr; POW; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Pow, lhs, rhs)) span
    }
  | lhs = expr; EQEQ; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Eq, lhs, rhs)) span
    }
  | lhs = expr; NE; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Ne, lhs, rhs)) span
    }
  | lhs = expr; LT; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Lt, lhs, rhs)) span
    }
  | lhs = expr; LE; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Le, lhs, rhs)) span
    }
  | lhs = expr; GT; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Gt, lhs, rhs)) span
    }
  | lhs = expr; GE; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Ge, lhs, rhs)) span
    }
  | lhs = expr; AND; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (And, lhs, rhs)) span
    }
  | lhs = expr; OR; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Binary (Or, lhs, rhs)) span
    }

unary_expr:
  | NOT; e = expr %prec UNOT
    {
      let span = make_span $startpos $endpos in
      make_expr (Unary (Not, e)) span
    }
  | MINUS; e = expr %prec UMINUS
    {
      let span = make_span $startpos $endpos in
      make_expr (Unary (Neg, e)) span
    }

pipe_expr:
  | lhs = expr; PIPE; rhs = expr
    {
      let span = merge_span lhs.expr_span rhs.expr_span in
      make_expr (Pipe (lhs, rhs)) span
    }

if_expr:
  | IF; cond = expr; THEN; then_br = expr; else_br = else_branch_opt
    {
      let span = make_span $startpos $endpos in
      make_expr (If (cond, then_br, else_br)) span
    }

else_branch_opt:
  | (* empty *) { None }
  | ELSE; e = expr { Some e }

lambda_expr:
  | BAR; params = lambda_param_list_opt; BAR; ret = return_type_opt; body = lambda_body
    {
      let span = make_span $startpos $endpos in
      make_expr (Lambda (params, ret, body)) span
    }

lambda_param_list_opt:
  | (* empty *) { [] }
  | params = lambda_param_list { params }

lambda_param_list:
  | p = param { [p] }
  | ps = lambda_param_list; COMMA; p = param { ps @ [p] }

lambda_body:
  | block = block_expr { block }
  | e = expr { e }

match_expr:
  | MATCH; scrutinee = expr; WITH; arms = match_arm_list
    {
      let span = make_span $startpos $endpos in
      make_expr (Match (scrutinee, arms)) span
    }

match_arm_list:
  | arm = match_arm { [arm] }
  | arms = match_arm_list; arm = match_arm { arms @ [arm] }

match_arm:
  | BAR; pat = pattern; guard = match_guard_opt; ARROW; body = expr
    {
      let span = make_span $startpos $endpos in
      { arm_pattern = pat; arm_guard = guard; arm_body = body; arm_span = span }
    }

match_guard_opt:
  | (* empty *) { None }
  | IF; e = expr { Some e }

while_expr:
  | WHILE; cond = expr; body = block_expr
    {
      let span = make_span $startpos $endpos in
      make_expr (While (cond, body)) span
    }

for_expr:
  | FOR; pat = pattern; IN; source = expr; body = block_expr
    {
      let span = make_span $startpos $endpos in
      make_expr (For (pat, source, body)) span
    }

loop_expr:
  | LOOP; body = block_expr
    {
      let span = make_span $startpos $endpos in
      make_expr (Loop body) span
    }

continue_expr:
  | CONTINUE
    {
      let span = make_span $startpos $endpos in
      make_expr Continue span
    }

return_expr:
  | RETURN; value = expr
    {
      let span = make_span $startpos $endpos in
      make_expr (Return (Some value)) span
    }
  | RETURN
    {
      let span = make_span $startpos $endpos in
      make_expr (Return None) span
    }

defer_expr:
  | DEFER; e = expr
    {
      let span = make_span $startpos $endpos in
      make_expr (Defer e) span
    }

unsafe_expr:
  | UNSAFE; body = block_expr
    {
      let span = make_span $startpos $endpos in
      make_expr (Unsafe body) span
    }

block_expr:
  | LBRACE; stmts = stmt_list; RBRACE
    {
      let span = make_span $startpos $endpos in
      make_expr (Block stmts) span
    }

(* ========== 文 ========== *)

block_stmt:
  | LBRACE; stmts = stmt_list; RBRACE { stmts }

stmt_list:
  | (* empty *) { [] }
  | stmts = stmt_list; s = stmt { stmts @ [s] }

stmt:
  | d = decl; SEMICOLON { DeclStmt d }
  | d = decl { DeclStmt d }  (* 最後の宣言はセミコロン省略可 *)
  | lvalue = postfix_expr; COLONEQ; rvalue = expr; SEMICOLON { AssignStmt (lvalue, rvalue) }
  | lvalue = postfix_expr; COLONEQ; rvalue = expr { AssignStmt (lvalue, rvalue) }  (* セミコロン省略可 *)
  | DEFER; value = expr; SEMICOLON { DeferStmt value }
  | e = expr; SEMICOLON { ExprStmt e }
  | e = expr { ExprStmt e }  (* 最後の式はセミコロン省略可 *)

(* ========== パターン ========== *)

pattern:
  | lit = literal
    {
      let span = make_span $startpos $endpos in
      make_pattern (PatLiteral lit) span
    }
  | id = lower_ident
    {
      make_pattern (PatVar id) id.span
    }
  | ctor = upper_ident
    {
      make_pattern (PatConstructor (ctor, [])) ctor.span
    }
  | UNDERSCORE
    {
      let span = make_span $startpos $endpos in
      make_pattern PatWildcard span
    }
  | LPAREN; pat = pattern; RPAREN { pat }
  | LPAREN; first = pattern; COMMA; rest = pattern_list; RPAREN
    {
      let patterns = first :: rest in
      let span = make_span $startpos $endpos in
      make_pattern (PatTuple patterns) span
    }
  | name = upper_ident; LPAREN; args = pattern_arg_list_opt; RPAREN
    {
      let span = make_span $startpos $endpos in
      make_pattern (PatConstructor (name, args)) span
    }
  | name = lower_ident; LPAREN; args = pattern_arg_list_opt; RPAREN
    {
      let span = make_span $startpos $endpos in
      make_pattern (PatConstructor (name, args)) span
    }
  | head = ident; DOT; rest = separated_nonempty_list(DOT, ident)
    {
      let ids = head :: rest in
      let span = make_span $startpos $endpos in
      match List.rev ids with
      | ctor :: rev_prefix ->
          let ctor_ident =
            if rev_prefix = [] then ctor
            else
              let prefix = List.rev rev_prefix |> List.map (fun id -> id.name) in
              make_qualified_ident (prefix @ [ctor.name]) span
          in
          make_pattern (PatConstructor (ctor_ident, [])) span
      | [] -> assert false
    }
  | head = ident; DOT; rest = separated_nonempty_list(DOT, ident); LPAREN; args = pattern_arg_list_opt; RPAREN
    {
      let ids = head :: rest in
      let span = make_span $startpos $endpos in
      match List.rev ids with
      | ctor :: rev_prefix ->
          let ctor_ident =
            if rev_prefix = [] then ctor
            else
              let prefix = List.rev rev_prefix |> List.map (fun id -> id.name) in
              make_qualified_ident (prefix @ [ctor.name]) span
          in
          make_pattern (PatConstructor (ctor_ident, args)) span
      | [] -> assert false
    }
  | LBRACE; body = record_pattern_body; RBRACE
    {
      let fields, has_rest = body in
      let span = make_span $startpos $endpos in
      make_pattern (PatRecord (fields, has_rest)) span
    }

pattern_list:
  | p = pattern { [p] }
  | ps = pattern_list; COMMA; p = pattern { ps @ [p] }

pattern_arg_list_opt:
  | (* empty *) { [] }
  | args = pattern_arg_list { args }

pattern_arg_list:
  | p = pattern { [p] }
  | ps = pattern_arg_list; COMMA; p = pattern { ps @ [p] }

record_pattern_body:
  | DOTDOT { ([], true) }
  | entries = record_pattern_entry_list; rest = record_pattern_rest_opt { (entries, rest) }

record_pattern_entry_list:
  | entry = record_pattern_entry { [entry] }
  | entries = record_pattern_entry_list; COMMA; entry = record_pattern_entry { entries @ [entry] }

record_pattern_entry:
  | name = ident; COLON; pat = pattern { (name, Some pat) }
  | name = ident { (name, None) }

record_pattern_rest_opt:
  | (* empty *) { false }
  | COMMA; DOTDOT { true }

(* ========== 型注釈 ========== *)

type_annot_opt:
  | (* empty *) { None }
  | COLON; ty = type_annot { Some ty }

type_annot:
  | ty = type_primary { ty }
  | lhs = type_primary; ARROW; rhs = type_annot
    {
      let span = make_span $startpos $endpos in
      make_type (TyFn ([lhs], rhs)) span
    }

type_primary:
  | id = ident; args = generic_args_opt
    {
      let span = make_span $startpos $endpos in
      match args with
      | [] -> make_type (TyIdent id) span
      | _ -> make_type (TyApp (id, args)) span
    }
  | LPAREN; ty = type_annot; RPAREN { ty }
  | LPAREN; first = type_annot; COMMA; rest = type_arg_list; RPAREN
    {
      let span = make_span $startpos $endpos in
      make_type (TyTuple (first :: rest)) span
    }
  | LBRACE; fields = type_record_fields; RBRACE
    {
      let span = make_span $startpos $endpos in
      make_type (TyRecord fields) span
    }

type_record_fields:
  | field = type_record_field { [field] }
  | fields = type_record_fields; COMMA; field = type_record_field { fields @ [field] }

type_record_field:
  | name = ident; COLON; ty = type_annot { (name, ty) }

(* ========== ヘルパー ========== *)

lower_ident:
  | id = IDENT
    {
      let span = make_span $startpos $endpos in
      make_ident id span
    }
  | SELF
    {
      let span = make_span $startpos $endpos in
      make_ident "self" span
    }

upper_ident:
  | id = UPPER_IDENT
    {
      let span = make_span $startpos $endpos in
      make_ident id span
    }

ident:
  | id = lower_ident { id }
  | id = upper_ident { id }

ident_list:
  | id = ident { [id] }
  | ids = ident_list; DOT; id = ident { ids @ [id] }

expr_list:
  | e = expr { [e] }
  | es = expr_list; COMMA; e = expr { es @ [e] }

expr_list_opt:
  | (* empty *) { [] }
  | exprs = expr_list { exprs }

record_field_list_opt:
  | (* empty *) { [] }
  | fields = record_field_list { fields }

record_field_list:
  | field = record_field { [field] }
  | fields = record_field_list; COMMA; field = record_field { fields @ [field] }

record_field:
  | name = ident; COLON; value = expr { (name, value) }

tuple_expr_rest:
  | e = expr { [e] }
  | rest = tuple_expr_rest; COMMA; e = expr { rest @ [e] }

(* ========== 仮トークン (未実装部分) ========== *)

%%
