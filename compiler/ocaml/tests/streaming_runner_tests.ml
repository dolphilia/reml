(* streaming_runner_tests.ml — ストリーミングランナー PoC の動作検証 *)

open Parser_driver

module Stream = Parser_driver.Streaming
module Run_config = Parser_run_config

let pass desc = Printf.printf "✓ %s\n" desc

let fail desc msg =
  Printf.printf "✗ %s: %s\n" desc msg;
  exit 1

let ensure predicate desc message =
  if predicate then ()
  else fail desc message

let test_streaming_matches_batch () =
  let desc = "run_stream がバッチランナーと同じ ParseResult を返す" in
  let input =
    {|
      module math
      fn add(x: i32, y: i32) -> i32 =
        x + y
    |}
  in
  let baseline = Parser_driver.run_string input in
  let index = ref 0 in
  let feeder () =
    match !index with
    | 0 ->
        incr index;
        Stream.Chunk input
    | _ ->
        Stream.Closed
  in
  match
    Stream.run_stream ~filename:"streaming_tests.reml"
      ~config:Run_config.default ~feeder ()
  with
  | Stream.Completed { result; _ } ->
      let legacy_baseline = Parser_driver.parse_result_to_legacy baseline in
      let legacy_stream = Parser_driver.parse_result_to_legacy result in
      ensure
        (match (legacy_baseline, legacy_stream) with
        | Ok _, Ok _ -> true
        | Error _, Error _ -> false
        | Ok _, Error _ | Error _, Ok _ -> false)
        desc "バッチとストリーミングの結果が一致しません";
      ensure
        (List.length baseline.diagnostics
        = List.length result.diagnostics)
        desc
        "診断件数がストリーミング経路で変化しました";
      pass desc
  | Stream.Pending _ ->
      fail desc "feeder が Pending を返さない想定で Completed になりませんでした"

let test_pending_resume_flow () =
  let desc = "Await -> Pending -> resume で Completed へ到達する" in
  let input = "fn twice(x: i32) -> i32 = x * 2" in
  let midpoint = String.length input / 2 in
  let chunk_a = String.sub input 0 midpoint in
  let chunk_b = String.sub input midpoint (String.length input - midpoint) in
  let config =
    Run_config.default
    |> Run_config.Stream.set_enabled true
    |> Run_config.Stream.set_resume_hint (Some "resume-token")
    |> Run_config.Stream.set_demand_min_bytes (Some 4)
    |> Run_config.Stream.set_demand_preferred_bytes (Some 8)
  in
  let step = ref 0 in
  let feeder () =
    match !step with
    | 0 ->
        incr step;
        Stream.Chunk chunk_a
    | 1 ->
        incr step;
        Stream.Await None
    | 2 ->
        incr step;
        Stream.Chunk chunk_b
    | _ ->
        Stream.Closed
  in
  match
    Stream.run_stream ~filename:"pending_resume.reml" ~config ~feeder ()
  with
  | Stream.Completed _ -> fail desc "Pending を経由せずに Completed になりました"
  | Stream.Pending pending ->
      ensure
        (pending.meta.await_count = 1 && pending.meta.resume_count = 0)
        desc "Pending メタデータの await/resume カウントが想定と異なります";
      ensure
        (pending.demand.action = `Pause
        && pending.demand.min_bytes = Some 4
        && pending.demand.preferred_bytes = Some 8)
        desc "DemandHint の min/preferred が RunConfig 設定を引き継いでいません";
      let after_chunk =
        Stream.resume pending.continuation (Stream.Chunk chunk_b)
      in
      let completed =
        match after_chunk with
        | Stream.Completed completed -> completed
        | Stream.Pending pending2 ->
            (* 追加入力待ちのままの場合は Closed で締める *)
            (match Stream.resume pending2.continuation Stream.Closed with
            | Stream.Completed completed -> completed
            | Stream.Pending _ ->
                fail desc "Closed 投入後も Pending のままです")
      in
      let legacy = Parser_driver.parse_result_to_legacy completed.result in
      ensure
        (match legacy with Ok _ -> true | Error _ -> false)
        desc "resume 後の ParseResult が成功しませんでした";
      ensure
        (completed.meta.resume_count >= 1 && completed.meta.await_count >= 1)
        desc "resume メタデータが更新されていません";
      pass desc

let () =
  let tests = [ test_streaming_matches_batch; test_pending_resume_flow ] in
  List.iter (fun fn -> fn ()) tests
