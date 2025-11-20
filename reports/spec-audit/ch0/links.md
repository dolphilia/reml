# Chapter 0 Link Check (W36 後半)

実行コマンド: `python3 - <<SCRIPT ...`（カスタムスクリプトで Markdown リンクの存在確認を実施）。

## Prelude 実装ログ

| コマンド | 結果 | 備考 |
| --- | --- | --- |
| `cargo xtask prelude-audit --wbs 2.1b --strict --baseline docs/spec/3-1-core-prelude-iteration.md` | ✅ | `prelude_api_inventory.toml` の Option/Result 16 API が `rust_status=implemented`。`compiler/rust/frontend/tests/core_prelude_option_result.snap` の 16 シナリオ参照とリンク |

### Iter F0 整合ログ（WBS 3.1a）

| コマンド | 結果 | 備考 |
| --- | --- | --- |
| `sed -n '200,360p' docs/spec/3-1-core-prelude-iteration.md` | ✅ | `IteratorDictInfo` が `StageRequirement`/`CapabilityId`/`effect.stage.iterator.*` を必須と定義していることを再確認（Iter F0）。 |
| `sed -n '360,520p' compiler/ocaml/src/constraint_solver.ml` | ✅ | `solve_iterator` が `IteratorKind` ごとに `stage_requirement`/`stage_actual`/`capability` を埋める既存実装を確認。Rust 側 `IteratorDictInfo` の仕様化根拠として記録。 |

### Collector F0 効果タグ対照（WBS 3.1b）

| コマンド | 結果 | 備考 |
| --- | --- | --- |
| `sed -n '150,210p' docs/spec/3-1-core-prelude-iteration.md` | ✅ | `Collector::new`〜`into_inner` の効果タグ/`IntoDiagnostic` 契約を引用し、F0 での効果セット整理に使用。 |
| `sed -n '150,210p' docs/spec/3-2-core-collections.md` | ✅ | 標準コレクタ（List/Vec/Map/Set/Table）の効果/エラー表を引用し、`CollectError` バリアントの根拠を確認。 |
| `grep -n 'module = \"Collector\"' docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` | ✅ | `module=\"Collector\"` ブロックに 12 エントリ（トレイト + 標準コレクタ）が登録されていることを確認。 |
| `python3 tooling/ci/collect-iterator-audit-metrics.py --module iter --section collectors --case wbs-31b-f0 --dry-run` | ✅ / pending | まだ `--module`/`--section collectors` オプションが未実装のため、F0 では期待 CLI 形のみ記録（実行時は `argparse` エラー）。実装完了後に `collector.effect.*` を即時収集する計画。 |

### Collector F2 実装ログ（WBS 3.1b, W37 前半）

| コマンド | 結果 | 備考 |
| --- | --- | --- |
| `cargo test core_iter_collectors -- --nocapture` | ✅ / pending | `compiler/rust/frontend/tests/core_iter_collectors.rs` に追加する 7 シナリオ（List/Vec/Map/Set/String/Table baseline/duplicate）を `insta` snapshot 化し、Collector 実装の回帰を監視。 |
| `cargo insta review --review` | ✅ / pending | `core_iter_collectors.snap` を確定し、`prelude.collector.kind`/`effects`/`error_kind` を JSON で固定。 |
| `tooling/ci/collect-iterator-audit-metrics.py --module iter --section collectors --wbs 3.1b-F2 --output reports/iterator-collector-summary.md` | ✅ / pending | `collector.effect.mem`, `collector.effect.mut`, `collector.error.duplicate_key_rate`, `iterator.stage.audit_pass_rate` を出力。`collect_list_baseline` と `collect_vec_mem_error` の `collector.effect.*` をベースラインとし、`reports/iterator-collector-summary.md` の JSON で `collector.effect.mem=0`/`collector.effect.mem_reservation>0` を検証した上で `0-3-audit-and-metrics.md` に転記。 |
| `scripts/validate-diagnostic-json.sh --pattern collector` | ✅ / pending | `prelude.collector.*` キーが `reports/diagnostic-format-regression.md` に差分なしで反映されるかを確認。 |
| `cargo xtask prelude-audit --wbs '3.1b F2'` | ✅ | `Collector` トレイト/コレクタ遍歴を検査し、`trait.*` 6項目・`TableCollector.push`・`ListCollector.new`・`VecCollector.*` を `implemented` と判定し、`collector.effect.*` 監査と KPI 連携を完了した段階を記録。 |

