# 3.1 Iter/Collector 補完計画（暫定）

## 目的
- `docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` §3 で未達となっている完了条件（API 在庫監査、collect-iterator-audit メトリクス、証跡ログ、CI テスト）を補完し、Phase 3-1 Step3 を Go 判定できる状態へ引き上げる。
- 既存の `prelude_api_inventory.toml`、`collect-iterator-audit-metrics.py`、`core_iter_*` テスト群を実データで連携させ、`reports/spec-audit/*` と `docs/notes/stdlib/core-library-outline.md` の記述を実装と整合させる。

## 背景と課題
- `cargo xtask prelude-audit` には `--section`/`--module` が存在せず、`docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` が求める `--section iter --strict` 実行ログを生成できない。
- `collect-iterator-audit-metrics.py` 側は `--section collectors` の CLI を想定しているが未実装であり、`reports/spec-audit/ch0/links.md` のコマンド例が実行不能。
- `docs/notes/stdlib/core-library-outline.md` や `reports/spec-audit/ch1/iter.json` には存在しない `core_iter_pipeline__*.snap` が引用され、実装状況を誤認させている。
- `Iter` ↔ `Collector` の終端操作や `core_iter_pipeline.rs` がまだ存在せず、完了条件の CI 連携 (`panic_forbidden.rs` と同一ジョブ) を満たせない。

## スコープ
- **含む**: `cargo xtask prelude-audit` の機能拡張、`collect-iterator-audit-metrics.py` とレポート類の同期、関連ドキュメントの修正、`Iter`/`Collector` 向けテスト整備。
- **含まない**: Phase 4 以降の並列イテレータ案、Text/Numeric/Diagnostics 章の仕様改訂。

## 手順概要

### 1. API 在庫監査の実装とログ整備
1. `compiler/xtask/src/main.rs` を拡張し、`--section <Option|Result|Iter|Collector>` または `--module` 指定で `Inventory` エントリをフィルタリングできるようにする。`--section iter` は `module in {"Iter","Collector"}` を対象とする仕様にし、`--strict` と組み合わせて exit code を制御する。
2. `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` を棚卸しし、`Iter`/`Collector` 各 API の `rust_status`・`notes`・`wbs` を現状へ更新、`meta.last_updated` を実施日（例: `2026-02-XX / Remediation Step3`）へ書き換える。
3. `cargo xtask prelude-audit --section iter --strict --baseline docs/spec/3-1-core-prelude-iteration.md` を実行し、出力を `reports/spec-audit/ch1/iter.json` に JSON 形式で保存。`reports/spec-audit/ch0/links.md` と `docs-migrations.log` にコマンドライン・日付・成果を追記する。
4. `docs/guides/tooling/audit-metrics.md` の `core_prelude.missing_api` セクションへ最新の pass/fail 値を反映し、必要に応じ `iterator.api.coverage` KPI を追加。
- 2027-02-24 更新: `cargo xtask prelude-audit --section iter --filter adapter --strict --output reports/spec-audit/ch1/core_iter_adapters.json` を実行し、adapter 12 API の `missing_entries = []` を `run_id = 2027-02-24-iter-adapter-g4` として保存。`reports/spec-audit/ch1/core_iter_adapters.json` へ `inventory_snapshot` を追加し、`iterator.adapter.coverage = 1.0` を `reports/iterator-adapter-metrics.json`・`docs/guides/tooling/audit-metrics.md`（KPI 表）・`docs/notes/stdlib/core-library-outline.md#iter-adapter` に連携した。CLI ログとスナップショット参照は `reports/spec-audit/ch0/links.md#iter-adapters-g4`、更新履歴は `docs-migrations.log` に記録済み。

