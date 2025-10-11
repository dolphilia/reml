(* test_cli_diagnostics.ml — CLI 診断出力のテスト
 *
 * Phase 1-6 の開発者体験整備タスクにおいて、診断フォーマッタの動作を検証する。
 *)

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
  test_snippet_display ();
  test_multiple_diagnostics ();
  Printf.printf "\n✓ すべてのテストが成功しました\n"
