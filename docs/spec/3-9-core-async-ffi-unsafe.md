# 3.9 Core Async / FFI / Unsafe

> 目的：Reml の非同期実行 (`Core.Async`)・FFI (`Core.Ffi`)・unsafe ブロック (`Core.Unsafe`) に関する基本方針と効果タグの枠組みを整理し、今後の詳細仕様策定に備える。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `effect {io.async}`, `effect {io.blocking}`, `effect {io.timer}`, `effect {ffi}`, `effect {unsafe}`, `effect {security}`, `effect {audit}` |
| 依存モジュール | `Core.Prelude`, `Core.Iter`, `Core.IO`, `Core.Runtime`, `Core.Diagnostics` |
| 相互参照 | [2.6 実行戦略](2-6-execution-strategy.md), [3.8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md), [guides/runtime-bridges.md](../guides/runtime/runtime-bridges.md), [guides/reml-ffi-handbook.md](../guides/ffi/reml-ffi-handbook.md), [guides/core-unsafe-ptr-api-draft.md](../guides/ffi/core-unsafe-ptr-api-draft.md) |

## 1. Core.Async の枠組み

```reml
pub type Context
pub type SchedulerHandle
pub type Duration
pub type AsyncError

pub type Future<T> = {
  poll: fn(&mut Context) -> Poll<T>,
}

pub enum Poll<T> = Ready(T) | Pending

pub type Task<T> = {
  future: Future<T>,
  scheduler: SchedulerHandle,
}

fn spawn<T>(future: Future<T>, scheduler: SchedulerHandle) -> Task<T>        // `effect {io.async}`
fn block_on<T>(future: Future<T>) -> Result<T, AsyncError>                    // `effect {io.blocking}`
fn sleep_async(duration: Duration) -> Future<()>                             // `effect {io.async, io.timer}`
```

- `Context` は Waker を含む非同期実行コンテキスト。
- `SchedulerHandle` は `Core.Runtime` の Capability Registry から取得する。
- `block_on` は同期ブロックするため `effect {io.blocking}` を要求し、CLI ツールなどで使用する際は注意が必要。
- `timeout` は期限超過時に `AsyncErrorKind::Timeout` を返し、`AsyncError.metadata["timeout"]` に `{ "waited": Duration, "limit": Duration, "origin": Str }` を格納する。これにより 0-1 §1.2 で定義された安全性（監査可能性）を満たし、診断では `async.timeout` コードで統一表示される。
- `AsyncError::timeout_info` / `into_timeout_info` はメタデータを構造化して取得するための標準ヘルパであり、既存コードの `TimeoutError` 依存は `#[deprecated]` の型エイリアス経由で段階的移行できる。

### 1.2 高度な非同期パターン

```reml
pub type Future<T>
pub type Duration
pub type AsyncError

fn join<T, U>(future1: Future<T>, future2: Future<U>) -> Future<(T, U)>         // `effect {io.async}`
fn select<T>(futures: List<Future<T>>) -> Future<(usize, T)>                   // `effect {io.async}`
fn timeout<T>(future: Future<T>, duration: Duration) -> Future<Result<T, AsyncError>> // `effect {io.async}`
fn retry<T>(future: () -> Future<T>, policy: RetryPolicy) -> Future<T>         // `effect {io.async}`

pub type RetryPolicy = {
  max_attempts: u32,
  backoff: BackoffStrategy,
  should_retry: (AsyncError) -> Bool,
}

pub enum BackoffStrategy = Linear(Duration) | Exponential { base: Duration, max: Duration } | Custom((u32) -> Duration)

pub type TimeoutInfo = {
  waited: Duration,
  limit: Duration,
  origin: TimeoutOrigin,
}

pub enum TimeoutOrigin = UserDeadline | RuntimeDefault | Capability(Str) | External(Str)

fn AsyncError::timeout_info(&self) -> Option<&TimeoutInfo>
fn AsyncError::into_timeout_info(self) -> Option<TimeoutInfo>

// deprecated: AsyncError::Timeout へ統一されたため
pub type TimeoutError = TimeoutInfo
```

### 1.3 ストリームとアシンクイテレータ

```reml
pub type Future<T>
pub type Iter<T>

pub type AsyncStream<T> = {
  next: fn() -> Future<Option<T>>,
}

fn from_iter<T>(iter: Iter<T>) -> AsyncStream<T>                               // `effect {io.async}`
fn buffer<T>(stream: AsyncStream<T>, size: usize) -> AsyncStream<T>             // `effect {io.async, mem}`
fn map_async<T, U>(stream: AsyncStream<T>, f: (T) -> Future<U>) -> AsyncStream<U> // `effect {io.async}`
fn filter_async<T>(stream: AsyncStream<T>, pred: (T) -> Future<Bool>) -> AsyncStream<T> // `effect {io.async}`
fn collect_async<T>(stream: AsyncStream<T>) -> Future<List<T>>                  // `effect {io.async, mem}`
```

### 1.4 DSLオーケストレーション支援 API

```reml
pub type Serialize
pub type Deserialize
pub type DslSender<T>
pub type DslReceiver<T>
pub type Codec<S, R>
pub type OverflowPolicy
pub type AsyncError
pub type DslSpec<T>
pub type AsyncStream<T>
pub type ExecutionStrategy
pub type ErrorPropagationPolicy
pub type SchedulingPolicy
pub type MemoryLimit
pub type CpuQuota
pub type Json

pub type Channel<Send, Recv> = {
  sender: DslSender<Send>,
  receiver: DslReceiver<Recv>,
  codec: Codec<Send, Recv>,
  buffer_size: usize,
  overflow: OverflowPolicy,
}

fn create_channel<S, R>(buffer_size: usize, codec: Codec<S, R>) -> Result<(DslSender<S>, DslReceiver<R>), AsyncError> // `effect {io.async}`
fn merge_channels<T>(receivers: List<DslReceiver<T> >) -> DslReceiver<T>                                            // `effect {io.async}`
fn split_channel<T>(receiver: DslReceiver<T>, predicate: (T) -> Bool) -> (DslReceiver<T>, DslReceiver<T>)           // `effect {io.async}`
fn with_execution_plan<T>(dsl: DslSpec<T>, plan: ExecutionPlan) -> DslSpec<T>                                     // `effect {io.async}`
fn with_plan<T>(stream: AsyncStream<T>, plan: ExecutionPlan) -> AsyncStream<T>                                    // `effect {io.async}`
fn with_resource_limits<T>(dsl: DslSpec<T>, limits: ResourceLimitSet) -> DslSpec<T>                               // `effect {io.async}`

pub type ExecutionPlan = {
  strategy: ExecutionStrategy,
  backpressure: BackpressurePolicy,
  error: ErrorPropagationPolicy,
  scheduling: SchedulingPolicy,
}

pub type ResourceLimitSet = {
  memory: Option<MemoryLimit>,
  cpu: Option<CpuQuota>,
  annotations: Map<Str, Json>,
}

fn ResourceLimitSet::new(memory: Option<MemoryLimit>, cpu: Option<CpuQuota>) -> ResourceLimitSet

pub enum BackpressurePolicy = Drop | DropOldest | Buffer(usize) | Block | Adaptive { high_watermark: usize, low_watermark: usize, strategy: AdaptiveStrategy }

pub enum AdaptiveStrategy = DropNewest | SlowProducer | SignalDownstream
```

- `Channel<Send, Recv>` は DSL 間通信を型安全に扱い、コード変換を `Codec<Send, Recv>` へ委譲する。
- `ExecutionPlan` は `conductor` の `execution { ... }` ブロックと 1:1 で対応し、スケジューラーへ渡す実行ポリシーを保持する。
- `with_execution_plan` は DSL 定義時に計画を合成するコンビネータであり、バックプレッシャー制御やエラー隔離を `Core.Async` ランタイムへ伝える。
- `with_plan` は `AsyncStream` に実行計画を適用し、上流で構築した `ExecutionPlan` の `strategy`/`backpressure`/`error`/`scheduling` 設定をストリームの実行器と共有する。適用時に `ExecutionPlan::validate_capabilities` を呼び出し、対応するスケジューラ Capability が不足している場合は即座に `AsyncErrorKind::InvalidConfiguration` を返す。
- `ResourceLimitSet` は `with_resource_limits` 経由で DSL ノードへ適用され、3.5 §9 の `MemoryLimit` / `CpuQuota` を保持する。`ResourceLimitSet::new` は `annotations = {}` を既定化し、typed 値のみを指定したいケースで簡潔に構築できる。`annotations` はベンダー固有拡張（IO 制限や GPU クォータ等）を JSON で記録し、監査ログにそのまま渡す用途に限定する。
- `with_execution_plan` / `with_resource_limits` で指定した情報は `Runtime::execution_scope` によって統合され、未指定項目は `RunConfig.extensions["runtime"].resource_limits` を既定値として補完する。スコープは `ResourceLimitSet` を正規化した `ResourceLimitDigest` を保持し、`ExecutionPlan` とともに 3-6 §6.1 の診断へ転写されるため、0-1 §1.1（性能）と §1.2（安全性）の評価が漏れなく行える。
- 埋め込み DSL の `EmbeddedMode` は `ExecutionPlan` の `strategy` に反映する。`ParallelSafe` のみが `ExecutionStrategy::Parallel` を許容し、`SequentialOnly` は `Serial` に固定する。`Exclusive` は埋め込み区間の実行中に他 DSL のタスクを停止する前提とし、`ExecutionPlan` の整合性検証で違反があれば `async.plan.invalid` を返す。

#### 1.4.1 Codec 契約

```reml
pub type SemVer
pub type Bytes
pub type Json

pub struct Codec<Send, Recv> {
  name: Str,
  version: Option<SemVer>,
  encode: fn(Send) -> Result<Bytes, CodecError>,
  decode: fn(Bytes) -> Result<Recv, CodecError>,
  validate: fn(&Recv) -> Result<(), CodecError>,
}

pub type CodecError = {
  kind: CodecErrorKind,
  message: Str,
  cause: Option<Json>,
}

pub enum CodecErrorKind = EncodeFailed | DecodeFailed | ValidationFailed | UnsupportedVersion
```

- `encode`/`decode` は **純粋** な関数であり、呼び出しに追加の副作用タグは不要である。
- `Bytes` は `Core.Text.Bytes`、`SemVer` は `Core.Numeric.SemVer` を利用する。
- `validate` はデコード後の追加整合チェックに利用し、失敗時は `CodecErrorKind::ValidationFailed` を返す。
- `name` と `version` は監査ログおよび互換性照合に使用され、`version` の不一致は `CodecErrorKind::UnsupportedVersion` で報告する。

#### 1.4.2 Channel 契約

- `create_channel` は `buffer_size > 0` を要求し、違反した場合は `AsyncErrorKind::InvalidConfiguration` を返す。
- `codec.encode` / `codec.decode` がエラーを返した場合、`AsyncErrorKind::CodecFailure` として伝播する。
- `OverflowPolicy::Buffer(n)` は `n >= buffer_size` を禁止し、違反時には `AsyncErrorKind::InvalidConfiguration`。
- `merge_channels` はすべての `DslReceiver` が同一型かつ同一 `Codec` を共有していることを前提とし、不一致が検出された場合は `AsyncErrorKind::CodecFailure`。
- `split_channel` の `predicate` は副作用を持たないことが推奨され、例外相当の失敗は `AsyncErrorKind::RuntimeUnavailable` にマップされる。

