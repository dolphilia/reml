# Reml Backlog (フェーズ4)

> フェーズ1〜3で作成した PoC / 擬似コードの結果を踏まえ、残リスクと未解決課題を整理する。優先度は `P0`（直近対応）/`P1`（次フェーズ）/`P2`（長期）で示す。

## 1. Core.Async / ストリーミング

| 課題 | 優先度 | 詳細 | 参照 |
| --- | --- | --- | --- |
| `StreamDriver` 実装 | P0 | PoC では擬似コードのみ。`StreamOutcome` の `StreamMeta`/`DemandHint` を実コードで検証し、バックプレッシャ計測を確定させる。 | 2-6 実行戦略 §F |
| LSP 連携サンプル | P1 | `Diagnostic.data.stream` を利用する LSP クライアントの実装例が未作成。lsp-sample-client でイベントを可視化する。 | guides/lsp-integration.md §7 |
| `AsyncFeeder` backoff | P1 | `Poll::Pending` の再スケジュール戦略。イベントループごとに最適化が必要。 | 2-1 parser type §J-4 |

## 2. データ品質 DSL / 統計

| 課題 | 優先度 | 詳細 | 参照 |
| --- | --- | --- | --- |
| `QualityRule` 検証実装 | P0 | サンプルコードは未実装。`reml-data quality run` の PoC を作成し JSON スキーマに合わせる。 | 2-8 Data §F, guides/data-model-reference.md §6 |
| `StatsProvider` バックエンド | P1 | 外部倉庫（warehouse）連携 API の設計が未確定。接続契約とエラー処理を追加する。 | 2-8 Data §B |
| プロファイル衝突検知 | P1 | `RunConfig.data_profile` と `QualityProfile` の組合せ検証が不足。CLI で警告を出す仕組みを整備。 | 2-7 Config §G |

## 3. GC Capability / Runtime

| 課題 | 優先度 | 詳細 | 参照 |
| --- | --- | --- | --- |
| GC ポリシー実装 | P0 | `GcCapability` は仕様のみ。Incremental / Generational の実装設計とテストベンチが必要。 | 2-9 runtime.md |
| `gc.stats` 発火ポイント | P0 | ランナー側でメトリクス収集を行う関数を未定義。`RunConfig.gc` と連動させる。 | guides/runtime-bridges.md §10 |
| バリア最適化 | P1 | `write_barrier` の呼出頻度を削減するためのバッチ API 検討。 | 2-9 runtime.md |

## 4. ドキュメント / 互換性

| 課題 | 優先度 | 詳細 | 参照 |
| --- | --- | --- | --- |
| JSON スキーマ同期 | P0 | `QualityReport` / `gc.stats` のバージョンをタグ付けし、互換性テスト（スナップショット）を CI に組み込む。 | guides/data-model-reference.md §6, guides/runtime-bridges.md §10 |
| LSP / CLI 差分テスト | P1 | `Diagnostic.data` の追加フィールドが古いクライアントでどう扱われるかを記録。互換ポリシー説明を README に追加。 | guides/lsp-integration.md §7 |
| シナリオ文書の連携図 | P2 | `scenario-requirements.md` と `scenario-priorities.md` の関連を図示し、主要仕様へのリンクを README に掲載。 | scenario-* docs |

---

更新日: 2025-??-??（次フェーズ開始時に更新）
