# 3.9 Core Async / FFI / Unsafe

> 目的：Reml の非同期実行 (`Core.Async`)・FFI (`Core.Ffi`)・unsafe ブロック (`Core.Unsafe`) に関する基本方針と効果タグの枠組みを整理し、今後の詳細仕様策定に備える。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `effect {io.async}`, `effect {io.blocking}`, `effect {io.timer}`, `effect {ffi}`, `effect {unsafe}`, `effect {security}`, `effect {audit}` |
| 依存モジュール | `Core.Prelude`, `Core.Iter`, `Core.IO`, `Core.Runtime`, `Core.Diagnostics` |
| 相互参照 | [2.6 実行戦略](2-6-execution-strategy.md), [3.8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md), [guides/runtime-bridges.md](guides/runtime-bridges.md), [guides/reml-ffi-handbook.md](guides/reml-ffi-handbook.md) |

## 1. Core.Async の枠組み

```reml
pub type Future<T> = {
  poll: fn(&mut Context) -> Poll<T>,
}

pub enum Poll<T> = Ready(T) | Pending

pub struct Task<T> {
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

### 1.2 高度な非同期パターン

```reml
fn join<T, U>(future1: Future<T>, future2: Future<U>) -> Future<(T, U)>         // `effect {io.async}`
fn select<T>(futures: List<Future<T>>) -> Future<(usize, T)>                   // `effect {io.async}`
fn timeout<T>(future: Future<T>, duration: Duration) -> Future<Result<T, TimeoutError>> // `effect {io.async}`
fn retry<T>(future: () -> Future<T>, policy: RetryPolicy) -> Future<T>         // `effect {io.async}`

pub type RetryPolicy = {
  max_attempts: u32,
  backoff: BackoffStrategy,
  should_retry: (AsyncError) -> Bool,
}

pub enum BackoffStrategy = Linear(Duration) | Exponential { base: Duration, max: Duration } | Custom((u32) -> Duration)
```

### 1.3 ストリームとアシンクイテレータ

```reml
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
pub struct Channel<Send, Recv> where Send: Serialize, Recv: Deserialize {
  sender: DslSender<Send>,
  receiver: DslReceiver<Recv>,
  codec: Codec<Send, Recv>,
  buffer_size: usize,
  overflow: OverflowPolicy,
}

fn create_channel<S, R>(buffer_size: usize, codec: Codec<S, R>) -> Result<(DslSender<S>, DslReceiver<R>), AsyncError> // `effect {io.async}`
fn merge_channels<T>(channels: List<DslReceiver<T>>) -> DslReceiver<T>                                             // `effect {io.async}`
fn split_channel<T>(channel: DslReceiver<T>, predicate: (T) -> Bool) -> (DslReceiver<T>, DslReceiver<T>)           // `effect {io.async}`
fn with_execution_plan<T>(dsl: DslSpec<T>, plan: ExecutionPlan) -> DslSpec<T>                                     // `effect {io.async}`

struct ExecutionPlan = {
  strategy: ExecutionStrategy,
  backpressure: BackpressurePolicy,
  error: ErrorPropagationPolicy,
  scheduling: SchedulingPolicy,
}

enum BackpressurePolicy = Drop | DropOldest | Buffer(usize) | Block | Adaptive { high_watermark: usize, low_watermark: usize, strategy: AdaptiveStrategy }

enum AdaptiveStrategy = DropNewest | SlowProducer | SignalDownstream
```

- `Channel<Send, Recv>` は DSL 間通信を型安全に扱い、コード変換を `Codec<Send, Recv>` へ委譲する。
- `ExecutionPlan` は `conductor` の `execution { ... }` ブロックと 1:1 で対応し、スケジューラーへ渡す実行ポリシーを保持する。
- `with_execution_plan` は DSL 定義時に計画を合成するコンビネータであり、バックプレッシャー制御やエラー隔離を `Core.Async` ランタイムへ伝える。

#### 1.4.1 Codec 契約

```reml
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
pub enum ExecutionStrategy = Serial | Parallel { max_concurrency: Option<usize> } | Streaming

pub enum ErrorPropagationPolicy = FailFast | Isolate { circuit_breaker: Option<Duration> } | Retry { policy: RetryPolicy }

