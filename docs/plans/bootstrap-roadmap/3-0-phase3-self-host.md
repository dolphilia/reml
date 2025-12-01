# 3.0 Phase 3 — Core Library 完成

Phase 3 では、Reml 標準ライブラリ Chapter 3 の正式仕様を Reml 実装へ揃えます。Prelude から Runtime Capability までの各モジュールを仕様と照合し、効果タグ・監査・Capability 契約が一貫して動作する状態を構築します。

## 3.0.0 Rust 実装への再統合
- Phase 2 以降は `docs/plans/rust-migration/` にある計画体系で Rust 版 Reml コンパイラの移植を進めてきましたが、Phase 3 では `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md` の監査完了を合図に元の Bootstrap Roadmap と接続し、Rust 実装を標準仕様に完全整合させる工程へシフトします。
- この段階では OCaml 実装はコード参照用として残し、Dual-write や検証に持ち込むことは避け、Rust 実装を唯一のアクティブな実装として扱います。
- 2-8 で確定した差分リストと監査結果をベースに、3-x 以降の標準ライブラリ計画を Rust の `compiler/rust/` 実装と同期させ、監査済み仕様とのギャップを残さない状態で Phase 3 に進みます。

## 3.0.1 目的
- `Core.Prelude`/`Core.Collections`/`Core.Text`/`Core.Numeric`/`Core.IO`/`Core.Diagnostics`/`Core.Config`/`Core.Runtime` の API を Reml で実装し、仕様書と相互参照が成立した状態で提供する。
- 効果タグと Capability Stage の境界を検証し、Chapter 3 全体の診断・監査連携が [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) と一致するように統合する。
- 標準ライブラリのコード例・サンプル・メトリクスを最新化し、Phase 4 の移行とエコシステム展開に備えたベースラインを整備する。

## 3.0.2 スコープ境界
- **含む**: Core Prelude/Collections/Text/Numeric/IO/Diagnostics/Config/Runtime Capability の実装・テスト・ドキュメント更新、効果タグ・Capability 検証、監査／メトリクスの記録。
- **含まない**: 非同期ランタイム (`3-9`)、プラグイン／DSL 拡張 (`4-x`)、エコシステム仕様 (`5-x`) の本格対応（Phase 4 以降に委譲）。
- **前提条件**: Phase 2 で確定した型クラス・効果システム・診断仕様が利用可能であり、`0-3-audit-and-metrics.md` と `0-4-risk-handling.md` に基準値・リスク管理手順が登録済みであること。
- **実装対象**: Rust 実装（`compiler/rust/` 配下）を第一実装とし、`docs/plans/rust-migration/` に記録された成果を反映して 2-8 監査で認められたカバレッジを維持する。OCaml 実装は差分比較や歴史資料として残すが、Phase 3 の開発フローでは積極的に利用しない。

## 3.0.2a 作業ディレクトリ
- `compiler/ocaml/src` : 標準ライブラリ各モジュールの実装（レガシー参照用）
- `compiler/ocaml/tests` : API ゴールデンテスト・性能ベンチマーク（比較用ゴールデン）
- `examples/` : API 使用例の整理（`examples/algebraic-effects/`, `examples/language-impl-comparison/` 等）
- `docs/spec/3-x` : 仕様本文の更新とリンク整備
- `docs/guides/` : 運用ガイドの同期 (`docs/guides/runtime-bridges.md` など)
- `docs/notes/` : 設計判断・メトリクスの記録 (`docs/notes/core-library-outline.md` ほか)
- `compiler/rust/` : Rust 版標準ライブラリの実装とテストベンチ
- `compiler/rust/tests` : Rust 実装向けの API テスト・監査ゴールデン
- `docs/plans/rust-migration/` : Rust 移植計画と Phase 2-8 の監査結果を参照しながら Bootstrap Roadmap へ戻すハンドオーバー資料

この段階では `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md` で生成された差分リスト・監査スナップショットを定常的に参照し、Phase 3 の各マイルストーンで 2-8 に記録されたリスクと TODO に対する完了コメント・フォローアップを残します。

