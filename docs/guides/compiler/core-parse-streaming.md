# Core.Parse.Streaming 運用ガイド

> 目的：仕様章 [2.7 Core.Parse.Streaming](../../spec/2-7-core-parse-streaming.md) で定義された API に基づき、実務でのチャンク処理・継続再開・監査連携の運用パターンを整理する。
> 注意：ランナー API や型定義の公式仕様は 2.7 に移管済み。本ガイドでは実装・運用時の補足ノート、ベストプラクティス、チェックリストに焦点を当てる。

## Rust 実装対応状況

- 4.1 フェーズ時点の Rust ランタイムはバッチ用 `Parser<T>` コンビネーターと Packrat を実装済みだが、`run_stream` / `resume` を含む Streaming Runner は未提供。CLI/LSP でもストリーミング経路はまだ提供されていない。
- `RunConfig.extensions["lex"]` のプロファイル共有や `Core.Parse.Plugin` 連携も未実装のため、Rust 版では字句トリビアや Capability をストリーミング経路に反映できない。Lex プロファイルや recover 同期トークンを Rust へ橋渡しする手順は `do../../notes/parser/core-parse-api-evolution.md#todo-rust-lex-streaming-plugin` に TODO として記録している。
- Rust でストリーミングを試す場合は既存のバッチ Runner をチャンク単位で呼び直す暫定策しかなく、Packrat 共有やバックプレッシャ計測が欠落することに留意する。

## 1. ランナー API

