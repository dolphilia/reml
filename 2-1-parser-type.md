# 2.1 パーサ型

> 目標：**小さく強いコア**で、**高品質エラー**と**実用性能（ゼロコピー・Packrat/左再帰対応）**を両立。
> 原則：**純粋（副作用なし）**・**Unicode前提**・**デフォルト安全**。
> スコープ：パーサの**型**・**入出力モデル**・**実行時状態**・**コミット/消費の意味論**を確定します（詳細なエラー統合は *2.5* で掘り下げ）。

---

## A. 主要型

```reml
// コア：パーサは Input を読み、成功/失敗と残り入力を返す純関数
type Parser<T> = fn(&mut State) -> Reply<T>

// 実行結果（consumed/committed の2ビットを明示）
type Reply<T> =
  | Ok(value: T, rest: Input, span: Span, consumed: Bool)
  | Err(error: ParseError, consumed: Bool, committed: Bool)

// 実行状態（不変入力 + 可変の解析状態）
type State = {
  input: Input,                // 現在の入力ビュー（不変データの参照＋オフセット）
  config: RunConfig,           // 実行設定（Packrat 等）
  memo: MemoTable,             // Packrat/左再帰用メモテーブル
  diag: DiagState,             // 最遠エラー等の集約
  trace: TraceState            // 追跡（オフ既定）
}
```

**ポイント**

* `Reply` は **4状態**を表現可能：
  `Ok(consumed=false/true)` / `Err(consumed=false/true, committed=false/true)`
  → `or` の分岐可否や `cut` の挙動を**分岐なし**で実装できる（Parsec 流の *empty/consumed* + *commit*）。
* `span` は **そのパーサが消費した範囲**（`Ok` のみ）。ノード単位の位置取りに使う。

---

## B. 入力モデル `Input`

```reml
type Input = {
  source: SourceId,        // ファイル/文字列単位の識別子
  bytes: Bytes,            // UTF-8 本体（共有参照/COW）
  byte_off: usize,         // 現在の先頭（バイト）
  line: usize,             // 現在の行番号（1-origin）
  column: usize,           // 現在の列（拡張書記素基準、1-origin）
  // 境界キャッシュ（必要時だけ構築、ビュー間で共有）
  cp_index: Option<CpIndex>,    // コードポイント境界表
  g_index: Option<GraphemeIndex>// グラフェム境界表
}
```

* **不変ビュー**：`Input` は参照共有の **ゼロコピー**スライス。`rest` は **オフセットを進めた写像**のみ。
* **位置**は 1.4 の文字モデルに準拠（行=LF 正規化、列=グラフェム）。
* `mark()/rewind()` は `Input` の**スナップショット**で安価に取れる（バックトラックに使用）。

---

## C. スパンとトレース

```reml
type Span = {
  source: SourceId,
  byte_start: usize, byte_end: usize,
  line_start: usize, col_start: usize,
  line_end: usize,   col_end: usize
}

// 成功断片の履歴（IDE/可視化目的）。既定は OFF。
type SpanTrace = List<(name: String, span: Span)>
```

* 既定では **成功スパンのみ**保持（軽量）。
* `.spanned()` コンビネータで **「値 + Span」** を得る（AST への位置付与に使う）。
* `SpanTrace` は実行時 `RunConfig.trace = On` のときのみ収集（オーバーヘッド遮断）。

---

## D. 実行設定 `RunConfig` とメモ

```reml
type RunConfig = {
  exec_mode: "normal" | "packrat" | "hybrid" | "streaming" = "normal",
  require_eof: Bool = false,            // 全消費を要求（parse_all 相当）
  packrat: Bool = false,                // Packrat メモ化を明示的に有効化
  left_recursion: "off" | "on" | "auto" = "auto",
  fuel_max_steps: Option<usize> = None, // 評価ステップ上限（DoS/ループ防止）
  fuel_on_empty_loop: "error" | "warn" = "error",
  packrat_window_bytes: Option<usize> = Some(1 << 20),
  memo_max_entries: Option<usize> = Some(1 << 20),
  trace: Bool = false,
  merge_warnings: Bool = true,
  stream_buffer_bytes: Option<usize> = Some(64 * 1024)
}

type ParserId = u32  // ルール毎に安定ID（rule()/label() が付与）
type MemoKey  = (ParserId, usize /*byte_off*/)
type MemoVal<T> = Reply<T>  // Ok/Err ごと丸ごとキャッシュ
type MemoTable = Map<MemoKey, Any>  // 実装上は型消去（内部用）
```

