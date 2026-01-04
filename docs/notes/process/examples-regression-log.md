# Examples Regression Log

Core.IO / Core.Path サンプルの自動実行結果と Runbook を記録する。Phase3 の `core_io.example_suite_pass_rate` 指標と連動し、失敗時の切り分け材料を残す。

## 2025-12-22 Core.IO & Path サンプル
- 追加ファイル: `examples/core_io/file_copy.reml`, `examples/core_path/security_check.reml`
- 実行コマンド: `tooling/examples/run_examples.sh --suite core_io`, `tooling/examples/run_examples.sh --suite core_path`
- 期待値: `cargo run --bin reml -- <example>` が 0 終了し、`IoContext.helper = "examples.core_io.file_copy"` / `metadata.security.reason` が診断へ出力される。
- 記録先: `reports/spec-audit/ch3/core_io_examples-20251222.md`, `core_io.example_suite_pass_rate = 1.0`
- 参照: `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md#6-ドキュメント・サンプル更新49-50週目`, `docs/notes/runtime/runtime-bridges-roadmap.md`

## Phase4 practical 反映（進行中）
- サンプル移設: `examples/practical/core_io/file_copy/canonical.reml`, `examples/practical/core_path/security_check/relative_denied.reml`（旧 `examples/core_io` / `examples/core_path`）
- ゴールデン: `expected/practical/core_io/file_copy/{canonical.stdout,canonical.audit.jsonl}`, `expected/practical/core_path/security_check/relative_denied.diagnostic.json`
- 実行コマンド: `tooling/examples/run_examples.sh --suite practical --scenario core_io|core_path`
- 備考: Phase4 シナリオマトリクス (`phase4-scenario-matrix.csv`) の ID `CH3-IO-101` / `CH3-PATH-202` とリンク

## 2025-12-11 Core.Path security_check（examples suite）
- 対象: `examples/core_path/security_check.reml`（EX-CORE-PATH-001 暫定）
- CLI: `cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/core_path/security_check.reml`
- 期待/実際: 期待=diagnostics `[]`、実際=diagnostics `[]` / run_id=`a55948c8-3a09-4a0e-8e3d-ab91f0f9eb51`
- ログ: `reports/spec-audit/ch5/logs/core_path-20251211T092454Z.md`
- 対応: Example Fix としてトップレベルを `struct` から `type ... = new { ... }` へ移行し、`is_safe_symlink` のエラー経路を `map_err(...)?` で統一。フェーズF チェックリストを `[x]` 化。

## 2025-12-11 Core.Config CLI DSL フェーズF
- 対象: `examples/core_config/cli/dsl/sample.reml`（EX-CORE-CONFIG-CLI-001 暫定）
- CLI: `cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/core_config/cli/dsl/sample.reml`
- 期待/実際: 期待=diagnostics `[]`、実際=diagnostics `[]` / run_id=`d8ffcb77-98f3-4b89-b10a-7c4fad72727d`
- ログ: `reports/spec-audit/ch5/logs/core_config-20251211T093350Z.md`
- 対応: Effect 宣言に `operation` を追加し、`ensure` の遅延診断クロージャを `| | diagnostic(...)` 形式へ修正して BNF (`OperationDecl+`) と整合。Parser の `parser.syntax.expected_tokens` を解消し、フェーズF チェックリストを `[x]` 化。

## 2025-12-11 Core.Config Telemetry DSL フェーズF
- 対象: `examples/core_config/dsl/telemetry_bridge.reml`（EX-CORE-CONFIG-DSL-002 暫定）
- CLI: `cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/core_config/dsl/telemetry_bridge.reml`
- 期待/実際: 期待=diagnostics `[]`、実際=diagnostics `[]` / run_id=`7f1bb8a3-f84a-43dc-99a7-97ca218ecf90`
- ログ: `reports/spec-audit/ch5/logs/core_config-20251211T093648Z.md`
- 備考: docs/guides 向け Telemetry DSL プレースホルダが Parser/Typeck を通過することを確認し、フェーズF チェックリストを `[x]` 化。

## 2025-12-11 Core.IO canonical フェーズF 回帰修正
- 対象: `examples/practical/core_io/file_copy/canonical.reml`（`CH3-IO-101` / `CH3-IO-201`）
- CLI: `cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_io/file_copy/canonical.reml`
- 期待/実際: 期待=成功、実際=diagnostics 0（run_id=`a1d9dcac-0505-4981-b5c8-5fe996ff28dd`）。`canonical.stdout` / `canonical.audit.jsonl` ゴールデンと整合。
- 対応: Parser を修正し (1) ブロック内ステートメントのセミコロンを任意化、(2) レコードリテラルで `:` / `=` / フィールド省略（punning）を受理できるように変更。これにより `CopyReport({...})` の省略フィールド `bytes` が受理され practical 実行が再開。
- リンク: `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` フェーズF チェックリストを `[x]` 化。`phase4-scenario-matrix.csv` の `CH3-IO-101/201` は `resolution=ok` 維持。

## 2025-12-11 Core.Runtime stage_mismatch_runtime_bridge フェーズF 再実行
- 対象: `examples/practical/core_runtime/capability/stage_mismatch_runtime_bridge.reml`（`CH3-RUNTIME-601`）
- CLI: `cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_runtime/capability/stage_mismatch_runtime_bridge.reml`
- 期待/実際: 期待=`runtime.bridge.stage_mismatch`、実際=diagnostics 1（同コード、run_id=`d91aebaa-d239-4443-adcd-01249a5aa85a`）
- ログ: `reports/spec-audit/ch5/logs/practical-20251211T014101Z.md`
- 備考: runtime フェーズ有効化後の診断生成が想定どおりであることを再確認し、PhaseF チェックリストを更新。

