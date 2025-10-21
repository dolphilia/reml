open Options

type t = {
  enabled : bool;
  profile : audit_store;
  profile_string : string;
  audit_level : audit_level;
  audit_level_string : string;
  target : string;
  target_slug : string;
  timestamp_iso : string;
  timestamp_compact : string;
  build_id : string;
  commit_id : string option;
  audit_path : string option;
  index_path : string option;
  summary_path : string option;
  history_path : string option;
  failed_dir : string option;
  run_dir : string option;
  store_root : string option;
  audit_dir_override : string option;
}

val string_of_audit_store : audit_store -> string
val string_of_audit_level : audit_level -> string
val resolve : Options.options -> t
