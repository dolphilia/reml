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
- `compiler/rust/runtime/src/prelude/iter/mod.rs`（以下 `Iter` モジュール）と `IterState`/`IterSeed`/`IterSource` の 3 層構造を実装し、`Iter<T>` が `IntoIter`/`FromIterator` トレイトと双方向に変換できる。同時に `compiler/rust/frontend/tests/core_iter_pipeline.rs` を追加して `Iter::from_list |> Iter.collect_list` の往復と `Iter::into_std_iter` の互換性を snapshot で固定する。
- `Collector<T, C>` トレイトおよび標準コレクタ (`ListCollector`, `VecCollector`, `MapCollector`, `SetCollector`, `StringCollector`) を `compiler/rust/runtime/src/prelude/collectors/` 以下で提供し、`effect {mut}`/`effect {mem}` の転写を `tooling/ci/collect-iterator-audit-metrics.py` の `collector.effects` カラムで観測できる状態にする。
- `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` を `Iter`/`Collector` 項目まで拡張し、`cargo xtask prelude-audit --section iter` を通じて 3-1 章の API を全件スキャン。結果を `reports/spec-audit/ch0/links.md` に貼り付け、`0-3-audit-and-metrics.md` の `iterator.stage.audit_pass_rate` を更新する。
- `docs/notes/core-library-outline.md` と `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` に Iter/Collector 実装状況とリスクを記録し、Phase 3 の他タスク（Text/Collections）から参照できるリンクを設置する。

#### 3.a Collector 実装トレース（W37 現在）
- `compiler/rust/runtime/src/prelude/iter/mod.rs` には `IterState`/`IterStep`/`EffectSet`/`IteratorStageProfile` が定義され、`Iter` が `EffectLabels` を公開して `collect-iterator-audit-metrics.py` の `collector` セクションと直結する設計になっている。標準コレクタは `compiler/rust/runtime/src/prelude/collectors/{list,map,set,string,table,vec}.rs` に分散し、`CollectOutcome::audit()` で `Diagnostic.extensions["prelude.collector"]` に `kind`/`stage`/`effect`/`markers` を書き出す。
- `compiler/rust/frontend/tests/core_iter_collectors.rs` と `__snapshots__/core_iter_collectors.snap` で `List/Vec/Map/Set/String/Table` の正常系・異常系を固定し、`collector.effect.*`/`collector.error.*`/`collector.stage.*` の JSON が `reports/iterator-collector-summary.md` に記録されている。`reports/spec-audit/ch0/links.md#collector-f2` には `cargo test core_iter_collectors -- --nocapture` や `cargo insta review --review`、`collect-iterator-audit-metrics.py --module iter --section collectors --wbs 3.1b-F2` などのコマンドと出力を時系列で列挙している。
- `tooling/ci/collect-iterator-audit-metrics.py` の `collect_collector_effect_metrics` で `collector.effect.{mem,mut,debug,async_pending}` と `collector.effect.{mem_reservation,reserve,finish}` を集計し、`reports/iterator-collector-summary.md` および `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` で `collector.effect.mem=0`/`collector.stage.audit_pass_rate=1.0` を KPI として追跡している。
- `scripts/validate-diagnostic-json.sh --pattern collector` を定期実行することで `prelude.collector.*` 拡張がすべて出力されていることを確認し、`reports/diagnostic-format-regression.md` に差分が出ない状態を M1 レビュー基準とする。
- 実装の証跡は `docs/notes/core-library-outline.md#collector-f2-監査ログ`、`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md#collector-f2-監査ログ`、`reports/spec-audit/ch0/links.md#collector-f2` の三者クロスリファレンスで参照可能な構成を維持している。

**主な依存資料**
- 仕様: `docs/spec/3-1-core-prelude-iteration.md`（Iter/Collector API）、`docs/spec/3-2-core-collections.md`（永続コレクション連携）、`docs/spec/3-6-core-diagnostics-audit.md`（Stage/監査キー）
- 型推論: `docs/spec/1-2-types-Inference.md`, `compiler/ocaml/src/constraint_solver.ml`（`solve_iterator` 実装参照）
- 観測: `docs/plans/rust-migration/3-1-observability-alignment.md`, `tooling/ci/collect-iterator-audit-metrics.py`
- リスク/ログ: `docs/plans/bootstrap-roadmap/0-4-risk-handling.md`, `docs-migrations.log`

3.1. `Iter<T>` の内部表現・所有権モデル（WBS 3.1a）
- `compiler/rust/runtime/src/prelude/iter/mod.rs` を新設し、`Iter<T>` を `Arc<IterState<T>>` ベースの遅延列として定義。`IterState` では `poll_next(&mut self) -> IterStep<T>` を提供し、`IterStep` は `Ready`, `Pending`, `Finished` の 3 状態で `effect` 情報を保持する。
- `Iter::from_iter` / `impl<T> FromIterator<T> for Iter<T>` を実装し、`std::iter::from_fn` 互換の `IterSeed` を `Iter::from_fn` で生成できるようにする。逆方向の `impl<T> IntoIterator for Iter<T>` では `IterIntoStd<T>` アダプタを提供して Rust 標準 `for` 構文と連携する。
- `compiler/rust/frontend/src/typeck/constraint/iterator.rs`（新規）で `IteratorDictInfo` を導入し、`Iter` を要求する型クラス拘束に `stage`, `capability`, `kind` を埋める。辞書生成時に `Diagnostic.extensions["iterator.stage.required"]` へ書き込み、`collect-iterator-audit-metrics.py` が参照する JSON のキーを `effect.stage.iterator.*` で統一する。
- `core_iter_pipeline.rs` テストでは `Iter::from_list |> Iter::map |> Iter.collect_list`/`Iter::try_fold` の 6 シナリオを snapshot 化し、`reports/diagnostic-format-regression.md` で差分監視。`compiler/ocaml/tests/test_type_inference.ml` の `iterator_kind` 出力と結果を比較し、差分は `docs/notes/core-library-outline.md` へ記録する。

> 2026-W05 更新（Remediation Step3）: `core_iter_pipeline.rs` と `compiler/rust/frontend/tests/snapshots/core_iter_pipeline__*.snap` は未整備のままであるため、F3 スナップショット/KPI は pending 扱いに変更した。`docs/notes/core-library-outline.md` と `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` から該当リンクを一時削除し、`reports/spec-audit/ch1/iter.json` は CLI 未実装・snapshot 不在を理由に `status = "pending"` として再記録した。再実装完了後に KPI の `iterator.stage.audit_pass_rate`／`collector.effect.mem` を復活させる。

##### WBS 3.1a 実装指針とタスク詳細

