# 3.1 Core Prelude & Iteration 実装計画

## 目的
- 標準仕様 [3-1-core-prelude-iteration.md](../../spec/3-1-core-prelude-iteration.md) に準拠した `Core.Prelude` / `Core.Iter` モジュール群を Reml 実装へ落とし込み、章内 API の完全性と効果タグ精度を確保する。
- Option/Result/Iter を中心とした失敗制御モデルを安定化し、Chapter 3 の他モジュール (Collections/Text/Numeric) と同一インターフェイスで連携できる状態へ引き上げる。
- 仕様と実装・ドキュメントの差分を可視化し、Phase 3 以降のセルフホスト工程で再利用できるベンチマークとテスト資産を準備する。

## スコープ
- **含む**: `Option`/`Result`/`Never`/`Iter` の型・演算、`Collector` 契約、`Iter` アダプタ/終端操作、効果タグの検証、章内サンプルコードの実装検証、仕様リンクの更新。
- **含まない**: DSL / プラグイン固有拡張、1.3 章の効果システムそのものの仕様変更、未来の並列イテレータ拡張案（Phase 4 以降）。
- **前提**: Phase 2 で確定した診断/効果仕様が `Core.Diagnostics` 側に実装されており、Option/Result/Iter を利用する既存コードの回帰テストが実行可能であること。

## 作業ブレークダウン

### 1. 仕様精査と API インベントリ化（35週目）
**担当領域**: 設計調整

1.1. [3-1-core-prelude-iteration.md](../../spec/3-1-core-prelude-iteration.md) の API 一覧を機械可読な表に整理し、既存実装との差分 (新規/変更/廃止) を抽出する。
1.2. 効果タグ・属性 (`@must_use`, `effect {debug}` 等) の整合表を作成し、Phase 2 の Diagnostic 実装で要求されるメタデータ列と突き合わせる。
1.3. Option/Result の内部実装スタイル (enum vs struct, インライン最適化) を評価し、性能/サイズベンチマークの計測指標を確定する。

### 2. Option/Result 系 API 実装（35-36週目）
**担当領域**: 失敗制御プリミティブ

**成果物と出口条件**
- `compiler/rust/runtime`（Prelude 用に新設する crate もしくは既存 crate の `core_prelude` モジュール）に `Option`/`Result`/`Never`/`Try` 相当の型とメソッド群を実装し、`compiler/rust/frontend/tests` へ追加するユニットテストで [docs/spec/3-1-core-prelude-iteration.md](../../spec/3-1-core-prelude-iteration.md) の挙動一覧を全件再現する。
- `@must_use`、`effect {debug}` などの効果タグが [docs/spec/1-3-effects-safety.md](../../spec/1-3-effects-safety.md) の要件通りに宣言されていることを `scripts/validate-diagnostic-json.sh`・`tooling/ci/collect-iterator-audit-metrics.py` の静的検証で確認し、タグ漏れ 0 件を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に記録する。
- panic 禁止テストと診断ハンドオフ結果を `reports/diagnostic-format-regression.md` へ差分なしで保存し、残課題がある場合は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ転記する。

**主な依存資料**
- 仕様・参照: `docs/spec/3-1-core-prelude-iteration.md`, `docs/spec/1-3-effects-safety.md`, `docs/spec/3-6-core-diagnostics-audit.md`
- ベンチ/リスク: `docs/plans/rust-migration/3-2-benchmark-baseline.md`, `docs/plans/rust-migration/unified-porting-principles.md`, `docs/plans/bootstrap-roadmap/0-4-risk-handling.md`
- 既存資産: `compiler/ocaml/src` 以下の Prelude 代替実装や `compiler/ocaml/tests` の `Result`/`Option` 利用箇所（`test_type_inference.ml`, `test_cli_diagnostics.ml` 等）

2.1. `Option`/`Result`/`Never` 型と付随メソッド (`map`/`and_then`/`expect` など) を Reml で実装し、`@must_use` と効果タグを正しく付与する。
- Rust 側では `#[must_use]` を型とメソッド戻り値に付与し、`expect`/`unwrap` 系は `effect {debug}` を `cfg(debug_assertions)` でラップする。`Never`（ZST）は `enum Never {}` で導入して `match` 展開を用いた発散伝播を確実にする。
- 仕様の表形式を `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml`（新規）へ機械可読に転記し、`.cargo/config.toml` で `cargo xtask prelude-audit` を定義して API 抜け漏れを自動検出する。`--wbs 2.1b --strict` を既定フィルタとし、実行結果は `reports/spec-audit/ch0/links.md` にも記録する。
- `compiler/rust/frontend/tests/core_prelude_option_result.rs` / `.snap` を作成し、`Some/None` × `Ok/Err` の 16 シナリオを snapshot で固定。旧 OCaml 実装（`compiler/ocaml/tests/test_type_inference.ml`）の挙動差分は `docs/notes/core-library-outline.md` に追記する。

2.2. `ensure`/`ensure_not_null` 等のユーティリティを組み込み、診断 (`Diagnostic`) への変換ヘルパと一緒に単体テストを整備する。
- `ensure` は `Result<(), E>` を返す軽量 API として設計し、`Diagnostic` への `From`/`Into` 実装を同時に用意。`ensure_not_null` は `Option<T>` を即時 `Result<T, Diagnostic>` へ昇格させ、`docs/spec/3-6-core-diagnostics-audit.md` の `core.prelude.ensure_failed` キーを診断テーブルへ追加する。
- `scripts/validate-diagnostic-json.sh` を Option/Result テストに組み込み、`compiler/rust/frontend/tests/diagnostics` の JSON と `reports/diagnostic-format-regression.md` を比較して差分をゼロ化。`tooling/ci/collect-iterator-audit-metrics.py` のメトリクスへ ensure 発火件数を追加する。
- `examples/language-impl-comparison/` に `ensure` を利用した DSL サンプルを追加（別タスクでコード化）し、`docs/spec/3-0-core-library-overview.md`・`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` へ当該ユーティリティの有効化を脚注として記録する。

2.3. 例外排除ポリシーを検証するため、Rust 実装で `panic`/`abort` を伴う経路を禁止するテストを作成し、期待差分を `0-3-audit-and-metrics.md` へ記録する。必要に応じて OCaml 実装の挙動を参考情報として添付するが、自動比較対象には含めない。
- panic を許容するのは `effect {debug}` のみとし、CI で `cargo test --release -Z panic-abort-tests`（または `RUSTFLAGS="-C panic=abort"`）を追加。失敗時は `docs/plans/bootstrap-roadmap/4-5-backward-compat-checklist.md` に回帰として残す。
- `compiler/rust/frontend/tests/panic_forbidden.rs`（`trybuild`/`ui` テスト想定）を追加し、`panic!` や `unwrap_unchecked` を利用した場合に `#[deny(panic_fmt)]` でコンパイルエラーを発生させる。例外的に `expect` を許容するルールはテストで明文化する。
- 計測ログ（panic 経路検出・禁止件数）は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に KPI として追記し、集計コマンドと実行日時を `reports/spec-audit/ch0/links.md` フォーマットで参照可能にする。

#### 2. Option/Result 実施スケジュールと責務