## 3.0.3 成果物とマイルストーン
| マイルストーン | 内容 | 検証方法 | 期限目安 |
|----------------|------|----------|----------|
| M1: Prelude & Iteration | `Option`/`Result`/`Iter` と Collector を実装し効果タグを整合 | API テスト、効果タグ静的検証、サンプル実行 | Phase 3 開始後 8 週 |
| M2: Collections | 永続・可変コレクションと差分 API を実装 | 構造共有プロパティテスト、`CollectError` シナリオ CI | 開始後 16 週 |
| M3: Text & Unicode | 文字列三層モデル・Unicode 正規化・Builder を実装 | UAX コンフォーマンス、Decode/Encode ストリーミングテスト | 開始後 20 週 |
| M4: Numeric / IO & Path | 統計・時間 API と IO 抽象／Path セキュリティを実装し、依存図（`docs/plans/bootstrap-roadmap/assets/core-numeric-time-dependency-map.drawio`）で管理される Core.Collections/Core.Iter/Core.Diagnostics/Core.Runtime との連携を確認。`examples/core_io/file_copy.reml` / `examples/core_path/security_check.reml` / `tooling/examples/run_examples.sh --suite core_io|core_path` を CI へ導入して `core_io.example_suite_pass_rate` を監視する。 | ベンチマーク ±15% 以内、IO/Path 統合テスト、`core_io.example_suite_pass_rate = 1.0` | 開始後 26 週 |
| M5: Diagnostics & Config | Diagnostic/Audit と Manifest/Schema を統合 | 診断スナップショット、Config Lint、監査ログ比較 | 開始後 30 週 |
| M6: Runtime Capability | Capability Registry と Stage 検証を完成 | Capability テストマトリクス、Manifest 契約検証 | 開始後 34 週 |

