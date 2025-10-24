(* llvm_toolchain_helpers.ml
 *
 * Linux/macOS でバージョン付き `llc-19` / `llvm-as-19` などを探索し、
 * テストが外部ツールを自動検出できるようにする補助モジュール。
 *)

let is_executable path =
  try
    let stats = Unix.stat path in
    stats.Unix.st_kind = Unix.S_REG
    && (Unix.access path [ Unix.X_OK ]; true)
  with Unix.Unix_error _ -> false

let which command =
  if command = "" then
    None
  else if String.contains command '/' then
    if is_executable command then Some command else None
  else
    let path_env = Option.value ~default:"" (Sys.getenv_opt "PATH") in
    path_env
    |> String.split_on_char ':'
    |> List.find_map (fun dir ->
           if dir = "" then
             None
           else
             let candidate = Filename.concat dir command in
             if is_executable candidate then Some candidate else None)

let resolve_env_command env_var =
  match Sys.getenv_opt env_var with
  | None | Some "" -> None
  | Some value ->
      if Filename.is_implicit value then
        which value
      else if is_executable value then
        Some value
      else
        None

let find_tool ?env_var ~name ~suffixes () =
  let env_candidate =
    match env_var with
    | None -> None
    | Some var -> resolve_env_command var
  in
  match env_candidate with
  | Some path -> path
  | None ->
      let base_candidates =
        name
        :: List.map (fun suffix -> Printf.sprintf "%s-%s" name suffix) suffixes
      in
      match List.find_map which base_candidates with
      | Some path -> path
      | None ->
          let suffixes_str = String.concat ", " suffixes in
          failwith
            (Printf.sprintf
               "%s が見つかりません。PATH に %s または %s-<%s> を追加してください。"
               name name name suffixes_str)

let llc () =
  find_tool ~env_var:"LLVM_LLC" ~name:"llc" ~suffixes:[ "19"; "18"; "17" ] ()

let llvm_as () =
  find_tool ~env_var:"LLVM_AS" ~name:"llvm-as" ~suffixes:[ "19"; "18"; "17" ] ()

let opt () =
  find_tool ~env_var:"LLVM_OPT" ~name:"opt" ~suffixes:[ "19"; "18"; "17" ] ()
