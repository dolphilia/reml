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
| `python3 tooling/ci/render-collector-audit-fixtures.py --snapshots compiler/rust/frontend/tests/__snapshots__/core_iter_collectors.snap --output reports/spec-audit/ch1/core_iter_collectors.json --audit-output reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` | ✅ | `core_iter_collectors.snap` から `diagnostics`/監査ログを生成。`collector.effect.*`/`collector.stage.*` を F0 の仕様確認項目と結び付け、`reports/spec-audit/ch1/core_iter_collectors.json` をソース・オブ・トゥルースとして固定。 |

### Collector F2 実装ログ（WBS 3.1b, W37 前半）

| コマンド | 結果 | 備考 |
| --- | --- | --- |
| `cargo test core_iter_collectors -- --nocapture` | ✅ | `compiler/rust/frontend/tests/core_iter_collectors.rs` に追加する 7 シナリオ（List/Vec/Map/Set/String/Table baseline/duplicate）を `insta` snapshot 化し、Collector 実装の回帰を監視。 |
| `cargo insta review --review` | ✅ | `core_iter_collectors.snap` を確定し、`prelude.collector.kind`/`effects`/`error_kind` を JSON で固定。 |
| `python3 tooling/ci/collect-iterator-audit-metrics.py --section collectors --module iter --case wbs-31b-f2 --source reports/spec-audit/ch1/core_iter_collectors.json --audit-source reports/spec-audit/ch1/core_iter_collectors.audit.jsonl --output reports/iterator-collector-metrics.json` | ✅ | `collector.effect.audit_snapshot` の実データ化に成功（`collector.stage.audit_pass_rate=1.0`、`collector.effect.mem=2/7`、`collector.effect.mut=4/7`、`collector.error.duplicate_key=2`、`collector.error.invalid_encoding=1`、`collector.error.rate_per_total=0.4286`）。出力 JSON を `reports/iterator-collector-metrics.json` として保存し、`reports/iterator-collector-summary.md`／`0-3-audit-and-metrics.md` の KPI と同期。 |
| `scripts/validate-diagnostic-json.sh --pattern collector` | ✅ | `prelude.collector.*` キーが `reports/diagnostic-format-regression.md` に差分なしで反映されるかを確認。 |
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

### <a id="iterator-f3"></a>Iter F3 Snapshot/KPI（WBS 3.1a）

| コマンド | 結果 | 備考 |
| --- | --- | --- |
| `cargo test --manifest-path compiler/rust/frontend/Cargo.toml --test core_iter_pipeline` | ✅ | `compiler/rust/frontend/tests/core_iter_pipeline.rs` の 6 シナリオを `tests/snapshots/core_iter_pipeline__core_iter_pipeline.snap` に保存し、`reports/spec-audit/ch1/iter.json#audit_cases.pipeline` / `reports/iterator-stage-summary.md` の KPI に反映。 |
| `cargo test --manifest-path compiler/rust/frontend/Cargo.toml --test core_iter_effects` | ✅ | `core_iter_effects.rs` の `EffectLabels`/`TryCollectError` ケースを `tests/snapshots/core_iter_effects__*.snap` に固定し、`reports/spec-audit/ch1/iter.json#audit_cases.effects` を更新。 |
| `cargo test --manifest-path compiler/rust/frontend/Cargo.toml --test core_iter_collectors` | ✅ | `core_iter_collectors.rs` の 7 ケースを `tests/__snapshots__/core_iter_collectors.snap` に反映し、`reports/iterator-collector-summary.md` の KPI と同期。 |
| `cargo test --manifest-path compiler/rust/frontend/Cargo.toml --test core_iter_terminators` | ✅ | `Iter::fold`/`reduce`/`all`/`any`/`find`/`try_fold` の新規テストを実施し、`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` (`try_fold`, `try_collect`) の `rust_status` を更新。 |
| `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case pipeline --source compiler/ocaml/tests/golden/_actual/typeclass_iterator_stage_mismatch.actual.json --output reports/iterator-stage-metrics.json --require-success` | ✅ | `iterator.stage.audit_pass_rate=1.0`、`collector.effect.mem=0`、`collector.error.duplicate_key=1` を `reports/iterator-stage-metrics.json` に記録し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` と同期。 |
* `reports/spec-audit/ch1/iter.json` には `audit_cases.pipeline`/`effects`/`try_collect_errors` と KPI (`iterator.stage.audit_pass_rate=1.0`, `collector.effect.mem=0`) を記録し、`reports/iterator-stage-summary.md` では同値をテキストで要約している。`docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md#3-itercollector-完了条件` と相互参照。
* `reports/iterator-collector-summary.md` には上述の 7 ケースの `collector.effect.*`/`collector.error.*` KPI をまとめて記録し、`collect_list_baseline` で `collector.effect.mem=0`、`collect_vec_mem_reservation` で `collector.effect.mem_reservation>0` を `collect-iterator-audit` JSON に反映すると同時に、`collect_string_invalid` の `collector.error.invalid_encoding`、`collect_table_duplicate` の `collector.error.key` を KPI に並列させて `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` と整合させている。

