# Core Path サンプル

Path 抽象とセキュリティヘルパ (`validate_path`, `sandbox_path`, `is_safe_symlink`) を組み合わせた運用例をまとめます。Phase 3-5 実装計画の §4〜§6 で規定された安全要件を確認する用途を想定しています。

## `security_check.reml`
- 任意の入力パスを `SecurityPolicy` で検証し、サンドボックスルート配下へ正規化する例です。
- `is_safe_symlink` の結果を診断やロギングへ転写するフックの位置をコメントで示しています。
- `Core.Path` と `Core.IO` の Capability チェック方法を `docs/spec/3-5-core-io-path.md` §4.2 と同期しました。

実行例:
```sh
cargo run --bin reml -- examples/core_path/security_check.reml
```

このサンプルの進捗は `docs/notes/stdlib/core-io-path-gap-log.md` と `docs/notes/runtime/runtime-bridges-roadmap.md` にも記録されます。
