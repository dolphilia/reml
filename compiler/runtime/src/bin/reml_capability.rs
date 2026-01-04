use std::{env, error::Error, process};

use reml_runtime::{
    capability::{CapabilityDescriptor, CapabilityProvider, CapabilityTimestamp},
    CapabilityDescriptorList, CapabilityRegistry,
};
use serde_json::json;

const CAPABILITY_LIST_SCHEMA_VERSION: &str = "3.0.0-alpha";

fn main() {
    if let Err(err) = run_cli() {
        eprintln!("[capability] {err}");
        process::exit(1);
    }
}

fn run_cli() -> Result<(), Box<dyn Error>> {
    let args = env::args().collect::<Vec<_>>();
    let program_name = args
        .get(0)
        .cloned()
        .unwrap_or_else(|| "reml_capability".to_string());
    let mut iter = args.into_iter();
    iter.next(); // discard program name
    let command = iter.next().unwrap_or_else(|| "list".to_string());
    match command.as_str() {
        "list" => run_list(iter.collect()),
        "help" | "--help" | "-h" => {
            print_help(&program_name);
            Ok(())
        }
        other => Err(format!("未知のサブコマンド `{other}` を指定しました").into()),
    }
}

fn run_list(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut format = OutputFormat::Markdown;
    let mut args_iter = args.into_iter();
    while let Some(arg) = args_iter.next() {
        match arg.as_str() {
            "--format" => {
                let value = args_iter
                    .next()
                    .ok_or("--format には json|markdown を指定してください")?;
                format = OutputFormat::parse(&value)?;
            }
            "--json" => format = OutputFormat::Json,
            "--markdown" | "--table" => format = OutputFormat::Markdown,
            "--help" | "-h" => {
                print_list_help();
                return Ok(());
            }
            other => {
                return Err(format!(
                    "list サブコマンドの未知のオプション `{other}` が指定されました"
                )
                .into())
            }
        }
    }

    let registry = CapabilityRegistry::registry();
    let descriptors = registry.describe_all();
    match format {
        OutputFormat::Json => emit_json(&descriptors)?,
        OutputFormat::Markdown => emit_markdown_table(registry.handles_all()),
    }
    Ok(())
}

fn emit_json(list: &CapabilityDescriptorList) -> Result<(), Box<dyn Error>> {
    let payload = json!({
        "schema_version": CAPABILITY_LIST_SCHEMA_VERSION,
        "capabilities": list.iter().cloned().collect::<Vec<_>>(),
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn emit_markdown_table(handles: Vec<reml_runtime::capability::CapabilityHandle>) {
    println!(
        "| Capability | Stage | Effect Scope | Provider | Plugin ID | Bundle ID | Registered At | Manifest Path |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- | --- |");
    for handle in handles {
        let descriptor = handle.descriptor();
        let effects = if descriptor.effect_scope().is_empty() {
            String::from("(none)")
        } else {
            descriptor
                .effect_scope()
                .iter()
                .map(|tag| format!("`{tag}`"))
                .collect::<Vec<_>>()
                .join("<br>")
        };
        let provider = format_provider(descriptor);
        let (plugin_id, bundle_id) = match handle.as_plugin() {
            Some(plugin) => {
                let metadata = plugin.metadata();
                let bundle_id = match (&metadata.bundle_id, &metadata.bundle_version) {
                    (Some(id), Some(version)) => format!("{id}@{version}"),
                    (Some(id), None) => id.to_string(),
                    _ => "-".to_string(),
                };
                (metadata.package.clone(), bundle_id)
            }
            None => ("-".to_string(), "-".to_string()),
        };
        let registered_at = descriptor
            .metadata()
            .last_verified_at
            .map(format_capability_timestamp)
            .unwrap_or_else(|| "-".to_string());
        let manifest_path = descriptor
            .metadata()
            .manifest_path
            .as_ref()
            .map(|path| format!("`{}`", path.display()))
            .unwrap_or_else(|| "-".to_string());
        println!(
            "| `{id}` | `{stage}` | {effects} | {provider} | {plugin_id} | {bundle_id} | {registered_at} | {manifest_path} |",
            id = descriptor.id,
            stage = descriptor.stage().as_str(),
        );
    }
}

fn format_provider(descriptor: &CapabilityDescriptor) -> String {
    match descriptor.metadata().provider {
        CapabilityProvider::Core => "core".to_string(),
        CapabilityProvider::Plugin {
            ref package,
            ref version,
        } => {
            if let Some(version) = version {
                format!("plugin:{package}@{version}")
            } else {
                format!("plugin:{package}")
            }
        }
        CapabilityProvider::ExternalBridge {
            ref name,
            ref version,
        } => {
            if let Some(version) = version {
                format!("bridge:{name}@{version}")
            } else {
                format!("bridge:{name}")
            }
        }
        CapabilityProvider::RuntimeComponent { ref name } => {
            format!("runtime:{name}")
        }
    }
}

fn format_capability_timestamp(timestamp: CapabilityTimestamp) -> String {
    format!(
        "{}.{:09}s (unix)",
        timestamp.seconds,
        timestamp.nanos.max(0)
    )
}

#[derive(Clone, Copy, Debug)]
enum OutputFormat {
    Json,
    Markdown,
}

impl OutputFormat {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        match value.trim().to_ascii_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "markdown" | "table" => Ok(OutputFormat::Markdown),
            other => Err(format!(
                "--format の値 `{other}` は json または markdown を指定してください"
            )
            .into()),
        }
    }
}

fn print_help(program: &str) {
    println!(
        "\
{prog} は Capability Registry の情報を CLI から参照するユーティリティです。

使用方法:
  {prog} [SUBCOMMAND]

SUBCOMMAND:
  list      登録済み Capability を一覧表示します（既定）
  help      本メッセージを表示します

`{prog} list --help` で list の詳細オプションを確認できます。
",
        prog = program
    );
}

fn print_list_help() {
    println!(
        "\
usage: reml_capability list [OPTIONS]

OPTIONS:
  --format <json|markdown>   出力形式を指定（既定: markdown）
  --json                     --format json と同じ
  --markdown                 --format markdown と同じ
"
    );
}