### 3.0.3a M1 Prelude & Iteration 進行管理
- `docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` の **WBS 3.1b** に沿って、Collector 契約・標準コレクタ・監査/KPI の 3 レイヤーを Phase 3 `M1` のフォーカスタスクとする。`compiler/rust/runtime/src/prelude/collectors/mod.rs` の骨格、`CollectError`/`CollectOutcome`、`EffectMarker` 付与点の設計が完了しない限り `M1` を Go させない。
- 2025-W36 F1 完了: `compiler/rust/runtime/src/prelude/collectors/mod.rs` を追加し、`Collector` トレイトと `CollectOutcome`/`CollectorStageProfile`/`CollectError` を Rust 実装へ取り込んだ。`collector.effect.mem_reservation`/`collector.effect.reserve`/`collector.effect.finish` の EffectMarker と `Diagnostic.extensions["prelude.collector"]` を `core_prelude` で再輸出済み。
- `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `module = "Collector"` を `F0` 仕様精査完了時点で埋め、`last_updated` を `2025-W37 / WBS 3.1b` に更新する。`cargo xtask prelude-audit --section iter --filter collector --strict` を nightly で実行し、欠落が 0 であるログを `reports/spec-audit/ch0/links.md` に貼り付けてから `M1` Exit を宣言する。[^collector-inventory]
- `tooling/ci/collect-iterator-audit-metrics.py` へ `collector.effect.mem`, `collector.effect.mut`, `collector.error.kind` 列を追加し、`iterator.stage.audit_pass_rate` と同じ CLI（`collect-iterator-audit --section collector`) で収集する。KPI (`collector.effect.mem_leak`, `collector.error.duplicate_key_rate`, `collector.effect.mem_reservation_hits`) を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ登録し、逸脱は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に転記する。
- `compiler/rust/frontend/tests/core_iter_collectors.rs`＋snapshot、`reports/spec-audit/ch0/collector-YYYYMMDD.json`、`reports/iterator-collector-summary.md`、`docs/notes/core-library-outline.md`（Collector セクション）の 4 点セットをレビュー artefact とし、`M1` の品質レビューでは (1) 正常系 3 ケース（List/Vec/Map）、(2) エラー系（`VecCollector::reserve`/`MapCollector` 重複キー/`StringCollector` エンコードエラー）、(3) KPI ログの整合を確認する。
- `docs/plans/rust-migration/3-1-observability-alignment.md` と `docs/spec/3-6-core-diagnostics-audit.md` を参照して `AuditEnvelope.metadata.collector.*` を記述、`docs/plans/bootstrap-roadmap/0-4-risk-handling.md` の `R-027 (Collector メモリ過剰確保)` を WBS 3.1b のリスクトラッキングへ紐付ける。対応策（`reserve` 事前計測、`CollectError::MemoryError` の診断変換、`collect-iterator-audit` のメモリ監査列）は `docs-migrations.log` と本節に日付付きで追記する。
- 2025-W37 F1-1: `compiler/rust/runtime/src/prelude/iter/generators.rs` を追加して `Iter` の `from_state`/`stage_snapshot`/`effect_labels` をここへ移管し、`IterStepMetadata`/`attach_effects` によって効果ラベル注入点を明示。`docs-migrations.log` と `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml`（`module = "Iter"` の `WBS 3.1c-F1` 脚注）にも進捗を記録済み。
- 2025-W37 F1-2: `Iter::from_list`/`Iter::from_result`/`Iter::from_fn` を `ListCollector` ノードと `IterSeed` 設計で実装し、`core_iter_generators.rs` の 3 ケース (`from_list_roundtrip`/`from_result_passthrough`/`from_fn_counter`) を `cargo test` + `cargo insta review` で確定。`collect-iterator-audit --section iter --case from_list|from_result|from_fn` の結果を `reports/spec-audit/ch1/iter.json#audit_cases.*` に保存し、`reports/spec-audit/ch0/links.md#iter-generators` から Go/No-Go 用エビデンスとして参照できるようにした。`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `module = "Iter"` では `last_updated = "2025-12-12 / WBS 3.1c-F1-2"` へ更新し、生成 API が `rust_status=implemented` となったことを明示。
- 2025-W37 F1-3: `Iter::empty`/`Iter::once`/`Iter::repeat`/`Iter::range` を `core_iter_generators.rs` に追加し、`RUSTFLAGS="-Zpanic-abort-tests" cargo test core_iter_generators -- --nocapture` のログと `collect-iterator-audit --section iter --case empty|once|repeat|range` の KPI (`iterator.range.overflow_guard`, `iterator.repeat.flagged`, `iterator.once.length`, `iterator.empty.items`) を `reports/spec-audit/ch1/iter.json` に保存。`reports/spec-audit/ch0/links.md#iter-generators` と `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml (last_updated = "2025-12-16 / WBS 3.1c-F1-3")` へ証跡を結線し、M1 で Iter 生成 API を網羅したと判断できる状態にした。
- 2025-W37 F1-4: `Iter::unfold`/`Iter::try_unfold` 実装を完了し、`collect-iterator-audit --section iter --case unfold|try_unfold` の KPI（`iterator.unfold.depth=8`, `iterator.try_unfold.error_kind="try_unfold"`, `EffectLabels::debug=true`）を `reports/spec-audit/ch1/iter.json#audit_cases.*` に保存。`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml`（`wbs = "3.1c F1-4"`）と `docs/notes/core-library-outline.md#iter-generators-f1-4-設計メモwbs-31c-f1-4` を更新し、Phase 3 M1 判定用の証跡を `reports/spec-audit/ch0/links.md#iter-f1-4` から参照できるようにした。
- 2025-W37 F1-5: `cargo xtask prelude-audit --section iter --baseline docs/spec/3-1-core-prelude-iteration.md --wbs 3.1c-F1-5` を実行し、生成 API 15 件の `iterator.api.coverage=1.0`・`iter.generators.entries=15` を `reports/spec-audit/ch1/iter.json` に保存。`pending_entries` が解消された状態を `reports/spec-audit/ch0/links.md#iter-generators` と `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 表へリンクし、`prelude_api_inventory.toml` の `meta.last_updated = "2025-12-22 / WBS 3.1c-F1-4/5"` を記録して Phase 3 M1 レビューに必要な証跡を確保した。

#### Collector F2 監査ログ

- `reports/spec-audit/ch0/links.md#collector-f2-監査ログ` では `cargo test core_iter_collectors`/`cargo insta review`/`collect-iterator-audit-metrics.py --module iter --section collectors --wbs 3.1b-F2 --output reports/iterator-collector-summary.md`/`scripts/validate-diagnostic-json.sh --pattern collector`/`cargo xtask prelude-audit --wbs '3.1b F2'` のコマンド履歴と KPI（`collector.effect.*`、`collector.error.*`、`iterator.stage.audit_pass_rate`）を記録し、`reports/iterator-collector-summary.md` に 7 ケース分のステージ・効果・エラー値を JSON でまとめている。
- この KPI セットは `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に同期され、`collector.effect.mem=0`／`collector.stage.audit_pass_rate=1.0`／`collector.error.invalid_encoding=0`（正常系）を通過条件とした F2/F3 の品質ゲートとして扱う。`docs/notes/core-library-outline.md#collector-f2-監査ログ` および `#collector-f3-監査ログ` でもイテレータ／コレクタ実装状況として同じログを参照している。
- M1 レビューでは `reports/spec-audit/ch0/links.md#collector-f2-監査ログ` と `#collector-f3-監査ログ` から監査ログを参照し、`Collector` トレイトおよび標準コレクタで `rust_status=implemented` となった API と KPI 再現性を確認する。このセクションは `docs/notes/core-library-outline.md#collector-f2-監査ログ` / `#collector-f3-監査ログ` とのクロスリンクにより、Collector 監査の証跡を 2 方向でたどれるようにしている。
- 2027-03-06 更新: `Iter::collect_*` 終端操作で Collector 監査情報（`Diagnostic.extensions["prelude.collector"]` / `AuditEnvelope.metadata["prelude.collector.*"]`）が欠落しないことを確認するため、`compiler/rust/runtime/src/prelude/iter/terminators.rs` に `fn collect_with<C: Collector>` と `CollectOutcome::audit` を追加し、`cargo test --manifest-path compiler/rust/frontend/Cargo.toml core_iter_terminators` → `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case terminators --source reports/spec-audit/ch1/core_iter_terminators.json --output reports/iterator-collector-metrics.json --require-success` → `scripts/validate-diagnostic-json.sh --pattern iterator.collect reports/spec-audit/ch1/core_iter_terminators.json` のパイプを `reports/spec-audit/ch0/links.md#iter-terminators-h1` に記録した。結果として `collector.effect.mem_reservation`（`collect_vec_reserve` ケース）、`collector.error.invalid_encoding`（`collect_string_invalid` ケース）が Collector 直接呼びと一貫していることを KPI（`iterator.stage.audit_pass_rate=1.0`）とともに `reports/iterator-collector-summary.md`・`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml`・`docs/notes/core-library-outline.md#collector-f2-監査ログ` に反映済み。

