# Core.Parse.Streaming 拡張ガイド

> 目的：`Core.Parse.Streaming` モジュールが提供するストリーミング実行・継続再開・インクリメンタル解析 API の仕様をまとめ、Reml コアとの境界を明確にする。

## 1. ランナー API

```reml
fn run_stream<T>(p: Parser<T>, feeder: Feeder, cfg: StreamingConfig = {}) -> StreamOutcome<T>
fn resume<T>(cont: Continuation<T>, more: Bytes) -> StreamOutcome<T>
```

- `run_stream` はチャンク入力を逐次処理し、`StreamOutcome::Pending` を返した場合は追加データが必要。
- `resume` は前回 `Pending` で停止した継続を再開する。
- いずれの API も `ParseResult` と同じ診断ポリシーを維持するため、`StreamOutcome::Completed` には `ParseResult<T>` を内包させることを推奨する。

```reml
type StreamOutcome<T> =
  | Completed { result: ParseResult<T>, meta: StreamMeta }
  | Pending { continuation: Continuation<T>, demand: DemandHint, meta: StreamMeta }
```

## 2. Feeder とデマンドヒント

```reml
type DemandHint = {
  min_bytes: usize,
  preferred_bytes: Option<usize>,
  frame_boundary: Option<TokenClass>
}

type Feeder = {
  pull: fn(DemandHint) -> FeederYield
}

type FeederYield =
  | Chunk(Bytes)
  | Await
  | Closed
  | Error(StreamError)
```

- `min_bytes` は再開に必要な最小バイト数、`preferred_bytes` はパフォーマンス上望ましいチャンクサイズを示す。
- `frame_boundary` を利用すると、IDE やログストリームで意味的な境界（ステートメント単位など）を維持できる。

## 3. 継続メタデータ

```reml
type Continuation<T> = {
  state: Opaque,
  meta: ContinuationMeta
}

type ContinuationMeta = {
  commit_watermark: usize,
  buffered: Input,
  resume_hint: Option<DemandHint>,
  expected_tokens: Set<Expectation>,
  last_checkpoint: Option<Span>,
  trace_label: Option<String>
}
```

- `commit_watermark` より前の Packrat キャッシュは安全に破棄できる。
- `expected_tokens` と `last_checkpoint` は IDE 補完や自動復旧に利用され、`trace_label` は SpanTrace (2.5) と連動する。

## 4. FlowController とバックプレッシャ

```reml
type FlowController = {
  mode: FlowMode,
  high_watermark: usize,
  low_watermark: usize,
  policy: FlowPolicy
}

type FlowMode = "push" | "pull" | "hybrid"

type FlowPolicy =
  | Manual { on_demand: fn() -> Demand }
  | Auto { backpressure: BackpressureSpec }

type Demand = { bytes: usize, frames: usize }

type BackpressureSpec = {
  max_lag: Option<Duration>,
  debounce: Option<Duration>,
  throttle: Option<Duration>
}
```

- `push` モードはストリーム側が能動的にチャンクを供給する用途（ログ集約、ライブ入力）向け。
- `pull` モードは IDE の差分適用など、必要な時だけチャンクを取得したいケースで利用する。
- `hybrid` は実行時にモードをスイッチするための妥協案。`BackpressureSpec` は遅延やバッファ占有率を監視して自動的に調整する。

## 5. StreamDriver ヘルパ

```reml
type StreamDriver<T, Sink> = {
  parser: Parser<T>,
  feeder: Feeder,
  sink: Sink,
  flow: FlowController,
  on_diagnostic: StreamDiagnosticHook,
  state: Option<Continuation<T>>,
  meta: StreamMeta
}

type StreamDiagnosticHook = fn(StreamEvent) -> ()

type StreamEvent =
  | Progress { consumed: usize, produced: usize, lap: Duration }
  | Pending { reason: PendingReason, meta: ContinuationMeta }
  | Error { diagnostic: ParseError, continuation: Option<ContinuationMeta> }

type PendingReason = "Backpressure" | "InputExhausted" | "FeederAwait" | "FeederClosed"
```

- `StreamDriver::pump()` で 1 ステップ進め、`sink` が `Completed`/`Pending` を受け取る。
- `on_diagnostic` により、インクリメンタル解析中の診断を IDE へ取り次げる。

## 6. インクリメンタル再パース

1. 編集差分（byte range + delta）を受け取ったら該当範囲を跨ぐ memo を無効化。
2. `ParserId` 依存グラフで影響範囲を計算し、局所的に `run_stream`/`resume` を再実行。
3. AST ノードは `Span` をキーにロープ状データ構造へ差し替え、元のバッファを維持する。

この手順は `Core.Parse.Streaming` 拡張の `apply_diff`（または同等ヘルパ）で提供することを推奨する。

## 7. 監視とメタデータ

```reml
type StreamMeta = {
  consumed_bytes: usize,
  resume_count: usize,
  lag_nanos: Option<u64>,
  buffer_fill_ratio: Option<f32>
}
```

- `StreamMeta` を監査ログ (`parser.stream`) に添付することで、バックプレッシャやラグを可視化できる。
- CLI/LSP 統合時には `StreamMeta` と `Diagnostic.extensions` を組み合わせ、ユーザーに補完候補や復旧策を提示する。

## 8. 参考実装

- `guides/runtime-bridges.md` にホットリロード／差分適用のワークフロー例を掲載。
- 非同期実行が必要な場合は `Core.Async` 拡張を併用し、`Feeder` を `Future` ベースで実装する。

## 9. RunConfig との統合

ストリーミング実装がバッチランナーと同じ診断品質・復旧性能を維持するために、以下の情報を `RunConfig` と共有する。

- **コメント・互換設定**: `RunConfig.extensions["lex"].profile` と `extensions["config"].compat` をそのまま継承し、字句処理や JSON5 互換モードをストリーミング側で再構成する。入力チャンクを切り替えてもコメントスキップの実装差が生まれない。
- **復旧戦略**: `extensions["recover"].sync_tokens` と `extensions["recover"].notes` を参照し、`Pending` 状態でも同じ同期トークンで回復を図る。`StreamDriver::pump` は `notes=true` のとき `StreamEvent::Pending` に復旧候補を添付し、LSP 側が提案できるようにする。
- **継続ヒント**: `extensions["stream"].resume_hint` を `ContinuationMeta.resume_hint` にコピーし、差分実行とバッチ実行でデマンドヒントを共通化する。`min_bytes`/`preferred_bytes` の推奨値は 0-1 §1.1 の性能指標（10MB 入力を線形時間で処理）を基準に計測する。
- **診断ロケール**: `RunConfig.locale` を尊重し、`StreamOutcome::Completed` の `ParseResult` がバッチ時と同じ翻訳済みメッセージを生成する。

`RunConfig` を共有する方針により、サンプル群が `RunConfig` を省略していた際のコメントスキップや復旧戦略の重複実装を排除し、0-1 §2.2 の診断整合性と §1.1 の性能要件を同時に満たせる。
