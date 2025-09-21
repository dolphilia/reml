# Reml ストリーム／リアクティブ拡張計画案

## 1. 背景整理
- `run_stream`/`resume` は `Feeder` と `Continuation` により逐次供給と停止再開を標準化している `2-6-execution-strategy.md:15` `2-6-execution-strategy.md:158`
- `StreamOutcome::Pending` は入力不足時に継続とリングバッファを返すが、現在はバックプレッシャの契約やエラーメタデータが薄く、IDE/ホットリロード/イベントDSLで運用ガイドを別途書く必要があった `guides/lsp-integration.md:37`
- シナリオ要件では差分パース・ホットリロード・イベント駆動処理が高優先で、標準APIとしてのヘルパとエラーメタデータ整備が求められている `scenario-requirements.md:26` `scenario-requirements.md:57`

## 2. 共通ヘルパ API 案
### 2.1 `StreamDriver`
```reml
type StreamDriver<T, Sink> = {
  parser: Parser<T>,
  feeder: Feeder,
  sink: Sink,                         // 成功時に結果/イベントを受け取るコールバック
  flow: FlowController,               // バックプレッシャ契約
  on_diagnostic: StreamDiagnosticHook,
  state: Option<Continuation<T>>,     // Pending 継続の保持
  meta: StreamMeta                    // 実行メトリクス
}
```
- `StreamDriver::pump()` は `run_stream` を1ステップ実行し、`Pending` の場合は `Continuation` を保持して外部の需要に従って再開。
- `StreamDriver::resume(demand)` は `FlowController` が許可したときにのみ継続を実行。
- `sink` は `Result<T, StreamError>` を受け取り、成功時イベントのストリーム化・累積処理・配信を担当。

### 2.2 `FlowController`
```reml
type FlowController = {
  mode: "push" | "pull" | "hybrid",
  high_watermark: usize,
  low_watermark: usize,
  policy: FlowPolicy
}

type FlowPolicy =
  | Manual { on_demand: fn() -> Demand }
  | Auto { backpressure: BackpressureSpec }

type Demand = { bytes: usize, frames: usize }
```
- IDE/LSPでは `pull` モードを既定とし、`Demand` を差分サイズに応じて供給。
- リアルタイムシナリオでは `hybrid` に設定し、`backpressure` に応じて `FlowController` がバッファ限界前に `FlowEvent::Throttle` をシグナル。

### 2.3 `StreamDiagnosticHook`
```reml
type StreamDiagnosticHook = fn(StreamEvent) -> ()

type StreamEvent =
  | Progress { consumed: usize, produced: usize, lap: Duration }
  | Pending { reason: PendingReason, meta: ContinuationMeta }
  | Error { diagnostic: Diagnostic, continuation: Option<ContinuationMeta> }
```
- `ContinuationMeta` を拡張し、`commit_watermark` と `buffered` だけでなく `frames_waiting`・`expected_tokens` を持たせる（`2-6-execution-strategy.md:15` と `2-5-error.md:40` の情報を再利用）。
- `PendingReason::Backpressure` と `::InputExhausted` を区別してログ出力し、外部制御ループが次の供給戦略を判断できるようにする。

## 3. バックプレッシャ拡張
### 3.1 `run_stream` からの戻り値拡張
```reml
type StreamOutcome<T> =
  | Completed { value: T, meta: StreamMeta }
  | Pending { continuation: Continuation<T>, demand: DemandHint }
  | Failed { error: ParseError, meta: StreamMeta }

type DemandHint = {
  min_bytes: usize,
  preferred_bytes: Option<usize>,
  frame_boundary: Option<TokenClass>
}
```
- `DemandHint` は再開時に必要な最小入力量と理想的バッチサイズを示す。`frame_boundary` はイベントDSLで整合性を保つためのトークンクラスヒント。
- 既存APIとの互換性は `preferred_bytes=None`・`frame_boundary=None` のデフォルトで維持しつつ、`StreamDriver` が存在する場合に新情報を活用。

### 3.2 バックプレッシャ指標
- `StreamMeta` に `lag_nanos`, `buffer_fill_ratio`, `resume_count` を追加し、監査ログ/メトリクス出力に流す。
- `FlowController` はこれら指標をしきい値比較し、`FlowEvent::Drain`（バッファ解放待ち）や `FlowEvent::Burst`（差分集中供給）を呼び出し側へ通知。

## 4. 継続処理のエラーメタデータ
### 4.1 `ContinuationMeta`
```reml
type ContinuationMeta = {
  commit_watermark: usize,
  buffered: Input,
  resume_hint: Option<DemandHint>,
  expected_tokens: List<TokenClass>,
  last_checkpoint: Option<Span>,
  trace_id: Option<TraceId>
}
```
- `expected_tokens` は `ParseError.expected` と揃えて `TokenClass` を保持し、リアクティブUIでの補完候補提示に使用。
- `last_checkpoint` は `attempt` 境界を示し、再開時の差分パッチ範囲を狭めるためのヒント。
- `trace_id` は `2-5-error.md` のトレース拡張と一致させ、ログから継続経路を辿れるようにする。

### 4.2 エラーレポート統合
- `StreamError = ParseError & { continuation: Option<ContinuationMeta> }` と定義し、IDE/LSPとCLIで同一JSONを共有。
- `StreamDiagnosticHook::Error` で `audit.log("parser.stream.error", ...)` を呼ぶテンプレートを提供し、`guides/runtime-bridges.md` の監査規約と一貫化。

## 5. リアクティブ・イベントDSLへの適用
- `StreamDriver` をベースに `Core.Parse.reactive` 名前空間を追加し、`event_loop(parser, sink, cfg)`・`with_timer(interval, handler)` などの糖衣を提供。
- イベントハンドラDSLでは `FlowController.mode="push"` を既定とし、タイムラインDSL（`scenario-requirements.md:29`）やIDEの通知ストリームに組み込む。
- `FlowPolicy::Auto` には `debounce`, `throttle`, `max_lag` を設定できるようにし、バックプレッシャ制御とイベントコアレスを統一。

## 6. 仕様更新・ドキュメント反映予定
- `2-6-execution-strategy.md` に `StreamDriver`・`FlowController` 等の追補節を追加。
- `2-5-error.md` の `ParseError` 拡張に `ContinuationMeta` 連携を明記。
- `guides/lsp-integration.md` と `guides/runtime-bridges.md` に JSON スキーマ更新例とバックプレッシャ制御の実装例を追加。
- `scenario-requirements.md` のホットリロード項で `DemandHint` を参照し、IDE/ゲームシナリオの実行ループをアップデート。

## 7. 次アクション
1. `StreamDriver` プロトタイピング：`run_stream` の戻り値拡張と互換APIの詳細検証。
2. バックプレッシャポリシ―実装ガイド：`FlowPolicy::Auto` の具体例（リングバッファ容量 8KiB、イベント間隔 16ms 等）をドキュメント化。
3. エラーJSON仕様策定：`StreamError` のスキーマを `2-5-error.md` と `guides/lsp-integration.md` へ反映。
4. リアクティブDSLサンプル：イベントタイムラインとホットリロードの擬似コード例を `guides/runtime-bridges.md` に追加する稿を準備。
