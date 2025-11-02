(* effect_handler_poc_tests.ml — EFFECT-002 Step3 PoC 動作確認
 *
 * perform/handle 構文が型推論と効果解析パイプラインに統合されることを検証する。
 *)

open Typed_ast

module Run_config = Parser_run_config

let parse_and_infer source =
  let base_config =
    Run_config.Legacy.bridge { require_eof = true; legacy_result = true }
  in
  let config = Run_config.set_experimental_effects base_config true in
  let parse_result = Parser_driver.run_string ~config source in
  match Parser_driver.parse_result_to_legacy parse_result with
  | Result.Error diag ->
      failwith (Printf.sprintf "Parse error: %s" (Diagnostic.to_string diag))
  | Result.Ok ast -> (
      Type_inference.reset_impl_registry ();
      match Type_inference.infer_compilation_unit ast with
      | Result.Ok tast -> tast
      | Result.Error err ->
          failwith
            (Printf.sprintf "Type error: %s"
               (Type_error.string_of_error err)))

let find_function tast name =
  tast.Typed_ast.tcu_items
  |> List.find_map (fun decl ->
         match decl.tdecl_kind with
         | TFnDecl fn when String.equal fn.tfn_name.name name -> Some fn
         | _ -> None)

let residual_tags fn =
  fn.tfn_effect_profile.Effect_profile.effect_set.Effect_profile.residual
  |> List.map (fun tag ->
         String.lowercase_ascii tag.Effect_profile.effect_name)

let ensure_has_console_tag fn fn_name =
  let tags = residual_tags fn in
  if not (List.exists (fun tag -> String.equal tag "console") tags) then
    failwith
      (Printf.sprintf "%s の残余効果集合に console が含まれていません" fn_name)

let test_count = ref 0
let success_count = ref 0

let run_test name f =
  incr test_count;
  try
    f ();
    incr success_count;
    Printf.printf "✓ %s\n%!" name
  with Failure msg ->
    Printf.eprintf "✗ %s: %s\n%!" name msg;
    exit 1

let perform_source =
  {|
effect Console : io {
  operation log(msg: String): ()
}

fn perform_demo(msg: String) -> () = {
  perform Console.log(msg)
}
|}

let handle_source =
  {|
effect Console : io {
  operation log(msg: String): ()
}

fn handle_demo(msg: String) -> () = {
  handle perform Console.log(msg) with handler ConsoleHandler {
    operation log(value) { }
  }
}
|}

let test_perform_effect_residual () =
  let tast = parse_and_infer perform_source in
  match find_function tast "perform_demo" with
  | None -> failwith "perform_demo が見つかりません"
  | Some fn -> ensure_has_console_tag fn "perform_demo"

let test_handle_expression () =
  let tast = parse_and_infer handle_source in
  match find_function tast "handle_demo" with
  | None -> failwith "handle_demo が見つかりません"
  | Some fn ->
      ensure_has_console_tag fn "handle_demo";
      (match fn.tfn_body with
      | TFnBlock [ TExprStmt expr ] -> (
          match expr.texpr_kind with
          | THandle _ -> ()
          | _ -> failwith "handle_demo が handle 式を含んでいません")
      | _ -> failwith "handle_demo の本体が想定した構造ではありません")

let () =
  run_test "perform で効果タグを収集できる" test_perform_effect_residual;
  run_test "handle 式を型推論できる" test_handle_expression;
  Printf.printf "合計 %d 件中 %d 件成功\n%!" !test_count !success_count
