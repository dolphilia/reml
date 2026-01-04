use reml_ffi_bindgen::{run_bindgen, BindgenError, CliOptions, DiagnosticEntry};
use serde_json::json;
use std::env;
use std::path::PathBuf;

fn main() {
  let args: Vec<String> = env::args().skip(1).collect();
  let cli = match parse_args(&args) {
    Ok(value) => value,
    Err(message) => {
      if message == "help" {
        print_usage();
        return;
      }
      emit_log(json!({
        "event": "bindgen.finish",
        "status": "failed",
        "diagnostics": [DiagnosticEntry {
          code: "ffi.bindgen.config_invalid".to_string(),
          symbol: None,
          c_type: None,
          reason: Some(message),
          hint: Some("--config <path> や --help を確認".to_string()),
        }]
      }));
      std::process::exit(1);
    }
  };

  let config_path = cli
    .config_path
    .clone()
    .unwrap_or_else(|| PathBuf::from("reml-bindgen.toml"));

  emit_log(json!({
    "event": "bindgen.start",
    "config": config_path.to_string_lossy(),
  }));

  match run_bindgen(&config_path, &cli) {
    Ok(result) => {
      emit_log(json!({
        "event": "bindgen.parse",
        "status": "success",
        "headers": result.manifest.headers,
      }));
      emit_log(json!({
        "event": "bindgen.generate",
        "status": "success",
        "output": result.output_path.to_string_lossy(),
        "manifest": result.manifest_path.to_string_lossy(),
      }));
      emit_log(json!({
        "event": "bindgen.finish",
        "status": "success",
        "generated": result.output_path.to_string_lossy(),
        "manifest": result.manifest_path.to_string_lossy(),
        "diagnostics": result.diagnostics,
      }));
    }
    Err(err) => {
      let diagnostics = vec![error_to_diagnostic(&err)];
      emit_log(json!({
        "event": "bindgen.finish",
        "status": "failed",
        "diagnostics": diagnostics,
      }));
      std::process::exit(1);
    }
  }
}

fn parse_args(args: &[String]) -> Result<CliOptions, String> {
  let mut cli = CliOptions::default();
  let mut iter = args.iter().peekable();

  while let Some(arg) = iter.next() {
    match arg.as_str() {
      "--config" => {
        cli.config_path = Some(next_value(&mut iter, "--config")?.into());
      }
      "--header" => {
        cli.headers.push(next_value(&mut iter, "--header")?);
      }
      "--include-path" | "-I" => {
        cli.include_paths
          .push(next_value(&mut iter, "--include-path")?);
      }
      "--compile-commands" => {
        cli.compile_commands = Some(next_value(&mut iter, "--compile-commands")?);
      }
      "--define" | "-D" => {
        cli.defines.push(next_value(&mut iter, "--define")?);
      }
      "--output" => {
        cli.output = Some(next_value(&mut iter, "--output")?);
      }
      "--manifest" => {
        cli.manifest = Some(next_value(&mut iter, "--manifest")?);
      }
      "--exclude" => {
        cli.exclude.push(next_value(&mut iter, "--exclude")?);
      }
      "--help" | "-h" => {
        return Err("help".to_string());
      }
      value if value.starts_with("-I") && value.len() > 2 => {
        cli.include_paths.push(value[2..].to_string());
      }
      value if value.starts_with("-D") && value.len() > 2 => {
        cli.defines.push(value[2..].to_string());
      }
      _ => {
        return Err(format!("不明なオプション: {}", arg));
      }
    }
  }

  Ok(cli)
}

fn next_value<I>(iter: &mut I, flag: &str) -> Result<String, String>
where
  I: Iterator<Item = &'_ String>,
{
  iter
    .next()
    .map(|value| value.to_string())
    .ok_or_else(|| format!("{} の値が必要です", flag))
}

fn emit_log(value: serde_json::Value) {
  if let Ok(line) = serde_json::to_string(&value) {
    println!("{}", line);
  }
}

fn error_to_diagnostic(err: &BindgenError) -> DiagnosticEntry {
  let code = match err {
    BindgenError::ConfigInvalid(_) => "ffi.bindgen.config_invalid",
    BindgenError::ParseFailed(_) => "ffi.bindgen.parse_failed",
    BindgenError::GenerateFailed(_) => "ffi.bindgen.generate_failed",
  };
  DiagnosticEntry {
    code: code.to_string(),
    symbol: None,
    c_type: None,
    reason: Some(err.to_string()),
    hint: Some("reml-bindgen の設定と入力を確認".to_string()),
  }
}

fn print_usage() {
  let usage = r#"reml-bindgen

USAGE:
  reml-bindgen [options]

OPTIONS:
  --config <path>           設定ファイルを指定（既定: reml-bindgen.toml）
  --header <path>           ヘッダを追加（複数指定可）
  --include-path <path>     include パスを追加（-I も可）
  --compile-commands <path> compile_commands.json を指定
  --define <name[=value]>   定義を追加（-D も可）
  --output <path>           出力 .reml
  --manifest <path>         出力 bindings.manifest.json
  --exclude <pattern>       除外パターン（正規表現）
  --help, -h                ヘルプ表示
"#;
  println!("{}", usage);
}