| フェーズ | 目的 | 主要作業 | 成果物 / 記録 | 検証手段 |
| --- | --- | --- | --- | --- |
| ✅ F0 仕様精査 | `Iter` の 3 層構造と `effect` 契約を仕様と型推論の両面で確認する | `docs/spec/3-1-core-prelude-iteration.md` および `compiler/ocaml/src/constraint_solver.ml`（`solve_iterator`）を突き合わせ、`IterStep` が保持する `effect`/`stage`/`capability` 情報を抽出する | `docs/notes/core-library-outline.md`（Iter セクション）へのメモ、`reports/spec-audit/ch0/links.md` の参照ログ | ドキュメントレビュー、`cargo doc -p core_prelude` |
| ✅ F1 実装スキャフォールド | `compiler/rust/runtime/src/prelude/iter/mod.rs` に `Iter`, `IterState`, `IterSeed`, `IterSource`, `IterStep` を定義し、`EffectMarker` の付与点を明示する | `IterState` 骨格、`IterStep::effect_set`、`Iter::from_state`/`Iter::new_seed` API、`EffectMarker` コメント | `docs-migrations.log` への Prelude/Iter 追記、`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` への `module = "Iter"` 追加 | `cargo check -p core_prelude`, `cargo fmt` |
| ✅ F2 トレイト/Typeck 統合 | `FromIterator`/`IntoIterator` 実装と `IteratorDictInfo` を提供し、診断の `effect.stage.iterator.*` を Rust 実装で生成する | `IterIntoStd<T>`/`IterFromStd<T>` アダプタ、`compiler/rust/frontend/src/typeck/constraint/iterator.rs`（新規）、`collect-iterator-audit-metrics.py` の `iterator.dict` 列 | `reports/spec-audit/ch0/links.md` に `collect-iterator-audit --section iter` ログ、`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` M1 更新メモ | `cargo test core_iter_pipeline`, `scripts/validate-diagnostic-json.sh` |
| ✅ F3 スナップショット/KPI | 6 シナリオ snapshot と KPI 更新を行い、Chapter 3.1 の指標を Phase 3 帳票へ接続する | `compiler/rust/frontend/tests/core_iter_pipeline.rs/.snap`, `collect-iterator-audit` 吐き出し、`0-3-audit-and-metrics.md` の `iterator.stage.audit_pass_rate` 更新 | `reports/spec-audit/ch0/links.md` にテスト/監査ログ、`docs/notes/core-library-outline.md` 差分記録 | `cargo test core_iter_pipeline -- --nocapture`, `collect-iterator-audit-metrics.py --module iter` |

- **F1 進捗メモ（2025-W36 着手）**: `compiler/rust/runtime/src/prelude/iter/mod.rs` を新設し、`Iter`/`IterState`/`IterSeed` と `IterStep` の骨格を `Arc<IterState<T>>` 共有モデルで実装。`EffectSet` と `IteratorStageProfile`（`StageRequirement::{Exact, AtLeast}` + Capability）を用意し、`iterator.effect.*`／`effect.stage.iterator.*` のダイアグノスティクス整合を確認できる API (`stage_snapshot`, `effect_labels`) を追加した。
- **F2 着手メモ（2025-W36 後半）**: `compiler/rust/frontend/src/typeck/constraint/iterator.rs` を追加し、`IteratorDictInfo` が `IteratorStageProfile`/`IteratorStageSnapshot` を内蔵する形で Typeck 側の辞書と Stage/Capability 情報を接続。`solve_iterator` で `Array`/`Slice`/`Iter`/`IteratorState`/`Option`/`Result` に対応し、`iterator.stage.required`/`actual`/`capability`/`kind`/`source` が一元的に取得できる状態を整備。さらに Typeck ドライバから実行時 Stage と突き合わせて `typeclass.iterator.stage_mismatch` 診断を生成し、`collect-iterator-audit-metrics.py` が期待する `effect.stage.*` / `iterator.stage.*` メタデータを JSON に書き込む配線を追加した。
- **F3 進行メモ（2025-W37 着手）**: Snapshot/KPI サイクル専用に `compiler/rust/frontend/tests/core_iter_pipeline.rs` のシナリオ一覧と `collect-iterator-audit-metrics.py --module iter --require-success` の運用手順を整理。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ `iterator.stage.audit_pass_rate` の更新ログと KPI しきい値（1.0 未満で CI 失敗）を追記し、`reports/spec-audit/ch0/links.md` にはテスト／監査リンクの収集テンプレートを追加。`docs/notes/core-library-outline.md` には WBS 3.1a F3 用の 6 シナリオ（`from_list`, `map`, `filter_map`, `flat_map`, `try_fold`, `try_collect`）と Collector 種別の組合せ表を記録し、Phase 3-0 M1 から参照できるよう `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` の M1 セクションに KPI 連動手順を加えた。Snapshot 作成完了後は `reports/diagnostic-format-regression.md` の差分確認をフックし、監査リンクとあわせて `docs-migrations.log` に記録する。

- **進行順序**: F0→F1→F2→F3 を 36〜37 週目の 2 スプリントで完了し、フェーズ完了ごとに `docs-migrations.log` と `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` の `M1` セクションへステータスを記録する。
- **効果タグ運用**: `IterStep` へ `bitflags` 化した `EffectSet` を保持させ、アダプタは `IterState::with_effects` を経由してタグを合成する。`collect-iterator-audit` で `iterator.effect.debug = 0` を維持することを KPI とする。
- **型推論連携**: `IteratorDictInfo` は `stage`, `capability`, `kind`, `source` を JSON として `Diagnostic.extensions["iterator.stage.required"]` に書き出し、OCaml 版 `iterator_kind` と同一のログが得られるようにする。差分は `docs/notes/core-library-outline.md` に控え、必要なら `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へリンクする。
- **テスト/監査ログ**: `core_iter_pipeline.rs` 実行と `collect-iterator-audit --section iter` の結果は `reports/spec-audit/ch0/links.md` へ貼り付け、`0-3-audit-and-metrics.md` の `iterator.stage.audit_pass_rate` を更新する。Snapshot 生成時は `scripts/validate-diagnostic-json.sh` を必須ステップにする。
- **完了判定**: `cargo xtask prelude-audit --section iter --strict` が差分 0、`collect-iterator-audit-metrics.py` が `iterator.stage.audit_pass_rate = 1.0` を報告し、`core_iter_pipeline` 実行時間が `docs/plans/rust-migration/3-2-benchmark-baseline.md` の Phase 2 値 ±10% 以内である場合に WBS 3.1a をクローズする。

###### F3 シナリオ構成と KPI 連携（WBS 3.1a）

| シナリオID | Pipeline | Collector | 効果タグ検証 | Snapshot 参照 | KPI/ログ |
| --- | --- | --- | --- | --- | --- |
| `iter_from_list_roundtrip` | `Iter::from_list |> Iter.collect_list` | `ListCollector` | `@pure` 維持、`iterator.effect.* = ∅` | `compiler/rust/frontend/tests/core_iter_pipeline.rs#L30` 近辺 | `reports/spec-audit/ch0/links.md` / `docs/notes/core-library-outline.md` |
| `iter_map_utf8` | `Iter::from_list |> Iter::map |> Iter.collect_list` | `VecCollector` | `effect {mem}` 非発生、`IteratorStageProfile::beta` を JSON 転写 | 同 `#L70` 付近 | `collect-iterator-audit-metrics.py --module iter --scenario map` |
| `iter_filter_map_cap` | `Iter::from_list |> Iter::filter_map` | `ListCollector` | `@pure` + `iterator.effect.debug = 0` を監視 | 同 `#L120` 付近 | `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` KPI ログ |
| `iter_flat_map_stage` | `Iter::from_list |> Iter::flat_map` | `VecCollector` | `StageRequirement::AtLeast(\"beta\")` の辞書整合 | 同 `#L170` 付近 | `reports/spec-audit/ch0/links.md` Stage 表 |
| `iter_try_fold_diag` | `Iter::map |> Iter::try_fold` | `ResultCollector` （診断用シュミレータ） | `effect {mut}` が残余へ出ないこと、`typeclass.iterator.stage_mismatch = 0` | 同 `#L230` 付近 | `reports/diagnostic-format-regression.md` 差分 |
| `iter_try_collect_set` | `Iter::from_list |> Iter::try_collect(SetCollector)` | `SetCollector` | `effect {mem}`/`collector.effect.mem` 転写、`iterator.stage.audit_pass_rate` 対象 | 同 `#L300` 付近 | `collect-iterator-audit-metrics.py --module iter --section collectors` |

