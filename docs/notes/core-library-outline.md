# 4.1 標準ライブラリ章 骨子（フェーズ2）

## 1. Chapter 4 の位置付けと目的
- Chapter 3 は Core.Parse 以外の共通 API を束ね、Reml の「小さく強いコア」をアプリケーション開発へ拡張する枠組みを提供する。
- フェーズ1 の範囲定義で洗い出した Tier0〜Tier3 のモジュールを、章構成に落とし込みレビュー単位を明確化する。
- Config/Data/Runtime など既存章で定義済みの仕様を Chapter 3 配下に再配置済み。

## 2. 章構成ドラフト（レビュー単位）
| セクション | 想定モジュール | 主な内容 | ステータス |
| --- | --- | --- | --- |
| 3.0 | 範囲定義メモ | 設計ゴール・採否基準・優先度の整理 | ✅ 完了（フェーズ1） |
| 4.1 | 章骨子（本ドキュメント） | モジュール一覧、レビュー単位、索引方針 | ✅ 完了（フェーズ2） |
| 4.2 | Core Prelude & Iteration | `Option`/`Result`、`?` 演算子、`match` 補助、イテレータ／パイプ操作の基本 | ✍️ ドラフト執筆中 |
| 4.3 | Core Collections | 不変リスト／マップ／セット、`Vec`/`Cell` 等の可変構造と効果タグの扱い + 使用例 | ✍️ ドラフト執筆中 |
| 4.4 | Core Text & Unicode | `String`/`Str`/`Bytes`/`Grapheme`、正規化・セグメンテーション、Lex との連携 + 使用例 | ✍️ ドラフト執筆中 |
| 4.5 | Core Numeric & Time | 数値ユーティリティ、`Duration`/`Timestamp`、統計補助 API + 使用例 | ✍️ ドラフト執筆中 |
| 4.6 | Core IO & Path | `io` 効果、`defer` 連携、ファイル／ストリーム／パス操作 + 使用例 | ✍️ ドラフト執筆中 |
| 4.7 | Core Diagnostics & Audit | `Diagnostic` モデル、`audit_id`/`change_set` 共有語彙、CLI/LSP 出力整合 + 使用例 | ✍️ ドラフト執筆中 |
| 4.8 | Core Config & Data | 設定スキーマ／データモデリング章の再配置、差分・監査との連携整理 + 使用例 | ✍️ ドラフト執筆中 |
| 4.9 | Core Runtime & Capability Registry | GC capability、メトリクス API、プラグイン／Capability の統合窓口 + 使用例 | ✍️ ドラフト執筆中 |
| 4.10 | Core Async / FFI / Unsafe（将来拡張） | `Future`/`Task`、`ffi` 効果、`unsafe` 境界、互換性ポリシー（調査メモ） | 🧭 ドラフトメモ更新中 |


## 3. 索引用ハイレベルリンク
- Chapter 2（Core.Parse）から Chapter 4 への参照は、`use Core` 経由での導入例とパーサ以外のユーティリティを対比して整理する。【F:2-1-parser-type.md†L1-L9】
- Config/Data/Runtime 既存章の内容は Chapter 3.7/3.8 に移行済み。元ファイル（2-7〜2-9）は削除済み。
- 横断テーマを扱うガイド（設定 CLI、LSP、Runtime、FFI、プラグイン）は対応する Chapter 4 節への逆リンクを設ける想定で README 索引を更新する。【F:guides/config-cli.md†L1-L7】【F:guides/lsp-integration.md†L1-L6】【F:guides/runtime-bridges.md†L1-L6】【F:guides/reml-ffi-handbook.md†L1-L6】【F:guides/DSL-plugin.md†L1-L6】

## 4. 次ステップ（フェーズ3 への引き継ぎ）
1. Tier 0（3.1〜3.4）について、型定義・主要関数シグネチャ・効果タグの仕様ドラフトを起草する。3.1（Prelude & Iteration）はドラフト補強済みであり、3.2（Collections）と3.3（Text & Unicode）は使用例を含むドラフトへ更新したため、次フェーズでは API 仕様の精緻化とテスト指針の策定を行う。
2. Tier 1（3.4〜3.6）で共有語彙 (`Diagnostic`, `audit_id`, `Duration`, `Path` 等) の共通フォーマットを明文化し、ガイドからの参照を誘導する。
3. Config/Data/Runtime の本文再配置時に差分追跡ルール（リネーム方針、旧リンク対応）を明記するためのドラフトテンプレートを作成する。
4. Async/FFI/Unsafe（3.9）については、効果タグと安全境界の互換性調査メモを用意し、レビュー対象とする範囲を確定する。

