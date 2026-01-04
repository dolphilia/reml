# 2.7 Core.Parse.Streaming

> 目的：`Core.Parse.Streaming` 拡張に属する API を仕様化し、バッチランナー（2.6）と同じ診断品質・復旧戦略を保ったままチャンク入力／継続再開／インクリメンタル解析を実現する。
> 範囲：ストリーミングランナーの公開関数、継続メタデータ、バックプレッシャ制御、RunConfig との統合、監査・診断出力。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | Draft（Core 仕様編入段階・今後の拡張で更新する） |
| 効果タグ | `@pure`（チャンク処理の境界までは純粋）、`effect {runtime}`（ランナー呼び出し）、`effect {io.async}`（非同期 Feeder 利用時） |
| 依存モジュール | `Core.Parse`（2.1〜2.6）、`Core.Diagnostics`（2.5, 3-6）、`Core.Async`（3-9、オプション） |
| 相互参照 | [2.1](2-1-parser-type.md) `RunConfig.extensions`, [2.5](2-5-error.md) 診断・recover, [3-6](3-6-core-diagnostics-audit.md) 監査ログ, [3-8](3-8-core-runtime-capability.md) Capability Registry |

---

## A. ランナー API

### A-1. 署名と戻り値

```reml
fn run_stream<T>(parser: Parser<T>, feeder: Feeder, cfg: StreamingConfig = {}) -> StreamOutcome<T> = todo
fn resume<T>(continuation: Continuation<T>, more: Bytes) -> StreamOutcome<T> = todo
```

* `run_stream` はバッチランナー（2.6 §A）と同じ診断ポリシーを保ちつつ、入力を **チャンク** 単位で処理する。チャンクが不足すると `StreamOutcome::Pending` を返し、継続（Continuation）を通じて再開できる。
* `resume` は前回の `Pending` から追加バイト列 `more` を受け取り続きから再開する。ランナーはチャンク境界で `RunConfig` を再評価しないため、再開時も同じ設定が適用される。

```reml
type StreamOutcome<T> =
  | Completed { result: ParseResult<T>, meta: StreamMeta }
  | Pending { continuation: Continuation<T>, demand: DemandHint, meta: StreamMeta }
```

* `Completed`：`ParseResult<T>`（2.1 §C）と同様に AST・診断・recover 情報を返し、`meta` にストリーム処理の統計値を含める。
* `Pending`：継続と追加入力の要求ヒント（`DemandHint`）を返す。要求が満たされない限りランナーを再入できない。

### A-2. StreamingConfig

```reml
type StreamingConfig = {
  mode: FlowMode = FlowMode::Auto,
  demand_cap: Option<Demand> = None,
  on_diagnostic: StreamDiagnosticHook = default_stream_hook,
  resume_notes: Bool = false,
  extensions: Map<Str, Any> = {}
}
```

* `mode` は FlowController の初期モード（§D）を規定する。`FlowMode::Auto` は Feeder が `FlowCapability` を宣言している場合に自動切替えを許可する。
* `demand_cap` でチャンク要求の上限（バイト数・フレーム数）を設定し、過大なメモリ使用を抑制する。
* `on_diagnostic` は ストリーム処理中に発生した `StreamEvent` を受け取り、IDE や監査ログへ転送するためのフック。既定では監査ログ (`audit.log("parser.stream", ...)`) へ送る。
* `resume_notes` を `true` にすると `Pending` を返す際に `ContinuationMeta.resume_hint` に加えて復旧候補（§C-2）を添付する。
* `extensions` は実装固有オプション（例：バックプレッシャ制御の閾値保存、メモ化バッファ共有）を収容する。

---

## B. Feeder と DemandHint {#feeder-demandhint}

### B-1. 入出力契約

```reml
type DemandHint = {
  min_bytes: usize,
  preferred_bytes: Option<usize>,
  frame_boundary: Option<TokenClass>
}

trait Feeder {
  fn pull(&mut self, demand: DemandHint) -> FeederYield
}

type FeederYield = Chunk | Await | Closed | FeederError

type Chunk = { bytes: Bytes }

type Await = {}

type Closed = {}

type FeederError = { error: StreamError }
```

