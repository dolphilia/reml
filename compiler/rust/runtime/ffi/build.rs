use std::path::PathBuf;

fn main() {
    // runtime/native/build で生成される libreml_runtime.a をリンク
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest
        .ancestors()
        .nth(4)
        .expect("リポジトリルートのパスが見つかりません");
    let runtime_build = repo_root.join("runtime").join("native").join("build");

    println!("cargo:rustc-link-search=native={}", runtime_build.display());
    println!("cargo:rustc-link-lib=static=reml_runtime");
}