### Collector F2 監査ログ（WBS 3.1b）

- `../../reports/spec-audit/ch0/links.md#collector-f2-監査ログ` に F2 で実行した 7 ケースの `cargo test`/`cargo insta review`/`collect-iterator-audit-metrics`/`scripts/validate-diagnostic-json.sh`/`cargo xtask prelude-audit` コマンド履歴と KPI 結果（`collector.effect.*`、`collector.error.*`、`iterator.stage.audit_pass_rate`）を列挙しており、`../../reports/iterator-collector-summary.md` への参照を含めてモジュール実装の一貫性を監査ログとして留めている。
- `../../reports/iterator-collector-summary.md` では `collect_list_baseline` の `collector.effect.mem=0`、`collect_vec_mem_reservation>0`、`collect_map_duplicate` の `collector.error.duplicate_key_rate=1`、`collect_string_invalid` の `collector.error.invalid_encoding` などを KPI として JSON 形式で記録し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 表にも同期している。
- この監査ログは `../plans/bootstrap-roadmap/3-0-phase3-self-host.md#collector-f2-監査ログ` でも M1 レビューの根拠資料として参照され、`Collector F2` 実装完了判定のトレースとして機能している。

## 5. WBS 3.1a F0（Iter 構造と solve_iterator）の整合メモ（2025-W36）

- 仕様 3-1 §3（`docs/spec/3-1-core-prelude-iteration.md`）では、`Iter`/`Collector` API に加えて `IteratorDictInfo` が保持すべきメタデータを列挙しており、`StageRequirement::{Exact, AtLeast}`、`CapabilityId`、`source` 型、`effect.stage.iterator.*` の JSON キーが必須とされている。【F:docs/spec/3-1-core-prelude-iteration.md†L200-L215】
- 型推論仕様 1-2 §B.4（`docs/spec/1-2-types-Inference.md`）も同じ辞書情報を診断／監査へ渡す必要を明記している。Rust 実装では `IteratorDictInfo` 生成時に `Diagnostic.extensions["iterator.stage.required"|"actual"|"capability"|"source"]` へ直接転記し、`AuditEnvelope.metadata` にも同一キーで出力する運用を継承する前提。【F:docs/spec/1-2-types-Inference.md†L90-L140】
- OCaml 実装の `solve_iterator`（`compiler/ocaml/src/constraint_solver.ml:400-470`）は `IteratorKind` ごとに `stage_requirement`, `capability`, `stage_actual` を決め打ちし、`Array`/`Slice`/`Iter`/`IteratorState`/`Option`/`Result` の 6 系列を自動解決している。Rust 版でも `IteratorDictInfo` を返す `solve_iterator` 相当層が同じ default を持つ必要がある。
- `capability_for_kind` は `IteratorArrayLike -> "core.iter.array"`, `IteratorCoreIter -> "core.iter.core"`, `IteratorOptionLike -> "core.iter.option"`, `IteratorResultLike -> "core.iter.result"` を返す。Stage 要件は `ArrayLike` のみ `Exact "stable"`、他は `AtLeast "beta"`。`stage_actual` も `ArrayLike=stable`、その他 `beta`（カスタムは `unknown`）。この差分を `EffectMarker` や監査 K/V へ落とし込む設計を Rust 側で維持する。
- `IteratorState` 型を `Core.Iter` 内部で露出させると `solve_iterator` が直接辞書化するため、Rust 実装の `IterState` も公開（または `type IteratorState<T>` alias）して型クラス解決経路を互換にする必要がある。そうしないと `Iterator` 制約付き API（例: `Collector` や `Iter::from_iter`）で stage/capability の監査情報が欠落する恐れがある。
- `IterStep` には `Ready|Pending|Finished` の 3 状態と `EffectSet`（bitflags）が必要と仕様に明記されており、アダプタは `IterState::with_effects` のようなヘルパでタグを合成すべき。`collect-iterator-audit-metrics.py` が読み取るキーは `iterator.effect.mem`, `iterator.effect.mut`, `iterator.effect.debug` を想定しているため、`EffectSet`→診断拡張の変換テーブルを Rust F1/F2 で整備する。
- TODO（F0 exit criteria）:
  1. `IterState`/`IterStep` の公開型が `solve_iterator` の `as_user_type "Iter"` / `"IteratorState"` と一致するよう Rust 側の module path を決定する。
  2. `IteratorKind` 相当を Rust 側で enum 定義し、`capability`/`stage_requirement`/`stage_actual` のテーブルを保持する（OCaml 実装の 1:1 移植）。
  3. `collect-iterator-audit-metrics.py` が期待する JSON キー（`effect.stage.iterator.required|actual|capability|kind|source`）の生成元を `IteratorDictInfo` から `Diagnostic`/`AuditEnvelope` への転写フローとして仕様→実装に反映する設計案を F1 で起草する。

