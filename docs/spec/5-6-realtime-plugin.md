# 5.6 RealTime Capability プラグイン — Scheduling & High-Precision Timers

> 位置付け: 公式プラグイン（オプション）。リアルタイムスケジューリングや精密タイマーは OS 権限を要求し `effect {realtime}` を含むため、標準APIから切り離して運用審査を前提とする。
>
> ドラフト再整理メモ: 標準ライブラリ移行の対象としては未確定のため、本章はプラグイン維持を前提に再検討中。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（再検討中） |
| プラグインID | `core.realtime` |
| 効果タグ | `effect {realtime}`, `effect {thread}`, `effect {memory}`, `effect {io.timer}`, `effect {security}`, `effect {audit}` |
| 依存モジュール | `Core.Runtime`, [3-18 Core System](3-18-core-system.md), [5-1 System Capability プラグイン](5-1-system-plugin.md), [5-3 Memory Capability プラグイン](5-3-memory-plugin.md), `Core.Diagnostics`, `Core.Numeric & Time` |
| 相互参照 | [3.18 Core System](3-18-core-system.md), [3.8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md), [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [3-5 Core IO & Path](3-5-core-io-path.md) |

## 0.5 改訂案（標準ライブラリとの境界）

- **プラグイン維持**: 高権限・リアルタイム制御は Capability として運用する。
- **標準 API の検討**: 将来必要になった場合は、`Core.Numeric.Time` の上位に安全なタイマー API を追加する。

## 1. RealTimeCapability API

```reml
pub type RealTimeCapability = {
  set_scheduler: fn(SchedulingPolicy, Priority) -> Result<PreviousScheduler, RealTimeError>, // effect {realtime, thread}
  lock_memory: fn(VoidPtr, usize) -> Result<(), MemoryError>,                                 // effect {realtime, memory}
  unlock_memory: fn(VoidPtr, usize) -> Result<(), MemoryError>,                               // effect {realtime, memory}
  sleep_precise: fn(Duration) -> Result<Duration, RealTimeError>,                             // effect {realtime, io.timer}
  create_timer: fn(Duration, TimerHandler) -> Result<TimerHandle, RealTimeError>,             // effect {realtime, io.timer}
  cancel_timer: fn(TimerHandle) -> Result<(), RealTimeError>,                                 // effect {realtime, io.timer}
}
```

- `lock_memory` / `unlock_memory` は `mlock` / `munlock` 相当であり、[5-3 Memory Capability プラグイン](5-3-memory-plugin.md) と連携する。
- `sleep_precise` はナノ秒精度のスリープ。戻り値に実際に経過した時間を返す。
- `create_timer` はリアルタイムタイマーを登録し、`TimerHandler` はバックグラウンドスレッドで実行される。

## 2. 型定義

```reml
pub enum SchedulingPolicy = Normal | Fifo | RoundRobin | Deadline | Custom(Str)

pub type Priority = i32

pub type PreviousScheduler = {
  policy: SchedulingPolicy,
  priority: Priority,
}

pub type TimerHandle = u64

pub type TimerHandler = fn(TimerEvent) -> ()

pub type TimerEvent = {
  handle: TimerHandle,
  scheduled: Duration,
  fired_at: Timestamp,
}
```

- `Deadline` は Linux の `SCHED_DEADLINE` を想定。`Custom` はプラットフォーム固有。
- `TimerHandler` の実装はできるだけ短時間で終了させる必要がある。

## 3. エラーと監査

```reml
pub type RealTimeError = {
  kind: RealTimeErrorKind,
  message: Str,
}

pub enum RealTimeErrorKind = Unsupported | PermissionDenied | InvalidPolicy | TimerOverflow | DeadlineMissed
```

- `DeadlineMissed` はスケジューラが要求を満たせなかった場合に返す。
- 監査ログは `audit.log("realtime.schedule", {...})` 等で記録し、`CapabilitySecurity` と整合させる。

## 4. 使用例ドラフト

- 高優先度タスク: `set_scheduler(SchedulingPolicy::Fifo, 80)` を呼び出し、完了後に `PreviousScheduler` を復元する。
- 精密タイマー: `create_timer` で周期処理を登録し、`sleep_precise` と合わせて JIT コンパイルのヒートアップに利用。

## 5. 今後の拡張

- `clock_nanosleep` ラッパやタイムラインスケジューリング。
- ハードウェアタイマー (`HPET`, `APIC`) との直接連携。
- リアルタイム統計（`missed_deadlines`, `jitter_ns`）の取得 API。

---

*本章はドラフトであり、公式プラグインとしての配布・審査プロセスは `Chapter 4` のエコシステム仕様と連携して今後更新される。*