- Snapshot 手順: 各シナリオを `compiler/rust/frontend/tests/core_iter_pipeline.rs` に記述し、`cargo test core_iter_pipeline -- --nocapture`→`cargo insta review` の順で `.snap` を固定。生成した snapshot パスを `reports/spec-audit/ch0/links.md` と `docs/notes/core-library-outline.md` に追記する。
- KPI 反映: `tooling/ci/collect-iterator-audit-metrics.py --module iter --output reports/iterator-stage-summary.md` を実行し、`iterator.stage.audit_pass_rate` を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI ログへ転記。しきい値 1.0 を下回った場合は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に是正タスクを作成する。
- 監査ログ: `reports/spec-audit/ch0/links.md` に `Iter F3 Snapshot/KPI` サブセクションを新增し、コマンド・結果・関連ファイルを列挙。`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` の `M1` 節から参照できるよう相互リンクを貼る。

3.2. `Collector` トレイトと標準コレクタ（WBS 3.1b）
- `compiler/rust/runtime/src/prelude/collectors/mod.rs` を作成し、仕様どおりの `Collector<T, C>` トレイトと `type Error: IntoDiagnostic` 制約を定義。`with_capacity`/`reserve` は `effect {mem}` を伴うため `#[cfg_attr]` で `EffectMarker` を付与する。
- `ListCollector`/`VecCollector`/`MapCollector`/`SetCollector`/`StringCollector` をそれぞれ `@pure` / `effect {mut}` / `effect {mem}` の組み合わせに沿って実装。`VecCollector` では `Result<Vec<T>, CollectError>` を返し、`CollectError` は `MemoryError`/`DuplicateKey`/`InvalidEncoding` 等のバリアントを備える。
- `compiler/rust/frontend/tests/core_iter_collectors.rs` を追加し、(1) 正常系で `Iter.try_collect` → `List`, `Vec`, `Map` を検証、(2) エラー系で `VecCollector::reserve` の `effect {mem}` が `Diagnostic` に転写されること、(3) `MapCollector` が重複キーを `CollectError::DuplicateKey` として報告すること、の 3 グループを snapshot で固定する。
- `tooling/ci/collect-iterator-audit-metrics.py` に Collector 列を追加し、`collector.effect.mem`, `collector.effect.mut`, `collector.error.kind` を集計。結果を `0-3-audit-and-metrics.md` の KPI 表（`iterator.stage.audit_pass_rate`, `collector.error.duplicate_key_rate`）へ貼り付ける。

##### WBS 3.1b 実装指針とタスク詳細

| フェーズ | 状態（目安週） | 目的 | 主な作業 | 成果物 / 記録 | 検証手段 |
| --- | --- | --- | --- | --- | --- |
| ✅ F0 仕様精査 | 予定（W36 前半） | Collector 契約とエラー体系を仕様/Chapter 3 横断で整理 | `docs/spec/3-1-core-prelude-iteration.md`・`docs/spec/3-2-core-collections.md`・`docs/notes/core-library-outline.md` の記述を比較し、`@pure`/`effect {mut}`/`effect {mem}` のタグ表と `CollectError` バリアント一覧を作る | `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `module = "Collector"` 節に effect/stage/wbs 情報を拡充、`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` M1 に脚注を追加 | ドキュメントレビュー、`tooling/ci/collect-iterator-audit-metrics.py --dry-run` |
| ✅ F1 トレイト骨格 & EffectMarker | 完了（W36 後半） | `Collector<T, C>` トレイト本体と `EffectMarker` の付与点を定義し `Iter.try_collect` と連携 | `compiler/rust/runtime/src/prelude/collectors/mod.rs` を新設し、`CollectOutcome`/`CollectorStageProfile`/`CollectError`/`EffectMarker` 定数を整備。`Collector::with_capacity`/`reserve`/`finish` に `effect {mem}` 契約と監査キーをコメントとして埋め込み `core_prelude` へ再輸出した。 | `docs-migrations.log` へ Collector 階層追加、`docs/plans/rust-migration/3-1-observability-alignment.md` に Collector 監査イベント参照を追記 | `cargo doc -p core_prelude`、`scripts/validate-diagnostic-json.sh --module collector`（仮ルール） |
| F2 標準コレクタ実装 | 予定（W37 前半） | List/Vec/Map/Set/String の Collector を実装し `Iter.try_collect` で使用 | `compiler/rust/runtime/src/prelude/collectors/{list,vec,map,set,string}.rs` を作成し `new`/`with_capacity`/`push`/`reserve`/`finish` を実装、`CollectError::MemoryError`/`DuplicateKey`/`InvalidEncoding` を返す | `compiler/rust/frontend/tests/core_iter_collectors.rs` 正常系 6 ケース、`docs/notes/core-library-outline.md` に Collector 実装メモ | `cargo test core_iter_collectors`, `collect-iterator-audit --section iter --filter collector` |
| F3 エラー/監査経路 | 予定（W37 中盤） | `CollectError` と診断/監査ログの連携を固定し KPI を更新 | `compiler/rust/runtime/src/prelude/collectors/error.rs` で `CollectError`↔`Diagnostic` 変換ヘルパを実装、`collect-iterator-audit-metrics.py` に `collector.effect.*` 列を追加し `reports/spec-audit/ch0/links.md` に記録 | `0-3-audit-and-metrics.md` の KPI 表更新、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ残課題を登録 | `scripts/validate-diagnostic-json.sh`, `tooling/ci/collect-iterator-audit-metrics.py --require-success` |
###### F0 仕様精査サマリ（WBS 3.1b, 2025-W37）

**Collector トレイト API と効果タグ**

| API | 効果 | Stage 要件 | 仕様根拠 | メモ |
| --- | --- | --- | --- | --- |
| `Collector::new` | `@pure` | `AtLeast("beta")` | `docs/spec/3-1-core-prelude-iteration.md†L156-L166` | `IterState` 側の `EffectSet` を変化させない前提で `EffectMarker` 不要。 |
| `Collector::with_capacity` | `effect {mem}` | `AtLeast("beta")` | `docs/spec/3-1-core-prelude-iteration.md†L156-L166` | 先行確保が `collector.effect.mem` を増分するので `collect-iterator-audit` で監視。 |
| `Collector::push` | `effect {mut}` | `AtLeast("beta")` | `docs/spec/3-1-core-prelude-iteration.md†L156-L166` | `Result<(), Error>` で短絡、`IntoDiagnostic` 経由で診断化。 |
| `Collector::reserve` | `effect {mut, mem}` | `AtLeast("beta")` | `docs/spec/3-1-core-prelude-iteration.md†L156-L166` | `VecCollector`/`StringCollector` などが `CapacityOverflow` を返す起点。 |
| `Collector::finish` | `effect {mem}` | `AtLeast("beta")` | `docs/spec/3-1-core-prelude-iteration.md†L156-L166` | 所有権を収束させて `AuditEnvelope.metadata.collector.kind` を確定させる。 |
| `Collector::into_inner` | `@pure` | `AtLeast("beta")` | `docs/spec/3-1-core-prelude-iteration.md†L156-L166` | 型変換のみを行い `iterator.effect.*` を汚さない軽量経路。 |

**標準コレクタと想定エラー**

| Collector | 効果 | エラー種別 | Stage 要件 | 仕様根拠 | WBS |
| --- | --- | --- | --- | --- | --- |
| `ListCollector` | `@pure` | なし | `Exact("stable")` | `docs/spec/3-1-core-prelude-iteration.md†L237-L253`, `docs/spec/3-2-core-collections.md†L154-L166` | 3.1b |
| `VecCollector` | `effect {mut, mem}` | `CollectError::MemoryError` / `CollectError::CapacityOverflow` | `AtLeast("beta")` | `docs/spec/3-1-core-prelude-iteration.md†L237-L253`, `docs/spec/3-2-core-collections.md†L154-L165` | 3.1b |
| `MapCollector` | `@pure` | `CollectError::DuplicateKey` | `AtLeast("beta")` | `docs/spec/3-1-core-prelude-iteration.md†L188-L198`, `docs/spec/3-2-core-collections.md†L75-L88` | 3.1b |
| `SetCollector` | `@pure` | `CollectError::DuplicateKey` | `Exact("stable")` | `docs/spec/3-2-core-collections.md†L154-L166` | 3.1b |
| `StringCollector` | `effect {mem}` | `StringError::InvalidEncoding` | `AtLeast("beta")` | `docs/spec/3-1-core-prelude-iteration.md†L237-L253` | 3.1b |
| `TableCollector` | `effect {mut}` | `CollectError::DuplicateKey` | `AtLeast("beta")` | `docs/spec/3-2-core-collections.md†L154-L168` | 3.1b |

**CollectError / StringError 整理**
- `CollectError::DuplicateKey` は `MapCollector`/`SetCollector`/`TableCollector` がキー競合を検出した際に返し、`docs/spec/3-6-core-diagnostics-audit.md†L40-L120` で求められる `change_set` 情報（競合キー/挿入順）を `Diagnostic` と `AuditEnvelope.metadata.collector.error.key` へ転写する。
- `CollectError::MemoryError` は `VecCollector::push`/`reserve` と `collect_vec` が確保失敗を `effect {mem}` 起点として表現する。`CapacityOverflow` は `effect.stage.iterator.*` と合わせて `R-027` の過剰確保リスク（`docs/plans/bootstrap-roadmap/0-4-risk-handling.md`）へ直結させる。
- `StringCollector` は UTF-8 正規化のため `StringError::InvalidEncoding`（Core Text 章で再利用）を返し、`collector.effect.mem` を `collect-iterator-audit` の `collector.effect.mem_leak` KPI に集計する。

**検証と在庫管理**
- `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `module = "Collector"` エントリに上記効果タグと Stage 区分を反映し、`last_updated = "2025-11-20 / WBS 3.1b F0"` に更新した。
- `reports/spec-audit/ch0/links.md#collector-f0` に `sed -n` ベースの根拠コマンドと `prelude_api_inventory` diff 参照を追記し、Phase 3 `M1` から辿れるようリンクした。
- `tooling/ci/collect-iterator-audit-metrics.py --module iter --section collectors --dry-run` の期待パラメータセットをメモしておき、F1 以降で実装が入った際に即座に KPI 収集へ移行できるようにした。

