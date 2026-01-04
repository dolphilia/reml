# Conductor パターン実践ガイド

Conductor 構文を活用して複数DSLを協調実行する際の設計パターンとベストプラクティスをまとめる。

## 1. 基本構造

```reml
use Core.Resource;

conductor pipeline_app {
  source: SourceDsl = load_source()
    |> with_capabilities(["fs.read"])
    |> with_resource_limits(ResourceLimitSet::new(
      memory = Some(MemoryLimit::mebibytes(128)),
      cpu = Some(CpuQuota::milli_cores(500)),
    ))

  transform: TransformDsl = rule("transform", pipeline_rules)
    |> depends_on([source])
    |> with_execution_plan(strategy: Strategy.parallel())

  sink: SinkDsl = render_output()
    |> depends_on([transform])
    |> with_execution_plan(strategy: Strategy.sequential())

  channels {
    source.items ~> transform.input : Channel<ItemBatch, ItemBatch>
    transform.events ~> sink.consume : Channel<Event, Event>
  }

  execution {
    strategy: "adaptive_parallel"
    backpressure: BackpressurePolicy.adaptive(high_watermark: 1000, low_watermark: 100, strategy: "drop_oldest")
    error_propagation: ErrorPolicy.isolate_with_circuit_breaker
    scheduling: SchedulePolicy.fair_share_with_priority
  }

  monitoring with Core.Diagnostics {
    health_check: every("5s") using dsl_health_probe
    metrics: collect([
      "dsl.latency" -> LatencyHistogram,
      "dsl.throughput" -> CounterMetric,
      "dsl.error_rate" -> RatioGauge
    ])
    tracing: when(RunConfig.trace_enabled) collect_spans
  }
}
```

`Core.Resource` の型を用いて `with_resource_limits` を構成し、メモリと CPU の上限をビルド時に検証可能な形で宣言する。相対指定が必要な場合は `CpuQuota::fraction` などのコンストラクタを利用し、検証エラーを早期に捕捉する。

## 2. 設計パターン

### 2.1 パイプライン構成

- 各 DSL を `rule` + ビルダ関数で構成し、`|>` で宣言的に機能を合成する。
- `depends_on` はコンパイル時に循環を検出するため、DSL ID を厳密に指定する。

### 2.2 チャネル設計