#### 1.4.3 ExecutionPlan の整合性

```reml
pub type Duration
pub type RetryPolicy
pub type SchedulerConfig

pub enum ExecutionStrategy = Serial | Parallel { max_concurrency: Option<usize> } | Streaming

pub enum ErrorPropagationPolicy = FailFast | Isolate { circuit_breaker: Option<Duration> } | Retry { policy: RetryPolicy }

pub enum SchedulingPolicy = Auto | Explicit(SchedulerConfig)
```

- `ExecutionPlan.strategy` で `Parallel` を指定する場合、`max_concurrency` に `Some(0)` を設定することは禁止とし `AsyncErrorKind::InvalidConfiguration` を返す。
- `ErrorPropagationPolicy::Retry` は `RetryPolicy.max_attempts >= 1` を要求し、違反時は即座にエラーを返す。
- `SchedulingPolicy::Explicit` を選択する場合、`SchedulerConfig` は `default_scheduler_config()` の制約（`worker_threads` が 1 以上、`max_blocking_threads <= worker_threads`）を満たす必要がある。同条件違反時は `AsyncErrorKind::InvalidConfiguration`。
- `BackpressurePolicy::Adaptive` は `high_watermark > low_watermark` かつ `low_watermark >= 1` を要求し、閾値の逆転やゼロ指定は静的検証段階でビルドを停止する。`BackpressurePolicy::Buffer(n)` も `n >= 1` を必須とし、`Adaptive.strategy = SlowProducer` を選択する場合は `ExecutionStrategy::Streaming` と併用したときのみ許可される。
- `ResourceLimitSet.memory` と `ResourceLimitSet.cpu` はビルド時に `MemoryLimit::resolve` / `CpuQuota::normalize` を実行し、物理メモリ・論理コア数が不明な場合や閾値超過時は `AsyncErrorKind::InvalidConfiguration` を生成する。相対指定（`Relative`・`Fraction`）を利用する場合は、`RunConfig.extensions["runtime"].resource_limits` に基準値が存在することを要求する。
- `RunConfig.extensions["runtime"].resource_limits` は `ResourceLimitSet` と同じ構造体を保持し、Conductor DSL と CLI/LSP が同一の正規化結果を共有する。CLI は `MemoryLimitResolved.hard_bytes` と `CpuQuotaNormalized.scheduler_slots` を監査ログ（3-6 §6.1.2）へ送信し、0-1 §1.2 の安全性レポート要件を満たす。
- コンパイラ (`remlc`) と CLI (`reml lint`, `reml build`) は `ExecutionPlan` を DSL/Conductor マニフェストと合成した段階で静的検証し、上記制約違反や `SchedulerConfig` の矛盾（`worker_threads` 未設定、`max_blocking_threads` の超過、`Parallel` 指定時の `Some(0)` など）をビルドエラーとする。検証は 0-1 §1.1–1.2 で定めた性能・安全性指針に従い、実行前に Backpressure 設定とスケジューラ構成を確定させる。
- `with_plan` は適用対象の `AsyncStream` が内部で `ExecutionPlan.strategy = Streaming` を要求する場合に、`Streaming` 対応スケジューラが存在するかを実行前に照合する。未対応の場合は `AsyncErrorKind::InvalidConfiguration` を返し、`Diagnostic.code = Some("async.plan.unsupported")` と `extensions["async.plan"].missing_capability` を設定して運用監査可能性（0-1 §1.2）を維持する。

#### 1.4.4 診断と監査

- すべてのチャンネル操作は `Diagnostic.domain = Async` とし、`extensions["channel"]` に `name`、`codec`、`buffer_size`、`overflow` を記録することを推奨する。
- `CodecError` が発生した場合は `Diagnostic.code = Some("async.codec.failure")` を用い、`cause` を診断拡張に埋め込む。
- `ExecutionPlan` の整合性エラーは静的検証・実行時に関わらず `Diagnostic.code = Some("async.plan.invalid")` を既定とし、`Diagnostic.severity = Error` で報告する。CLI は `extensions["async.plan"].reason` に検出理由と `plan` スナップショット JSON を添付し、LSP/CI から同一フォーマットで参照できるようにする。
- `with_execution_plan` と `with_plan` によって計画が適用された DSL / ストリームは、運用時診断に `extensions["async.plan"] = { "applied": true, "strategy": plan.strategy.to_string(), "backpressure": plan.backpressure.to_string() }` を追加する。列挙名をそのまま文字列化し、Plan ハッシュ `plan_hash`（`Blake3` 128bit）を併せて記録することで 0-1 §1.1 の性能監査指標を追跡しやすくする。
- `with_plan` がスケジューラ Capability 不足で失敗した場合は `Diagnostic.code = Some("async.plan.unsupported")` を返し、`extensions["async.plan"].missing_capability` に `RuntimeCapabilityId` を格納する。`severity = Error` を既定とし、`audit_id` を保持して監査ログと突合できるようにする。

#### 1.4.5 チャネルメトリクス API

```reml
pub type GaugeMetric
pub type CounterMetric
pub type LatencyHistogram
pub type Duration
pub type Timestamp
pub type ExecutionMetricsScope
pub type DslReceiver<T>
pub type ChannelId
pub type AsyncError
pub type u64

pub type ChannelMetricsHandle = {
  queue_depth: GaugeMetric,
  dropped_messages: CounterMetric,
  producer_latency: LatencyHistogram,
  consumer_latency: LatencyHistogram,
  throughput: CounterMetric,
}

pub type ChannelMetricsSample = {
  queue_depth: usize,
  dropped_messages: u64,
  producer_latency_p95: Duration,
  consumer_latency_p95: Duration,
  throughput_per_sec: f64,
  observed_at: Timestamp,
}

fn channel_metrics<T>(scope: ExecutionMetricsScope, recv: DslReceiver<T>, channel_id: ChannelId, opts: ChannelMetricOptions)
  -> Result<ChannelMetricsHandle, AsyncError>                            // `effect {io.async}`

fn snapshot_channel_metrics(metrics_handle: ChannelMetricsHandle)
  -> Result<ChannelMetricsSample, AsyncError>                            // `effect {io.async}`

pub type ChannelMetricOptions = {
  collect_dropped_messages: Bool,
  collect_latency: Bool,
  collect_throughput: Bool,
}
```

- `ChannelMetricsHandle` は 3.6 §6.1 の `DslMetricsHandle` と同じ `ExecutionMetricsScope` を利用し、`scope.registry()` に対して `channel_id` を名前空間に含めた計測キー（例: `channel.data_pipeline.source.items.queue_depth`）を登録する。
- `channel_id` は `conductor` マニフェストのチャネル識別子（`manifest.conductor.channels[].id`）と一致させ、CLI/LSP の差分表示で人間が追跡しやすいようにする。`scope.node_path` にチャネル経路が追加されるため、Nested DSL でも衝突を避けられる。
- すべてのオプションが既定で `true` のため、監視メトリクスを省略した場合でも `queue_depth`、`dropped_messages`、`producer_latency`、`consumer_latency`、`throughput` を自動収集する。
- `snapshot_channel_metrics` は CLI/テレメトリバッチ収集で利用し、`throughput_per_sec` は `throughput` カウンタのデルタから算出する。`observed_at` のタイムスタンプにより 0-1 §1.1 が求める性能監視を支援する。異常値は `Diagnostic.code = Some("async.channel.backpressure")` を推奨し、`AuditEnvelope.metadata["queue_depth"]` へ数値を添付する。`scope.resolved_limits()` は `ChannelMetricsHandle` 生成時に `ResourceLimitDigest` をキャッシュし、警告診断へ `resource_limits` を自動添付する。
- `collect_dropped_messages=false` を指定した場合でも、水位超過による `Drop` ポリシー発生時には `AsyncErrorKind::Backpressure` を `Diagnostic.domain = Async` とともに記録し、運用時の安全性（0-1 §1.2）を損なわないようにする。

##### Stage 差分診断と Capability 検証

1. `channel_metrics`・`merge_channels` などランタイム起動時に Capability を検証する API は、まず `Runtime.verify_capability_stage("runtime.async", StageRequirement::AtLeast(StageId::Stable))` を呼び出し、成功した `CapabilityHandle` をコンテキストに保持する。
2. `Result::Err(err)` が返り、`err.kind = CapabilityErrorKind::StageViolation` の場合は `Diag.EffectDiagnostic.stage_violation(span, cap_id, err)`（3-6 §2.4.1）を即座に作成し、`AsyncErrorKind::RuntimeUnavailable` へラップして呼び出し側へ伝搬する。`span` は呼び出し元 DSL/Conductor の源情報を指し、`cap_id` は検証対象となった Capability ID である。
3. 生成された診断は `Diagnostic.extensions["effects"]` に `required_stage`・`actual_stage`・`capability_metadata` を含み、`AuditEnvelope.metadata` へ `effect.provider`・`effect.manifest_path` を転写する。これにより 0-1 §1.2 の安全保障レビューが参照する Stage 差分を即時に共有できる。
4. Stage 不在による `CapabilityErrorKind::NotFound` と区別するため、`AsyncError` 側では `err.actual_stage.is_none()` を検証し、`Diagnostics` へ「Capability 未登録」と明示するか `StageViolation` として詳細を示すかを判定する。`StageViolation` で `None` が返るケースはランタイム実装の欠陥と見なして監査ログへ警告を残す。

### 1.5 プラットフォーム適応スケジューラ

```reml
pub type SchedulerConfig

fn default_scheduler_config() -> SchedulerConfig                          // `effect {runtime}`
```

* `platform_info()` は `Core.Runtime` から取得した実行環境を返し、`RuntimeCapability` に応じてスケジューラ構成を切り替えられる。
* `platform_features()` で `RunConfig.extensions["target"].features` と同期したフラグ（例: `feature = "io.blocking.strict"`）を参照し、DSL ごとのバックプレッシャ設定やタスクプールサイズを調整する。
* Core.DSL モジュールはこの関数を利用して `ExecutionPlan` の既定値を決定し、`@cfg` で有効化した機能と矛盾しないようにする。

### 1.6 効果ハンドラによる Async 差し替え

実験的な代数的効果ハンドラを用いることで、`io.async` を発生させる API をテスト用モックへ差し替えたり、能力別に stage を切り替えることができる。

```reml
pub type Console
pub type Text
pub type AsyncError

fn with_console_mock() -> Result<Text, AsyncError>                          // `effect {io.async, audit}`
```

- `@handles(Console)` で捕捉効果を宣言し、ハンドラ内部では `resume` をワンショットで呼び出す。
- `Diagnostic.extensions["effects"].stage = Experimental` を付随情報として付け、`@requires_capability(stage="experimental")` を付与した API からのみ呼び出せるようにする。

### 1.7 Stage 切り替え手順

1. **Stage 設定**: `effect Console : io { ... }` で `stage = Experimental` を宣言し、Capability Registry に登録する。
2. **PoC/テスト**: 実験フラグ `-Zalgebraic-effects` を有効化した環境でハンドラを用いたテストを実施し、`Diagnostic.extensions["effects"].residual = {}` を確認する。
3. **Beta へ昇格**: 実運用で必要なモジュールに `@requires_capability(stage="beta")` を付与し、Capability Registry の設定を更新する。`effects.stage.promote_without_checks` 診断が発生しないことを確認する。
4. **Stable 化**: 監査ログや CLI の互換性チェックを通過したら `stage = Stable` へ更新し、実験フラグ無しでビルドを通す。