###### F1 トレイト骨格サマリ（WBS 3.1b, 2025-W36）

- `compiler/rust/runtime/src/prelude/collectors/mod.rs` を追加し、`Collector<T, C>` トレイト本体、`CollectOutcome`、`CollectorStageProfile`/`CollectorStageSnapshot`、`CollectorAuditTrail` を定義。`EffectMarker` 用キー `collector.effect.mem_reservation`・`collector.effect.reserve`・`collector.effect.finish` を導入し、`with_capacity`/`reserve`/`finish` の効果タグを実装側で明示できるようにした。
- `CollectError`/`CollectErrorKind` を実装し、`IntoDiagnostic` で `Diagnostic.extensions["prelude.collector"]` と `AuditEnvelope.metadata.collector.*` に `kind`/`stage`/`effects`/`error_kind` を書き込む。これにより `docs/plans/rust-migration/3-1-observability-alignment.md` で要求される `collector.effect.*` KPI を Rust 実装で観測できる足場を確保した。
- `compiler/rust/runtime/ffi/src/core_prelude/mod.rs` に `iter`/`collectors` モジュールを `#[path = "../../../src/prelude/**"]` で取り込み、`Collector` まわりの型・EffectMarker 定数を再輸出。`cargo check --manifest-path compiler/rust/runtime/ffi/Cargo.toml` でビルドを確認し、F2 以降がこの骨格を再利用できる状態にした。

###### F2 標準コレクタ実装サマリ（WBS 3.1b, 2025-W37 前半）

- F2 のゴールは、`List`/`Vec`/`Map`/`Set`/`String` の 5 種コレクタを Rust 実装で完成させ、`Iter.try_collect`・`collect_*` 終端操作・監査メトリクスを連結させること。仕様根拠は `docs/spec/3-1-core-prelude-iteration.md†L188-L253` と `docs/spec/3-2-core-collections.md†L75-L168`。
- 実装順序: **不変コレクタ**（List/Set/Map）→ **可変・メモリ要求**（Vec/String）→ **補助モジュール**（共通エラー、TableCollector）。`ListCollector` をテンプレートとして作り、以降の Collector では効果タグと `CollectError` 派生に集中できるようにする。
- KPI は `collect-iterator-audit-metrics.py --module iter --section collectors --wbs 3.1b-F2` の JSON から `collector.effect.mem`, `collector.effect.mut`, `collector.error.duplicate_key_rate`, `iterator.stage.audit_pass_rate` を抽出し `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に追記。しきい値割れ時は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` にフォローアップチケットを登録する。
- テストは `compiler/rust/frontend/tests/core_iter_collectors.rs` をベースに ①通常系（List/Vec/String）②重複キー（Map/Set）③メモリ/エンコーディング異常（Vec/String）を `insta` snapshot 化し、`reports/spec-audit/ch0/links.md#collector-f2` にコマンドと参照リンクを記録する。

| 手順 | 目的 | 主要ファイル / コマンド | 成果物・チェックポイント |
| --- | --- | --- | --- |
| ✅ F2-1 List/Vec 雛形 | 遅延列と Collector トレイトの往復を最小構成で確認 | `compiler/rust/runtime/src/prelude/collectors/{list,vec}.rs`, `compiler/rust/frontend/tests/core_iter_collectors.rs` | `collector.effect.* = ∅`（List）、`collector.effect.mem_reservation>0`（Vec）を `collect-iterator-audit` ログで確認。《Diag拡張: prelude.collector.kind=list/vec》 |
| ✅ F2-2 Map/Set Stage 宣言 | `CollectError::DuplicateKey` と Stage 要件を辞書・診断へ転写 | `.../map.rs`, `.../set.rs`, `tooling/ci/collect-iterator-audit-metrics.py --case collector-duplicate` | `AuditEnvelope.metadata.collector.error.key` に重複キーを記録、`iterator.stage.audit_pass_rate = 1.0` を維持。 |
| ✅ F2-3 String/UTF-8 | `StringCollector` の UTF-8 正規化と `StringError::InvalidEncoding` を実装し、invalid case を診断へ変換 | `.../string.rs`, `docs/spec/3-3-core-text-unicode.md`, `core_iter_collectors.rs` (string ケース) | `collect_string_invalid` で `collector.error.invalid_encoding` を `GuardDiagnostic` 化し `reports/iterator-collector-summary.md` を通じて `collector.effect.mem` と合わせて KPI を観測。 |
| ✅ F2-4 API インベントリ更新 | Collector API の在庫・WBS・効果タグを機械管理 | `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml`, `cargo xtask prelude-audit --section iter --filter collector` | `rust_status` を `planned→implemented` へ更新し、`last_updated = "2025-11-25 / WBS 3.1b F2"` を記録。 |
| ✅ F2-5 監査ログ整備 | KPI とコマンド履歴を公開しクロスリファレンスを確立 | `reports/spec-audit/ch0/links.md`, `docs/notes/core-library-outline.md`, `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` | `Collector F2` セクションを追加し、タスク・スナップショット・ベンチ結果を相互参照。 |

F2-5 で追加した `reports/spec-audit/ch0/links.md#collector-f2-監査ログ` セクションは `reports/iterator-collector-summary.md`・`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`・`docs/notes/core-library-outline.md#collector-f2-監査ログ` との三者クロスリファレンスを提供しており、`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md#collector-f2-監査ログ` で M1 レビューの証跡として参照可能な構成に整備済み。