| WBS | サブタスク | 入力/依存 | 成果物 | 検証/ログ | 担当 | 期限 |
| --- | --- | --- | --- | --- | --- | --- |
| ✅ 2.1a | `compiler/rust/runtime` に Prelude 揮発モジュールを新設し、`Cargo.toml` へ feature `core_prelude` を追加 | `docs/spec/3-1-core-prelude-iteration.md`, `compiler/rust/frontend/src/lib.rs` | `core_prelude` module skeleton、`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の雛形 | `cargo check`, `docs-migrations.log` へ追記 | Rust Impl チーム | 35週目末 |
| ✅ 2.1b | `Option`/`Result`/`Never` API を実装し、16 シナリオ snapshot と `cargo xtask prelude-audit` プロトタイプを追加 | `docs/spec/3-1-core-prelude-iteration.md:21-120`, `compiler/ocaml/tests/test_type_inference.ml` | `core_prelude_option_result.rs/.snap`, `xtask/src/main.rs` | `cargo test core_prelude_option_result`, `cargo xtask prelude-audit --wbs 2.1b --strict` | Core Library チーム | 36週目前半 |
| ✅ 2.2a | `ensure`/`ensure_not_null` を実装し、`Diagnostic` 変換と `core.prelude.ensure_failed` キーを登録 | `docs/spec/3-6-core-diagnostics-audit.md:210-260`, `tooling/ci/collect-iterator-audit-metrics.py` | `compiler/rust/runtime/src/prelude/ensure.rs`, `docs/spec/3-6-core-diagnostics-audit.md` 脚注更新案 | `scripts/validate-diagnostic-json.sh`, `reports/diagnostic-format-regression.md` 比較 | Diagnostics チーム | 36週目前半 |
| ✅ 2.2b | `examples/language-impl-comparison/` の DSL サンプルと `docs/spec/3-0-core-library-overview.md` の脚注を更新 | `docs/spec/3-0-core-library-overview.md`, `examples/language-impl-comparison/` | `reml/prelude_guard_template.reml`、脚注リンク、`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` の更新案 | `render_preview` の Result 伝播ログ、`core.prelude.ensure_failed` メタデータの検証メモ | Docs チーム | 36週目後半 |
| ✅ 2.3a | panic 禁止 UI テストと lint 設定（`panic_fmt` 欠如のため `non-fmt-panics`＋静的検証に置換）を実装 | `docs/spec/1-3-effects-safety.md`, `docs/plans/rust-migration/unified-porting-principles.md` | `compiler/rust/frontend/tests/panic_forbidden.rs`, `.cargo/config.toml` lint 更新 | `cargo test panic_forbidden`, `RUSTFLAGS="-Dnon-fmt-panics"` | Core Library チーム | 36週目後半 |
| ✅ 2.3b | `0-3-audit-and-metrics.md`/`4-5-backward-compat-checklist.md` への KPI・回帰項目追加 | `reports/diagnostic-format-regression.md`, `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` | KPI 記録・回帰トラッキングチケット | KPI CSV 更新、監査ログ照合作業メモ | QA/PM | 36週目末 |

> 2.3a 補足: Rust 安定版には `panic_fmt` lint が存在しないため、`.cargo/config.toml` に `-Dnon-fmt-panics` を設定したうえで `compiler/rust/frontend/tests/panic_forbidden.rs` による静的検証で `panic!`/`unwrap_unchecked` を監視し、`effect {debug}` 以外の経路が導入された場合にはテストで検出する。

> 実装補足: `compiler/rust/frontend/tests/core_prelude_option_result.{rs,snap}` で 16 シナリオ snapshot を維持し、`cargo xtask prelude-audit --wbs 2.1b --strict --baseline docs/spec/3-1-core-prelude-iteration.md` の出力を `reports/spec-audit/ch0/links.md` に貼り付ける運用とする。`prelude_api_inventory.toml` には `wbs` フィールドを追加しており、`2.2a` 以降の未実装項目は `--wbs` フィルタにより `strict` 判定から除外できる。

##### WBS 2.2a 実装方針（`ensure` 系ユーティリティ）
- **API と変換規約**: `compiler/rust/runtime/src/prelude/ensure.rs` に `ensure(cond: Bool, err: () -> E) -> Result<(), E>` と `ensure_not_null<T>(ptr: Option<T>, err: () -> E) -> Result<T, E>` をまとめ、`E: IntoDiagnostic` を必須化する。`EnsureGuard` のような軽量構造で `@must_use` を維持し、`Result<(), Diagnostic>` への昇格を `impl From<EnsureError> for Diagnostic` で吸収する。panic 禁止方針に合わせ、`effect {debug}` を伴う `expect` 系とは別レイヤで `?` オペレーターに接続する。
- **診断キーとメタデータ**: `core.prelude.ensure_failed` を `docs/spec/3-6-core-diagnostics-audit.md` に追加し、`Diagnostic.domain = Runtime`・Severity=Error を既定とする。`Diagnostic.extensions["prelude.guard"]` と `AuditEnvelope.metadata["core.prelude.guard.*"]` に `kind`（`ensure`/`ensure_not_null`）、`trigger`（失敗した条件式や識別子）、`pointer_class`（`ffi`/`plugin`/`core` 等）、`stage`（Stage Requirement がある場合）を必須記録し、`scripts/validate-diagnostic-json.sh` で欠落を検知する。
- **メトリクスと CI 連動**: `tooling/ci/collect-iterator-audit-metrics.py` を拡張し、`core.prelude.ensure_failed` を読み取って `core_prelude.guard.failures`／`core_prelude.guard.ensure_not_null` といったカウンタを JSON に書き出す。Nightly CI では `--require-success --section prelude-guard` を追加し、結果リンクを `reports/spec-audit/ch0/links.md` に追記する。
- **ドキュメント／ログ更新**: 実装完了時に `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` の M1 脚注、`0-3-audit-and-metrics.md` の KPI、`docs-migrations.log` に `core.prelude.ensure_failed` 追加を記録し、仕様（3-1/3-6）やガイドから参照できるよう脚注を整備する。

#### 2. Option/Result 完了条件
- 仕様由来の API 一覧が `prelude_api_inventory.toml` と `cargo xtask prelude-audit` で自動検証され、CI の nightly run で欠落が 0 件である（結果を `0-3-audit-and-metrics.md` に記録）。
- `Option`/`Result`/`Never` 実装が `#[must_use]` と効果タグを満たし、`scripts/validate-diagnostic-json.sh` のチェックを通過する。
- `ensure` 系ユーティリティと panic 禁止テストが `reports/diagnostic-format-regression.md` に差分なしで反映され、想定された `effect {debug}` 以外の panic 経路が存在しないことを `cargo test --release -Z panic-abort-tests` で証明する。
- 進行ログ（`docs-migrations.log`, `reports/spec-audit/ch0/links.md`）に API 追加と検証フローを追記し、Phase 3-1 の他タスクから参照できる状態にする。

### 3. Iter コア構造と Collectors（36-37週目）
**担当領域**: 遅延列基盤

**成果物と出口条件**
- `compiler/rust/runtime/src/prelude/iter/mod.rs` に `IterState`/`IterSeed`/`IterSource` の 3 層構造、`EffectLabels`/`IteratorStageProfile` の計測インターフェイスを実装し、`Iter<T>` が `IntoIterator`/`FromIterator` と双方向に連携できる。
- `Collector<T, C>` トレイトおよび `List/Vec/Map/Set/String` の標準コレクタを `compiler/rust/runtime/src/prelude/collectors/` 以下に実装し、`tooling/ci/collect-iterator-audit-metrics.py --section collectors` が `effect {mut,mem}` と Stage 要件を取得できる。
- `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` を Iter/Collector 項目まで更新し、`cargo xtask prelude-audit --section iter --strict` の結果を `reports/spec-audit/ch1/iter.json`、`0-3-audit-and-metrics.md` の KPI に反映する。
- `docs/notes/core-library-outline.md`、`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md`、`docs-migrations.log` に作業ログと参照リンクを記録し、Phase 3 の他計画から追跡できる状態にする。
- 2026-02-21 更新: `collect-iterator-audit-metrics.py --section iterator --case pipeline` の実行結果を `reports/iterator-stage-summary.md` / `reports/iterator-stage-metrics.json` に再採取し、`core_iter_pipeline.rs` の 6 ケース + `core_iter_effects.rs` の 3 ケースを `reports/spec-audit/ch1/iter.json` に差し戻した。`IterState`/`IterSeed`/`Collector` の 3 層構造と `EffectLabels` の計測が snapshot・KPI・`prelude_api_inventory.toml` で整合することを確認済み。`docs-migrations.log` へ WBS 3.1c-Iter Core の完了記録を追加した。

