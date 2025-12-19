use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Deserialize, Clone)]
pub struct BindgenConfig {
  pub headers: Vec<String>,
  pub include_paths: Vec<String>,
  pub compile_commands: Option<String>,
  pub defines: Vec<String>,
  pub output: String,
  pub manifest: String,
  pub exclude: Vec<String>,
}

impl BindgenConfig {
  fn apply_overrides(&mut self, cli: &CliOptions) {
    self.headers.extend(cli.headers.clone());
    self.include_paths.extend(cli.include_paths.clone());
    self.defines.extend(cli.defines.clone());
    self.exclude.extend(cli.exclude.clone());

    if let Some(value) = &cli.compile_commands {
      self.compile_commands = Some(value.clone());
    }
    if let Some(value) = &cli.output {
      self.output = value.clone();
    }
    if let Some(value) = &cli.manifest {
      self.manifest = value.clone();
    }
  }
}

impl Default for BindgenConfig {
  fn default() -> Self {
    Self {
      headers: Vec::new(),
      include_paths: Vec::new(),
      compile_commands: None,
      defines: Vec::new(),
      output: String::new(),
      manifest: String::new(),
      exclude: Vec::new(),
    }
  }
}

#[derive(Debug, Default, Clone)]
pub struct CliOptions {
  pub config_path: Option<PathBuf>,
  pub headers: Vec<String>,
  pub include_paths: Vec<String>,
  pub compile_commands: Option<String>,
  pub defines: Vec<String>,
  pub output: Option<String>,
  pub manifest: Option<String>,
  pub exclude: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct DiagnosticEntry {
  pub code: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub symbol: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub c_type: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub reason: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub hint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Manifest {
  pub version: String,
  pub headers: Vec<String>,
  pub generated: String,
  pub input_hash: String,
  pub types: Vec<ManifestType>,
  pub diagnostics: Vec<DiagnosticEntry>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct ManifestType {
  pub c: String,
  pub reml: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub qualifiers: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct RunResult {
  pub output_path: PathBuf,
  pub manifest_path: PathBuf,
  pub diagnostics: Vec<DiagnosticEntry>,
  pub manifest: Manifest,
}

#[derive(Debug, Error)]
pub enum BindgenError {
  #[error("設定ファイルが不正です: {0}")]
  ConfigInvalid(String),
  #[error("ヘッダ解析に失敗しました: {0}")]
  ParseFailed(String),
  #[error("生成に失敗しました: {0}")]
  GenerateFailed(String),
}

pub fn load_config(config_path: &Path) -> Result<BindgenConfig, BindgenError> {
  let content = fs::read_to_string(config_path)
    .map_err(|err| BindgenError::ConfigInvalid(err.to_string()))?;
  let mut config: BindgenConfig = toml::from_str(&content)
    .map_err(|err| BindgenError::ConfigInvalid(err.to_string()))?;

  if config.headers.is_empty() {
    return Err(BindgenError::ConfigInvalid("headers が空です".to_string()));
  }
  if config.include_paths.is_empty() {
    return Err(BindgenError::ConfigInvalid("include_paths が空です".to_string()));
  }
  if config.output.trim().is_empty() {
    return Err(BindgenError::ConfigInvalid("output が空です".to_string()));
  }
  if config.manifest.trim().is_empty() {
    return Err(BindgenError::ConfigInvalid("manifest が空です".to_string()));
  }

  Ok(config)
}

pub fn run_bindgen(config_path: &Path, cli: &CliOptions) -> Result<RunResult, BindgenError> {
  let mut config = load_config(config_path)?;
  config.apply_overrides(cli);

  if config.headers.is_empty() {
    return Err(BindgenError::ConfigInvalid("headers が空です".to_string()));
  }
  if config.include_paths.is_empty() {
    return Err(BindgenError::ConfigInvalid("include_paths が空です".to_string()));
  }
  if config.output.trim().is_empty() {
    return Err(BindgenError::ConfigInvalid("output が空です".to_string()));
  }
  if config.manifest.trim().is_empty() {
    return Err(BindgenError::ConfigInvalid("manifest が空です".to_string()));
  }

  let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
  let output_path = resolve_path(config_dir, &config.output);
  let manifest_path = resolve_path(config_dir, &config.manifest);
  let header_paths = resolve_paths(config_dir, &config.headers);

  let input_hash = calculate_input_hash(&config, &header_paths);

  let exclude_patterns = compile_excludes(&config.exclude);

  let mut diagnostics = Vec::new();
  let mut functions = Vec::new();
  let mut types = Vec::new();
  let mut types_seen = BTreeSet::new();

  for header in &header_paths {
    let content = fs::read_to_string(header)
      .map_err(|err| BindgenError::ParseFailed(err.to_string()))?;
    let parsed = parse_header(&content, header, &exclude_patterns, &mut diagnostics)?;
    for func in parsed.functions {
      for ty in func.types {
        if types_seen.insert((ty.c.clone(), ty.reml.clone(), ty.qualifiers.clone())) {
          types.push(ty);
        }
      }
      functions.push(func);
    }
  }

  let module_name = infer_module_name(&output_path);
  let reml_source = render_reml(&module_name, &functions);

  write_file(&output_path, &reml_source, &mut diagnostics)?;

  let manifest = Manifest {
    version: "0.1".to_string(),
    headers: header_paths
      .iter()
      .map(|path| path.to_string_lossy().to_string())
      .collect(),
    generated: output_path.to_string_lossy().to_string(),
    input_hash: input_hash.clone(),
    types,
    diagnostics: diagnostics.clone(),
  };

  let manifest_json = serde_json::to_string_pretty(&manifest)
    .map_err(|err| BindgenError::GenerateFailed(err.to_string()))?;
  write_file(&manifest_path, &manifest_json, &mut diagnostics)?;

  Ok(RunResult {
    output_path,
    manifest_path,
    diagnostics,
    manifest,
  })
}

fn resolve_path(base: &Path, value: &str) -> PathBuf {
  let path = Path::new(value);
  if path.is_absolute() {
    path.to_path_buf()
  } else {
    base.join(path)
  }
}

fn resolve_paths(base: &Path, values: &[String]) -> Vec<PathBuf> {
  values.iter().map(|value| resolve_path(base, value)).collect()
}

fn calculate_input_hash(config: &BindgenConfig, header_paths: &[PathBuf]) -> String {
  let mut hasher = Sha256::new();
  hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
  hasher.update(b"\n");
  for header in header_paths {
    hasher.update(header.to_string_lossy().as_bytes());
    hasher.update(b"\n");
  }
  for include in &config.include_paths {
    hasher.update(include.as_bytes());
    hasher.update(b"\n");
  }
  if let Some(commands) = &config.compile_commands {
    hasher.update(commands.as_bytes());
    hasher.update(b"\n");
  }
  for define in &config.defines {
    hasher.update(define.as_bytes());
    hasher.update(b"\n");
  }
  for exclude in &config.exclude {
    hasher.update(exclude.as_bytes());
    hasher.update(b"\n");
  }
  let digest = hasher.finalize();
  let mut hex = String::new();
  for byte in digest.iter().take(8) {
    hex.push_str(&format!("{:02x}", byte));
  }
  hex
}

fn compile_excludes(excludes: &[String]) -> Vec<Regex> {
  excludes
    .iter()
    .filter_map(|pattern| Regex::new(pattern).ok())
    .collect()
}

#[derive(Debug)]
struct ParsedHeader {
  functions: Vec<ParsedFunction>,
}

#[derive(Debug, Clone)]
struct ParsedFunction {
  name: String,
  params: Vec<ParsedParam>,
  return_type: ParsedType,
  types: Vec<ManifestType>,
}

#[derive(Debug, Clone)]
struct ParsedParam {
  name: Option<String>,
  ty: ParsedType,
}

#[derive(Debug, Clone)]
struct ParsedType {
  c_type: String,
  reml: String,
  manifest: String,
  qualifiers: Vec<String>,
}

fn parse_header(
  content: &str,
  header_path: &Path,
  excludes: &[Regex],
  diagnostics: &mut Vec<DiagnosticEntry>,
) -> Result<ParsedHeader, BindgenError> {
  let function_re = Regex::new(r"^\s*([A-Za-z_][\w\s\*]*?)\s+([A-Za-z_]\w*)\s*\(([^)]*)\)\s*;")
    .map_err(|err| BindgenError::ParseFailed(err.to_string()))?;

  let mut functions = Vec::new();
  for line in content.lines() {
    let trimmed = strip_line_comment(line).trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
      continue;
    }
    if let Some(caps) = function_re.captures(trimmed) {
      let return_raw = caps.get(1).unwrap().as_str();
      let name = caps.get(2).unwrap().as_str();
      if excludes.iter().any(|regex| regex.is_match(name)) {
        continue;
      }
      let params_raw = caps.get(3).unwrap().as_str();

      let mut local_diagnostics = Vec::new();
      let return_type = match parse_type(return_raw, name, &mut local_diagnostics) {
        Some(value) => value,
        None => {
          diagnostics.append(&mut local_diagnostics);
          continue;
        }
      };

      let params = match parse_params(params_raw, name, &mut local_diagnostics) {
        Some(value) => value,
        None => {
          diagnostics.append(&mut local_diagnostics);
          continue;
        }
      };

      let mut types = Vec::new();
      let mut types_seen = BTreeSet::new();
      collect_type_entry(&return_type, &mut types, &mut types_seen);
      for param in &params {
        collect_type_entry(&param.ty, &mut types, &mut types_seen);
      }

      diagnostics.append(&mut local_diagnostics);
      functions.push(ParsedFunction {
        name: name.to_string(),
        params,
        return_type,
        types,
      });
    }
  }

  Ok(ParsedHeader { functions })
}

fn collect_type_entry(
  parsed: &ParsedType,
  types: &mut Vec<ManifestType>,
  types_seen: &mut BTreeSet<(String, String, Option<Vec<String>>)>,
) {
  let qualifiers = if parsed.qualifiers.is_empty() {
    None
  } else {
    Some(parsed.qualifiers.clone())
  };
  let entry = ManifestType {
    c: parsed.c_type.clone(),
    reml: parsed.manifest.clone(),
    qualifiers,
  };
  if types_seen.insert((entry.c.clone(), entry.reml.clone(), entry.qualifiers.clone())) {
    types.push(entry);
  }
}

fn strip_line_comment(line: &str) -> &str {
  match line.find("//") {
    Some(index) => &line[..index],
    None => line,
  }
}

fn parse_params(
  raw: &str,
  symbol: &str,
  diagnostics: &mut Vec<DiagnosticEntry>,
) -> Option<Vec<ParsedParam>> {
  let trimmed = raw.trim();
  if trimmed.is_empty() || trimmed == "void" {
    return Some(Vec::new());
  }
  if trimmed.contains("...") {
    diagnostics.push(DiagnosticEntry {
      code: "ffi.bindgen.unknown_type".to_string(),
      symbol: Some(symbol.to_string()),
      c_type: Some(trimmed.to_string()),
      reason: Some("unsupported_variadic".to_string()),
      hint: Some("phase2".to_string()),
    });
    return None;
  }

  let mut params = Vec::new();
  for chunk in trimmed.split(',') {
    let chunk = chunk.trim();
    if chunk.is_empty() {
      continue;
    }
    let (type_part, name_part) = split_type_and_name(chunk);
    let parsed = match parse_type(&type_part, symbol, diagnostics) {
      Some(value) => value,
      None => return None,
    };
    params.push(ParsedParam {
      name: name_part,
      ty: parsed,
    });
  }

  Some(params)
}

fn split_type_and_name(raw: &str) -> (String, Option<String>) {
  let spaced = raw.replace('*', " * ");
  let tokens: Vec<&str> = spaced.split_whitespace().collect();
  if tokens.is_empty() {
    return (raw.to_string(), None);
  }

  let qualifiers = ["const", "volatile", "restrict"];
  let base_tokens = [
    "bool",
    "char",
    "signed",
    "unsigned",
    "short",
    "int",
    "long",
    "size_t",
    "intptr_t",
    "uintptr_t",
    "float",
    "double",
    "void",
  ];

  let last = tokens[tokens.len() - 1];
  let is_name = !qualifiers.contains(&last)
    && !base_tokens.contains(&last)
    && last != "*"
    && !last.contains('*');

  if tokens.len() >= 2 && is_name {
    let type_tokens = &tokens[..tokens.len() - 1];
    return (type_tokens.join(" "), Some(last.to_string()));
  }

  (raw.to_string(), None)
}

fn parse_type(
  raw: &str,
  symbol: &str,
  diagnostics: &mut Vec<DiagnosticEntry>,
) -> Option<ParsedType> {
  let trimmed = raw.trim();
  if trimmed.contains('[') {
    diagnostics.push(DiagnosticEntry {
      code: "ffi.bindgen.unknown_type".to_string(),
      symbol: Some(symbol.to_string()),
      c_type: Some(trimmed.to_string()),
      reason: Some("unsupported_array".to_string()),
      hint: Some("phase2".to_string()),
    });
    return None;
  }
  if trimmed.contains("(*") {
    diagnostics.push(DiagnosticEntry {
      code: "ffi.bindgen.unknown_type".to_string(),
      symbol: Some(symbol.to_string()),
      c_type: Some(trimmed.to_string()),
      reason: Some("unsupported_fn_ptr".to_string()),
      hint: Some("phase2".to_string()),
    });
    return None;
  }

  let spaced = trimmed.replace('*', " * ");
  let tokens: Vec<&str> = spaced.split_whitespace().collect();
  if tokens.is_empty() {
    diagnostics.push(DiagnosticEntry {
      code: "ffi.bindgen.unknown_type".to_string(),
      symbol: Some(symbol.to_string()),
      c_type: Some(trimmed.to_string()),
      reason: Some("empty".to_string()),
      hint: Some("phase2".to_string()),
    });
    return None;
  }

  let mut qualifiers = Vec::new();
  let mut base_tokens = Vec::new();
  let mut pointer_count = 0;
  for token in tokens {
    match token {
      "const" | "volatile" | "restrict" => qualifiers.push(token.to_string()),
      "*" => pointer_count += 1,
      _ => base_tokens.push(token),
    }
  }

  if base_tokens.is_empty() {
    diagnostics.push(DiagnosticEntry {
      code: "ffi.bindgen.unknown_type".to_string(),
      symbol: Some(symbol.to_string()),
      c_type: Some(trimmed.to_string()),
      reason: Some("unsupported_type".to_string()),
      hint: Some("phase2".to_string()),
    });
    return None;
  }

  let base = base_tokens.join(" ");
  let mapped = match base.as_str() {
    "bool" => Some(("Bool", "bool")),
    "char" => Some(("I8", "i8")),
    "signed char" => Some(("I8", "i8")),
    "unsigned char" => Some(("U8", "u8")),
    "short" => Some(("I16", "i16")),
    "unsigned short" => Some(("U16", "u16")),
    "int" => Some(("I32", "i32")),
    "unsigned int" => Some(("U32", "u32")),
    "long" => Some(("I64", "i64")),
    "unsigned long" => Some(("U64", "u64")),
    "long long" => Some(("I64", "i64")),
    "unsigned long long" => Some(("U64", "u64")),
    "size_t" => Some(("USize", "usize")),
    "intptr_t" => Some(("ISize", "isize")),
    "uintptr_t" => Some(("USize", "usize")),
    "float" => Some(("F32", "f32")),
    "double" => Some(("F64", "f64")),
    "void" => Some(("Unit", "()")),
    _ => None,
  };

  let (manifest_base, reml_base) = match mapped {
    Some(value) => value,
    None => {
      diagnostics.push(DiagnosticEntry {
        code: "ffi.bindgen.unknown_type".to_string(),
        symbol: Some(symbol.to_string()),
        c_type: Some(trimmed.to_string()),
        reason: Some("unsupported_type".to_string()),
        hint: Some("phase2".to_string()),
      });
      return None;
    }
  };

  let mut manifest_type = manifest_base.to_string();
  let mut reml_type = reml_base.to_string();
  for _ in 0..pointer_count {
    manifest_type = format!("Ptr<{}>", manifest_type);
    reml_type = format!("Ptr<{}>", reml_type);
  }

  Some(ParsedType {
    c_type: trimmed.to_string(),
    reml: reml_type,
    manifest: manifest_type,
    qualifiers,
  })
}

fn render_reml(module_name: &str, functions: &[ParsedFunction]) -> String {
  let mut output = String::new();
  output.push_str("module ");
  output.push_str(module_name);
  output.push_str("\n\n");
  output.push_str("// generated by reml-bindgen\n");
  output.push_str("extern \"C\" {\n");
  for func in functions {
    output.push_str("  fn ");
    output.push_str(&func.name);
    output.push('(');
    for (index, param) in func.params.iter().enumerate() {
      if index > 0 {
        output.push_str(", ");
      }
      if let Some(name) = &param.name {
        output.push_str(name);
        output.push_str(": ");
      }
      output.push_str(&param.ty.reml);
    }
    output.push_str(") -> ");
    output.push_str(&func.return_type.reml);
    output.push_str(";\n");
  }
  output.push_str("}\n");
  output
}

fn infer_module_name(output_path: &Path) -> String {
  let repo_root = find_repo_root(output_path)
    .unwrap_or_else(|| output_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf());
  let relative = output_path
    .strip_prefix(&repo_root)
    .unwrap_or(output_path);
  let mut module = relative
    .to_string_lossy()
    .replace(['/', '\\'], ".");
  if let Some(stripped) = module.strip_suffix(".reml") {
    module = stripped.to_string();
  }
  module
}

fn find_repo_root(start: &Path) -> Option<PathBuf> {
  let mut current = if start.is_dir() {
    start.to_path_buf()
  } else {
    start.parent()?.to_path_buf()
  };
  loop {
    let agent = current.join("AGENTS.md");
    let git = current.join(".git");
    if agent.exists() || git.exists() {
      return Some(current);
    }
    if !current.pop() {
      break;
    }
  }
  None
}

fn write_file(
  path: &Path,
  content: &str,
  diagnostics: &mut Vec<DiagnosticEntry>,
) -> Result<(), BindgenError> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)
      .map_err(|err| BindgenError::GenerateFailed(err.to_string()))?;
  }
  if path.exists() {
    diagnostics.push(DiagnosticEntry {
      code: "ffi.bindgen.output_overwrite".to_string(),
      symbol: None,
      c_type: None,
      reason: Some("output_exists".to_string()),
      hint: Some("overwrite".to_string()),
    });
  }
  fs::write(path, content).map_err(|err| BindgenError::GenerateFailed(err.to_string()))
}
