use reml_frontend::parser::ast::Module;
use reml_frontend::parser::ParserDriver;
use std::fs;
use std::path::{Path, PathBuf};

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

pub fn parse_example_module(relative_path: &str) -> Module {
    let source_path = repo_root().join(relative_path);
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let result = ParserDriver::parse(&source);
    if !result.diagnostics.is_empty() {
        let messages = result
            .diagnostics
            .iter()
            .map(|diag| diag.message.clone())
            .collect::<Vec<_>>();
        panic!(
            "unexpected parser diagnostics for {}: {:?}",
            relative_path, messages
        );
    }
    result
        .value
        .unwrap_or_else(|| panic!("parser did not return a module for {relative_path}"))
}

pub fn parse_example_module_with_diagnostics(relative_path: &str) -> (Module, Vec<String>) {
    let source_path = repo_root().join(relative_path);
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let result = ParserDriver::parse(&source);
    let messages = result
        .diagnostics
        .iter()
        .map(|diag| diag.message.clone())
        .collect::<Vec<_>>();
    let module = result
        .value
        .unwrap_or_else(|| panic!("parser did not return a module for {relative_path}"));
    (module, messages)
}
