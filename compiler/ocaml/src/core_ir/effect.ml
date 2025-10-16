(* Core_ir.Effect — Effect metadata helpers for Core IR (Phase 2-2)
 *
 * 効果タグと Stage 要件を共有するための小さなユーティリティ。
 * Parser / Typer / Core IR / Runtime で同じ構造を扱えるよう、
 * 型と基本操作のみを提供する。
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

type stage_requirement =
  | StageExact of stage_id
  | StageAtLeast of stage_id

let starts_with ~prefix value =
  let plen = String.length prefix in
  let vlen = String.length value in
  vlen >= plen && String.sub value 0 plen = prefix

let stage_requirement_of_string (value : string) =
  let trimmed = String.trim value in
  let lowered = String.lowercase_ascii trimmed in
  let at_least_prefix = "at_least:" in
  let exact_prefix = "exact:" in
  if starts_with ~prefix:at_least_prefix lowered then
    let suffix =
      String.sub trimmed (String.length at_least_prefix)
        (String.length trimmed - String.length at_least_prefix)
    in
    StageAtLeast (stage_id_of_string suffix)
  else if starts_with ~prefix:exact_prefix lowered then
    let suffix =
      String.sub trimmed (String.length exact_prefix)
        (String.length trimmed - String.length exact_prefix)
    in
    StageExact (stage_id_of_string suffix)
  else
    StageExact (stage_id_of_string trimmed)

let stage_requirement_to_string = function
  | StageExact stage -> stage_id_to_string stage
  | StageAtLeast stage -> "at_least:" ^ stage_id_to_string stage

let stage_requirement_satisfied ~requirement ~actual =
  match requirement with
  | StageExact expected -> compare_stage_id expected actual = 0
  | StageAtLeast minimum -> compare_stage_id minimum actual <= 0

(* ========== 効果タグ・効果集合 ========== *)

type tag = {
  effect_name : string;
  effect_span : span;
}

type set = {
  declared : tag list;
  residual : tag list;
}

let empty = { declared = []; residual = [] }

let is_empty { declared; residual } = declared = [] && residual = []

let normalize_effect_name name = String.lowercase_ascii name

let rec contains_tag name = function
  | [] -> false
  | tag :: rest ->
      if String.equal (normalize_effect_name tag.effect_name) (normalize_effect_name name) then
        true
      else
        contains_tag name rest

let append_unique tag tags =
  if contains_tag tag.effect_name tags then tags else tags @ [ tag ]

let add_declared tag set = { set with declared = append_unique tag set.declared }

let add_residual tag set = { set with residual = append_unique tag set.residual }

let of_declared tags =
  List.fold_left (fun acc tag -> add_declared tag acc) empty tags

let of_residual tags =
  List.fold_left (fun acc tag -> add_residual tag acc) empty tags

let union lhs rhs =
  let add_all f tags acc = List.fold_left (fun acc tag -> f tag acc) acc tags in
  add_all add_residual rhs.residual (add_all add_declared rhs.declared lhs)