* **RunConfig の主な項目**
  - `exec_mode` で実行戦略を切替え（詳細は [2.6 実行戦略](2-6-execution-strategy.md)）。
  - `packrat` と `left_recursion` は Packrat メモ化と seed-growing 左再帰を制御。
  - `fuel_max_steps` / `fuel_on_empty_loop` は停止性の安全弁。
  - `packrat_window_bytes` / `memo_max_entries` はキャッシュのメモリ上限。
  - `stream_buffer_bytes` はストリーム入力のリングバッファ既定サイズ。
  - それ以外は 1.1 で説明したエラー報告やトレースの挙動を調整する。
* `rule(name, p)` が **ParserId とラベル**を付与し、Packrat と診断に使う。

---

## E. コミットと消費の意味論

* `consumed`：**入力を1バイト以上前進**したか。
* `committed`：`cut` 境界を**越えた**とマーク（消費の有無に関わらず）。

**合成の基本規則（抜粋）**

* `p.or(q)`：

  * `p` が `Err(consumed=true, _ )` または `Err(_, committed=true)` → **q を試さない**。
  * `p` が `Err(consumed=false, committed=false)` → **q を試す**。
* `p.then(q)`：

  * `p` が `Ok(consumed=*)` → `q` へ続行（`consumed` は合成：`p||q`）
  * `p` が `Err` → そのまま `Err`。
* `cut`：以降で失敗したら **`committed=true`** を返す（期待集合は 2.5 参照）。
* `label("x", p)`：`p` の期待名を `"x"` に差し替え（エラー統合で優先）。

> この規則で **`try` 相当**は不要：`cut` を使わず書けば *empty エラー* として `or` に落ちる。必要なら `recover` を使う。

---

## F. 失敗表現（最小要素：2.5 と両立）

```reml
type ParseError = {
  at: Span,                           // 失敗位置（最狭）
  expected: Set<Expectation>,         // 期待集合（トークン/ラベル/EOF 等）
  context: List<Label>,               // 直近の label からの文脈
  committed: Bool,                    // cut を越えた失敗か
  notes: List<String>                 // 補助（回復やヒント）
}
```

* **Ok/Err に `consumed/committed` を分離**したことで、エラー統合（最遠位置の採用・期待セットの和/差）を**一意に定義**できる（詳細は *2.5*）。

---

## G. ランナー API（外部からの呼び出し）

```reml
fn run<T>(p: Parser<T>, src: String, cfg: RunConfig = {}) -> Result<(T, Span), ParseError>
// 成功時は値と**全体のスパン**（開始〜終了）を返す。
// cfg.require_eof=true なら残余があれば EOF 期待エラーを返す。

fn run_partial<T>(p: Parser<T>, src: String, cfg: RunConfig = {}) -> Result<(T, Input, Span), ParseError>
// 部分パース：残り Input も返す（REPL/トークナイザ向け）。

fn run_stream<T>(p: Parser<T>, feeder: Feeder, cfg: RunConfig = {}) -> Result<StreamOutcome<T>, ParseError>
// ストリーム入力。Pending が返った場合は続きが必要。

fn resume<T>(cont: Continuation<T>, more: Bytes) -> Result<StreamOutcome<T>, ParseError>
// 追加バイトを供給してストリームを再開。

type StreamOutcome<T> =
  | Completed { value: T, span: Span, meta: StreamMeta }
  | Pending { continuation: Continuation<T>, demand: DemandHint, meta: StreamMeta }

type StreamMeta = {
  consumed_bytes: usize,
  resume_count: usize,
  lag_nanos: Option<u64>,
  buffer_fill_ratio: Option<f32>
}

type DemandHint = {
  min_bytes: usize,
  preferred_bytes: Option<usize>,
  frame_boundary: Option<TokenClass>
}

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

type Feeder = {
  pull: fn(hint: DemandHint) -> FeederYield
}

type FeederYield =
  | Chunk(Bytes)
  | Await
  | Closed
  | Error(StreamError)

type StreamError = { kind: String, detail: Option<String> }
```