`Stage` は 3.6 §1.3 の `EffectsExtension.stage`、3.8 節の Capability stage と同期している。昇格時には `@dsl_export` の `allows_effects` とマニフェスト側 `expect_effects` の更新を忘れないこと。

### 1.8 AsyncError


```reml
pub type Span
pub type Json

pub type AsyncError = {
  kind: AsyncErrorKind,
  message: Str,
  span: Option<Span>,
  cause: List<AsyncErrorLink>,
  metadata: Map<Str, Json>,
}

pub type AsyncErrorLink = {
  kind: AsyncErrorKind,
  message: Str,
  origin: AsyncErrorOrigin,
  span: Option<Span>,
  metadata: Map<Str, Json>,
}

pub enum AsyncErrorOrigin = Task | Channel | Actor | Scheduler | Timer | Capability(Str) | Ffi | Config | External(Str)

pub enum AsyncErrorKind = Cancelled | Timeout | RuntimeUnavailable | InvalidConfiguration | CodecFailure
```

- `span` はエラー発生地点のソース情報を保持する。取得できない場合は 1-1 §B の合成 Span 規約に従い疑似位置を割り当て、診断の主たる位置を欠損させない。
- `cause` は最新の原因を先頭に並べた逆時系列リストであり、Actor -> Channel -> Scheduler のような多段の伝搬を追跡できる。`origin` により分析時のフィルタリングが容易になる。
- `metadata` には `retry_attempt`, `task_id`, `channel`, `scheduler`, `diagnostic_id` 等のキーを格納する。`diagnostic_id` は 3.6 §2.5 の遡及診断連携に利用し、重複する診断生成を防ぐ。
- `AsyncErrorOrigin::Capability(id)` は Capability Registry (3.8 §1.2) に登録された ID を指し、Stage 不整合や権限不足を Async レイヤで可視化する。`External` はホストランタイムやプラグインなど Reml 外部のソースを表す。
- `AsyncError` は `IntoDiagnostic` を実装し、`cause` および `metadata` を 3.6 §2.5 で定義する `Diagnostic.secondary` と `Diagnostic.extensions["async"]` へ写像する。これにより 0-1 §1.2 と §2.2 が求める安全性・説明責任を満たす。

### 1.9 アクターモデルと分散メッセージング {#core-async-actor}

```reml
pub type Uuid
pub type OverflowPolicy
pub type AsyncError
pub type MailboxStats
pub type Counter
pub type Histogram
pub type SchedulerHandle
pub type TransportHandle
pub type CapabilityRegistry
pub type Duration
pub type Any
pub type EffectTag
pub type DiagnosticSpan
pub type Future<T>
pub type ExitStatus

pub type ActorId = Uuid
pub type NodeId = Str

pub type MailboxHandle<Message> = {
  capacity: usize,
  overflow: OverflowPolicy,
  enqueue: fn(Message) -> Result<(), AsyncError>,
  metrics: MailboxStats,
}

pub type TransportMetrics = {
  throughput: Counter,
  latency: Histogram,
}

pub type ActorSystem = {
  scheduler: SchedulerHandle,
  transport: Option<TransportHandle>,
  registry: CapabilityRegistry,
  config: ActorSystemConfig,
}

pub type ActorSystemConfig = {
  mailbox_high_watermark: usize,
  mailbox_low_watermark: usize,
  ask_timeout: Duration,
}

pub type ActorRef<Message> = {
  id: ActorId,
  mailbox: MailboxHandle<Message>,
  system: ActorSystem,
}

pub type ActorContext = {
  self_ref: ActorRef<Any>,
  tags: Set<EffectTag>,
  span: DiagnosticSpan,
}

fn spawn_actor<Message, State>(system: ActorSystem, init: () -> State,
  on_message: fn(Message, State, ActorContext) -> Future<()>)
  -> Result<ActorRef<Message>, AsyncError>                             // `effect {io.async}`

fn send<Message>(target: ActorRef<Message>, message: Message)
  -> Result<(), AsyncError>                                            // `effect {io.async}`

fn ask<Message, Reply>(target: ActorRef<Message>, message: Message,
  timeout: Duration) -> Future<Result<Reply, AsyncError>>              // `effect {io.async, io.timer}`

fn link(l: ActorRef<Any>, r: ActorRef<Any>) -> Result<(), AsyncError>  // `effect {io.async}`
fn monitor(actor: ActorRef<Any>) -> Future<ExitStatus>                 // `effect {io.async}`
```

- `ActorSystem` は `CapabilityRegistry` から `RuntimeCapability::AsyncScheduler` と `RuntimeCapability::ActorMailbox` を必須とし、未登録の場合は `AsyncErrorKind::RuntimeUnavailable` を返す。
- `ActorRef<Message>` はメッセージ型に `Serialize + Deserialize` を要求する。分散環境では `TransportHandle` が `Codec<Bytes>` を用いて透過的にエンコードする。
- メールボックス容量は `ActorSystem.config.mailbox_high_watermark` と `mailbox_low_watermark` を尊重し、0-1-project-purpose.md §1.1 の性能基準（線形スループット）を損なわないようバッチ配送を既定とする。
- `ActorContext.tags` は生成元 DSL の効果タグを保持し、`effect {audit}` が含まれる場合は自動で監査ログ (`Diagnostic.domain = Async`) を送出する。

#### 1.9.1 Mailbox 契約

| 契約項目 | 説明 | 安全対策 |
| --- | --- | --- |
| スループット | 1 actor あたり 1 秒間に 100k メッセージを目標とし、バッチ出力で O(n) を維持する | キュー実装は固定長リングバッファを推奨し、`AsyncErrorKind::InvalidConfiguration` で限度超過を検出 |
| 優先度 | `mailbox.policy = { kind: FIFO | Priority, drop: DropNew | DropOld }` | 優先度変更は `link`/`monitor` が監査タグ `async.mailbox.policy_changed` を発行 |
| バックプレッシャ | 高水位到達時は `send` が `Pending` を返し、`RuntimeCapability::AsyncBackpressure` が無い場合は `DropNew` 強制 | `ask` のタイムアウトは `CapabilityRegistry` の監査経由で `async.ask.timeout` を記録 |

#### 1.9.2 分散トランスポート

```reml
pub type Codec<S, R>
pub type Bytes
pub type TransportMetrics
pub type ActorSystem
pub type AsyncError
pub type NodeId
pub type ActorId
pub type ActorRef<T>

pub type TransportHandle = {
  name: Str,
  codec: Codec<Bytes, Bytes>,
  secure: TransportSecurity,
  metrics: TransportMetrics,
}

fn register_transport(system: &ActorSystem, transport: TransportHandle)
  -> Result<(), AsyncError>                                             // `effect {io.async, audit}`

fn route(system: &ActorSystem, node: NodeId, actor: ActorId)
  -> Result<ActorRef<Bytes>, AsyncError>                                // `effect {io.async}`

pub enum TransportSecurity = None | TLS { alpn: Str, pin: Option<Bytes> }
```

- 分散モードを有効化する場合、`RuntimeCapability::DistributedActor` と `SecurityCapability` の `Network` 許可が必要。欠落時は `AsyncErrorKind::RuntimeUnavailable` を診断 `async.transport.capability_missing` とともに返す。
- `register_transport` は `audit.log("async.transport.register", transport.name)` を必須とし、0-1-project-purpose.md §1.2 に従い暗号化が無い場合は `Diagnostic.severity = Warning` を発行する。
- `route` はリモート mail box へプロキシ `ActorRef<Bytes>` を返す。接続確立まで `Pending` を返し、失敗時は `AsyncErrorKind::Timeout`。

#### 1.9.3 DSL からの利用例

```reml
pub type ActorSystem
pub type ActorRef<T>
pub type AsyncError
pub type Duration
pub type Future<T>
pub type Text

pub enum Message = Greet(Text) | Hello(Text)

fn spawn_greeter(system: &ActorSystem) -> Result<ActorRef<Message>, AsyncError> // `effect {io.async}`
fn greet(system: &ActorSystem, greeter: ActorRef<Message>, name: Text, timeout: Duration)
  -> Future<Result<Message, AsyncError>>                                       // `effect {io.async}`
```

- `actor spec` はコード生成フェーズで上記 API を呼ぶテンプレートを展開し、`Core.Async` が提供するバックプレッシャ制御を透過的に利用する。
- リモート呼び出しの場合は `system.link_remote(greeter, node)` を明示し、`CapabilityRegistry::stage_of(effect {io.async})` が `Stable` であることを確認する。
- LSP は `actor spec` の診断を `Diagnostic.domain = Async` とし、`async.actor.unhandled_message` を未処理パターンの検出に用いる。

#### 1.9.4 Capability 検証手順

1. `ExecutionPlan::validate_capabilities`（3-9 §1.4.3）で `CapabilityRegistry::verify_conductor_contract` を呼び出し、`with_capabilities` から得た `ConductorCapabilityRequirement` が全て満たされているか静的に確認する。検証結果は `AuditEvent::CapabilityMismatch`（3-6 §1.1.1）として監査ログへ送信され、0-1-project-purpose.md §1.2 の安全性指針に沿って欠落をブロックする。
2. ランタイム起動時は `CapabilityRegistry::verify_capability_stage("runtime.async", StageRequirement::AtLeast(StageId::Stable))` を実行し、返された `CapabilityHandle` から `SchedulerHandle::supports_mailbox()` が `true` であることを確認する。Stage 不足は `CapabilityError::StageViolation` と `async.actor.capability_missing` 診断で報告する。
3. `ActorRuntimeCapability` は `verify_capability_stage("runtime.actor", StageRequirement::AtLeast(StageId::Experimental))` で取得し、`StageId::Experimental` の場合は公開 API に `@requires_capability(stage="experimental")` を付与する。Stage が `Beta` 以上であれば属性は任意だが、`@cfg(capability = "runtime.actor")` と同期させる。
4. 分散を有効化する DSL は `../guides/runtime/runtime-bridges.md §11` のチェックリスト（監査・TLS・再接続ポリシー）を満たした上で、`verify_conductor_contract` の結果に基づき `RuntimeCapability::DistributedActor` の Stage を `AtLeast(StageId::Beta)` として要求する。
5. いずれかの検証が失敗した場合は `Diagnostic.code = Some("async.actor.capability_missing")` を返し、Stage 違反であれば `Diag.EffectDiagnostic.stage_violation(span, cap_id, err)`（3-6 §2.4.1）を利用して `extensions["effects"]` に差分を格納する。その他のエラーでも `extensions["capability"].required_stage` / `.actual_stage` を併用し、復旧手順と監査ログの突き合わせを支援する。

#### 1.9.5 Supervisor パターンと再起動戦略

