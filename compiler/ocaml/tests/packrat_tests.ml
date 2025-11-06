(* packrat_tests.ml — Core Parse コンビネーターと Packrat 計測の検証
 *
 * Phase 2-5 PARSER-003 Step5: `Core_parse` 由来のメタデータと Packrat 指標を
 * テストで保証し、CLI/CI のゴールデン・メトリクス整備へ接続する。
 *)

open Parser_driver

module Extensions = Diagnostic.Extensions

let pass desc = Printf.printf "✓ %s\n" desc

let fail desc msg =
  Printf.printf "✗ %s: %s\n" desc msg;
  exit 1

let parse_with_error ?config input =
  let result =
    match config with
    | Some cfg -> Parser_driver.run_string ~config:cfg input
    | None -> Parser_driver.run_string input
  in
  match result.diagnostics with
  | diag :: _ -> (result, diag)
  | [] -> fail "parser diagnostics presence" "構文エラー診断が生成されませんでした"

let find_assoc key fields =
  match List.assoc_opt key fields with
  | Some value -> value
  | None ->
      fail "parser extensions" (Printf.sprintf "キー %s が見つかりません" key)

let expect_string_field desc json expected =
  match json with
  | `String value when String.equal value expected -> ()
  | `String value ->
      fail desc
        (Printf.sprintf "期待値 %s に対して %s が返されました" expected value)
  | other ->
      fail desc
        (Printf.sprintf "文字列が期待されましたが %s が返されました"
           (Yojson.Basic.to_string other))

let expect_int_field desc json expected =
  match json with
  | `Int value when value = expected -> ()
  | `Int value ->
      fail desc
        (Printf.sprintf "期待値 %d に対して %d が返されました" expected value)
  | other ->
      fail desc
        (Printf.sprintf "整数が期待されましたが %s が返されました"
           (Yojson.Basic.to_string other))

let test_core_rule_metadata () =
  let desc = "Core_parse.rule が parser_id と監査メタデータを付与する" in
  let _result, diag =
    parse_with_error "fn missing(x, y = x + y"
  in
  let parse_extension =
    match Extensions.get "parse" diag.Diagnostic.extensions with
    | Some (`Assoc fields) -> fields
    | _ ->
        fail desc "extensions.parse が期待した形式 (`Assoc`) ではありません"
  in
  let parser_id =
    match find_assoc "parser_id" parse_extension with
    | `Assoc fields -> fields
    | other ->
        fail desc
          (Printf.sprintf "parser_id がオブジェクトではありません: %s"
             (Yojson.Basic.to_string other))
  in
  expect_string_field desc (find_assoc "namespace" parser_id) "menhir";
  expect_string_field desc (find_assoc "name" parser_id) "compilation_unit";
  expect_int_field desc (find_assoc "ordinal" parser_id) 0;
  expect_string_field desc (find_assoc "origin" parser_id) "static";
  ignore (find_assoc "fingerprint" parser_id);
  let audit_metadata = diag.Diagnostic.audit_metadata in
  let expect_meta key expected =
    match Extensions.get key audit_metadata with
    | Some json -> expect_string_field desc json expected
    | None ->
        fail desc (Printf.sprintf "監査メタデータ %s が存在しません" key)
  in
  expect_meta "parser.core.rule.namespace" "menhir";
  expect_meta "parser.core.rule.name" "compilation_unit";
  (match Extensions.get "parser.core.rule.ordinal" audit_metadata with
  | Some json -> expect_int_field desc json 0
  | None ->
      fail desc "parser.core.rule.ordinal が監査メタデータに存在しません");
  ignore (Extensions.get "parser.core.rule.fingerprint" audit_metadata);
  (match Extensions.get "parser" audit_metadata with
  | Some (`Assoc parser_fields) -> (
      match List.assoc_opt "core" parser_fields with
      | Some (`Assoc core_fields) -> (
          match List.assoc_opt "rule" core_fields with
          | Some (`Assoc rule_fields) ->
              expect_string_field desc
                (find_assoc "namespace" rule_fields)
                "menhir";
              expect_string_field desc (find_assoc "name" rule_fields)
                "compilation_unit";
              expect_int_field desc (find_assoc "ordinal" rule_fields) 0;
              expect_string_field desc (find_assoc "origin" rule_fields)
                "static";
              ignore (find_assoc "fingerprint" rule_fields)
          | Some other ->
              fail desc
                (Printf.sprintf
                   "parser.core.rule がオブジェクトではありません: %s"
                   (Yojson.Basic.to_string other))
          | None -> fail desc "parser.core.rule が欠落しています")
      | Some other ->
          fail desc
            (Printf.sprintf "parser.core がオブジェクトではありません: %s"
               (Yojson.Basic.to_string other))
      | None -> fail desc "parser.core が欠落しています")
  | Some other ->
      fail desc
        (Printf.sprintf "parser メタデータがオブジェクトではありません: %s"
           (Yojson.Basic.to_string other))
  | None -> fail desc "parser メタデータが存在しません");
  pass desc

let test_packrat_stats_disabled () =
  let desc = "Packrat 無効時は Packrat 統計が出力されない" in
  let result, _ = parse_with_error "fn main() = { 1 + }" in
  match result.packrat_stats with
  | None -> pass desc
  | Some (queries, hits) ->
      fail desc
        (Printf.sprintf
           "packrat_stats が設定されました (queries=%d hits=%d)"
           queries hits)

let test_packrat_stats_enabled () =
  let desc = "Packrat 有効時にヒット率が閾値を超える" in
  let config = { Parser_run_config.default with packrat = true } in
  let result, _ = parse_with_error ~config "fn main() = { 1 + }" in
  match result.packrat_stats with
  | None -> fail desc "packrat_stats が None でした"
  | Some (queries, hits) ->
      if queries <= 0 then
        fail desc "packrat クエリ数が 0 でした"
      else
        let ratio = float_of_int hits /. float_of_int queries in
        if ratio >= 0.85 then pass desc
        else
          fail desc
            (Printf.sprintf "ヒット率が低すぎます (queries=%d hits=%d)"
               queries hits)

let () =
  let tests =
    [
      test_core_rule_metadata;
      test_packrat_stats_disabled;
      test_packrat_stats_enabled;
    ]
  in
  List.iter (fun fn -> fn ()) tests