* `StreamOutcome::Pending` が返った場合は `Continuation` と `demand` を参照し、必要量を供給して `resume` を呼び出す。
* `Feeder.pull` は `DemandHint` を入力とし、`FeederYield::Chunk` でバイト列を返し、`::Await` でバックプレッシャ、`::Closed` で終端、`::Error` でストリームエラーを通知。
* **ゼロコピー**：`src` は `Input.bytes` へ **参照共有**。
* 文字モデル（1.4）により、列は**グラフェム**、`Span` は**バイトと行列の両方**を保持。

### G-1. ストリーム補助型（ドラフト）

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

type StreamDiagnosticHook = fn(StreamEvent) -> ()

type StreamEvent =
  | Progress { consumed: usize, produced: usize, lap: Duration }
  | Pending { reason: PendingReason, meta: ContinuationMeta }
  | Error { diagnostic: ParseError, continuation: Option<ContinuationMeta> }

type PendingReason = "Backpressure" | "InputExhausted" | "FeederAwait" | "FeederClosed"
```

* `StreamDriver` は 2-6 節で説明する `pump`/`resume` の制御ループをカプセル化し、バックプレッシャや診断イベントを一元管理するためのヘルパ。
* `FlowController` の `mode` と `policy` は IDE 向けの pull 型、ゲーム/リアルタイム向けの push 型、混合運用を切り替える。
* `StreamDiagnosticHook` は `StreamEvent` を受け取り、監査ログ出力や IDE 連携に利用する。

---

## H. 代数則（使用者向けの直観）

* **純度**：`Parser<T>` は参照透過（同じ `State` → 同じ `Reply`）。
* **Functor**：`map` は恒等・合成を保つ。
* **Applicative/Monadic**：`then/andThen` は結合律を満たす（エラー統合規則の範囲で）。
* **`or` の単位**：`fail("x")` は空失敗（`consumed=false, committed=false`）。
* **`cut`**：`label("x", cut(p))` で「ここから先は x を期待」を強制。

---

## I. プラグイン登録と Capability

> DSL プラグインを登録し、Parser capability を管理するための標準 API を定義する。

```reml
type CapabilitySet = Set<String>

type PluginCapability = {
  name: String,
  version: SemVer,
  traits: Set<String>,
  since: Option<SemVer>,
  deprecated: Option<SemVer>
}

type PluginRegistrar = {
  register_schema: fn(name: String, schema: Any) -> (),
  register_parser: fn(name: String, factory: fn() -> Parser<Any>) -> (),
  register_capability: fn(CapabilitySet) -> ()
}

type ParserPlugin = {
  name: String,
  version: SemVer,
  capabilities: List<PluginCapability>,
  dependencies: List<PluginDependency>,
  signature: Option<PluginSignature>,
  register: fn(PluginRegistrar) -> ()
}

fn register_plugin(plugin: ParserPlugin) -> Result<(), PluginError>
fn with_capabilities<T>(cap: CapabilitySet, p: Parser<T>) -> Parser<T>
fn register_bundle(bundle: PluginBundle) -> Result<(), PluginError>
fn verify_plugin(plugin: &ParserPlugin, policy: VerificationPolicy) -> Result<(), PluginWarning>

type PluginDependency = {
  name: String,
  version_req: VersionReq,
  required_capabilities: CapabilitySet
}

type VersionReq = {
  predicate: String
}

type PluginBundle = {
  name: String,
  version: SemVer,
  plugins: List<ParserPlugin>,
  manifest: BundleManifest
}