### <a id="collector-f2-監査ログ"></a>Collector F2 監査ログ（WBS 3.1b）

- `python3 tooling/ci/render-collector-audit-fixtures.py --snapshots compiler/rust/frontend/tests/__snapshots__/core_iter_collectors.snap --output reports/spec-audit/ch1/core_iter_collectors.json --audit-output reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` で `prelude.collector` スナップショットを診断 JSON（7 ケース）と audit JSONL へ変換し、Stage/Effect/Marker 情報を `AuditEnvelope.metadata.collector.*` に転写した。
- `python3 tooling/ci/collect-iterator-audit-metrics.py --section collectors --module iter --case wbs-31b-f2 --source reports/spec-audit/ch1/core_iter_collectors.json --audit-source reports/spec-audit/ch1/core_iter_collectors.audit.jsonl --output reports/iterator-collector-metrics.json` を実行し、`collector.stage.audit_pass_rate=1.0`・`collector.effect.mem=2/7`・`collector.effect.mut=4/7`・`collector.effect.mem_reservation=4`・`collector.effect.reserve=2`・`collector.error.duplicate_key=2`・`collector.error.invalid_encoding=1` を採取。`reports/iterator-collector-summary.md` と `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI に同じ数値を貼り付けた。
- `scripts/validate-diagnostic-json.sh --pattern collector` で `Diagnostic.extensions["prelude.collector.*"]` が `reports/diagnostic-format-regression.md` に差分なしで保存され、`cargo xtask prelude-audit --wbs '3.1b F2'` で `Collector` トレイトおよび `List/Vec/Map/Set/String/Table` API の `rust_status=implemented` を検査したログと結び付いている。
- `../../../docs/notes/core-library-outline.md#collector-f2-監査ログ` と `../../../docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md#collector-f2-監査ログ` にこの `Collector F2` ログのクロスリファレンスを張り、監査ログ（コマンド履歴＋KPI）を M1 レビューで再利用できる形でまとめてある。

### <a id="collector-f3-監査ログ"></a>Collector F3 監査ログ（WBS 3.1b, W37 後半）

| コマンド | 結果 | 備考 |
| --- | --- | --- |
| `python3 tooling/ci/collect-iterator-audit-metrics.py --section collectors --module iter --case wbs-31b-f2 --source reports/spec-audit/ch1/core_iter_collectors.json --audit-source reports/spec-audit/ch1/core_iter_collectors.audit.jsonl --output reports/iterator-collector-metrics.json --require-success` | ✅ | `collector.stage.audit_pass_rate=1.0`、`collector.effect.mem=2/7`、`collector.effect.mut=4/7`、`collector.error.duplicate_key=2`、`collector.error.invalid_encoding=1` を `reports/iterator-collector-metrics.json` に保存し、`reports/iterator-collector-summary.md` と `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI を更新。 |
| `scripts/validate-diagnostic-json.sh --pattern collector` | ✅ | 監査ログ生成後に Diagnostic JSON を再比較し、`reports/diagnostic-format-regression.md` に差分なしで吸収されたことを確認。`reports/spec-audit/ch1/core_iter_collectors.json` と `core_iter_collectors.audit.jsonl` のヘッダをリスト化。 |
- `reports/iterator-collector-summary.md` の `collect_vec_mem_reservation`/`collect_map_duplicate`/`collect_string_invalid` では `collector.effect.mem_reservation` や `collector.error.key` の推移を JSON で参照でき、`docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md#31b-実装指針` と突き合わせた。`docs/notes/core-library-outline.md#collector-f3-監査ログ` にも同じリンク列を記録済み。