### 2. collect-iterator-audit メトリクスの実データ化
1. `tooling/ci/collect-iterator-audit-metrics.py` に `--section collectors` を実装し、`collector.effect.*`・`collector.stage.*`・`collector.error.*` を集計する専用ルーチンを追加。`--module iter` など既存の「placeholder」引数は削除せず挙動を定義する。
2. `compiler/frontend/tests/core_iter_collectors.rs` のスナップショットを基に、診断 JSON / 監査ログを生成するスクリプトを作成し、`collect-iterator-audit-metrics.py --section collectors --output reports/iterator-collector-summary.md` を Nightly で実行。
3. `reports/spec-audit/ch0/links.md#collector-f2` を更新し、実際に動作するコマンド・出力ファイル・KPI 値（`collector.effect.mem`, `collector.error.duplicate_key_rate` 等）を掲載。`docs/guides/tooling/audit-metrics.md` の KPI 表へ最新の数値を貼り付ける。
- 2027-03-06 更新（WBS 3.1c-H1）: `Iter::collect_list/collect_vec/collect_string` が Collector を内部的に呼び出す経路でも `collector.effect.*` / `collector.stage.*` / `Diagnostic.extensions["prelude.collector.*"]` を欠落させないため、`compiler/runtime/src/prelude/iter/terminators.rs` に `fn collect_with<C: Collector>`・`CollectOutcome::audit` を導入し、`cargo test --manifest-path compiler/frontend/Cargo.toml core_iter_terminators` → `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case terminators --source reports/spec-audit/ch1/core_iter_terminators.json --output reports/iterator-collector-metrics.json --require-success` → `scripts/validate-diagnostic-json.sh --pattern iterator.collect --pattern prelude.collector reports/spec-audit/ch1/core_iter_terminators.json` の連鎖で監査ログを取得する手順を `reports/spec-audit/ch0/links.md#iter-terminators-h1` に記録。`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml`（collect_list/collect_vec/collect_string）、`reports/iterator-collector-summary.md`、`docs/guides/tooling/audit-metrics.md`（`iterator.collect.bridge_parity`）とクロスリンクして Phase 3-1 判定で参照できるようにした。

### 3. ドキュメント整合性の回復
1. `docs/notes/stdlib/core-library-outline.md` と `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` から存在しない `core_iter_pipeline__*.snap` 参照を一旦削除し、代替として「TODO: pipeline snapshot 作成後にリンク復活」と明記。
2. `reports/spec-audit/ch1/iter.json` のエントリを現実のテスト・スナップショットへ置き換え、未整備のケースは `status = "pending"` でマークする。`reports/spec-audit/ch0/README.md` から参照されるリンクも確認。
3. 上記変更を `docs-migrations.log` に記述し、`docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` §3 の状況説明を更新（例: 「2026-W05 時点で pipeline snapshot 未整備」など）。

### 4. Iter/Collector 実装と CI テスト補完
1. `compiler/runtime/src/prelude/iter/mod.rs` に `impl<T> FromIterator<T> for Iter<T>`、`impl<T> IntoIterator for Iter<T>`、および `Iter::collect_list`/`collect_vec`/`try_collect` 終端操作を実装し、`collectors` モジュールと `EffectSet`/`StageProfile` 連携を確立する。
2. 新規 `compiler/frontend/tests/core_iter_pipeline.rs` を作成し、`Iter::from_list |> Iter::map |> Iter::collect_list` など 6 シナリオを `insta` で固定。`core_iter_effects.rs`（効果タグ検証）も同時に用意し、`RUSTFLAGS="-Zpanic-abort-tests"` で実行する。
3. CI 設定（`.github/workflows/` または `tooling/ci/record-metrics.sh`）に `cargo test --manifest-path compiler/frontend/Cargo.toml core_iter_pipeline core_iter_effects` を追加し、`scripts/validate-diagnostic-json.sh --pattern iterator --pattern collector` を同じジョブで走らせる。
4. テスト追加後、`collect-iterator-audit-metrics.py --section iterator --case pipeline` を実行して KPI を採取し、`reports/iterator-stage-summary.md`・`reports/spec-audit/ch1/iter.json` に追記。`docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` の完了条件チェックを更新する。

