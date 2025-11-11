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

let normalize_effect_names names =
  names
  |> List.map (fun name -> name |> String.trim |> String.lowercase_ascii)

let filter_effect_decls ast =
  {
    ast with
    Ast.decls =
      List.filter
        (fun decl ->
          match decl.Ast.decl_kind with Ast.EffectDecl _ -> false | _ -> true)
        ast.Ast.decls;
  }

let streaming_expected_token_count =
  Parser_expectation.streaming_expression_summary ()
  |> fun summary -> List.length summary.Diagnostic.alternatives

let has_streaming_expected_tokens diag =
  match diag.Diagnostic.expected with
  | Some summary ->
      List.length summary.Diagnostic.alternatives = streaming_expected_token_count
  | None -> false

let recover_extension_token_count diag =
  Diagnostic.Extensions.recover_expected_tokens diag.Diagnostic.extensions
  |> List.length

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
    { Run_config.default with packrat = true }
    |> Run_config.Stream.set_enabled true
    |> Run_config.Stream.set_resume_hint (Some "resume-token")
    |> Run_config.Stream.set_demand_min_bytes (Some 4)
    |> Run_config.Stream.set_demand_preferred_bytes (Some 8)
    |> Run_config.Stream.set_flow_policy (Some Run_config.Stream.Flow.Auto)
    |> Run_config.Stream.set_flow_max_lag_bytes (Some 8192)
    |> Run_config.Stream.set_flow_debounce_ms (Some 10)
    |> Run_config.Stream.set_flow_throttle_ratio (Some 0.75)
    |> Run_config.Effects.set_stage_override (Some "beta")
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
        (pending.continuation.meta.commit_watermark = String.length chunk_a)
        desc "commit_watermark がバッファ長と一致しません";
      ensure
        (match pending.meta.memo_bytes with Some _ -> true | None -> false)
        desc "Packrat メモ統計が memo_bytes に反映されていません";
      ensure
        (pending.meta.last_reason = Some "pending.backpressure")
        desc "last_reason がバックプレッシャ理由を指していません";
      ensure
        (pending.continuation.meta.resume_lineage
        = [ "pending.backpressure" ])
        desc "resume_lineage がバックプレッシャ理由を保持していません";
      ensure
        (pending.continuation.meta.backpressure_counter = 1)
        desc "backpressure_counter がインクリメントされていません";
      ensure
        (match pending.continuation.packrat_cache with
        | Some _ -> true
        | None -> false)
        desc "Packrat キャッシュが継続へ共有されていません";
      ensure
        (pending.demand.action = `Pause
        && pending.demand.min_bytes = Some 4
        && pending.demand.preferred_bytes = Some 6)
        desc "DemandHint の min/preferred が FlowController Auto で再計算されていません";
      ensure
        (pending.meta.backpressure_policy = Some "auto"
        && pending.meta.backpressure_events = 1)
        desc "バックプレッシャメタデータが期待通りに記録されていません";
      ensure
        (List.exists
           (fun event ->
             String.equal event.Audit_envelope.category "parser.stream.pending")
           pending.audit_events)
        desc "Pending 監査イベントが作成されていません";
      ensure
        (List.exists
           (fun event ->
             String.equal event.Audit_envelope.category "parser.stream.error")
           pending.audit_events)
        desc "Pending のエラー監査イベントが不足しています";
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
      ensure
        (completed.meta.backpressure_policy = Some "auto"
        && completed.meta.backpressure_events = 1)
        desc "Completed メタデータのバックプレッシャ情報が不足しています";
      let error_events =
        completed.audit_events
        |> List.filter (fun event ->
               String.equal event.Audit_envelope.category "parser.stream.error")
      in
      let error_events_require_fail events =
        List.exists
          (fun event ->
            let metadata =
              event.Audit_envelope.envelope.Audit_envelope.metadata
            in
            let has_expected_tokens =
              List.exists
                (fun (key, value) ->
                  String.equal key "parser.stream.error.expected_tokens"
                  && (not (Yojson.Basic.equal value `Null)))
                metadata
            in
            not has_expected_tokens)
          events
      in
      if error_events_require_fail error_events then (
        let summaries =
          error_events
          |> List.mapi (fun idx event ->
                 let metadata =
                   event.Audit_envelope.envelope.Audit_envelope.metadata
                   |> List.map (fun (key, value) ->
                          Printf.sprintf "%s=%s" key
                            (Yojson.Basic.to_string value))
                   |> String.concat "; "
                 in
                 Printf.sprintf "#%d(%s)" (idx + 1) metadata)
          |> String.concat ", "
        in
        let diagnostics =
          completed.result.diagnostics
          |> List.filter (fun diag ->
                 diag.Diagnostic.severity = Diagnostic.Error)
          |> List.map (fun diag ->
                 Printf.sprintf "%s[%s]" diag.Diagnostic.message
                   (String.concat "," diag.Diagnostic.codes))
          |> String.concat "; "
        in
        let details =
          if diagnostics = "" then summaries
          else Printf.sprintf "%s; diagnostics=%s" summaries diagnostics
        in
        fail desc
          (Printf.sprintf
             "Completed の監査イベントに parser.stream.error が含まれます: %s"
             details));
      pass desc

