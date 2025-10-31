(* test_cli_diagnostics.ml — CLI 診断出力のテスト
 *
 * Phase 1-6 の開発者体験整備タスクにおいて、診断フォーマッタの動作を検証する。
 *)

let () = Unix.putenv "REMLC_FIXED_TIMESTAMP" "1970-01-01T00:00:00Z"

let project_root =
  match Sys.getenv_opt "DUNE_SOURCEROOT" with
  | Some root -> root
  | None -> Filename.dirname Sys.argv.(0)

let resolve path = Filename.concat project_root path
let golden_dir = resolve "tests/golden"

let write_actual_snapshot name content =
  let actual_dir = Filename.concat golden_dir "_actual" in
  if not (Sys.file_exists actual_dir) then Unix.mkdir actual_dir 0o755;
  let path = Filename.concat actual_dir (name ^ ".actual.json") in
  Out_channel.with_open_text path (fun oc ->
      output_string oc content;
      if content = "" || content.[String.length content - 1] <> '\n' then
        output_char oc '\n');
  path

(** テスト用の診断情報を生成 *)
let make_test_diagnostic () =
  let start_pos =
    Diagnostic.{ filename = "test.reml"; line = 2; column = 5; offset = 15 }
  in
  let end_pos =
    Diagnostic.{ filename = "test.reml"; line = 2; column = 11; offset = 21 }
  in
  Diagnostic.(
    Builder.create ~severity:Error ~domain:Type ~message:"型が一致しません"
      ~primary:{ start_pos; end_pos } ()
    |> Builder.set_primary_code "E7001"
    |> Builder.add_notes
         [ (None, "期待される型: i64"); (None, "実際の型:     String") ]
    |> Builder.build)

(** テスト用のソースコード *)
let test_source = "fn main() -> i64 =\n  let x: String = \"hello\" in\n  x + 42"

(** カラー出力のテスト *)
let test_color_output () =
  let diag = make_test_diagnostic () in

  (* カラーなしでの出力 *)
  let no_color_output =
    Cli.Diagnostic_formatter.format_diagnostic ~source:(Some test_source) ~diag
      ~color_mode:Cli.Options.Never ~include_snippet:true
  in
  assert (not (String.contains no_color_output '\027'));

  (* カラーありでの出力 *)
  let color_output =
    Cli.Diagnostic_formatter.format_diagnostic ~source:(Some test_source) ~diag
      ~color_mode:Cli.Options.Always ~include_snippet:true
  in
  assert (String.contains color_output '\027');

  (* メッセージ本体は両方に含まれる *)
  assert (Str.string_match (Str.regexp ".*型.*") no_color_output 0);
  assert (Str.string_match (Str.regexp ".*型.*") color_output 0);
  Printf.printf "✓ カラー出力テスト成功\n"

(** JSON 出力のテスト *)
let test_json_output () =
  let diag = make_test_diagnostic () in

  (* JSON 出力を生成 *)
  let json_str =
    Cli.Json_formatter.diagnostic_to_json ~mode:Cli.Options.JsonPretty diag
  in

  (* JSON としてパース可能か確認 *)
  let json = Yojson.Basic.from_string json_str in
  let diagnostics = json |> Yojson.Basic.Util.member "diagnostics" in
  let diag_list = diagnostics |> Yojson.Basic.Util.to_list in

  assert (List.length diag_list = 1);

  let first_diag = List.hd diag_list in
  let severity =
    first_diag
    |> Yojson.Basic.Util.member "severity"
    |> Yojson.Basic.Util.to_string
  in
  let code =
    first_diag |> Yojson.Basic.Util.member "code" |> Yojson.Basic.Util.to_string
  in
  let message =
    first_diag
    |> Yojson.Basic.Util.member "message"
    |> Yojson.Basic.Util.to_string
  in
  let domain =
    first_diag
    |> Yojson.Basic.Util.member "domain"
    |> Yojson.Basic.Util.to_string
  in

  assert (severity = "error");
  assert (code = "E7001");
  assert (message = "型が一致しません");
  assert (domain = "type");
  Printf.printf "✓ JSON出力テスト成功\n"