###### F2-1 List/Vec 雛形実装メモ（W37 前半）

- `Compiler/rust/runtime/src/prelude/collectors/list.rs` / `vec.rs` を `Collector` トレイトの `new`/`with_capacity`/`push`/`finish` を実装するテンプレートとして設置し、`ListCollector` は `effect = @pure` の永続リスト構築、`VecCollector` は `effect {mem}` を `with_capacity`→`reserve` で記録する。
- `ListCollector::finish` では `CollectOutcome::audit()` を呼び出し `collector.stage.actual = "stable"` を `Diagnostic.extensions["prelude.collector"]` に、`VecCollector` では `collector.effect.mem_reservation` を `EffectLabels` 経由で `collect-iterator-audit` ログに渡し、`CollectOutcome` 内に snapshot 用情報を埋め込む。
- `compiler/rust/frontend/tests/core_iter_collectors.rs` に `collect_list_baseline`/`collect_vec_mem_error` ケースを追加し、`insta` snapshot で `List` は `collector.effect.* = ∅`、`Vec` では `collector.effect.mem_reservation>0` を再現。`prelude.collector.kind`/`collector.effect.mem` を `scripts/validate-diagnostic-json.sh` の `--pattern collector` で検出できるようにする。
- `compiler/rust/runtime/src/prelude/collectors/map.rs`/`set.rs`/`string.rs` を追加し、重複キー検出・Stage の記録・UTF-8 およびメモリ効果を `CollectOutcome` と `CollectError` に流す。`List/Vec` に続く `CollectorKind` 全般で `CollectorEffectMarkers` を共有し、`collector.effect.mem_reservation`/`reserve`/`finish` を `Diagnostic` 拡張および `collect-iterator-audit` 監査ログに含める。
- `tooling/ci/collect-iterator-audit-metrics.py --section collectors` を `collector.effect.*` を読み取るように拡張し、`collector.effect.mem_reservation`/`collector.effect.reserve`/`collector.effect.finish` の合計と `stage/kind` の分布を `reports/iterator-collector-summary.md` に出力できるようにする。
- `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` では `ListCollector`/`VecCollector` に `rust_status = "working"` 様式を導入し `notes` に F2-1 での `Diag` 拡張（`prelude.collector.kind=list/vec`）と KPI 参照 (`collect-iterator-audit --section collectors --case list,vec`) を追記する。
- 成果物は `reports/spec-audit/ch0/links.md#collector-f2` へ `collect-iterator-audit` の出力と `core_iter_collectors.rs` 実行コマンドを記録し、`docs/notes/core-library-outline.md` に `Collector` 監査構造の要約を追加、Phase 3 自身の参照線を確保する。

**Collector ごとの実装指針**
1. **ListCollector**: `effect = @pure` を保証するベースライン。`List::cons` + `finish` で永続リストを構築し、`IteratorStageProfile::stable` を固定。`collector.stage.actual = "stable"` を `Diagnostic.extensions["prelude.collector"]` へ書き込む。
2. **VecCollector**: `Vec<T>` 内部バッファと `EffectMarker::mem_reservation` を `with_capacity` で発火。確保失敗は `CollectError::MemoryError { attempted, collector: "Vec" }` を返し、`collector.effect.mem` にバイト数を記録。
3. **MapCollector / SetCollector**: `collectors/common.rs` に `fn check_duplicate<K: Eq + Hash>` を用意し、重複キー検出時に `AuditEnvelope.metadata.collector.error.key = format!("{:?}", key)` を残す。`SetCollector` は `StageRequirement::Exact("stable")` を `IteratorDictInfo` へ転写。
4. **StringCollector**: Core Text 章予定の `StringError` を `CollectError::InvalidEncoding(StringError)` 経由で再利用。`effect {mem}` により `collector.effect.mem_leak` KPI をモニタし、invalid case は `core_iter_collectors.rs::string_invalid` で snapshot 固定。
5. すべての Collector で `finish` に `CollectOutcome::audit()` を呼び `Diagnostic.extensions["prelude.collector"]` の `kind`/`stage`/`effect`/`wbs` を埋める。Snapshot には `kind`, `effects`, `error_kind` を JSON で残し `reports/spec-audit/ch0/links.md` から逆引き可能にする。

**テストと検証**
- `cargo test core_iter_collectors -- --nocapture` を CI ジョブへ追加し、`RUSTFLAGS="-Zpanic-abort-tests"` を共有。`collect_list_baseline`/`collect_vec_mem_error`/`collect_map_duplicate`/`collect_set_stage`/`collect_string_invalid` の 5 ケースを `insta` で固定。
- `scripts/validate-diagnostic-json.sh --pattern collector` を走らせ、`prelude.collector.*` キーが `reports/diagnostic-format-regression.md` に差分なしで反映されることを確認。
- `tooling/ci/collect-iterator-audit-metrics.py --module iter --section collectors --output reports/iterator-collector-summary.md` を実行し、`collector.effect.mem = 0（List/Set/Map）`、`collector.error.duplicate_key_rate = 0`、`iterator.stage.audit_pass_rate = 1.0` を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI に転記する。

###### F2-2 Map/Set Stage 宣言（W37 中盤）

- `docs/spec/3-1-core-prelude-iteration.md:188-253` と `docs/spec/3-2-core-collections.md:75-168` に記された `MapCollector`/`SetCollector` の Stage 要件を `CollectorStageProfile` へ盛り込み、`CollectorEffectMarkers` の `stage_requirement` に `beta`/`stable` の境界を記録する。`IteratorDictInfo` の `stage_mismatch` フラグは `Diagnostic.extensions["prelude.collector.stage"]` へ出力して `collect-iterator-audit` で検出できるようにする。
- `MapCollector` は `StageRequirement::AtLeast("beta")` を `CollectOutcome::stage()` から `AuditEnvelope.metadata.collector.stage.required`/`actual` へ転写し、`CollectError::DuplicateKey` で `AuditEnvelope.metadata.collector.error.key` と `Diagnostic.extensions["prelude.collector.error_key"]` にキー文字列を残す。`SetCollector` では `StageRequirement::Exact("stable")` を `collector.stage.actual` に固定し、重複キー・順序違反などが出た時点で `stage` を `beta` から `stable` への遷移差分として監査ログに書き出す。
- `tooling/ci/collect-iterator-audit-metrics.py --case collector-duplicate` を通じて `collector.error.duplicate_key_rate`、`collector.error.key`、`iterator.stage.audit_pass_rate`、`collector.stage.mismatch_rate` を JSON 出力し、`reports/iterator-collector-summary.md` と `reports/spec-audit/ch0/links.md#collector-f2` にコマンドと結果を記録する。
- `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `MapCollector`/`SetCollector` 項目で `rust_status = "working"` を維持しつつ `notes` を F2-2 向けに更新し、`CollectError::DuplicateKey` 新規エントリを `AuditEnvelope.metadata` と `Diagnostic.extensions` の紐付けで追加。`last_updated` を `2025-12-01 / WBS 3.1b F2-2` へ差し替え、`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` M1 節の脚注にも `Map/Set` Stage と `collector-stage` メトリクスを追記する。
- `docs/notes/core-library-outline.md` には `CollectorStageProfile` と `CollectError::DuplicateKey` の実装状況を追記し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の `collector.effect.*` KPI に `collector.stage.audit_pass_rate` と `collector.error.duplicate_key_rate` を併記する。Stage を失敗させたケースは `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に是正タスクを作成する。
- `reports/spec-audit/ch0/links.md` の `Collector F2` セクションには `collect-iterator-audit --case collector-duplicate` の実行出力、小計 `reports/iterator-collector-summary.md`、`core_iter_collectors.rs` スナップショットファイル群を列挙して参照可能にして、Phase 3 要求の `iterator.stage.audit_pass_rate = 1.0` を証明する。

