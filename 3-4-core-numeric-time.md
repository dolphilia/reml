# 4.5 Core Numeric & Time（フェーズ3 ドラフト）

Status: Draft（内部レビュー中）

> 目的：Reml の数値演算・測定・時間表現を統一し、データ品質 API（Core.Data）や診断ログから一貫したメトリクスを生成できるようにする。

## 0. ドラフトメタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | Draft（フェーズ3） |
| 効果タグ | `@pure`, `effect {mem}`, `effect {audit}`, `effect {time}` |
| 依存モジュール | `Core.Prelude`, `Core.Iter`, `Core.Collections`, `Core.Diagnostics` |
| 相互参照 | [2.8 データモデリング API](2-8-data.md), [3.2 Core Collections](3-2-core-collections.md), 3.6（Core Diagnostics, 執筆予定） |

## 1. 数値プリミティブとユーティリティ

Reml の数値型は `Int`, `UInt`, `Float`, `Decimal`, `Ratio`, `BigInt` を標準サポートする。`Core.Numeric` はこれらの型に共通するトレイトとユーティリティを提供する。

```reml
trait Numeric<T> {
  fn zero() -> T;
  fn one() -> T;
  fn abs(self: T) -> T;
  fn clamp(self: T, min: T, max: T) -> T;
}

trait OrderedFloat<T> {
  fn is_nan(self: T) -> Bool;
  fn is_infinite(self: T) -> Bool;
  fn total_cmp(self: T, other: T) -> Ordering;
}

fn lerp<T: Numeric + Copy>(start: T, end: T, t: Float) -> T // `@pure`
fn mean<T: Numeric>(iter: Iter<T>) -> Option<T>             // `@pure`
fn variance<T: Numeric>(iter: Iter<T>) -> Option<T>         // `@pure`
fn percentile(iter: Iter<Float>, p: Float) -> Option<Float> // `@pure`
```

- `Numeric` トレイトは Reml コンパイラが型推論で利用する制約（`where Numeric<T>`）としても定義される。
- `OrderedFloat` は `NaN` を含む比較を全順序化するためのヘルパ。
- 集計系関数は `Iter` に依存し、`Iter.try_fold` ベースで実装される（効果タグは呼び出し元へ転写されない）。

## 2. 統計・データ品質サポート

Core.Data の `ColumnStats` と整合する統計ヘルパを提供する。

```reml
pub type HistogramBucketState = {
  bucket: HistogramBucket,
  count: u64,
  sum: Option<Float>
}

fn histogram(iter: Iter<Float>, buckets: List<HistogramBucket>) -> Result<List<HistogramBucketState>, Diagnostic> // `@pure`
fn rolling_average(window: usize, values: Iter<Float>) -> Iter<Float>                                            // `@pure`
fn z_score(value: Float, mean: Float, stddev: Float) -> Option<Float>                                            // `@pure`
fn quantiles(iter: Iter<Float>, points: List<Float>) -> Result<Map<Float, Float>, Diagnostic>                     // `@pure`
```

- `histogram` はバケット境界の妥当性を検証し、不正な区間が含まれる場合 `Diagnostic` を返す。
- `rolling_average` は遅延 `Iter` として実装され、`Core.Collections` の `Vec` を内部バッファに利用する場合 `effect {mem}` が付与される。
- `quantiles` は `points` をソートし `Map` へ格納するため `Core.Collections` を利用する。

## 3. 時間・期間型

```reml
pub type Timestamp = {
  seconds: i64,
  nanos: i32,
}

pub type Duration = {
  seconds: i64,
  nanos: i32,
}

fn now() -> Result<Timestamp, TimeError>                      // `effect {time}`
fn monotonic_now() -> Result<Timestamp, TimeError>            // `effect {time}`
fn duration_between(start: Timestamp, end: Timestamp) -> Duration // `@pure`
fn add_duration(ts: Timestamp, delta: Duration) -> Timestamp       // `@pure`
fn sleep(duration: Duration) -> Result<(), TimeError>         // `effect {time}`
```

- `Timestamp` は UNIX エポック基準。`Duration` は最大 2^63-1 秒までをサポートする。
- `TimeError` は OS からのエラーを `Diagnostic` と互換の形式で保持する。

### 3.1 時刻フォーマット

```reml
enum TimeFormat = Rfc3339 | Unix | Custom(Str)

fn format(ts: Timestamp, fmt: TimeFormat) -> Result<String, Diagnostic> // `effect {unicode}`
fn parse(str: Str, fmt: TimeFormat) -> Result<Timestamp, Diagnostic>    // `effect {unicode}`
```

- `Custom` フォーマットは ICU ベースのパターン。解析失敗時は `Diagnostic::invalid_value` を返す。

## 4. メトリクスと監査連携

`Core.Diagnostics` で利用する `MetricPoint` 構造体を定義し、数値・期間を統一フォーマットで監査ログへ送出する。

```reml
pub type MetricPoint<T> = {
  name: Str,
  value: T,
  timestamp: Timestamp,
  tags: Map<Str, Str>
}

fn metric_point<T: IntoMetricValue>(name: Str, value: T) -> MetricPoint<T> // `@pure`
fn attach_audit(mp: MetricPoint<T>, audit_id: Option<Uuid>) -> MetricPoint<T> // `@pure`
fn emit_metric(mp: MetricPoint<Float>, sink: AuditSink) -> Result<(), Diagnostic> // `effect {audit}`
```

- `emit_metric` は AuditSink を介して CLI / LSP / runtime へ転送する。
- `IntoMetricValue` トレイトは `Float`/`Int`/`Duration`/`Timestamp` を実装し、JSON 表現へ変換する際の型情報を保持する。

## 5. 使用例（統計 + メトリクス）

```reml
use Core;
use Core.Numeric;
use Core.Data;

fn summarize_latency(samples: Iter<Duration>, audit: AuditSink) -> Result<MetricPoint<Float>, Diagnostic> =
  let ms = samples
    |> Iter.map(|d| d.seconds as Float * 1000.0 + (d.nanos as Float / 1_000_000.0))
    |> Iter.collect_vec();

  let p95 = quantiles(ms.iter(), List::from([0.95]))?.get(&0.95).unwrap_or(0.0);
  let mean = mean(ms.iter()).unwrap_or(0.0);

  let mp = metric_point("latency.mean", mean)
    |> attach_audit(Some(AuditId::current()));

  emit_metric(metric_point("latency.p95", p95), audit)?;
  Ok(mp)
```

- `Duration` からミリ秒へ変換し、`quantiles` と `mean` を利用してメトリクスを生成。
- `emit_metric` で監査ログへ出力しつつ、平均値を呼び出し元へ返す。

> 関連: [2.8 データモデリング API](2-8-data.md), [3.2 Core Collections](3-2-core-collections.md), [3.6 Core Diagnostics & Audit（予定）]
