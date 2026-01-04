# 3.4 Core Numeric & Time

> 目的：Reml の数値演算・測定・時間表現を統一し、データ品質 API（Core.Data）や診断ログから一貫したメトリクスを生成できるようにする。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `@pure`, `effect {mem}`, `effect {audit}`, `effect {time}`, `effect {unicode}` |
| 依存モジュール | `Core.Prelude`, `Core.Iter`, `Core.Collections`, `Core.Diagnostics` |
| 相互参照 | [3.7 Core Config & Data](3-7-core-config-data.md), [3.2 Core Collections](3-2-core-collections.md), [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md) |

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
fn median<T: Numeric + Ord>(iter: Iter<T>) -> Option<T>     // `@pure`
fn mode<T: Numeric + Eq + Hash>(iter: Iter<T>) -> Option<T> // `@pure`
fn range<T: Numeric + Ord>(iter: Iter<T>) -> Option<(T, T)> // `@pure`
```

- `Numeric` トレイトは Reml コンパイラが型推論で利用する制約（`where Numeric<T>`）としても定義される。
- `OrderedFloat` は `NaN` を含む比較を全順序化するためのヘルパ。
- 集計系関数は `Iter` に依存し、`Iter.try_fold` ベースで実装される（効果タグは呼び出し元へ転写されない）。

## 2. 統計・データ品質サポート

Core.Data の `ColumnStats` と整合する統計ヘルパを提供する。

```reml
pub type u64
pub type HistogramBucket
pub type LinearModel

pub type HistogramBucketState = {
  bucket: HistogramBucket,
  count: u64,
  sum: Option<Float>
}

fn histogram(iter: Iter<Float>, buckets: List<HistogramBucket>) -> Result<List<HistogramBucketState>, Diagnostic> // `@pure`
fn rolling_average(window: usize, values: Iter<Float>) -> Iter<Float>                                            // `@pure`
fn z_score(value: Float, mean: Float, stddev: Float) -> Option<Float>                                            // `@pure`
fn quantiles(iter: Iter<Float>, points: List<Float>) -> Result<Map<Float, Float>, StatisticsError>             // `@pure`
fn correlation(x: Iter<Float>, y: Iter<Float>) -> Result<Float, StatisticsError>                               // `@pure`
fn linear_regression(points: Iter<(Float, Float)>) -> Result<LinearModel, StatisticsError>                      // `@pure`

pub type StatisticsError = {
  kind: StatisticsErrorKind,
  message: Str,
}

pub enum StatisticsErrorKind = InsufficientData | InvalidParameter | NumericalInstability
```

- `histogram` はバケット境界の妥当性を検証し、不正な区間が含まれる場合 `StatisticsError` を返す。
- 数値的不安定性（オーバーフロー、アンダーフロー、NaN）は自動的に検出され `StatisticsError::NumericalInstability` として報告される。
- `rolling_average` は遅延 `Iter` として実装され、`Core.Collections` の `Vec` を内部バッファに利用する場合 `effect {mem}` が付与される。
- `quantiles` は `points` をソートし `Map` へ格納するため `Core.Collections` を利用する。

## 3. 時間・期間型

```reml
pub type i64
pub type i32
pub type TimeError

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
- `TimeError` は OS からのエラーをラップし、`IntoDiagnostic` トレイト経由で診断システムと連携する。

```reml
pub type Timestamp

pub type TimeError = {
  kind: TimeErrorKind,
  message: Str,
  timestamp: Option<Timestamp>,
  timezone: Option<Str>,
  format_pattern: Option<Str>,
  locale: Option<Str>,
}

pub enum TimeErrorKind = SystemClockUnavailable | InvalidTimezone | TimeOverflow | InvalidFormat

fn time_error_code(kind: TimeErrorKind) -> Str =
  match kind with
  | SystemClockUnavailable -> "TIME_SYSTEM_CLOCK_UNAVAILABLE"
  | InvalidTimezone -> "TIME_INVALID_TIMEZONE"
  | TimeOverflow -> "TIME_OVERFLOW"
  | InvalidFormat -> "TIME_INVALID_FORMAT"

impl IntoDiagnostic for TimeError {
  fn into_diagnostic(self) -> Diagnostic {
    Diagnostic::system_error(self.message)
      .with_code(time_error_code(self.kind))
      .with_metadata("timestamp", self.timestamp)
      .with_metadata("format_pattern", self.format_pattern)
      .with_metadata("locale", self.locale)
  }
}
```

- フォーマット/パースの失敗は `TimeErrorKind::InvalidFormat` として扱い、`format_pattern`・`locale` メタデータを診断および監査ログに必ず付与する。