```reml
pub type ActorSystem
pub type ActorRef<T>
pub type Any
pub type AsyncError
pub type EffectTag
pub type BackoffStrategy
pub type NonZeroU16
pub type Duration
pub type Future<T>
pub type AsyncStream<T>
pub type ActorId
pub type Diagnostic
pub type Timestamp
pub type AsyncErrorKind
pub type Uuid
pub type u16

pub type SupervisorSpec = {
  name: Str,
  strategy: RestartStrategy,
  children: List<ChildSpec>,
  health_check: Option<SupervisorHealthCheck>,
  audit_label: Option<Str>,
}

pub type ChildSpec = {
  build: fn(&ActorSystem) -> Result<ActorRef<Any>, AsyncError>,
  policy: ChildRestartPolicy,
  tags: Set<EffectTag>,
  backoff: Option<BackoffStrategy>,
}

pub enum RestartStrategy = OneForOne { budget: RestartBudget }
  | OneForAll { budget: RestartBudget }
  | Temporary

pub type RestartBudget = {
  max_restarts: NonZeroU16,
  within: Duration,
  cooldown: Duration,
}

pub enum ChildRestartPolicy = Permanent | Transient | Temporary

pub type SupervisorHandle = {
  id: Uuid,
  descriptor: SupervisorDescriptor,
  observe: fn() -> AsyncStream<SupervisorEvent>,
  restart: fn(ActorId) -> Result<(), AsyncError>,
  shutdown: fn(Duration) -> Future<Result<(), AsyncError>>,
}

pub type SupervisorEvent = {
  actor: ActorId,
  outcome: SupervisorOutcome,
  restart_count: u16,
  observed_at: Timestamp,
  diagnostic: Option<Diagnostic>,
}

pub enum SupervisorOutcome = Restarted | Escalated | Exhausted | Stopped | Failed(AsyncErrorKind)

pub type SupervisorDescriptor = {
  name: Str,
  strategy: RestartStrategy,
  children: List<ChildDigest>,
}

pub type ChildDigest = {
  actor: ActorId,
  policy: ChildRestartPolicy,
  tags: Set<EffectTag>,
}

fn spawn_supervised(system: ActorSystem, spec: SupervisorSpec)
  -> Result<SupervisorHandle, AsyncError>                                  // `effect {io.async, audit}`

fn supervisor_stats(supervisor: SupervisorHandle) -> SupervisorStats         // `@pure`

pub type SupervisorStats = {
  restarts_in_window: u16,
  window_started_at: Timestamp,
  exhausted: Bool,
}

pub type SupervisorHealthCheck = {
  interval: Duration,
  probe: fn(SupervisorHandle) -> Future<Result<(), AsyncError>>,
}
```

- `spawn_supervised` は `CapabilityRegistry::require(RuntimeCapability::AsyncSupervisor)` を内部で呼び出し、権限欠如時は `AsyncErrorKind::RuntimeUnavailable` と `Diagnostic.code = Some("async.supervisor.capability_missing")` を発行する。監査ログには `audit_label` を `AuditEnvelope.metadata["async.supervisor"]` として保存し、0-1 §1.2 の安全性指針に沿って監査可能性を確保する。
- `RestartBudget` は `max_restarts` 回数（0 は許可しない）と監視期間 `within` を定義し、期間内に閾値を超えた場合は `SupervisorOutcome::Exhausted` を生成する。`cooldown` に達するまでは再起動を抑制し、再起動スパイクが性能要件（0-1 §1.1）を破らないようにする。
- `ChildRestartPolicy::Temporary` の子役者は失敗しても再起動しない。`Transient` は `RestartStrategy::OneForAll` の場合のみ親 supervisor の判断で再起動される。`Permanent` は常に再起動対象であり、`backoff` が指定されていない場合は指数バックオフ (`BackoffStrategy::Exponential`) を既定とする。
- `observe` は `SupervisorEvent` のストリームを返し、CLI や監視ツールがリアルタイムで `async.supervisor.restart` / `async.supervisor.escalation` 診断（3.6 §2.5.1）を受け取れるようにする。`diagnostic` に `Some` が格納されている場合はその診断を `AuditEnvelope` に転写する。
- `SupervisorStats.exhausted = true` の場合、ランタイムは `AuditEvent::AsyncSupervisorExhausted`（3.6 §1.1.1 / §2.5.1 に準拠）を発行し、当該 DSL ノードを `ExecutionPlan` レベルで隔離する。隔離中は `restart` の呼び出しを拒否し、`AsyncErrorKind::InvalidConfiguration` を返す。
- `SupervisorHealthCheck` は `interval` ごとに `probe` を実行し、失敗すると `SupervisorOutcome::Failed` を `diagnostic` 付きで報告する。ヘルスチェック自体は `effect {io.timer}` を内部的に要求し、`probe` が `Ok` を返すまで再起動を試行しない。
- Supervisor で再起動が発生した場合、`SupervisorDescriptor.children` に含まれる `ChildDigest` の `tags` と `SupervisorSpec.children[].tags` を比較し、`effect {audit}` が欠落している子役者に対しては `AsyncErrorKind::InvalidConfiguration` を返す。これにより監査対象外のタスクが無制限に再起動することを防ぐ。


## 2. Core.Ffi の枠組み

```reml
pub type FnPtr<Args, Ret>
pub type VoidPtr
pub type Path
pub type LibraryHandle
pub type FfiError
pub type Span<T>
pub type u8

pub type ForeignFunction = FnPtr<VoidPtr, VoidPtr>

fn bind_library(path: Path) -> Result<LibraryHandle, FfiError>               // `effect {ffi}`
fn get_function(library: LibraryHandle, name: Str) -> Result<ForeignFunction, FfiError> // `effect {ffi}`
fn call_ffi(fn_ptr: ForeignFunction, args: FfiArgs) -> Result<FfiValue, FfiError> // `effect {ffi, unsafe}`

pub type FfiArgs = Span<u8>
pub type FfiValue = Span<u8>
```

### 2.0 バインディング生成と Capability 連携

```reml
pub type LibraryHandle
pub type FfiError
pub type BoundLibrary
pub type ForeignFunction
pub type FfiArgs
pub type FfiValue
pub type FfiType
pub type CallingConvention
pub type FfiSandbox
pub type AuditHandle
pub type RuntimeCapability
pub type Path

fn auto_bind(library: LibraryHandle, name: Str, signature: FfiSignature) -> Result<TypedForeignFn, FfiError> // `effect {ffi}`
fn auto_bind_all(library: LibraryHandle, spec: [FfiBinding]) -> Result<BoundLibrary, FfiError>               // `effect {ffi}`
fn call_with_capability(cap: FfiCapability, symbol: ForeignFunction, args: FfiArgs) -> Result<FfiValue, FfiError> // `effect {ffi, security, audit}`

pub type FfiSignature = { params: [FfiType], return_type: FfiType }
pub type FfiBinding   = { name: Str, signature: FfiSignature, conventions: CallingConvention }
pub type SymbolHandle = { library: LibraryHandle, function: ForeignFunction }
pub type TypedForeignFn = { call: fn(FfiArgs) -> Result<FfiValue, FfiError>, symbol: ForeignFunction, metadata: FfiBinding }
pub type FfiCapability = { call_function: fn(SymbolHandle, FfiArgs) -> Result<FfiValue, FfiError>, sandbox: Option<FfiSandbox>, audit: AuditHandle }
pub type LibraryMetadata = { path: Path, preferred_convention: Option<CallingConvention>, required_capabilities: Set<RuntimeCapability> }
```

- `auto_bind` は署名情報からシリアライザ/デシリアライザを自動生成し、返却された `TypedForeignFn` 経由で型安全な `call` を提供する。
- `auto_bind_all` は複数シンボルを一括登録し、Capability Registry と連携する `BoundLibrary` を構築する。
- `call_with_capability`（および `FfiCapability.call_function`）は [3.8](3-8-core-runtime-capability.md) の `CapabilityRegistry` 経由で取得した権限を通じて FFI 呼び出しを実行し、監査ログやサンドボックスを適用する。

- `FfiError` は OS 依存エラーやシンボル解決失敗をラップ。

```reml
pub type Path

pub type FfiError = {
  kind: FfiErrorKind,
  message: Str,
  library_path: Option<Path>,
  symbol_name: Option<Str>,
}

pub enum FfiErrorKind = LibraryNotFound
  | SymbolNotFound
  | InvalidSignature
  | CallFailed
  | SecurityViolation
  | UnsupportedPlatform
```

### 2.1 ABI とデータレイアウト