- `Channel`/`Codec` の契約と失敗モードは [Core.Async 仕様 1.4 節](../../spec/3-9-core-async-ffi-unsafe.md#14-2-channel-契約) に従う。このガイドでは、そのパラメータを実運用へ最適化する手順に集中する。
- `buffer_size` と `OverflowPolicy` は仕様で定義された上限を守りつつ、期待スループットに合わせてプロファイルし、`merge_channels` の前後でメトリクスを観測する。

### 2.3 実行ポリシー

- `ExecutionPlan` の契約は [Core.Async 仕様 1.4.3 節](../../spec/3-9-core-async-ffi-unsafe.md#14-3-executionplan-の整合性) に従い、ガイドでは DAG 分析や `RetryPolicy` のチューニング手順を提示する。
- `ErrorPolicy.isolate_with_circuit_breaker` や `RetryPolicy` の数値は、本番ワークロードの負荷テストで決め、`async.plan.invalid` 診断が出ないか CI で検証する。

### 2.4 監視

- Core.Diagnostics で宣言するメトリクスは `dsl.latency`, `dsl.throughput`, `dsl.error_rate`, `dsl.in_flight` の4種を最低限含める。
- `health_check` は Capability Registry 経由で提供されるプローブを利用する。
- `Runtime::execution_scope` を通じて `ExecutionMetricsScope` を取得し、`register_dsl_metrics` と `channel_metrics` を同一スコープで呼び出す。これにより DSL メトリクスとチャネルメトリクスのリソース文脈が一致し、監査ログに `ResourceLimitDigest` が自動連携される。

### 2.5 埋め込み DSL の診断/監査ログ例

`embedded_dsl` で発生した診断は `source_dsl` を持ち、監査ログには `dsl.*` メタデータが付与される。

診断 JSON の例:

```json
{
  "schema_version": "3.0.0-alpha",
  "message": "parser.unexpected_eof",
  "severity": "Error",
  "code": "parser.unexpected_eof",
  "source_dsl": "reml",
  "primary": { "start": 120, "end": 120 },
  "audit_metadata": {
    "dsl.id": "reml",
    "dsl.parent_id": "markdown",
    "dsl.embedding.span": { "start": 120, "end": 240 },
    "dsl.embedding.mode": "ParallelSafe"
  }
}
```

監査ログ（JSON Lines）の例:

```json
{"event":"parse_diagnostic","dsl.id":"reml","dsl.parent_id":"markdown","dsl.embedding.span":{"start":120,"end":240},"dsl.embedding.mode":"ParallelSafe","dsl.embedding.start":"```reml","dsl.embedding.end":"```"}
```

### 2.6 Markdown + Reml の埋め込み例

```reml
let reml_block = embedded_dsl(
  dsl_id = "reml",
  start = "```reml",
  end = "```",
  parser = Reml.Parser.main,
  lsp = Reml.Lsp.server,
  mode = EmbeddedMode::SequentialOnly,
  context = ContextBridge::inherit(["scope", "type_env", "config"])
)

conductor docs_pipeline {
  markdown: Markdown.Parser.main
    |> with_embedded([reml_block])
    |> with_capabilities(["core.parse", "core.lsp"])

  execution {
    parallel markdown
  }
}
```

`with_embedded` は埋め込み DSL の境界情報を親 DSL に登録し、境界内の診断を `source_dsl` 付きで分離する。`EmbeddedMode::SequentialOnly` は親 DSL の順序実行を優先し、`execution` の並列戦略と衝突しない構成を確保する。

### 2.7 埋め込み DSL 運用チェックリスト

- 境界トークン（`start`/`end`）は他の DSL と衝突しない形式に固定し、入力側のエスケープ規約も合わせて定義する。
- `end` 欠落時は `parser.unexpected_eof` を出し、`source_dsl` と `dsl.embedding.*` を含む診断で境界の復帰位置を特定できるようにする。
- `ContextBridge::inherit` は最小限のキーだけ渡し、親 DSL の状態が子 DSL に汚染されないことを確認する。
- `EmbeddedMode` と `execution` の戦略を一致させ、並列不可 DSL を `ParallelSafe` として登録しない。
- `dsl_id` は `conductor` 内で一意に保ち、`Diagnostic.source_dsl` と `AuditEnvelope.metadata["dsl.id"]` の追跡が切れないようにする。

## 3. ベストプラクティス

1. **小さな DSL から統合** — 大規模な DSL を一度に導入せず、段階的に Conductor へ組み込む。
2. **Capability 宣言の明確化** — `with_capabilities`・`with_resource_limits` を全DSLで必須化し、権限忘れを防止。
3. **フォールバック戦略の準備** — `attempt_dsl` や `first_success` コンビネータで冗長化パスを事前に定義。
4. **観測データの活用** — `start_dsl_span` で生成されるトレースIDをログ・アラートと連携させる。
5. **テンプレートプラグインとの連携** — `reml-plugin-dsl-template` が生成する構成をベースに、プロジェクト固有の DSL を追加する。

## 4. トラブルシューティング

| 症状 | 原因例 | 対応 |
| --- | --- | --- |
| DSL 起動順が期待と異なる | `depends_on` を記述していない | 依存関係を追加し、循環チェックを実行する |
| チャネルで型エラー | 仕様 1.4.2 の前提（`Codec` 互換）が破られている | `Codec` を揃えるか `AsyncErrorKind::CodecFailure` の診断で差異を特定する |
| バックプレッシャーが効かない | `ExecutionPlan.backpressure` の閾値が適切でない | 閾値を見直し、`async.plan.invalid` 診断が出ていないか確認する |
| 監視データが欠落 | `ExecutionMetricsScope` を取得せずに DSL/チャネルを起動 | プラグインまたは Conductor `monitoring` セクションで `Runtime::execution_scope` を呼び出し、`register_dsl_metrics` と `channel_metrics` を同じスコープで登録する |

## 5. 参考

- [1-1 構文仕様 B.8節](../../spec/1-1-syntax.md)
- [3-9 Core Async / FFI / Unsafe 1.4節](../../spec/3-9-core-async-ffi-unsafe.md)
- [3-6 Core Diagnostics & Audit 6章](../../spec/3-6-core-diagnostics-audit.md)
- [notes/dsl-plugin-roadmap.md](../../notes/dsl/dsl-plugin-roadmap.md)