#### 3.a Iter コア土台（W36 前半）
- Phase 2 で確定した `IterState`/`IterStep`/`EffectSet` の設計を `compiler/rust/runtime/src/prelude/iter/mod.rs` に移植し、`EffectLabels` が `collect-iterator-audit` と互換な JSON を生成することを `docs/plans/bootstrap-roadmap/3-1-iter-collector-remediation.md` §4 と同期する。
- `IterSeed` を中心に `Iter::from_iter`・`IntoIterator for Iter<T>`・`Iter::try_collect` を連結し、`Try`/`Result` 経由のエラー伝搬を `docs/spec/1-2-types-Inference.md` の型拘束と一致させる。
- `compiler/rust/frontend/tests/core_iter_pipeline.rs` と `core_iter_effects.rs` を新設し、`Iter::from_list |> Iter::map |> Iter.collect_list`、`Iter::from_result |> Iter::try_collect`、`Iter::buffered` の効果タグを `insta` snapshot で固定。`scripts/validate-diagnostic-json.sh --pattern iterator` をテストと同じジョブで実行する。
- 生成した snapshot / JSON / CLI ログを `reports/spec-audit/ch0/links.md#iter-f3` に追記し、`collect-iterator-audit-metrics.py --section iterator --case pipeline` のコマンド列を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI セクションへ引用する。
- `docs/plans/bootstrap-roadmap/3-1-iter-collector-remediation.md` で定義した Remediation Step3 の「Iter/Collector 実装」チェックリストを転記し、後工程（Collector/Adapter）で参照できるようアップデートする。
- 2026-02-18 更新: `IterDriver::stepper` が `EffectSet` を受け取るよう拡張し、`AdapterPlan`（`compiler/rust/runtime/src/prelude/iter/adapters/{mod,map,filter}.rs`）を導入。これにより `EffectLabels::predicate_calls` や `effect {pending}` を実行時に加算できるようになり、`core_iter_pipeline.rs` と `core_iter_adapters.rs` の snapshot（`tests/snapshots/core_iter_pipeline__core_iter_pipeline.snap` / `core_iter_adapters__core_iter_adapters.snap`）で Stage/Effect の結果を固定済み。

#### 3.b Collector 実装ロードマップ（W36 後半〜W37）
`docs/plans/bootstrap-roadmap/3-1-iter-collector-remediation.md` の手順 1〜3 を本節に織り込み、Collector 実装と監査ログを同時進行で整備する。WBS/F タスクは次表の通り。

| 手順 | 目的 | 主なファイル / コマンド | 成果物・検証 |
| --- | --- | --- | --- |
| F2-1 List/Vec 雛形 | 永続構造（List）と可変構造（Vec）の Collector テンプレートを実装し、`effect {mem}` の記録方法を確立 | `compiler/rust/runtime/src/prelude/collectors/{list,vec}.rs`, `compiler/rust/frontend/tests/core_iter_collectors.rs::collect_list_baseline` | `python3 tooling/ci/collect-iterator-audit-metrics.py --section collectors --case list-vec` が `collector.effect.mem = 0/collector.effect.mem_reservation > 0` を報告、診断 JSON を `reports/spec-audit/ch1/core_iter_collectors.json` に保存 |
| F2-2 Map/Set Stage 宣言 | 重複キー検知と Stage 要件 (`Exact("stable")` など) を `CollectError`/`Diagnostic` に伝播 | `collectors/{map,set}.rs`, `core_iter_collectors.rs::collect_map_duplicate`, `tooling/ci/collect-iterator-audit-metrics.py --case collector-duplicate` | `AuditEnvelope.metadata.collector.error.key` に重複キーが記録され、`iterator.stage.audit_pass_rate = 1.0` を維持 |
| F2-3 String/UTF-8 | UTF-8 正規化付き `StringCollector` と `StringError::InvalidEncoding` を実装し、`effect {mem}` と診断変換を実証 | `collectors/string.rs`, `docs/spec/3-3-core-text-unicode.md`, `core_iter_collectors.rs::collect_string_invalid` | `collect-iterator-audit` で `collector.error.invalid_encoding = 1` が出力され、`reports/iterator-collector-summary.md` に KPI を転記 |
| F2-4 API インベントリ更新 | Collector API の在庫・WBS・効果タグを機械管理 | `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml`, `cargo xtask prelude-audit --section iter --filter collector --strict` | `rust_status` を `planned→implemented` に更新し、`last_updated = "Phase3-W37 / F2"`、`reports/spec-audit/ch1/iter.json` を再生成 |
| F2-5 監査ログ整備 | KPI とコマンド履歴を公開しクロスリファレンスを確立 | `reports/spec-audit/ch0/links.md`, `docs/notes/core-library-outline.md`, `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` | `Collector F2` セクションを追加し、実行コマンド・スナップショット・ベンチ結果を相互参照。`docs-migrations.log` に更新履歴を残す |

###### 3.c 監査ログ・KPI 更新
- `python3 tooling/ci/collect-iterator-audit-metrics.py --section collectors --source reports/spec-audit/ch1/core_iter_collectors.json --audit-source reports/spec-audit/ch1/core_iter_collectors.audit.jsonl --output reports/iterator-collector-metrics.json --require-success` を週次で実行し、`collector.stage.audit_pass_rate`、`collector.effect.mem`、`collector.error.duplicate_key` を `0-3-audit-and-metrics.md` の KPI テーブルに転記する。
- `scripts/validate-diagnostic-json.sh --pattern collector --pattern iterator` の出力を `reports/spec-audit/ch0/links.md#collector-f2`／`#collector-f3` に貼り付け、診断フォーマットの差分を `reports/diagnostic-format-regression.md` へ共有する。
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ KPI 逸脱時のフォローアップチケットを登録し、`docs/notes/core-library-outline.md#collector-f2-監査ログ` に根拠ファイルを列挙する。
- `IterState`/`IterSeed` の 3 層構造と `IteratorStageProfile` を Rust 実装へ落とし込む際は、`compiler/rust/runtime/src/prelude/iter/mod.rs` に `EffectLabels::from_iter_step` を追加し、`collect-iterator-audit --section iter --case seed-stage-profile --output reports/spec-audit/ch1/iter_seed_profile.json` を実行して Stage/Effect の測定結果を `reports/spec-audit/ch0/links.md#iter-core-structure` に記録する。
- `cargo xtask prelude-audit --section iter --filter collector --strict --baseline docs/spec/3-1-core-prelude-iteration.md` を nightly ジョブへ組み込み、成功ログを `reports/spec-audit/ch0/links.md#collector-f2-監査ログ` に追記する。Run-ID・日付・担当者を `docs-migrations.log` に転記し、証跡を維持する。
- `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `module = "Iter"` / `"Collector"` エントリにテスト名・KPI・効果タグを追記し、`0-3-audit-and-metrics.md` から参照できるようにする。更新後は `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` §3.0.3a へリンクを追加して Phase 3 判定資料へ反映する。
- G2 以降のアダプタ実装へ接続するため、`flat_map`/`zip` の Stage 要件と `EffectLabels::mem_reservation`/`iterator.error.zip_shorter` を `reports/spec-audit/ch1/core_iter_adapters.json` に追加出力し、`docs/notes/core-library-outline.md#iter-g2-flat-zip` と `docs/plans/bootstrap-roadmap/3-1-iter-collector-remediation.md` へ更新ログを残す。Stage や effect の差異は `prelude_api_inventory.toml` の各行にも記録し、G2 手順開始時点のベースラインを確定させる。

