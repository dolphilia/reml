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

let string_of_audit_store = function
  | AuditStoreTmp -> "tmp"
  | AuditStoreLocal -> "local"
  | AuditStoreCi -> "ci"

let string_of_audit_level = function
  | AuditLevelSummary -> "summary"
  | AuditLevelFull -> "full"
  | AuditLevelDebug -> "debug"

let sanitize_target value =
  let buffer = Buffer.create (String.length value) in
  String.iter
    (fun ch ->
      match ch with
      | '/' | '\\' | ' ' -> Buffer.add_char buffer '_'
      | ':' -> Buffer.add_char buffer '-'
      | c -> Buffer.add_char buffer c)
    value;
  Buffer.contents buffer

let compact_timestamp iso =
  let buffer = Buffer.create (String.length iso) in
  String.iter
    (fun ch ->
      match ch with
      | '-' | ':' -> ()
      | _ -> Buffer.add_char buffer ch)
    iso;
  Buffer.contents buffer

let git_commit_sha () =
  match Sys.getenv_opt "REMLC_GIT_COMMIT" with
  | Some value when String.trim value <> "" -> Some (String.trim value)
  | _ -> (
      try
        let ic = Unix.open_process_in "git rev-parse --short HEAD" in
        let line =
          try Some (String.trim (input_line ic)) with End_of_file -> None
        in
        let status = Unix.close_process_in ic in
        match (status, line) with
        | Unix.WEXITED 0, Some value when String.trim value <> "" ->
            Some value
        | _ -> None
      with _ -> None)

let build_identifier timestamp commit =
  match commit with
  | Some sha when String.trim sha <> "" ->
      Printf.sprintf "%s-%s" timestamp sha
  | _ -> timestamp

let is_explicit_file path =
  Filename.check_suffix path ".json" || Filename.check_suffix path ".jsonl"

let resolve opts =
  let profile = opts.audit_store in
  let audit_level = opts.audit_level in
  let profile_string = string_of_audit_store profile in
  let audit_level_string = string_of_audit_level audit_level in
  let target_slug = sanitize_target opts.target in
  match opts.audit_enabled with
  | false ->
      {
        enabled = false;
        profile;
        profile_string;
        audit_level;
        audit_level_string;
        target = opts.target;
        target_slug;
        timestamp_iso = "";
        timestamp_compact = "";
        build_id = "";
        commit_id = None;
        audit_path = None;
        index_path = None;
        summary_path = None;
        history_path = None;
        failed_dir = None;
        run_dir = None;
        store_root = None;
        audit_dir_override = opts.audit_dir_override;
      }
  | true -> (
      let timestamp_iso = Audit_envelope.iso8601_timestamp () in
      let timestamp_compact = compact_timestamp timestamp_iso in
      let commit_id = git_commit_sha () in
      let build_id = build_identifier timestamp_compact commit_id in
      match opts.emit_audit_path with
      | Some explicit_path ->
          {
            enabled = true;
            profile;
            profile_string;
            audit_level;
            audit_level_string;
            target = opts.target;
            target_slug;
            timestamp_iso;
            timestamp_compact;
            build_id;
            commit_id;
            audit_path = Some explicit_path;
            index_path = None;
            summary_path = None;
            history_path = None;
            failed_dir = None;
            run_dir = None;
            store_root = None;
            audit_dir_override = opts.audit_dir_override;
          }
      | None -> (
          let year =
            if String.length timestamp_iso >= 4 then
              String.sub timestamp_iso 0 4
            else "0000"
          in
          let month =
            if String.length timestamp_iso >= 7 then
              String.sub timestamp_iso 5 2
            else "00"
          in
          let day =
            if String.length timestamp_iso >= 10 then
              String.sub timestamp_iso 8 2
            else "00"
          in
          let resolved =
            match profile with
            | AuditStoreTmp ->
                let root =
                  match opts.audit_dir_override with
                  | Some dir when not (is_explicit_file dir) -> dir
                  | _ -> Filename.concat "tmp" "cli-callconv-out"
                in
                let run_dir = Filename.concat root target_slug in
                let audit_path = Filename.concat run_dir "audit.jsonl" in
                {
                  enabled = true;
                  profile;
                  profile_string;
                  audit_level;
                  audit_level_string;
                  target = opts.target;
                  target_slug;
                  timestamp_iso;
                  timestamp_compact;
                  build_id;
                  commit_id;
                  audit_path = Some audit_path;
                  index_path = None;
                  summary_path = None;
                  history_path = None;
                  failed_dir = None;
                  run_dir = Some run_dir;
                  store_root = Some root;
                  audit_dir_override = opts.audit_dir_override;
                }
            | AuditStoreLocal ->
                let root =
                  match opts.audit_dir_override with
                  | Some dir when not (is_explicit_file dir) -> dir
                  | _ ->
                      Filename.concat
                        (Filename.concat "tooling" "audit-store")
                        "local"
                in
                let run_dir = Filename.concat root timestamp_compact in
                let audit_path = Filename.concat run_dir "audit.jsonl" in
                let index_path = Filename.concat root "index.json" in
                let summary_path = Filename.concat root "summary.md" in
                {
                  enabled = true;
                  profile;
                  profile_string;
                  audit_level;
                  audit_level_string;
                  target = opts.target;
                  target_slug;
                  timestamp_iso;
                  timestamp_compact;
                  build_id;
                  commit_id;
                  audit_path = Some audit_path;
                  index_path = Some index_path;
                  summary_path = Some summary_path;
                  history_path = None;
                  failed_dir = None;
                  run_dir = Some run_dir;
                  store_root = Some root;
                  audit_dir_override = opts.audit_dir_override;
                }
            | AuditStoreCi ->
                let root =
                  match opts.audit_dir_override with
                  | Some dir when not (is_explicit_file dir) -> dir
                  | _ -> Filename.concat "reports" "audit"
                in
                let target_dir = Filename.concat root target_slug in
                let year_dir = Filename.concat target_dir year in
                let month_dir = Filename.concat year_dir month in
                let day_dir = Filename.concat month_dir day in
                let filename = build_id ^ ".jsonl" in
                let audit_path = Filename.concat day_dir filename in
                let index_path = Filename.concat root "index.json" in
                let summary_path = Filename.concat root "summary.md" in
                let history_dir = Filename.concat root "history" in
                let history_path =
                  Filename.concat history_dir (target_slug ^ ".jsonl.gz")
                in
                let failed_dir = Filename.concat (Filename.concat root "failed") build_id in
                {
                  enabled = true;
                  profile;
                  profile_string;
                  audit_level;
                  audit_level_string;
                  target = opts.target;
                  target_slug;
                  timestamp_iso;
                  timestamp_compact;
                  build_id;
                  commit_id;
                  audit_path = Some audit_path;
                  index_path = Some index_path;
                  summary_path = Some summary_path;
                  history_path = Some history_path;
                  failed_dir = Some failed_dir;
                  run_dir = Some day_dir;
                  store_root = Some root;
                  audit_dir_override = opts.audit_dir_override;
                }
          in
          resolved)))

