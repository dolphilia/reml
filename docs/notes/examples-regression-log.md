# Examples Regression Log

Core.IO / Core.Path サンプルの自動実行結果と Runbook を記録する。Phase3 の `core_io.example_suite_pass_rate` 指標と連動し、失敗時の切り分け材料を残す。

## 2025-12-22 Core.IO & Path サンプル
- 追加ファイル: `examples/core_io/file_copy.reml`, `examples/core_path/security_check.reml`
- 実行コマンド: `tooling/examples/run_examples.sh --suite core_io`, `tooling/examples/run_examples.sh --suite core_path`
- 期待値: `cargo run --bin reml -- <example>` が 0 終了し、`IoContext.helper = "examples.core_io.file_copy"` / `metadata.security.reason` が診断へ出力される。
- 記録先: `reports/spec-audit/ch3/core_io_examples-20251222.md`, `core_io.example_suite_pass_rate = 1.0`
- 参照: `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md#6-ドキュメント・サンプル更新49-50週目`, `docs/notes/runtime-bridges-roadmap.md`

## Phase4 practical 反映（進行中）
- サンプル移設: `examples/practical/core_io/file_copy/canonical.reml`, `examples/practical/core_path/security_check/relative_denied.reml`（旧 `examples/core_io` / `examples/core_path`）
- ゴールデン: `expected/practical/core_io/file_copy/{canonical.stdout,canonical.audit.jsonl}`, `expected/practical/core_path/security_check/relative_denied.diagnostic.json`
- 実行コマンド: `tooling/examples/run_examples.sh --suite practical --scenario core_io|core_path`
- 備考: Phase4 シナリオマトリクス (`phase4-scenario-matrix.csv`) の ID `CH3-IO-101` / `CH3-PATH-202` とリンク

## 2026-02-20 Core.Path relative_denied 回帰 → runtime 実行フェーズで解消
- 対象: `examples/practical/core_path/security_check/relative_denied.reml`（`CH3-PATH-202`）
- CLI: `cargo run --quiet --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_path/security_check/relative_denied.reml`
- 期待/実際: 期待=`core.path.security.invalid`（`security.reason=relative_path_denied`）、実際=diagnostics 1（同コード） / run_id=`55290137-a91c-4340-9627-dd59ab196690`
- 対応: Rust Frontend に runtime フェーズを追加し、パース/型検査完了後に `runtime_path::validate_path`→`sandbox_path`→`is_safe_symlink` を実行して診断を生成
- リンク: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` の `CH3-PATH-202` を `ok` へ更新

## 2026-02-20 Core.Runtime stage_mismatch_runtime_bridge を runtime フェーズで実行
- 対象: `examples/practical/core_runtime/capability/stage_mismatch_runtime_bridge.reml`（`CH3-RUNTIME-601`）
- CLI: `cargo run --quiet --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_runtime/capability/stage_mismatch_runtime_bridge.reml`
- 期待/実際: 期待=`runtime.bridge.stage_mismatch`、実際=diagnostics 1（同コード） / run_id=`aa50ccd2-05f2-4755-bb8d-73527871b68e`
- 対応: runtime フェーズに Bridge stage mismatch 用プランを追加し、typecheck 後に診断を生成
- リンク: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` の `CH3-RUNTIME-601` を `ok` へ更新

## 2025-12-10 OpBuilder DSL フェーズF
- 対象: `examples/spec_core/chapter2/op_builder/core-opbuilder-level-conflict-error.reml`
- CLI: `cargo run --quiet --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter2/op_builder/core-opbuilder-level-conflict-error.reml`
- 期待/実際 Diagnostics: `core.parse.opbuilder.level_conflict`（Exit code 1, 診断想定）
- ゴールデン: `expected/spec_core/chapter2/op_builder/core-opbuilder-level-conflict-error.diagnostic.json`
- ログ: `reports/spec-audit/ch4/logs/spec_core-20251210T125446Z.md`
- 備考: DSL fixity シンボル（`:infix_left` / `:infix_right`）を再受理し、`phase4-scenario-matrix.csv` の `CH2-OP-401` を `resolution=ok` へ更新。