*仕様参照: [2-7 §A](../../spec/2-7-core-parse-streaming.md#a-ランナー-api)*

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

*仕様参照: [2-7 §B](../../spec/2-7-core-parse-streaming.md#feeder-demandhint)*

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

*仕様参照: [2-7 §C](../../spec/2-7-core-parse-streaming.md#c-継続とメタデータ)*

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
- `expected_tokens` と `last_checkpoint` は IDE 補完や自動復旧に利用され、`trace_label` は SpanTrace (2.5) と連動する。Phase 2-5 ERR-001（期待集合出力整備計画）でストリーミング経路も `ExpectationSummary` を共有するよう更新されたため、`StreamEvent::Error` から `Diagnostic.expected` を取り出すだけで CLI/LSP と同じ候補一覧を提示できる。

## 4. FlowController とバックプレッシャ

*仕様参照: [2-7 §D](../../spec/2-7-core-parse-streaming.md#flow-controller)*

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

### 4.1 親子 DSL のバックプレッシャ協調ベストプラクティス

埋め込み DSL を含むストリーミング解析では、親 DSL と子 DSL の `FlowController` が競合しやすいため、以下の運用を推奨する。

- **DemandHint の伝播**: 親 DSL が `Pending` を返す際、子 DSL の `DemandHint` を統合し `min_bytes` の最大値を採用する。`frame_boundary` は親子で矛盾しない境界のみを保持する。
- **水位共有の最小単位**: `high_watermark` / `low_watermark` は親 DSL のバッファ単位で統一し、子 DSL が独自に拡張バッファを持つ場合は上限を明示してメモリ総量を固定する。
- **Pending 理由の連鎖**: 子 DSL が `PendingReason::Backpressure` で停止した場合、親 DSL も同理由として `StreamEvent::Pending` に記録し、IDE/LSP が二重に待機しないようにする。
- **境界ごとの隔離**: 子 DSL の解析が長時間化する場合は `FlowMode::pull` に切り替え、親 DSL が先に進みすぎないよう `commit_watermark` を境界で固定する。
- **EmbeddedMode の整合**: 子 DSL が `SequentialOnly` または `Exclusive` の場合、親 DSL は同一境界で並列フェッチを行わず、`FlowMode::pull` と `commit_watermark` 固定で順序実行を維持する。
- **監査メタデータ統合**: `StreamMeta.buffer_fill_ratio` と `lag_nanos` は親子 DSL の最大値を採用し、`Diagnostic.extensions["stream.parent_dsl"]` / `["stream.child_dsl"]` に由来を記録する。

## 5. StreamDriver ヘルパ

*仕様参照: [2-7 §E](../../spec/2-7-core-parse-streaming.md#streamdriver-helper)*

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

*仕様参照: [2-7 §F](../../spec/2-7-core-parse-streaming.md#f-インクリメンタル再パース)*

1. 編集差分（byte range + delta）を受け取ったら該当範囲を跨ぐ memo を無効化。
2. `ParserId` 依存グラフで影響範囲を計算し、局所的に `run_stream`/`resume` を再実行。
3. AST ノードは `Span` をキーにロープ状データ構造へ差し替え、元のバッファを維持する。

この手順は `Core.Parse.Streaming` 拡張の `apply_diff`（または同等ヘルパ）で提供することを推奨する。

## 7. 監視とメタデータ

*仕様参照: [2-7 §G](../../spec/2-7-core-parse-streaming.md#g-診断・監査・runconfig-との統合)*

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
- Phase 2-5 ERR-001 の導入以降、`tooling/ci/collect-iterator-audit-metrics.py` は `parser.expected_summary_presence` / `parser.expected_tokens_per_error` を監視し、ストリーミング経路で期待集合が欠落した場合に CI を失敗させる。ストリーミング実装でも `Diagnostic.expected` を欠かさず転送すること。

## 8. 参考実装

*仕様参照: 2.7 §E, §G（監査連携）*

- `../runtimeruntime-bridges.md` にホットリロード／差分適用のワークフロー例を掲載。
- 非同期実行が必要な場合は `Core.Async` 拡張を併用し、`Feeder` を `Future` ベースで実装する。

## 9. RunConfig との統合

*仕様参照: [2-7 §G-2](../../spec/2-7-core-parse-streaming.md#g-2-runconfig-共有キー)*

ストリーミング実装がバッチランナーと同じ診断品質・復旧性能を維持するために、以下の情報を `RunConfig` と共有する。

- **コメント・互換設定**: `RunConfig.extensions["lex"].profile` と `extensions["config"].compat` をそのまま継承し、字句処理や JSON5 互換モードをストリーミング側で再構成する。入力チャンクを切り替えてもコメントスキップの実装差が生まれない。
- **復旧戦略**: `extensions["recover"].sync_tokens` と `extensions["recover"].notes` を参照し、`Pending` 状態でも同じ同期トークンで回復を図る。`StreamDriver::pump` は `notes=true` のとき `StreamEvent::Pending` に復旧候補を添付し、LSP 側が提案できるようにする。
- **継続ヒント**: `extensions["stream"].resume_hint` を `ContinuationMeta.resume_hint` にコピーし、差分実行とバッチ実行でデマンドヒントを共通化する。`min_bytes`/`preferred_bytes` の推奨値は 0-1 §1.1 の性能指標（10MB 入力を線形時間で処理）を基準に計測する。
- **診断ロケール**: `RunConfig.locale` を尊重し、`StreamOutcome::Completed` の `ParseResult` がバッチ時と同じ翻訳済みメッセージを生成する。

`RunConfig` を共有する方針により、サンプル群が `RunConfig` を省略していた際のコメントスキップや復旧戦略の重複実装を排除し、0-1 §2.2 の診断整合性と §1.1 の性能要件を同時に満たせる。

### 9.1 CLI/LSP 設定との連携手順（Phase 2-5 Step6）

1. CLI では `compiler/frontend/src/bin/remlc.rs` の RunConfig 組み立てを通じて `RunConfig` を構築し、監査ログに `parser.runconfig.*` メタデータを残す。`parser-runconfig-packrat.json.golden` はこの経路で出力した JSON を保存したものであり、仕様スキーマと突合できる。
2. LSP 側も同じ構成を読み込み、`extensions["lex"|"recover"|"stream"]` をそのまま `Core.Parse.Streaming.run_stream` に渡す。
3. ストリーミング実装は `RunConfig` から `extensions["stream"].resume_hint` と `RunConfig.locale` を取り出し、`StreamOutcome::Pending` / `Completed` 双方で同じ診断ロケールとデマンドヒントを提供する。CI では RunConfig 拡張が監視に反映されているかを確認する。
4. 共有設定を追加する場合は `do../../notes/parser/core-parser-migration.md` の TODO リストに追記し、`PARSER-003`（Packrat/左再帰）・`LEXER-002`（Lex シム）・`EXEC-001`（ストリーミング PoC）で再利用できるかをレビューする。

これらの手順により、CLI/LSP/ストリーミングが同一 RunConfig を基点に動作し、Phase 2-5 で導入した指標（`parser.runconfig_switch_coverage` / `parser.runconfig_extension_pass_rate`）を通じて設定の逸脱を検知できる。

### 9.2 Core.Parse.Lex プロファイル共有サンプル（Phase 2-5 Step6）

`Core.Parse.Lex.Bridge` と `Core.Parse.Lex.Api` を利用すると、ストリーミング経路もバッチと同じ `lexeme` / `symbol` / `leading` 処理を共有できる。以下のように `RunConfig` を構築し、`Bridge.derive` で `ConfigTriviaProfile` を復元したうえで `Lexer.read_token` を `Core.Parse.Lex.Api` で包む。

```reml
let builder =
  RunConfig::builder()
    .with_extension("lex", {
      profile: "strict_json",
      line: ["//"],
      block: { start: "/*", end: "*/", nested: true },
      shebang: true,
      space_id: ParserId::JsonStrict,
    })
    .with_extension("stream", {
      resume_hint: { min_bytes: 4096 },
    });

let run_config = builder.build();
let (lex_pack, cfg) = Core.Parse.Lex.Bridge.derive(run_config);

// lexer 呼び出しは Core.Parse.Lex.Api で包む
let read_token = fn lexbuf ->
  Core.Parse.Lex.Api.lexeme(lex_pack, Lexer.read_token, lexbuf);
```

`lex_pack` は `ConfigTriviaProfile` や `space_id` を保持し、`cfg` は `extensions["config"].trivia` を同期済みの `RunConfig`。`StreamDriver`／`run_stream` に渡す前に `cfg` を採用し、`read_token` で字句を取得することで、バッチ経路と同じトリビア設定・監査キー（`lexer.shared_profile_pass_rate` 等）をストリーミング側でも確保できる[^stream-lex-bridge-phase25].

[^stream-lex-bridge-phase25]:
    2025-11-30 更新。`Core.Parse.Lex.Bridge` が `RunConfig.extensions["lex"]` と `extensions["config"]` を同期し、`lexeme`/`symbol` が `RunConfig` 由来のプロファイルを利用できるようになった。`parser_driver.run` は `lex_pack` と更新済み `RunConfig` を組み合わせてドライバを初期化し、CLI/LSP も同じ `lex.profile` を注入する。適用率は `lexer.shared_profile_pass_rate` として `do../../plans/bootstrap-roadmap/0-3-audit-and-metrics.md:215` に記録。

### 9.3 Core.Parse コンビネーター共有（Phase 2-5 Step6）

- `PARSER-003` Step6 で整備された `Core_parse` モジュール経由で、ストリーミング経路にも `rule`/`label`/`cut`/`recover` のメタデータが付与されるようになった。`RunConfig` と Capability を併せて渡すことで、Packrat メモ化や回復同期トークンが CLI/LSP と同じ監査指標（`parser.core_comb_rule_coverage`、`parser.recover_sync_success_rate` 等）に反映される。詳細は `do../../notes/parser/core-parse-api-evolution.md` Phase 2-5 Step6 と `do../../plans/bootstrap-roadmap/2-5-review-log.md` 2025-12-24 エントリを参照。
- `Core.Parse.Plugin.with_capabilities` と `RunConfig.extensions["stream"]` を併用し、Packrat/Recover の有効化状態をドライバ間で同期させる。これにより `collect-iterator-audit-metrics.py --require-success` の Packrat 指標や `parser.core.rule.*` メタデータが欠落した際も再現性のあるフィードバックが得られる。

```reml
use Core.Parse
use Core.Parse.Streaming
use Core.Parse.Plugin

let parser =
  Core.Parse.rule("stream.entry",
    syntax_tree()
      |> Core.Parse.recover(
        until = Core.Parse.symbol(space(), ";"),
        with = |_| default_stmt()
      )
  )
    |> Core.Parse.Plugin.with_capabilities({"parser.recover", "parser.trace"})

let run_config =
  RunConfig::builder()
    .with_extension("lex", shared_profile())
    .with_extension("recover", { sync_tokens: [";"], notes: true })
    .with_extension("stream", { resume_hint: { min_bytes: 64 } })
    .finish()

let outcome =
  run_stream(parser, feeder, {
    run_config = run_config,
    flow = default_flow(),
    on_diagnostic = handle_event,
  })
```

この構成により、ストリーミング実行とバッチ実行が同じ `parser.core.rule.*` メタデータと Packrat 指標を共有し、Phase 2-7 で予定されているテレメトリ統合や Menhir 置換判断に必要な計測値を維持できる。

### 9.4 Parser から StreamingParser への変換指針（Phase 11）

- **変換方針**: 既存の `Parser<T>` をそのまま `StreamDriver` / `run_stream` に渡し、`ContinuationMeta.commit_watermark` と Packrat キャッシュを共有する。`rule` で固定した `ParserId` が Stream 経路でも維持されることを前提に、`StreamEvent::Pending` に `expected_tokens` を含めて IDE での補完を可能にする。
- **Lex/autoWhitespace の共有**: autoWhitespace/Layout や `lexeme`/`symbol` の挙動を一致させるため、`RunConfig.extensions["lex"]` と `layout_profile` をストリーミング側へ必ず注入する。`extensions["parse"].operator_table` を用いて演算子優先度を上書きしている場合も同じテーブルを共有し、バッチとストリーミングで期待集合がずれないようにする。
- **Capability/Stage の伝播**: プラグイン経由で `Core.Parse.Plugin.with_capabilities` を利用している場合、`bridge.stage.*` / `effect.capabilities[*]` を `StreamEvent::Error` へ転写し、`RuntimeBridgeAuditSpec`（`do../../spec/3-8-core-runtime-capability.md`）の要求を満たす。署名検証や Stage チェックで失敗した場合は `Pending` を返さずエラー完了とし、復旧経路に乗せない。
- **観測フラグの扱い**: `RunConfig.profile` / `extensions["parse"].profile_output` が有効な場合は、バッチと同じ `ParserProfile` を `StreamOutcome` に含める。`profile_output` の書き出しは best-effort（失敗しても診断に影響しない）であり、Phase 4 シナリオ `CH2-PARSE-902` で JSON 出力経路を監視する。
- **既知の制約**: Rust ランタイムでは Streaming Runner が未実装のため、上記方針は将来の実装指針として扱う。Packrat 共有や `resume` のバックプレッシャ制御は `do../../notes/parser/core-parse-api-evolution.md#todo-rust-lex-streaming-plugin` に記録し、実装時に本節を最新の API 契約へ更新する。

#### Rust 例: layout_token の利用

Rust ランタイムでは `layout_token` を使って Layout 由来の仮想トークンを消費する。`RunConfig.extensions["lex"].layout_profile` で Layout を有効化した上で、`indent`/`dedent` などのトークン名を明示する。

```rust
use reml_runtime::parse::{layout_token, rule, Parser};

fn block(stmt: Parser<Stmt>) -> Parser<Stmt> {
    rule(
        "block",
        layout_token("<indent>")
            .skip_l(stmt)
            .skip_r(layout_token("<dedent>")),
    )
}
```

Rust 側では `RunConfig.extensions["lex"].layout_profile` に `LayoutProfile` を設定して Layout を有効化する。

```rust
use reml_runtime::run_config::RunConfig;
use serde_json::json;

let run_config = RunConfig::default().with_extension("lex", |mut ext| {
    ext.insert(
        "layout_profile".to_string(),
        json!({
            "indent_token": "<indent>",
            "dedent_token": "<dedent>",
            "newline_token": "<newline>",
            "offside": true,
            "allow_mixed_tabs": false
        }),
    );
    ext
});
```

## 10. `decode_stream` と Grapheme 監査の連携

- `Core.Text.decode_stream` は Phase 3 で `TextDecodeOptions` を通じて BOM/不正シーケンス処理を指定できる。Streaming Runner の `on_chunk` で UTF-8 検証が必要になった場合は `decode_stream` を呼び、戻り値の `String`/`Str` をそのまま `Unicode.segment_graphemes` へ渡す。`log_grapheme_stats` の値は `AuditEnvelope.metadata["text.grapheme_stats"]` と CLI/LSP 診断 (`Diagnostic.extensions["text.grapheme_stats"]`) へ転写し、`text.grapheme.cache_hit` KPI は監査メトリクス集計ツールで追跡する。  
- Rust runtime には `compiler/runtime/examples/io/text_stream_decode.rs` があり、`cargo run --manifest-path compiler/runtime/Cargo.toml --bin text_stream_decode -- --input tests/data/unicode/streaming/sample_input.txt --output examples/core-text/expected/text_unicode.stream_decode.golden` でストリーミング decode + Grapheme 統計を JSON へ保存できる。CI ではこのゴールデンを `scripts/validate-diagnostic-json.sh --pattern text_stream_decode` の参照として利用する。  
- `examples/core-text/text_unicode.reml` は `Bytes`→`Str`→`String` の三層モデル、`TextBuilder`, `Unicode.prepare_identifier`, `log_grapheme_stats` を 1 つのシナリオに統合したサンプルであり、`expected/text_unicode.{tokens,grapheme_stats}.golden` に CLI 出力と監査メタデータを保持している。Streaming Runner で `decode_stream` を導入する場合はこのサンプルをベースに自動テストを構築し、`reports/spec-audit/ch1/core_text_examples-YYYYMMDD.md` に実行ログを追加する。

---

## 付録A. `match` の評価順序とストリーミング運用

ストリーミング実装では `StreamOutcome`／`FeederYield` の分岐や、診断フック内での状態更新に `match` を多用する。ここでは、`match` の評価順序（言語仕様）と、ストリーミング運用上の注意点を最小限にまとめる。

*仕様参照: [1-1 §C.4 `match` 式](../../spec/1-1-syntax.md#c4-制御構文), [1-5 §4 `MatchExpr`](../../spec/1-5-formal-grammar-bnf.md)*

### A.1 評価順序（重要）

- スクラティニー（`match expr with ...` の `expr`）は **1 回だけ評価**され、その値に対してアームを上から順に照合する。
- 各アームは **パターン照合 → ガード（`when`）→ エイリアス束縛（`as`/`@`）→ 本体** の順で評価される。
- ガードが偽の場合は、そのアームの本体は評価されず **次のアームへフォールスルー**する。
- 部分アクティブパターン（`(|Name|_|)`）の `None` は「照合失敗」として扱い、**診断を出さずに次アームへ進む**（失敗をエラーとして扱いたい場合は `Result`／診断生成を本体側で明示する）。

### A.2 ストリーミングでの実務的ガイドライン

- `Pending`／`Await` 系の分岐では、**入力要求と状態更新をアーム本体に寄せる**。ガード内で `resume`／`pull` を呼ぶと、分岐条件の見通しが悪化し「どこで入力が消費されたか」を追跡しづらくなる。
- ガードはできるだけ **純粋（副作用なし）** に保つ。`Pending` の有無や `DemandHint` の比較など、軽量な判定に限定する（0-1 §1.1 の性能要件と §1.2 の安全性を満たしやすい）。
- 例外や診断の生成は「最初に一致したアーム」の本体で行い、`None` などのフォールスルーを例外扱いしない（診断ポリシーが必要な場合は `core.parse.*`／`parser.stream.*` の運用キーと整合させる）。

### A.3 代表パターン（`StreamOutcome` 分岐）

```reml
match outcome with
| Completed { result, meta } -> finalize(result, meta)
| Pending { continuation, demand, meta } when demand.min_bytes > 0 ->
    request_more(continuation, demand, meta)
| Pending { continuation, demand, meta } ->
    park(continuation, demand, meta)
```

上例は「`Pending` の詳細条件（`min_bytes`）をガードで切り替え、実際の入力要求・待機処理は本体で完結させる」典型形。`resume` の呼び出しや `Feeder.pull` は本体に閉じ込め、`Pending` 判定が評価順序や再入に依存しないようにする。
