type stream_signal = {
  bridge_id : string;
  span : Diagnostic.span;
  policy : string;
  reason : string;
  demand : Yojson.Basic.t;
  await_count : int;
  resume_count : int;
  backpressure_events : int;
  stage_required : string;
  stage_actual : string;
}

val stream_signal : stream_signal -> Diagnostic.t list