| F4 Snapshot & ハンドオーバー | 予定（W37 後半） | Snapshot 試験・API 在庫更新・Phase 3 ハンドオーバー資料を完成 | `core_iter_collectors.rs` で `VecCollector::reserve`/`MapCollector` 重複キー/`StringCollector` 文字列化失敗などエラーケースを snapshot 化、`reports/spec-audit/ch0/links.md` と `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` M1 へリンク、`prelude_api_inventory.toml` の `last_updated` を更新 | `compiler/rust/frontend/tests/__snapshots__/core_iter_collectors.snap`, `collect-iterator-audit --section collector --output reports/spec-audit/ch0/collector-YYYYMMDD.json` | `cargo test core_iter_collectors -- --ignored`, `collect-iterator-audit --section collector` |

- `CollectError` では `MemoryError`（`effect {mem}` 発生源を `Diagnostic.extensions["collector.effect.mem"]` へ書き込み）、`DuplicateKey`（`effect {mut}`）、`InvalidEncoding`（`Core.Text` と連携）、`CapacityOverflow`（`effect {mem}` + `effect {debug}`）を最低限のバリアントとして定義し、`docs/plans/bootstrap-roadmap/0-4-risk-handling.md` の `R-027 (Collector メモリ過剰確保)` に紐付ける。
- `ListCollector`/`VecCollector`/`SetCollector` の `finish` では `AuditEnvelope.metadata.collector.kind` を設定し、`collect-iterator-audit-metrics.py` が `collector.effect.mem_leak` を算出できるよう `collector.effect.mem`, `collector.effect.mut` を JSON Schema に追加する。スキーマ更新は `docs/plans/rust-migration/unified-porting-principles.md` の監査節を参照し、差分を `reports/spec-audit/ch0/links.md` に残す。
- Snapshot では `Iter::try_collect(ListCollector::new)`（ゼロアロケーション）、`Iter::try_collect(VecCollector::with_capacity(4))`（`effect {mem}` 発生の観測）、`Iter::try_collect(MapCollector::new)`（重複キー検出）、`Iter::try_collect(StringCollector::utf8())`（`InvalidEncoding`）を最小構成とし、`docs/plans/bootstrap-roadmap/4-4-ecosystem-migration.md` の `SDK/Bundles` 表と feature 名を同期する。
- KPI 更新時は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `collector.error.duplicate_key_rate`・`collector.effect.mem_leak`・`collector.effect.mem_reservation_hits` を追記し、結果 JSON を `reports/spec-audit/ch0/collector-YYYYMMDD.json` として保存、`reports/spec-audit/ch0/links.md` から参照する。

3.3. 生成系ヘルパと `Iter` API 網羅（WBS 3.1c）
- `Iter::empty`/`once`/`repeat`/`range`/`from_list`/`from_result`/`from_fn`/`unfold`/`try_unfold` を `iter/generators.rs`（新規モジュール）へ分割し、`@pure`/`effect {mem}` のタグを仕様表（本計画書の API マトリクス）と同期させる。`range` は `Int` 型専用、`try_unfold` は `Result` を伝播するフェイルファスト契約を記載する。
- `Iter::buffered`/`Iter::enumerate`/`Iter::zip` など Chapter 3.1 の変換 API を `iter/adapters.rs` へ実装し、`effect` を `IterState` に保持する。`buffered` は内部バッファサイズを `usize` で管理し、`effect {mem}` の計測を `collect-iterator-audit` に報告する。
- `compiler/rust/frontend/tests/core_iter_generators.rs` で生成系 API の黄金テストを追加し、`Iter.range` + `take` + `collect_vec` のような複合シナリオを 12 ケース固定。更に `compiler/rust/frontend/tests/core_iter_effects.rs` を用意して `effect {mem}`/`effect {mut}` の伝播を `Diagnostic` 拡張領域で確認する。
- `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` に `module = "Iter"` エントリを追加し、`cargo xtask prelude-audit --section iter --baseline docs/spec/3-1-core-prelude-iteration.md` を nightly で実行。結果 JSON を `reports/spec-audit/ch1/iter.json`（新規）に保存し、`reports/spec-audit/ch0/links.md` から参照する。

