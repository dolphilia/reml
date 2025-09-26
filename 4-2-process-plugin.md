# 4.2 Process Capability プラグイン — Native Process & Thread Control

> 位置付け: 公式プラグイン（オプション）。プロセス生成やスレッド制御は `effect {process}` / `effect {thread}` を伴い安全性リスクが高いため、標準API（Chapter 3）から切り離し、導入時に `SecurityCapability` で審査できるよう別章で管理する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（公式プラグイン） |
| プラグインID | `core.process` |
| 効果タグ | `effect {process}`, `effect {thread}`, `effect {io.blocking}`, `effect {signal}`, `effect {hardware}`, `effect {security}` |
| 依存モジュール | `Core.Runtime`, [4-1 System Capability プラグイン](4-1-system-plugin.md), [4-3 Memory Capability プラグイン](4-3-memory-plugin.md), `Core.Diagnostics`, `Core.Config` |
| 相互参照 | [3.8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md), [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [4-4 Signal Capability プラグイン](4-4-signal-plugin.md) |

## 1. ProcessCapability API

```reml
pub type ProcessCapability = {
  spawn_process: fn(Command, Environment) -> Result<ProcessHandle, ProcessError>,      // effect {process}
  kill_process: fn(ProcessHandle, Signal) -> Result<(), ProcessError>,                 // effect {process, signal}
  wait_process: fn(ProcessHandle, Option<Duration>) -> Result<ExitStatus, ProcessError>, // effect {process, io.blocking}
  create_thread: fn(ThreadStart, ThreadOptions) -> Result<ThreadHandle, ThreadError>,  // effect {thread}
  join_thread: fn(ThreadHandle, Option<Duration>) -> Result<ThreadResult, ThreadError>,// effect {thread, io.blocking}
  set_thread_affinity: fn(ThreadHandle, Set<CpuId>) -> Result<(), ThreadError>,        // effect {thread, hardware}
}
```

- `Command` と `Environment` はプロセス生成時のコマンドラインと環境変数を表す構造体。
- `ProcessHandle` / `ThreadHandle` は OS 依存ハンドルをラップし、`Drop` 時の自動解放は行わない（明示的解放を推奨）。
- `Signal` 型は [4-4 Signal Capability プラグイン](4-4-signal-plugin.md) で定義される。

## 2. プロセス生成と監査

```reml
pub type Command = {
  program: Path,
  args: List<Str>,
  cwd: Option<Path>,
  env: Environment,
  stdin: StdioSpec,
  stdout: StdioSpec,
  stderr: StdioSpec,
}

pub enum StdioSpec = Inherit | Null | Pipe | File(Path)
```

- `spawn_process` は `effect {process}` を持ち、必要に応じて `effect {io.blocking}` を追加で表明する。
- 監査ログは `AuditContext` を利用し、プロセス ID・コマンドライン・終了コードを記録する。`CapabilitySecurity.effect_scope` に `audit` を含めることが推奨される。

### 2.1 `spawn_process` の動作例

```reml
fn spawn_pipeline(cmd: Command, audit: AuditSink) -> Result<ProcessHandle, ProcessError> = {
  let ctx = AuditContext::new("process", cmd.program.to_string())?;
  let handle = ProcessCapability::spawn_process(cmd.clone(), Environment::default())?;
  ctx.log("process.spawned", json!({ "pid": handle.pid, "args": cmd.args }))?;
  Ok(handle)
}
```

## 3. プロセス終了待ちと時間制限

```reml
pub type WaitOptions = {
  timeout: Option<Duration>,
  check_interval: Duration,
  collect_output: Bool,
}

fn wait_with_options(handle: ProcessHandle, options: WaitOptions) -> Result<ExitStatus, ProcessError> // effect {process, io.blocking}
```

- タイムアウト発生時は `ProcessErrorKind::TimedOut` を返す。
- `collect_output = true` の場合は `Pipe` を指定した標準出力/標準エラーをバッファし、`effect {memory}`（[4-3](4-3-memory-plugin.md)）を追加で要求する。

## 4. スレッド API

```reml
pub type ThreadStart = fn(ThreadHandle) -> Result<(), ThreadError>

pub type ThreadOptions = {
  name: Option<Str>,
  stack_size: Option<usize>,
  detached: Bool,
  priority: Option<ThreadPriority>,
}

pub enum ThreadPriority = Low | Normal | High | Realtime
```

- `create_thread` は `detached = true` の場合に `join_thread` を呼び出せない旨を明示する。
- `set_thread_affinity` は [4-1 System Capability プラグイン](4-1-system-plugin.md) の `SyscallCapability` を用いて `sched_setaffinity` や `SetThreadAffinityMask` を内部的に呼び出す。

## 5. エラー構造

```reml
pub type ProcessError = {
  kind: ProcessErrorKind,
  message: Str,
  code: Option<i32>,
}

pub enum ProcessErrorKind = SpawnFailed | TimedOut | TerminatedBySignal | InvalidCommand | PermissionDenied | Unsupported

pub type ThreadError = {
  kind: ThreadErrorKind,
  message: Str,
}

pub enum ThreadErrorKind = CreationFailed | JoinFailed | InvalidAffinity | Unsupported
```

- `ProcessError` / `ThreadError` は `IntoDiagnostic` を実装し、監査ログとの統合を容易にする。

## 6. Capability Registry 連携と審査

- `CapabilityRegistry::register("process", CapabilityHandle::Process(...))` により実装を差し込む。
- 依存する Capability: `system`（システムコール）、`memory`（共有メモリ）、`signal`（プロセスシグナル）。
- `effect_scope` は `{process, thread, signal, hardware, audit}` を含めることを推奨。
- 導入時は `SecurityCapability.verify_capability_security` で外部コマンド許可リストやリソース上限を検証する。

## 7. 今後の拡張項目

- プロセスグループ／ジョブ制御 API (`setpgid`, `tcsetpgrp`) の追加。
- `ThreadCapability` の独立化と `Core.Async` との統合ポイント整理。
- プロセス監査のテンプレート (`audited_process`) を追加し、`security_audit` との連携を図る。
- [4-3 Memory Capability プラグイン](4-3-memory-plugin.md) で提供する共有メモリ API との統合ガイドを作成する。

---

*本章はドラフトであり、公式プラグインとしての配布・審査プロセスは `Chapter 4` のエコシステム仕様と連携して今後更新される。*