### 4. Iter アダプタと終端操作（37-38週目）
**担当領域**: 宣言的データフロー

4.1. `map`/`filter`/`flat_map`/`zip`/`buffered` 等のアダプタを実装し、`effect {mem}` や `effect {mut}` の発生箇所を網羅的にテストする。
4.2. `collect_list`/`collect_vec`/`fold`/`reduce`/`try_fold` など終端操作の実装を行い、`Collector` との連携とエラー伝播経路を検証する。
4.3. パフォーマンス計測ベンチマークを作成し、Rust 実装の Phase 2 ベースライン（`docs/plans/rust-migration/3-2-benchmark-baseline.md`）と比較して ±10% 以内に収束するかを測定し、`0-3-audit-and-metrics.md` に反映する。

#### 4.a アダプタ実装ロードマップ（W37 後半）
- `compiler/rust/runtime/src/prelude/iter/adapters/` を新設し、各アダプタの `AdapterPlan`（効果・Stage・依存 Collector）をコメントで明記。`Iter<T>` (`iter/mod.rs`) の `impl` には `#[must_use]` と `EffectLabels` 連携を追加し、`collect-iterator-audit-metrics.py` が `iterator.effect.*` を収集できるようにする。
- `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `Iter` セクションへ `adapter` カテゴリを追加し、`cargo xtask prelude-audit --section iter --filter adapter --strict` でステータス遷移を管理する。CLI 出力は `reports/spec-audit/ch1/core_iter_adapters.json` に保存する。
- `compiler/rust/frontend/tests/core_iter_adapters.rs` を新設し、`map`/`filter`/`flat_map`/`zip`/`buffered` の 5 ケースを `insta` snapshot 化。`reports/spec-audit/ch0/links.md#iter-adapters` に実行コマンドを残して `docs/plans/bootstrap-roadmap/3-1-iter-collector-remediation.md` から参照できるようにする。

| アダプタ | 効果タグ | Stage 要件 | 主なファイル / コマンド | KPI / 検証 |
| --- | --- | --- | --- | --- |
| `map` | `@pure` | `Stage::Stable` | `iter/adapters/map.rs`, `core_iter_adapters.rs::map_pipeline` | `iterator.effect.residual = ∅` を `collect-iterator-audit --section iterator --case map` で確認 |
| `filter` | `effect {mut}` (`EffectLabels::predicate_calls`) | `Stage::Stable` | `iter/adapters/filter.rs`, `core_iter_effects.rs::filter_effect` | `iterator.effect.mut = predicate_count` を KPI に記録 |
| `flat_map` | `effect {mem}`（中間バッファ） | `Stage::Beta` | `iter/adapters/flat_map.rs`, `core_iter_adapters.rs::flat_map_vec` | `iterator.effect.mem_reservation` を `reports/iterator-stage-summary.md` に反映 |
| `zip` | `@pure` / `effect {mut}`（長さ調整） | `Stage::Stable` | `iter/adapters/zip.rs`, `core_iter_adapters.rs::zip_mismatch` | `collect-iterator-audit` で `iterator.error.zip_shorter` が 0 件 |
| `buffered` | `effect {mem}`（リングバッファ） | `Stage::Experimental` | `iter/adapters/buffered.rs`, `core_iter_adapters.rs::buffered_window` | `iterator.effect.mem_bytes` を `0-3-audit-and-metrics.md` の `iterator.mem.window` に反映 |

###### G1: map/filter 立ち上げ（W37 後半）
- `IterState::adapter` で `map`/`filter` がチェーンできるよう `FnMut`/`Predicate` のトレイト束縛を導入し、`EffectLabels::predicate_calls` を `collect-iterator-audit` へ露出させる。
- `core_iter_adapters.rs::map_pipeline` と `core_iter_effects.rs::filter_effect` を追加し、`scripts/validate-diagnostic-json.sh --pattern iterator.map --pattern iterator.filter` の出力を `reports/diagnostic-format-regression.md` へ連携。KPI は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の `iterator.map.latency` / `iterator.filter.predicate_count` 欄に記録する。
- `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `Iter.map`/`Iter.filter` 行へ `rust_status = "working"` とテスト名を追記し、完了時に `implemented` へ遷移させる。

###### G1 実施タスクリストと成果物
- `compiler/rust/runtime/src/prelude/iter/adapters/map.rs` / `filter.rs` を追加し、`IterState::adapter` から `AdapterPlan` を経由して `IteratorStageProfile` を更新する。`EffectLabels::predicate_calls` を `collect-iterator-audit-metrics.py --section iterator --case map|filter --output reports/iterator-map-filter-metrics.json` で採取し、`reports/spec-audit/ch0/links.md#iter-g1-map-filter` へリンクする。
- `compiler/rust/frontend/tests/core_iter_adapters.rs` に `map_pipeline` / `filter_effect` / `map_filter_chain_panic_guard` を追加し、`cargo test core_iter_adapters -- --nocapture` と `cargo insta review` のログを `reports/spec-audit/ch1/core_iter_adapters.json` へ保存する。同ジョブで `scripts/validate-diagnostic-json.sh --pattern iterator.map --pattern iterator.filter` を実行し、`reports/diagnostic-format-regression.md` に差分なしで反映する。
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `iterator.map.latency`（`Iter::from_list |> map |> collect_vec`）と `iterator.filter.predicate_count`（`filter` の `predicate_calls` 期待値）を KPI として追記し、Nightly 実行の集計結果を貼り付ける。逸脱が発生した場合は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へフォローアップを記載する。
- `docs/plans/bootstrap-roadmap/3-1-iter-collector-remediation.md` §4 と `docs/notes/core-library-outline.md#iter-g1-map-filter` に G1 手順・スナップショットファイル・コマンド履歴をまとめ、Phase 3-2 の Adapter 拡張作業から参照できるようにする。
- `prelude_api_inventory.toml` の `Iter.map`/`Iter.filter` へ KPI・テスト名・効果タグの記録方法を追加し、更新内容を `docs-migrations.log` に「WBS 3.1c-G1 map/filter 立ち上げ」として残して監査証跡を確保する。
- 2026-02-18 更新: 上記タスクリストは `compiler/rust/runtime/src/prelude/iter/adapters/{map,filter}.rs`・`compiler/rust/frontend/tests/snapshots/core_iter_adapters__core_iter_adapters.snap`・`reports/iterator-map-filter-metrics.json` で完了済み。`iterator.filter.predicate_count` は `filter_effect` ケースで 4 を記録し、Stage は `Exact(\"stable\")` へ移行した。

