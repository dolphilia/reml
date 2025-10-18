(* Cli.File_util — 出力ディレクトリ操作ヘルパー
 *
 * --typeclass-mode=both で辞書版とモノモルフィック版を切り替える際に
 * 必要となる出力ディレクトリの生成を担当する。
 *)

let rec ensure_directory path =
  if path = "." || path = "" then ()
  else if Sys.file_exists path then (
    if not (Sys.is_directory path) then
      invalid_arg (Printf.sprintf "\"%s\" はディレクトリではありません" path))
  else
    let parent = Filename.dirname path in
    if parent <> path then ensure_directory parent;
    Unix.mkdir path 0o755