### 3.1 時刻フォーマット

```reml
pub type Timestamp

enum TimeFormat = Rfc3339 | Unix | Custom(Str)

fn format(ts: Timestamp, fmt: TimeFormat) -> Result<String, Diagnostic> // `effect {unicode}`
fn parse(str: Str, fmt: TimeFormat) -> Result<Timestamp, Diagnostic>    // `effect {unicode}`
```

- `Custom` フォーマットは ICU ベースのパターン。解析失敗時は `TimeError` を経由して `Diagnostic` へ変換される。

### 3.2 タイムゾーンサポート

```reml
pub type Duration
pub type TimeError
pub type Timestamp

pub type Timezone = {
  name: Str,
  offset: Duration,
}

fn utc() -> Timezone                                              // `@pure`
fn local() -> Result<Timezone, TimeError>                         // `effect {time}`
fn timezone(name: Str) -> Result<Timezone, TimeError>             // `effect {time}`
fn convert_timezone(ts: Timestamp, from: Timezone, to: Timezone) -> Result<Timestamp, TimeError> // `@pure`
```

## 4. メトリクスと監査連携

`Core.Diagnostics` で利用する `MetricPoint` 構造体を定義し、数値・期間を統一フォーマットで監査ログへ送出する。

```reml
pub type Timestamp
pub type Uuid
pub type AuditSink

trait IntoMetricValue {
}

pub type MetricPoint<T> = {
  name: Str,
  value: T,
  timestamp: Timestamp,
  tags: Map<Str, Str>
}

fn metric_point<T: IntoMetricValue>(name: Str, value: T) -> MetricPoint<T> // `@pure`
fn attach_audit<T>(mp: MetricPoint<T>, audit_id: Option<Uuid>) -> MetricPoint<T> // `@pure`
fn emit_metric(mp: MetricPoint<Float>, sink: AuditSink) -> Result<(), Diagnostic> // `effect {audit}`
```

- `emit_metric` は AuditSink を介して CLI / LSP / runtime へ転送する。
- `IntoMetricValue` トレイトは `Float`/`Int`/`Duration`/`Timestamp` を実装し、JSON 表現へ変換する際の型情報を保持する。

## 5. 使用例（統計 + メトリクス）

```reml
use Core;
use Core.Numeric;
use Core.Data;

pub type Duration
pub type AuditSink

fn duration_to_ms(d: Duration) -> Float // `@pure`

fn summarize_latency(samples: Iter<Duration>, audit: AuditSink) -> Result<MetricPoint<Float>, Diagnostic> {
  let ms = samples
    |> Iter.map(duration_to_ms)
    |> Iter.collect_vec();

  let stats = quantiles(ms.iter(), List::from([0.95]))?;
  let p95 = stats.get(0.95).unwrap_or(0.0);
  let mean = mean(ms.iter()).unwrap_or(0.0);

  let mp = metric_point("latency.mean", mean)
    |> attach_audit(Some(AuditId::current()));

  emit_metric(metric_point("latency.p95", p95), audit)?;
  Ok(mp)
}
```

- `Duration` からミリ秒へ変換し、`quantiles` と `mean` を利用してメトリクスを生成。
- `emit_metric` で監査ログへ出力しつつ、平均値を呼び出し元へ返す。

## 6. 数値精度と丸め設定

### 6.1 数値精度の制御

```reml
pub type u8
pub type NumericError

trait Numeric<T> {
}

pub enum Precision =
  | Float32
  | Float64
  | Decimal { scale: u8, precision: u8 }
  | Arbitrary

fn with_precision<T>(value: T, precision: Precision) -> Result<T, NumericError>    // `@pure`
fn round_to<T: Numeric>(value: T, places: u8) -> T                                // `@pure`
fn truncate_to<T: Numeric>(value: T, places: u8) -> T                             // `@pure`
```

### 6.2 金融計算向け最適化

```reml
pub type Decimal
pub type CurrencyCode
pub type NumericError

// 金融計算用の高精度 Decimal 型
fn currency_add(a: Decimal, b: Decimal, currency: CurrencyCode) -> Result<Decimal, NumericError> // `@pure`
fn compound_interest(principal: Decimal, rate: Float, periods: u32) -> Result<Decimal, NumericError> // `@pure`
fn net_present_value(cashflows: Iter<Decimal>, rate: Float) -> Result<Decimal, NumericError>    // `@pure`
```

> 関連: [3.7 Core Config & Data](3-7-core-config-data.md), [3.2 Core Collections](3-2-core-collections.md), [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md)
