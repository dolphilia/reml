# ランタイム連携ガイド

> 目的：FFI・ホットリロード・差分適用など実行基盤との橋渡しを行う際の指針を示す。ここで言及する `config` / `audit` / `runtime` 等の効果タグは Reml コアの5効果に追加される拡張タグであり、監査プラグインが提供する属性として実装する。

## 1. FFI 境界の設計

| 対象 | 推奨効果 | 安全対策 |
| --- | --- | --- |
| クラウド API / REST | `network`, `audit` | 署名・リトライ・`audit_id` で追跡 |
| データベース | `db`, `audit` | トランザクション境界を型で明示、ロールバックログを出力 |
| GPU / アクセラレータ | `gpu`, `runtime` | `unsafe` 内でハンドル管理、`defer` で解放 |
| 組み込み I/O | `runtime` | レジスタアクセスを DSL 化、割込み制御のチェックリスト |

- `unsafe` ブロックではリソース管理 (`defer`) と `audit` ログを必須とする。
- 効果タグの組み合わせは `1-3-effects-safety.md` の表を参照。

## 2. ホットリロード

```reml
fn reload<T>(parser: Parser<T>, state: ReloadState<T>, diff: SchemaDiff<Old, New>)
  -> Result<ReloadState<T>, ReloadError>
```

| ステップ | 説明 |
| --- | --- |
| 1 | `diff` を検証 (`Config.compare`) し、危険な変更を弾く |
| 2 | `applyDiff` で新しいパーサ/設定を構築 |
| 3 | `audit.log("parser.reload", diff)` を出力 |
| 4 | 失敗時は `RollbackInfo` を返却し、`reml-run reload --rollback` で復旧 |

## 3. 差分適用ワークフロー

1. `schema`（2-7）で定義された設定に対し `Config.compare` を実行。
2. 差分 (`change_set`) を `reml-config diff old new` で可視化し、レビューを経て `Config.apply_diff` を実行。
3. `audit_id` を発行し、`guides/config-cli.md` に記載された CLI でログを残す。
4. ランタイム側は `reload` API で新設定を適用、監査ログと照合する。

## 4. CLI 統合

| コマンド | 目的 | 代表オプション |
| --- | --- | --- |
| `reml-run lint <file>` | 構文/設定検証 | `--format json`, `--domain config`, `--fail-on-warning` |
| `reml-run diff <old> <new>` | スキーマ差分 | `--format table`, `--apply`, `--audit` |
| `reml-run reload <state> <diff>` | ランタイム更新 | `--dry-run`, `--rollback`, `--audit` |

```bash
reml-run reload runtime.state diff.json --audit   | jq '.result | {status, audit_id}'
```

## 5. 監査ログ出力

- 構造化ログ例：`{"event":"reml.reload", "audit_id":..., "change_set":...}`。
- CLI と LSP/IDE の診断が同じ `audit_id` を共有することで、エラー追跡と承認フローを一体化できる。

## 6. チェックリストとメトリクス

| 項目 | 内容 | 備考 |
| --- | --- | --- |
| GPU チェック | メモリ割当/解放のペア、カーネル境界での `unsafe` 区切り、`audit_id` を記録 | GPU 温度・エラーイベントを構造化ログに追加 |
| 組み込みチェック | レジスタマップと DSL の整合性、割込みマスクの設定確認、フェイルセーフ手順 | `Config.compare` と `SchemaDiff` を使って差分を検証 |
| ロールバック | `RollbackInfo` を保存し、`reml-run reload --rollback` で復旧する | 監査ログにロールバック結果 (`status`, `audit_id`) を記録 |
| メトリクス統合 | 遅延 (`latency_ms`), エラー率 (`error_rate`), スループットなどを構造化ログに出力 | 監視ツール（Prometheus等）と連携し SLA を監視 |

```reml
type RuntimeMetrics = {
  latency_ms: f64,
  throughput_per_min: f64,
  error_rate: f64,
  last_audit_id: Option<Uuid>,
  custom: Map<Str, Any>
}

fn emit_metrics(event: Str, metrics: RuntimeMetrics) {
  log.json({
    "event": event,
    "audit_id": metrics.last_audit_id,
    "latency_ms": metrics.latency_ms,
    "throughput_per_min": metrics.throughput_per_min,
    "error_rate": metrics.error_rate,
    "custom": metrics.custom
  })
}
```