##### 3.3b 生成 API 実装ステップ（WBS 3.1c-F1）
| 手順 | 内容 | 主担当ファイル / コマンド | 完了条件 |
| --- | --- | --- | --- |
| ✅ F1-1 モジュール分離 | `compiler/rust/runtime/src/prelude/iter/generators.rs` を追加し、`IterState`/`IterSeed`/`IterSource` の内部 API を `crate` 可視で再編。`Iter` 公開 API は `mod generators; pub use generators::*;` で委譲し、`EffectLabels` を注入できる `fn attach_effects(step: &mut IterStepMetadata)` を確保する。 | `compiler/rust/runtime/src/prelude/iter/mod.rs`, `.../iter/generators.rs` | `cargo fmt`/`cargo clippy` が警告 0。`Iter` の公開関数群が 3-1 仕様表と 1:1 で並び、`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `module = "Iter"` に WBS 3.1c-F1 脚注が記録される。 |
| ✅ F1-2 `from_list`/`from_result`/`from_fn` | `Iter::from_list` は `ListCollector` と同じノード表現（`Arc<ListNode<T>>`）を共有し、Stage/効果タグを `CollectOutcome` と揃える。`from_result` は `Result<T, E>` を `Iter<T>` に変換、`from_fn` は `FnMut() -> Option<T>` を `IterSeed` に包む。いずれも `@pure` を維持し `EffectLabels::residual = []` とする。 | `.../iter/generators.rs`, `.../collectors/list.rs`, `docs/spec/3-1-core-prelude-iteration.md:100-175` | `core_iter_generators.rs::from_list_roundtrip` / `from_result_passthrough` / `from_fn_counter` を追加し、`collect-iterator-audit --section iter --case from_list` の JSON を `reports/spec-audit/ch1/iter.json` に保存しつつ `reports/spec-audit/ch0/links.md#iter-generators` に出力を添付。 |
| ✅ F1-3 `empty`/`once`/`repeat`/`range` | `Iter::range` は `RangeState { current, end, step }` と `IterRangeError` を新設し、オーバーフロー時は `IterStep::Error` を返す。`repeat` は `effect {debug}` を含まない `@pure` だが、`diagnostic.extensions["iterator.repeat"] = true` を埋め `collect-iterator-audit` で識別できるようにする。`empty`/`once` は ZST を共有し GC コストを抑える。 | `.../iter/generators.rs`, `.../iter/errors.rs`（新規） | `core_iter_generators.rs::range_basic` `range_overflow_guard` `repeat_take` `empty_collect` を追加し、`RUSTFLAGS="-Zpanic-abort-tests" cargo test core_iter_generators` が成功。`reports/spec-audit/ch1/iter.json` に `range` KPI を記録。 |
| ✅ F1-4 `unfold`/`try_unfold` | `Iter::unfold` は状態クロージャを `IterSeed` に格納。`try_unfold` は `Result<Option<(T, State)>, E>` を受け、`Err(E)` を `IterStep::Error` として `EffectLabels::residual = ["debug"]`、`AuditEnvelope.metadata.iterator.error.kind = "try_unfold"` を設定。 | `.../iter/generators.rs`, `.../iter/effects.rs` | `core_iter_generators.rs::unfold_fibonacci_pipeline` / `try_unfold_error_passthrough` を snapshot 化し、`collect-iterator-audit --section iter --case unfold|try_unfold` の KPI を `reports/spec-audit/ch1/iter.json#audit_cases.*` に保存。 |
| ✅ F1-5 API 在庫同期 | `prelude_api_inventory.toml` に `module = "Iter"` エントリを追加し `rust_status` を `working`→`implemented` に段階更新。`cargo xtask prelude-audit --section iter --baseline docs/spec/3-1-core-prelude-iteration.md --wbs 3.1c-F1` を nightly job へ登録し、JSON 出力を `reports/spec-audit/ch1/iter.json` に保存、`reports/spec-audit/ch0/links.md` から参照する。 | `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml`, `reports/spec-audit/ch1/iter.json` | `prelude_api_inventory` の `last_updated = "2025-12-22 / WBS 3.1c-F1-4/5"` を記録し、`iterator.api.coverage=1.0`（生成 API 15 件）と `pending_entries` 解消の結果を `reports/spec-audit/ch0/links.md#iter-generators` に記録する。 |
- **F1-1 実装ログ（2025-12-08）**: `compiler/rust/runtime/src/prelude/iter/generators.rs` を新設し `Iter` の `from_state`/`stage_snapshot`/`effect_labels` をここへ移管するとともに `IterStepMetadata`/`attach_effects` を追加して効果ラベル注入点を確保。`docs-migrations.log` に日付付きの F1-1 報告を追記し、`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `module = "Iter"` 行に `WBS 3.1c-F1` 脚注を残した。 |
- **F1-2 実装ログ（2025-12-12）**: `Iter::from_list`/`Iter::from_result`/`Iter::from_fn` を `ListCollector` のノード構造と `IteratorSeed` 設計で実装し、`EffectLabels::residual = []` を維持した `@pure` 生成器として `CollectOutcome` の `stage`/`effect` との整合を取った。`core_iter_generators.rs` の `from_list_roundtrip`/`from_result_passthrough`/`from_fn_counter` に `insta` スナップショットを追加し、`collect-iterator-audit --section iter --case from_list` の `iterator.stage.audit_pass_rate=1.0`/`collector.effect.mem=0` KPI を `reports/spec-audit/ch1/iter.json` に保存して `reports/spec-audit/ch0/links.md#iter-generators` に連携した。`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` では該当 API を `rust_status=implemented`・`wbs = "3.1c F1"` に更新し、`cargo xtask prelude-audit --section iter --wbs 3.1c-F1` の自動チェックが `reports/spec-audit/ch0/links.md#iter-generators` に記録された。 |
- **F1-3 実装ログ（2025-12-16）**: `Iter::empty`/`once`/`repeat`/`range` を `compiler/rust/runtime/src/prelude/iter/generators.rs` に追加し、`IterRangeError` と `RangeState` を `iter/errors.rs` へ分離。`core_iter_generators.rs` へ `range_basic`/`range_overflow_guard`/`repeat_take`/`once_collect`/`empty_collect` ケースを追加し、`RUSTFLAGS="-Zpanic-abort-tests" cargo test core_iter_generators -- --nocapture` のログを `reports/spec-audit/ch0/links.md#iter-generators` に収集した。`collect-iterator-audit --section iter --case range|repeat|once|empty` の出力を `reports/spec-audit/ch1/iter.json#audit_cases.*` に連携し、`iterator.stage.audit_pass_rate=1.0`/`iterator.range.overflow_guard=1`/`iterator.repeat.flagged=true` KPI を記録。`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `Iter` セクションでは 4 API を `rust_status=implemented`、`wbs = "3.1c F1-3"` とし、`meta.last_updated` を `2025-12-16 / WBS 3.1c-F1-3` に更新した。 |
- **F1-4 実装ログ（2025-12-22 完了）**: `compiler/rust/runtime/src/prelude/iter/generators.rs` に `Iter::unfold`/`Iter::try_unfold` を追加し、`IterSeed` のステージ／ラベル情報と `IterStepMetadata::flag_error`（`iterator.error.kind` 追記）を整備。`core_iter_generators.rs::unfold_fibonacci_pipeline` と `::try_unfold_error_passthrough` の snapshot を `cargo insta review --review core_iter_generators --filter unfold` で確定し、`collect-iterator-audit --section iter --case unfold|try_unfold --output reports/spec-audit/ch1/iter.json` を実行して `iterator.stage.audit_pass_rate=1.0`・`iterator.unfold.depth=8`・`iterator.try_unfold.error_kind=\"try_unfold\"` を KPI として記録した。`scripts/validate-diagnostic-json.sh --pattern iterator --module try_unfold` を通過させた上で `prelude_api_inventory.toml` の `Iter::unfold`/`Iter::try_unfold` を `rust_status=implemented` へ更新し、`pending_entries` を `reports/spec-audit/ch1/iter.json` から除去。`docs/notes/core-library-outline.md#iter-generators-f1-4` と `reports/spec-audit/ch0/links.md#iter-f1-4` に証跡を追記し、Phase 3 M1 の根拠資料へリンクした。
- **F1-5 実装ログ（2025-12-20 完了）**: `cargo xtask prelude-audit --section iter --baseline docs/spec/3-1-core-prelude-iteration.md --wbs 3.1c-F1-5` を実行し、生成 API 15 件（from_list〜try_unfold）を `reports/spec-audit/ch1/iter.json` の `iterator.api.coverage=1.0`・`iter.generators.entries=15` として記録。`prelude_api_inventory.toml` の `module = "Iter"` ブロックを最新スナップショットへ揃え、`meta.last_updated = "2025-12-22 / WBS 3.1c-F1-4/5"` に更新した。実行ログは `reports/spec-audit/ch0/links.md#iter-generators` と `docs/notes/core-library-outline.md#iter-f1-生成-api-監査ログ` に転記し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 表で `iterator.api.coverage` を追跡できるようにした。 |
##### 3.3c 変換・バッファアダプタの Rust 実装（WBS 3.1c-F2）
- `compiler/rust/runtime/src/prelude/iter/adapters.rs` を追加し、`Iter::map`/`filter`/`filter_map`/`flat_map`/`scan`/`take`/`drop`/`enumerate`/`zip`/`buffered` を `IteratorAdapter` 構造体として実装する。`buffered` は `VecDeque<T>` を内部キューに採用し、`buffered(capacity: usize)` 呼び出し時に確保したメモリを `EffectLabels::mem_bytes` に書き込んで `collect-iterator-audit` が `iterator.effect.mem_buffered` を集計できるようにする。
- `Iter::buffered` は `BufferStrategy::{DropOldest,Grow}` を受け取り、`diagnostic.extensions["iterator.buffered.strategy"]` と `AuditEnvelope.metadata.iterator.buffer.capacity` を出力。容量超過時は `IterStep::Error(IterBufferError)` を返し、`docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に `R-031 Buffered Iterator Memory` を追記して監視する。
- `map`/`filter`/`flat_map`/`scan` 等の `@pure` アダプタは `IteratorStageProfile::stable` を維持し、`IterState` から呼び出される `AdapterState::poll_next` で親 `EffectLabels` を合成。`buffered` は `effect {mem}` を residual に追加し、`collect-iterator-audit-metrics.py --module iter --section adapters` の JSON スキーマに `iterator.effect.mem_buffered` を追加する。
- `zip`/`enumerate` は Rust 標準 `Iterator` との橋渡しを行い、`Iter::into_std_iter` のラッパーとして提供。`IntoIterator for Iter<T>` 実装を拡張し `adapter.stage` 情報を `reports/spec-audit/ch1/iter.json` に書き出して Phase 3 の Stage KPI を維持する。

###### F2 実装ログ（2025-W47）

- `compiler/rust/runtime/src/prelude/iter/adapters.rs` を新設し、`Iter` の変換系 API をすべて `IteratorAdapter` で共通化。`BufferStrategy`／`IterBufferError`／`IterStep::Error` を導入して `Iter::buffered` の容量超過を `IterError::Buffer` へ正規化し、`EffectSet::with_mem_bytes` と `EffectLabels.mem_bytes` を追加して `iterator.effect.mem_buffered` を算出できるようにした。これに伴い `IterState::effect_labels` と `Collector` 各実装の `EffectLabels` 初期値を更新。
- `compiler/rust/frontend/tests/core_iter_adapters.rs` を追加し、`iter_map_filter_pipeline`／`iter_filter_map_skips_invalid`／`iter_scan_tracks_running_totals`／`iter_take_drop_enumerate`／`iter_zip_pairs_sequences`／`iter_buffered_sets_mem_effects` など 7 ケースで `Iter` アダプタを検証。`cargo test --manifest-path compiler/rust/frontend/Cargo.toml iter_buffered_sets_mem_effects` を実行して `effect_labels.mem=true` / `mem_bytes=2` の測定値を確認済み。
- `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `Iter.map`/`filter_map`/`flat_map` を `rust_status=implemented` へ繰上げ、`filter`/`scan`/`take`/`drop`/`enumerate`/`zip`/`buffered` の在庫行を追加。各エントリでは `compiler/rust/runtime/src/prelude/iter/adapters.rs` と `compiler/rust/frontend/tests/core_iter_adapters.rs` の証跡をリンクし、WBS `3.1c-F2` の進捗記録を更新済み。