### 5. G1 map/filter 立ち上げ手順（W37 後半）
- `compiler/runtime/src/prelude/iter/adapters/map.rs` / `filter.rs` を追加し、`IterState::adapter` → `AdapterPlan` → `IteratorStageProfile` のチェーンを再計算できるようにする。`EffectLabels::predicate_calls` を `collect-iterator-audit-metrics.py --section iterator --case map|filter --output reports/iterator-map-filter-metrics.json` で取得し、`reports/spec-audit/ch0/links.md#iter-g1-map-filter` にコマンドログを貼り付ける。
- `compiler/frontend/tests/core_iter_adapters.rs` へ `map_pipeline` / `filter_effect` / `map_filter_chain_panic_guard` を追加し、`cargo test core_iter_adapters -- --nocapture` と `cargo insta review` を CI に組み込む。同ジョブで `scripts/validate-diagnostic-json.sh --pattern iterator.map --pattern iterator.filter` を実行して `reports/diagnostic-format-regression.md` に差分なしで反映する。2026-02-18 時点で `tests/snapshots/core_iter_adapters__core_iter_adapters.snap` が確定し、`EffectLabels::predicate_calls = 4` を含む JSON を共有済み。
- `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `Iter.map` / `Iter.filter` 行に KPI・テスト名・効果タグの取り扱いを追記し、`rust_status = "working"` → `implemented` への遷移条件を明示する。更新後は `docs-migrations.log` に「WBS 3.1c-G1 map/filter 立ち上げ」を追記する。
- `docs/guides/tooling/audit-metrics.md` へ `iterator.map.latency` と `iterator.filter.predicate_count` を KPI として追加し、Nightly 実行で集めた値を記録する。閾値を超えた場合は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` にエスカレーションする。
- `docs/notes/stdlib/core-library-outline.md#iter-g1-map-filter` と `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` §3.0.3a に G1 実行ログをリンクし、Phase 3-2 以降の Adapter/Collections 計画から参照できるようにする。

### 6. G2 flat_map / zip Stage 適用（W38 前半）
1. `compiler/runtime/src/prelude/iter/adapters/flat_map.rs` を `IterSeed::FlatMap` ベースへ再設計し、ネストされた `Iter` を消費するときに `IteratorStageProfile::request_stage(Stage::Beta)` と `EffectLabels::mem_reservation` を増分更新する。`collect-iterator-audit-metrics.py --section iterator --case flat_map --output reports/iterator-flatmap-metrics.json --require-success` を実行し、`reports/spec-audit/ch0/links.md#iter-adapters` にコマンド履歴と KPI を追記する。
2. `iter/adapters/zip.rs` に長さ差検知ロジック (`ZipState::remaining_left/right`) を導入し、短い入力が検出された場合は `iterator.error.zip_shorter` を `Diagnostic.extensions["iterator.zip"]` に記録する。`compiler/frontend/tests/core_iter_adapters.rs::zip_mismatch` を `insta` で更新し、`scripts/validate-diagnostic-json.sh --pattern iterator.zip` の出力を `reports/diagnostic-format-regression.md#iterator.zip_mismatch` に保存する。
3. `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `Iter.flat_map` 行に `effect = "effect {mem}"`、`Iter.zip` 行に `effect = "@pure / effect {mut}"` と `notes = "flat_map_vec / zip_mismatch snapshot, reports/iterator-flatmap-metrics.json"` 等を追記し、`cargo xtask prelude-audit --section iter --filter adapter --strict` の再実行結果を `reports/spec-audit/ch1/core_iter_adapters.json` に反映する。更新日は `meta.last_updated = "2026-02-XX / WBS 3.1c-G2"` へ繰り上げ、`docs-migrations.log` に記録する。
4. `docs/notes/stdlib/core-library-outline.md#iter-g2-flat-zip` と `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md#iter-adapter` へ Stage/Effect の判断基準、KPI 連携（`reports/iterator-stage-summary.md`, `reports/iterator-flatmap-metrics.json`, `reports/diagnostic-format-regression.md#iterator.zip_mismatch`）を記述する。
5. `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に「zip 長さ差 / flat_map mem 逸脱」用のテンプレート項目を追加し、Nightly で `collect-iterator-audit-metrics.py --section iterator --case zip` または `flat_map` が失敗した際のエスカレーション経路を確立する。
- 2026-02-21 更新: 上記 G2 実装は `cargo test core_iter_adapters -- --include-ignored flat_map_vec zip_mismatch` と `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case flat_map --case zip --output reports/iterator-flatmap-metrics.json --secondary-output reports/iterator-zip-metrics.json --require-success` で実測済み。`reports/iterator-flatmap-metrics.json` には `adapter_metrics.flat_map_vec.effects.mem = true` / `mem_reservation_bytes = 3` を、`reports/iterator-zip-metrics.json` には `adapter_metrics.zip_mismatch.iterator.error.zip_shorter = 1` / `stage.required = Exact("stable")` を記録。`reports/spec-audit/ch1/core_iter_adapters.json`・`reports/spec-audit/ch0/links.md#iter-adapters`・`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml`・`docs/guides/tooling/audit-metrics.md`・`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`・`docs/notes/stdlib/core-library-outline.md#iter-g2-flat-zip` へ同じ Run-ID を反映し、`docs-migrations.log` に WBS 3.1c-G2 完了ログを追記した。

