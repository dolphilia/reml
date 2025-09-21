# 2.9 実行時基盤（Core.Runtime）ドラフト

> 目的：`RunConfig` や GC/監査・計測といった実行時インフラを抽象化し、パーサランナーや将来の self-host 実装が共通の Capability を利用できるようにする。

## A. GC Capability インターフェイス

```reml
type GcCapability = {
  configure: fn(GcConfig) -> (),
  register_root: fn(RootSet) -> (),
  unregister_root: fn(RootSet) -> (),
  write_barrier: fn(ObjectRef, FieldRef) -> (),
  metrics: fn() -> GcMetrics,
  trigger: fn(GcReason) -> ()
}

type GcConfig = {
  policy: GcPolicy,
  heap_max_bytes: Option<usize>,
  pause_target_ms: Option<f64>,
  profile: Option<GcProfileId>
}

type GcPolicy = "Rc" | "Incremental" | "Generational" | "Region"

type RootSet = {
  stack_roots: List<ObjectRef>,
  global_roots: List<ObjectRef>
}

type ObjectRef = Ptr<Object>
type FieldRef = Ptr<Object>

type GcMetrics = {
  heap_bytes: usize,
  heap_limit: usize,
  last_pause_ms: f64,
  total_collections: u64,
  policy: GcPolicy
}

type GcReason = "Manual" | "Threshold" | "Idle" | "Emergency"
```

- `configure` は `RunConfig.gc` で渡された設定を適用し、ポリシー切替時は内部状態を再初期化する。
- `register_root` / `unregister_root` は実行スレッドのフレーム切替に合わせて呼ばれ、GC が到達可能集合を把握する。
- `write_barrier` はミュータブル構造体の参照更新時に呼び出し、世代別／インクリメンタル GC の整合性を保つ。
- `metrics` は `guides/runtime-bridges.md` の `gc.stats` と同じキーを持つ統計情報を返す。
- `trigger` は外部からの明示的なコレクション要求を受け付け、`GcReason` を監査ログに残す。

## B. Capability レジストリ

```reml
type RuntimeCapabilities = {
  gc: Option<GcCapability>,
  metrics: fn() -> RuntimeMetrics,
  audit: fn(Json) -> ()
}

fn register_runtime(cap: RuntimeCapabilities)

fn with_runtime<T>(f: fn(RuntimeCapabilities) -> T) -> T
```

- ランナーは `with_runtime` を通じて GC や監査ログ出力にアクセスする。Capability が登録されていなければ `None` を返し、実装はフォールバック動作を選択できる。
- `RuntimeMetrics` は `guides/runtime-bridges.md` で示したメトリクス構造を再利用する。

## C. 実装メモ

1. GC ポリシーごとに `GcCapability` を実装したモジュール（例：`Core.Runtime.Gc.Incremental`）を用意し、`configure` 時に選択する。
2. `write_barrier` の呼び出しコストを抑えるため、バッチ通知 API の検討を行う。
3. `metrics()` はロックレスに取得できることが望ましく、Atomic カウンタで最新値を追跡する。
4. `trigger(GcReason::Emergency)` が発生した場合は `pause_target_ms` 超過を `gc.stats` に記録し、ランナーへ警告を返す。

---

本節はドラフトであり、Core.Runtime を正式に定義する際に改訂する。