* `min_bytes` は再開に必要な最小バイト数。満たさないチャンクを渡すとランナーは再び `Pending` を返す。
* `preferred_bytes` は性能上好ましいチャンクサイズを示し、Feeder が対応可能ならそのサイズを供給する。
* `frame_boundary` は構文的境界（TokenClass）を指定し、IDE などが整合した単位でチャンクを生成できるようにする。
* `pull` は **純粋** である必要はないが、`Chunk` を返した場合は `Bytes` が無効化されるまで所有権を保持する。`Await` は非同期入力を待機中であることを示し、呼び出し側は適切な待機機構（`Core.Async` 等）へ切り替える。
* `Closed` は入力終端。`run_stream`/`resume` は `Closed` を受け取った際、残り入力がなければ `Completed`、未完なら `UnexpectedEof` 診断を生成して `Completed` を返す。

### B-2. StreamError

```reml
type StreamError = {
  kind: StreamErrorKind,
  message: Str,
  cause: Option<Json>
}

enum StreamErrorKind = IoFailed | DecoderFailed | FeederBug | UserCancelled
```

* `IoFailed`：基底 I/O が失敗。`cause` に OS エラーコード（`errno` 等）を格納する。
* `DecoderFailed`：チャンクが UTF-8 ではない、または期待するトランスポート形式に合致しない。
* `FeederBug`：Feeder 実装の契約違反（複数の `Chunk` が重複範囲を含む等）。
* `UserCancelled`：外部キャンセルトークンによって中断。

`StreamError` は `on_diagnostic` フック経由で `Diagnostic` に変換され、`ParseResult.diagnostics` に追加される。

---

## C. 継続とメタデータ

### C-1. Continuation 型

```reml
type Continuation<T> = {
  state: Opaque,
  parser: Parser<T>,
  config: RunConfig,
  meta: ContinuationMeta
}

struct ContinuationMeta {
  commit_watermark: usize,
  buffered: Input,
  resume_hint: Option<DemandHint>,
  expected_tokens: Set<Expectation>,
  last_checkpoint: Option<Span>,
  trace_label: Option<Str>,
  resume_lineage: List<Str>
}
```

* `state` は実装依存のシリアライズ不可な継続データ。`parser` と `config` を保持し、`resume` 時に同一環境で実行されることを保証する。
* `commit_watermark` は Packrat メモを安全に破棄できる位置（2.6 §C-2）。バッチランナーは `RunConfig.extensions["stream"].checkpoint` を介してこの値と同期する。
* `buffered` には未処理の入力（`Input` ビュー）が格納され、`resume` はこのバッファと新しい `Bytes` を結合して再解析する。
* `expected_tokens` と `last_checkpoint` は 2.5 で定義される期待集合・同期点を反映し、IDE/LSP 補完の根拠となる。
* `resume_hint` は Feeder が最適なチャンクを供給するためのヒントであり、`StreamOutcome::Pending.demand` と一致している必要がある。
* `resume_lineage` は Pending/Resume の履歴を格納し、`parser.stream.outcome_consistency` で逸脱が発生した際に根拠ログとして共有する。[^resume-lineage-phase27]

### C-2. 復旧と注釈

`resume_notes=true` の場合、`Pending` を返す際に `ContinuationMeta.expected_tokens` から `Diagnostic` 用の注釈を生成し、`StreamEvent::Pending` に `notes` を添付する。IDE はこれを利用して未完入力に対する補完候補を提示できる。

---

## D. フロー制御とバックプレッシャ {#flow-controller}

```reml
type FlowController = {
  mode: FlowMode,
  high_watermark: usize,
  low_watermark: usize,
  policy: FlowPolicy
}

enum FlowMode = Auto | Push | Pull | Hybrid

type FlowPolicy =
  | Manual { on_demand: fn() -> Demand }
  | Auto { backpressure: BackpressureSpec }

type Demand = { bytes: usize, frames: usize }

struct BackpressureSpec {
  max_lag: Option<Duration>,
  debounce: Option<Duration>,
  throttle: Option<Duration>
}
```