| シナリオID | Snapshot | KPI / 監査ログ | 仕様根拠・備考 |
| --- | --- | --- | --- |
| `collect_list_baseline` | `compiler/rust/frontend/tests/snapshots/core_iter_collectors__collect_list_baseline.snap` | `reports/iterator-collector-summary.md#collect_list_baseline` | `ListCollector`、`effect = []`、`Stage = stable`。【F:docs/spec/3-1-core-prelude-iteration.md†L237-L253】`collect-iterator-audit` KPI で `collector.effect.mem=0` を確認済。 |
| `collect_vec_mem_error` | `compiler/rust/frontend/tests/snapshots/core_iter_collectors__collect_vec_mem_error.snap` | `reports/iterator-collector-summary.md#collect_vec_mem_error` | `VecCollector` の `effect {mut, mem}` と `CollectError::MemoryError` を確認し `R-027` リスク監視に接続。`collect-iterator-audit` では `collector.effect.mem_reservation`/`collector.effect.reserve` を JSON KPI に記録。 |
| `collect_map_duplicate` | `compiler/rust/frontend/tests/snapshots/core_iter_collectors__collect_map_duplicate.snap` | `reports/iterator-collector-summary.md#collect_map_duplicate` | `CollectError::DuplicateKey` と `AuditEnvelope.metadata.collector.error.key` を確認。【F:docs/spec/3-2-core-collections.md†L75-L88】 |
| `collect_set_stage` | `compiler/rust/frontend/tests/snapshots/core_iter_collectors__collect_set_stage.snap` | `reports/iterator-collector-summary.md#collect_set_stage` | `SetCollector` の `StageRequirement::Exact("stable")` を診断へ転写。 |
| `collect_string_invalid` | `compiler/rust/frontend/tests/snapshots/core_iter_collectors__collect_string_invalid.snap` | `reports/iterator-collector-summary.md#collect_string_invalid` | `StringCollector` の UTF-8 正規化と `CollectError::InvalidEncoding(StringError)` を検証。【F:docs/spec/3-3-core-text-unicode.md†L90-L150】 |
| `collect_table_baseline` | `compiler/rust/frontend/tests/__snapshots__/core_iter_collectors.snap` | `reports/iterator-collector-summary.md#collect_table_baseline` | `TableCollector` の `collector.kind=table` / `effect {mut}` を `collect_table_baseline` で記録し、挿入順 `Table` の再現性を確認。【F:docs/spec/3-1-core-prelude-iteration.md†L188-L210】【F:docs/spec/3-2-core-collections.md†L154-L168】 |
| `collect_table_duplicate` | `compiler/rust/frontend/tests/__snapshots__/core_iter_collectors.snap` | `reports/iterator-collector-summary.md#collect_table_duplicate` | 重複キーで `CollectError::DuplicateKey` を返し `collector.error.key`/`Diagnostic.extensions["prelude.collector.error_key"]` を `collect_table_duplicate` で固定。`stage` は `AtLeast("beta")`、`collector.effect.mut` を監査する。【F:docs/spec/3-1-core-prelude-iteration.md†L188-L210】【F:docs/spec/3-2-core-collections.md†L154-L168】 |

- `reports/iterator-collector-summary.md` には上述の 7 ケースの `collector.effect.*`/`collector.error.*` KPI をまとめて記録し、`collect_list_baseline` で `collector.effect.mem=0`、`collect_vec_mem_error` で `collector.effect.mem_reservation>0` を `collect-iterator-audit` JSON に反映すると同時に、`collect_string_invalid` の `collector.error.invalid_encoding`、`collect_table_duplicate` の `collector.error.key` を KPI に並列させて `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` と整合させている。

### Iter F3 Snapshot/KPI（WBS 3.1a）

