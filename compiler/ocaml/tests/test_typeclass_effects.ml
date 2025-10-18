(* Test_typeclass_effects — Ensure effect constraints stay independent from typeclass dictionaries *)

open Types
open Constraint_solver
open Effect_profile

let names_of_tags tags =
  tags
  |> List.map (fun tag -> tag.effect_name |> String.lowercase_ascii)
  |> List.sort_uniq String.compare

let show_names names = "[" ^ String.concat ", " names ^ "]"

let () =
  (* リセットして環境を初期化 *)
  Type_inference.reset_impl_registry ();
  reset_effect_constraints ();

  (* モックの効果プロファイルを登録 *)
  let span = Ast.dummy_span in
  let mk_tag name = { effect_name = name; effect_span = span } in
  let effect_set = { declared = [ mk_tag "io" ]; residual = [ mk_tag "io" ] } in
  let profile =
    make_profile ~stage_requirement:(StageAtLeast Stable) ~effect_set ~span
      ~source_name:"effectful_sum" ()
  in
  record_effect_profile ~symbol:"effectful_sum" profile;

  (* 効果テーブルから関数の効果集合を取得 *)
  let entry =
    match resolve_effect_profile ~symbol:"effectful_sum" with
    | Some entry -> entry
    | None -> failwith "effect profile not registered for effectful_sum"
  in

  let declared = names_of_tags entry.effect_set.declared in
  if List.mem "io" declared then
    Printf.printf "✓ effectful_sum declared effects %s\n" (show_names declared)
  else
    failwith
      ("expected declared effects to contain io, got " ^ show_names declared);

  (match entry.stage_requirement with
  | StageAtLeast stage when stage = Stable ->
      Printf.printf "✓ effectful_sum stage requirement >= stable\n"
  | StageExact stage when stage = Stable ->
      Printf.printf "✓ effectful_sum stage requirement == stable\n"
  | StageExact stage ->
      failwith
        (Printf.sprintf "unexpected stage requirement (exact %s)"
           (stage_id_to_string stage))
  | StageAtLeast stage ->
      failwith
        (Printf.sprintf "unexpected stage requirement (at least %s)"
           (stage_id_to_string stage)));

  (* 型クラス制約の解決が効果テーブルと独立であることを確認 *)
  let eq_constraint =
    {
      trait_name = "Eq";
      type_args = [ ty_i64 ];
      constraint_span = Ast.dummy_span;
    }
  in
  (match solve_constraints (Impl_registry.empty ()) [ eq_constraint ] with
  | Ok _ -> Printf.printf "✓ constraint solver resolved Eq<i64>\n"
  | Error _ -> failwith "constraint solver failed for Eq<i64>");

  (* 再取得して効果情報が変わっていないことを検証 *)
  let entry_after =
    match resolve_effect_profile ~symbol:"effectful_sum" with
    | Some entry -> entry
    | None -> failwith "effect profile missing after constraint solve"
  in
  if effect_set_includes ~super:entry_after.effect_set ~sub:entry.effect_set
  then Printf.printf "✓ effect profile unchanged after constraint solving\n"
  else failwith "effect profile was unexpectedly mutated"