* `mode`：`Auto` は Feeder の `FlowCapability` を参照し `Push`/`Pull`/`Hybrid` を選ぶ初期モード。`Push` はストリーム側が能動的にチャンクを送る用途（ログ収集等）に適し、`Pull` は IDE の差分適用など必要時にのみ取得する用途向け。`Hybrid` は実行途中で切替え可能。
* `high_watermark`/`low_watermark`：内部バッファの閾値。バッファ量が `high` を超えると Feeder へ抑制、`low` を下回るとチャンク要求を再開する。
* `policy`：Manual モードでは `on_demand` で外部からの明示的な要求を行う。Auto モードではバックプレッシャ仕様を用い、遅延(`lag`)、デバウンス(`debounce`)、スロットリング(`throttle`)を自動適用する。

`FlowController` は `StreamingConfig.mode` と `RunConfig.extensions["stream"].flow`（任意）から初期化する。`resume` 時は新しい FlowController を差し込めるが、`high_watermark >= low_watermark` を満たさないと `StreamErrorKind::FeederBug` を返す。

---

## E. StreamDriver ヘルパ {#streamdriver-helper}

```reml
struct StreamDriver<T, Sink> {
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
  | StreamEventError { diagnostic: ParseError, continuation: Option<ContinuationMeta> }

enum PendingReason = Backpressure | InputExhausted | FeederAwait | FeederClosed
```

* `StreamDriver::pump()`（実装提供）は 1 ステップ進め、`Sink` に `StreamOutcome` を渡す。`pump` が `Pending` を受け取った場合、`FlowController` と `DemandHint` を用いて次のチャンク要求を決定する。
* `on_diagnostic` は進行状況・未完情報・エラーを外部へ伝達する。既定では監査ログへ送出するが、IDE では LSP 通知に変換しても良い。

`Sink` は `Completed`/`Pending` いずれも受け取れるべきであり、`Pending` の場合は継続を保持するか、`on_diagnostic` で通知した後に外部ストレージへ退避させる。

---

## F. インクリメンタル再パース

1. 編集差分（`byte_range`, `delta`）を受け取ったら、該当範囲を跨ぐ Packrat memo を無効化し、`commit_watermark` より前のエントリは維持する。
2. `ParserId` 依存グラフ（2.2 §F-3）で影響範囲を評価し、必要なサブパーサのみ `run_stream`/`resume` で再実行する。
3. AST は `Span` をキーにロープ状データ構造へ差し替え、もとのバッファを維持して差分適用コストを最小化する。
4. 差分適用後は `StreamMeta.resume_count` をインクリメントし、IDE/監査ログへ反映する。

---

## G. 診断・監査・RunConfig との統合

### G-1. StreamMeta

```reml
struct StreamMeta {
  consumed_bytes: usize,
  resume_count: usize,
  lag_nanos: Option<u64>,
  buffer_fill_ratio: Option<f32>,
  memo_bytes: Option<usize>
}
```

* `consumed_bytes`：累計で処理したバイト数。`Completed` 時は最終入力長と一致する。
* `resume_count`：継続再開した回数。差分適用・バックプレッシャ挙動の可視化に利用する。
* `lag_nanos`/`buffer_fill_ratio`：バックプレッシャや待機状況の監視指標。`FlowController` が自動モードのときのみ設定される。
* `memo_bytes`：Packrat キャッシュが保持している概算ヒープサイズ。`parser.stream.outcome_consistency` や `STREAM-POC-PACKRAT` の監査ログに転記する。[^stream-meta-memo-bytes]

### G-2. RunConfig 共有キー

`RunConfig.extensions` は次のキーを予約し、バッチ/ストリームの整合を保証する。