[^collector-inventory]: 2025-11-20 時点で `prelude_api_inventory.toml` の `module="Collector"` にトレイト API（`new`/`with_capacity`/`push`/`reserve`/`finish`/`into_inner`）と標準コレクタ（List/Vec/Map/Set/String/Table）の効果タグ・Stage 情報を登録し、`reports/spec-audit/ch0/links.md#collector-f0` に根拠コマンドを記録した。


### 3.0.3b Iter コア構造（WBS 3.1a）進行管理
- `docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` の「WBS 3.1a 実装指針」で規定した F0〜F3 を M1 の第 2 サイクルとして採用する。F0（仕様精査）は `docs/spec/3-1-core-prelude-iteration.md` と `compiler/ocaml/src/constraint_solver.ml` を照合し、`IterStep` が保持する `effect`/`stage`/`capability` 情報と `IteratorDict` の JSON 形式を `docs/notes/core-library-outline.md` に記す。
- F1 では `compiler/rust/runtime/src/prelude/iter/mod.rs` に `Iter`/`IterState`/`IterSeed`/`IterSource`/`IterStep` を追加し、`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` に `module = "Iter"` を登録する。完了時に `docs-migrations.log` と本節へ日付付きログ（例: `2025W36 F1 done`) を追加する。
- F2 は `compiler/rust/frontend/src/typeck/constraint/iterator.rs` を新設して `IteratorDictInfo` を生成し、`collect-iterator-audit-metrics.py` の `iterator.dict` 列へ書き出す。`cargo xtask prelude-audit --section iter --strict` の結果を `reports/spec-audit/ch0/links.md` に貼り付け、`0-3-audit-and-metrics.md` へ `iterator.stage.audit_pass_rate` KPI を登録する。
- F3 では `compiler/rust/frontend/tests/core_iter_pipeline.rs`（6 シナリオ）および `core_iter_effects.rs` の snapshot を `compiler/rust/frontend/tests/snapshots/core_iter_pipeline__core_iter_pipeline.snap`/`core_iter_effects__*.snap` に保存し、`scripts/validate-diagnostic-json.sh --pattern iterator --pattern collector` と `tooling/ci/collect-iterator-audit-metrics.py --section iterator --case pipeline --source reports/spec-audit/ch1/iter.json --output reports/iterator-stage-metrics.json` を組み合わせて `reports/iterator-stage-summary.md` を更新する。`reports/spec-audit/ch0/links.md#iterator-f3` には `cargo +nightly test --manifest-path compiler/rust/frontend/Cargo.toml --test core_iter_pipeline|core_iter_effects` の実行ログと KPI を添付し、逸脱時は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ登録する。
- KPI/出口条件: `cargo xtask prelude-audit --section iter --strict` 差分 0、`collect-iterator-audit-metrics.py` で `iterator.stage.audit_pass_rate = 1.0`、`docs/notes/core-library-outline.md` に Iter 実装メモと診断ログが揃っていること。未達時は M1 マイルストーンを Go/No-Go とし、`Phase 3` 全体レビュー前に是正する。