type BundleManifest = {
  description: Option<Str>,
  checksum: Hash256,
  signed_by: Option<PluginSignature>
}

type PluginSignature = {
  algorithm: "ed25519" | "rsa-pss",
  certificate: Bytes,
  issued_to: Str,
  valid_until: Option<Timestamp>
}

type PluginError =
  | MissingCapability { name: String }
  | MissingDependency { name: String, required: VersionReq }
  | Conflict { plugin: String, existing: SemVer, incoming: SemVer }
  | RegistrationFailed { reason: String }
  | VerificationFailed { plugin: String, reason: String }

type PluginWarning =
  | DeprecatedCapability { name: String, deprecated: SemVer }
  | ExpiringSignature { plugin: String, valid_until: Timestamp }
```

* `register_plugin` はプラグインが提供する DSL/コンビネータを登録し、`PluginRegistrar` 経由で `ParserId` を割り当てる。
* `register_bundle` は署名付きのバンドルを一括登録し、依存解決・バージョン整合性・署名検証を順に適用する。
* `CapabilitySet` は `parser.requires({"template"})` のような照会・制約に利用。
* `with_capabilities` はプラグインが要求する capability を宣言し、満たされない場合 `PluginError::MissingCapability` を返す。
* `verify_plugin` は署名・証明書チェーン・ハッシュを検証し、失効間近の場合は `PluginWarning::ExpiringSignature` を返す。

### I-1. 互換性とバージョン

* `SemVer` 準拠で互換性チェックを行い、競合時は `PluginError::Conflict { plugin, existing }` を返す。
* `PluginCapability` の `since` / `deprecated` により、利用側が警告やフェーズアウトを制御できる。

### I-2. サンプル

```reml
// 既存プラグイン（例: 基本構文サポート）
let syntaxPlugin = ParserPlugin {
  name = "Reml.Core.Syntax",
  version = SemVer(1, 5, 0),
  capabilities = [],
  dependencies = [],
  register = |reg| { /* ... */ }
}

let templating = ParserPlugin {
  name = "Reml.Web.Templating",
  version = SemVer(1, 2, 0),
  capabilities = [
    { name = "template", version = SemVer(1,0,0), traits = {"render"}, since = Some(SemVer(1,0,0)), deprecated = None }
  ],
  dependencies = [
    { name = "Reml.Core.Syntax", version_req = VersionReq{ predicate = "^1.5" } }
  ],
  signature = Some(load_signature("templating.sig")),
  register = |reg| {
    reg.register_schema("TemplateConfig", templateSchema);
    reg.register_parser("render", || renderParser);
  }
}

verify_plugin(&templating, VerificationPolicy::Strict)?
register_plugin(templating)?

let render = with_capabilities({"template"}, renderParser)

let bundle = PluginBundle {
  name = "reml-web-bundle",
  version = SemVer(1, 0, 0),
  plugins = [templating, syntaxPlugin],
  manifest = {
    description = Some("Web テンプレート DSL 一式"),
    checksum = Hash256::from_file("bundle.sha256"),
    signed_by = Some(load_signature("bundle.sig"))
  }
}

register_bundle(bundle)?
```

## J. メモリと性能（実装規約）

* **Input**：COW/RC・SSO（短文字列インライン）・部分文字列は親バッファ参照。
* **Span**：必要最小を保持。`SpanTrace` は OFF 既定。
* **Packrat**：

  * キーは `(ParserId, byte_off)`、値は `Reply<T>`。
  * LRU/リングで上限を設け、巨大入力でのメモリ爆発を回避。
* **左再帰**：`left_recursion=true` のとき、既知の **種別変換法**（seed-growing）を使用（ルールに `ParserId` が必須）。
* **ステップ上限**：`fuel_max_steps` で無限ループ検出（診断に直近のルール列を含める）。

### J-4. Core.Async（ドラフト）

```reml
type Poll<T> =
  | Ready(T)
  | Pending { wake: Waker }

type Waker = fn() -> ()

type AsyncContext = {
  task_id: TaskId,
  scheduler: SchedulerHandle
}

