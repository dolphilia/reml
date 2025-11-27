# Core Numeric & Time ギャップログ

Core.Numeric/Core.Time 仕様 (docs/spec/3-4-core-numeric-time.md) と Rust 実装の差分、対応計画、参照リンクを記録する。Phase3 Bootstrap Roadmap と Rust Migration 計画の連携資料として利用する。

## 記入フォーマット
| 日付 | 区分 | 概要 | 影響範囲 | 対応状況 | チケット/リンク |
| --- | --- | --- | --- | --- | --- |

## 最新エントリ
| 日付 | 区分 | 概要 | 影響範囲 | 対応状況 | チケット/リンク |
| --- | --- | --- | --- | --- | --- |
| 2025-11-27 | API 差分 | `Numeric`/`OrderedFloat` トレイトおよび `lerp`/`mean`/`variance` 等の基本ユーティリティが Rust runtime に存在しない。`compiler/rust/runtime/src/numeric/` ディレクトリ自体が未作成で、型推論制約に紐づく実装ポイントが空白。 | `docs/spec/3-4-core-numeric-time.md#1-数値プリミティブとユーティリティ`, `compiler/rust/runtime/src/numeric/*` | Pending | docs/plans/bootstrap-roadmap/assets/core-numeric-time-api-diff.csv |
| 2025-11-27 | 統計/品質 | `histogram`/`rolling_average`/`quantiles` など統計 API、`StatisticsError`/`StatisticsErrorKind` 型が未実装。`Iter`/`Core.Collections` に依存する導線や `effect {mem}` 記録戦略も決まっていない。 | `docs/spec/3-4-core-numeric-time.md#2-統計・データ品質サポート`, `compiler/rust/runtime/src/numeric/{histogram,statistics,error}.rs` | Pending | docs/plans/bootstrap-roadmap/assets/core-numeric-time-api-diff.csv |
| 2025-11-27 | 時刻/タイムゾーン | `Timestamp`/`Duration` 型と `now`/`monotonic_now`/`sleep`/`timezone` 等の Core.Time API が欠落。現状 `compiler/rust/adapter/src/time.rs` の Capability ラッパのみで、Runtime/Diagnostics から時刻を扱うインターフェースが露出していない。 | `docs/spec/3-4-core-numeric-time.md#3-時間・期間型`, `#31-時刻フォーマット`, `#32-タイムゾーンサポート`, `compiler/rust/runtime/src/time/*` | Pending | docs/plans/bootstrap-roadmap/assets/core-numeric-time-api-diff.csv, compiler/rust/adapter/src/time.rs |
| 2025-11-27 | メトリクス/監査 | `MetricPoint`/`IntoMetricValue`/`emit_metric` が未定義。`effect {audit}` を計測して `AuditSink` へ送る経路や Stage 検証ロジックが runtime になく、Phase3 KPI (`MetricPoint` → `AuditEnvelope`) を満たせない。 | `docs/spec/3-4-core-numeric-time.md#4-メトリクスと監査連携`, `docs/spec/3-6-core-diagnostics-audit.md`, `compiler/rust/runtime/src/diagnostics/metric_point.rs` | Pending | docs/plans/bootstrap-roadmap/assets/core-numeric-time-api-diff.csv |
| 2025-11-27 | 精度/金融 | `Precision` 列挙・`with_precision`・`round_to` など丸め API と `currency_add`/`net_present_value` など金融向け Decimal 関連が丸ごと未実装。`NumericError` も未定義のため、仕様 6 章の要件に着手できない。 | `docs/spec/3-4-core-numeric-time.md#6-数値精度と丸め設定`, `#62-金融計算向け最適化`, `compiler/rust/runtime/src/numeric/{precision,finance}.rs` | Pending | docs/plans/bootstrap-roadmap/assets/core-numeric-time-api-diff.csv |

## TODO
- [ ] エントリごとに Responsible/Owner を割り当て、Phase3 スプリント計画に紐付ける。
- [ ] `docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md` の進行状況に合わせて本ログを定期更新する。