## 3.0.4 主要タスク

1. **Core Prelude & Iteration** (`3-1`)
   - `Option`/`Result`/`Never` と `Iter` 本体・アダプタ・終端操作を Reml で実装。
   - 効果タグと `@must_use` 属性を静的解析し、Chapter 1 の構文・効果仕様と一致させる。
   - `Collector` 契約を定義し、`Core.Collections`／`Core.Text` から再利用できるよう拡張。
2. **Core Collections** (`3-2`)
   - 永続構造（List/Map/Set）と可変構造（Vec/Cell/Ref/Table）を実装し、構造共有・順序保持・効果タグを検証。
   - `Iter` との相互運用（`collect_*`, `Map.from_iter`）と監査差分 (`ChangeSet`) を整備。
3. **Core Text & Unicode** (`3-3`)
   - 文字列三層モデル（Bytes/Str/String）と `GraphemeSeq`/`TextBuilder` を実装。
   - Unicode 正規化・ケース変換・幅変換 API を `UnicodeError` と診断変換で統合。
   - IO/Diagnostics と連携したストリーミング decode・監査ログ API (`log_grapheme_stats`) を検証。
4. **Core Numeric & Time** (`3-4`)
   - 数値トレイト／統計ヘルパ／Histogram／回帰 API を実装し、`Iter` ベースでテスト。
   - `Timestamp`/`Duration`/`Timezone` とフォーマット／パースを整備し、`Core.IO` と統合。
   - `MetricPoint` と監査メトリクス送出を整備し、`AuditEnvelope` メタデータを共通化。
5. **Core IO & Path** (`3-5`)
   - `Reader`/`Writer` 抽象、ファイル API、バッファリング、IO エラー体系を実装。
   - Path 抽象・セキュリティヘルパ・ファイル監視 (オプション) を整備し、クロスプラットフォーム差異を `TargetCapability` で吸収。
6. **Core Diagnostics & Audit** (`3-6`)
   - `Diagnostic` 構造・`AuditEnvelope`・`TraitResolutionTelemetry` 等を実装。
   - CLI/LSP 出力フォーマット、ステージ別フィルタ・抑制ポリシー、監査ログ記録を統合。
7. **Core Config & Data** (`3-7`)
   - `Manifest`/`Schema`/`ConfigCompatibility` API を実装し、DSL エクスポート情報・Capability Stage を同期。
   - Config Diff・ChangeSet を Diagnostics/Audit に連携し、CLI (`reml config lint/diff`) フローを整備。
8. **Core Runtime & Capability** (`3-8`)
   - `CapabilityRegistry`・`CapabilityHandle`・`verify_capability_stage`・`verify_conductor_contract` を実装。
   - Stage/Capability 情報を Diagnostics/Config/Runtime 各層で共有し、監査イベント (`CapabilityMismatch`) を記録。
9. **横断タスク**
   - API ドキュメント・サンプル・ガイド（`docs/guides/runtime-bridges.md`, `docs/notes/dsl-plugin-roadmap.md` 等）を更新。
   - `0-3-audit-and-metrics.md` へベンチマーク・監査指標を継続記録し、差分理由を明示。

