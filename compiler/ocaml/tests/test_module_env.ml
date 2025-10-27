(* test_module_env.ml — Module_env ユニットテスト
 *
 * SYNTAX-002 S4: `use` 多段ネストの束縛展開を検証する。
 *)

open Module_env
open Ast

let parse_uses_or_fail desc source =
  match Parser_driver.parse_string source with
  | Ok cu -> cu.uses
  | Error diag ->
      Printf.printf "✗ %s: parse failed\n%s\n" desc
        (Diagnostic.to_string diag);
      exit 1

let binding_signature binding =
  ( binding.binding_local.name,
    string_of_module_path binding.binding_path,
    Option.map (fun id -> id.name) binding.binding_source,
    binding.binding_is_pub )

let pp_signature (local, path, source, is_pub) =
  let src =
    match source with
    | Some s -> s
    | None -> "<none>"
  in
  Printf.sprintf "{ local=%s; path=%s; source=%s; pub=%b }" local path src
    is_pub

let expect_bindings desc source expected =
  let uses = parse_uses_or_fail desc source in
  let actual =
    flatten_use_decls uses |> List.map binding_signature
  in
  if actual = expected then Printf.printf "✓ %s\n" desc
  else (
    Printf.printf "✗ %s: bindings mismatch\n" desc;
    Printf.printf "  expected:\n";
    List.iter (fun sig_ -> Printf.printf "    %s\n" (pp_signature sig_))
      expected;
    Printf.printf "  actual:\n";
    List.iter (fun sig_ -> Printf.printf "    %s\n" (pp_signature sig_))
      actual;
    exit 1)

let () =
  expect_bindings "Module_env: simple path"
    "use ::Core.Parse"
    [ ("Parse", "::Core.Parse", Some "Parse", false) ];

  expect_bindings "Module_env: alias path"
    "use ::Core.Parse as Parser"
    [ ("Parser", "::Core.Parse", Some "Parse", false) ];

  expect_bindings "Module_env: nested braces"
    "use Core.Parse.{Lex, Op.{Infix, Prefix}}"
    [
      ("Lex", "Core.Parse.Lex", Some "Lex", false);
      ("Infix", "Core.Parse.Op.Infix", Some "Infix", false);
      ("Prefix", "Core.Parse.Op.Prefix", Some "Prefix", false);
    ];

  expect_bindings "Module_env: pub alias with nested"
    "pub use Core.Parse.{Op as Operator, Op.{Infix}}"
    [
      ("Operator", "Core.Parse.Op", Some "Op", true);
      ("Infix", "Core.Parse.Op.Infix", Some "Infix", true);
    ]
