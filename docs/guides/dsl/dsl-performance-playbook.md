# DSLパフォーマンスプレイブック

DSLファーストアプローチを運用する際の性能測定・最適化手順をまとめる。`5-1-dsl-first-development.md` 第3章および Core.Async/Core.Diagnostics 仕様を前提とする。

## 1. ベンチマーク戦略

### 1.1 マイクロベンチ

- 目的: 個々の Core.Parse コンビネータや DSL ルールの性能計測。
- 手順:
  1. `bench/` ディレクトリに対象ルール単体を呼び出すスクリプトを用意。
  2. 入力サイズを 1KB, 10KB, 100KB, 1MB の4段階で実行。
  3. `dsl.latency` メトリクスをヒストグラム集計し、前回測定との差分を確認。

### 1.2 ミドルベンチ

- 目的: DSL定義→Conductor→外部連携まで含む一連のフローを評価。
- 手順:
  1. `create_channel` を利用した本番同等のチャネル構成を再現。
  2. Backpressure 設定を変化させ、`dsl.in_flight` とエラー率を監視。
  3. Circuit Breaker プラグインを有効にし、障害注入テストを行う。

### 1.3 マクロベンチ

- 目的: エンドツーエンドの利用ケースで性能上限を確認。
- 手順:
  1. 実際の入力データを anonymize したサンプルを用意。
  2. Conductor `execution` ブロックの戦略を複数パターン試験（`adaptive_parallel`, `sequential`, `batch`）。
  3. `dsl.throughput` が要求値を下回った場合、ExecutionPlan を調整し再計測。

## 2. モニタリングとアラート

| 指標 | しきい値例 | アクション |
| --- | --- | --- |
| `dsl.latency` p99 | 500ms | チャネルバッファ調整、DSLルール最適化 |
| `dsl.error_rate` | 1% 超 | Circuit Breaker を半開に、障害 DSL を切り替え |
| `dsl.in_flight` | 平常の 2倍 | BackpressurePolicy を Adaptive に変更 |
| `dsl.throughput` | 要求比 -20% | ExecutionPlan の strategy/scheduling を再計算 |

- アラートは Core.Diagnostics の `record_dsl_failure` をトリガとして発火させ、監査ログと連携させる。

## 3. 最適化のチェックポイント

1. **Core.Parse 層**: Packrat キャッシュサイズ、左再帰最適化、トランポリンの適用状況を確認。
2. **Core.Async 層**: `with_execution_plan` で指定したスケジューリングと実際のランタイム挙動を比較。
3. **Core.Ffi 層**: `auto_bind` された関数の呼び出し時間を計測し、同期ブロッキングを検出。
4. **Diagnostics 層**: メトリクス収集のオーバーヘッドを `observe_backpressure` で測定し、必要ならサンプリング率を調整。

## 4. 手順テンプレート

- `BenchConfig.execution_scope` は 3-8 §4 の `ExecutionMetricsScope` を保持し、メトリクスとリソースリミットを同一文脈で評価できるようにする。

```reml
fn run_dsl_benchmark(config: BenchConfig) -> Result<BenchReport, Diagnostic> = {
  let setup = prepare_environment(config)?;
  let scope = config.execution_scope?;
  let metrics = register_dsl_metrics(&scope, config.dsl_id)?;
  let start = Stopwatch::now();
  let result = execute_dsl(config.plan, config.input)?;
  let elapsed = start.elapsed();

  match result {
    Ok(_) => record_dsl_success(metrics, elapsed),
    Err(diag) => record_dsl_failure(metrics, diag, elapsed),
  }

  collect_report(metrics)
}
```

## 5. 参考リンク

- [5-1 Reml実用プロジェクト開発：DSLファーストアプローチ](../5-1-dsl-first-development.md)
- [3-9 Core Async / FFI / Unsafe](../../spec/3-9-core-async-ffi-unsafe.md)
- [3-6 Core Diagnostics & Audit](../../spec/3-6-core-diagnostics-audit.md)
- [notes/dsl-plugin-roadmap.md](../../notes/dsl/dsl-plugin-roadmap.md)