type TaskId = Uuid
type SchedulerHandle = Opaque

type Future<T> = {
  poll: fn(&mut AsyncContext) -> Poll<T>
}

type Task<T> = {
  id: TaskId,
  join: fn() -> Future<Result<T, Cancelled>>,
  cancel: fn(CancelToken) -> (),
  span: Option<Span>
}

type Cancelled = {
  reason: Option<String>
}

type CancelToken = {
  request: fn() -> (),
  is_cancelled: fn() -> Bool
}

type AsyncFeeder = fn(DemandHint) -> Future<FeederYield>

fn run_stream_async<T>(p: Parser<T>, feeder: AsyncFeeder, cfg: AsyncConfig = {})
  -> Task<Result<T, ParseError>>

type AsyncConfig = {
  executor: SchedulerHandle,
  max_inflight: usize,
  backpressure: BackpressureSpec,
  diagnostics: StreamDiagnosticHook,
  cancellation: CancelToken
}
```

* `Future` は `poll` ベースで定義し、`Poll::Pending` の際に `wake` を登録したスケジューラへ通知する。
* `Task` は構造化並行性を想定し、`CancelToken` 経由で子タスクへキャンセルを伝播させる。
* `AsyncFeeder` は `DemandHint` を受け取り非同期に `FeederYield` を返す。`run_stream_async` は `StreamOutcome` を内部で逐次処理しつつ `Task` を返却する。
* `AsyncConfig.backpressure` は `FlowController` と同一の `BackpressureSpec` を共有し、同期ランナーと診断情報を揃える。

---

## K. ミニ例（意味論の確認）

```reml
// トークン
let sym = |s: Str| rule("sym(" + s + ")", Lex.symbol(sc, s))

// 式: atom ('*' atom)*
let atom: Parser<i64> =
  rule("atom",
    (Lex.int(10).map(|n| n)                      // Ok(..., consumed=true)
     .or(sym("(").then(expr).then(sym(")")).map(|(_,v,_)| v))  // 括弧
     .or(label("number or '('", fail())))        // 空失敗 → or が次を試す
  )

let term: Parser<i64> =
  rule("term",
    atom.andThen( many( sym("*").cut().then(atom) ) )
        .map(|(h, tail)| tail.fold(h, |a, (_,b)| a * b))
  )
// '*' の直後に cut → 以降の err は committed=true になり、
// `atom or (...)` に戻らず “ここは '*' の右項が必要” と報告される。
```

---

## K. 仕様チェックリスト

* [ ] `Reply` は **Ok/Err × consumed/committed** を表現（4状態）。
* [ ] `Input` は UTF-8/COW、行=LF正規化、列=グラフェム、**ゼロコピー**。
* [ ] `Span` は**開始/終了の行列＋バイト**を保持。
* [ ] `run / run_partial / run_stream / resume` の外部 API を定義（`require_eof` やストリーム継続など）。
* [ ] `RunConfig` で **Packrat/左再帰/トレース**を切替。
* [ ] `rule(name, p)` で **ParserId/ラベル**を付与（Packrat & 診断）。
* [ ] `or/then/cut/label` の**合成規則**を確定。
* [ ] メモ上限・ステップ上限の**安全弁**を持つ。
* [ ] 文字モデル（1.4）と**列=グラフェム**で位置整合。
* [ ] すべて**純関数**（1.3 効果）— 外界作用は `Parser` の外で扱う。

---

### まとめ

* `Parser<T> = fn(&mut State) -> Reply<T>` による**最小核**に、

  * **consumed/committed** の2ビット、
  * **ゼロコピー入力と正確な Span**、
  * **Packrat/左再帰/トレース**の *ON/OFF* を備え、
* **書きやすさ**（`cut/label` が直観的）、**読みやすさ**（`rule` 命名・位置情報）、**エラー品質**（期待集合×最遠位置）、**性能**（メモ化・ゼロコピー）を同時に満たします。

この 2.1 を土台に、次は **2.2 コア・コンビネータ**で API の最小公理系を詰めましょう。