###### G2: flat_map / zip Stage 適用（W38 前半）
- `flat_map` 用にネストした `IterSeed` を `iter/adapters/flat_map.rs` へ実装し、中間バッファの確保時に `EffectLabels::mem_reservation` を積算する。`core_iter_adapters.rs::flat_map_vec` で `iterator.effect.mem` スナップショットを取得する。
- `zip` は入力の長さ差を `iterator.error.zip_shorter` で表現し、`EffectLabels::stage.required = Stage::Stable` を `collect-iterator-audit --case zip` の JSON に含める。`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` には長さ不一致時のフォローアップルールを記述する。
- `reports/spec-audit/ch0/links.md#iter-adapters` に `cargo test core_iter_adapters -- --include-ignored zip_mismatch` などのコマンド列を追加し、`reports/iterator-stage-summary.md` の `iterator.effect.mem_reservation` を更新する。
- **G2 実施タスクリスト**
  1. `compiler/rust/runtime/src/prelude/iter/adapters/flat_map.rs` を再編し、`IterSeed::FlatMap` が `IteratorStageProfile::request_stage(Stage::Beta)` と `EffectLabels::mem_reservation` を更新するフックを実装する。`collect-iterator-audit-metrics.py --section iterator --case flat_map --output reports/iterator-flatmap-metrics.json` の結果を `reports/spec-audit/ch0/links.md#iter-adapters` にリンクし、`docs-migrations.log` へジョブ ID を追記する。
  2. `iter/adapters/zip.rs` に長さ差検知用の `ZipState::Remaining` を追加し、短い入力が検出されたら `iterator.error.zip_shorter` を `Diagnostic.extensions` に書き出す。`core_iter_adapters.rs::zip_mismatch` の snapshot を更新し、`scripts/validate-diagnostic-json.sh --pattern iterator.zip` の結果を `reports/diagnostic-format-regression.md` に反映する。
  3. `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` で `Iter.flat_map` の `effect` を `effect {mem}`、`Iter.zip` の `effect` を `@pure / effect {mut}` の複合に書き換え、`stage` と `rust_status` を `implemented` に保ちつつ `notes` 欄へ `flat_map_vec`/`zip_mismatch` テスト参照と KPI ファイル名を追記する。更新履歴を `reports/spec-audit/ch1/core_iter_adapters.json` と `reports/spec-audit/ch0/links.md#iter-adapters` に反映。
  4. `docs/notes/core-library-outline.md#iter-g2-flat-zip` と `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md#iter-adapter` に Stage 設計・KPI 連携の要点を記述し、Phase 3 内での参照先を一本化する。
- 2026-02-21 実施結果: 上記 4 項目は `cargo test --manifest-path compiler/rust/frontend/Cargo.toml core_iter_adapters -- --include-ignored flat_map_vec zip_mismatch` → `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case flat_map --case zip --output reports/iterator-flatmap-metrics.json --secondary-output reports/iterator-zip-metrics.json --require-success` の連続実行で完了した。`flat_map` の `EffectLabels::mem_reservation` は `reports/iterator-flatmap-metrics.json` で `bytes_reserved = 3` を計測し、`zip` の長さ差検出は `reports/iterator-zip-metrics.json` と `reports/diagnostic-format-regression.md#iterator.zip_mismatch` で `iterator.error.zip_shorter = 1` を検証。`prelude_api_inventory.toml` の `Iter.flat_map` / `Iter.zip` 行と `reports/spec-audit/ch1/core_iter_adapters.json`、`reports/spec-audit/ch0/links.md#iter-adapters`、`docs-migrations.log` へ同日付の更新履歴を追記し、`docs/notes/core-library-outline.md#iter-g2-flat-zip`・`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md#iter-adapter` の Stage/KPI 要約を同期済み。

| 検証項目 | 成果物 | 依存ファイル/コマンド |
| --- | --- | --- |
| `flat_map` の `effect {mem}` 計測 | `reports/iterator-flatmap-metrics.json`, `tests/snapshots/core_iter_adapters__core_iter_adapters.snap` | `collect-iterator-audit-metrics.py --section iterator --case flat_map`, `cargo test core_iter_adapters -- --include-ignored flat_map_vec` |
| `zip` 長さ差検出 | `tests/snapshots/core_iter_adapters__core_iter_adapters.snap`, `reports/diagnostic-format-regression.md#iterator.zip_mismatch` | `cargo test core_iter_adapters -- --include-ignored zip_mismatch`, `scripts/validate-diagnostic-json.sh --pattern iterator.zip` |
| Stage/KPI 反映 | `prelude_api_inventory.toml`, `reports/spec-audit/ch1/core_iter_adapters.json`, `docs-migrations.log` | `cargo xtask prelude-audit --section iter --filter adapter --strict`, `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case zip` |

###### G3: buffered と backpressure（W38 中盤）
- `iter/adapters/buffered.rs` にリングバッファ実装を追加し、`EffectLabels::mem_bytes` を `IteratorStageProfile` で記録する。`core_iter_adapters.rs::buffered_window` で空間効率とバックプレッシャを可視化する。
- `docs/plans/rust-migration/3-2-benchmark-baseline.md` に倣い、`cargo bench -p compiler-rust-frontend iter_buffered` を走らせて ±10% の性能目標を検証。結果値は `0-3-audit-and-metrics.md` の `iterator.mem.window` 欄へ転記する。
- `collect-iterator-audit-metrics.py --section iterator --case buffered --output reports/iterator-buffered-metrics.json --require-success` を Nightly ジョブへ組み込み、逸脱時は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ記録する。
- 2027-02-22 実施結果: `cargo test --manifest-path compiler/rust/frontend/Cargo.toml core_iter_adapters -- --include-ignored buffered_window` → `cargo bench -p compiler-rust-frontend iter_buffered -- warmup-time 3 --measurement-time 10` → `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case buffered --output reports/iterator-buffered-metrics.json --require-success` の順で実行。`buffered_window` snapshot は `EffectLabels.mem_bytes = 2` / `IteratorStageProfile.required = Exact("experimental")` を保持し、Criterion ベンチ結果（`reports/benchmarks/iter_buffered-2027-02-22.json`, delta = +3.8%）が ±10% 目標内であることを確認した。`iterator.mem.window.bytes = 2` / `iterator.mem.window.backpressure = 0.33` を `reports/iterator-buffered-metrics.json` に記録し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`・`docs/notes/core-library-outline.md#iter-g3-buffered-backpressure`・`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md#iter-adapter`・`reports/spec-audit/ch0/links.md#iter-buffered`・`docs-migrations.log` へ Run-ID `2027-02-22-iter-adapter-g3` を反映した。

###### G4: Adapter KPI と文書同期（W38 後半）
- `cargo xtask prelude-audit --section iter --filter adapter --strict` の結果を `reports/spec-audit/ch1/core_iter_adapters.json` に保存し、`docs/migrations.log` と `docs/notes/core-library-outline.md#iter-adapter` にリンクを追加する。
- `reports/spec-audit/ch0/links.md` に adapter 向けの CLI 手順と snapshot ファイルへのリンクを追記し、`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md#iter-adapter` から参照できるようにする。
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ `iterator.adapter.coverage` KPI を新設し、`collect-iterator-audit` と `scripts/validate-diagnostic-json.sh` の両結果を掲載する。
- 2027-02-24 実施結果: `cargo xtask prelude-audit --section iter --filter adapter --strict --output reports/spec-audit/ch1/core_iter_adapters.json` → `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case adapters --source reports/spec-audit/ch1/core_iter_adapters.json --output reports/iterator-adapter-metrics.json --require-success` → `scripts/validate-diagnostic-json.sh --pattern iterator.map --pattern iterator.zip reports/spec-audit/ch1/core_iter_adapters.json` を実行し、`run_id = 2027-02-24-iter-adapter-g4` を `reports/spec-audit/ch1/core_iter_adapters.json` / `reports/spec-audit/ch0/links.md#iter-adapters-g4` / `docs/notes/core-library-outline.md#iter-adapter` / `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md#iter-adapter` に記録。`iterator.adapter.coverage = 1.0`（adapter 12 件、欠落 0）と `diagnostic.audit_presence_rate = 1.0` を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 表へ転記し、`docs-migrations.log` に WBS 3.1c-G4 完了ログを残した。