let test_other_domain_serialization () =
  let start_pos =
    Diagnostic.{ filename = "other-domain.reml"; line = 1; column = 1; offset = 0 }
  in
  let end_pos =
    Diagnostic.{ filename = "other-domain.reml"; line = 1; column = 5; offset = 4 }
  in
  let span = Diagnostic.{ start_pos; end_pos } in
  let diag =
    Diagnostic.Builder.create ~severity:Diagnostic.Warning
      ~domain:(Diagnostic.Domain.other "plugin_bundle")
      ~timestamp:"1970-01-01T00:00:00Z"
      ~message:"プラグイン診断のテスト" ~primary:span ()
    |> Diagnostic.Builder.set_primary_code "demo.domain.plugin"
    |> Diagnostic.Builder.add_note "domain other シリアライズ確認"
    |> Diagnostic.Builder.build
  in
  let json_str =
    Cli.Json_formatter.diagnostic_to_json ~mode:Cli.Options.JsonPretty diag
  in
  let json = Yojson.Basic.from_string json_str in
  let diag_json =
    json |> Yojson.Basic.Util.member "diagnostics"
    |> Yojson.Basic.Util.to_list |> List.hd
  in
  let domain =
    Yojson.Basic.Util.(diag_json |> member "domain" |> to_string)
  in
  assert (domain = "other");
  let extensions =
    Yojson.Basic.Util.(diag_json |> member "extensions" |> to_assoc)
  in
  let other_value =
    match List.assoc_opt "domain.other" extensions with
    | Some value -> Yojson.Basic.Util.to_string value
    | None ->
        Printf.printf "%s\n" json_str;
        failwith "domain.other extension was not found"
  in
  assert (other_value = "plugin_bundle");
  Printf.printf "✓ Other ドメインのシリアライズテスト成功\n"

