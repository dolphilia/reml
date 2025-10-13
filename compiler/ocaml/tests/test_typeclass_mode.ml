(* test_typeclass_mode.ml — 型クラス戦略モードと PoC パスのテスト
 *
 * Phase 2 M1 で導入する `--typeclass-mode` フラグと
 * Core_ir.Monomorphize_poc パスの骨格実装を検証する。
 *)

open Types
open Core_ir.Ir

let assert_bool msg cond =
  if not cond then failwith (Printf.sprintf "Assertion failed: %s" msg)

let assert_equal msg expected actual =
  if expected <> actual then
    failwith
      (Printf.sprintf "Assertion failed: %s (expected: %s, actual: %s)" msg
         expected actual)

let string_of_mode = function
  | Cli.Options.TypeclassDictionary -> "dictionary"
  | Cli.Options.TypeclassMonomorph -> "monomorph"
  | Cli.Options.TypeclassBoth -> "both"

let test_parse_typeclass_mode () =
  (* 1. 既定値は dictionary *)
  let argv_default = [| "remlc"; "input.reml" |] in
  let default_mode =
    match Cli.Options.parse_args argv_default with
    | Ok opts -> opts.Cli.Options.typeclass_mode
    | Error msg -> failwith msg
  in
  assert_equal "default mode" "dictionary" (string_of_mode default_mode);

  (* 2. monomorph 指定が解釈される *)
  let argv_mono =
    [| "remlc"; "--typeclass-mode"; "monomorph"; "input.reml" |]
  in
  let mono_mode =
    match Cli.Options.parse_args argv_mono with
    | Ok opts -> opts.Cli.Options.typeclass_mode
    | Error msg -> failwith msg
  in
  assert_equal "monomorph mode" "monomorph" (string_of_mode mono_mode);

  (* 3. 無効な値は警告の上で dictionary にフォールバックする *)
  let argv_invalid =
    [| "remlc"; "--typeclass-mode"; "unknown"; "input.reml" |]
  in
  let invalid_mode =
    match Cli.Options.parse_args argv_invalid with
    | Ok opts -> opts.Cli.Options.typeclass_mode
    | Error msg -> failwith msg
  in
  assert_equal "invalid fallback" "dictionary"
    (string_of_mode invalid_mode);
  Printf.printf "✓ test_parse_typeclass_mode passed\n%!"

let has_entry trait_name ty entries =
  List.exists
    (fun instance ->
      instance.Type_env.Monomorph_registry.trait_name = trait_name
      &&
      match instance.Type_env.Monomorph_registry.type_args with
      | [ ty' ] -> type_equal ty ty'
      | _ -> false)
    entries

let test_monomorphize_summary () =
  let empty_module =
    {
      module_name = "test";
      type_defs = [];
      global_defs = [];
      function_defs = [];
    }
  in

  (* 1. 辞書モードではサマリーがリセットされる *)
  Type_env.Monomorph_registry.reset ();
  Type_env.Monomorph_registry.record
    Type_env.Monomorph_registry.
      { trait_name = "Eq"; type_args = [ ty_i64 ]; methods = [] };
  ignore
    (Core_ir.Monomorphize_poc.apply
       ~mode:Core_ir.Monomorphize_poc.UseDictionary empty_module);
  assert_bool "dictionary mode clears summary"
    (Core_ir.Monomorphize_poc.Summary.entries () = []);

  (* 2. モノモルフィゼーションモードで記録される *)
  Type_env.Monomorph_registry.reset ();
  Type_env.Monomorph_registry.record
    Type_env.Monomorph_registry.
      { trait_name = "Eq"; type_args = [ ty_i64 ]; methods = [] };
  ignore
    (Core_ir.Monomorphize_poc.apply
       ~mode:Core_ir.Monomorphize_poc.UseMonomorph empty_module);
  let entries = Core_ir.Monomorphize_poc.Summary.entries () in
  assert_bool "monomorph entries recorded"
    (List.length entries = 1
    && has_entry "Eq" ty_i64 entries);
  assert_bool "mode stored as monomorph"
    (Core_ir.Monomorphize_poc.Summary.mode ()
    = Core_ir.Monomorphize_poc.UseMonomorph);

  (* 3. both モードでも同様に記録される *)
  Type_env.Monomorph_registry.reset ();
  Type_env.Monomorph_registry.record
    Type_env.Monomorph_registry.
      { trait_name = "Ord"; type_args = [ ty_string ]; methods = [] };
  ignore
    (Core_ir.Monomorphize_poc.apply
       ~mode:Core_ir.Monomorphize_poc.UseBoth empty_module);
  let both_entries = Core_ir.Monomorphize_poc.Summary.entries () in
  assert_bool "both mode recorded"
    (List.length both_entries = 1
    && has_entry "Ord" ty_string both_entries);
  assert_bool "mode stored as both"
    (Core_ir.Monomorphize_poc.Summary.mode ()
    = Core_ir.Monomorphize_poc.UseBoth);

  Printf.printf "✓ test_monomorphize_summary passed\n%!"

let test_wrapper_generation () =
  let empty_module =
    {
      module_name = "test";
      type_defs = [];
      global_defs = [];
      function_defs = [];
    }
  in
  Type_env.Monomorph_registry.reset ();
  Type_env.Monomorph_registry.record
    Type_env.Monomorph_registry.
      {
        trait_name = "Eq";
        type_args = [ ty_i64 ];
        methods = [ ("eq", "__Eq_i64_eq") ];
      };
  let result =
    Core_ir.Monomorphize_poc.apply
      ~mode:Core_ir.Monomorphize_poc.UseMonomorph empty_module
  in
  let wrapper =
    List.find_opt
      (fun fn -> String.equal fn.fn_name "__Eq_i64_eq_mono")
      result.function_defs
  in
  (match wrapper with
  | Some fn ->
      assert_bool "wrapper has two params"
        (List.length fn.fn_params = 2);
      assert_bool "wrapper returns Bool"
        (type_equal fn.fn_return_ty ty_bool)
  | None -> failwith "wrapper was not generated");
  Printf.printf "✓ test_wrapper_generation passed\n%!"

let () =
  test_parse_typeclass_mode ();
  test_monomorphize_summary ();
  test_wrapper_generation ()