#### 4.b 終端操作と Collector 連携（W38 前半）
- `Iter::collect_list`/`collect_vec`/`collect_string` と `Iter::fold`/`reduce`/`try_fold` を `compiler/rust/runtime/src/prelude/iter/terminators.rs`（新設）に集約し、`Collector` 実装を内部的に再利用する。`try_*` 系は `Result` を返し、`CollectError` を `Diagnostic` へ昇格させるヘルパを追加。
- `compiler/rust/frontend/tests/core_iter_terminators.rs` を追加し、Collector 経由と直接終端 API の整合を `insta` snapshot で記録。`reports/spec-audit/ch1/core_iter_terminators.json` を生成し、`reports/spec-audit/ch0/links.md#iter-terminators` にコマンドを追記。

| 終端操作 | 効果 | Collector 依存 | 主なテスト | 診断/KPI |
| --- | --- | --- | --- | --- |
| `collect_list` | `@pure` | `ListCollector` | `core_iter_terminators.rs::collect_list_pipeline` | `collector.stage.actual = "stable"` を確認 |
| `collect_vec` | `effect {mem}` | `VecCollector` | `core_iter_terminators.rs::collect_vec_reserve` | `iterator.effect.mem_reservation` を KPI 化 |
| `collect_string` | `effect {mem}`/`effect {text}` | `StringCollector` | `core_iter_terminators.rs::collect_string_invalid` | `collector.error.invalid_encoding` を `reports/iterator-collector-summary.md` に記録 |
| `fold` | `@pure` | なし | `core_iter_terminators.rs::fold_sum` | `iterator.effect.residual = ∅` を監査 |
| `reduce` | `@pure` | なし | `core_iter_terminators.rs::reduce_empty` | `IteratorReduceError::Empty` を `Diagnostic` に反映 |
| `try_fold` | `effect {mut}`（Acc 更新） | 任意 Collector | `core_iter_terminators.rs::try_fold_error` | `Result` 経路が `reports/diagnostic-format-regression.md` と一致 |

###### H1: collect_* の Collector 連携確認
- `Iter::collect_*` で `Collector` を内部的に生成する補助関数 (`fn collect_with<C: Collector>(...)`) を実装し、`CollectOutcome::audit()` を `Diagnostic.extensions["prelude.collector"]` へ転写。`collect_vec_reserve` で `collector.effect.mem_reservation` が期待通り出力されるか `scripts/validate-diagnostic-json.sh --pattern iterator.collect` で検証する。
- `docs/notes/core-library-outline.md#collector-f2-監査ログ` に `Iter::collect_*` 経由の監査手順を追記し、`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md#collector-f2-監査ログ` にもリンクを追加する。

###### H2: fold/reduce/try_fold の診断整備
- `fold`/`reduce` に `IteratorEmptyError` を導入し、`Diagnostic` の `effects.residual` を `∅` に固定。`reduce_empty` ケースで `core_iter_terminators.snap` に `iterator.error.reduce_empty` を記録。
- `try_fold` は `Result` ベースの早期終了を提供し、`effect {mut}` を `EffectLabels` に転写。`try_fold_error` テストで `Result::Err` がそのまま `CollectError` に昇格しないことを確認し、`reports/spec-audit/ch1/core_iter_terminators.json` に早期終了経路を残す。

#### 4.c ベンチマークと KPI 更新（W38 後半）
- `compiler/rust/benchmarks/core_iter_adapters.rs` を追加し、`map`/`filter`/`flat_map`/`buffered` + `collect_*` の 5 コンボを `criterion` で測定。`baseline = reports/benchmarks/phase2/core_iter_adapters.json` と比較し、±10% を越えた場合は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` の `iterator.performance` リスクを更新する。
- `tooling/ci/collect-iterator-audit-metrics.py --section iterator --case adapters --require-success --source reports/spec-audit/ch1/core_iter_adapters.json` を nightly へ登録し、`reports/iterator-stage-summary.md` に `iterator.effect.mem_bytes`, `iterator.effect.mut`, `iterator.error.*` を自動書き込み。`0-3-audit-and-metrics.md` の `iterator.stage.audit_pass_rate`/`iterator.perf.delta` を更新し、`docs-migrations.log` にベンチ計測の記録を残す。
- パフォーマンス比較結果を `docs/plans/rust-migration/3-2-benchmark-baseline.md#iter-adapters` へ脚注として追記し、Phase 3-2 の観測ラインと同期させる。

### 5. Diagnostics/Unicode 連携（38週目）
**担当領域**: 他章との統合

5.1. `Iter`/`Collector` が `Core.Text` の `GraphemeSeq` や `Core.Collections` の永続構造と相互運用できることを確認し、必要な補助関数を追加する。
5.2. Option/Result と `Diagnostic`/`AuditEnvelope` の相互変換ヘルパを整備し、失敗制御が監査ログに正しく反映されるか統合テストを実施する。
5.3. `effect` タグと `CapabilityStage` の境界を検証し、`effect {debug}` の利用箇所にデバッグビルド限定ステップを組み込む。

### 6. サンプルコード検証とドキュメント更新（38-39週目）
**担当領域**: 情報整備

6.1. 仕様書内サンプル (`reml` コードブロック) を Reml 実装で実行し、必要に応じて修正または `NOTE` 追記を行う。
6.2. `README.md` および `3-0-phase3-self-host.md` に Prelude/Iter 移行ステータスを追記し、利用者向けハイライトを作成する。
6.3. 新規 API の使用例を `examples/` ディレクトリに追加し、`docs/guides/core-parse-streaming.md` 等関連ガイドへのリンクを更新する。

### 7. テスト・ベンチマーク統合とリリース準備（39週目）
**担当領域**: 品質保証

7.1. 単体/統合テストを CI に追加し、`--features core-prelude` など機能ゲートを導入する。
7.2. ベンチマーク結果と API 完了状況を `0-3-audit-and-metrics.md`/`0-4-risk-handling.md` に記録し、リスク項目を更新する。
7.3. レビュー資料 (API 差分一覧、ベンチマーク、リリースノート草案) を準備し、Phase 3-2 以降へ引き継ぐ。

## 35週目: Step 1 実施結果

- 標準仕様の根拠は `docs/spec/3-1-core-prelude-iteration.md:21-197` で示された `Option`/`Result`/`Iter`/`Collector` API であり、インポート規則と効果タグまで明文化されている。
- OCaml 実装は `compiler/ocaml/src/constraint_solver.ml:371-477` で `Collector`/`Iterator` 辞書を自動生成し、`compiler/ocaml/tests/test_type_inference.ml:1799-1845` で Stage/Capability メタデータを診断へ転写するテストが存在するものの、Prelude/Iter の API 本体はまだ提供していない。
- Rust 実装側は `compiler/rust/frontend/src/lib.rs:1-32` の通り `diagnostic`/`parser` 等の骨格モジュールのみ公開しており、`Core.Prelude` や `Core.Iter` を含むモジュール階層が存在しない。
- 以下の表では `差分` を `新規`/`変更` の 2 値で記録し、`Rust` 列は `未実装` のみ、`OCaml` 列は `未実装`（完全に欠如）、`型推論のみ`（constraint solver で型名を扱うが API がない）、`診断メタデータ`（Stage 情報を diag に転写）を用いる。

### 1.1 API インベントリ（仕様 vs 実装差分）

#### Prelude (`Option`/`Result`/Guards)