let test_backpressure_recover_diagnostic () =
  let desc = "バックプレッシャ Pending で recover 診断が生成される" in
  let input =
    "module diagnostics.w4.stream_backpressure\n\
fn produce(limit: i32) -> i32 =\n\
  if limit <= 0 then 0 else produce(limit - 1)\n\
fn main() = produce(4)\n"
  in
  let len = String.length input in
  let split = max 1 (len / 2) in
  let chunk_a = String.sub input 0 split in
  let chunk_b = String.sub input split (len - split) in
  let config =
    Run_config.default
    |> Run_config.Stream.set_enabled true
    |> Run_config.Stream.set_flow_policy (Some Run_config.Stream.Flow.Auto)
    |> Run_config.Stream.set_demand_min_bytes (Some 4)
    |> Run_config.Stream.set_demand_preferred_bytes (Some 8)
    |> Run_config.Stream.set_flow_max_lag_bytes (Some 8192)
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
    | _ -> Stream.Closed
  in
  match
    Stream.run_stream ~filename:"stream_backpressure_hint.reml" ~config ~feeder
      ()
  with
  | Stream.Completed _ ->
      fail desc "バックプレッシャ Pending を再現できませんでした"
  | Stream.Pending pending ->
      ensure
        (List.length pending.diagnostics >= 1)
        desc "Pending へ recover 診断が添付されていません";
      ensure
        (match pending.demand.reason with
        | Some reason -> String.equal (String.trim reason) "pending.backpressure"
        | None -> false)
        desc "Pending demand.reason が pending.backpressure ではありません";
      let pending_has_expected =
        List.exists
          (fun diag ->
            match diag.Diagnostic.expected with
            | Some summary -> summary.Diagnostic.alternatives <> []
            | None -> false)
          pending.diagnostics
      in
      if not pending_has_expected then (
        let details =
          pending.diagnostics
          |> List.mapi (fun idx diag ->
                 let label = diag.Diagnostic.message in
                 let expected_info =
                   match diag.Diagnostic.expected with
                   | Some summary ->
                       string_of_int (List.length summary.Diagnostic.alternatives)
                   | None -> "none"
                 in
                 Printf.sprintf "#%d:%s(%s)" (idx + 1) label expected_info)
          |> String.concat "; "
        in
        fail desc
          (Printf.sprintf
             "Pending 診断に expected_tokens がありません (details: %s)" details));
      ensure
        (List.exists
           (fun diag -> recover_extension_token_count diag > 0)
           pending.diagnostics)
        desc "Pending recover 拡張に expected_tokens が含まれていません";
      let after_chunk =
        Stream.resume pending.continuation (Stream.Chunk chunk_b)
      in
      let completed =
        match after_chunk with
        | Stream.Completed completed -> completed
        | Stream.Pending pending2 -> (
            match Stream.resume pending2.continuation Stream.Closed with
            | Stream.Completed completed -> completed
            | Stream.Pending _ ->
                fail desc "resume 後に Completed へ到達しませんでした")
      in
      let diag_count = List.length completed.result.diagnostics in
      ensure (diag_count >= 1) desc "Completed 結果に診断がありません";
      let has_expected =
        List.exists
          (fun diag ->
            match diag.Diagnostic.expected with
            | Some summary -> summary.Diagnostic.alternatives <> []
            | None -> false)
          completed.result.diagnostics
      in
      if not has_expected then (
        let details =
          completed.result.diagnostics
          |> List.mapi (fun idx diag ->
                 let expected_info =
                   match diag.Diagnostic.expected with
                   | Some summary ->
                       string_of_int (List.length summary.Diagnostic.alternatives)
                   | None -> "none"
                 in
                 Printf.sprintf "#%d:%s" (idx + 1) expected_info)
          |> String.concat "; "
        in
        fail desc
          (Printf.sprintf
             "recover 診断に expected_tokens が含まれていません (details: %s)"
             details));
      ensure
        (List.exists
           (fun diag -> recover_extension_token_count diag > 0)
           completed.result.diagnostics)
        desc "Completed recover 拡張に expected_tokens が含まれていません";
      pass desc