## 6. WBS 3.1a F3 Snapshot/KPI（2025-W37）
- 目的: `core_iter_pipeline` の 6 シナリオを snapshot で固定し、`collect-iterator-audit-metrics.py --module iter --section collectors` の KPI (`iterator.stage.audit_pass_rate = 1.0`, `collector.effect.mem = 0`, `collector.effect.mut = 0`, `collector.error.duplicate_key_rate = 0`) を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ転記する。【F:docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md†L95-L150】
- テスト手順: `cargo test core_iter_pipeline -- --nocapture` → `cargo insta review` で Snapshot (`compiler/rust/frontend/tests/snapshots/core_iter_pipeline__*.snap`) を確定し、`reports/spec-audit/ch0/links.md#iter-f3-snapshotkpi` に結果を貼り付ける。診断ログは `reports/diagnostic-format-regression.md`、監査ログは `reports/iterator-stage-summary.md` に格納。
- KPI 反映: `tooling/ci/collect-iterator-audit-metrics.py --module iter --output reports/iterator-stage-summary.md` を WBS 3.1a F3 の exit 条件に設定。値が 1.0 未満の場合は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ是正タスクを追加し、`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md#303a-m1-prelude--iteration-進行管理` で Go/No-Go を再確認する。
- Collector 実装: `Iter::try_collect(SetCollector)` など `effect {mem}` が絡むシナリオは未実装のため、現時点では KPI の監視ルーチンのみ。Collector 実装完了後に `core_iter_collectors.rs` へ移し、`core_iter_pipeline` では Stage/KPI 確認を担う。

| シナリオ | Pipeline | Collector / Stage | Snapshot | 監査ログ |
| --- | --- | --- | --- | --- |
| `iter_from_list_roundtrip` | `Iter::from_list |> Iter.collect_list` | `ListCollector` / `Exact("stable")` | `compiler/rust/frontend/tests/snapshots/core_iter_pipeline__iter_from_list_roundtrip.snap` | `reports/iterator-stage-summary.md#iter_from_list_roundtrip` |
| `iter_map_utf8` | `Iter::from_list |> Iter::map |> Iter.collect_list` | `VecCollector` / `AtLeast("beta")` | `compiler/rust/frontend/tests/snapshots/core_iter_pipeline__iter_map_utf8.snap` | `reports/iterator-stage-summary.md#iter_map_utf8` |
| `iter_filter_map_cap` | `Iter::filter_map` | `ListCollector` / `AtLeast("beta")` | `compiler/rust/frontend/tests/snapshots/core_iter_pipeline__iter_filter_map_cap.snap` | `reports/iterator-stage-summary.md#iter_filter_map_cap` |
| `iter_flat_map_stage` | `Iter::flat_map` | `VecCollector` / `AtLeast("beta")` | `compiler/rust/frontend/tests/snapshots/core_iter_pipeline__iter_flat_map_stage.snap` | `reports/iterator-stage-summary.md#iter_flat_map_stage` |
| `iter_try_fold_diag` | `Iter::map |> Iter::try_fold` | Result 返却 / `AtLeast("beta")` | `compiler/rust/frontend/tests/snapshots/core_iter_pipeline__iter_try_fold_diag.snap` | `reports/iterator-stage-summary.md#iter_try_fold_diag` |
| `iter_try_collect_set` | `Iter::try_collect(SetCollector)` | `SetCollector` / `Exact("stable")` | `compiler/rust/frontend/tests/snapshots/core_iter_pipeline__iter_try_collect_set.snap` | `reports/iterator-stage-summary.md#iter_try_collect_set` |

