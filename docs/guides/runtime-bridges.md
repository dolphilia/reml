# ランタイム連携ガイド

> 目的：FFI・ホットリロード・差分適用など実行基盤との橋渡しを行う際の指針を示す。ここで言及する `config` / `audit` / `runtime` 等の効果タグは Reml コアの 5 効果に追加される拡張タグであり、監査プラグインが提供する属性として実装する。
>
> **仕様リンク**: Runtime Bridge の公式契約・Stage ポリシー・監査要件は [3-8 Core Runtime & Capability Registry §10](../spec/3-8-core-runtime-capability.md#runtime-bridge-contract) に統合されました。本ガイドは同節の契約に基づく運用手順とケーススタディを提供します。
>
> **段階的導入メモ**: 実験機能を利用する場合は `reml run -Z<feature>` で opt-in し、`RuntimeBridgeDescriptor.stage` に応じたチェックリストを完了してください。`Experimental` ブリッジはロールバック手順と監査ログ (`bridge.reload` / `bridge.rollback`) を必須とし、`Beta` → `Stable` 昇格時は §10 の Stage 要件に従って `audit.log("bridge.promote", ...)` / `audit.log("bridge.rollout", ...)` を記録します。

## 0. ターゲット同期と `@cfg`

* `Core.Env.infer_target_from_env()`（[3-10](../spec/3-10-core-env.md)）で得たターゲット情報を `RunConfig.extensions["target"]` へマージし、コンパイル時と実行時のプラットフォーム差異を監視する。
* ランタイム起動時は `platform_info()`（[3-8](../spec/3-8-core-runtime-capability.md)）を取得し、`extensions["target"].diagnostics=true` を設定すると `@cfg` 評価のログを `Diagnostic.extensions["cfg"]` に反映できる。`Diagnostic.domain = Target` の詳細から `requested` / `detected` を比較し、クロスリンカ設定の齟齬を特定できる。Phase 2-5 DIAG-003 Step5 で `Target` / `Plugin` / `Lsp` ドメインの監査メタデータと CLI/LSP 出力を再整理し、本ガイドの参照先を仕様と揃えた[^diag003-phase25-runtime-guide]。
* クロスコンパイル時は `reml toolchain verify` と `reml target validate` を実行し、`ffi.callconv.*` を含む `TargetCapability` が満たされているか確認する。手順は `docs/guides/cross-compilation.md` を参照。
* CI では `REML_TARGET_PROFILE`, `REML_TARGET_CAPABILITIES` 等の環境変数をセットし、`Core.Env` が期待通りに解決したか `target.config.*` 診断を確認する。誤ったプロファイルで起動した場合は即座に `Error` を発生させて差異を明らかにする。

## 1. FFI 境界の設計

| 対象 | 推奨効果 | 安全対策 |
| --- | --- | --- |
| クラウド API / REST | `network`, `audit` | 署名・リトライ・`audit_id` で追跡 |
| データベース | `db`, `audit` | トランザクション境界を型で明示、ロールバックログを出力 |
| GPU / アクセラレータ | `gpu`, `runtime` | `unsafe` 内でハンドル管理、`defer` で解放 |
| 組み込み I/O | `runtime` | レジスタアクセスを DSL 化、割込み制御のチェックリスト |

- `unsafe` ブロックではリソース管理 (`defer`) と `audit` ログを必須とする。
- 効果タグの組み合わせは `1-3-effects-safety.md` の表を参照。

### 1.1 クロスリンカと ABI 検証

1. **ターゲットの確定**: `RunConfigTarget.capabilities` に `ffi.callconv.*` が含まれていることを `reml target show` で確認します。未知の Capability がある場合は `target.capability.unknown` を解消してから進めてください。
2. **リンカ設定**: `reml build --emit-metadata build/target.json` から `llvm_triple`・`data_layout` を取得し、外部リンカ（`clang`, `lld`, `link.exe` 等）に同じ値を渡します。値が異なると `target.config.mismatch` が発生します。
3. **検証ステップ**: CI で `reml toolchain verify <profile>` を実行し、結果を `audit.log("toolchain.verify", report)` として保存すると、ランタイム差異のトレースが容易になります。
4. **テスト**: `resolve_calling_convention(platform_info(), metadata)` を用いた単体テストを各ターゲットで実行し、期待する `CallingConvention` が返らない場合は `RunConfigTarget` とプロファイルを再確認してください。

### 1.2 監査ログの整備 (`AuditEnvelope`)

- FFI ブリッジを導入する場合は `AuditEnvelope.metadata.bridge` を必ず出力し、`tooling/runtime/audit-schema.json` で定義されたキー（`status`, `target`, `arch`, `platform`, `abi`, `expected_abi`, `ownership`, `extern_symbol`, `return.*`）を揃えます。
- `bridge.return` には返り値の取り扱いを明示します。Borrowed → `wrap_foreign_ptr`、Transferred → `wrap_foreign_ptr` + `dec_ref` といった処理を `status`・`wrap`・`release_handler`・`rc_adjustment` で追跡し、監査ゲートが参照できるようにします。
- CLI で `remlc --emit-audit` を実行した結果は `compiler/ocaml/tests/golden/audit/cli-ffi-bridge-*.jsonl.golden` に固定し、プラットフォーム別（`linux`, `windows`, `macos-arm64`）の成功ログを最低 1 件ずつ保持します。
- CI では `tooling/ci/collect-iterator-audit-metrics.py` → `tooling/ci/sync-iterator-audit.sh` の流れで `ffi_bridge.audit_pass_rate` を収集します。macOS（`macos-arm64`）の pass_rate が 1.0 未満、もしくはログが欠落している場合はジョブを失敗させ、再取得を促してください。

### 1.3 型付き `CapabilityHandle` の取り扱い

- `CapabilityRegistry::verify_capability_stage` は型付きバリアント (`Gc`/`Io`/`Async` など) を返す設計になったため、FFI 境界では `match handle { CapabilityHandle::Gc(cap) => ... }` あるいは `handle.as_gc()` のようなヘルパを使って目的の API にアクセスしてください。型ごとに `descriptor()` で `stage`/`effect_scope` も利用でき、`docs/spec/3-8-core-runtime-capability.md` の契約と整合する監査ログを出しやすくなります。
- `SecurityCapability` には `SecurityPolicy` を適用する `enforce` メソッドがあり、`AuditEnvelope` に `stage_requirement`/`effect_scope` 情報を追加したい場合は `SecurityCapability` を経由して `audit.log` へ送ってください。具体的な `CapabilityHandle` の分解例とライフサイクルは `docs/guides/reml-ffi-handbook.md#11-3-capability-handle` を参照し、DSL や Bridge 側での型安全な分岐を検証してください。

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
3. `audit_id` を発行し、`docs/guides/config-cli.md` に記載された CLI でログを残す。
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

`RuntimeMetrics` は [3-7-core-config-data.md](../spec/3-7-core-config-data.md#43-プロファイル別評価とメトリクス) で定義する品質指標と同じフィールドを共有し、LSP/CLI の `audit_id` と突合できる。

- FFI ブリッジ固有の計測はランタイム API（`reml_ffi_bridge_get_metrics`, `reml_ffi_bridge_pass_rate`）経由で取得し、`ffi_bridge.audit_pass_rate` と同期させる。`runtime/native/src/ffi_bridge.c` の出力を CI ログへ取り込み、`reports/ffi-bridge-summary.md` のチェック項目に反映する。

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

3. **テレメトリ**
   - 電圧・温度・エラーフラグを構造化ログとして出力し、監視システムに送信。
   - `emit_metrics("embedded.telemetry", metrics)` を用いて SLA 指標を継続監視。
   - フィールド更新失敗時は `ConfigError::ValidationError` を返し、即座にロールバック。

## 9. WASM / クラウド連携

### 9.1 WASI での実行

```reml
fn run_wasi(parser: Parser<T>, bytes: Bytes) -> Result<T, Diagnostic> =
  wasm_run(parser, bytes, RunConfig { left_recursion = "off", packrat = false, ..default });
```

- `docs/guides/portability.md` のチェックリストに従い、`RunConfig.left_recursion` と `packrat` の既定値を `off` にし、`RunConfig.extensions["target"].profile_id = Some("wasi-preview2")` を設定して誤ったランタイムを検知する。
- I/O は WASI 標準の `stdin`/`stdout` のみに限定し、`Core.Env` を通じて環境変数取得を行う。

### 9.2 コンテナ / サーバーレス

- `container_profile("serverless")` をベースに `RunConfig` を初期化し、短時間ジョブ向けに診断を最小化する。
- ローリングデプロイでは `docs/guides/ci-strategy.md` の構造化ログを活用し、`target_config_errors` をダッシュボード表示する。

## 10. ストリーミング / async ランナー活用例

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

### 10.4 Phase 2-5 PoC 監査連携とブリッジ統合

- `RunConfig.extensions["stream"]` から `enabled` / `demand_min_bytes` / `demand_preferred_bytes` / `chunk_size` を Runtime Bridge 初期化時に受け取り、ストリーミング経路と CLI/LSP の設定差分を監視する。`parser.stream.outcome_consistency`・`parser.stream.demandhint_coverage` の集計結果を `audit.log("parser.stream.metrics", ...)` に転送してダッシュボードへ反映する。[^exec001-bridge]
- `DemandHint` と `StreamMeta` を Runtime Bridge のバックプレッシャ制御へ伝播し、`FlowController.policy=Auto` を利用する場合は `BackpressureSpec` をランタイム側へ同期する。`PendingReason::Backpressure` を検出したら `audit.log("parser.stream.pending", { reason, resume_hint })` を出力し、Phase 2-7 で予定している Pending/Error 監査フローと互換にする。
- `StreamEvent::Error` を受信した際は `bridge.stage` 監査と同じフォーマットで `effect.stage.*`／`bridge.reload` を記録し、`Stream.resume` エラーパスが Runtime Bridge 側で検出可能になるよう `AuditEnvelope.metadata["stream.last_reason"]` を必須化する。
- Core.Async 経由で `Await` をハンドリングする構成では、`RuntimeBridgeDescriptor.capabilities` に `{"async.stream"}` を追加し、`effects.contract.stage_mismatch` の監査キーと照合する。`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` で要求される Stage 一致率のレポートにこの Capability を含める。

[^exec001-bridge]:
    `docs/plans/bootstrap-roadmap/2-5-proposals/EXEC-001-proposal.md` Step5 実施記録および `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`「ストリーミング PoC フォローアップ」参照。PoC で導入したストリーミング指標と Runtime Bridge 連携の TODO を列挙。

---

## 10. GC プロファイルと監査統合（ドラフト）

### 10.1 プロファイルテンプレート

| プロファイル | ポリシー | 目的 | 推奨設定 |
| --- | --- | --- | --- |
| `game` | Incremental | フレーム落ち回避 | `pause_target_ms = Some(4.0)`, `heap_max_bytes = Some(256 << 20)` |
| `ide` | Generational | インタラクティブ編集 | `pause_target_ms = Some(8.0)`, `heap_max_bytes = None` |
| `web` | Rc | レイテンシより throughput 重視 | `heap_max_bytes = Some(512 << 20)` |
| `data` | Region | バッチ処理で明示的リリース | `pause_target_ms = None`, `heap_max_bytes = Some(2 << 30)` |

- `RunConfig.extensions["runtime"].gc.profile` に上記 ID を指定すると、実装は既定値を適用しつつポリシーの上書きを許可する。カスタムプロファイル文字列を指定した場合は、`Core.Runtime` 側で事前登録が必要。

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
- `pause_target_ms` は `RunConfig.extensions["runtime"].gc.pause_target_ms` と一致しない場合警告を出す。

### 10.3 監査テストケース

1. **Profile Consistency**: `RunConfig.extensions["runtime"].gc.profile="game"` で起動したセッションが `gc.stats.profile="game"` を報告する。
2. **Emergency Trigger**: `heap_bytes > heap_limit` のタイミングで `GcCapability.trigger("Emergency")` を呼び、監査ログに `reason="Emergency"` を残す。
3. **Pause Budget**: `last_pause_ms > pause_target_ms` の場合、CLI に `gc.pause_budget_exceeded` 警告を表示し、ログに `severity="warn"` を添付する。
4. **Policy Switch**: `policy` を `Generational` に変更した際、初回コレクションログで `policy="Generational"` と出力され、`total_collections=0` から再カウントされる。

### 10.4 互換性チェックリスト

| 項目 | 内容 | 参照 |
| --- | --- | --- |
| `gc.stats` JSON | すべてのフィールドが `docs/guides/runtime-bridges.md#10-2` の例に従うか | 本節 |
| プロファイル既定値 | `RunConfig.extensions["runtime"].gc.profile` が `game/ide/web/data` の場合、テンプレート表の既定値が適用されるか | §10.1 |
| Metrics API | `RuntimeCapabilities.metrics()` が `heap_bytes` 等 GC メトリクスを含む構造体を返すか | 2-9 実行時基盤 |
| Legacy 互換 | GC 設定を指定しない場合でも従来の RC/ヒープ動作が維持されるか | 2-6 実行戦略 |
| 監査連携 | `gc.stats` と `audit.log` のドメインが重複しないこと、既存ログ解析ツールが新フィールドを無視しても動作するか | 監査運用 |

### 10.5 ストリーミング Flow Signal と Runtime Bridge 連携

`RuntimeBridgeRegistry` はストリーミングランナーの状態変化を `stream_signal` で受け取り、Stage 監査に反映する。以下のチェックリストを完了してからブリッジを `Beta` 以上へ昇格させる。

1. **Signal の配線**  
   - ストリーミングランナーが `PendingReason::Backpressure` や `PendingReason::DemandHint` を検出した際に `RuntimeBridgeRegistry::stream_signal(id, signal)` を呼び出し、`signal` には `kind`, `resume_hint`, `demand_min_bytes`, `preferred_bytes`, `last_reason` を含める。  
   - `FlowController.policy = Auto` の場合は必ず Backpressure 情報（`BackpressureSignal { max_lag, debounce, throttle }`）を埋める。CLI/LSP は `stream_meta.bridge.signal` を表示し、監査ログでは `AuditEnvelope.metadata["bridge.stream.signal"]` として保存する。

2. **Stage 監査の統合**  
   - `PendingReason::Backpressure` を受け取ったら `bridge.stage.backpressure` 診断を発生させ、`extensions["bridge"].stage.required` / `stage.actual` を `RuntimeBridgeDescriptor.stage` と比較した値で埋める。  
   - `effects.contract.stage_mismatch` を同じイベントで再利用し、効果ハンドラが要求する Stage と Runtime Bridge Stage の差異を二重に検知する。`Diagnostic.related` に橋渡しした Capability ID を載せることで、Type チームが `TYPE-002` の残課題と突き合わせられる。

3. **Windows/MSVC での検証**  
   - `python3 tooling/ci/collect-iterator-audit-metrics.py --section streaming --platform windows-msvc --require-success` を週次で実行し、`parser.stream.bridge_backpressure_diagnostics` と `parser.stream.bridge_stage_propagation` が 1.0 であることを確認する。  
   - `reports/ffi-bridge-summary.md` の Windows 節に結果を追記し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI ログを更新する。  
   - 失敗時は Stage/Capability の昇格を停止し、`bridge.stage.backpressure` の欠落要因（例えば CLI が `RuntimeBridgeDescriptor.stage` を更新していない等）を `0-4-risk-handling.md` に記録する。

4. **CLI / LSP 連携**  
   - CLI では `--stream-diagnostics` 有効時に `bridge.stage.backpressure` を優先表示し、LSP は `stream_meta` の `bridge.signal` と `resume_hint` を結合してツールチップを生成する。  
   - `collect-iterator-audit-metrics.py` による `parser.stream.bridge_stage_propagation` が 1.0 未満のときは LSP へ警告バナーを表示し、`effects.contract.stage_mismatch` の解消を促す。

## 11. 効果ハンドラと Stage 運用（実験段階）

`-Zalgebraic-effects` を用いてランタイム機能を差し替える際は、ステージ管理と Capability の整合を必ず記録する。

### 11.1 Async 差し替えのチェックリスト

1. **実験フラグを有効化**: `reml run -Zalgebraic-effects test async::collect_logs`.
2. **Capability を opt-in**: `reml capability enable console --stage experimental`.
3. **ハンドラ実装**: `@handles(Console)` と `@requires_capability(stage="experimental")` を併用し、`Diagnostic.extensions["effects"].stage` が `Experimental` であることを確認する。
4. **昇格手順**: テスト完了後、`reml capability stage promote console --to beta` → 再テスト → `--to stable`。CLI は `effects.stage.promote_without_checks` が残っていれば失敗させる。

サンプル CLI 出力（`--effects-debug` 有効時）:

```json
{
  "effects": {
    "stage": "experimental",
    "before": ["io"],
    "handled": ["io"],
    "residual": [],
    "handler": "Console"
  },
  "message": "effects.contract.mismatch resolved"
}
```

### 11.2 FFI サンドボックスと監査

`ForeignCall` 効果を捕捉してモック応答を返す際の標準的なフロー。

```reml
effect ForeignCall : ffi {
  operation call(name: Text, payload: Bytes) -> Result<Bytes, FfiError>
}

@handles(ForeignCall)
@requires_capability(stage="experimental")
fn with_foreign_stub(req: Request) -> Result<Response, FfiError> ! {} =
  handle do ForeignCall.call("service", encode(req)) with
    handler ForeignCall {
      operation call(name, payload, resume) {
        audit.log("ffi.call", { "name": name, "bytes": payload.len() })
        resume(Ok(stub_response(name, payload)))
      }
      return result { result.and_then(decode_response) }
    }
```

昇格時は `reml capability stage promote foreign-call --to beta` を実行し、マニフェスト側の `expect_effects_stage` を同じ値に更新する。監査ログには `stage` と `capability` を必ず含め、運用時に Experimental 機能が残っていないかダッシュボードで確認する。


## 11. Actor / 分散メッセージング

| チェック項目 | 詳細 | 推奨設定 |
| --- | --- | --- |
| Capability 登録 | `RuntimeCapability::AsyncScheduler` と `ActorMailbox` が `CapabilityRegistry` に登録されているか確認。`DistributedActor` を利用する場合は TLS 設定を含む。 | `reml capability list --stage stable` で確認し、欠落時はフォールバック構成を検討 |
| Mailbox 水位 | `ActorSystem.config.mailbox_high_watermark` と `mailbox_low_watermark` を 0-1-project-purpose.md §1.1 の性能基準に合わせて設定。 | ハイ 4096 / ロー 1024 を初期値とし、`AsyncBackpressure` が無い場合は DropNew を有効化 |
| 監査ログ | `audit.log("async.actor.*")` が有効になっているか、`SecurityCapability.permissions` に `Network` が含まれるか。 | `audit` 効果を必須化し、`CapabilityRegistry::stage_of(effect {io.async})` を Stable で運用 |
| トランスポート | `TransportHandle.secure` の TLS/ALPN 設定と再接続ポリシー、`TransportMetrics` からの遅延監視。 | `transport.secure = TLS`、`retry = exponential` を既定にする |

1. `runtime.actor_system()` を初期化する前に `CapabilityRegistry::get("async")` と `CapabilityRegistry::get("actor")` を検証する。欠落時は `async.actor.capability_missing` を生成し、フォールバックパス（逐次実行）を提示する。
2. 分散構成では `route(node, actor)` の結果をキャッシュし、失敗時に `audit.log("async.transport.fail", {...})` を必ず残す。0-1-project-purpose.md §1.2 の安全性基準に従い、暗号化が無い場合は警告を昇格させる。
3. CI では `reml-run lint --domain async --deny experimental` を実行し、`@requires_capability(stage="experimental")` が残っていないか監査する。`DistributedActor` が `experimental` のときは本番ビルドから除外する。
4. IDE/LSP 連携では `ActorContext.span` を `AsyncTracing` から受け取り、`async.trace.latency` メトリクスをダッシュボードに流す。メトリクス未対応ならトレースを無効化し、警告を一度だけ表示する。

> 参考: Reml Actor DSL の追加コード生成フローは `3-9-core-async-ffi-unsafe.md §1.9` を参照。Transport 設定の CLI ワークフローは今後 `docs/guides/runtime-bridges.md` に拡充予定。

[^diag003-phase25-runtime-guide]: Phase 2-5 DIAG-003 診断ドメイン語彙拡張計画（`docs/plans/bootstrap-roadmap/2-5-proposals/DIAG-003-proposal.md`）Step5（2025-11-30 完了）で本ガイドと関連仕様を更新し、`Target` / `Plugin` / `Lsp` ドメインの監査メタデータを `Diagnostic.extensions["target"]`, `["plugin"]`, `["lsp"]` に統一した。CI 監査ダッシュボード改修 TODO は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` で追跡中。
