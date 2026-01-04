//! `panic!` / `unwrap_unchecked` の使用を監視し、Core Prelude 実装が
//! `effect {debug}` のみで発火するというポリシーを守っているか確認する。
use std::{
    fs,
    path::{Path, PathBuf},
};

const TARGET_DIRECTORIES: &[&str] = &[
    "compiler/runtime/src/prelude",
    "compiler/runtime/ffi/src/core_prelude",
];

const ALLOWED_PANIC_CONTEXTS: &[&str] = &[
    "panic!(\"Reml Option.expect が None を検出: {message}\");",
    "panic!(\"Reml Option.expect (release) が None を検出: {message}\");",
    "panic!(\"Reml Result.expect が Err({err}) を検出: {message}\");",
    "panic!(\"Reml Result.expect (release) が Err({err}) を検出: {message}\");",
];

#[test]
fn panic_invocations_are_restricted() {
    let repo_root = repo_root_path();
    let mut unexpected_panics = Vec::new();
    let mut unwrap_hits = Vec::new();

    for rel in TARGET_DIRECTORIES {
        let abs = repo_root.join(rel);
        let mut files = Vec::new();
        gather_rust_files(&abs, &mut files);
        for file in files {
            let content = fs::read_to_string(&file)
                .unwrap_or_else(|err| panic!("ファイル {file:?} の読み込みに失敗: {err}"));
            unexpected_panics.extend(find_unexpected_panics(&repo_root, &file, &content));
            if content.contains("unwrap_unchecked") {
                let key = relative(&repo_root, &file);
                unwrap_hits.push(key);
            }
        }
    }

    if !unexpected_panics.is_empty() {
        panic!(
            "panic! マクロの使用が許可された文脈を外れて検出されました:\n{}",
            unexpected_panics.join("\n")
        );
    }

    if !unwrap_hits.is_empty() {
        panic!(
            "unwrap_unchecked の使用箇所を検出しました: {}",
            unwrap_hits.join(", ")
        );
    }
}

fn find_unexpected_panics(repo_root: &Path, file: &Path, content: &str) -> Vec<String> {
    let mut hits = Vec::new();
    for (offset, _) in content.match_indices("panic!") {
        let line = current_line(content, offset);
        if ALLOWED_PANIC_CONTEXTS
            .iter()
            .any(|allowed| line.contains(allowed))
        {
            continue;
        }
        hits.push(format!("{}: {}", relative(repo_root, file), line.trim()));
    }
    hits
}

fn current_line(content: &str, offset: usize) -> &str {
    let start = content[..offset]
        .rfind('\n')
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let end = content[offset..]
        .find('\n')
        .map(|idx| idx + offset)
        .unwrap_or_else(|| content.len());
    &content[start..end]
}

fn gather_rust_files(dir: &Path, acc: &mut Vec<PathBuf>) {
    if !dir.exists() {
        return;
    }
    for entry in fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("ディレクトリ {dir:?} を走査できません: {err}"))
    {
        let entry = entry.expect("DirEntry の取得に失敗しました");
        let path = entry.path();
        if path.is_dir() {
            gather_rust_files(&path, acc);
        } else if path.extension().map(|ext| ext == "rs").unwrap_or(false) {
            acc.push(path);
        }
    }
}

fn repo_root_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("リポジトリルートを特定できませんでした")
        .to_path_buf()
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}