### <a id="iter-adapter"></a>3.0.3a Prelude/Iter WBS 3.1c 連携
- `docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` の `WBS 3.1c` で定義した `Iter` 生成系（`from_list`/`range`/`repeat`/`unfold`）と変換系（`buffered`/`enumerate`/`zip`）は Phase 3 M1 の第一優先タスクとする。Rust 実装では `compiler/rust/runtime/src/prelude/iter/{generators,adapters}.rs` を新設し、`IterState`/`IterSeed`/`IterSource` の 3 層構造を Rust 模式で確立する。
- 生成 API 実装後は `compiler/rust/frontend/tests/core_iter_generators.rs`・`core_iter_pipeline.rs`・`core_iter_effects.rs` を組み合わせて `Iter::from_list |> Iter.collect_list`、`Iter::range |> Iter.take |> Iter.collect_vec`、`Iter::buffered` の `effect {mem}` 計測、および `TryCollectError` の伝播を `collect-iterator-audit-metrics.py` で固定する。最新の snapshot は `compiler/rust/frontend/tests/snapshots/core_iter_pipeline__core_iter_pipeline.snap` / `core_iter_effects__*.snap` に保存されており、`reports/spec-audit/ch1/iter.json`・`reports/spec-audit/ch0/links.md#iterator-f3` で参照できる。
- `Iter::buffered` のメモリ計測と `Iter::range` のオーバーフロー監視を Phase 3 のリスク台帳に追加し、`docs/plans/bootstrap-roadmap/0-4-risk-handling.md` の `R-031 Buffered Iterator Memory`/`R-032 Range Overflow` で追跡する。フォローアップや未解決項目は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に紐付ける。
- `cargo xtask prelude-audit --section iter --baseline docs/spec/3-1-core-prelude-iteration.md` を nightly シナリオに加え、`WBS 3.1c-F1/F2/F3` の進捗（API 完了、アダプタ実装、監査ログ）を `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` と `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 表に反映する。
- Adapter ゴールの第 1 弾（G1: map/filter 立ち上げ）は `compiler/rust/runtime/src/prelude/iter/adapters/map.rs` / `filter.rs` と `compiler/rust/frontend/tests/core_iter_adapters.rs` の 3 ケース（`map_pipeline`/`filter_effect`/`map_filter_chain_panic_guard`）を対象にし、`collect-iterator-audit-metrics.py --section iterator --case map|filter` と `scripts/validate-diagnostic-json.sh --pattern iterator.map --pattern iterator.filter` を同一ジョブで実行する。取得した `iterator.map.latency` / `iterator.filter.predicate_count` を `0-3-audit-and-metrics.md` に登録し、`prelude_api_inventory.toml` と `docs/notes/core-library-outline.md#iter-g1-map-filter` へリンクを張る。`tests/snapshots/core_iter_adapters__core_iter_adapters.snap` を用意し、`filter_effect` の `predicate_calls = 4` と Stage=`stable` を証跡化する。
- Adapter ゴールの第 2 弾（G2: flat_map / zip Stage 適用）は `compiler/rust/runtime/src/prelude/iter/adapters/{flat_map,zip}.rs` と `compiler/rust/frontend/tests/core_iter_adapters.rs::{flat_map_vec,zip_mismatch}` を対象にし、`flat_map` の `effect {mem}`（`EffectLabels::mem_reservation`）と `zip` の `Stage::Stable` + `iterator.error.zip_shorter` を `collect-iterator-audit-metrics.py --section iterator --case flat_map|zip` / `scripts/validate-diagnostic-json.sh --pattern iterator.zip` で検証する。`reports/iterator-flatmap-metrics.json`、`reports/iterator-stage-summary.md`、`reports/diagnostic-format-regression.md#iterator.zip_mismatch` を `0-3-audit-and-metrics.md`、`docs/notes/core-library-outline.md#iter-g2-flat-zip`、`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` と同期し、`docs-migrations.log` に G2 ログを残す。
- Adapter ゴールの第 3 弾（G3: buffered/backpressure）は `compiler/rust/runtime/src/prelude/iter/adapters/buffered.rs` と `compiler/rust/frontend/tests/core_iter_adapters.rs::buffered_window` を対象にし、リングバッファ容量 (`EffectLabels.mem_bytes`) とバックプレッシャ率を `collect-iterator-audit-metrics.py --section iterator --case buffered --output reports/iterator-buffered-metrics.json --require-success` で採取する。`cargo bench -p compiler-rust-frontend iter_buffered` の結果 (`reports/benchmarks/iter_buffered-2027-02-22.json`) を `iterator.mem.window` KPI（`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`）に転記し、±10% の性能目標を監視する。`reports/spec-audit/ch0/links.md#iter-buffered`・`docs/notes/core-library-outline.md#iter-g3-buffered-backpressure`・`docs-migrations.log` に Run-ID `2027-02-22-iter-adapter-g3` を記録して Phase 3 M1 の完了条件へ反映する。
- 2026-02-21 G2 完了: `cargo test core_iter_adapters -- --include-ignored flat_map_vec zip_mismatch` → `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case flat_map --case zip --output reports/iterator-flatmap-metrics.json --secondary-output reports/iterator-zip-metrics.json --require-success` を実行し、`flat_map_vec` の `EffectLabels.mem_reservation_bytes = 3`、`zip_mismatch` の `iterator.error.zip_shorter = 1` / Stage=`Exact("stable")` を KPI 化。`reports/spec-audit/ch1/core_iter_adapters.json`・`reports/spec-audit/ch0/links.md#iter-adapters`・`prelude_api_inventory.toml`・`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`・`docs/notes/core-library-outline.md#iter-g2-flat-zip` を同日に更新し、Phase 3 M1 のアダプタ完了条件を満たした。
- 2027-02-22 G3 完了: `cargo test --manifest-path compiler/rust/frontend/Cargo.toml core_iter_adapters -- --include-ignored buffered_window` → `cargo bench -p compiler-rust-frontend iter_buffered` → `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case buffered --output reports/iterator-buffered-metrics.json --require-success` を同一 CI で実行。`reports/iterator-buffered-metrics.json` に `iterator.mem.window.bytes = 2` / `backpressure.ratio = 0.33` を記録し、Criterion ベンチ（`reports/benchmarks/iter_buffered-2027-02-22.json`）が ±10% 以内 ( +3.8% ) であることを確認した。`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml`、`reports/spec-audit/ch1/core_iter_adapters.json`、`reports/spec-audit/ch0/links.md#iter-buffered`、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`、`docs/notes/core-library-outline.md#iter-g3-buffered-backpressure`、`docs-migrations.log` を Run-ID `2027-02-22-iter-adapter-g3` で更新済み。
- Adapter ゴールの第 4 弾（G4: Adapter KPI & 文書同期）は `cargo xtask prelude-audit --section iter --filter adapter --strict` と `collect-iterator-audit-metrics.py --section iterator --case adapters --source reports/spec-audit/ch1/core_iter_adapters.json --output reports/iterator-adapter-metrics.json --require-success` をセットで実行し、adapter 12 API の欠落が 0 件（`iterator.adapter.coverage = 1.0`）かつ `diagnostic.audit_presence_rate = 1.0` であることを証跡化する。結果ログは `reports/spec-audit/ch0/links.md#iter-adapters-g4` に集約し、KPI は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に追加、Run-ID と参照リンクは `docs/notes/core-library-outline.md#iter-adapter`・`docs-migrations.log` へ転記する。
- 2027-02-24 G4 完了: `cargo xtask prelude-audit --section iter --filter adapter --strict --output reports/spec-audit/ch1/core_iter_adapters.json` → `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case adapters --source reports/spec-audit/ch1/core_iter_adapters.json --output reports/iterator-adapter-metrics.json --require-success` → `scripts/validate-diagnostic-json.sh --pattern iterator.map --pattern iterator.zip reports/spec-audit/ch1/core_iter_adapters.json` を同一ブランチで実施。`reports/spec-audit/ch1/core_iter_adapters.json` の `run_id` を `2027-02-24-iter-adapter-g4` に更新し、`iterator.adapter.coverage = 1.0`、`diagnostic.audit_presence_rate = 1.0`、`iterator.stage.audit_pass_rate = 1.0` を `reports/iterator-adapter-metrics.json` に保存、`docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` §4.a・`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml`・`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` と連携させた。