### Iter Generators

（WBS 3.1c-F1-3 で `Iter::from_list`/`Iter::from_result`/`Iter::from_fn`/`Iter::empty`/`Iter::once`/`Iter::repeat`/`Iter::range` を生成 API として実装し、`ListCollector` 相当の Stage・Effect を `CollectOutcome` と同期したログ）

| コマンド | 結果 | 備考 |
| --- | --- | --- |
| `RUSTFLAGS="-Zpanic-abort-tests" cargo test core_iter_generators -- --nocapture` | ✅ | `core_iter_generators.rs` に `from_list_roundtrip`/`from_result_passthrough`/`from_fn_counter`/`range_basic`/`range_overflow_guard`/`repeat_take`/`empty_collect` を追加し、`IterSeed` が `EffectLabels::residual = []` を維持したまま `Iter` を生成できることを `insta` スナップショットで検証。 |
| `cargo insta review --review core_iter_generators` | ✅ | `compiler/rust/frontend/tests/snapshots/core_iter_generators__*.snap` を承認し、`ListCollector`（Stage=stable）・`Result`/`FnMut` 変換時の差分をレビュー済み。 |
| `collect-iterator-audit --section iter --case from_list` | ✅ | `collector.effect.mem=0` と `iterator.stage.audit_pass_rate=1.0` を `reports/spec-audit/ch1/iter.json#audit_cases.from_list` に保存し、`Iter::from_list` が `ListCollector` と同一 Stage を報告することを確認。 |
| `collect-iterator-audit --section iter --case from_result` | ✅ | `reports/spec-audit/ch1/iter.json#audit_cases.from_result` で `iterator.error.* = 0` を確認し、`Result<T,E>` に潜在する `Err` が `Iter` へ伝播しないことを監査ログに記録。 |
| `collect-iterator-audit --section iter --case from_fn` | ✅ | `reports/spec-audit/ch1/iter.json#audit_cases.from_fn` に `iterator.residual_effects = []` を保存し、クロージャ生成でも `@pure` を維持できることを可視化。 |
| `collect-iterator-audit --section iter --case range` | ✅ | `reports/spec-audit/ch1/iter.json#audit_cases.range` で `iterator.range.overflow_guard=1` を確認し、`IterRangeError` が監査ログへ書き出されることを証跡化。 |
| `collect-iterator-audit --section iter --case repeat` | ✅ | `reports/spec-audit/ch1/iter.json#audit_cases.repeat` に `iterator.repeat.flagged=true` が残り、`diagnostic.extensions["iterator.repeat"]` と同期できていることを確認。 |
| `collect-iterator-audit --section iter --case once` | ✅ | `reports/spec-audit/ch1/iter.json#audit_cases.once` に `iterator.once.length=1` を保存し、単一要素ストリームの stage/effect が `@pure` であることを保証。 |
| `collect-iterator-audit --section iter --case empty` | ✅ | `reports/spec-audit/ch1/iter.json#audit_cases.empty` に `iterator.empty.items=0` を記録し、ゼロ要素生成器でも `iterator.stage.audit_pass_rate=1.0` を維持していることを確認。 |
| `collect-iterator-audit --section iter --case unfold` | ✅ | `reports/spec-audit/ch1/iter.json#audit_cases.unfold` に `iterator.unfold.depth=8` を記録し、`EffectLabels::residual=[]` のまま Stage=beta を維持していることを確認。 |
| `collect-iterator-audit --section iter --case try_unfold` | ✅ | `reports/spec-audit/ch1/iter.json#audit_cases.try_unfold` に `iterator.try_unfold.error_kind="try_unfold"` と `EffectLabels::debug=true` を保存し、診断と監査ログの整合を検証。 |
| `cargo xtask prelude-audit --section iter --baseline docs/spec/3-1-core-prelude-iteration.md --wbs 3.1c-F1-5` | ✅ | 生成 API 15 件を `iterator.api.coverage=1.0` として測定し、`reports/spec-audit/ch1/iter.json` から `pending_entries` を解消。`prelude_api_inventory.toml` の `meta.last_updated` を `2025-12-22 / WBS 3.1c-F1-4/5` に更新し、Nightly で JSON 差分を監視する。 |
- `reports/spec-audit/ch1/iter.json` では `audit_cases.from_list/from_result/from_fn/empty/once/repeat/range/unfold/try_unfold` に `collect-iterator-audit --section iter --case ...` の KPI を保存し、`snapshots` セクションで `core_iter_generators__*.snap` 9 ケース分を `cargo insta review` と突き合わせられるようにした。`references` に `docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md#33b-生成-api-実装ステップ（wbs-31c-f1）` と `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` を追記し、計画書 ↔ 監査ログの往復リンクを確保済み。 |

