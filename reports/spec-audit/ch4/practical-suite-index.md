# practical スイート実行レポート

- 実行時刻: 2025-12-24 20:16:07Z
- 対象シナリオ: 29 件 / 成功 23 件 / 失敗 6 件
- 入力ソース: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv`

| Scenario | File | 期待 Diagnostics | 実際 Diagnostics | Exit | 判定 | 備考 |
| --- | --- | --- | --- | --- | --- | --- |
| `CH3-IO-101` | `examples/practical/core_io/file_copy/canonical.reml` | — | — | 0 | ✅ pass | 実用 IO シナリオ。Capability Stage の検証と Golden が必要。 |
| `CH3-IO-201` | `examples/practical/core_io/file_copy/canonical.reml` | — | — | 0 | ✅ pass | `with_reader`/`with_writer`/`copy`/`sandbox_path` を組み合わせた Core.IO サンプル。`expected/practical/core_io/file_copy/canonical.audit.jsonl` で `metadata.io.helper = copy` と StageRequirement を証跡化。 |
| `CH3-PATH-202` | `examples/practical/core_path/security_check/relative_denied.reml` | `core.path.security.invalid` | `core.path.security.invalid` | 1 | ✅ pass | `SecurityPolicy` と `validate_path` の境界テスト。`expected/practical/core_path/security_check/relative_denied.diagnostic.json` で `security.reason=relative_path_denied` を確認可能にした。 |
| `CH3-PLG-310` | `examples/practical/core_config/audit_bridge/audit_bridge.reml` | — | — | 0 | ✅ pass | `@dsl_export` の `requires_capabilities`/`stage_bounds` を `reml.toml` と同期させる Core.Config ブリッジ。`expected/practical/core_config/audit_bridge/manifest_snapshot.json` で Manifest/DSL の Stage/Capability を比較可能にした。 |
| `CH3-TEXT-401` | `examples/practical/core_text/unicode/grapheme_nfc_mix.reml` | — | — | 0 | ✅ pass | `Core.Text.graphemes` と `normalize(:nfc)` の往復で Emoji + 合成文字列を損なわないことを確認する。 |
| `CH3-TEXT-402` | `examples/practical/core_text/unicode/grapheme_boundary_edge.reml` | — | — | 0 | ✅ pass | `Text.slice_graphemes` で結合文字を安全に切り出し、diagnostics=[] で完了することを確認する。 |
| `CH3-DIAG-501` | `examples/practical/core_diagnostics/audit_envelope/stage_tag_capture.reml` | — | — | 0 | ✅ pass | `AuditEnvelope.metadata` に `scenario.id`/`effect.stage.required` を埋め chapter3 §1.1 の契約を満たすことを確認する。 |
| `CH3-RUNTIME-601` | `examples/practical/core_runtime/capability/stage_mismatch_runtime_bridge.reml` | `runtime.bridge.stage_mismatch` | `runtime.bridge.stage_mismatch` | 1 | ✅ pass | RuntimeBridge の stage ポリシーと manifest の要求が食い違った場合に `runtime.bridge.stage_mismatch` を返すことを確認する。 |
| `CH3-ENV-701` | `examples/practical/core_env/envcfg/env_merge_by_profile.reml` | — | — | 0 | ✅ pass | `core.env.merge_profiles` が `@cfg` と同期して profile ごとの環境差分を出力することを確認する。 |
| `CH3-ASYNC-901` | `examples/practical/core_async/basic_sleep.reml` | — | — | 0 | ✅ pass | Core.Async の sleep_async/join/block_on を使った最小サンプル。 |
| `CH3-ASYNC-902` | `examples/practical/core_async/timeout_basic.reml` | — | — | 0 | ✅ pass | Core.Async の timeout/sleep_async/block_on を組み合わせた失敗経路の最小サンプル。 |
| `CH3-TEST-401` | `examples/practical/core_test/snapshot/basic_ok.reml` | — | — | 0 | ✅ pass | Core.Test スナップショットの最小例を固定する。 |
| `CH3-TEST-402` | `examples/practical/core_test/table/basic_ok.reml` | — | — | 0 | ✅ pass | Core.Test テーブル駆動の最小例を固定する。 |
| `CH3-TEST-403` | `examples/practical/core_test/fuzz/basic_ok.reml` | — | — | 0 | ✅ pass | Core.Test ファズの最小例を固定する。 |
| `CH3-TEST-410` | `examples/practical/core_test/dsl/ast_matcher_basic.reml` | — | — | 0 | ✅ pass | DSL Test Kit の AST Matcher 最小例を固定する。 |
| `CH3-TEST-411` | `examples/practical/core_test/dsl/error_expectation_basic.reml` | `parser.unexpected_eof` | — | 0 | ❌ fail | DSL Test Kit の Error Expectation を診断 JSON で固定する。 |
| `CH3-TEST-412` | `examples/practical/core_test/dsl/golden/basic.input` | — | `parser.top_level_expr.disallowed` | 1 | ❌ fail | DSL Test Kit のゴールデンファイル運用を固定する。 |
| `CH3-CLI-401` | `examples/practical/core_cli/parse_flags/basic_ok.reml` | — | — | 0 | ✅ pass | Core.Cli の宣言的フラグ解析を最小ケースで確認する。 |
| `CH3-CLI-402` | `examples/practical/core_cli/validate/basic_ok.reml` | — | — | 0 | ✅ pass | Core.Cli の validate サブコマンドを確認する。 |
| `CH3-CLI-403` | `examples/practical/core_cli/format/basic_ok.reml` | — | — | 0 | ✅ pass | Core.Cli の format サブコマンドを確認する。 |
| `CH3-PRETTY-401` | `examples/practical/core_text/pretty/layout_width_basic.reml` | — | — | 0 | ✅ pass | Core.Text.Pretty の幅差レイアウトを固定する。 |
| `CH3-DOC-401` | `examples/practical/core_doc/basic_generate_ok.reml` | — | — | 0 | ✅ pass | Core.Doc のドキュメント生成を最小例で固定する。 |
| `CH3-LSP-401` | `examples/practical/core_lsp/basic_diagnostics_ok.reml` | — | — | 0 | ✅ pass | Core.Lsp の診断送信最小例を固定する。 |
| `CH3-LSP-402` | `examples/practical/core_lsp/auto_derive_basic.reml` | — | — | 0 | ✅ pass | Core.Lsp.Derive の最小導出（空モデル）を固定する。 |
| `CH2-PARSE-930` | `examples/practical/core_parse/cst_lossless.reml` | — | — | 0 | ✅ pass | CST/Pretty のロスレス経路を最小例で固定する。 |
| `CH4-DSL-COMP-001` | `examples/practical/embedded_dsl/markdown_reml_basic.reml` | — | `parser.syntax.expected_tokens` | 1 | ❌ fail | Markdown 内の embedded_dsl を最小構成で合成し、dsl_id と境界スパンを監査に残す。 |
| `CH4-DSL-COMP-002` | `examples/practical/embedded_dsl/markdown_reml_error.reml` | `parser.unexpected_eof` | `parser.syntax.expected_tokens` | 1 | ❌ fail | 子 DSL の parse エラーが `source_dsl=""reml""` で報告されることを検証する。 |
| `CH4-DSL-COMP-003` | `examples/practical/embedded_dsl/markdown_reml_parallel.reml` | — | `parser.syntax.expected_tokens` | 1 | ❌ fail | ParallelSafe の埋め込み区間を `ExecutionPlan` に反映し `dsl.embedding.mode` を監査ログへ出力する。 |
| `CH5-LITE-001` | `examples/practical/lite_template/templates/sample.input` | — | `parser.top_level_expr.disallowed` | 1 | ❌ fail | Lite テンプレートの最短実行確認（`templates/sample.input` → `sample.ast.expected`）。 |