## 3.0.4a Rust CLI 安定化
- Phase 3 は `reml_frontend` を唯一の CLI とし、`schema_version = "3.0.0-alpha"` を含む診断・監査 JSON（`reports/spec-audit/ch1/use_nested-YYYYMMDD-typeck.json` 等）をベースラインに採用する。Stage/Audit 由来の `stage_trace` と `used_impls` は `typeck/typeck-debug.rust.json` に保管され、Chapter 3 の Capability 監査を直接支援する。
- AST を生成できなかった入力は `typeck.aborted.ast_unavailable` 診断で停止し、`docs/spec/0-3-code-style-guide.md` / `docs/spec/1-1-syntax/examples/README.md` / `docs/spec/3-6-core-diagnostics-audit.md` に記載された CLI 手順と整合する必要がある。Phase 3 の CI/ガイド更新では同じコマンドを利用し、`reports/spec-audit/ch1/typeck-fallback-removal-20251122.md` を参照して検証する。

## 3.0.5 測定と検証
- **API 完全性**: 仕様書に列挙された公開 API が Reml 実装に存在し、効果タグ・属性が一致することを静的チェックと API テストで確認。
- **効果タグ／Capability 整合**: `effect` タグと `CapabilityStage` の組み合わせを検証し、違反時は Diagnostics で再現できることを確認。
- **性能ベンチマーク**: Prelude/Collections/Numeric/IO の代表関数で Rust 実装の Phase 2 ベースライン（`docs/plans/rust-migration/3-2-benchmark-baseline.md` で定義）に対し ±15% 以内を目標に測定し、結果を `0-3-audit-and-metrics.md` に記録。必要に応じて OCaml 実装のデータを参考資料として付録に掲載する。
- **Core.Collections 永続構造**: `compiler/rust/runtime/ffi/src/core_collections_metrics.rs` による `ListPersistentPatch`（1.7158）/`MapPersistentMerge`（1.3903）のピークメモリ比を `docs/plans/bootstrap-roadmap/assets/metrics/core_collections_persistent.csv` に保存し、いずれも入力サイズ比 1.8 以下を満たしていることを Phase 3 M2 の Go/No-Go 条件に追加する。測定コマンドは `cargo run --manifest-path compiler/rust/runtime/ffi/Cargo.toml --features core_prelude --example core_collections_metrics -- docs/plans/bootstrap-roadmap/assets/metrics/core_collections_persistent.csv`。
- **ドキュメント同調**: 仕様書・ガイド・サンプルが更新され、リンク切れ・用語揺れがないことをレビュー。
- **Core.Text サンプル**: `examples/core-text/text_unicode.reml` と `expected/text_unicode.*.golden` を最新版へ保ち、`reports/spec-audit/ch1/core_text_examples-YYYYMMDD.md` で CLI/監査ログを保存する。`docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md#5` の出口条件として扱い、README/ガイドと相互参照させる。
- **Core.Collections ドキュメント**: `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md` §6 でサンプル検証と README/Phase3 概要の更新を完了し、`docs/spec/3-2-core-collections.md` には `Map.from_pairs` の NOTE で `examples/core-collections/usage.reml` への実行例を追加しました。サンプルを貫く API ハイライト（`List.push_front`, `Vec.collect_from`, `Table.insert`, `Cell`/`Ref`）は Phase 3 のドキュメント整合性チェックの中心項目です。
- **監査／診断スナップショット**: `Diagnostic` と `AuditEnvelope` の出力をゴールデンテスト化し、CI で差分を検出。