| カテゴリ | API | 効果 | 差分 | Rust | OCaml |
| --- | --- | --- | --- | --- | --- |
| Option | `Option.is_some` | `@pure` | 新規 | 未実装 | 型推論のみ |
| Option | `Option.map` | `@pure` | 新規 | 未実装 | 型推論のみ |
| Option | `Option.and_then` | `@pure` | 新規 | 未実装 | 型推論のみ |
| Option | `Option.ok_or` | `@pure` | 新規 | 未実装 | 型推論のみ |
| Option | `Option.unwrap_or` | `@pure` | 新規 | 未実装 | 型推論のみ |
| Option | `Option.expect` | `effect {debug}` | 新規 | 未実装 | 型推論のみ |
| Result | `Result.map` | `@pure` | 新規 | 未実装 | 型推論のみ |
| Result | `Result.map_err` | `@pure` | 新規 | 未実装 | 型推論のみ |
| Result | `Result.and_then` | `@pure` | 新規 | 未実装 | 型推論のみ |
| Result | `Result.or_else` | `@pure` | 新規 | 未実装 | 型推論のみ |
| Result | `Result.unwrap_or` | `@pure` | 新規 | 未実装 | 型推論のみ |
| Result | `Result.expect` | `effect {debug}` | 新規 | 未実装 | 型推論のみ |
| Result | `Result.to_option` | `@pure` | 新規 | 未実装 | 型推論のみ |
| Result | `Result.from_option` | `@pure` | 新規 | 未実装 | 型推論のみ |
| Guard | `ensure` | `@pure` | 新規 | 未実装 | 未実装 |
| Guard | `ensure_not_null` | `@pure` | 新規 | 未実装 | 未実装 |

`Option`/`Result` の型名自体は OCaml の制約解決で参照されており（`compiler/ocaml/src/constraint_solver.ml:384-390`）、`Collector` 自動実装にも利用されているが、仕様で定義された API 群や `@must_use` 属性はまだコード化されていない。

#### Iter 生成・変換・終端・Collector

| カテゴリ | API | 効果 | 差分 | Rust | OCaml |
| --- | --- | --- | --- | --- | --- |
| 生成 | `Iter.empty` | `@pure` | 新規 | 未実装 | 未実装 |
| 生成 | `Iter.once` | `@pure` | 新規 | 未実装 | 未実装 |
| 生成 | `Iter.repeat` | `@pure` | 新規 | 未実装 | 未実装 |
| 生成 | `Iter.from_list` | `@pure` | 新規 | 未実装 | 型推論のみ |
| 生成 | `Iter.from_result` | `@pure` | 新規 | 未実装 | 型推論のみ |
| 生成 | `Iter.range` | `@pure` | 新規 | 未実装 | 未実装 |
| 生成 | `Iter.unfold` | `@pure` | 新規 | 未実装 | 未実装 |
| 生成 | `Iter.try_unfold` | `@pure` | 新規 | 未実装 | 未実装 |
| 変換 | `Iter.map` | `@pure` | 新規 | 未実装 | 型推論のみ |
| 変換 | `Iter.filter` | `@pure` | 新規 | 未実装 | 型推論のみ |
| 変換 | `Iter.filter_map` | `@pure` | 新規 | 未実装 | 型推論のみ |
| 変換 | `Iter.flat_map` | `@pure` | 新規 | 未実装 | 型推論のみ |
| 変換 | `Iter.scan` | `@pure` | 新規 | 未実装 | 未実装 |
| 変換 | `Iter.take` | `@pure` | 新規 | 未実装 | 未実装 |
| 変換 | `Iter.drop` | `@pure` | 新規 | 未実装 | 未実装 |
| 変換 | `Iter.enumerate` | `@pure` | 新規 | 未実装 | 未実装 |
| 変換 | `Iter.zip` | `@pure` | 新規 | 未実装 | 未実装 |
| 変換 | `Iter.buffered` | `effect {mem}` | 新規 | 未実装 | 未実装 |
| 終端 | `Iter.collect_list` | `@pure` | 新規 | 未実装 | 型推論のみ |
| 終端 | `Iter.collect_vec` | `effect {mut}` | 新規 | 未実装 | 型推論のみ |
| 終端 | `Iter.fold` | `@pure` | 新規 | 未実装 | 未実装 |
| 終端 | `Iter.reduce` | `@pure` | 新規 | 未実装 | 未実装 |
| 終端 | `Iter.all` | `@pure` | 新規 | 未実装 | 未実装 |
| 終端 | `Iter.any` | `@pure` | 新規 | 未実装 | 未実装 |
| 終端 | `Iter.find` | `@pure` | 新規 | 未実装 | 未実装 |
| 終端 | `Iter.try_fold` | `@pure` | 新規 | 未実装 | 型推論のみ |
| 終端 | `Iter.try_collect` | `@pure` | 新規 | 未実装 | 型推論のみ |
| Collector | `Collector.new` | `@pure` | 新規 | 未実装 | 型推論のみ |
| Collector | `Collector.with_capacity` | `effect {mem}` | 新規 | 未実装 | 未実装 |
| Collector | `Collector.push` | `effect {mut}` | 新規 | 未実装 | 型推論のみ |
| Collector | `Collector.reserve` | `effect {mut, mem}` | 新規 | 未実装 | 未実装 |
| Collector | `Collector.finish` | `effect {mem}` | 新規 | 未実装 | 未実装 |
| Collector | `Collector.into_inner` | `@pure` | 新規 | 未実装 | 未実装 |

OCaml の `solve_collector`/`solve_iterator` は `Iter`/`Collector`/`Option`/`Result` の型を辞書解決に利用しているため（`compiler/ocaml/src/constraint_solver.ml:371-477`）、Rust 側の実装でも同一トレイト名と Stage 要件を露出させる必要がある。Rust / OCaml いずれも `Iter.buffered` や `Collector.reserve` のような `effect {mem}` 系 API をまだ持っていないため、Phase 3 の実装タスクで初出となる。

### 1.2 効果タグ・属性と診断メタデータ整合

| タグ/属性 | 仕様出典 | 要求メタデータ（`Diagnostic`/`AuditEnvelope`） | Phase 2 実装状況 |
| --- | --- | --- | --- |
| `@must_use` (`Option`/`Result`) | `docs/spec/3-1-core-prelude-iteration.md:23-67`, `docs/spec/1-3-effects-safety.md:92-113` | `Diagnostic` 本体に `code`/`domain`/`audit` を必ず添付する（`docs/spec/3-6-core-diagnostics-audit.md:1-74`）。未使用検知は Lint ドメインとして `change_set` を伴い警告化する。 | 実装なし（OCaml/Rust 共通）。Lint ルール定義を Phase 3-2 で追加する必要あり。 |
| `@pure` 契約 | `docs/spec/3-1-core-prelude-iteration.md:19-75`, `docs/spec/1-3-effects-safety.md:70-125` | `Diagnostic.extensions["effects"]` に `before`/`handled`/`residual`/`stage` を出力し、`residual = ∅` を監査できるようにする（`docs/spec/3-6-core-diagnostics-audit.md:108-127`）。 | OCaml タイプエラー経路で `EffectsExtension` を生成済み（`compiler/ocaml/tests/test_type_inference.ml:1799-1845`）。Rust では未実装。 |
| `effect {mut}` / `{mem}` | `docs/spec/3-1-core-prelude-iteration.md:100-155` | `AuditEnvelope.metadata["effect.stage.required"]`/`["effect.stage.actual"]` に加えて `effects.residual` を同期し、`collect-iterator-audit-metrics.py` が Stage 不整合を検知できるようにする（`docs/spec/3-6-core-diagnostics-audit.md:335-357`）。 | OCaml の診断 JSON で `effect.stage.iterator.*` を出力済み（`compiler/ocaml/tests/test_type_inference.ml:1799-1845`）。Rust でのメタデータ転写は未実装。 |
| `effect {debug}` (`expect`) | `docs/spec/3-1-core-prelude-iteration.md:51-66` | Debug 用 API の診断は `effects.stage = Experimental` で発行し、`AuditEnvelope.metadata["effect.stage.required"] = "debug"` を記録する（`docs/spec/3-6-core-diagnostics-audit.md:335-346`）。 | 仕様のみ。フェールファスト系 API がまだ存在しないため、Phase 3 実装で `--deny-debug-effects` を追加予定。 |
| `Collector`/`Iterator` Stage | `docs/spec/3-1-core-prelude-iteration.md:151-197`, `docs/spec/3-6-core-diagnostics-audit.md:335-357` | `effect.stage.iterator.*` と `typeclass.*` の両方に Stage/Capability/Kind/Source を書き出す（`docs/spec/3-6-core-diagnostics-audit.md:349-372`）。 | OCaml: `Type_error.trait_constraint_stage_extension` で `iterator_kind`/`capability` を出力済み（`compiler/ocaml/tests/test_type_inference.ml:1799-1845`）。Rust: Typeck 未移植。 |