`RuntimeMetrics` は `guides/data-model-reference.md` で定義する品質指標と同一スキーマを共有し、LSP/CLI の `audit_id` と突合できる。

## 7. GPU 運用フロー

1. **初期化**
   - `gpu::init(device_id)` でデバイスを選択し、`audit.log("gpu.init", device_id)` を記録。
   - ハンドル管理は `unsafe` ブロック内で行い、`defer` で解放を保証。

2. **カーネル実行**
   - `gpu::launch(kernel, params)` を呼び出す前に `runtime` 効果を許可。
   - 実行結果は構造化ログに `latency_ms`, `error_code` を含める。

3. **監視**
   - GPU 温度・エラーイベントを `audit` ログに出力し、監視ツールで収集。
   - `emit_metrics("gpu.kernel", metrics)` でカーネルごとの遅延/エラー率を送信。
   - 重大なエラー時は `reml-run reload --rollback` を使用して安全な状態へ戻す。

## 8. 組み込み運用フロー

1. **レジスタ設定**
   - `config` DSL でレジスタマップを宣言し、`Config.compare` で差分を検証。
   - `runtime` 効果内で `unsafe` を使用し、アクセスは専用 DSL 経由で行う。

2. **割込み制御**
   - 割込みマスクを DSL で宣言し、更新時には `audit.log("interrupt.update", diff)` を記録。
   - フェイルセーフ手順（例: ウォッチドッグリセット）を `Runtime Bridges` のチェックリストに登録。

3. **テレメトリ**
   - 電圧・温度・エラーフラグを構造化ログとして出力し、監視システムに送信。
   - `emit_metrics("embedded.telemetry", metrics)` を用いて SLA 指標を継続監視。
   - フィールド更新失敗時は `ConfigError::ValidationError` を返し、即座にロールバック。

## 9. ストリーミング / async ランナー活用例

### 9.1 ゲームホットリロード（`FlowMode = "push"`）

```reml
let driver = StreamDriver {
  parser = sceneParser,
  feeder = assetWatcher.feeder(),         // ファイル変更をバイト列に変換
  sink = |result| match result {
    Completed { value, meta, .. } => apply_scene_update(value, meta),
    Pending { demand, .. } => log.trace("scene.pending", demand)
  },
  flow = FlowController {
    mode = "push",
    high_watermark = 64 * 1024,
    low_watermark = 16 * 1024,
    policy = Auto { backpressure = { max_lag = Some(16.ms), debounce = Some(4.ms), throttle = None } }
  },
  on_diagnostic = |event| audit.log("parser.stream", event),
  state = None,
  meta = initial_meta()
}

game_loop.on_tick(|dt| {
  driver.flow = driver.flow.adjust(dt);
  driver.pump();
})
```

- アセット変更が頻繁に届くため push モードを採用し、`BackpressureSpec.max_lag` を 16ms に設定してフレーム落ちを防止。
- `StreamMeta` を `apply_scene_update` に渡してホットリロードの統計（再開回数/遅延）を HUD に表示。

### 9.2 IDE 増分解析（`FlowMode = "pull"`）

```reml
fn handle_diff(diff: TextDiff) {
  let demand = DemandHint {
    min_bytes = diff.span.bytes,
    preferred_bytes = Some(diff.span.bytes + 1024),
    frame_boundary = Some(TokenClass::Statement)
  };

  driver.flow = driver.flow.with_mode("pull");
  let chunk = file_cache.patch_and_slice(diff);
  driver.state = Some(resume(driver.state?, chunk.bytes));
  driver.on_diagnostic(Pending { reason = "InputExhausted", meta = driver.state?.meta });
}
```

- エディタ差分で `DemandHint` を明示し、必要最小限の再解析バイトを供給。
- `ContinuationMeta.expected_tokens` を LSP の補完エンジンに流し込み、キャレット位置で候補を表示。

### 9.3 Web SSE パイプライン（`run_stream_async`）

