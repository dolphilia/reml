# practical スイート実行レポート

- 実行時刻: 2025-12-07 10:47:07Z
- 対象シナリオ: 9 件 / 成功 0 件 / 失敗 9 件
- 入力ソース: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv`

| Scenario | File | 期待 Diagnostics | 実際 Diagnostics | Exit | 判定 | 備考 |
| --- | --- | --- | --- | --- | --- | --- |
| `CH3-IO-101` | `examples/practical/core_io/file_copy/canonical.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | 実用 IO シナリオ。Capability Stage の検証と Golden が必要。 |
| `CH3-IO-201` | `examples/practical/core_io/file_copy/canonical.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `with_reader`/`with_writer`/`copy`/`sandbox_path` を組み合わせた Core.IO サンプル。`expected/practical/core_io/file_copy/canonical.audit.jsonl` で `metadata.io.helper = copy` と StageRequirement を証跡化。 |
| `CH3-PATH-202` | `examples/practical/core_path/security_check/relative_denied.reml` | `core.path.security.invalid` | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `SecurityPolicy` と `validate_path` の境界テスト。`expected/practical/core_path/security_check/relative_denied.diagnostic.json` で `security.reason=relative_path_denied` を確認可能にした。 |
| `CH3-PLG-310` | `examples/practical/core_config/audit_bridge/audit_bridge.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `@dsl_export` の `requires_capabilities`/`stage_bounds` を `reml.toml` と同期させる Core.Config ブリッジ。`expected/practical/core_config/audit_bridge/manifest_snapshot.json` で Manifest/DSL の Stage/Capability を比較可能にした。 |
| `CH3-TEXT-401` | `examples/practical/core_text/unicode/grapheme_nfc_mix.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `Core.Text.graphemes` と `normalize(:nfc)` の往復で Emoji + 合成文字列を損なわないことを確認する。 |
| `CH3-TEXT-402` | `examples/practical/core_text/unicode/grapheme_boundary_edge.reml` | `core.text.unicode.segment_mismatch` | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | 結合文字を途中で切断した場合に `core.text.unicode.segment_mismatch` が発生することを診断 JSON で確認する。 |
| `CH3-DIAG-501` | `examples/practical/core_diagnostics/audit_envelope/stage_tag_capture.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `AuditEnvelope.metadata` に `scenario.id`/`effect.stage.required` を埋め chapter3 §1.1 の契約を満たすことを確認する。 |
| `CH3-RUNTIME-601` | `examples/practical/core_runtime/capability/stage_mismatch_runtime_bridge.reml` | `runtime.bridge.stage_mismatch` | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | RuntimeBridge の stage ポリシーと manifest の要求が食い違った場合に `runtime.bridge.stage_mismatch` を返すことを確認する。 |
| `CH3-ENV-701` | `examples/practical/core_env/envcfg/env_merge_by_profile.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `core.env.merge_profiles` が `@cfg` と同期して profile ごとの環境差分を出力することを確認する。 |

## フォローアップ (2025-12-11)

- `CH3-TEXT-402`: CLI=`cargo run --quiet --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_text/unicode/grapheme_boundary_edge.reml` / diagnostics=`[]` / run_id=`c39eec0c-e343-42da-913a-2cec905343bb`。`Text.slice_graphemes` で境界安全にスライスしているため `core.text.unicode.segment_mismatch` は発生せず、PhaseF チェックリストと `phase4-scenario-matrix.csv` を `ok`（example_fix）へ更新済み。ログ: `reports/spec-audit/ch4/logs/practical-20251211T082727Z.md`。