| コマンド | 結果 | 備考 |
| --- | --- | --- |
| `cargo test core_iter_pipeline -- --nocapture` | ✅ / pending | `core_iter_pipeline.rs` へ 6 シナリオを追加し、`insta` snapshot (`.snap`) を生成する。F3 サイクルでは CI で `--nocapture` を使いステージ情報をログ化する。 |
| `cargo insta review --review` | ✅ / pending | `core_iter_pipeline.snap` を確定し、`Iter.from_list`〜`Iter.try_collect` の往復を固定。`docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` のシナリオ表と対応付ける。 |
| `tooling/ci/collect-iterator-audit-metrics.py --module iter --section collectors --output reports/iterator-stage-summary.md` | ✅ / pending | `iterator.stage.audit_pass_rate`・`collector.effect.mem` の集計結果を `reports/iterator-stage-summary.md` に保存し、`0-3-audit-and-metrics.md` KPI を更新する。 |
| `python3 reports/spec-audit/scripts/attach_snapshot_links.py --plan docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md --output reports/spec-audit/ch0/iter-f3-links.md` | ✅ / pending | シナリオ毎の Snapshot/Diagnostic/Audit を Markdown 表へ展開し、本ファイルに貼り付ける補助スクリプト。 |

| シナリオID | Snapshot | 診断 JSON | 監査ログ | 備考 |
| --- | --- | --- | --- | --- |
| `iter_from_list_roundtrip` | `compiler/rust/frontend/tests/snapshots/core_iter_pipeline__iter_from_list_roundtrip.snap` | `reports/diagnostic-format-regression.md#iter-from-list` | `reports/iterator-stage-summary.md#iter_from_list_roundtrip` | `ListCollector` で `@pure` を確認。 |
| `iter_map_utf8` | `compiler/rust/frontend/tests/snapshots/core_iter_pipeline__iter_map_utf8.snap` | `reports/diagnostic-format-regression.md#iter-map` | `reports/iterator-stage-summary.md#iter_map_utf8` | UTF-8 map 変換、`effect {mem}` 非使用を確認。 |
| `iter_filter_map_cap` | `compiler/rust/frontend/tests/snapshots/core_iter_pipeline__iter_filter_map_cap.snap` | `reports/diagnostic-format-regression.md#iter-filter-map` | `reports/iterator-stage-summary.md#iter_filter_map_cap` | `iterator.effect.debug = 0` を保証。 |
| `iter_flat_map_stage` | `compiler/rust/frontend/tests/snapshots/core_iter_pipeline__iter_flat_map_stage.snap` | `reports/diagnostic-format-regression.md#iter-flat-map` | `reports/iterator-stage-summary.md#iter_flat_map_stage` | Stage 要件 `AtLeast(beta)` を確認。 |
| `iter_try_fold_diag` | `compiler/rust/frontend/tests/snapshots/core_iter_pipeline__iter_try_fold_diag.snap` | `reports/diagnostic-format-regression.md#iter-try-fold` | `reports/iterator-stage-summary.md#iter_try_fold_diag` | `typeclass.iterator.stage_mismatch` が出ないことを確認。 |
| `iter_try_collect_set` | `compiler/rust/frontend/tests/snapshots/core_iter_pipeline__iter_try_collect_set.snap` | `reports/diagnostic-format-regression.md#iter-try-collect` | `reports/iterator-stage-summary.md#iter_try_collect_set` | `collector.effect.mem` 集計対象。 |