let test_backpressure_hint_without_pending () =
  let desc = "Flow.Auto で Await 無しでも recover 診断を生成する" in
  let input =
    "module diagnostics.w4.stream_backpressure\n\
fn produce(limit: i32) -> i32 =\n\
  if limit <= 0 then limit else produce(limit - 1)\n\
fn main() = produce(2)\n"
  in
  let finished = ref false in
  let feeder () =
    if !finished then Stream.Closed
    else (
      finished := true;
      Stream.Chunk input)
  in
  let config =
    Run_config.default
    |> Run_config.Stream.set_enabled true
    |> Run_config.Stream.set_flow_policy (Some Run_config.Stream.Flow.Auto)
    |> Run_config.Stream.set_flow_max_lag_bytes (Some 8192)
  in
  match
    Stream.run_stream ~filename:"stream_backpressure_hint_auto.reml" ~config
      ~feeder ()
  with
  | Stream.Pending _ ->
      fail desc "Await を返さない feeder で Pending になりました"
  | Stream.Completed completed ->
      ensure
        (Option.is_some completed.result.value)
        desc "Completed 結果に AST が含まれていません";
      let diagnostics = completed.result.diagnostics in
      ensure (List.length diagnostics >= 1) desc
        "Flow.Auto で recover 診断が生成されませんでした";
      ensure
        (List.exists has_streaming_expected_tokens diagnostics)
        desc
        "Flow.Auto recover 診断に streaming placeholder expected_tokens がありません";
      ensure
        (completed.meta.backpressure_events >= 1)
        desc "backpressure_events が記録されていません";
      ensure
        (completed.meta.last_reason = Some "pending.backpressure")
        desc "meta.last_reason が pending.backpressure ではありません";
      ensure
        (List.exists
           (fun event ->
             String.equal event.Audit_envelope.category "parser.stream.error")
           completed.audit_events)
        desc "parser.stream.error 監査イベントが生成されていません";
      pass desc

