# Examples Regression Log

Core.IO / Core.Path サンプルの自動実行結果と Runbook を記録する。Phase3 の `core_io.example_suite_pass_rate` 指標と連動し、失敗時の切り分け材料を残す。

## 2025-12-22 Core.IO & Path サンプル
- 追加ファイル: `examples/core_io/file_copy.reml`, `examples/core_path/security_check.reml`
- 実行コマンド: `tooling/examples/run_examples.sh --suite core_io`, `tooling/examples/run_examples.sh --suite core_path`
- 期待値: `cargo run --bin reml -- <example>` が 0 終了し、`IoContext.helper = "examples.core_io.file_copy"` / `metadata.security.reason` が診断へ出力される。
- 記録先: `reports/spec-audit/ch3/core_io_examples-20251222.md`, `core_io.example_suite_pass_rate = 1.0`
- 参照: `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md#6-ドキュメント・サンプル更新49-50週目`, `docs/notes/runtime-bridges-roadmap.md`
