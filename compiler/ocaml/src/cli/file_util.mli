val ensure_directory : string -> unit
(** [ensure_directory path] は指定されたパスのディレクトリを作成する。
    既に存在する場合はそのまま利用し、存在しない場合は親ディレクトリから再帰的に生成する。
    ファイルが存在してディレクトリでない場合は [invalid_arg] を送出する。 *)
