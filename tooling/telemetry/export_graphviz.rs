use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

type CliResult<T> = Result<T, String>;

struct CliArgs {
    input: PathBuf,
    dot_out: Option<PathBuf>,
    svg_out: Option<PathBuf>,
    graph_name: String,
}

#[derive(Debug, Deserialize, Default)]
struct Graph {
    #[serde(default)]
    nodes: Vec<GraphNode>,
    #[serde(default)]
    edges: Vec<GraphEdge>,
}

impl Graph {
    fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.edges.is_empty()
    }
}

#[derive(Debug, Deserialize)]
struct GraphNode {
    #[serde(default)]
    id: usize,
    #[serde(default)]
    label: String,
    #[serde(default)]
    kind: GraphNodeKind,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum GraphNodeKind {
    Type,
    Capability,
    Implementation,
    #[serde(other)]
    Other,
}

impl Default for GraphNodeKind {
    fn default() -> Self {
        GraphNodeKind::Type
    }
}

#[derive(Debug, Deserialize)]
struct GraphEdge {
    #[serde(default)]
    from: usize,
    #[serde(default)]
    to: usize,
    #[serde(default)]
    kind: GraphEdgeKind,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum GraphEdgeKind {
    Equal,
    Capability,
    Implementation,
    #[serde(other)]
    Other,
}

impl Default for GraphEdgeKind {
    fn default() -> Self {
        GraphEdgeKind::Equal
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("[export_graphviz] {err}");
        std::process::exit(1);
    }
}

fn run() -> CliResult<()> {
    let args = parse_args()?;
    let graph = read_graph(&args.input)?;
    if graph.is_empty() {
        return Err(format!(
            "グラフにノードが存在しません: {}",
            args.input.display()
        ));
    }
    let graph_name = sanitize_name(&args.graph_name);
    let dot_text = build_dot(&graph, &graph_name);

    let dot_path = args
        .dot_out
        .clone()
        .unwrap_or_else(|| args.input.with_extension("dot"));
    write_text(&dot_path, &dot_text)?;

    if let Some(svg_path) = args.svg_out.clone() {
        run_dot(&dot_path, &svg_path)?;
    }

    Ok(())
}

fn parse_args() -> CliResult<CliArgs> {
    let mut args = env::args().skip(1);
    let mut input: Option<PathBuf> = None;
    let mut dot_out: Option<PathBuf> = None;
    let mut svg_out: Option<PathBuf> = None;
    let mut graph_name: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dot-out" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--dot-out の値が指定されていません".to_string())?;
                dot_out = Some(PathBuf::from(value));
            }
            "--svg-out" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--svg-out の値が指定されていません".to_string())?;
                svg_out = Some(PathBuf::from(value));
            }
            "--graph-name" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--graph-name の値が指定されていません".to_string())?;
                graph_name = Some(value);
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            value if value.starts_with("--") => {
                return Err(format!("未知のオプションです: {value}"));
            }
            value => {
                if input.is_some() {
                    return Err("入力ファイルは 1 つのみ指定してください".to_string());
                }
                input = Some(PathBuf::from(value));
            }
        }
    }

    let input = input.ok_or_else(|| "Telemetry JSON のパスを指定してください".to_string())?;
    let graph_name = graph_name.unwrap_or_else(|| "ConstraintGraph".to_string());
    Ok(CliArgs {
        input,
        dot_out,
        svg_out,
        graph_name,
    })
}

fn print_help() {
    println!("TraitResolutionTelemetry JSON を Graphviz DOT/SVG へ変換します。");
    println!("usage: export_graphviz [OPTIONS] <input.json>");
    println!("  --dot-out <PATH>     DOT の出力先 (省略時: <input>.dot)");
    println!("  --svg-out <PATH>     SVG の出力先 (Graphviz dot が必要)");
    println!("  --graph-name <NAME>  Graphviz 上のグラフ名");
}

fn read_graph(path: &Path) -> CliResult<Graph> {
    let raw = fs::read_to_string(path).map_err(|err| {
        format!(
            "テレメトリ JSON を読み込めません ({}): {err}",
            path.display()
        )
    })?;
    let value: serde_json::Value = serde_json::from_str(&raw).map_err(|err| {
        format!(
            "JSON の解析に失敗しました ({}): {err}",
            path.display()
        )
    })?;
    if let Some(graph_value) = value.get("graph") {
        serde_json::from_value(graph_value.clone())
            .map_err(|err| format!("graph フィールドの読み取りに失敗しました: {err}"))
    } else {
        serde_json::from_value(value)
            .map_err(|err| format!("入力全体を Graph として解析できません: {err}"))
    }
}

fn write_text(path: &Path, text: &str) -> CliResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "ディレクトリを作成できません ({}): {err}",
                parent.display()
            )
        })?;
    }
    fs::write(path, text).map_err(|err| {
        format!("DOT の書き込みに失敗しました ({}): {err}", path.display())
    })
}

fn run_dot(dot_path: &Path, svg_path: &Path) -> CliResult<()> {
    if let Some(parent) = svg_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "ディレクトリを作成できません ({}): {err}",
                parent.display()
            )
        })?;
    }
    let dot_in = dot_path
        .to_str()
        .ok_or_else(|| format!("DOT パスを UTF-8 へ変換できません: {}", dot_path.display()))?;
    let svg_out = svg_path
        .to_str()
        .ok_or_else(|| format!("SVG パスを UTF-8 へ変換できません: {}", svg_path.display()))?;
    Command::new("dot")
        .args(["-Tsvg", dot_in, "-o", svg_out])
        .status()
        .map_err(|err| format!("Graphviz dot コマンドの起動に失敗しました: {err}"))?
        .success()
        .then_some(())
        .ok_or_else(|| "dot コマンドがエラー終了しました".to_string())
}

fn sanitize_name(name: &str) -> String {
    let mut cleaned = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            cleaned.push(ch);
        } else {
            cleaned.push('_');
        }
    }
    if cleaned.is_empty() {
        "ConstraintGraph".to_string()
    } else {
        cleaned
    }
}

fn build_dot(graph: &Graph, graph_name: &str) -> String {
    let mut lines = Vec::new();
    lines.push(format!("digraph {} {{", graph_name));
    lines.push("  rankdir=LR;".to_string());
    for node in &graph.nodes {
        let node_id = format!("n{}", node.id);
        let label = escape_label(&node.label);
        let shape = match node.kind {
            GraphNodeKind::Type => "ellipse",
            GraphNodeKind::Capability => "box",
            GraphNodeKind::Implementation => "diamond",
            GraphNodeKind::Other => "ellipse",
        };
        lines.push(format!("  {} [label=\"{}\", shape={}];", node_id, label, shape));
    }
    for edge in &graph.edges {
        let from = format!("n{}", edge.from);
        let to = format!("n{}", edge.to);
        let attr = match edge.kind {
            GraphEdgeKind::Equal => String::new(),
            _ => format!(" [label=\"{}\"]", edge_kind_label(&edge.kind)),
        };
        lines.push(format!("  {} -> {}{};", from, to, attr));
    }
    lines.push("}".to_string());
    lines.join("\n")
}

fn edge_kind_label(kind: &GraphEdgeKind) -> &'static str {
    match kind {
        GraphEdgeKind::Equal => "",
        GraphEdgeKind::Capability => "capability",
        GraphEdgeKind::Implementation => "implementation",
        GraphEdgeKind::Other => "other",
    }
}

fn escape_label(label: &str) -> String {
    label
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}
