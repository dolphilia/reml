(* Effect_profile — Shared effect metadata utilities (Phase 2-2)
 *
 * Parser / Typer / Core IR / Runtime で共通利用する効果タグと Stage 要件の定義。
 * 仕様書 docs/spec/1-3-effects-safety.md および設計ノート
 * compiler/ocaml/docs/effect-system-design-note.md を参照。
 *)

open Ast

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

(* ========== Effect Profile ========== *)

type profile = {
  effect_set : set;
  stage_requirement : stage_requirement;
  source_span : span;
  source_name : string option;
  resolved_stage : stage_id option;
  resolved_capability : string option;
}

let make_profile ?source_name ?resolved_stage ?resolved_capability
    ~stage_requirement ~effect_set ~span () =
  {
    effect_set;
    stage_requirement;
    source_span = span;
    source_name;
    resolved_stage;
    resolved_capability;
  }

let default_profile ?source_name ~span () =
  make_profile ?source_name ~stage_requirement:default_stage_requirement
    ~effect_set:empty_set ~span ()

let profile_of_ast ?source_name (node : effect_profile_node) =
  let declared = tags_of_idents node.effect_declared in
  let residual =
    match node.effect_residual with
    | [] -> declared
    | entries -> tags_of_idents entries
  in
  let effect_set = set_of_ast_nodes ~declared ~residual in
  let stage_requirement =
    match node.effect_stage with
    | Some annot -> stage_requirement_of_annot annot
    | None -> default_stage_requirement
  in
  make_profile ?source_name ~stage_requirement ~effect_set
    ~span:node.effect_span ()
