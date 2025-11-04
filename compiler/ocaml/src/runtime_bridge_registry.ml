open Diagnostic

module Json = Yojson.Basic

type stream_signal = {
  bridge_id : string;
  span : span;
  policy : string;
  reason : string;
  demand : Json.t;
  await_count : int;
  resume_count : int;
  backpressure_events : int;
  stage_required : string;
  stage_actual : string;
}

let message_for_signal signal =
  Printf.sprintf
    "Runtime Bridge \"%s\" は Stage %s を要求しますが、現在の Stage は %s です。Flow policy=%s でバックプレッシャ信号を処理できません。"
    signal.bridge_id
    (String.uppercase_ascii signal.stage_required)
    (String.uppercase_ascii signal.stage_actual)
    (String.lowercase_ascii signal.policy)

let make_bridge_extension signal =
  `Assoc
    [
      ("id", `String signal.bridge_id);
      ( "stage",
        `Assoc
          [
            ("required", `String signal.stage_required);
            ("actual", `String signal.stage_actual);
          ] );
      ( "signal",
        `Assoc
          [
            ("kind", `String "pending");
            ("reason", `String signal.reason);
            ("policy", `String signal.policy);
            ("backpressure_events", `Int signal.backpressure_events);
            ("await_count", `Int signal.await_count);
            ("resume_count", `Int signal.resume_count);
          ] );
      ("demand", signal.demand);
    ]

let make_effects_extension signal =
  `Assoc
    [
      ( "stage",
        `Assoc
          [
            ("required", `String signal.stage_required);
            ("actual", `String signal.stage_actual);
          ] );
      ("reason", `String signal.reason);
    ]

let merge_audit signal diag =
  let entries =
    [
      ("bridge.stream.signal.kind", `String "pending");
      ("bridge.stream.signal.reason", `String signal.reason);
      ("bridge.stream.stage.required", `String signal.stage_required);
      ("bridge.stream.stage.actual", `String signal.stage_actual);
    ]
  in
  merge_audit_metadata entries diag

let stream_signal signal : Diagnostic.t list =
  let builder =
    Builder.create ~message:(message_for_signal signal) ~primary:signal.span ()
    |> Builder.set_domain Diagnostic.Runtime
    |> Builder.set_primary_code "bridge.stage.backpressure"
    |> Builder.add_code "effects.contract.stage_mismatch"
    |> Builder.set_severity_hint (Some Diagnostic.Rollback)
  in
  let diagnostic = Builder.build builder in
  let diagnostic =
    diagnostic
    |> set_extension "bridge" (make_bridge_extension signal)
    |> set_extension "effects" (make_effects_extension signal)
    |> set_extension
         "diagnostic.v2"
         (`Assoc
            [
              ("timestamp", `String diagnostic.timestamp);
              ( "codes",
                `List
                  [
                    `String "bridge.stage.backpressure";
                    `String "effects.contract.stage_mismatch";
                  ] );
            ])
    |> merge_audit signal
  in
  [ diagnostic ]