##### 3.3d テスト・監査・ドキュメント反映（WBS 3.1c-F3）
1. **テスト拡充**: `compiler/rust/frontend/tests/core_iter_generators.rs` に `from_list`・`range`・`unfold`・`buffered` を組み合わせた 12 ケースを追加。`compiler/rust/frontend/tests/core_iter_effects.rs` では `Iter::buffered` の `effect {mem}`、`Iter::enumerate` の `@pure` を snapshot 化し、`cargo insta test core_iter_generators core_iter_effects` を CI の nightly へ追加する。
2. **監査ログ連携**: `tooling/ci/collect-iterator-audit-metrics.py --module iter --section adapters --output reports/spec-audit/ch1/iter.json` を nightly 実行し、`iterator.effect.mem_buffered`/`iterator.stage.adapter_pass_rate`/`iterator.generator.coverage` を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に記録。`reports/spec-audit/ch0/links.md` には `WBS 3.1c-F3` のコマンド履歴 (`cargo test core_iter_generators`, `collect-iterator-audit --section iter`) と JSON を添付する。
3. **ドキュメント整備**: `docs/notes/core-library-outline.md` に `Iter.from_list`/`Iter.range`/`Iter.buffered` のシナリオ図と `collector` 連携メモを追記し、`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` M1 節で `WBS 3.1c`進捗を参照できる脚注を追加。`docs-migrations.log` には `Iter generators/adapters` の追加を `2025-12-xx WBS 3.1c-F1/F2` として記載する。
4. **リスク更新**: `buffered` の過剰確保や `range` のオーバーフローを `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` の `R-031 Buffered Iterator Memory` `R-032 Range Overflow` として追記し、必要に応じて `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に是正タスクを生成。`scripts/validate-diagnostic-json.sh --pattern iterator` で `iterator.buffered.*`/`iterator.range.*` のキーが出力されることを確認する。

#### 3.3a 生成 API カバレッジのトレース（WBS 3.1c）
- `docs/notes/core-library-outline.md` の `iter_from_list_roundtrip`〜`iter_try_collect_set` の 6 シナリオ表は `Iter` 生成系／Collector 終端の交差点を網羅する証跡であり、現在の進捗と残作業を比較するベースラインとして取り込む。各行は `core_iter_pipeline` テスト→スナップショット→`reports/iterator-stage-summary.md` の KPI →`reports/spec-audit/ch0/links.md` のコマンドログという流れを想定している。
- `compiler/rust/frontend/tests/core_iter_pipeline.rs`（および将来的な `__snapshots__/core_iter_pipeline.snap`）で各パイプラインを生成し、`reports/diagnostic-format-regression.md` に記載された手順で `effect.*`/`collector` 拡張の差分を検出できないことを確認しながら `Iter::from_list` や `Iter::try_fold` の Stage/効果値を固定する。
- `tooling/ci/collect-iterator-audit-metrics.py --module iter --section collectors`（および `collect_metrics` 系の CLI）を nightly に組み込み、`iterator.stage.audit_pass_rate` `collector.effect.mem` `collector.effect.mem_reservation` を `reports/iterator-stage-summary.md` に出力して `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI セクションと同期させる。`reports/spec-audit/ch0/links.md` の `#collector-f2-監査ログ` に加えて `#iterator-f3` 相当の新セクションを設け、`cargo test core_iter_pipeline -- --nocapture` や `collect-iterator-audit-metrics.py --module iter --section collectors --case iter-f3` のコマンド履歴と JSON を並べて記録する。
- `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `module = "Iter"` エントリは `rust_status=implemented` を維持しており、各シナリオのスナップショットと `cargo xtask prelude-audit --section iter --baseline docs/spec/3-1-core-prelude-iteration.md` の出力をもって在庫更新を自動化している。更新結果は `reports/spec-audit/ch1/iter.json` に保存しつつ `reports/spec-audit/ch0/links.md` から参照できるようにする。

#### 3. Iter/Collector 完了条件
- `Iter`/`Collector` API が `cargo xtask prelude-audit --section iter --strict` で欠落 0 件となり、`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `last_updated` が 37 週目の日付に更新されている。
- `collect-iterator-audit-metrics.py` で `iterator.stage.audit_pass_rate = 1.0`、`collector.effect.mem_leak = 0` を達成し、結果を `0-3-audit-and-metrics.md`/`reports/spec-audit/ch0/links.md` の両方に貼り付けたログが存在する。
- `docs/notes/core-library-outline.md` と `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` が Iter/Collector 実装状況のサマリを持ち、`docs-migrations.log` に `Iter` モジュール追加・Collector 階層作成の記録が残っている。
- `compiler/rust/frontend/tests/core_iter_pipeline.rs` `core_iter_collectors.rs` `core_iter_generators.rs` `core_iter_effects.rs` が CI へ追加され、`panic_forbidden.rs` と同じジョブで `RUSTFLAGS="-Zpanic-abort-tests"` を通過する。

### 4. Iter アダプタと終端操作（37-38週目）
**担当領域**: 宣言的データフロー

4.1. `map`/`filter`/`flat_map`/`zip`/`buffered` 等のアダプタを実装し、`effect {mem}` や `effect {mut}` の発生箇所を網羅的にテストする。
4.2. `collect_list`/`collect_vec`/`fold`/`reduce`/`try_fold` など終端操作の実装を行い、`Collector` との連携とエラー伝播経路を検証する。
4.3. パフォーマンス計測ベンチマークを作成し、Rust 実装の Phase 2 ベースライン（`docs/plans/rust-migration/3-2-benchmark-baseline.md`）と比較して ±10% 以内に収束するかを測定し、`0-3-audit-and-metrics.md` に反映する。

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
- [ ] `cargo xtask prelude-audit --strict` が `Option`/`Result` API で差分 0 を返し、結果 JSON を `reports/spec-audit/ch0/links.md` 形式で保存。
- [ ] `cargo test core_prelude_option_result panic_forbidden` が成功し、`scripts/validate-diagnostic-json.sh` を通して `reports/diagnostic-format-regression.md` に再生成ファイルが発生しない。
- [ ] `0-3-audit-and-metrics.md` に `core_prelude.missing_api = 0`、`core_prelude.panic_path = 0` を追加し、`4-5-backward-compat-checklist.md` に fallback プランを登録。
- [ ] `docs/spec/3-0-core-library-overview.md` と `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` へ Option/Result 実装ステータスの脚注リンクを追加し、Phase 3-1 以降の参照経路を確立。

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