- 既定ターゲットは System V AMD64 と Windows x64 を対象とし、将来的な ARM64 / WASM 追加は [guides/llvm-integration-notes.md](../guides/compiler/llvm-integration-notes.md#ターゲット-abi--データレイアウト) で追跡する。
- Reml からエクスポートされる複合型は `repr(C)` と等価な自然境界を保持し、未定義のパディングやフィールド再配置を禁止する。
- FFI 境界で共有する主要レイアウトは次の表に従う。

| 型カテゴリ | ABI 表現 | 備考 |
| --- | --- | --- |
| レコード（構造体・列挙型） | C ABI と等価。フィールド順序はソース宣言通り、アラインはターゲット ABI の自然境界 | `#[repr(C)]` 想定。可変長メンバは末尾に限定 |
| `Text` / `Str` | `{ data: Ptr<u8>, len: i64 }` | `len` は 64bit。UTF-8 を前提に RC カウンタを共有し、引き渡し前に `inc_ref` を行う |
| `Span<T>` / `ForeignBuffer` | `{ ptr: NonNullPtr<T>, len: usize }` | `ptr` は NULL 非許容。所有権は `Ownership` メタデータで示し、`len = 0` でもダングリング不可 |
| 例外・パニック | 伝播禁止。Reml→C/C++ は `abort`、C++ 例外は FFI 内で捕捉・消費する | 異常終了は `FfiErrorKind::CallFailed` へ変換し、戻り値で通知 |

- 上記と異なるレイアウトを要求する場合は `FfiSignature` で明示し、`resolve_calling_convention`（§2.5）と整合させる。

### 2.2 効果タグと unsafe 境界

- すべての FFI 呼び出しは `effect {ffi}` を伴い、`unsafe` ブロック内部でのみ許可する。`call_ffi` / `call_with_capability` は `effect {ffi, unsafe}` を最小要件とし、追加の I/O 効果はシナリオに応じて宣言する。
- `CapabilitySecurity.effect_scope` は `{ffi, audit, security}` を最低限含め、[3-8](3-8-core-runtime-capability.md) §5.2 のステージ検証と一致させる。
- 効果タグと推奨アトリビュートの組み合わせは次の通り。

| シナリオ | 必須効果タグ | 推奨アトリビュート | 備考 |
| --- | --- | --- | --- |
| 同期 FFI 呼び出し | `ffi`, `unsafe` | `@no_blocking` を付与してブロッキング禁止を明示 | CPU バインド処理向け |
| ブロッキング I/O ラッパ | `ffi`, `unsafe`, `io.blocking` | `@no_timer` を付与し、タイマ依存を拒否 | スレッド待機を伴うデータベース等 |
| 非同期ハンドオフ | `ffi`, `unsafe`, `io.async` | `@async_free` を付与してランタイム制約を共有 | `libuv` / `io_uring` 経由の呼び出し |
| タイマ・イベント登録 | `ffi`, `unsafe`, `io.timer` | `@async_free`, `@no_blocking` を併用 | 外部イベントループやスケジューラ連携 |

- 効果タグの整合性検証は [1-3-effects-safety.md](1-3-effects-safety.md#unsafe-ptr-spec) の安全規則に従い、`ForeignCall` 効果を導入する際は `@requires_capability(stage=...)` で Stage 条件を明示する。

### 2.3 効果ハンドラによる FFI サンドボックス（実験段階）

`ffi` 効果を捕捉するハンドラを用意すると、危険なネイティブ呼び出しをテスト用スタブや監査ロガーへ差し替えられる。

```reml
pub type Text
pub type Bytes
pub type FfiError
pub type Request
pub type Response

fn foreign_call(name: Text, payload: Bytes) -> Result<Bytes, FfiError>        // `effect {ffi}`
fn with_foreign_stub(request: Request) -> Result<Response, FfiError>          // `effect {ffi, audit}`
```

- `@handles(ForeignCall)` で捕捉可能な効果を宣言し、`resume` に `Result<Bytes, FfiError>` を渡して元の計算へ戻す。
- Stage が `Experimental` の間は `@requires_capability(stage="experimental")` を併用し、Capability Registry 側で明示的に opt-in した環境でのみこのハンドラを利用できるようにする。
- `effects.handler.unhandled_operation` 診断を避けるため、`ForeignCall` で定義されたすべての `operation` を実装すること。

ステージを `Beta`/`Stable` へ引き上げる際は、Async と同様に `Diagnostic.extensions["effects"].stage` を更新し、`effects.stage.promote_without_checks` が解消されてから Capability Registry とマニフェストの整合を取る（§1.7）。

### 2.4 タイプセーフな FFI ラッパー

```reml
pub type ForeignFunction
pub type FfiArgs
pub type FfiValue
pub type FfiError

fn foreign_fn(lib: Str, name: Str, signature: Str) -> ForeignFunction
fn call_foreign(fn_ptr: ForeignFunction, args: FfiArgs) -> Result<FfiValue, FfiError>
fn decode_result<T>(raw: FfiValue) -> Result<T, FfiError>
```

`ffi::encode_args` / `ffi::decode_result` は `FfiSignature` と互換のシリアライズヘルパで、`Span<u8>` を安全に生成・復元する。低レベル API を直接利用する場合は `span_from_raw_parts` と `CapabilitySecurity.effect_scope` を併用し、境界検査と監査記録を怠らないこと。

### 2.4.1 Core.Ffi.Dsl

> 目的：`reml-bindgen` 生成物や手書き `extern` を DSL で包み、`unsafe` 境界を局所化した安全な FFI 呼び出しを提供する。

```reml
pub type ForeignFunction
pub type FfiError
pub type Ownership

pub enum FfiType = Void | Bool
  | I8 | U8 | I16 | U16 | I32 | U32 | I64 | U64
  | F32 | F64
  | Ptr(FfiType)
  | ConstPtr(FfiType)
  | Struct(FfiStruct)
  | Enum(FfiEnum)
  | Fn(FfiFnSig)

fn int() -> FfiType
fn double() -> FfiType
fn ptr(inner: FfiType) -> FfiType
fn const_ptr(inner: FfiType) -> FfiType

pub type FfiFnSig = {
  params: List<FfiType>,
  returns: FfiType,
  variadic: Bool,
}

pub type FfiStruct = { name: Str, fields: List<FfiField>, repr: FfiRepr }
pub type FfiField = { name: Str, ty: FfiType }
pub enum FfiRepr = C | Transparent | Packed

pub type FfiEnum = { name: Str, repr: FfiIntRepr, variants: List<FfiVariant> }
pub type FfiVariant = { name: Str, value: Option<Int> }
pub enum FfiIntRepr = I8 | U8 | I16 | U16 | I32 | U32 | I64 | U64

pub type FfiLibrary = { name: Str }
pub type FfiRawFn = { symbol: ForeignFunction, signature: FfiFnSig }

pub type FfiWrapSpec = {
  name: Str,
  null_check: Bool,
  ownership: Option<Ownership>,
  error_map: Option<Str>,
}

fn fn_sig(params: List<FfiType>, returns: FfiType, variadic: Bool) -> FfiFnSig
fn bind_library(name: Str) -> Result<FfiLibrary, FfiError>              // `effect {ffi}`
fn FfiLibrary.bind_fn(name: Str, sig: FfiFnSig) -> Result<FfiRawFn, FfiError> // `effect {ffi, unsafe}`
fn wrap<Args, Ret>(raw: FfiRawFn, spec: FfiWrapSpec) -> Result<fn(Args) -> Result<Ret, FfiError>, FfiError> // `effect {ffi}`
```

- `FfiType` は `ffi.int` / `ffi.double` といった定数 DSL ではなく **値としての型**を表し、`Ptr` / `ConstPtr` / `Struct` / `Enum` / `Fn` によって複合型を構成する。
- `FfiLibrary.bind_fn` は `unsafe` 境界を含む低レベル API であり、直接呼び出す場合は `effect {ffi, unsafe}` を明示する。
- `wrap` は `FfiWrapSpec` を元に引数数・戻り値の `null`・`Ownership` 前提を検証し、`effect {ffi}` のみで呼び出せる安全関数を生成する。
- `wrap` は診断キー `ffi.wrap.invalid_argument` / `ffi.wrap.null_return` / `ffi.wrap.ownership_violation` を使用し、監査ログでは `ffi.wrapper.*` を必須記録とする（3-6 §5.1.1）。

#### 2.4.1.1 DSL 利用例

```reml
pub type FfiLibrary
pub type FfiFnSig
pub type FfiWrapSpec
pub type FfiError

fn bind_libm() -> Result<FfiLibrary, FfiError>                         // `effect {ffi}`
fn bind_cos<Args, Ret>(lib: FfiLibrary, sig: FfiFnSig, spec: FfiWrapSpec)
  -> Result<fn(Args) -> Result<Ret, FfiError>, FfiError>               // `effect {ffi}`
```

### 2.5 呼出規約とプラットフォーム適応

```reml
pub type PlatformInfo
pub type LibraryMetadata
pub type FfiError
pub type Path
pub type LibraryHandle
pub type ForeignFunction

pub enum CallingConvention = C | StdCall | FastCall | SysV | WasmSystemV | Custom(Str)

fn resolve_calling_convention(target: PlatformInfo, foreign: LibraryMetadata) -> Result<CallingConvention, FfiError> // `effect {runtime}`
fn link_foreign_library(path: Path, target: PlatformInfo) -> Result<LibraryHandle, FfiError> // `effect {ffi}`
fn with_abi_adaptation(fn_ptr: ForeignFunction, conv: CallingConvention) -> Result<ForeignFunction, FfiError> // `effect {ffi, unsafe}`
```

* 既定では `RunConfig.extensions["target"]` を用いて呼出規約を決定し、`platform_info()`（[3-8](3-8-core-runtime-capability.md)）が提供する実行時情報と突き合わせる。
* ターゲットの `family` が `Windows` かつ `arch = X64` の場合は `StdCall` を採用し、`Unix` ファミリでは `SysV` を既定とする。WASM ターゲットでは `WasmSystemV` を利用し、サポート外の場合は `FfiErrorKind::UnsupportedPlatform` を返す。
* `resolve_calling_convention` は `LibraryMetadata` に含まれる `preferred_convention` を尊重しつつ、実行環境で利用できない場合は `target.config.unsupported_value` 診断を併せて発行する。診断は `Diagnostic.extensions["cfg"].evaluated` にターゲット値とライブラリ要求を記録する。
* `with_abi_adaptation` は必要に応じてシム層を挿入し、レジスタ引数配置やスタック整列を調整する。性能への影響を抑えるため、変換は初回呼び出し時にキャッシュする。

### 2.6 メモリ管理と所有権境界

```reml
pub type Ptr<T>
pub type Layout
pub type FnPtr<Args, Ret>
pub type VoidPtr
pub type Span<T>
pub type FfiError
pub type MutPtr<T>
pub type u8

pub type ForeignPtr<T> = {
  raw: Ptr<T>,
  layout: Option<Layout>,
  release: Option<FnPtr<(VoidPtr,), ()>>,
}

pub type ForeignBuffer = {
  span: Span<u8>,
  release: Option<FnPtr<(VoidPtr,), ()>>,
  ownership: Ownership,
}

pub enum Ownership = Borrowed | Owned | Transferred

fn wrap_foreign_ptr<T>(raw: Ptr<T>, layout: Option<Layout>) -> Result<ForeignPtr<T>, FfiError>      // `effect {unsafe}`
fn borrow_span<T>(raw: Ptr<T>, len: usize) -> Result<Span<T>, FfiError>                             // `effect {unsafe}`
fn acquire_mut_span<T>(raw: MutPtr<T>, len: usize) -> Result<Span<T>, FfiError>                     // `effect {unsafe}`
fn release_foreign_ptr<T>(ptr: ForeignPtr<T>) -> Result<(), FfiError>                               // `effect {unsafe, memory}`
fn transfer_buffer(buffer: ForeignBuffer, release: FnPtr<(VoidPtr,), ()>) -> Result<(), FfiError>   // `effect {unsafe, memory}`
```

- `ForeignPtr<T>` は `Ptr<T>` を内包し、必要に応じて `NonNullPtr<T>` へ昇格して利用する。`layout` には [5-3 Memory Capability プラグイン](5-3-memory-plugin.md) で定義する `Layout` 情報を格納する。
- `ForeignBuffer` は `Span<u8>` と所有権メタデータを保持し、`Ownership::Borrowed` の場合は解放禁止とする。
- `call_ffi` は `unsafe` を要求し、境界で `AuditEnvelope` を付与することが推奨される。`transfer_buffer` では Capability Registry を通じて `MemoryCapability` の監査フックを呼び出す。
- Reml → C へ値を移譲する際は RC カウンタを `inc_ref` で増加させ、ホスト側が保有を終了するときに `reml_release_*`（将来提供予定のラッパ）または `ForeignPtr.release` を呼び出す契約とする。違反時は `UnsafeErrorKind::MemoryLeak` を `FfiErrorKind::CallFailed` へ昇格し、監査ログ `ffi.call.status = "leak"` を記録する。
- C / C++ → Reml で渡されるポインタは `wrap_foreign_ptr` で `Ownership::Borrowed` として包み、ライフタイムが呼び出し中に限定されることを明文化する。恒常的に保持する場合は `Ownership::Transferred` を選び、`release` ハンドラで解放手順を登録する。
- エラーは `FfiError` を起点に `Diagnostic` へ変換し、`Diagnostic.domain = Some(DiagnosticDomain::Runtime)` と `code = Some("ffi.call.failed")` を既定とする。監査テンプレートの `status`（§2.7）と整合させ、CLI/LSP が同一粒度で表示できるようにする。

### 2.7 監査テンプレートと可観測性

- すべての FFI 呼び出しは `audit.log("ffi.call", ...)` を通じて記録し、ログフォーマットは [3-6](3-6-core-diagnostics-audit.md#ffi-呼び出し監査テンプレート) §5.1 に定義するテンプレートへ従う。
- 収集すべきキーと意味は次の通り。

| キー | 必須 | 内容 | 参照 |
| --- | --- | --- | --- |
| `library` | Required | 実行中に解決したライブラリパスまたは識別子 | `LibraryMetadata.path` |
| `symbol` | Required | 呼び出したシンボル名 | `FfiBinding.name` |
| `call_site` | Optional | 呼び出し元ソース位置（`SourceSpan`） | `Diagnostic.primary` |
| `effect_flags` | Required | 実際に付与した効果タグの集合 | §2.2 |
| `latency_ns` | Optional | 呼び出し完了までのナノ秒計測値 | 3-6 §2.5 |
| `status` | Required | `success` / `failed` / `stubbed` などの実行結果 | `FfiErrorKind`, 3-6 §5.1 |

```json
{
  "event": "ffi.call",
  "library": "libcrypto.so",
  "symbol": "EVP_DigestInit_ex",
  "call_site": "core/crypto.reml:218",
  "effect_flags": ["ffi", "unsafe", "io.blocking"],
  "latency_ns": 32050,
  "status": "success"
}
```

- `status = "stubbed"` の場合は §2.3 の効果ハンドラ経由であることを示し、`CapabilityRegistry::stage` が `Experimental`/`Beta` のいずれかを返したかを `Diagnostic.extensions["effects"].stage` に複写する。
- Capability レジストリと連携する場合は、`call_with_capability` の戻り値に含まれる `FfiCapability.audit` を介して上記テンプレートを自動転送し、`CapabilitySecurity` チェックリスト（3-8 §5.2.1）で効果タグ・Stage の整合を検証する。

### 2.8 reml-bindgen（仕様反映セクション）

- `reml-bindgen` は C/C++ ヘッダから `extern` 定義を自動生成する CLI であり、生成物は **低レベル・`unsafe` 前提**とする。
- 設定ファイルは `reml-bindgen.toml` とし、最小構成は次のキーを持つ。
  - `headers`: 対象ヘッダの配列
  - `include_paths`: include パスの配列
  - `defines`: `-D` 相当の定義（任意）
  - `output`: 生成 `.reml` の出力先
  - `manifest`: `bindings.manifest.json` の出力先
  - `exclude`: 除外パターン（正規表現、任意）
- 生成物は次の 2 点を必須とする。
  - `.reml`: `extern "C"` ブロックと `repr(C)` 定義
  - `bindings.manifest.json`: 生成元・型変換・診断・入力ハッシュのメタデータ
- C 型→Reml 型の変換表（一次範囲）は Phase 1 で確定し、`const` / `volatile` / `restrict` は
  `bindings.manifest.json` の `qualifiers` に記録する（型変換自体は `T` に準拠）。
- `ffi.bindgen.*` の診断キーを固定し、未対応型・解析失敗・未解決シンボルは
  `ffi.bindgen.unknown_type` / `ffi.bindgen.parse_failed` / `ffi.bindgen.unresolved_symbol` を使用する。
- 生成結果のレビューでは `bindings.manifest.json` の差分を一次情報とし、
  `.reml` 側の変更は手書きラッパーと分離された領域のみを対象とする。
- 生成ログの形式とレビュー手順は `docs/guides/ffi/reml-bindgen-guide.md` を参照する（ログ例を含む）。

#### 2.8.1 型変換表（確定・一次範囲）

| C 型 | Reml 型 | 補足 |
| --- | --- | --- |
| `bool` | `Bool` | - |
| `char` | `I8` | `signed char` / `unsigned char` は下記を優先 |
| `signed char` | `I8` | - |
| `unsigned char` | `U8` | - |
| `short` | `I16` | - |
| `unsigned short` | `U16` | - |
| `int` | `I32` | - |
| `unsigned int` | `U32` | - |
| `long` | `I64` | LP64 を基準 |
| `unsigned long` | `U64` | LP64 を基準 |
| `long long` | `I64` | - |
| `unsigned long long` | `U64` | - |
| `size_t` | `USize` | - |
| `intptr_t` | `ISize` | - |
| `uintptr_t` | `USize` | - |
| `float` | `F32` | - |
| `double` | `F64` | - |
| `void` | `Unit` | - |
| `char*` | `Ptr<I8>` | 文字列の安全化は Phase 2 で定義 |
| `void*` | `Ptr<Unit>` | - |
| `T*` | `Ptr<T>` | - |
| `struct` | `repr(C)` レコード | フィールド順固定 |
| `enum` | 整数型 | 基底型指定がない場合は `I32` |

- `const` / `volatile` / `restrict` は Reml 型へ反映せず、`bindings.manifest.json` の `qualifiers` に記録する。
- 配列型・可変長引数・関数ポインタは Phase 1 の対象外とし、未対応型診断へ送る。

#### 2.8.2 未対応型の診断キー案

- 未対応型は `ffi.bindgen.unknown_type` を使用し、`bindings.manifest.json` に以下のメタデータを残す。
  - `c_type`: 変換に失敗した C 型表現
  - `reason`: `unsupported_array` / `unsupported_variadic` / `unsupported_fn_ptr` / `unsupported_bitfield` など
  - `hint`: 対応予定（例: `phase2`）または手書きラッパー誘導の短い文言

#### 2.8.3 例の範囲

- `reml-bindgen.toml` の最小構成例（`headers` / `include_paths` / `output` / `manifest`）。
- `bindings.manifest.json` の最小例（型変換と `qualifiers` の記録）。
- 生成された `extern "C"` ブロックの抜粋（1〜2 関数の宣言）。

```json
// bindings.manifest.json（要点）
{
  "version": "0.1",
  "headers": ["openssl/ssl.h"],
  "generated": "generated/openssl.reml",
  "input_hash": "b3a1c9d4b65f1f27",
  "types": [
    { "c": "size_t", "reml": "USize" },
    { "c": "const char*", "reml": "Ptr<I8>", "qualifiers": ["const"] }
  ],
  "diagnostics": [
    { "code": "ffi.bindgen.unknown_type", "symbol": "EVP_MD_CTX" }
  ]
}
```

### 2.9 Core.Ffi.Dsl（仕様反映セクション）

- `Core.Ffi.Dsl` は `reml-bindgen` 生成物の上に **安全な利用レイヤ**を提供する。
- API は `bind_library` / `bind_fn` / `wrap` と型 DSL を中心に構成する。
  - `bind_library`: ライブラリ探索とハンドル取得（`effect {ffi}`）
  - `bind_fn`: 低レベル関数の束縛（`effect {ffi, unsafe}`）
  - `wrap`: `unsafe` な呼び出しを安全 API へ昇格（監査・検証込み）
- 型 DSL は `ffi.int` / `ffi.double` / `ffi.ptr(ffi.char)` のような表現を基本とし、
  `struct` / `enum` については Phase 2 で最小セットを定義する。
- `ffi.wrap` は **引数検証・戻り値検証・NULL チェック**を担当し、
  失敗時は `Result` を返す。診断キーは `ffi.wrap.invalid_argument` /
  `ffi.wrap.null_return` を基本とする。
- 監査ログは `ffi.call` テンプレートに `wrapper = "ffi.wrap"` を追記し、
  `unsafe` を隠蔽した経路を識別できるようにする。

#### 2.9.1 例の範囲

- `bind_library` / `bind_fn` / `wrap` の最小利用例。
- `unsafe` 直呼びと `ffi.wrap` の対比例。
- `Result` を返す失敗例（`null` 返却の扱い）。

### 2.10 reml build 統合（仕様反映セクション）

`reml build` は `reml.json` に定義された FFI 依存を取り込み、ヘッダ解析・生成・リンクまでを単一フローで統合する。

#### 2.10.1 `reml.json` FFI セクション定義

`ffi` セクションは次のキーを持つ。キーの省略時は空配列 / `None` を既定とする。

| キー | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `libraries` | `List<Str>` | Optional | リンク対象のライブラリ名。プラットフォーム解決時に `lib{name}` 接頭辞や拡張子は自動補完する。 |
| `headers` | `List<Path>` | Optional | `reml-bindgen` に渡すヘッダパス。相対パスは `project_root` を基準に解決する。 |
| `bindgen` | `BindgenConfig` | Optional | `reml-bindgen` 実行の制御。 |
| `linker` | `LinkerConfig` | Optional | リンク検索パス・Framework 指定などプラットフォーム固有の設定を束ねる。 |

`BindgenConfig` と `LinkerConfig` は次の形を取る。

| キー | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `bindgen.enabled` | `Bool` | Optional | `true` の場合に `reml-bindgen` を実行する。既定は `false`。 |
| `bindgen.output` | `Path` | Conditional | `enabled = true` の場合は必須。生成される `.reml` 出力先。 |
| `bindgen.config` | `Path` | Optional | `reml-bindgen.toml` のパス。省略時は既定の探索規則に従う。 |
| `linker.search_paths` | `List<Path>` | Optional | ライブラリ検索パス。相対パスは `project_root` 起点で解決する。 |
| `linker.frameworks` | `List<Str>` | Optional | macOS での Framework 名。macOS 以外では警告診断を発行する。 |
| `linker.extra_args` | `List<Str>` | Optional | 追加のリンカ引数。CLI が `--link-arg` を受け取る場合はここへ正規化する。 |

#### 2.10.2 検証ルール

- `bindgen.enabled = true` の場合は `headers` を空にできない。空配列の場合は `ffi.build.config_invalid` を返す。
- `bindgen.output` は拡張子 `.reml` を要求し、既存ファイルを上書きする場合は `ffi.bindgen.output_overwrite` を `Warning` で記録する。
- `headers` / `linker.search_paths` は `Core.Path.normalize_path` に通し、`project_root` 外のパスは `ffi.build.path_outside_project` で拒否できる。
- `linker.frameworks` は `TargetProfile.os = "macos"` 以外の場合 `ffi.build.framework_unsupported` を記録し、ビルドは継続する。
- `libraries` は重複を除去し、解決不能なライブラリは `ffi.build.link_failed` に昇格する。

#### 2.10.3 `reml build` 実行フロー

```
manifest 読み込み
  → FFI 設定検証
  → headers 解決
  → bindgen 入力ハッシュ計算
  → キャッシュ判定 (hit/miss)
  → reml-bindgen 実行 (miss の場合)
  → 生成物キャッシュ保存
  → コンパイル/リンク
  → 監査ログ出力
```

- 入力ハッシュは `headers` 実体・`bindgen.config`・`TargetProfile`・`reml-bindgen` バージョンを正規化して連結した値とし、`ffi.bindgen.input_hash` として監査に残す。
- 生成物キャッシュは `cache_dir("reml")/ffi/{input_hash}` を既定パスとし、`reml build --no-cache` が指定された場合は常に `miss` とする。
- `ffi.build.*` と `ffi.bindgen.*` の監査イベントは責務を分離し、`ffi.bindgen.*` は生成、`ffi.build.*` はリンク・パッケージングに限定する。

#### 2.10.4 監査・診断の要点

- `ffi.build.config_invalid` / `ffi.build.link_failed` を基本診断とし、詳細は `extensions["ffi.build"]` に格納する。
- `ffi.bindgen.*` の監査イベントは [3-6](3-6-core-diagnostics-audit.md#ffi-ビルド生成監査テンプレート) に従い、`input_hash` を必須フィールドとして記録する。

#### 2.10.5 例の範囲

- `reml.json` の FFI セクション例（`libraries` / `headers` / `bindgen` / `linker`）。
- `reml build` 実行時のフロー図（テキスト手順で可）。

### 2.11 WASM Component Model（将来拡張セクション）

- WIT の `string` / `record` / `variant` / `list` を Reml 型へ写像する一次案を整理する。
  - `string` → `Text`
  - `record` → `struct` 相当
  - `variant` → `enum` 相当
  - `list<T>` → `List<T>`（境界でのコピー規約を明記）
- Canonical ABI では Shared Nothing を前提とし、所有権の移譲とコピー境界を厳密化する。
- Phase 4 は **調査と設計整理のみ**とし、実装・ツール統合は別計画に分離する。

#### 2.11.1 例の範囲

- WIT の最小 `record` / `variant` 定義と Reml 型の対応例（抜粋）。

## 3. Core.Unsafe.Ptr API

> 目的：`unsafe` 境界で扱う生ポインタ操作を公式 API として定義し、FFI・低レベルバッファ操作・GC 連携に必要な安全策と監査契約を明文化する。

### 3.1 型定義

```reml
pub type Ptr<T>
pub type MutPtr<T>
pub type NonNullPtr<T>
pub type Void
pub type FnPtr<Args, Ret>

pub type VoidPtr = Ptr<Void>

pub type Span<T> = { ptr: NonNullPtr<T>, len: usize }
pub type TaggedPtr<T> = { raw: Ptr<T>, label: Option<Str> }
```

- `Ptr<T>` は NULL 許容で読み取り専用。`MutPtr<T>` は書き込みを許可するがデータ競合は未定義動作（UB）となる。
- `NonNullPtr<T>` は非 NULL を静的に保証し、`Span<T>` や GC ルート管理の基盤となる。
- `Span<T>` は `{ptr, len}` の境界情報付きビューであり、`len = 0` の場合でも `ptr` は無効な非 NULL にならない。
- `TaggedPtr<T>` は監査・診断向けに任意ラベルを添付したポインタ。

### 3.2 生成・変換 API

```reml
pub type Ptr<T>
pub type MutPtr<T>
pub type NonNullPtr<T>
pub type UnsafeError

fn addr_of<T>(value: &T) -> Ptr<T>
fn addr_of_mut<T>(value: &mut T) -> MutPtr<T>
fn from_option<T>(value: Option<NonNullPtr<T>>) -> Ptr<T>
fn require_non_null<T>(ptr: Ptr<T>) -> Result<NonNullPtr<T>, UnsafeError>
fn cast<T, U>(ptr: Ptr<T>) -> Ptr<U>                                       // `unsafe`
fn cast_mut<T, U>(ptr: MutPtr<T>) -> MutPtr<U>                             // `unsafe`
fn to_int<T>(ptr: Ptr<T>) -> usize                                         // `unsafe`
fn from_int<T>(addr: usize) -> Ptr<T>                                      // `unsafe`
```

- `addr_of*` は評価順序を固定してアドレスを取得し、未初期化メモリへの参照生成を避ける。
- `require_non_null` は NULL を検証し、失敗時は `UnsafeErrorKind::NullPointer` を生成してアドレス値を `message` に含める。
- `cast*` / `to_int` / `from_int` は整列・サイズ違反が UB となるため `unsafe` を必須とし、実装は `check_alignment` 等の補助関数を併用すること。

### 3.3 読み書き・コピー API

```reml
pub type Ptr<T>
pub type MutPtr<T>
pub type UnsafeError

fn read<T>(ptr: Ptr<T>) -> Result<T, UnsafeError>                           // `unsafe`
fn read_unaligned<T>(ptr: Ptr<T>) -> Result<T, UnsafeError>                 // `unsafe`
fn write<T>(ptr: MutPtr<T>, value: T) -> Result<(), UnsafeError>            // `unsafe`
fn write_unaligned<T>(ptr: MutPtr<T>, value: T) -> Result<(), UnsafeError>  // `unsafe`
fn copy_to<T>(src: Ptr<T>, dst: MutPtr<T>, count: usize) -> Result<(), UnsafeError> // `unsafe`
fn copy_nonoverlapping<T>(src: Ptr<T>, dst: MutPtr<T>, count: usize) -> Result<(), UnsafeError> // `unsafe`
fn fill<T>(dst: MutPtr<T>, value: T, count: usize) -> Result<(), UnsafeError> // `unsafe`
```

- `read`/`write` は自然整列が満たされている必要があり、違反時は `UnsafeErrorKind::InvalidAlignment` を返す。整列保証が無い場合は `*_unaligned` を利用する。
- `copy_to` は重複領域を許容（`memmove` 相当）、`copy_nonoverlapping` は高速化のために非重複を前提とし違反時は `UnsafeErrorKind::OutOfBounds` を返す。
- `fill` は `T: Copy` を要求し、部分的に初期化された領域を一括初期化するユーティリティ。

### 3.4 アドレス計算と Span ユーティリティ

```reml
pub type Ptr<T>
pub type MutPtr<T>
pub type Span<T>
pub type UnsafeError
pub type isize

fn add<T>(ptr: Ptr<T>, count: usize) -> Ptr<T>                              // `unsafe`
fn add_mut<T>(ptr: MutPtr<T>, count: usize) -> MutPtr<T>                    // `unsafe`
fn offset<T>(ptr: Ptr<T>, delta: isize) -> Ptr<T>                           // `unsafe`
fn byte_offset<T>(ptr: Ptr<T>, bytes: isize) -> Ptr<T>                      // `unsafe`

fn span_from_raw_parts<T>(ptr: Ptr<T>, len: usize) -> Result<Span<T>, UnsafeError>
fn span_split_at<T>(span: Span<T>, index: usize) -> Result<(Span<T>, Span<T>), UnsafeError>
fn span_as_ptr<T>(span: Span<T>) -> Ptr<T>
fn span_as_mut_ptr<T>(span: Span<T>) -> MutPtr<T>
```

- `add`/`offset` は同一アロケーション内での移動のみを想定し、境界外アクセスは UB。`span_from_raw_parts` は境界チェックを行い、ゼロ長スパンでも `ptr` が非 NULL になるよう検証する。
- `span_split_at` は境界外インデックスで `UnsafeErrorKind::OutOfBounds` を返却する。
- `span_as_*` は `Span<T>` から生ポインタへ降格するため、直後の操作が `unsafe` 境界内に収まるようにする。

### 3.5 診断・監査補助

```reml
pub type Ptr<T>
pub type TaggedPtr<T>

fn tag<T>(ptr: Ptr<T>, label: Str) -> TaggedPtr<T>
fn debug_repr<T>(ptr: Ptr<T>) -> Str
```

- `tag` はデバッグビルドで監査ログやテスト診断に付与するメタデータを保持し、リリースビルドではオプションで No-Op にできる。`TaggedPtr` は `raw` と `label` を保持し、`label` は監査ログ `ptr.label` として記録される。
- `debug_repr` は `0x` 接頭辞付き 16 進アドレスを返し、監査ログや `Diagnostic.note` にコピーできる文字列として利用する。

### 3.6 代表ユースケース

#### 3.6.1 FFI コール境界

```reml
pub type Ptr<T>
pub type u8

fn strlen(ptr: Ptr<u8>) -> usize                                           // `extern "C"`
fn c_strlen(input: Str) -> usize                                           // `unsafe`
```

- FFI 側が NULL を返す場合は `require_non_null` を組み合わせ、`UnsafeErrorKind::NullPointer` を `FfiError` に昇格させる。
- `bind_fn_ptr` により Reml クロージャを ABI 検証済み `ForeignStub` へ変換し、`audit.log("ffi.call", {"label": tag_label})` と組み合わせて監査を残す。

#### 3.6.2 バッファ操作

```reml
pub type Span<T>
pub type Header
pub type ParseError
pub type u8

fn parse_header(bytes: Span<u8>) -> Result<Header, ParseError>              // `unsafe`
```

- `Span<T>` による境界チェックを行った後、局所的な `unsafe` ブロックを閉じ込めて利用すること。
- 長さ不明の外部入力を扱う場合は `span_split_at` や `span_from_raw_parts` を併用し、`UnsafeError` を `ParseError` へ変換する。

#### 3.6.3 GC ルート登録

```reml
pub type NonNullPtr<T>
pub type Object
pub type UnsafeError

pub type RootGuard = {
  ptr: NonNullPtr<Object>,
}

fn RootGuard::new(ptr: NonNullPtr<Object>) -> Result<RootGuard, UnsafeError>  // `unsafe`
fn RootGuard::release(self) -> Result<(), UnsafeError>                       // `unsafe`
```

- `register_root`/`unregister_root` は `unsafe` 操作であり、`RuntimeCapability::Gc` の監査フックを通じて `audit.log("gc.root", {"ptr": debug_repr(self.ptr)})` を残す。

### 3.7 CI と監査要件

| テスト | 目的 | 成功条件 |
| --- | --- | --- |
| `ffi-smoke` | NULL 許容／非許容ポインタの検証 | `audit.log("ffi.call")` に `ptr_label` が出力され、`UnsafeError` が発生しない |
| `buffer-span` | バッファ操作と境界チェック | `span_from_raw_parts` が境界外で `UnsafeErrorKind::OutOfBounds` を返し、`Diagnostic.code = "unsafe.span.out_of_bounds"` を生成 |
| `gc-root-guard` | GC ルートの登録・解除 | `audit.log("gc.root")` と `audit.log("gc.root.release")` が対で出力される |

監査ログでは `audited_unsafe_block`（§4.2）が `TaggedPtr` ラベルを `audit_id` と紐付け、Capability Registry の `SecurityCapability.effect_scope` に `{unsafe, audit}` が含まれていることを検証する。

---

## 4. Core.Unsafe の指針

```reml
fn unsafe_block<T>(f: () -> T) -> T                      // `effect {unsafe}`
fn assume(cond: Bool, message: Str) -> ()                // `effect {unsafe}`
fn transmute<T, U>(value: T) -> U                        // `effect {unsafe}`
```

- `unsafe_block` は安全性検証済みのコード領域を明示的に囲む。
- `assume` はコンパイラに対するヒントであり、偽の場合は未定義動作となる。
- `transmute` は型の同じビット表現を再解釈する際に使用。

### 4.1 安全性検証メカニズム

```reml
pub type Ptr<T>
pub type CodeLocation
pub type u8
pub type isize

fn verify_memory_safety(ptr: Ptr<u8>, size: usize) -> Result<(), UnsafeError>  // `effect {unsafe}`
fn check_alignment<T>(ptr: Ptr<T>) -> Bool                                     // `effect {unsafe}`
fn bounds_check(ptr: Ptr<u8>, offset: isize, bounds: (usize, usize)) -> Result<(), UnsafeError> // `effect {unsafe}`

pub type UnsafeError = {
  kind: UnsafeErrorKind,
  message: Str,
  location: Option<CodeLocation>,
}

pub enum UnsafeErrorKind = NullPointer
  | OutOfBounds
  | InvalidAlignment
  | UseAfterFree
  | DoubleFree
  | MemoryLeak
```

### 4.2 監査された unsafe 操作

```reml
pub type UnsafeContext
pub type CodeLocation

fn audited_unsafe_block<T>(operation: Str, f: () -> T) -> T                    // `effect {unsafe, audit}`
fn log_unsafe_operation(op: UnsafeOperation, context: UnsafeContext) -> ()     // `effect {audit}`

pub type UnsafeOperation = {
  operation_type: UnsafeOperationType,
  memory_address: Option<usize>,
  size: Option<usize>,
  stack_trace: List<CodeLocation>,
}

pub enum UnsafeOperationType = PointerDereference
  | MemoryAllocation
  | MemoryDeallocation
  | TypeTransmutation
  | ForeignCall
```

## 5. Capability Registry との連携

### 5.1 非同期 Capability

```reml
pub type AsyncRuntime
pub type CapabilityError
pub type TaskHandle<T>
pub type TimerCallback
pub type TimerId
pub type AsyncMetrics
pub type Future<T>
pub type Duration

pub type AsyncCapability<T> = {
  create_runtime: fn(AsyncRuntimeConfig) -> Result<AsyncRuntime, CapabilityError>,
  spawn_task: fn(Future<T>) -> Result<TaskHandle<T>, CapabilityError>,
  schedule_timer: fn(Duration, TimerCallback) -> Result<TimerId, CapabilityError>,
  get_metrics: fn() -> Result<AsyncMetrics, CapabilityError>,
}

pub type AsyncRuntimeConfig = {
  worker_threads: Option<usize>,
  max_blocking_threads: Option<usize>,
  thread_stack_size: Option<usize>,
  enable_io_driver: Bool,
  enable_time_driver: Bool,
}
```

### 5.2 FFI Capability

```reml
pub type Path
pub type LibraryHandle
pub type CapabilityError
pub type SymbolHandle
pub type FfiSignature
pub type FfiArgs
pub type FfiValue
pub type PathPattern

pub type FfiCapability = {
  load_library: fn(Path, FfiSecurity) -> Result<LibraryHandle, CapabilityError>,
  resolve_symbol: fn(LibraryHandle, Str) -> Result<SymbolHandle, CapabilityError>,
  verify_abi: fn(SymbolHandle, FfiSignature) -> Result<(), CapabilityError>,
  call_function: fn(SymbolHandle, FfiArgs) -> Result<FfiValue, CapabilityError>,
}

pub type FfiSecurity = {
  allowed_libraries: Option<List<PathPattern>>,
  signature_verification: Bool,
  sandbox_calls: Bool,
}
```

#### 5.2.1 契約検証と診断フック

`TypeInference::check_extern_bridge_contract` は `ffi_contract` モジュールを通じて `extern` 宣言の契約を静的に検証する。リンク名・ターゲット・呼出規約・所有権の各要素は次の規約に従う。違反時は 3-6 §2.4.3 の診断 (`ffi.contract.*`) を発火し、`AuditEnvelope.metadata["bridge"]` および `Diagnostic.extensions["bridge"]` に共通メタデータを記録する。

| チェック項目 | 必須条件 | 診断コード |
| --- | --- | --- |
| シンボル名 | `#[link_name("foo")]` または `ffi_link_name` 属性で明示する。空文字列や未指定は許容されない。 | `ffi.contract.symbol_missing` |
| 所有権 | `#[ownership("borrowed"|"transferred"|"reference")]` のいずれか。省略時は `borrowed` と同等だが診断を発火する。 | `ffi.contract.ownership_mismatch` |
| 呼出規約 | ターゲットトリプルと一致する ABI を選択する。標準サポートは下表のとおり。 | `ffi.contract.unsupported_abi` |

| ターゲット（例） | 期待される ABI | 備考 |
| --- | --- | --- |
| `*-unknown-linux-gnu`, `*-pc-linux-gnu` | `system_v` | System V AMD64 ABI。 | 
| `x86_64-pc-windows-msvc`, `*-windows-msvc` | `msvc` | Windows x64 (MSVC) ABI。 | 
| `arm64-apple-darwin`, `aarch64-apple-darwin` | `darwin_aapcs64` | Apple Silicon（AAPCS64 with Darwin extensions）。 |

ターゲットを明示しない場合は Capability Registry の既定値に従うが、ABI が未指定 (`AbiUnspecified`) もしくは `AbiCustom(_)` の場合は必ず `ffi.contract.unsupported_abi` を報告する。監査ログには `AuditEnvelope.metadata.bridge` オブジェクトを付与し、少なくとも次のキーを出力することを必須要件とする（`tooling/runtime/audit-schema.json` 参照）。

- `status`: ブリッジ処理の結果（`"ok"` / `"error"` / `"leak"` など）
- `target`, `arch`, `platform`: 解決済みターゲットトリプルと監査用プラットフォーム識別子（例: `macos-arm64`）
- `abi`, `expected_abi`: 実際に採用された呼出規約と Typer の期待値
- `ownership`: 引数側の所有権契約
- `extern_symbol`, `extern_name`, `link_name`: 実際にリンクされたシンボル
- `return`: 返り値処理の監査情報。`ownership` / `status` / `wrap` / `release_handler` / `rc_adjustment` を必須フィールドとし、Borrowed/Transferred が正しく包まれたかを記録する。

CI の `ffi_bridge.audit_pass_rate` が 1.0 を下回った場合はブリッジ契約の破綻として扱い、`collect-iterator-audit-metrics.py` のレポートに従って不足しているキーを補完すること。

LLVM lowering では `reml.bridge.version = 1` のモジュールフラグと `reml.bridge.stubs` Named Metadata を付与し、スタブごとに `bridge.stub_index`, `bridge.callconv`, `bridge.platform` などのキーを記録する。これにより Core 側の `AuditEnvelope.metadata["bridge"]` と LLVM IR のメタデータが同一キーで突合でき、`tooling/ci/collect-iterator-audit-metrics.py` が `ffi_bridge.audit_pass_rate` を算出する際の整合性を保証する。

### 5.3 Unsafe Capability

```reml
pub type CapabilityError
pub type MutPtr<T>
pub type NonNullPtr<T>
pub type Ptr<T>
pub type AllocationId
pub type PointerInfo
pub type u8

pub type UnsafeCapability = {
  enable_raw_pointers: fn(UnsafePolicy) -> Result<(), CapabilityError>,
  allocate_raw: fn(usize, usize) -> Result<MutPtr<u8>, CapabilityError>,
  deallocate_raw: fn(MutPtr<u8>, usize, usize) -> Result<(), CapabilityError>,
  track_allocation: fn(NonNullPtr<u8>, usize) -> Result<AllocationId, CapabilityError>,
  verify_pointer: fn(Ptr<u8>) -> Result<PointerInfo, CapabilityError>,
}

pub type UnsafePolicy = {
  enable_bounds_checking: Bool,
  enable_use_after_free_detection: Bool,
  enable_double_free_detection: Bool,
  max_allocations: Option<usize>,
  allocation_size_limit: Option<usize>,
}
```

## 6. 使用例（調査メモ）

```reml
pub type Path
pub type AsyncError
pub type Diagnostic

fn async_file_copy(src: Path, dest: Path) -> Result<(), Diagnostic>          // `effect {io.async}`
```

- 将来的な AsyncFile API の利用例（現時点では概念メモ）。`await` 構文は Reml の非同期拡張候補。
- エラーは `Diagnostic` へ変換し、監査連携の対象にする思考過程を示す。

## 7. セキュリティとベストプラクティス

### 7.1 非同期セキュリティ

```reml
pub type Future<T>
pub type LimitError
pub type Duration

// タイムアウトとリソース制限
fn with_async_limits<T>(limits: AsyncLimits, future: Future<T>) -> Future<Result<T, LimitError>>

pub type AsyncLimits = {
  execution_timeout: Option<Duration>,
  memory_limit: Option<usize>,
  concurrent_tasks_limit: Option<usize>,
}
```

### 7.2 FFI セキュリティ

```reml
pub type ForeignFunction
pub type FfiArgs
pub type FfiError
pub type Duration
pub type SyscallId
pub type FileAccessPolicy

// サンドボックス内での FFI 呼び出し
fn call_sandboxed<T>(foreign_fn: ForeignFunction, args: FfiArgs, sandbox: FfiSandbox) -> Result<T, FfiError> // `effect {ffi, unsafe, security, audit}`

pub type FfiSandbox = {
  memory_limit: usize,
  cpu_time_limit: Duration,
  syscall_whitelist: Option<List<SyscallId>>,
  network_access: Bool,
  file_access: FileAccessPolicy,
}
```

### 7.3 Unsafe セキュリティ

```reml
pub type UnsafeError

// メモリ安全性の動的検証
fn enable_memory_sanitizer(config: SanitizerConfig) -> Result<(), UnsafeError>  // `effect {unsafe, debug}`

pub type SanitizerConfig = {
  detect_out_of_bounds: Bool,
  detect_use_after_free: Bool,
  detect_double_free: Bool,
  detect_memory_leaks: Bool,
  quarantine_freed_memory: Bool,
}
```

## 8. パフォーマンス最適化

### 8.1 非同期最適化

```reml
pub type AsyncMetrics
pub type AsyncRuntimeConfig
pub type WorkloadProfile
pub type SchedulingStrategy
pub type Future<T>
pub type AsyncStream<T>

// タスクスケジューリングの調整
fn tune_async_runtime(metrics: AsyncMetrics) -> AsyncRuntimeConfig
fn adaptive_scheduling(workload: WorkloadProfile) -> SchedulingStrategy

// バッチ処理とストリーミングの最適化
fn batch_futures<T>(futures: List<Future<T>>, batch_size: usize) -> Future<List<T>>
fn stream_with_backpressure<T>(stream: AsyncStream<T>, buffer_size: usize) -> AsyncStream<T>
```

### 8.2 FFI 最適化

```reml
pub type ForeignFunction
pub type CachedForeignFunction
pub type FfiCall
pub type FfiValue
pub type FfiError
pub type FfiSignature
pub type CompiledWrapper

// 関数呼び出しのキャッシュ
fn cache_ffi_function(foreign_fn: ForeignFunction, cache_size: usize) -> CachedForeignFunction
fn batch_ffi_calls(calls: List<FfiCall>) -> Result<List<FfiValue>, FfiError>

// JIT コンパイルされた FFI ラッパー
fn compile_ffi_wrapper(signature: FfiSignature) -> Result<CompiledWrapper, FfiError>  // `effect {jit}`
```

## 9. デバッグとテストサポート

### 9.1 非同期デバッグ

```reml
pub type Future<T>
pub type ExecutionTrace
pub type DeadlockInfo
pub type DebugError
pub type Duration
pub type TestResult<T>

fn trace_async_execution<T>(future: Future<T>) -> Future<(T, ExecutionTrace)>      // `effect {debug}`
fn debug_deadlock_detection() -> Result<List<DeadlockInfo>, DebugError>           // `effect {debug}`
fn async_test_harness<T>(test: Future<T>, timeout: Duration) -> TestResult<T>     // `effect {test}`
```

### 9.2 FFI テスト

```reml
pub type FfiSignature
pub type MockBehavior
pub type MockForeignFunction
pub type ForeignFunction
pub type FfiContract
pub type FfiError

fn mock_foreign_function(signature: FfiSignature, behavior: MockBehavior) -> MockForeignFunction
fn verify_ffi_contract(foreign_fn: ForeignFunction, contract: FfiContract) -> Result<(), FfiError>
```

### 9.3 Unsafe テスト

```reml
pub type CorruptionPattern
pub type UnsafeInvariant
pub type TestResult<T>

fn simulate_memory_corruption(pattern: CorruptionPattern) -> ()                 // `effect {unsafe, test}`
fn test_unsafe_invariants(invariants: List<UnsafeInvariant>) -> TestResult<()> // `effect {unsafe, test}`
```

## 10. 将来拡張: WASM Component Model / WIT（調査段階）

- Phase 4 は調査と設計整理のみを対象とし、現行の FFI 仕様・ABI を変更しない。
- WIT 型と Reml 型の対応表は `docs/notes/ffi/ffi-wasm-component-model-log.md` に一次案を整理する。
- Shared Nothing 前提のため、境界コピー・所有権移譲・エラーの `Result` マッピングを明文化する必要がある。
- WIT 経由であっても `effect {ffi}` と監査ログの対象範囲は維持する（`unsafe` 境界の緩和は別途検討）。
- WIT 監査キーの命名方針は `docs/spec/3-6-core-diagnostics-audit.md` の `ffi.wit.*` を参照する。

> 関連: [guides/runtime-bridges.md](../guides/runtime/runtime-bridges.md), [guides/reml-ffi-handbook.md](../guides/ffi/reml-ffi-handbook.md), [guides/ffi-wit-poc.md](../guides/ffi/ffi-wit-poc.md), [notes/ffi-wasm-component-model-log.md](../notes/ffi/ffi-wasm-component-model-log.md), [2.6 実行戦略](2-6-execution-strategy.md), [3.8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md)