let test_streaming_effect_row_stage_consistency () =
  let desc = "effect.stage 監査イベントに効果行メタデータが含まれる" in
  let source =
    {|
effect Console : io {
  operation log : String -> Unit
}

@allows_effects(Console)
fn handled_demo(msg: String) = {
  handle perform Console.log(msg) with handler ConsoleHandler {
    operation log(value) {
      ()
    }
  }
}
|}
  in
  let index = ref 0 in
  let feeder () =
    match !index with
    | 0 ->
        incr index;
        Stream.Chunk source
    | _ -> Stream.Closed
  in
  let config = Run_config.set_experimental_effects Run_config.default true in
  match
    Stream.run_stream ~filename:"streaming_effects.reml" ~config ~feeder ()
  with
  | Stream.Pending _ ->
      fail desc "Pending のまま終了しました"
  | Stream.Completed completed ->
      let parse_result = completed.result in
      let ast =
        match parse_result.value with
        | Some cu -> cu
        | None -> fail desc "ストリーミング解析の結果に AST が含まれていません"
      in
      let ast = filter_effect_decls ast in
      let typer_config =
        Type_inference.make_config
          ~type_row_mode:Type_inference.Type_row_dual_write ()
      in
      let tast =
        match Type_inference.infer_compilation_unit ~config:typer_config ast with
        | Result.Ok tast -> tast
        | Result.Error err ->
            fail desc
              (Printf.sprintf "型推論に失敗しました: %s"
                 (Type_error.string_of_error err))
      in
      let fn_opt =
        tast.Typed_ast.tcu_items
        |> List.find_map (fun decl ->
               match decl.Typed_ast.tdecl_kind with
               | Typed_ast.TFnDecl fn when String.equal fn.tfn_name.name "handled_demo"
                 -> Some fn
               | _ -> None)
      in
      let fn =
        match fn_opt with
        | Some value -> value
        | None -> fail desc "handled_demo の型付き関数が見つかりませんでした"
      in
      let entry =
        match Constraint_solver.resolve_effect_profile ~symbol:"handled_demo" with
        | Some entry -> entry
        | None ->
            fail desc "効果制約テーブルから handled_demo を取得できませんでした"
      in
      ensure
        (match entry.Constraint_solver.EffectConstraintTable.type_row with
        | Some row -> Types.effect_row_equal row fn.Typed_ast.tfn_effect_row
        | None -> false)
        desc "constraint table の効果行が型情報と一致しません";
      let declared_expected =
        normalize_effect_names fn.Typed_ast.tfn_effect_row.declared
      in
      let residual_expected =
        normalize_effect_names fn.Typed_ast.tfn_effect_row.residual
      in
      let canonical_expected =
        Types.Effect_name_set.fold (fun name acc -> name :: acc)
          fn.Typed_ast.tfn_effect_row.canonical []
        |> List.sort_uniq String.compare
      in
      let effect_set =
        entry.Constraint_solver.EffectConstraintTable.effect_set
      in
      let effect_declared =
        effect_set.Effect_profile.declared
        |> List.map (fun tag ->
               String.lowercase_ascii tag.Effect_profile.effect_name)
      in
      let effect_residual =
        effect_set.residual
        |> List.map (fun tag ->
               String.lowercase_ascii tag.Effect_profile.effect_name)
      in
      ensure (effect_declared = declared_expected) desc
        "effect_set.declared が型情報と一致しません";
      ensure (effect_residual = residual_expected) desc
        "effect_set.residual が型情報と一致しません";
      ensure
        (canonical_expected
        = List.sort_uniq String.compare (effect_declared @ effect_residual))
        desc "effect_set の正規化結果が canonical と一致しません";
      let required_stage =
        Effect_profile.stage_requirement_to_string entry.stage_requirement
      in
      ensure
        (String.equal required_stage "at_least:stable") desc
        "Stage 要件が期待値と一致しません";
      let actual_stage =
        Option.map Effect_profile.stage_id_to_string entry.resolved_stage
      in
      ensure
        (match actual_stage with Some value -> String.equal value "stable" | _ -> false)
        desc "実行時 Stage が期待値と一致しません";
      Constraint_solver.reset_effect_constraints ()

let () =
  let tests =
    [
      test_streaming_matches_batch;
      test_pending_resume_flow;
      test_backpressure_recover_diagnostic;
      test_backpressure_hint_without_pending;
      test_streaming_effect_row_stage_consistency;
    ]
  in
  List.iter (fun fn -> fn ()) tests