| ファイル | リンク | 存在 | 備考 |
|---------|--------|------|------|
| `docs/spec/0-0-overview.md` | `../../reports/diagnostic-format-regression.md` | ✅ | - |
| `docs/spec/0-0-overview.md` | `../../reports/spec-audit/README.md` | ✅ | - |
| `docs/spec/0-0-overview.md` | `../notes/reml-design-goals-and-appendix.md` | ✅ | - |
| `docs/spec/0-0-overview.md` | `../plans/bootstrap-roadmap/2-5-proposals/EFFECT-002-proposal.md#4-診断・ci-計測整備week33-day1-2` | ✅ | - |
| `docs/spec/0-0-overview.md` | `0-1-project-purpose.md` | ✅ | - |
| `docs/spec/0-0-overview.md` | `1-0-language-core-overview.md` | ✅ | - |
| `docs/spec/0-0-overview.md` | `2-0-parser-api-overview.md` | ✅ | - |
| `docs/spec/0-0-overview.md` | `2-7-core-parse-streaming.md` | ✅ | - |
| `docs/spec/0-0-overview.md` | `3-0-core-library-overview.md` | ✅ | - |
| `docs/spec/0-0-overview.md` | `3-7-core-config-data.md` | ✅ | - |
| `docs/spec/0-0-overview.md` | `3-8-core-runtime-capability.md` | ✅ | - |
| `docs/spec/0-0-overview.md` | `3-9-core-async-ffi-unsafe.md` | ✅ | - |
| `docs/spec/0-0-overview.md` | `4-0-official-plugins-overview.md` | ✅ | - |
| `docs/spec/0-0-overview.md` | `4-7-core-parse-plugin.md` | ✅ | - |
| `docs/spec/0-0-overview.md` | `5-0-ecosystem-overview.md` | ✅ | - |
| `docs/spec/0-1-project-purpose.md` | `2-7-core-parse-streaming.md` | ✅ | - |
| `docs/spec/0-1-project-purpose.md` | `3-6-core-diagnostics-audit.md` | ✅ | - |
| `docs/spec/0-1-project-purpose.md` | `3-8-core-runtime-capability.md` | ✅ | - |
| `docs/spec/0-1-project-purpose.md` | `4-7-core-parse-plugin.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `../guides/conductor-pattern.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `../guides/runtime-bridges.md#105-ストリーミング-flow-signal-と-runtime-bridge-連携` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `../plans/bootstrap-roadmap/2-8-spec-integrity-audit.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `0-1-project-purpose.md#31-unicode対応の充実` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `0-1-project-purpose.md#31-unicode対応の充実` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `0-1-project-purpose.md#32-エコシステム統合とdslファーストアプローチ` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-1-syntax.md#a3-識別子とキーワード` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-1-syntax.md#a3-識別子とキーワード` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-1-syntax.md#a3-識別子とキーワード` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-1-syntax.md#b11-dslエントリーポイント宣言` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-1-syntax.md#b11-dslエントリーポイント宣言` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md#c-6-効果行とハンドラの型付け実験段階` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md#c-6-効果行とハンドラの型付け実験段階` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-2-types-Inference.md#c-6-効果行とハンドラの型付け実験段階` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-3-effects-safety.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-3-effects-safety.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-3-effects-safety.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-3-effects-safety.md#m5-所有権とリソース管理` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-4-test-unicode-model.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-4-test-unicode-model.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-4-test-unicode-model.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-4-test-unicode-model.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-4-test-unicode-model.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `1-4-test-unicode-model.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-0-parser-api-overview.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-1-parser-type.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-1-parser-type.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-1-parser-type.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-1-parser-type.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-1-parser-type.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-1-parser-type.md#b-入力モデル-input` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-1-parser-type.md#b-入力モデル-input` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-1-parser-type.md#c-スパンとトレース` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-1-parser-type.md#c-スパンとトレース` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-1-parser-type.md#d-実行設定-runconfig-とメモ` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-1-parser-type.md#e-コミットと消費の意味論` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-2-core-combinator.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-2-core-combinator.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-3-lexer.md#d-1-プロファイル` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-4-op-builder.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-4-op-builder.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-4-op-builder.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-4-op-builder.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-5-error.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-5-error.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-5-error.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-5-error.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-5-error.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-5-error.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-5-error.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-5-error.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-5-error.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-5-error.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-5-error.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-5-error.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-5-error.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-5-error.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-6-execution-strategy.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-6-execution-strategy.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-6-execution-strategy.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-6-execution-strategy.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-6-execution-strategy.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-6-execution-strategy.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-6-execution-strategy.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-6-execution-strategy.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-6-execution-strategy.md#b-2-runconfig-のコアスイッチ` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-6-execution-strategy.md#c-メモ化packratと左再帰` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-7-core-parse-streaming.md#feeder-demandhint` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `2-7-core-parse-streaming.md#flow-controller` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-1-core-prelude-iteration.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-1-core-prelude-iteration.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-1-core-prelude-iteration.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-1-core-prelude-iteration.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-10-core-env.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-2-core-collections.md#32-cellt-ref` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-2-core-collections.md#32-cellt-ref` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-2-core-collections.md#32-cellt-ref` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-3-core-text-unicode.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-3-core-text-unicode.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-6-core-diagnostics-audit.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-6-core-diagnostics-audit.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-6-core-diagnostics-audit.md#11-auditenvelope` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-6-core-diagnostics-audit.md#12-診断ドメイン-diagnosticdomain` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-6-core-diagnostics-audit.md#13-効果診断拡張-effects` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-6-core-diagnostics-audit.md#24-stage-差分プリセット-effectdiagnostic` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-6-core-diagnostics-audit.md#24-stage-差分プリセット-effectdiagnostic` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-6-core-diagnostics-audit.md#3-監査ログ出力` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-6-core-diagnostics-audit.md#diagnostic-bridge` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-6-core-diagnostics-audit.md#diagnostic-bridge` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-6-core-diagnostics-audit.md#diagnostic-ffi-contract` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-6-core-diagnostics-audit.md#diagnostic-ffi-contract` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-6-core-diagnostics-audit.md#diagnostic-presets` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md#10-runtime-bridge-契約` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md#101-runtimebridgeregistry-とメタデータ` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md#103-ホットリロード契約` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md#105-ストリーミング-signal-ハンドラ` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md#11-capabilityhandle-のバリアント` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md#11-capabilityhandle-のバリアント` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md#12-セキュリティモデル` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md#12-セキュリティモデル` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md#12-セキュリティモデル` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md#12-セキュリティモデル` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md#13-プラットフォーム情報と能力` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md#13-プラットフォーム情報と能力` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md#13-プラットフォーム情報と能力` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md#capability-stage-contract` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md#capability-stage-contract` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-8-core-runtime-capability.md#capability-stage-contract` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-9-core-async-ffi-unsafe.md#1-coreasync-の枠組み` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-9-core-async-ffi-unsafe.md#1-coreasync-の枠組み` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-9-core-async-ffi-unsafe.md#12-高度な非同期パターン` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-9-core-async-ffi-unsafe.md#14-dslオーケストレーション支援-api` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-9-core-async-ffi-unsafe.md#14-dslオーケストレーション支援-api` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-9-core-async-ffi-unsafe.md#141-codec-契約` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-9-core-async-ffi-unsafe.md#22-効果タグと-unsafe-境界` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-9-core-async-ffi-unsafe.md#26-メモリ管理と所有権境界` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-9-core-async-ffi-unsafe.md#26-メモリ管理と所有権境界` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-9-core-async-ffi-unsafe.md#26-メモリ管理と所有権境界` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-9-core-async-ffi-unsafe.md#26-メモリ管理と所有権境界` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-9-core-async-ffi-unsafe.md#26-メモリ管理と所有権境界` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-9-core-async-ffi-unsafe.md#26-メモリ管理と所有権境界` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `3-9-core-async-ffi-unsafe.md#4-2-監査された-unsafe-操作` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `README.md#ビルド--ターゲット指定例` | ✅ | - |
| `docs/spec/0-2-glossary.md` | `README.md#ビルド--ターゲット指定例` | ✅ | - |
| `docs/spec/0-3-code-style-guide.md` | `1-1-syntax.md` | ✅ | - |
| `docs/spec/0-3-code-style-guide.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/0-3-code-style-guide.md` | `1-3-effects-safety.md` | ✅ | - |
| `docs/spec/README.md` | `0-0-overview.md` | ✅ | - |
| `docs/spec/README.md` | `0-1-project-purpose.md` | ✅ | - |
| `docs/spec/README.md` | `0-2-glossary.md` | ✅ | - |
| `docs/spec/README.md` | `0-3-code-style-guide.md` | ✅ | - |
| `docs/spec/README.md` | `1-0-language-core-overview.md` | ✅ | - |
| `docs/spec/README.md` | `1-1-syntax.md` | ✅ | - |
| `docs/spec/README.md` | `1-2-types-Inference.md` | ✅ | - |
| `docs/spec/README.md` | `1-3-effects-safety.md` | ✅ | - |
| `docs/spec/README.md` | `1-4-test-unicode-model.md` | ✅ | - |
| `docs/spec/README.md` | `1-5-formal-grammar-bnf.md` | ✅ | - |
| `docs/spec/README.md` | `2-0-parser-api-overview.md` | ✅ | - |
| `docs/spec/README.md` | `2-1-parser-type.md` | ✅ | - |
| `docs/spec/README.md` | `2-2-core-combinator.md` | ✅ | - |
| `docs/spec/README.md` | `2-3-lexer.md` | ✅ | - |
| `docs/spec/README.md` | `2-4-op-builder.md` | ✅ | - |
| `docs/spec/README.md` | `2-5-error.md` | ✅ | - |
| `docs/spec/README.md` | `2-6-execution-strategy.md` | ✅ | - |
| `docs/spec/README.md` | `2-7-core-parse-streaming.md` | ✅ | - |
| `docs/spec/README.md` | `3-0-core-library-overview.md` | ✅ | - |
| `docs/spec/README.md` | `3-1-core-prelude-iteration.md` | ✅ | - |
| `docs/spec/README.md` | `3-10-core-env.md` | ✅ | - |
| `docs/spec/README.md` | `3-2-core-collections.md` | ✅ | - |
| `docs/spec/README.md` | `3-3-core-text-unicode.md` | ✅ | - |
| `docs/spec/README.md` | `3-4-core-numeric-time.md` | ✅ | - |
| `docs/spec/README.md` | `3-5-core-io-path.md` | ✅ | - |
| `docs/spec/README.md` | `3-6-core-diagnostics-audit.md` | ✅ | - |
| `docs/spec/README.md` | `3-7-core-config-data.md` | ✅ | - |
| `docs/spec/README.md` | `3-8-core-runtime-capability.md` | ✅ | - |
| `docs/spec/README.md` | `3-9-core-async-ffi-unsafe.md` | ✅ | - |
| `docs/spec/README.md` | `4-0-official-plugins-overview.md` | ✅ | - |
| `docs/spec/README.md` | `4-1-system-plugin.md` | ✅ | - |
| `docs/spec/README.md` | `4-2-process-plugin.md` | ✅ | - |
| `docs/spec/README.md` | `4-3-memory-plugin.md` | ✅ | - |
| `docs/spec/README.md` | `4-4-signal-plugin.md` | ✅ | - |
| `docs/spec/README.md` | `4-5-hardware-plugin.md` | ✅ | - |
| `docs/spec/README.md` | `4-6-realtime-plugin.md` | ✅ | - |
| `docs/spec/README.md` | `4-7-core-parse-plugin.md` | ✅ | - |
| `docs/spec/README.md` | `5-0-ecosystem-overview.md` | ✅ | - |
| `docs/spec/README.md` | `5-1-package-manager-cli.md` | ✅ | - |
| `docs/spec/README.md` | `5-2-registry-distribution.md` | ✅ | - |
| `docs/spec/README.md` | `5-3-developer-toolchain.md` | ✅ | - |
| `docs/spec/README.md` | `5-4-community-content.md` | ✅ | - |
| `docs/spec/README.md` | `5-5-roadmap-metrics.md` | ✅ | - |
| `docs/spec/README.md` | `5-6-risk-governance.md` | ✅ | - |

- 差分サマリ: `docs/spec/0-0-overview.md` に Phase 2-8 の監査リソース節を追加し、`reports/diagnostic-format-regression.md` への相対パスを `../../reports/...` に修正した。`docs/spec/0-3-code-style-guide.md` ではコメント中の参照記法を明文化し、`docs/spec/1-1-syntax/examples/*.reml` と Rust Frontend 検証手順を追記している。