### 1.3 Option/Result 内部実装スタイル評価

1. **データ表現**: Reml 仕様は `enum` 形で `Option`/`Result` を定義しており（`docs/spec/3-1-core-prelude-iteration.md:23-41`）、Rust 実装も `#[repr(u8)]` を付与した `enum` + `#[must_use]` で表現するのが最小。`Never` は `enum Never {}` ではなく `Result<Never, Never>` 型 alias で再現し、型推論と一致させる。
2. **インライン戦略**: `map`/`and_then` 系 API は `#[inline(always)]`、`expect` 系は `#[cold] #[track_caller]` でコンパイル時の `panic!` をデバッグビルドに限定する。`effect {debug}` の契約に従い、リリースビルドでは `panic` 経路を feature flag で排除する。
3. **型推論への適合**: OCaml の `solve_collector`/`solve_iterator` は `Iter`/`Collector` の Stage/Capability を辞書として吐き出している（`compiler/ocaml/src/constraint_solver.ml:371-477`）。Rust 実装では同じ `IteratorDictInfo` 相当を `typeck` 層に導入し、`Diagnostic.extensions["typeclass"]` に `stage_mismatch` を書き出す必要がある。
4. **計測指標**: `docs/plans/rust-migration/3-2-benchmark-baseline.md:1-78` の ±10% 規準に沿って、`core_prelude_bench` を作成し `size_of::<Option<Result<(), ()>>>()` / `iter_pipeline_throughput` / `collector_heap_bytes` を `reports/benchmarks/*.json` へ記録する。`0-3-audit-and-metrics.md` 側では `effect_analysis.missing_tag` と `iterator.stage.audit_pass_rate` を更新し、`Iter.buffered` の `effect {mem}` コストを追跡する。
5. **FFI/Diagnostic 連携**: `ensure_not_null` は FFI 入口で使用するため、`Result` → `Diagnostic` 変換ヘルパと併せて `compiler/rust/adapter` 層へ配置し、`AuditEnvelope.metadata["ffi.pointer.check"]` を残す（Phase 3-5 タスクと共有）。

## 36週目: Step 2 実施計画（Option/Result）

### 36週の到達目標
- `compiler/rust/runtime` に Prelude モジュールを追加し、`Option`/`Result`/`Never` API の 16 シナリオ snapshot テストと `cargo xtask prelude-audit` プロトタイプを整備する。
- `ensure`/`ensure_not_null` の診断連携と `core.prelude.ensure_failed` キーの定義を完了させ、`scripts/validate-diagnostic-json.sh` を通じて `reports/diagnostic-format-regression.md` への差分をゼロに保つ。
- `panic` 排除テスト (`panic_forbidden.rs`) と `cargo test --release -Z panic-abort-tests` を CI チェックリストに追加し、`effect {debug}` 以外のパスで `panic!` が呼ばれていないことを `0-3-audit-and-metrics.md` の KPI に記録する。

### 進行手順
1. **Day 1-2**: `core_prelude` module scaffolding を作成し、`prelude_api_inventory.toml` を初期化。API 抜け漏れ検出用の `cargo xtask prelude-audit --baseline docs/spec/3-1-core-prelude-iteration.md` を走らせ、既存の差分を `docs/notes/core-library-outline.md` に記録する。
2. **Day 2-3**: `Option`/`Result`/`Never` メソッド本体を実装し、`#[must_use]`/`effect {debug}` の実装を `cfg(debug_assertions)` 付きで確認。`core_prelude_option_result.rs` で 16 シナリオ snapshot を生成し、`compiler/ocaml/tests/test_type_inference.ml` の結果を比較ログとして添付する。
3. **Day 3-4**: `ensure`/`ensure_not_null` 及び `Diagnostic` 変換を実装。`scripts/validate-diagnostic-json.sh` + `tooling/ci/collect-iterator-audit-metrics.py` を実行し、`core.prelude.ensure_failed` の発火数と Stage 情報を `reports/spec-audit/ch0/links.md` に追記する。
4. **Day 4-5**: `panic_forbidden.rs` UI テストと `RUSTFLAGS="-Dpanic_fmt -Z panic-abort-tests"` 経路をセットアップ。`0-3-audit-and-metrics.md` へ KPI を書き込み、例外的に許容する `expect` の `effect {debug}` 契約を `docs/spec/3-1-core-prelude-iteration.md` の参照付きでリンクさせる。
5. **Week end**: `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` `M1` セクション、`docs/spec/3-0-core-library-overview.md` 脚注、`docs-migrations.log`（Prelude モジュール追加）を更新し、`p0` からの移行ログを結線する。

### 完了チェックリスト
- [x] `cargo xtask prelude-audit --strict` が `Option`/`Result` API で差分 0 を返し（`reports/spec-audit/ch0/links.md#prelude-実装ログ` 参照）、`core_prelude.missing_api = 0` の測定結果を保存した。
- [x] `cargo test core_prelude_option_result panic_forbidden` が成功し、`scripts/validate-diagnostic-json.sh` の比較結果を `reports/diagnostic-format-regression.md` に差分なしで反映した。
- [x] `0-3-audit-and-metrics.md` に `core_prelude.missing_api = 0`、`core_prelude.panic_path = 0` を追加し、`4-5-backward-compat-checklist.md` へ fallback プランを登録済み。
- [x] `docs/spec/3-0-core-library-overview.md` と `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` に Option/Result 実装ステータスの脚注リンクを追記し、Phase 3-1 以降の参照経路を確立した。

## 成果物と検証
- `Core.Prelude`/`Core.Iter` 実装および Collector 群が CI テストを通過し、効果タグ/属性が仕様と一致していること。
- Rust 実装のベースライン（Phase 2 ベンチマーク）と比較した性能が ±10% 以内に収まり、差分が存在する場合はメトリクスに記録されていること。OCaml 実装のデータは参考値として付録に残す。
- ドキュメント (仕様引用、ガイド、サンプル) が更新され、仕様と実装の相互参照が解決していること。

## リスクとフォローアップ
- 効果タグ伝播に不備がある場合、Phase 2 の診断タスクへエスカレートする。
- `Iter` の所有権モデルが `Core.Collections` と競合した場合は、一時的に `unsafe` ブロックの導入を避け、代替設計を `docs/notes/core-library-outline.md` に記録する。
- ベンチマーク遅延が解消しない場合、RC 最適化や並列イテレータの検討を Phase 4 の改善項目に追加する。

## 参考資料
- [3-1-core-prelude-iteration.md](../../spec/3-1-core-prelude-iteration.md)
- [3-2-core-collections.md](../../spec/3-2-core-collections.md)
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)