- Snapshot/KPI の更新結果は `docs-migrations.log` に「Iter F3 Snapshot/KPI テンプレート追記」として残し、Phase 3-0 `M1` レビューで参照できるようにする。

## 7. WBS 3.1b F0（Collector 契約・エラー体系）の整理（2025-W37）
- `Collector<T, C>` トレイトの 6 API (`new`/`with_capacity`/`push`/`reserve`/`finish`/`into_inner`) はいずれも `docs/spec/3-1-core-prelude-iteration.md†L150-L170` に記載された効果タグをそのまま Rust 実装へ移植する。`new`/`into_inner` は `@pure`、`with_capacity`/`finish` は `effect {mem}`、`push` は `effect {mut}`、`reserve` は `effect {mut, mem}` を宣言し、`IterState` の `EffectSet` と `collect-iterator-audit` の `collector.effect.*` 列で追跡する。ステージは `IteratorKind` と同様に最低 `beta`（`StageRequirement::AtLeast("beta")`）扱いとし、`IteratorDictInfo` へ `iterator.stage.iterator.*` を転写する設計を Rust 版 F1 で保持する。
- 標準コレクタの効果タグとエラー: `ListCollector`/`SetCollector` は `@pure` で `Exact("stable")`、`VecCollector`/`MapCollector`/`TableCollector`/`StringCollector` は `AtLeast("beta")` とし、`CollectError::MemoryError`/`CapacityOverflow`（Vec 系）、`CollectError::DuplicateKey`（Map/Set/Table）、`StringError::InvalidEncoding`（StringCollector）を起点に `Diagnostic.extensions["collector.error.*"]` と `AuditEnvelope.metadata.collector.error.*` へ書き出す。【F:docs/spec/3-1-core-prelude-iteration.md†L188-L253】【F:docs/spec/3-2-core-collections.md†L75-L168】
- `CollectError` と監査の対応: `docs/spec/3-6-core-diagnostics-audit.md†L40-L120` で要求される `change_set`/`audit_id` へのキー情報転写に合わせ、`CollectError::DuplicateKey` は衝突キー、`MemoryError`/`CapacityOverflow` は要求容量、`InvalidEncoding` は `StringCollector` が受信したペイロード断片を `Diagnostic` へ含める。`R-027 (Collector メモリ過剰確保)` の緩和策として、`reserve` 呼び出し前に `EffectMarker::mem_reservation` を付与し `collect-iterator-audit` で `collector.effect.mem_reservation_hits` を計測する設計メモを残した。【F:docs/plans/bootstrap-roadmap/0-4-risk-handling.md†L210-L230】
- `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` は `module="Collector"` にトレイト/標準コレクタ 12 エントリを登録済み（F0 時点）。F2 で `last_updated = "2025-11-25 / WBS 3.1b F2"` へ更新し、テスト/KPI 参照先を備考へ追記した。`reports/spec-audit/ch0/links.md#collector-f0` には F0 の仕様根拠、`#collector-f2` には最新コマンドログを記録して Phase 3 `M1` から参照できるようにした。
- `tooling/ci/collect-iterator-audit-metrics.py --module iter --section collectors --case wbs-31b-f0 --dry-run` を想定パラメータとし、F1 で Rust 実装が揃い次第 `collector.effect.mem`/`collector.effect.mut`/`collector.error.kind` の KPI を即時収集できるよう CLI ノートを `reports/spec-audit/ch0/links.md` へ追記した。