| シナリオID | Snapshot | KPI / 監査ログ | 備考 |
| --- | --- | --- | --- |
| `iter_from_list_roundtrip` | `compiler/rust/frontend/tests/snapshots/core_iter_generators__from_list_roundtrip.snap` | `reports/spec-audit/ch1/iter.json#audit_cases.from_list` | `ListCollector` ノード再利用と Stage=beta 確定 (`collector.effect.mem=0`)。 |
| `iter_from_result_passthrough` | `compiler/rust/frontend/tests/snapshots/core_iter_generators__from_result_passthrough.snap` | `reports/spec-audit/ch1/iter.json#audit_cases.from_result` | `Result<T,E>` の `Ok` のみをストリーム化し residual effect を空集合で維持。 |
| `iter_from_fn_counter` | `compiler/rust/frontend/tests/snapshots/core_iter_generators__from_fn_counter.snap` | `reports/spec-audit/ch1/iter.json#audit_cases.from_fn` | `FnMut() -> Option<T>` ベースの生成器を `IterSeed` で包み、`iterator.stage.audit_pass_rate=1.0` を確認。 |
| `iter_range_basic` | `compiler/rust/frontend/tests/snapshots/core_iter_generators__range_basic.snap` | `reports/spec-audit/ch1/iter.json#audit_cases.range` | `RangeState` が `effect=@pure` のままインクリメントし、`iterator.range.overflow_guard=1` を監査。 |
| `iter_range_overflow_guard` | `compiler/rust/frontend/tests/snapshots/core_iter_generators__range_overflow_guard.snap` | `reports/spec-audit/ch1/iter.json#audit_cases.range` | 上限超過時に `IterRangeError::Overflow` を `IterStep::Error` で返す経路を固定。 |
| `iter_repeat_take` | `compiler/rust/frontend/tests/snapshots/core_iter_generators__repeat_take.snap` | `reports/spec-audit/ch1/iter.json#audit_cases.repeat` | 無限列を `take(3)` 相当で截断し、`diagnostic.extensions["iterator.repeat"]` が観測できることを確認。 |
| `iter_once_collect` | `compiler/rust/frontend/tests/snapshots/core_iter_generators__once_collect.snap` | `reports/spec-audit/ch1/iter.json#audit_cases.once` | 単一要素の生成が `iterator.once.length=1` を維持し `@pure` であることを確認。 |
| `iter_empty_collect` | `compiler/rust/frontend/tests/snapshots/core_iter_generators__empty_collect.snap` | `reports/spec-audit/ch1/iter.json#audit_cases.empty` | ゼロ要素ケースで `EffectLabels::residual = []` を保ちつつ `iterator.empty.items=0` を記録。 |
| `iter_unfold_fibonacci_pipeline` | `compiler/rust/frontend/tests/snapshots/core_iter_generators__unfold_fibonacci_pipeline.snap` | `reports/spec-audit/ch1/iter.json#audit_cases.unfold` | `Iter::unfold` が `iterator.unfold.depth=8` を報告し、Stage=beta を維持。 |
| `iter_try_unfold_error_passthrough` | `compiler/rust/frontend/tests/snapshots/core_iter_generators__try_unfold_error_passthrough.snap` | `reports/spec-audit/ch1/iter.json#audit_cases.try_unfold` | `EffectLabels::debug=true` と `iterator.error.kind="try_unfold"` を同時に記録。 |

