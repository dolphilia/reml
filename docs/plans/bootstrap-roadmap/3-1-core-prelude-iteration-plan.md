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

2.1. `Option`/`Result`/`Never` 型と付随メソッド (`map`/`and_then`/`expect` など) を Reml で実装し、`@must_use` と効果タグを正しく付与する。
2.2. `ensure`/`ensure_not_null` 等のユーティリティを組み込み、診断 (`Diagnostic`) への変換ヘルパと一緒に単体テストを整備する。
2.3. 例外排除ポリシーを検証するため、Rust 実装で `panic`/`abort` を伴う経路を禁止するテストを作成し、期待差分を `0-3-audit-and-metrics.md` へ記録する。必要に応じて OCaml 実装の挙動を参考情報として添付するが、自動比較対象には含めない。

### 3. Iter コア構造と Collectors（36-37週目）
**担当領域**: 遅延列基盤

3.1. `Iter<T>` の内部表現・所有権モデルを実装し、`IntoIter`/`FromIterator` の変換を整える。
3.2. `Collector` トレイトと標準コレクタ (`ListCollector`/`VecCollector`/`MapCollector` 等) を実装し、失敗時エラー型と効果タグの伝播をテストする。
3.3. `Iter::from_fn`/`Iter::once` など生成系ヘルパを実装し、`Iterator` 互換 API の命名・挙動差分を仕様と揃える。

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