## 8. WBS 3.1b F2（標準コレクタ実装の進行メモ, 2025-W37）
- `docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` §3.2 に F2 のスコープ（List/Vec/Map/Set/String Collector 実装、`core_iter_collectors.rs` テスト、`collect-iterator-audit` KPI）が定義された。`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` も `last_updated = "2025-11-25 / WBS 3.1b F2"` へ更新し、Collector 系エントリの `notes` にテスト/監査の参照先を追記済み。
- 各 Collector の実装ポイント:
  - `ListCollector` は `effect = @pure` を維持し `IteratorStageProfile::stable` を固定。`finish` で `CollectOutcome::audit()` を呼び、`prelude.collector.kind = "list"`/`stage.actual = "stable"` を `Diagnostic` 拡張へ記録する。
- `VecCollector` と `StringCollector` は `EffectMarker::mem_reservation`/`collector.effect.mem` を増分し、`CollectError::MemoryError` および `CollectError::InvalidEncoding(StringError)` を `AuditEnvelope.metadata.collector.error.*` に書き出す。`docs/spec/3-3-core-text-unicode.md†L90-L150` を参照して UTF-8 エラーメッセージ仕様と整合させる。
- `reports/iterator-collector-summary.md` に `collect_list_baseline`/`collect_vec_mem_reservation`/`collect_map_duplicate`/`collect_set_stage`/`collect_string_invalid` の KPI/ステージ/エラー別トレースをまとめ、`collector.error.invalid_encoding` の監視ロジックをテストスナップショット（`core_iter_collectors__collect_string_invalid.snap`）と同期させている。
  - `MapCollector`/`SetCollector` は `StageRequirement::AtLeast("beta")` / `Exact("stable")` を `IteratorDictInfo` へ転写し、重複キーを `AuditEnvelope.metadata.collector.error.key` に残す共通ヘルパ（`collectors/common.rs::check_duplicate`) を利用する。
- Snapshot & KPI 計画:

| テスト ID | Collector | 目的 / 仕様根拠 | Snapshot / KPI | 備考 |
| --- | --- | --- | --- | --- |
| `collect_list_baseline` | List | 永続リスト収束と `effect = ∅` を確認【F:docs/spec/3-1-core-prelude-iteration.md†L237-L253】 | `compiler/rust/frontend/tests/snapshots/core_iter_collectors__collect_list_baseline.snap`、`iterator-collector-summary.md#collect_list_baseline` | `collector.stage.actual = stable` を必須 |
| `collect_vec_mem_error` | Vec | `with_capacity`/`push` の `effect {mut, mem}` と `MemoryError` を監視 | 同 `__collect_vec_mem_error.snap`、`collector.effect.mem` カラム | `R-027` リスク (0-4 risk) への入力データ |
| `collect_map_duplicate` | Map | `CollectError::DuplicateKey` と Stage `beta` を検証【F:docs/spec/3-2-core-collections.md†L75-L88】 | `__collect_map_duplicate.snap`, `iterator-collector-summary.md#collect_map_duplicate` | `AuditEnvelope.metadata.collector.error.key` にキー文字列を残す |
| `collect_set_stage` | Set | `StageRequirement::Exact("stable")` と `effect = ∅` を同期 | `__collect_set_stage.snap` | Stage 不整合は `typeclass.iterator.stage_mismatch` で検出 |
| `collect_string_invalid` | String | UTF-8 正規化・`InvalidEncoding` 報告を検証 | `__collect_string_invalid.snap`, `collector.error.invalid_encoding` KPI | Core Text 章と同じ `StringError` を再利用 |

- コマンドログ: `reports/spec-audit/ch0/links.md#collector-f2` に以下を追記する。
  1. `cargo test core_iter_collectors -- --nocapture`
  2. `cargo insta review --review`
  3. `tooling/ci/collect-iterator-audit-metrics.py --module iter --section collectors --wbs 3.1b-F2 --output reports/iterator-collector-summary.md`
  4. `scripts/validate-diagnostic-json.sh --pattern collector`
- KPI 登録: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ `collector.effect.mem = {list:0, vec:>0}`、`collector.error.duplicate_key_rate = 0`、`iterator.stage.audit_pass_rate = 1.0` を追記し、逸脱した場合は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に ESCALATE を記録する。`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md#303a-m1-prelude--iteration-進行管理` からも F2 の進捗を参照できるよう脚注リンクを追加予定。