- `reports/spec-audit/ch1/iter.json` では `iterator.stage.audit_pass_rate=1.0`、`iterator.range.overflow_guard=1`、`iterator.repeat.flagged=true`、`iterator.once.length=1`、`iterator.empty.items=0`、`collector.effect.mem=0`、`iter.generators.entries=15`、`iterator.unfold.depth=8`、`iterator.try_unfold.error_kind="try_unfold"`、`iterator.api.coverage=1.0` を `references` セクション（`docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md`, `0-3-audit-and-metrics.md`）と連携させた。 |
### Iter F1-4 `unfold`/`try_unfold` 実装ログ（WBS 3.1c-F1-4） {#iter-f1-4}

| コマンド | 結果 | 備考 |
| --- | --- | --- |
| `cargo test core_iter_generators::unfold_fibonacci_pipeline -- --nocapture` | ✅ | `Iter::unfold` が `EffectSet::PURE` を維持し `ListCollector` へ往復できることを確認。`reports/spec-audit/ch1/iter.json#audit_cases.unfold` に `iterator.unfold.depth=8`・`iterator.stage.audit_pass_rate=1.0` を記録。 |
| `cargo test core_iter_generators::try_unfold_error_passthrough -- --nocapture` | ✅ | `Err(E)` の発生が `iterator.error.kind="try_unfold"` として `Diagnostic.extensions` および `AuditEnvelope.metadata.iterator.error.kind` に現れることを Snapshot 確認。 |
| `cargo insta review --review core_iter_generators --filter unfold` | ✅ | `compiler/rust/frontend/tests/snapshots/core_iter_generators__unfold_fibonacci_pipeline.snap` / `__try_unfold_error_passthrough.snap` を承認し、生成 API 追加分の差分を固定。 |
| `collect-iterator-audit --section iter --case unfold|try_unfold --output reports/spec-audit/ch1/iter.json` | ✅ | `unfold` は `EffectLabels::residual = []`、`try_unfold` は `EffectLabels::debug=true` と `AuditEnvelope.metadata.iterator.error.kind` を JSON KPI に追加し、`audit_cases.unfold/try_unfold` に保存。 |
| `scripts/validate-diagnostic-json.sh --pattern iterator --module try_unfold` | ✅ | `iterator.error.kind` が `reports/diagnostic-format-regression.md` に差分なしで反映されることを確認。 |
| `cargo xtask prelude-audit --section iter --wbs '3.1c F1-4' --baseline docs/spec/3-1-core-prelude-iteration.md` | ✅ | `Iter::unfold`/`Iter::try_unfold` の仕様差分と `prelude_api_inventory.toml`（`wbs = "3.1c F1-4"`、`rust_status=implemented`）の整合を自動で検証。 |

| シナリオID | Snapshot | KPI / 監査ログ | 仕様根拠・備考 |
| --- | --- | --- | --- |
| `unfold_fibonacci_pipeline` | `compiler/rust/frontend/tests/snapshots/core_iter_generators__unfold_fibonacci_pipeline.snap` | `reports/spec-audit/ch1/iter.json#audit_cases.unfold` | `Iter::unfold` の `@pure` 生成器が `ListCollector` へ往復し `iterator.unfold.depth=8` を維持することを証跡化。【F:docs/spec/3-1-core-prelude-iteration.md†L176-L200】 |
| `try_unfold_error_passthrough` | `compiler/rust/frontend/tests/snapshots/core_iter_generators__try_unfold_error_passthrough.snap` | `reports/spec-audit/ch1/iter.json#audit_cases.try_unfold` | `Result<Option<(T, State)>, E>` の `Err(E)` が `EffectLabels::debug=true` と共に監査ログへ記録される経路を固定。【F:docs/spec/3-1-core-prelude-iteration.md†L200-L221】 |

