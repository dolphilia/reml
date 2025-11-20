use serde::Deserialize;
use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

fn main() {
    if let Err(err) = run() {
        eprintln!("xtask エラー: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let Some(cmd) = args.next() else {
        print_usage();
        return Ok(());
    };
    let rest: Vec<String> = args.collect();
    match cmd.as_str() {
        "prelude-audit" => run_prelude_audit(rest.into_iter()),
        _ => {
            eprintln!("未知のサブコマンド: {cmd}");
            print_usage();
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("usage: cargo xtask <subcommand>");
    eprintln!(
        "  prelude-audit [--inventory <path>] [--wbs <id>] [--baseline <path>] \
         [--section <Option|Result|Iter|Collector>] [--module <name>] [--strict]"
    );
}

fn run_prelude_audit<I>(args: I) -> Result<(), Box<dyn Error>>
where
    I: Iterator<Item = String>,
{
    let mut inventory_path =
        PathBuf::from("docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml");
    let mut strict = false;
    let mut wbs_filter: Option<String> = None;
    let mut baseline: Option<String> = None;
    let mut section_filter: Option<SectionFilter> = None;
    let mut module_filters: Vec<String> = Vec::new();

    let mut iter = args.peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--inventory" => {
                let Some(value) = iter.next() else {
                    return Err("`--inventory` にパスが指定されていません".into());
                };
                inventory_path = PathBuf::from(value);
            }
            "--wbs" => {
                let Some(value) = iter.next() else {
                    return Err("`--wbs` に ID が指定されていません".into());
                };
                wbs_filter = Some(value);
            }
            "--baseline" => {
                let Some(value) = iter.next() else {
                    return Err("`--baseline` にパスが指定されていません".into());
                };
                baseline = Some(value);
            }
            "--section" => {
                let Some(value) = iter.next() else {
                    return Err("`--section` に値が指定されていません".into());
                };
                section_filter = Some(
                    SectionFilter::parse(&value)
                        .map_err(|msg| format!("`--section` の値が不正です: {msg}"))?,
                );
            }
            "--module" => {
                let Some(value) = iter.next() else {
                    return Err("`--module` に値が指定されていません".into());
                };
                module_filters.push(value);
            }
            "--strict" => strict = true,
            other => {
                return Err(format!("未対応の引数: {other}").into());
            }
        }
    }

    let inventory_text = fs::read_to_string(&inventory_path)?;
    let inventory: Inventory = toml::from_str(&inventory_text)?;
    let mut entries: Vec<&ApiEntry> = inventory.api.iter().collect();
    if let Some(filter) = &wbs_filter {
        entries.retain(|entry| entry.wbs.as_deref() == Some(filter.as_str()));
    }
    if let Some(section) = &section_filter {
        entries.retain(|entry| section.matches(&entry.module));
    }
    if !module_filters.is_empty() {
        entries.retain(|entry| {
            let module_name = entry.module.to_ascii_lowercase();
            module_filters
                .iter()
                .any(|filter| module_name == filter.to_ascii_lowercase())
        });
    }

    println!("== Core Prelude API Audit ==");
    println!("inventory : {}", display_rel(&inventory_path));
    if let Some(baseline_path) = &baseline {
        println!("baseline  : {baseline_path}");
    }
    if let Some(section) = &section_filter {
        println!("section   : {}", section.label());
    }
    if !module_filters.is_empty() {
        println!("modules   : {}", module_filters.join(", "));
    }
    if let Some(filter) = &wbs_filter {
        println!("wbs filter: {filter}");
    }
    println!("---");

    if entries.is_empty() {
        println!("検査対象の API が見つかりませんでした。");
        return Ok(());
    }

    let total = entries.len();
    let mut missing = Vec::new();
    for entry in entries {
        let effect = entry.effect.as_deref().unwrap_or("n/a");
        let rust_status = entry.rust_status.as_deref().unwrap_or("unknown");
        println!(
            "- {}/{} (effect={}) => rust_status={}",
            entry.module, entry.name, effect, rust_status
        );
        if rust_status != "implemented" {
            missing.push(format!("{}::{}", entry.module, entry.name));
        }
    }

    println!("---");
    let completed = total.saturating_sub(missing.len());
    println!(
        "対象 {} 件 / 実装済み {} 件 / 未完 {} 件",
        total,
        completed,
        missing.len()
    );

    if strict && !missing.is_empty() {
        eprintln!("未実装 API: {}", missing.join(", "));
        std::process::exit(1);
    }

    Ok(())
}

fn display_rel(path: &Path) -> String {
    path.strip_prefix(env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

#[derive(Deserialize)]
struct Inventory {
    #[allow(dead_code)]
    meta: Option<Meta>,
    api: Vec<ApiEntry>,
}

#[derive(Deserialize)]
struct Meta {
    #[allow(dead_code)]
    schema: Option<String>,
    #[allow(dead_code)]
    spec: Option<String>,
    #[allow(dead_code)]
    last_updated: Option<String>,
}

#[derive(Deserialize)]
struct ApiEntry {
    module: String,
    name: String,
    effect: Option<String>,
    rust_status: Option<String>,
    wbs: Option<String>,
    #[allow(dead_code)]
    notes: Option<String>,
}

#[derive(Clone, Copy, Debug)]
enum SectionFilter {
    Option,
    Result,
    Iter,
    Collector,
}

impl SectionFilter {
    fn parse(value: &str) -> Result<Self, String> {
        let normalized = value.to_ascii_lowercase();
        match normalized.as_str() {
            "option" | "options" => Ok(Self::Option),
            "result" | "results" => Ok(Self::Result),
            "iter" | "iters" | "iterator" | "iteration" => Ok(Self::Iter),
            "collector" | "collectors" => Ok(Self::Collector),
            other => Err(format!(
                "{other}（許可されている値: Option, Result, Iter, Collector）"
            )),
        }
    }

    fn matches(&self, module: &str) -> bool {
        let module_lower = module.to_ascii_lowercase();
        self.modules()
            .iter()
            .any(|candidate| module_lower == candidate.to_ascii_lowercase())
    }

    fn modules(&self) -> &'static [&'static str] {
        match self {
            SectionFilter::Option => &["Option"],
            SectionFilter::Result => &["Result"],
            SectionFilter::Iter => &["Iter", "Collector"],
            SectionFilter::Collector => &["Collector"],
        }
    }

    fn label(&self) -> &'static str {
        match self {
            SectionFilter::Option => "Option",
            SectionFilter::Result => "Result",
            SectionFilter::Iter => "Iter (Iter + Collector)",
            SectionFilter::Collector => "Collector",
        }
    }
}