pub enum SchedulingPolicy = Auto | Explicit(SchedulerConfig)
```

- `ExecutionPlan.strategy` で `Parallel` を指定する場合、`max_concurrency` に `Some(0)` を設定することは禁止とし `AsyncErrorKind::InvalidConfiguration` を返す。
- `ErrorPropagationPolicy::Retry` は `RetryPolicy.max_attempts >= 1` を要求し、違反時は即座にエラーを返す。
- `SchedulingPolicy::Explicit` を選択する場合、`SchedulerConfig` は `default_scheduler_config()` の制約（`worker_threads` が 1 以上、`max_blocking_threads <= worker_threads`）を満たす必要がある。同条件違反時は `AsyncErrorKind::InvalidConfiguration`。
- `BackpressurePolicy::Adaptive` は `high_watermark > low_watermark` かつ `low_watermark >= 1` を要求し、閾値の逆転やゼロ指定は静的検証段階でビルドを停止する。`BackpressurePolicy::Buffer(n)` も `n >= 1` を必須とし、`Adaptive.strategy = SlowProducer` を選択する場合は `ExecutionStrategy::Streaming` と併用したときのみ許可される。
- コンパイラ (`remlc`) と CLI (`reml lint`, `reml build`) は `ExecutionPlan` を DSL/Conductor マニフェストと合成した段階で静的検証し、上記制約違反や `SchedulerConfig` の矛盾（`worker_threads` 未設定、`max_blocking_threads` の超過、`Parallel` 指定時の `Some(0)` など）をビルドエラーとする。検証は 0-1 §1.1–1.2 で定めた性能・安全性指針に従い、実行前に Backpressure 設定とスケジューラ構成を確定させる。

#### 1.4.4 診断と監査

- すべてのチャンネル操作は `Diagnostic.domain = Async` とし、`extensions["channel"]` に `name`、`codec`、`buffer_size`、`overflow` を記録することを推奨する。
- `CodecError` が発生した場合は `Diagnostic.code = Some("async.codec.failure")` を用い、`cause` を診断拡張に埋め込む。
- `ExecutionPlan` の整合性エラーは静的検証・実行時に関わらず `Diagnostic.code = Some("async.plan.invalid")` を既定とし、`Diagnostic.severity = Error` で報告する。CLI は `extensions["async.plan"].reason` に検出理由と `plan` スナップショット JSON を添付し、LSP/CI から同一フォーマットで参照できるようにする。

#### 1.4.5 チャネルメトリクス API

```reml
pub struct ChannelMetricsHandle = {
  queue_depth: GaugeMetric,
  dropped_messages: CounterMetric,
  producer_latency: LatencyHistogram,
  consumer_latency: LatencyHistogram,
  throughput: CounterMetric,
}

pub struct ChannelMetricsSample = {
  queue_depth: usize,
  dropped_messages: u64,
  producer_latency_p95: Duration,
  consumer_latency_p95: Duration,
  throughput_per_sec: f64,
  observed_at: Timestamp,
}

fn channel_metrics<T>(recv: &DslReceiver<T>,
  registry: MetricsRegistry,
  channel_id: ChannelId,
  opts: ChannelMetricOptions = ChannelMetricOptions::default())
  -> Result<ChannelMetricsHandle, AsyncError>                            // `effect {io.async}`

fn snapshot_channel_metrics(handle: &ChannelMetricsHandle)
  -> Result<ChannelMetricsSample, AsyncError>                            // `effect {io.async}`