## 2025-12-11 Core.Text grapheme_boundary_edge フェーズF 是正
- 対象: `examples/practical/core_text/unicode/grapheme_boundary_edge.reml`（`CH3-TEXT-402`）
- CLI: `cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_text/unicode/grapheme_boundary_edge.reml`
- 期待/実際: 期待=diagnostics `[]`（segment_mismatch 期待を削除）、実際=diagnostics `[]` / run_id=`c39eec0c-e343-42da-913a-2cec905343bb`
- 対応: grapheme 境界安全な `Text.slice_graphemes` を使っており、診断なしが正しいため Example Fix として expected を空診断に更新。`phase4-scenario-matrix.csv` の `CH3-TEXT-402` を `ok` / `example_fix` へ変更し、PhaseF チェックリストも `[x]` 化。
- ログ: `reports/spec-audit/ch5/logs/practical-20251211T082727Z.md`

## 2025-12-11 Core.Text grapheme_nfc_mix フェーズF 実行
- 対象: `examples/practical/core_text/unicode/grapheme_nfc_mix.reml`（`CH3-TEXT-401`）
- CLI: `cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_text/unicode/grapheme_nfc_mix.reml`
- 期待/実際: 期待=diagnostics `[]`、実際=diagnostics `[]` / run_id=`250a3b7c-b790-422f-9b30-e654d2343265`
- stdout: ゴールデン `expected/practical/core_text/unicode/grapheme_nfc_mix.stdout`（`graphemes=2`、runtime_phase=none / artifact=null）
- ログ: `reports/spec-audit/ch5/logs/practical-20251211T083527Z.md`
- 備考: PhaseF チェックリストと `phase4-scenario-matrix.csv` を `resolution=ok`・`spec_vs_impl_decision=ok` へ更新。runtime フェーズ対象外だが parse/typeck は成功。

## 2025-12-11 Core.Env env_merge_by_profile フェーズF 実行
- 対象: `examples/practical/core_env/envcfg/env_merge_by_profile.reml`（`CH3-ENV-701`）
- CLI: `cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_env/envcfg/env_merge_by_profile.reml`
- 期待/実際: 期待=diagnostics `[]` / stdout=`expected/practical/core_env/envcfg/env_merge_by_profile.stdout`（`https://cli.local`）、実際=diagnostics `[]`（run_id=`2f9ecb5d-3d75-4ba4-92f2-7233b6b00b5b`）
- ログ: `reports/spec-audit/ch5/logs/practical-20251211T091650Z.md`
- 備考: env プロファイル merge の runtime フェーズが診断なしで完了することを確認し、PhaseF チェックリストと `phase4-scenario-matrix.csv` を `resolution=ok` / `spec_vs_impl_decision=ok` へ更新。

## 2026-02-20 Core.Path relative_denied 回帰 → runtime 実行フェーズで解消
- 対象: `examples/practical/core_path/security_check/relative_denied.reml`（`CH3-PATH-202`）
- CLI: `cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_path/security_check/relative_denied.reml`
- 期待/実際: 期待=`core.path.security.invalid`（`security.reason=relative_path_denied`）、実際=diagnostics 1（同コード） / run_id=`55290137-a91c-4340-9627-dd59ab196690`
- 対応: Rust Frontend に runtime フェーズを追加し、パース/型検査完了後に `runtime_path::validate_path`→`sandbox_path`→`is_safe_symlink` を実行して診断を生成
- リンク: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` の `CH3-PATH-202` を `ok` へ更新

## 2026-02-20 Core.Runtime stage_mismatch_runtime_bridge を runtime フェーズで実行
- 対象: `examples/practical/core_runtime/capability/stage_mismatch_runtime_bridge.reml`（`CH3-RUNTIME-601`）
- CLI: `cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_runtime/capability/stage_mismatch_runtime_bridge.reml`
- 期待/実際: 期待=`runtime.bridge.stage_mismatch`、実際=diagnostics 1（同コード） / run_id=`aa50ccd2-05f2-4755-bb8d-73527871b68e`
- 対応: runtime フェーズに Bridge stage mismatch 用プランを追加し、typecheck 後に診断を生成
- リンク: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` の `CH3-RUNTIME-601` を `ok` へ更新

## 2025-12-10 CH3-PATH-202 runtime フェーズF 再実行
- CLI: `cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_path/security_check/relative_denied.reml`
- 期待/実際: 期待=`core.path.security.invalid`（relative_path_denied）、実際=diagnostics 1（同コード、run_id=`59e7be86-650c-406e-b865-a9a0a625c767`）
- ログ: `reports/spec-audit/ch5/logs/practical-20251210T205757Z.md`
- 備考: runtime フェーズの `validate_path`→`sandbox_path`→`is_safe_symlink` 経路で `security.reason=relative_path_denied` を確認し、PhaseF practical チェックリストを更新。

## 2025-12-10 OpBuilder DSL フェーズF
- 対象: `examples/spec_core/chapter2/op_builder/core-opbuilder-level-conflict-error.reml`
- CLI: `cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter2/op_builder/core-opbuilder-level-conflict-error.reml`
- 期待/実際 Diagnostics: `core.parse.opbuilder.level_conflict`（Exit code 1, 診断想定）
- ゴールデン: `expected/spec_core/chapter2/op_builder/core-opbuilder-level-conflict-error.diagnostic.json`
- ログ: `reports/spec-audit/ch5/logs/spec_core-20251210T125446Z.md`
- 備考: DSL fixity シンボル（`:infix_left` / `:infix_right`）を再受理し、`phase4-scenario-matrix.csv` の `CH2-OP-401` を `resolution=ok` へ更新。
