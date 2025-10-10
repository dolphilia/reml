(* test_cli_trace.ml — CLI トレース機能のテスト
 *
 * Phase 1-6 Week 15 のトレース機能実装の動作確認テスト
 *)

(* テストヘルパー: トレースをリセット *)
let reset_trace () =
  Cli.Trace.reset ();
  Cli.Stats.reset ()

(* テストヘルパー: アサーション *)
let assert_equal ~msg expected actual =
  if expected <> actual then
    failwith (Printf.sprintf "Assertion failed: %s (expected: %d, actual: %d)" msg expected actual)

let assert_bool msg cond =
  if not cond then
    failwith (Printf.sprintf "Assertion failed: %s" msg)

let assert_failure msg =
  failwith msg

(* テスト1: トレース機能の基本動作 *)
let test_trace_basic () =
  reset_trace ();

  (* フェーズ実行をシミュレート *)
  Cli.Trace.start_phase Cli.Trace.Parsing;
  Unix.sleepf 0.001;  (* 1ms 待機 *)
  Cli.Trace.end_phase Cli.Trace.Parsing;

  Cli.Trace.start_phase Cli.Trace.TypeChecking;
  Unix.sleepf 0.001;
  Cli.Trace.end_phase Cli.Trace.TypeChecking;

  (* トレース履歴を取得 *)
  let history = Cli.Trace.get_history () in
  assert_equal ~msg:"2つのフェーズが記録されているべき" 2 (List.length history);

  (* パース時間を確認 *)
  let parse_time = Cli.Trace.get_phase_time Cli.Trace.Parsing in
  match parse_time with
  | Some t ->
      assert_bool "パース時間が0秒以上であるべき" (t >= 0.0)
  | None ->
      assert_failure "パース時間が記録されていない";
  Printf.printf "✓ test_trace_basic passed\n%!"

(* テスト2: 統計情報収集の基本動作 *)
let test_stats_basic () =
  reset_trace ();

  (* カウンタをインクリメント *)
  Cli.Stats.incr_token_count ();
  Cli.Stats.incr_token_count ();
  Cli.Stats.incr_ast_node_count ();
  Cli.Stats.incr_unify_calls ();
  Cli.Stats.incr_optimization_passes ();
  Cli.Stats.incr_llvm_instructions ();

  let stats = Cli.Stats.get_stats () in
  assert_equal ~msg:"トークン数" 2 stats.token_count;
  assert_equal ~msg:"ASTノード数" 1 stats.ast_node_count;
  assert_equal ~msg:"unify呼び出し回数" 1 stats.unify_calls;
  assert_equal ~msg:"最適化パス数" 1 stats.optimization_passes;
  assert_equal ~msg:"LLVM命令数" 1 stats.llvm_instructions;
  Printf.printf "✓ test_stats_basic passed\n%!"

(* テスト3: フェーズの不一致検出 *)
let test_trace_phase_mismatch () =
  reset_trace ();

  (* フェーズを開始 *)
  Cli.Trace.start_phase Cli.Trace.Parsing;

  (* 異なるフェーズで終了（警告が出るはず） *)
  Cli.Trace.end_phase Cli.Trace.TypeChecking;

  (* トレース履歴は記録されているべき *)
  let history = Cli.Trace.get_history () in
  assert_equal ~msg:"不一致でも記録されるべき" 1 (List.length history);
  Printf.printf "✓ test_trace_phase_mismatch passed\n%!"

(* テスト4: リセット機能 *)
let test_trace_reset () =
  reset_trace ();

  (* トレースを実行 *)
  Cli.Trace.start_phase Cli.Trace.Parsing;
  Cli.Trace.end_phase Cli.Trace.Parsing;

  (* 履歴が記録されている *)
  let history1 = Cli.Trace.get_history () in
  assert_equal ~msg:"リセット前は履歴がある" 1 (List.length history1);

  (* リセット *)
  Cli.Trace.reset ();

  (* 履歴がクリアされている *)
  let history2 = Cli.Trace.get_history () in
  assert_equal ~msg:"リセット後は履歴がない" 0 (List.length history2);
  Printf.printf "✓ test_trace_reset passed\n%!"

(* テスト5: 統計情報のリセット *)
let test_stats_reset () =
  reset_trace ();

  (* カウンタをインクリメント *)
  Cli.Stats.incr_token_count ();
  Cli.Stats.incr_ast_node_count ();

  let stats1 = Cli.Stats.get_stats () in
  assert_equal ~msg:"リセット前はカウンタが増えている" 1 stats1.token_count;

  (* リセット *)
  Cli.Stats.reset ();

  let stats2 = Cli.Stats.get_stats () in
  assert_equal ~msg:"リセット後はカウンタが0" 0 stats2.token_count;
  assert_equal ~msg:"リセット後はカウンタが0" 0 stats2.ast_node_count;
  Printf.printf "✓ test_stats_reset passed\n%!"

(* テスト6: JSON出力 *)
let test_stats_json () =
  reset_trace ();

  Cli.Stats.incr_token_count ();
  Cli.Stats.incr_ast_node_count ();

  let json = Cli.Stats.to_json () in
  (* JSON形式であることを確認（簡易チェック） *)
  assert_bool "JSONに 'tokens_parsed' が含まれる" (String.contains json '{');
  assert_bool "JSONに 'tokens_parsed' が含まれる" (Str.string_match (Str.regexp ".*tokens_parsed.*") json 0);
  Printf.printf "✓ test_stats_json passed\n%!"

(* テストスイート *)
let () =
  Printf.printf "Running CLI Trace Tests...\n%!";
  test_trace_basic ();
  test_stats_basic ();
  test_trace_phase_mismatch ();
  test_trace_reset ();
  test_stats_reset ();
  test_stats_json ();
  Printf.printf "\nAll tests passed! ✓\n%!"