- `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` では `module = "Iter"` に `unfold`/`try_unfold` エントリ（`rust_status=implemented`, `wbs = "3.1c F1-4"`, `last_updated = "2025-12-22 / WBS 3.1c-F1-4/5"`）を登録し、`docs/notes/core-library-outline.md#iter-f1-生成-api-監査ログ` と `#iter-generators-f1-4-設計メモwbs-31c-f1-4` で `collect-iterator-audit` コマンドと KPI を共有する。Phase 3 M1 判定では本節と計画書の双方を根拠資料として参照する。

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

### Iter/Collector Remediation (2025-11-20)

| コマンド | 結果 | 備考 |
| --- | --- | --- |
| `cargo xtask prelude-audit --section iter --strict --baseline docs/spec/3-1-core-prelude-iteration.md` | ⚠️（未実装 2 件） | `Iter::try_fold` / `Iter::try_collect` が `rust_status=pending` のため Strict 失敗。`reports/spec-audit/ch1/iter.json` に 38 件中 36 件完了・2 件 pending のサマリを保存し、Collector 領域は 100% 実装済みと確認。 |

### <a id="iter-g1-map-filter"></a>Iter G1 map/filter ログ（WBS 3.1c-G1）

| コマンド | 結果 | 備考 |
| --- | --- | --- |
| `cargo test --manifest-path compiler/rust/frontend/Cargo.toml core_iter_adapters -- --nocapture map_pipeline filter_effect map_filter_chain_panic_guard` | ✅ | `map_pipeline` / `filter_effect` / `map_filter_chain_panic_guard` を `insta` snapshot と合わせて実行。`Iter::map`/`Iter::filter` のチェーンが VecCollector まで通ること、`EffectLabels::predicate_calls` が 入力要素数（`filter_effect` は 4）に一致することを確認。 |
| `tmp/iter_metrics (CARGO_NET_OFFLINE=true cargo run)` | ✅ | `reml_runtime_ffi::core_prelude::iter` を直接呼び出し、`map_pipeline`/`filter_effect` の Stage/Efffect/latency（ns）を JSON（`tmp/iterator_map_filter_raw.json`）で取得。 |
| `python3 tooling/ci/collect-iterator-audit-metrics.py --source reports/spec-audit/ch1/iterator.map-filter.diagnostics.json --section iterator --case map --case filter --output reports/iterator-map-filter-metrics.json --require-success` | ✅ | 新規診断（`reports/spec-audit/ch1/iterator.map-filter.diagnostics.json`）を入力に Stage 監査を実行。`iterator.stage.audit_pass_rate=1.0 (total=2)`・`diagnostic.audit_presence_rate=1.0` を `reports/iterator-map-filter-metrics.json` に記録し、`adapter_metrics.filter_effect.effects.predicate_calls=4` を KPI に加える。 |
| `scripts/validate-diagnostic-json.sh --pattern iterator.map --pattern iterator.filter reports/spec-audit/ch1/iterator.map-filter.diagnostics.json` | ✅ | map/filter 用診断 JSON が schema v2.0.0-draft に適合することを確認。 |

- KPI: `reports/iterator-map-filter-metrics.json` の `adapter_metrics` には `iterator.map.latency_ns = 16750`、`iterator.filter.latency_ns = 2875`、`iterator.filter.predicate_calls = 4` を保存。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の `iterator.map.latency` / `iterator.filter.predicate_count` と同期済み。
- 関連資料: `docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` §4.a、`docs/plans/bootstrap-roadmap/3-1-iter-collector-remediation.md` §5、`docs/notes/core-library-outline.md#iter-g1-map-filter`、`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml`（`Iter.map`/`Iter.filter` 行）。
