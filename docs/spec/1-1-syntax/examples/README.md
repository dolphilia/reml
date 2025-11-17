# 1.1 構文サンプルセット

Chapter 1 の仕様で参照している Reml コード例を `.reml` ファイルとして切り出し、`poc_frontend` で監査できるようにした。Phase 2-8 では次の方針でメンテナンスする。

- `use_nested.reml` / `effect_handler.reml` などは仕様本文と 1:1 で対応する**正準サンプル**。脚注から直接リンクし、`reports/spec-audit/ch1/` のログと突き合わせて状態を記録する。
- `*_rustcap.reml` は Rust Frontend の現状制限（`module` / 先頭 `use` / `fn` の戻り値シグネチャなど）を迂回するフォールバック。`rust-gap` ラベルを伴う診断ログとセットで管理し、制限が解消され次第削除する。
- 検証は `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics <sample>` を基本とし、必要に応じて `--emit-ast` / `--emit-typeck-debug` を追加する。コマンドと結果は `reports/spec-audit/summary.md` に追記する。
- 新しいコード片を仕様に追加する際は、ここへも `.reml` を追加し、`docs/spec/0-3-code-style-guide.md` §8 のチェックリストを更新する。

`rust-gap` の解消順序は `docs/notes/spec-integrity-audit-checklist.md#rust-gap-トラッキング表` に従う。
