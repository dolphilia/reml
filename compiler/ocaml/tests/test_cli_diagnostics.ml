(* test_cli_diagnostics.ml — CLI 診断出力のテスト
 *
 * Phase 1-6 の開発者体験整備タスクにおいて、診断フォーマッタの動作を検証する。
 *)

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
  Diagnostic.
    {
      severity = Error;
      severity_hint = None;
      domain = Some Type;
      code = Some "E7001";
      message = "型が一致しません";
      span = { start_pos; end_pos };
      expected_summary = None;
      notes = [ (None, "期待される型: i64"); (None, "実際の型:     String") ];
      fixits = [];
      extensions = Diagnostic.Extensions.empty;
    }

(** テスト用のソースコード *)
let test_source = "fn main() -> i64 =\n  let x: String = \"hello\" in\n  x + 42"

(** カラー出力のテスト *)
let test_color_output () =
  let diag = make_test_diagnostic () in

  (* カラーなしでの出力 *)
  let no_color_output =
    Cli.Diagnostic_formatter.format_diagnostic ~source:(Some test_source) ~diag
      ~color_mode:Cli.Options.Never
  in
  assert (not (String.contains no_color_output '\027'));

  (* カラーありでの出力 *)
  let color_output =
    Cli.Diagnostic_formatter.format_diagnostic ~source:(Some test_source) ~diag
      ~color_mode:Cli.Options.Always
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
  let json_str = Cli.Json_formatter.diagnostic_to_json diag in

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

  assert (severity = "error");
  assert (code = "E7001");
  assert (message = "型が一致しません");
  Printf.printf "✓ JSON出力テスト成功\n"

let test_stage_extension_snapshot () =
  let start_pos =
    Diagnostic.{ filename = "iter.reml"; line = 4; column = 3; offset = 42 }
  in
  let end_pos =
    Diagnostic.{ filename = "iter.reml"; line = 4; column = 18; offset = 57 }
  in
  let span = { Diagnostic.start_pos = start_pos; end_pos = end_pos } in
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
  let diag =
    Diagnostic.make_type_error
      ~code:"typeclass.iterator.stage_mismatch"
      ~message:"Iterator Capability が要求された Stage を満たしていません"
      ~span
      ~notes:
        [
          ( None,
            "要求 Stage: beta / Capability Stage: experimental (core.iterator.collect)"
          );
        ]
      ()
    |> Diagnostic.with_effect_stage_extension ~required_stage:"beta"
         ~actual_stage:"experimental" ~capability:"core.iterator.collect"
         ~provider:"Core.Iter" ~manifest_path:"dsl/core.iter.toml"
         ~residual ~capability_meta:metadata
  in
  let json_str = Cli.Json_formatter.diagnostic_to_json diag in
  let golden_path =
    resolve "tests/golden/typeclass_iterator_stage_mismatch.json.golden"
  in
  if not (Sys.file_exists golden_path) then (
    let actual_path =
      write_actual_snapshot "typeclass_iterator_stage_mismatch" json_str
    in
    Printf.eprintf
      "✗ typeclass.iterator.stage_mismatch: ゴールデン %s が存在しません。\n"
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
    Printf.printf
      "✗ typeclass.iterator.stage_mismatch: JSON スナップショットが一致しません\n";
    Printf.printf "  ゴールデン: %s\n" golden_path;
    Printf.printf "  現在の出力を %s に書き出しました。\n" actual_path;
    exit 1)
  else Printf.printf "✓ typeclass.iterator.stage_mismatch JSON スナップショット\n"

(** ソースコードスニペットのテスト *)
let test_snippet_display () =
  let diag = make_test_diagnostic () in

  (* ソースコード付き出力を生成 *)
  let output =
    Cli.Diagnostic_formatter.format_diagnostic ~source:(Some test_source) ~diag
      ~color_mode:Cli.Options.Never
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
    { diag1 with Diagnostic.message = "別のエラー"; code = Some "E7002" }
  in

  (* 複数診断の JSON 出力 *)
  let json_str = Cli.Json_formatter.diagnostics_to_json [ diag1; diag2 ] in
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
  test_stage_extension_snapshot ();
  test_snippet_display ();
  test_multiple_diagnostics ();
  Printf.printf "\n✓ すべてのテストが成功しました\n"