## 成果物
- 更新済み `cargo xtask prelude-audit` ソースと CLI ドキュメント
- 最新 `prelude_api_inventory.toml`・`reports/spec-audit/ch1/iter.json`・`reports/spec-audit/ch0/links.md`・`reports/iterator-{stage,collector}-summary.md`
- 補正文書: `docs/notes/stdlib/core-library-outline.md`, `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md`, `docs-migrations.log`
- 新規/更新テスト: `core_iter_pipeline.rs`, `core_iter_effects.rs`, CI 設定

## フォローアップ
- 実装完了後に `3-1-core-prelude-iteration-plan.md` §3 の「完了条件」を実データで再検証し、Step4 へ進む判断を Phase 3-0 M1 ミーティングで下す。
- KPI の継続監視は `tooling/ci/collect-iterator-audit-metrics.py --require-success` を Nightly に組み込み、逸脱時は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` にタスクを登録する運用へ移行する。

### 5. buffered/backpressure 監査の追加手順（W38 中盤）
1. `compiler/runtime/src/prelude/iter/adapters/buffered.rs` にリングバッファ実装を追加し、`IteratorStageProfile::request_stage(Stage::Experimental)` と `EffectLabels::mem_bytes` を更新できるようにする。`compiler/frontend/tests/core_iter_adapters.rs::buffered_window` でバッファ深度とドロップ件数を snapshot 化し、`reports/spec-audit/ch1/core_iter_adapters.json` / `reports/spec-audit/ch1/iterator.buffered.diagnostics.json` に記録する。
2. Nightly CI に `cargo test --manifest-path compiler/frontend/Cargo.toml core_iter_adapters -- --include-ignored buffered_window`、`cargo bench -p compiler-rust-frontend iter_buffered`、`python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case buffered --output reports/iterator-buffered-metrics.json --require-success` を直列で登録し、`iterator.mem.window.bytes`（リングバッファ容量）と `iterator.mem.window.backpressure`（`dropped / produced`）を `reports/iterator-buffered-metrics.json` へ保存する。ベンチ結果は `reports/benchmarks/iter_buffered-YYYY-MM-DD.json` として残し、±10% 以内の性能維持を確認する。
3. `docs/guides/tooling/audit-metrics.md` の KPI 表へ `iterator.mem.window` を追加し、`docs/plans/rust-migration/3-2-benchmark-baseline.md` §3.2.4 とリンクさせる。Run-ID は `reports/spec-audit/ch0/links.md#iter-buffered` / `reports/iterator-stage-summary.md#iter-buffered` / `docs-migrations.log` に残し、`docs/notes/stdlib/core-library-outline.md#iter-g3-buffered-backpressure` と `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md#iter-adapter` から参照できるようにする。
4. 逸脱時（`collect-iterator-audit-metrics` 失敗、`delta_pct` > ±10% など）は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#iterator-adapter-esc` にフォローアップを登録し、`reports/spec-audit/ch1/core_iter_adapters.json` の `buffered_window` ケースへ `status = "failed"` を記録する。
5. 2027-02-22 実施結果: `cargo test --manifest-path compiler/frontend/Cargo.toml core_iter_adapters -- --include-ignored buffered_window` → `cargo bench -p compiler-rust-frontend iter_buffered -- warmup-time 3 --measurement-time 10` → `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case buffered --output reports/iterator-buffered-metrics.json --require-success` を完了。`reports/iterator-buffered-metrics.json` に `adapter_metrics.buffered_window.effects.mem_bytes = 2` / `backpressure = {"produced": 6, "consumed": 4, "dropped": 2, "ratio": 0.33}` を記録し、Criterion 結果 `reports/benchmarks/iter_buffered-2027-02-22.json` で `windows_per_sec = 1.89e6` / `delta_pct = +0.038`（±10% 以内）を確認した。Run-ID `2027-02-22-iter-adapter-g3` を `reports/spec-audit/ch0/links.md#iter-buffered` と `reports/iterator-stage-summary.md#iter-buffered`、`docs-migrations.log`、`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` に反映済み。