| key | 代表キー | 説明 |
| --- | --- | --- |
| `"stream"` | `checkpoint: Option<Span>`, `resume_hint: DemandHint`, `flow: Option<FlowController>` | ストリームランナーで取得した継続情報をバッチランナーや他プロセスへ共有する。`checkpoint` は `ContinuationMeta.last_checkpoint` と同期する。 |
| `"recover"` | `sync_tokens`, `notes` | 2.5 §B-11 の復旧同期集合。ストリーミングでは `Pending` の補助情報として利用する。 |
| `"lex"` / `"config"` | 2.1 §D-3 参照 | 字句プロファイル・互換モード。チャンク境界が変わっても字句処理の一貫性を維持する。 |

### G-3. 監査イベント

| イベント ID | 内容 |
| --- | --- |
| `parser.stream.progress` | `StreamEvent::Progress` を記録。`consumed_bytes`, `lap`, `buffer_fill_ratio` を添付する。 |
| `parser.stream.pending` | `Pending` が発生した際に継続メタデータを記録。`pending.reason`, `resume_hint` を含める。 |
| `parser.stream.error` | `StreamEvent::Error`。`ParseError` を JSON に変換し、`continuation` が存在すれば `last_checkpoint` を添付する。 |

監査イベントは 3-6 §2.2 の構造化ログ規約に従い、`audit_id` をバッチ処理と共通化する。

---

## H. 互換性とフォールバック

* `run_stream` は常に `run`（2.6 §A）と同じ結果を生成する境界条件を維持しなければならない。チャンクを一度にすべて渡した場合、`Completed.result` は `run` とビット単位で一致する。
* Packrat が無効 (`RunConfig.packrat=false`) でもストリーミングは利用可能だが、`commit_watermark` と差分再利用の効果は限定的になる。
* 左再帰（2.6 §C-3）が有効な場合、ストリーム再開時には最新の種成長状態を継続に含める必要がある。継続が古い `seed` を保持している場合は `StreamErrorKind::FeederBug` として再初期化を要求する。
* `Await` を返す Feeder を利用する場合は `Core.Async` または同等のランタイムが必要となる。`StreamingConfig.mode=FlowMode::Push` で同期実装を強制し、`Await` を禁止する構成も可能。

---

## I. テストと検証

1. **一致性テスト**：同一入力を `run` と `run_stream` に渡し、`ParseResult` が一致することを確認する（診断・recover・警告を含む）。
2. **Pending/Resume テスト**：チャンクサイズを小さく設定して `Pending` を発生させ、`resume` が同じ AST を再構築するかを確認する。`resume_count` が期待値と一致すること。
3. **Backpressure テスト**：`FlowController.policy=Auto` で高低水位を設定し、`FlowController` が適切に `PendingReason::Backpressure` を発行するかを確認する。
4. **エラー伝播**：`FeederYield::Error` を返してランナーが `StreamEvent::Error` を発行するか、`ParseResult.diagnostics` に `StreamErrorKind` を含む診断が追加されるかを確認する。
5. **監査ログ**：`on_diagnostic` を監査シンクに設定し、`parser.stream.*` イベントが 3-6 §2.2 のフォーマットと整合するか検証する。

---

> 補足: 非同期ランナー (`run_stream_async`) は `Core.Async` 章（3-9）で規定する。ここでは同期 API の仕様を定義し、非同期版は上記契約に `Future` ベースの戻り値を適用した拡張として扱う。

[^resume-lineage-phase27]:
    2026-11-04 追記。`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §6.1 で定義された「Packrat キャッシュ共有と KPI 監視」タスクに従い、`ContinuationMeta.resume_lineage` を `parser.stream.outcome_consistency` の失敗ログへ添付する運用を導入した。

[^stream-meta-memo-bytes]:
    2026-11-04 追記。`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §6.1 および `reports/audit/dashboard/streaming.md` を参照。`memo_bytes` は Packrat キャッシュのおおよそのヒープ使用量であり、`STREAM-POC-PACKRAT` リスクの監視指標として転記する。
