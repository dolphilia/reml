# 3.1 Iter/Collector 補完計画（暫定）

## 目的
- `docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` §3 で未達となっている完了条件（API 在庫監査、collect-iterator-audit メトリクス、証跡ログ、CI テスト）を補完し、Phase 3-1 Step3 を Go 判定できる状態へ引き上げる。
- 既存の `prelude_api_inventory.toml`、`collect-iterator-audit-metrics.py`、`core_iter_*` テスト群を実データで連携させ、`reports/spec-audit/*` と `docs/notes/core-library-outline.md` の記述を実装と整合させる。

## 背景と課題
- `cargo xtask prelude-audit` には `--section`/`--module` が存在せず、`docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` が求める `--section iter --strict` 実行ログを生成できない。
- `collect-iterator-audit-metrics.py` 側は `--section collectors` の CLI を想定しているが未実装であり、`reports/spec-audit/ch0/links.md` のコマンド例が実行不能。
- `docs/notes/core-library-outline.md` や `reports/spec-audit/ch1/iter.json` には存在しない `core_iter_pipeline__*.snap` が引用され、実装状況を誤認させている。
- `Iter` ↔ `Collector` の終端操作や `core_iter_pipeline.rs` がまだ存在せず、完了条件の CI 連携 (`panic_forbidden.rs` と同一ジョブ) を満たせない。

## スコープ
- **含む**: `cargo xtask prelude-audit` の機能拡張、`collect-iterator-audit-metrics.py` とレポート類の同期、関連ドキュメントの修正、`Iter`/`Collector` 向けテスト整備。
- **含まない**: Phase 4 以降の並列イテレータ案、Text/Numeric/Diagnostics 章の仕様改訂。

## 手順概要

### 1. API 在庫監査の実装とログ整備
1. `compiler/rust/xtask/src/main.rs` を拡張し、`--section <Option|Result|Iter|Collector>` または `--module` 指定で `Inventory` エントリをフィルタリングできるようにする。`--section iter` は `module in {"Iter","Collector"}` を対象とする仕様にし、`--strict` と組み合わせて exit code を制御する。
2. `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` を棚卸しし、`Iter`/`Collector` 各 API の `rust_status`・`notes`・`wbs` を現状へ更新、`meta.last_updated` を実施日（例: `2026-02-XX / Remediation Step3`）へ書き換える。
3. `cargo xtask prelude-audit --section iter --strict --baseline docs/spec/3-1-core-prelude-iteration.md` を実行し、出力を `reports/spec-audit/ch1/iter.json` に JSON 形式で保存。`reports/spec-audit/ch0/links.md` と `docs-migrations.log` にコマンドライン・日付・成果を追記する。
4. `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の `core_prelude.missing_api` セクションへ最新の pass/fail 値を反映し、必要に応じ `iterator.api.coverage` KPI を追加。

### 2. collect-iterator-audit メトリクスの実データ化
1. `tooling/ci/collect-iterator-audit-metrics.py` に `--section collectors` を実装し、`collector.effect.*`・`collector.stage.*`・`collector.error.*` を集計する専用ルーチンを追加。`--module iter` など既存の「placeholder」引数は削除せず挙動を定義する。
2. `compiler/rust/frontend/tests/core_iter_collectors.rs` のスナップショットを基に、診断 JSON / 監査ログを生成するスクリプトを作成し、`collect-iterator-audit-metrics.py --section collectors --output reports/iterator-collector-summary.md` を Nightly で実行。
3. `reports/spec-audit/ch0/links.md#collector-f2` を更新し、実際に動作するコマンド・出力ファイル・KPI 値（`collector.effect.mem`, `collector.error.duplicate_key_rate` 等）を掲載。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 表へ最新の数値を貼り付ける。

### 3. ドキュメント整合性の回復
1. `docs/notes/core-library-outline.md` と `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` から存在しない `core_iter_pipeline__*.snap` 参照を一旦削除し、代替として「TODO: pipeline snapshot 作成後にリンク復活」と明記。
2. `reports/spec-audit/ch1/iter.json` のエントリを現実のテスト・スナップショットへ置き換え、未整備のケースは `status = "pending"` でマークする。`reports/spec-audit/ch0/README.md` から参照されるリンクも確認。
3. 上記変更を `docs-migrations.log` に記述し、`docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` §3 の状況説明を更新（例: 「2026-W05 時点で pipeline snapshot 未整備」など）。

### 4. Iter/Collector 実装と CI テスト補完
1. `compiler/rust/runtime/src/prelude/iter/mod.rs` に `impl<T> FromIterator<T> for Iter<T>`、`impl<T> IntoIterator for Iter<T>`、および `Iter::collect_list`/`collect_vec`/`try_collect` 終端操作を実装し、`collectors` モジュールと `EffectSet`/`StageProfile` 連携を確立する。
2. 新規 `compiler/rust/frontend/tests/core_iter_pipeline.rs` を作成し、`Iter::from_list |> Iter::map |> Iter::collect_list` など 6 シナリオを `insta` で固定。`core_iter_effects.rs`（効果タグ検証）も同時に用意し、`RUSTFLAGS="-Zpanic-abort-tests"` で実行する。
3. CI 設定（`.github/workflows/` または `tooling/ci/record-metrics.sh`）に `cargo test --manifest-path compiler/rust/frontend/Cargo.toml core_iter_pipeline core_iter_effects` を追加し、`scripts/validate-diagnostic-json.sh --pattern iterator --pattern collector` を同じジョブで走らせる。
4. テスト追加後、`collect-iterator-audit-metrics.py --section iterator --case pipeline` を実行して KPI を採取し、`reports/iterator-stage-summary.md`・`reports/spec-audit/ch1/iter.json` に追記。`docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` の完了条件チェックを更新する。

## 成果物
- 更新済み `cargo xtask prelude-audit` ソースと CLI ドキュメント
- 最新 `prelude_api_inventory.toml`・`reports/spec-audit/ch1/iter.json`・`reports/spec-audit/ch0/links.md`・`reports/iterator-{stage,collector}-summary.md`
- 補正文書: `docs/notes/core-library-outline.md`, `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md`, `docs-migrations.log`
- 新規/更新テスト: `core_iter_pipeline.rs`, `core_iter_effects.rs`, CI 設定

## フォローアップ
- 実装完了後に `3-1-core-prelude-iteration-plan.md` §3 の「完了条件」を実データで再検証し、Step4 へ進む判断を Phase 3-0 M1 ミーティングで下す。
- KPI の継続監視は `tooling/ci/collect-iterator-audit-metrics.py --require-success` を Nightly に組み込み、逸脱時は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` にタスクを登録する運用へ移行する。
