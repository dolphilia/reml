type outcome =
  | Success
  | Failure

val append_events :
  Audit_path_resolver.t ->
  ?outcome:outcome ->
  Audit_envelope.event list ->
  unit