```reml
let feeder_async: AsyncFeeder = |hint| async move {
  let chunk = await sse_client.fetch(hint.preferred_bytes.unwrap_or(4096)).await;
  match chunk {
    Ok(bytes) => Poll::Ready(FeederYield::Chunk(bytes)),
    Err(e) if e.is_retryable() => Poll::Pending { wake = retry_after(100.ms) },
    Err(e) => Poll::Ready(FeederYield::Error(StreamError { kind = e.kind(), detail = Some(e.message()) }))
  }
};

let task = run_stream_async(eventsParser, feeder_async, AsyncConfig {
  executor = runtime.executor(),
  max_inflight = 4,
  backpressure = { max_lag = Some(250.ms), debounce = Some(25.ms), throttle = Some(50.ms) },
  diagnostics = |event| log.json(event),
  cancellation = shutdown_token.clone()
});

task.join().await?;
```

- SSE クライアントは `AsyncFeeder` として実装し、`DemandHint` の `preferred_bytes` を尊重してネットワークバッチを最適化。
- `AsyncConfig.backpressure` と監査ログを一元化し、CLI と同じ指標をダッシュボードに送信。
- `shutdown_token` を用いてデプロイ時に安全にタスクを停止する。

---

## 10. GC プロファイルと監査統合（ドラフト）

### 10.1 プロファイルテンプレート

| プロファイル | ポリシー | 目的 | 推奨設定 |
| --- | --- | --- | --- |
| `game` | Incremental | フレーム落ち回避 | `pause_target_ms = Some(4.0)`, `heap_max_bytes = Some(256 << 20)` |
| `ide` | Generational | インタラクティブ編集 | `pause_target_ms = Some(8.0)`, `heap_max_bytes = None` |
| `web` | Rc | レイテンシより throughput 重視 | `heap_max_bytes = Some(512 << 20)` |
| `data` | Region | バッチ処理で明示的リリース | `pause_target_ms = None`, `heap_max_bytes = Some(2 << 30)` |

`RunConfig.gc.profile` に上記 ID を指定すると、実装は既定値を適用しつつポリシーの上書きを許可する。カスタムプロファイル文字列を指定した場合は、`Core.Runtime` 側で事前登録が必要。

### 10.2 監査ログ `gc.stats`

```json
{
  "event": "gc.stats",
  "policy": "Incremental",
  "profile": "game",
  "heap_bytes": 134217728,
  "heap_limit": 268435456,
  "last_pause_ms": 3.2,
  "total_collections": 42,
  "pause_target_ms": 4.0,
  "run_id": "...",
  "timestamp": "2025-06-14T12:34:56.123Z"
}
```

- ランナーはコレクション完了時に `GcCapability.metrics()` を呼び、上記 JSON を生成して `audit.log("gc.stats", payload)` を実行する。
- `run_id` はホットリロードや長期セッションごとに一意となる識別子。
- `pause_target_ms` は `RunConfig.gc.pause_target_ms` と一致しない場合警告を出す。

### 10.3 監査テストケース

1. **Profile Consistency**: `RunConfig.gc.profile="game"` で起動したセッションが `gc.stats.profile="game"` を報告する。
2. **Emergency Trigger**: `heap_bytes > heap_limit` のタイミングで `GcCapability.trigger("Emergency")` を呼び、監査ログに `reason="Emergency"` を残す。
3. **Pause Budget**: `last_pause_ms > pause_target_ms` の場合、CLI に `gc.pause_budget_exceeded` 警告を表示し、ログに `severity="warn"` を添付する。
4. **Policy Switch**: `policy` を `Generational` に変更した際、初回コレクションログで `policy="Generational"` と出力され、`total_collections=0` から再カウントされる。

### 10.4 互換性チェックリスト

| 項目 | 内容 | 参照 |
| --- | --- | --- |
| `gc.stats` JSON | すべてのフィールドが `guides/runtime-bridges.md#10-2` の例に従うか | 本節 |
| プロファイル既定値 | `RunConfig.gc.profile` が `game/ide/web/data` の場合、テンプレート表の既定値が適用されるか | §10.1 |
| Metrics API | `RuntimeCapabilities.metrics()` が `heap_bytes` 等 GC メトリクスを含む構造体を返すか | 2-9 実行時基盤 |
| Legacy 互換 | GC 設定を指定しない場合でも従来の RC/ヒープ動作が維持されるか | 2-6 実行戦略 |
| 監査連携 | `gc.stats` と `audit.log` のドメインが重複しないこと、既存ログ解析ツールが新フィールドを無視しても動作するか | 監査運用 |
