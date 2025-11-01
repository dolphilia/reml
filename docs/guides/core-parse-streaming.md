# Core.Parse.Streaming 運用ガイド

> 目的：仕様章 [2.7 Core.Parse.Streaming](../spec/2-7-core-parse-streaming.md) で定義された API に基づき、実務でのチャンク処理・継続再開・監査連携の運用パターンを整理する。
> 注意：ランナー API や型定義の公式仕様は 2.7 に移管済み。本ガイドでは実装・運用時の補足ノート、ベストプラクティス、チェックリストに焦点を当てる。

## 1. ランナー API

*仕様参照: [2-7 §A](../spec/2-7-core-parse-streaming.md#a-ランナー-api)*

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

*仕様参照: [2-7 §B](../spec/2-7-core-parse-streaming.md#feeder-demandhint)*

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

*仕様参照: [2-7 §C](../spec/2-7-core-parse-streaming.md#c-継続とメタデータ)*

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

*仕様参照: [2-7 §D](../spec/2-7-core-parse-streaming.md#flow-controller)*

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

*仕様参照: [2-7 §E](../spec/2-7-core-parse-streaming.md#streamdriver-helper)*

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

*仕様参照: [2-7 §F](../spec/2-7-core-parse-streaming.md#f-インクリメンタル再パース)*

1. 編集差分（byte range + delta）を受け取ったら該当範囲を跨ぐ memo を無効化。
2. `ParserId` 依存グラフで影響範囲を計算し、局所的に `run_stream`/`resume` を再実行。
3. AST ノードは `Span` をキーにロープ状データ構造へ差し替え、元のバッファを維持する。

この手順は `Core.Parse.Streaming` 拡張の `apply_diff`（または同等ヘルパ）で提供することを推奨する。

## 7. 監視とメタデータ

*仕様参照: [2-7 §G](../spec/2-7-core-parse-streaming.md#g-診断・監査・runconfig-との統合)*

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

- `docs/guides/runtime-bridges.md` にホットリロード／差分適用のワークフロー例を掲載。
- 非同期実行が必要な場合は `Core.Async` 拡張を併用し、`Feeder` を `Future` ベースで実装する。

## 9. RunConfig との統合

*仕様参照: [2-7 §G-2](../spec/2-7-core-parse-streaming.md#g-2-runconfig-共有キー)*

ストリーミング実装がバッチランナーと同じ診断品質・復旧性能を維持するために、以下の情報を `RunConfig` と共有する。

- **コメント・互換設定**: `RunConfig.extensions["lex"].profile` と `extensions["config"].compat` をそのまま継承し、字句処理や JSON5 互換モードをストリーミング側で再構成する。入力チャンクを切り替えてもコメントスキップの実装差が生まれない。
- **復旧戦略**: `extensions["recover"].sync_tokens` と `extensions["recover"].notes` を参照し、`Pending` 状態でも同じ同期トークンで回復を図る。`StreamDriver::pump` は `notes=true` のとき `StreamEvent::Pending` に復旧候補を添付し、LSP 側が提案できるようにする。
- **継続ヒント**: `extensions["stream"].resume_hint` を `ContinuationMeta.resume_hint` にコピーし、差分実行とバッチ実行でデマンドヒントを共通化する。`min_bytes`/`preferred_bytes` の推奨値は 0-1 §1.1 の性能指標（10MB 入力を線形時間で処理）を基準に計測する。
- **診断ロケール**: `RunConfig.locale` を尊重し、`StreamOutcome::Completed` の `ParseResult` がバッチ時と同じ翻訳済みメッセージを生成する。

`RunConfig` を共有する方針により、サンプル群が `RunConfig` を省略していた際のコメントスキップや復旧戦略の重複実装を排除し、0-1 §2.2 の診断整合性と §1.1 の性能要件を同時に満たせる。

### 9.1 CLI/LSP 設定との連携手順（Phase 2-5 Step6）

1. CLI では `compiler/ocaml/src/main.ml` の `Run_config.Builder`（仮称）を利用し、`RunConfig` を構築して監査ログに `parser.runconfig.*` メタデータを残す。`parser-runconfig-packrat.json.golden` はこの経路で出力した JSON を保存したものであり、`scripts/validate-diagnostic-json.sh` を通じて仕様スキーマと突合できる。
2. LSP は `tooling/lsp/run_config_loader.ml` で同じ構成を読み込み、`extensions["lex"|"recover"|"stream"]` をそのまま `Core.Parse.Streaming.run_stream` に渡す。`docs/spec/2-1-parser-type.md` §D の脚注 `[^runconfig-ocaml-phase25]` では `with_extension` を用いた共有例を提示している。
3. ストリーミング実装は `RunConfig` から `extensions["stream"].resume_hint` と `RunConfig.locale` を取り出し、`StreamOutcome::Pending` / `Completed` 双方で同じ診断ロケールとデマンドヒントを提供する。`collect-iterator-audit-metrics.py --require-success --source compiler/ocaml/tests/golden/diagnostics/parser/parser-runconfig-packrat.json.golden` を実行すると、RunConfig 拡張が CI 監視に反映されているかを確認できる。
4. 共有設定を追加する場合は `docs/notes/core-parser-migration.md` の TODO リストに追記し、`PARSER-003`（Packrat/左再帰）・`LEXER-002`（Lex シム）・`EXEC-001`（ストリーミング PoC）で再利用できるかをレビューする。

これらの手順により、CLI/LSP/ストリーミングが同一 RunConfig を基点に動作し、Phase 2-5 で導入した指標（`parser.runconfig_switch_coverage` / `parser.runconfig_extension_pass_rate`）を通じて設定の逸脱を検知できる。

### 9.2 Core.Parse.Lex プロファイル共有サンプル（Phase 2-5 Step6）

Phase 2-5 Step6 で OCaml 実装が導入した `Core.Parse.Lex.Bridge` と `Core.Parse.Lex.Api` を利用すると、ストリーミング経路もバッチと同じ `lexeme` / `symbol` / `leading` 処理を共有できる。以下のように `RunConfig` を構築し、`Bridge.derive` で `ConfigTriviaProfile` を復元したうえで `Lexer.read_token` を `Core.Parse.Lex.Api` で包む。

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
    2025-11-30 更新。`Core.Parse.Lex.Bridge` が `RunConfig.extensions["lex"]` と `extensions["config"]` を同期し、`lexeme`/`symbol` が `RunConfig` 由来のプロファイルを利用できるようになった（`compiler/ocaml/src/core_parse_lex.ml:119`, `:170`）。`parser_driver.run` は `lex_pack` と更新済み `RunConfig` を組み合わせて Menhir ドライバを初期化し（`compiler/ocaml/src/parser_driver.ml:170`）、CLI/LSP も同じ `lex.profile` を注入する（`compiler/ocaml/src/main.ml:608`, `tooling/lsp/run_config_loader.ml:130`）。適用率は `lexer.shared_profile_pass_rate` として `tooling/ci/collect-iterator-audit-metrics.py:732` と `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md:215` に記録。

### 9.3 Core.Parse コンビネーター共有（Phase 2-5 Step6）

- `PARSER-003` Step6 で整備された `Core_parse` モジュール経由で、ストリーミング経路にも `rule`/`label`/`cut`/`recover` のメタデータが付与されるようになった。`RunConfig` と Capability を併せて渡すことで、Packrat メモ化や回復同期トークンが CLI/LSP と同じ監査指標（`parser.core_comb_rule_coverage`、`parser.recover_sync_success_rate` 等）に反映される。詳細は `docs/notes/core-parse-api-evolution.md` Phase 2-5 Step6 と `docs/plans/bootstrap-roadmap/2-5-review-log.md` 2025-12-24 エントリを参照。
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

## 10. Phase 2-5 PoC 状態と既知制限

- 2026-01-24 時点で OCaml 実装は `Parser_driver.Streaming.run_stream` / `resume` を実装し、CLI (`--streaming` フラグ)・LSP・CI へ統合済み。`streaming_runner_tests.ml` と `streaming-outcome.json.golden` によりバッチ結果との一致を継続的に検証している。[^exec001-step4]
- Step5 では PoC 状態を公表するため、本ガイドを含む関連文書へ脚注を追加し、`parser.stream.outcome_consistency`・`parser.stream.demandhint_coverage` を `collect-iterator-audit-metrics.py` で収集する運用を開始した。[^exec001-step5]
- 既知の制限として、チャンク処理は依然として内部でバッチランナーを再利用しており Packrat キャッシュ共有やバックプレッシャの自動制御は未実装。`Stream.resume` のエラーパスは監査ログへ転送されず、CLI メトリクス (`Cli.Stats`) との連携も Phase 2-7 のフォローアップとして残っている。[^exec001-limit]

[^exec001-step4]:
    `docs/plans/bootstrap-roadmap/2-5-review-log.md`「EXEC-001 Step4 CLI/LSP/CI 連携（2026-01-24）」を参照。CLI/LSP/CI への統合とゴールデン整備を記録。

[^exec001-step5]:
    `docs/plans/bootstrap-roadmap/2-5-proposals/EXEC-001-proposal.md` Step5 実施記録。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` にストリーミング指標を登録し、CI で `parser.stream.outcome_consistency` / `parser.stream.demandhint_coverage` を監視する手順を追記した。

[^exec001-limit]:
    `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の `EXEC-001` 引き継ぎ項目を参照。Packrat キャッシュ共有、バックプレッシャ自動化、監査ログの Pending/Error 伝送は Phase 2-7 での改善対象。