pub struct ChannelMetricOptions = {
  collect_dropped_messages: Bool = true,
  collect_latency: Bool = true,
  collect_throughput: Bool = true,
}
```

- `ChannelMetricsHandle` は 3.6 §6.1 の `DslMetricsHandle` と同じ `MetricsRegistry` を利用し、`channel_id` を名前空間に含めた計測キー（例: `channel.data_pipeline.source.items.queue_depth`）を生成する。
- `channel_id` は `conductor` マニフェストのチャネル識別子（`manifest.conductor.channels[].id`）と一致させ、CLI/LSP の差分表示で人間が追跡しやすいようにする。
- すべてのオプションが既定で `true` のため、監視メトリクスを省略した場合でも `queue_depth`、`dropped_messages`、`producer_latency`、`consumer_latency`、`throughput` を自動収集する。
- `snapshot_channel_metrics` は CLI/テレメトリバッチ収集で利用し、`throughput_per_sec` は `throughput` カウンタのデルタから算出する。`observed_at` のタイムスタンプにより 0-1 §1.1 が求める性能監視を支援する。異常値は `Diagnostic.code = Some("async.channel.backpressure")` を推奨し、`AuditEnvelope.metadata["queue_depth"]` へ数値を添付する。
- `collect_dropped_messages=false` を指定した場合でも、水位超過による `Drop` ポリシー発生時には `AsyncErrorKind::Backpressure` を `Diagnostic.domain = Async` とともに記録し、運用時の安全性（0-1 §1.2）を損なわないようにする。

### 1.5 プラットフォーム適応スケジューラ

```reml
fn default_scheduler_config() -> SchedulerConfig = {
  let info = platform_info();
  let hints = scheduler_hints(info); // Core.Async が提供する CPU/IO 推奨値
  SchedulerConfig {
    worker_threads: Some(if has_capability(RuntimeCapability::Vector512) {
      hints.prefer_physical_threads
    } else {
      hints.prefer_logical_threads
    }),
    max_blocking_threads: if platform_features().contains("io.blocking.strict") {
      Some(hints.blocking_guard_threads)
    } else {
      None
    },
    io_driver: info.family == TargetFamily::Unix,
    time_driver: true,
  }
}
```

* `platform_info()` は `Core.Runtime` から取得した実行環境を返し、`RuntimeCapability` に応じてスケジューラ構成を切り替えられる。
* `platform_features()` で `RunConfig.extensions["target"].features` と同期したフラグ（例: `feature = "io.blocking.strict"`）を参照し、DSL ごとのバックプレッシャ設定やタスクプールサイズを調整する。
* Core.DSL モジュールはこの関数を利用して `ExecutionPlan` の既定値を決定し、`@cfg` で有効化した機能と矛盾しないようにする。

### 1.6 効果ハンドラによる Async 差し替え

実験的な代数的効果ハンドラを用いることで、`io.async` を発生させる API をテスト用モックへ差し替えたり、能力別に stage を切り替えることができる。

```reml
@handles(Console)
fn with_console_mock() -> Result<Text, AsyncError> ! {} =
  handle greet() with
    handler Console {
      operation log(msg, resume) {
        audit.log("console.log", msg)
        resume(())
      }
      operation ask(_, resume) {
        resume("Reml")
      }
      return value {
        Ok(value)
      }
    }
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
pub type ActorId = Uuid
pub type NodeId = Str

pub struct MailboxHandle<Message> {
  capacity: usize,
  overflow: OverflowPolicy,
  enqueue: fn(Message) -> Result<(), AsyncError>,
  metrics: MailboxStats,
}

pub struct TransportMetrics {
  throughput: Counter,
  latency: Histogram,
}

pub struct ActorSystem {
  scheduler: SchedulerHandle,
  transport: Option<TransportHandle>,
  registry: CapabilityRegistry,
  config: ActorSystemConfig,
}

pub struct ActorSystemConfig {
  mailbox_high_watermark: usize,
  mailbox_low_watermark: usize,
  ask_timeout: Duration,
}

pub struct ActorRef<Message> {
  id: ActorId,
  mailbox: MailboxHandle<Message>,
  system: &'static ActorSystem,
}

pub struct ActorContext {
  self_ref: ActorRef<Any>,
  tags: Set<EffectTag>,
  span: DiagnosticSpan,
}