let test_parser_expectation_snapshot () =
  Diagnostic.reset_audit_sequence ();
  let start_pos =
    Diagnostic.{ filename = "parser-example.reml"; line = 3; column = 12; offset = 34 }
  in
  let end_pos =
    Diagnostic.{ filename = "parser-example.reml"; line = 3; column = 12; offset = 34 }
  in
  let span = Diagnostic.{ start_pos; end_pos } in
  let expectation_tokens =
    [
      Token.RPAREN;
      Token.COMMA;
      Token.ELSE;
    ]
  in
  let expectations =
    expectation_tokens |> List.map Parser_expectation.expectation_of_token
  in
  let summary = Parser_expectation.summarize_with_defaults expectations in
  let builder =
    Diagnostic.Builder.create
      ~message:"構文エラー: 入力を解釈できません"
      ~primary:span ~domain:Diagnostic.Parser ()
    |> Diagnostic.Builder.set_primary_code "parser.expected_token"
    |> Diagnostic.Builder.add_code "parser.recovery.pending"
    |> Diagnostic.Builder.set_expected summary
    |> Diagnostic.Builder.add_extension "parse"
         (`Assoc
            [
              ("input_name", `String "parser-example.reml");
              ("stage_trace", `List []);
            ])
    |> Diagnostic.Builder.add_audit_metadata "parse.input_name"
         (`String "parser-example.reml")
    |> Diagnostic.Builder.add_audit_metadata
         "parser.expected.alternatives"
         (`Int (List.length summary.Diagnostic.alternatives))
  in
  let diag = Diagnostic.Builder.build builder in
  let json_str =
    Cli.Json_formatter.diagnostic_to_json ~mode:Cli.Options.JsonPretty diag
  in
  let golden_path =
    resolve "tests/golden/diagnostics/parser/expected-summary.json.golden"
  in
  if not (Sys.file_exists golden_path) then (
    let actual_path = write_actual_snapshot "parser-expected-summary" json_str in
    Printf.eprintf
      "✗ parser.expected_token: ゴールデン %s が存在しません。\n" golden_path;
    Printf.eprintf "  現在の出力を %s に書き出しました。\n" actual_path;
    exit 1);
  let expected =
    In_channel.with_open_text golden_path (fun ic -> In_channel.input_all ic)
  in
  if String.trim expected <> String.trim json_str then (
    let actual_path = write_actual_snapshot "parser-expected-summary" json_str in
    Printf.printf
      "✗ parser.expected_token: JSON スナップショットが一致しません\n";
    Printf.printf "  ゴールデン: %s\n" golden_path;
    Printf.printf "  現在の出力を %s に書き出しました。\n" actual_path;
    exit 1)
  else Printf.printf "✓ parser.expected_token JSON スナップショット\n"

let test_stage_extension_snapshot () =
  Diagnostic.reset_audit_sequence ();
  let start_pos =
    Diagnostic.{ filename = "iter.reml"; line = 4; column = 3; offset = 42 }
  in
  let end_pos =
    Diagnostic.{ filename = "iter.reml"; line = 4; column = 18; offset = 57 }
  in
  let span = Diagnostic.{ start_pos; end_pos } in
  let residual =
    `Assoc
      [
        ( "missing_ops",
          `List [ `String "Iterator::next"; `String "Iterator::size_hint" ] );
      ]
  in
  let metadata =
    `Assoc
      [
        ("provider", `String "core.iter");
        ("last_verified_at", `String "2025-10-21T03:15:00Z");
      ]
  in
  let iterator_fields =
    [
      ("required", `String "at_least:beta");
      ("actual", `String "experimental");
      ("kind", `String "custom:iterator.snapshot");
      ("capability", `String "core.iterator.collect");
      ("source", `String "dsl/core.iter.toml");
    ]
  in
  let capability_entries =
    [
      ("core.iterator.collect", Some "experimental");
      ("core.iterator.reduce", Some "stable");
    ]
  in
  let diag =
    Diagnostic.make_type_error ~code:"typeclass.iterator.stage_mismatch"
      ~message:"Iterator Capability が要求された Stage を満たしていません" ~span
      ~notes:
        [
          ( None,
            "要求 Stage: beta / Capability Stage: experimental \
             (core.iterator.collect)" );
        ]
      ()
    |> Diagnostic.with_effect_stage_extension ~required_stage:"beta"
         ~actual_stage:"experimental" ~capability:"core.iterator.collect"
         ~capability_stages:capability_entries ~provider:"Core.Iter"
         ~manifest_path:"dsl/core.iter.toml" ~residual
         ~capability_meta:metadata ~iterator_fields
  in
  let typeclass_constraint : Types.trait_constraint =
    {
      trait_name = "Iterator";
      type_args =
        [ Types.TCon (Types.TCUser "SampleStream"); Types.TCon (Types.TCUser "SampleItem") ];
      constraint_span = Ast.{ start = start_pos.offset; end_ = end_pos.offset };
    }
  in
  let typeclass_summary =
    Typeclass_metadata.make_summary ~constraint_:typeclass_constraint
      ~resolution_state:Typeclass_metadata.StageMismatch ()
  in
  let diag =
    let diag_with_base =
      Diagnostic.set_extension "typeclass"
        (Typeclass_metadata.extension_json typeclass_summary) diag
    in
    let diag_with_pairs =
      List.fold_left
        (fun acc (key, value) -> Diagnostic.set_extension key value acc)
        diag_with_base
        (Typeclass_metadata.extension_pairs typeclass_summary)
    in
    let diag_with_timestamp =
      { diag_with_pairs with timestamp = "1970-01-01T00:00:00Z" }
    in
    let diag_with_metadata =
      Diagnostic.merge_audit_metadata
        (Typeclass_metadata.metadata_pairs typeclass_summary)
        diag_with_timestamp
    in
    diag_with_metadata
  in
  let workspace = Some "." in
  let build_id =
    Diagnostic.compute_build_id ~timestamp:diag.timestamp ()
  in
  let sequence = 0 in
  let command_item =
    `Assoc
      [
        ("kind", `String "cli-command");
        ("command", `String "stage-diagnostic-test");
      ]
  in
  let input_item =
    `Assoc
      [
        ("kind", `String "input");
        ("path", `String "iter.reml");
      ]
  in
  let items = [ command_item; input_item ] in
  let change_set =
    Diagnostic.make_change_set_template ~origin:"cli" ~build_id ?workspace
      ?sequence:(Some sequence) ~items ()
  in
  let diag =
    diag
    |> Diagnostic.apply_audit_policy_metadata ~channel:"cli" ~build_id
         ~sequence ?workspace
    |> Diagnostic.set_audit_id
         (Printf.sprintf "cli/%s#%d" build_id sequence)
    |> Diagnostic.set_change_set change_set
  in
  let diag =
    let handler_stack = `List [ `String "core.iter.collect" ] in
    let unhandled_ops = `List [ `String "Iterator::size_hint" ] in
    let effects_payload =
      match Diagnostic.Extensions.get "effects" diag.Diagnostic.extensions with
      | Some (`Assoc fields) ->
          `Assoc
            (fields
             @ [
                 ("handler_stack", handler_stack);
                 ("unhandled_operations", unhandled_ops);
               ])
      | Some other -> other
      | None ->
          `Assoc
            [
              ("handler_stack", handler_stack);
              ("unhandled_operations", unhandled_ops);
            ]
    in
    diag
    |> Diagnostic.set_extension "effects" effects_payload
    |> Diagnostic.set_extension "parse"
         (`Assoc
            [
              ("input_name", `String "iter.reml");
              ("stage_trace", `List []);
            ])
    |> Diagnostic.set_extension "typeclass"
         (`Assoc
            [
              ("trait", `String "Iterator");
              ( "type_args",
                `List [ `String "SampleStream"; `String "SampleItem" ] );
              ("constraint", `String "Iterator<SampleStream, SampleItem>");
              ("resolution_state", `String "stage_mismatch");
              ( "dictionary",
                `Assoc
                  [
                    ("kind", `String "diagnostic-sample");
                    ("identifier", `String "Iterator.collect#stage_mismatch");
                    ("trait", `String "Iterator");
                    ( "type_args",
                      `List
                        [ `String "SampleStream"; `String "SampleItem" ] );
                    ("repr", `String "{}");
                  ] );
              ("candidates", `List [ `String "Iterator.collect" ]);
              ("pending", `List [ `String "Iterator.size_hint" ]);
              ("generalized_typevars", `List [ `String "T" ]);
              ("graph", `Assoc [ ("export_dot", `Null) ]);
              ("stage", `Null);
            ])
    |> Diagnostic.set_audit_metadata "effect.handler_stack" handler_stack
    |> Diagnostic.set_audit_metadata "effect.unhandled_operations" unhandled_ops
    |> Diagnostic.set_audit_metadata "bridge.audit_pass_rate" (`Float 1.0)
    |> Diagnostic.set_audit_metadata "bridge.status" (`String "ok")
    |> Diagnostic.set_audit_metadata "cli"
         (`Assoc
            [
              ("audit_id", `String (Printf.sprintf "cli/%s#%d" build_id sequence));
              ("change_set", change_set);
            ])
    |> Diagnostic.set_audit_metadata "schema"
         (`Assoc [ ("version", `String "1.1") ])
    |> Diagnostic.set_extension "typeclass.dictionary"
         (`Assoc
            [
              ("kind", `String "diagnostic-sample");
              ("identifier", `String "Iterator.collect#stage_mismatch");
              ("trait", `String "Iterator");
              ( "type_args",
                `List [ `String "SampleStream"; `String "SampleItem" ] );
              ("repr", `String "{}");
            ])
    |> Diagnostic.set_extension "typeclass.candidates"
         (`List [ `String "Iterator.collect" ])
    |> Diagnostic.set_extension "typeclass.pending"
         (`List [ `String "Iterator.size_hint" ])
    |> Diagnostic.set_extension "typeclass.generalized_typevars"
         (`List [ `String "T" ])
    |> Diagnostic.set_audit_metadata "typeclass.dictionary.kind"
         (`String "diagnostic-sample")
    |> Diagnostic.set_audit_metadata "typeclass.dictionary.identifier"
         (`String "Iterator.collect#stage_mismatch")
    |> Diagnostic.set_audit_metadata "typeclass.dictionary.trait"
         (`String "Iterator")
    |> Diagnostic.set_audit_metadata "typeclass.dictionary.type_args"
         (`List [ `String "SampleStream"; `String "SampleItem" ])
    |> Diagnostic.set_audit_metadata "typeclass.dictionary.repr"
         (`String "{}")
    |> Diagnostic.set_audit_metadata "typeclass.candidates"
         (`List [ `String "Iterator.collect" ])
    |> Diagnostic.set_audit_metadata "typeclass.pending"
         (`List [ `String "Iterator.size_hint" ])
    |> Diagnostic.set_audit_metadata "typeclass.generalized_typevars"
         (`List [ `String "T" ])
  in
  let json_str =
    Cli.Json_formatter.diagnostic_to_json ~mode:Cli.Options.JsonPretty diag
  in
  let golden_path =
    resolve "tests/golden/typeclass_iterator_stage_mismatch.json.golden"
  in
  if not (Sys.file_exists golden_path) then (
    let actual_path =
      write_actual_snapshot "typeclass_iterator_stage_mismatch" json_str
    in
    Printf.eprintf "✗ typeclass.iterator.stage_mismatch: ゴールデン %s が存在しません。\n"
      golden_path;
    Printf.eprintf "  現在の出力を %s に書き出しました。\n" actual_path;
    exit 1);
  let expected =
    In_channel.with_open_text golden_path (fun ic -> In_channel.input_all ic)
  in
  if String.trim expected <> String.trim json_str then (
    let actual_path =
      write_actual_snapshot "typeclass_iterator_stage_mismatch" json_str
    in
    Printf.printf "✗ typeclass.iterator.stage_mismatch: JSON スナップショットが一致しません\n";
    Printf.printf "  ゴールデン: %s\n" golden_path;
    Printf.printf "  現在の出力を %s に書き出しました。\n" actual_path;
    exit 1)
  else (
    let parsed = Yojson.Basic.from_string json_str in
    let diag_json =
      parsed
      |> Yojson.Basic.Util.member "diagnostics"
      |> Yojson.Basic.Util.to_list
      |> List.hd
    in
    let audit_metadata =
      Yojson.Basic.Util.member "audit_metadata" diag_json
    in
    let event_domain =
      Yojson.Basic.Util.member "event.domain" audit_metadata
      |> Yojson.Basic.Util.to_string
    in
    let event_kind =
      Yojson.Basic.Util.member "event.kind" audit_metadata
      |> Yojson.Basic.Util.to_string
    in
    assert (String.equal event_domain "type");
    assert (String.equal event_kind "typeclass.iterator.stage_mismatch");
    let expected_capabilities =
      [ "core.iterator.collect"; "core.iterator.reduce" ]
    in
    let capability_ids =
      Yojson.Basic.Util.member "capability.ids" audit_metadata
      |> Yojson.Basic.Util.to_list
      |> List.map Yojson.Basic.Util.to_string
    in
    assert (capability_ids = expected_capabilities);
    let audit_metadata_from_envelope =
      Yojson.Basic.Util.member "audit" diag_json
      |> Yojson.Basic.Util.member "metadata"
    in
    let envelope_cap_ids =
      Yojson.Basic.Util.member "capability.ids" audit_metadata_from_envelope
      |> Yojson.Basic.Util.to_list
      |> List.map Yojson.Basic.Util.to_string
    in
    assert (envelope_cap_ids = capability_ids);
    let capability_extension =
      Yojson.Basic.Util.member "extensions" diag_json
      |> Yojson.Basic.Util.member "capability"
    in
    let extension_cap_ids =
      Yojson.Basic.Util.member "ids" capability_extension
      |> Yojson.Basic.Util.to_list
      |> List.map Yojson.Basic.Util.to_string
    in
    assert (extension_cap_ids = capability_ids);
    let capability_primary =
      Yojson.Basic.Util.member "primary" capability_extension
      |> Yojson.Basic.Util.to_string
    in
    assert (String.equal capability_primary "core.iterator.collect");
    let effects_extension =
      Yojson.Basic.Util.member "extensions" diag_json
      |> Yojson.Basic.Util.member "effects"
    in
    let to_string_list json field =
      Yojson.Basic.Util.member field json
      |> Yojson.Basic.Util.to_list
      |> List.map Yojson.Basic.Util.to_string
    in
    let to_capability_stage_list json field =
      Yojson.Basic.Util.member field json
      |> Yojson.Basic.Util.to_list
      |> List.map (fun entry ->
             let cap =
               Yojson.Basic.Util.member "capability" entry
               |> Yojson.Basic.Util.to_string
             in
             let stage =
               Yojson.Basic.Util.member "stage" entry
               |> Yojson.Basic.Util.to_string
             in
             (cap, stage))
    in
    let required_caps =
      to_string_list effects_extension "required_capabilities"
    in
    let actual_caps =
      to_capability_stage_list effects_extension "actual_capabilities"
    in
    assert (
      required_caps
      = [ "core.iterator.collect"; "core.iterator.reduce" ]);
    assert (
      actual_caps
      = [
          ("core.iterator.collect", "experimental");
          ("core.iterator.reduce", "stable");
        ]);
    let audit_required_caps =
      to_string_list audit_metadata "effect.stage.required_capabilities"
    in
    let audit_actual_caps =
      to_capability_stage_list audit_metadata "effect.stage.actual_capabilities"
    in
    assert (audit_required_caps = required_caps);
    assert (audit_actual_caps = actual_caps);
    let envelope_required_caps =
      to_string_list audit_metadata_from_envelope
        "effect.stage.required_capabilities"
    in
    let envelope_actual_caps =
      to_capability_stage_list audit_metadata_from_envelope
        "effect.stage.actual_capabilities"
    in
    assert (envelope_required_caps = required_caps);
    assert (envelope_actual_caps = actual_caps)
  );
  Printf.printf "✓ typeclass.iterator.stage_mismatch JSON スナップショット\n"

let test_plugin_bundle_metadata () =
  let start_pos =
    Diagnostic.{ filename = "plugin-demo.reml"; line = 1; column = 1; offset = 0 }
  in
  let end_pos =
    Diagnostic.{ filename = "plugin-demo.reml"; line = 1; column = 5; offset = 4 }
  in
  let primary = Diagnostic.{ start_pos; end_pos } in
  let base_diag =
    Diagnostic.Builder.create ~severity:Diagnostic.Error
      ~domain:Diagnostic.Plugin
      ~message:"プラグインの署名が検証できません"
      ~primary ()
    |> Diagnostic.Builder.set_primary_code "plugin.bundle.signature_invalid"
    |> Diagnostic.Builder.build
  in
  let diag =
    base_diag
    |> Diagnostic.with_plugin_metadata
         ~bundle_id:"demo.bundle"
         ~signature:
           (`Assoc
              [
                ("provided", `String "sha256:deadbeef");
                ("status", `String "invalid");
              ])
  in
  let json =
    Cli.Json_formatter.diagnostic_to_json ~mode:Cli.Options.JsonPretty diag
    |> Yojson.Basic.from_string
  in
  let first_diag =
    json |> Yojson.Basic.Util.member "diagnostics"
    |> Yojson.Basic.Util.to_list |> List.hd
  in
  let audit_metadata =
    Yojson.Basic.Util.member "audit_metadata" first_diag
  in
  let bundle_id =
    Yojson.Basic.Util.member "plugin.bundle_id" audit_metadata
    |> Yojson.Basic.Util.to_string
  in
  assert (String.equal bundle_id "demo.bundle");
  let signature_status =
    Yojson.Basic.Util.member "plugin.signature.status" audit_metadata
    |> Yojson.Basic.Util.to_string
  in
  assert (String.equal signature_status "invalid");
  let extensions =
    Yojson.Basic.Util.member "extensions" first_diag
    |> Yojson.Basic.Util.to_assoc
  in
  let plugin_extension =
    List.assoc "plugin" extensions |> Yojson.Basic.Util.to_assoc
  in
  let plugin_bundle =
    List.assoc "bundle_id" plugin_extension |> Yojson.Basic.Util.to_string
  in
  assert (String.equal plugin_bundle "demo.bundle")

let test_typeclass_dictionary_snapshot () =
  Diagnostic.reset_audit_sequence ();
  let start_pos =
    Diagnostic.{ filename = "dict.reml"; line = 5; column = 7; offset = 58 }
  in
  let end_pos =
    Diagnostic.{ filename = "dict.reml"; line = 5; column = 18; offset = 69 }
  in
  let span = Diagnostic.{ start_pos; end_pos } in
  let diag =
    Diagnostic.make_type_error
      ~code:"typeclass.dictionary.resolved"
      ~message:"Eq 辞書を Core IR へ渡して監査ログへ記録しました"
      ~span ()
    |> Diagnostic.set_audit_id "22222222-2222-2222-2222-222222222222"
    |> Diagnostic.set_change_set
         (`Assoc
           [
             ("command", `String "typeclass-diagnostic-test");
             ("input", `String "dict.reml");
           ])
  in
  let ty_i64 = Types.TCon (Types.TCInt Types.I64) in
  let typeclass_constraint : Types.trait_constraint =
    {
      trait_name = "Eq";
      type_args = [ ty_i64 ];
      constraint_span = Ast.{ start = start_pos.offset; end_ = end_pos.offset };
    }
  in
  let dict_ref = Constraint_solver.DictImplicit ("Eq", [ ty_i64 ]) in
  let typeclass_summary =
    Typeclass_metadata.make_summary ~constraint_:typeclass_constraint
      ~resolution_state:Typeclass_metadata.Resolved ~dict_ref:dict_ref ()
  in
  let diag =
    let diag_with_base =
      Diagnostic.set_extension "typeclass"
        (Typeclass_metadata.extension_json typeclass_summary) diag
    in
    let diag_with_pairs =
      List.fold_left
        (fun acc (key, value) -> Diagnostic.set_extension key value acc)
        diag_with_base
        (Typeclass_metadata.extension_pairs typeclass_summary)
    in
    let diag_with_timestamp =
      { diag_with_pairs with timestamp = "1970-01-01T00:00:00Z" }
    in
    Diagnostic.merge_audit_metadata
      (Typeclass_metadata.metadata_pairs typeclass_summary)
      diag_with_timestamp
  in
  let json_str =
    Cli.Json_formatter.diagnostic_to_json ~mode:Cli.Options.JsonPretty diag
  in
  let golden_path =
    resolve "tests/golden/typeclass_dictionary_resolved.json.golden"
  in
  if not (Sys.file_exists golden_path) then (
    let actual_path =
      write_actual_snapshot "typeclass_dictionary_resolved" json_str
    in
    Printf.eprintf
      "✗ typeclass.dictionary.resolved: ゴールデン %s が存在しません。\n"
      golden_path;
    Printf.eprintf "  現在の出力を %s に書き出しました。\n" actual_path;
    exit 1);
  let expected =
    In_channel.with_open_text golden_path (fun ic -> In_channel.input_all ic)
  in
  if String.trim expected <> String.trim json_str then (
    let actual_path =
      write_actual_snapshot "typeclass_dictionary_resolved" json_str
    in
    Printf.printf "✗ typeclass.dictionary.resolved: JSON スナップショットが一致しません\n";
    Printf.printf "  ゴールデン: %s\n" golden_path;
    Printf.printf "  現在の出力を %s に書き出しました。\n" actual_path;
    exit 1)
  else Printf.printf "✓ typeclass.dictionary.resolved JSON スナップショット\n"

let test_info_hint_snapshot () =
  let mk_loc line col offset =
    Diagnostic.{ filename = "demo.reml"; line; column = col; offset }
  in
  let info_span =
    Diagnostic.
      {
        start_pos = mk_loc 1 0 0;
        end_pos = mk_loc 1 4 4;
      }
  in
  let hint_span =
    Diagnostic.
      {
        start_pos = mk_loc 2 0 5;
        end_pos = mk_loc 2 5 10;
      }
  in
  let info_diag =
    Diagnostic.Builder.create ~severity:Diagnostic.Info ~domain:Diagnostic.Cli
      ~timestamp:"1970-01-01T00:00:00Z"
      ~message:"情報レベルの診断メッセージです" ~primary:info_span ()
    |> Diagnostic.Builder.set_primary_code "demo.info.sample"
    |> Diagnostic.Builder.add_audit_metadata "demo.kind" (`String "info")
    |> Diagnostic.Builder.build
  in
  let hint_diag =
    Diagnostic.Builder.create ~severity:Diagnostic.Hint
      ~severity_hint:Diagnostic.Ignore ~domain:Diagnostic.Cli
      ~timestamp:"1970-01-01T00:00:00Z"
      ~message:"ヒントレベルの診断メッセージです" ~primary:hint_span ()
    |> Diagnostic.Builder.set_primary_code "demo.hint.sample"
    |> Diagnostic.Builder.add_hint
         ~message:"構文ヒント: use 文を展開してください"
         ~actions:[]
    |> Diagnostic.Builder.add_audit_metadata "demo.kind" (`String "hint")
    |> Diagnostic.Builder.build
  in
  let json_str =
    Cli.Json_formatter.diagnostics_to_json ~mode:Cli.Options.JsonPretty
      [ info_diag; hint_diag ]
  in
  let golden_path =
    resolve "tests/golden/diagnostics/severity/info-hint.json.golden"
  in
  if not (Sys.file_exists golden_path) then (
    let actual_path =
      write_actual_snapshot "diagnostics_severity_info_hint" json_str
    in
    Printf.eprintf
      "✗ diagnostics.severity.info_hint: ゴールデン %s が存在しません。\n"
      golden_path;
    Printf.eprintf "  現在の出力を %s に書き出しました。\n" actual_path;
    exit 1);
  let expected =
    In_channel.with_open_text golden_path (fun ic -> In_channel.input_all ic)
  in
  if String.trim expected <> String.trim json_str then (
    let actual_path =
      write_actual_snapshot "diagnostics_severity_info_hint" json_str
    in
    Printf.printf
      "✗ diagnostics.severity.info_hint: JSON スナップショットが一致しません\n";
    Printf.printf "  ゴールデン: %s\n" golden_path;
    Printf.printf "  現在の出力を %s に書き出しました。\n" actual_path;
    exit 1)
  else Printf.printf "✓ diagnostics.severity.info_hint JSON スナップショット\n"

(** ソースコードスニペットのテスト *)
let test_snippet_display () =
  let diag = make_test_diagnostic () in

  (* ソースコード付き出力を生成 *)
  let output =
    Cli.Diagnostic_formatter.format_diagnostic ~source:(Some test_source) ~diag
      ~color_mode:Cli.Options.Never ~include_snippet:true
  in

  (* スニペットに行番号区切り文字 " | " が含まれている *)
  assert (String.contains output '|');

  (* スニペットにソースコードが含まれている *)
  (* let x という文字列を含む *)
  let contains_let_x =
    try
      let _ = Str.search_forward (Str.regexp "let x") output 0 in
      true
    with Not_found -> false
  in
  assert contains_let_x;

  (* 型情報（String）を含む *)
  let contains_string =
    try
      let _ = Str.search_forward (Str.regexp "String") output 0 in
      true
    with Not_found -> false
  in
  assert contains_string;

  (* ポインタが含まれている *)
  assert (String.contains output '^');
  Printf.printf "✓ ソースコードスニペット表示テスト成功\n"

(** 複数診断のバッチ出力テスト *)
let test_multiple_diagnostics () =
  let diag1 = make_test_diagnostic () in
  let diag2 =
    {
      diag1 with
      Diagnostic.message = "別のエラー";
      codes = [ "E7002" ];
    }
  in

  (* 複数診断の JSON 出力 *)
  let json_str =
    Cli.Json_formatter.diagnostics_to_json ~mode:Cli.Options.JsonPretty
      [ diag1; diag2 ]
  in
  let json = Yojson.Basic.from_string json_str in
  let diagnostics = json |> Yojson.Basic.Util.member "diagnostics" in
  let diag_list = diagnostics |> Yojson.Basic.Util.to_list in

  assert (List.length diag_list = 2);
  Printf.printf "✓ 複数診断のバッチ出力テスト成功\n"

(** すべてのテストを実行 *)
let () =
  Printf.printf "\n=== CLI 診断出力テスト ===\n";
  test_color_output ();
  test_json_output ();
  test_other_domain_serialization ();
  test_info_hint_snapshot ();
  test_parser_expectation_snapshot ();
  test_plugin_bundle_metadata ();
  test_stage_extension_snapshot ();
  test_typeclass_dictionary_snapshot ();
  test_snippet_display ();
  test_multiple_diagnostics ();
  Printf.printf "\n✓ すべてのテストが成功しました\n"