## 3.0.6 リスクとフォローアップ
- **効果タグの逸脱**: 実装と仕様で効果タグが不一致の場合、クロスレビューとツール支援を追加し、`0-4-risk-handling.md` に改善タスクを登録。
- **Unicode/IO の性能劣化**: UAX コンフォーマンスを優先した結果として性能が不足する場合、キャッシュ・バッファリング戦略の改善や Phase 4 の最適化項目として記録。
- **Config/Capability のルール変更**: Manifest と Capability の整合が難航した場合、Phase 4 の移行計画と連携し、`docs/notes/dsl-plugin-roadmap.md` に暫定運用を明記。
- **テストボリューム増加**: Chapter 3 全体の CI 実行時間が長くなる恐れがあるため、テスト分割・キャッシュ・nightly ジョブを検討。
- **Phase 2 実装との差分**: 型クラス方式や効果システムが Phase 2 結果と異なる場合、差分を `docs/notes/llvm-spec-status-survey.md` にまとめ、Phase 4 の移行判断に備える。

## 3.0.7 2-8 監査からのフォローアップ
- `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md` で記録された差分リスト・未解決リスクを Phase 3 各マイルストーンの「監査結果」欄で参照・完了報告し、必要があれば 2-9 監査補足セクションを新設して Rust 実装固有の検証項目（ランタイム Capability、監査ログ整合、ABI）を明文化します。
- 監査用スナップショット（`reports/spec-audit/`）および `docs/notes/spec-integrity-audit-checklist.md` の TODO を `docs/plans/rust-migration/` の関連ドキュメントと同期させ、Rust 実装の進捗が 2-8 の前提（CI リンクチェック、ガイド参照整合、診断/効果タグの一致）に照らして評価できるようにします。
- 2-8 監査の成果は `docs/plans/rust-migration/overview.md` や `docs/plans/rust-migration/3-0-ci-and-dual-write-strategy.md` にも脚注として残し、Phase 3 以降の標準ライブラリ作業と CI/監査運用の間をつなぎます。

---

Phase 3 完了時点で Reml 標準ライブラリの基盤が整い、Phase 4 ではマルチターゲット互換性検証とエコシステム移行に集中できる状態になる。