fn spawn_actor<Message, State>(system: &ActorSystem, init: () -> State,
  handler: fn(Message, &mut State, &mut ActorContext) -> Future<()>)
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
pub struct TransportHandle {
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
let system = runtime.actor_system()?;

actor spec Greeter {
  state = { greeted: Set<Text> }

  on Message::Greet(name) -> ! { io.async } {
    if !state.greeted.contains(name) {
      state.greeted.insert(name);
      log.info("greet", name);
    }
    reply(Message::Hello(name))?;
  }
}

let greeter = system.spawn(Greeter::new())?;
let response = await system.ask(greeter, Message::Greet("Reml"), 2.s)?;
```

- `actor spec` はコード生成フェーズで上記 API を呼ぶテンプレートを展開し、`Core.Async` が提供するバックプレッシャ制御を透過的に利用する。
- リモート呼び出しの場合は `system.link_remote(greeter, node)` を明示し、`CapabilityRegistry::stage_of(effect {io.async})` が `Stable` であることを確認する。
- LSP は `actor spec` の診断を `Diagnostic.domain = Async` とし、`async.actor.unhandled_message` を未処理パターンの検出に用いる。

#### 1.9.4 Capability 検証手順

1. `CapabilityRegistry::get("runtime.async")` で `RuntimeCapability::AsyncScheduler` を確認し、`SchedulerHandle::supports_mailbox()` が `true` であること。
2. `CapabilityRegistry::get("runtime.actor")` で `ActorRuntimeCapability` を取得し、`stage` が `Experimental` の場合は `@requires_capability(stage="experimental")` を付与する。
3. 分散を有効化する DSL は `guides/runtime-bridges.md §11` のチェックリスト（監査・TLS・再接続ポリシー）を満たす。
4. いずれかが欠落した場合は `Diagnostic.code = Some("async.actor.capability_missing")` を返し、復旧手順を提示する。


## 2. Core.Ffi の枠組み

```reml
pub type ForeignFunction = FnPtr<(VoidPtr,), VoidPtr>

fn bind_library(path: Path) -> Result<LibraryHandle, FfiError>               // `effect {ffi}`
fn get_function(handle: LibraryHandle, name: Str) -> Result<ForeignFunction, FfiError> // `effect {ffi}`
fn call_ffi(fn_ptr: ForeignFunction, args: FfiArgs) -> Result<FfiValue, FfiError> // `effect {ffi, unsafe}`

pub type FfiArgs = Span<u8>
pub type FfiValue = Span<u8>
```

### 2.0 バインディング生成と Capability 連携

```reml
fn auto_bind(handle: LibraryHandle, name: Str, signature: FfiSignature) -> Result<TypedForeignFn, FfiError> // `effect {ffi}`
fn auto_bind_all(handle: LibraryHandle, spec: [FfiBinding]) -> Result<BoundLibrary, FfiError>               // `effect {ffi}`
fn call_with_capability(cap: FfiCapability, symbol: ForeignFunction, args: FfiArgs) -> Result<FfiValue, FfiError> // `effect {ffi, security, audit}`

struct FfiSignature = { params: [FfiType], return_type: FfiType }
struct FfiBinding   = { name: Str, signature: FfiSignature, conventions: CallingConvention }
struct SymbolHandle = { library: LibraryHandle, function: ForeignFunction }
struct TypedForeignFn = { call: fn(FfiArgs) -> Result<FfiValue, FfiError>, symbol: ForeignFunction, metadata: FfiBinding }
struct FfiCapability = { call_function: fn(SymbolHandle, FfiArgs) -> Result<FfiValue, FfiError>, sandbox: Option<FfiSandbox>, audit: AuditHandle }
struct LibraryMetadata = { path: Path, preferred_convention: Option<CallingConvention>, required_capabilities: Set<RuntimeCapability> }
```

- `auto_bind` は署名情報からシリアライザ/デシリアライザを自動生成し、返却された `TypedForeignFn` 経由で型安全な `call` を提供する。
- `auto_bind_all` は複数シンボルを一括登録し、Capability Registry と連携する `BoundLibrary` を構築する。
- `call_with_capability`（および `FfiCapability.call_function`）は [3.8](3-8-core-runtime-capability.md) の `CapabilityRegistry` 経由で取得した権限を通じて FFI 呼び出しを実行し、監査ログやサンドボックスを適用する。

- `FfiError` は OS 依存エラーやシンボル解決失敗をラップ。

```reml
pub type FfiError = {
  kind: FfiErrorKind,
  message: Str,
  library_path: Option<Path>,
  symbol_name: Option<Str>,
}

pub enum FfiErrorKind = {
  LibraryNotFound,
  SymbolNotFound,
  InvalidSignature,
  CallFailed,
  SecurityViolation,
  UnsupportedPlatform,
}
```

### 2.1 効果ハンドラによる FFI サンドボックス（実験段階）

`ffi` 効果を捕捉するハンドラを用意すると、危険なネイティブ呼び出しをテスト用スタブや監査ロガーへ差し替えられる。

```reml
effect ForeignCall : ffi {
  operation call(name: Text, payload: Bytes) -> Result<Bytes, FfiError>
}

@handles(ForeignCall)
@requires_capability(stage="experimental")
fn with_foreign_stub(request: Request) -> Result<Response, FfiError> ! {} =
  handle do ForeignCall.call("service", encode(request)) with
    handler ForeignCall {
      operation call(name, payload, resume) {
        audit.log("ffi.call", {"name": name, "bytes": payload.len()})
        // スタブ応答を返し、本物の FFI を呼び出さず終了
        resume(Ok(stub_response(name, payload)))
      }
      return result {
        result.and_then(decode_response)
      }
    }
```

- `@handles(ForeignCall)` で捕捉可能な効果を宣言し、`resume` に `Result<Bytes, FfiError>` を渡して元の計算へ戻す。
- Stage が `Experimental` の間は `@requires_capability(stage="experimental")` を併用し、Capability Registry 側で明示的に opt-in した環境でのみこのハンドラを利用できるようにする。
- `effects.handler.unhandled_operation` 診断を避けるため、`ForeignCall` で定義されたすべての `operation` を実装すること。

ステージを `Beta`/`Stable` へ引き上げる際は、Async と同様に `Diagnostic.extensions["effects"].stage` を更新し、`effects.stage.promote_without_checks` が解消されてから Capability Registry とマニフェストの整合を取る（§1.7）。

### 2.2 タイプセーフな FFI ラッパー

```reml
// 自動的なラッパー生成
macro foreign_fn(lib: Str, name: Str, signature: Str) -> ForeignFunction

// 使用例
let add_numbers = foreign_fn!("math", "add", "fn(i32, i32) -> i32");
let args = ffi::encode_args(&(42i32, 24i32));
let raw = add_numbers.call(args)?;            // raw は FfiValue (Span<u8>)
let sum: i32 = ffi::decode_result(raw)?;
```

`ffi::encode_args` / `ffi::decode_result` は `FfiSignature` と互換のシリアライズヘルパで、`Span<u8>` を安全に生成・復元する。低レベル API を直接利用する場合は `span_from_raw_parts` と `CapabilitySecurity.effect_scope` を併用し、境界検査と監査記録を怠らないこと。

### 2.3 呼出規約とプラットフォーム適応

```reml
pub enum CallingConvention = C | StdCall | FastCall | SysV | WasmSystemV | Custom(Str)

fn resolve_calling_convention(target: PlatformInfo, foreign: LibraryMetadata) -> Result<CallingConvention, FfiError> // `effect {runtime}`
fn link_foreign_library(path: Path, target: PlatformInfo) -> Result<LibraryHandle, FfiError> // `effect {ffi}`
fn with_abi_adaptation(fn_ptr: ForeignFunction, conv: CallingConvention) -> Result<ForeignFunction, FfiError> // `effect {ffi, unsafe}`
```

* 既定では `RunConfig.extensions["target"]` を用いて呼出規約を決定し、`platform_info()`（[3-8](3-8-core-runtime-capability.md)）が提供する実行時情報と突き合わせる。
* ターゲットの `family` が `Windows` かつ `arch = X64` の場合は `StdCall` を採用し、`Unix` ファミリでは `SysV` を既定とする。WASM ターゲットでは `WasmSystemV` を利用し、サポート外の場合は `FfiErrorKind::UnsupportedPlatform` を返す。
* `resolve_calling_convention` は `LibraryMetadata` に含まれる `preferred_convention` を尊重しつつ、実行環境で利用できない場合は `target.config.unsupported_value` 診断を併せて発行する。診断は `Diagnostic.extensions["cfg"].evaluated` にターゲット値とライブラリ要求を記録する。
* `with_abi_adaptation` は必要に応じてシム層を挿入し、レジスタ引数配置やスタック整列を調整する。性能への影響を抑えるため、変換は初回呼び出し時にキャッシュする。

### 2.4 メモリ管理と所有権境界

```reml
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

- `ForeignPtr<T>` は `Ptr<T>` を内包し、必要に応じて `NonNullPtr<T>` へ昇格して利用する。`layout` には [4-3 Memory Capability プラグイン](4-3-memory-plugin.md) で定義する `Layout` 情報を格納する。
- `ForeignBuffer` は `Span<u8>` と所有権メタデータを保持し、`Ownership::Borrowed` の場合は解放禁止とする。
- `call_ffi` は `unsafe` を要求し、境界で `AuditEnvelope` を付与することが推奨される。`transfer_buffer` では Capability Registry を通じて `MemoryCapability` の監査フックを呼び出す。

## 3. Core.Unsafe.Ptr API

```reml
// 基本ポインタ型
type Ptr<T>
type MutPtr<T>
type NonNullPtr<T>
type VoidPtr
type FnPtr<Args, Ret>
type Span<T> = { ptr: NonNullPtr<T>, len: usize }

// 基本操作
fn read<T>(ptr: Ptr<T>) -> Result<T, UnsafeError>                               // `effect {unsafe}`
fn write<T>(ptr: MutPtr<T>, value: T) -> Result<(), UnsafeError>                // `effect {unsafe}`
fn copy_to<T>(src: Ptr<T>, dst: MutPtr<T>, count: usize) -> Result<(), UnsafeError> // `effect {unsafe, memory}`
fn cast<T, U>(ptr: Ptr<T>) -> Ptr<U>                                            // `effect {unsafe}`
fn offset<T>(ptr: Ptr<T>, count: isize) -> Ptr<T>                               // `effect {unsafe}`
fn as_non_null<T>(ptr: Ptr<T>) -> Result<NonNullPtr<T>, UnsafeError>            // `effect {unsafe}`

// Span ユーティリティ
fn span_from_raw_parts<T>(ptr: Ptr<T>, len: usize) -> Result<Span<T>, UnsafeError>     // `effect {unsafe}`
fn span_split_at<T>(span: Span<T>, index: usize) -> Result<(Span<T>, Span<T>), UnsafeError> // `effect {unsafe}`
fn span_as_ptr<T>(span: Span<T>) -> Ptr<T>                                              // `effect {unsafe}`
fn span_as_mut_ptr<T>(span: Span<T>) -> MutPtr<T>                                       // `effect {unsafe}`

// VoidPtr / FnPtr ブリッジ
fn to_void_ptr<T>(ptr: Ptr<T>) -> VoidPtr                                      // `effect {unsafe}`
fn from_void_ptr<T>(ptr: VoidPtr) -> Ptr<T>                                    // `effect {unsafe}`
fn bind_fn_ptr<Args, Ret>(ptr: FnPtr<Args, Ret>) -> Result<ForeignStub<Args, Ret>, UnsafeError> // `effect {unsafe}`

pub type ForeignStub<Args, Ret> = {
  call: fn(Args) -> Ret,
  raw: FnPtr<Args, Ret>,
}
```

* `Span<T>` は `len = 0` の場合でも `ptr` は無効な非NULLダングリング値を許容しないため、ゼロ長スライスは安全に扱える。
* `copy_to` は `memory` 効果を併発し、`CapabilitySecurity.effect_scope` に `memory` を含む API からのみ呼び出す。
* `bind_fn_ptr` は FFI の `foreign_fn!` マクロと連携し、ABI 検証後に型安全な呼び出しを生成する。

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
fn verify_memory_safety(ptr: Ptr<u8>, size: usize) -> Result<(), UnsafeError>  // `effect {unsafe}`
fn check_alignment<T>(ptr: Ptr<T>) -> Bool                                     // `effect {unsafe}`
fn bounds_check(ptr: Ptr<u8>, offset: isize, bounds: (usize, usize)) -> Result<(), UnsafeError> // `effect {unsafe}`

pub type UnsafeError = {
  kind: UnsafeErrorKind,
  message: Str,
  location: Option<CodeLocation>,
}

pub enum UnsafeErrorKind = {
  NullPointer,
  OutOfBounds,
  InvalidAlignment,
  UseAfterFree,
  DoubleFree,
  MemoryLeak,
}
```

### 4.2 監査された unsafe 操作

```reml
fn audited_unsafe_block<T>(operation: Str, f: () -> T) -> T                    // `effect {unsafe, audit}`
fn log_unsafe_operation(op: UnsafeOperation, context: UnsafeContext) -> ()     // `effect {audit}`

pub type UnsafeOperation = {
  operation_type: UnsafeOperationType,
  memory_address: Option<usize>,
  size: Option<usize>,
  stack_trace: List<CodeLocation>,
}

pub enum UnsafeOperationType = {
  PointerDereference,
  MemoryAllocation,
  MemoryDeallocation,
  TypeTransmutation,
  ForeignCall,
}
```

## 5. Capability Registry との連携

### 5.1 非同期 Capability

```reml
pub type AsyncCapability = {
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

### 5.3 Unsafe Capability

```reml
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
use Core;
use Core.Async;
use Core.Ffi;

fn async_file_copy(src: Path, dest: Path) -> Result<(), Diagnostic> =
  spawn(async move {
    let mut reader = AsyncFile::open(src).await?;
    let mut writer = AsyncFile::create(dest).await?;
    while let Some(chunk) = reader.next_chunk().await? {
      writer.write_all(chunk).await?;
    }
    Ok(())
  }, scheduler())
    .await
    .map_err(|err| Diagnostic::from_async_error(err))
```

- 将来的な AsyncFile API の利用例（現時点では概念メモ）。`await` 構文は Reml の非同期拡張候補。
- エラーは `Diagnostic` へ変換し、監査連携の対象にする思考過程を示す。

## 7. セキュリティとベストプラクティス

### 7.1 非同期セキュリティ

```reml
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
// タスクスケジューリングの調整
fn tune_async_runtime(metrics: AsyncMetrics) -> AsyncRuntimeConfig
fn adaptive_scheduling(workload: WorkloadProfile) -> SchedulingStrategy

// バッチ処理とストリーミングの最適化
fn batch_futures<T>(futures: List<Future<T>>, batch_size: usize) -> Future<List<T>>
fn stream_with_backpressure<T>(stream: AsyncStream<T>, buffer_size: usize) -> AsyncStream<T>
```

### 8.2 FFI 最適化

```reml
// 関数呼び出しのキャッシュ
fn cache_ffi_function(foreign_fn: ForeignFunction, cache_size: usize) -> CachedForeignFunction
fn batch_ffi_calls(calls: List<FfiCall>) -> Result<List<FfiValue>, FfiError>

// JIT コンパイルされた FFI ラッパー
fn compile_ffi_wrapper(signature: FfiSignature) -> Result<CompiledWrapper, FfiError>  // `effect {jit}`
```

## 9. デバッグとテストサポート

### 9.1 非同期デバッグ

```reml
fn trace_async_execution(future: Future<T>) -> Future<(T, ExecutionTrace)>      // `effect {debug}`
fn debug_deadlock_detection() -> Result<List<DeadlockInfo>, DebugError>        // `effect {debug}`
fn async_test_harness<T>(test: Future<T>, timeout: Duration) -> TestResult<T>   // `effect {test}`
```

### 9.2 FFI テスト

```reml
fn mock_foreign_function(signature: FfiSignature, behavior: MockBehavior) -> MockForeignFunction
fn verify_ffi_contract(foreign_fn: ForeignFunction, contract: FfiContract) -> Result<(), FfiError>
```

### 9.3 Unsafe テスト

```reml
fn simulate_memory_corruption(pattern: CorruptionPattern) -> ()                 // `effect {unsafe, test}`
fn test_unsafe_invariants(invariants: List<UnsafeInvariant>) -> TestResult<()> // `effect {unsafe, test}`
```

> 関連: [guides/runtime-bridges.md](guides/runtime-bridges.md), [guides/reml-ffi-handbook.md](guides/reml-ffi-handbook.md), [2.6 実行戦略](2-6-execution-strategy.md), [3.8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md)
