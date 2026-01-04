# 3.18 Core System

> 目的：`Core.System` を標準ライブラリの OS 統合窓口として定義し、`Process`/`Signal`/`Env`/`Daemon` の安全な API と Capability ブリッジの境界を明文化する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（Phase 4 対象） |
| 効果タグ | `effect {process}`, `effect {signal}`, `effect {io}`, `effect {io.blocking}`, `effect {security}`, `effect {runtime}` |
| 依存モジュール | `Core.Prelude`, `Core.IO`, `Core.Path`, `Core.Numeric & Time`, `Core.Diagnostics`, `Core.Runtime` |
| 相互参照 | [3-0 Core Library Overview](3-0-core-library-overview.md), [3-8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md), [3-10 Core Env & Platform Bridge](3-10-core-env.md), [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md) |

## 1. 位置付け

- `Core.System` は `Core.Env` を含む OS 連携 API の正準モジュールであり、プロセス・シグナル・環境・デーモン機能を段階的に標準化する。
- 低レベル操作は `Core.Runtime` の Capability Registry（`core.process`, `core.signal`, `core.system`）に委ね、標準 API は安全なラッパを提供する。
- `Core.Env` は互換エイリアスとして残し、`Core.System.Env` を正準 API とする。

## 2. モジュール構成

- `Core.System.Process`: プロセス生成・待機・終了制御の安全 API。
- `Core.System.Signal`: シグナル送受信と詳細情報の整形、監査連携。
- `Core.System.Env`: 環境変数とプラットフォーム情報（`Core.Env` の互換エイリアス）。
- `Core.System.Daemon`: デーモン化・PID 管理・シャットダウンフック（Phase 5 で拡張）。

## 3. Core.System.Process

```reml
module Core.System.Process

pub type ProcessId = Int
pub type ExitStatus = Int
pub type Duration = Core.Numeric.Time.Duration

pub type Command = {
  program: Path,
  args: List<Str>,
  cwd: Option<Path>,
  env: Option<Map<Str, Str>>,
}

pub type SpawnOptions = {
  stdin: Option<Path>,
  stdout: Option<Path>,
  stderr: Option<Path>,
  detach: Bool,
}

pub type ProcessHandle = {
  pid: ProcessId,
  started_at: Option<Core.Numeric.Time.Timestamp>,
}

pub type ThreadHandle = Int
pub type ThreadStart = fn() -> Result<(), ThreadError>

pub type ThreadOptions = {
  name: Option<Str>,
  stack_size: Option<Int>,
  detached: Bool,
}

pub enum ThreadErrorKind = CreationFailed | JoinFailed | Unsupported

pub type ThreadError = {
  kind: ThreadErrorKind,
  message: Str,
}

pub enum ProcessErrorKind = SpawnFailed | PermissionDenied | TimedOut | TerminatedBySignal | Unsupported

pub type ProcessError = {
  kind: ProcessErrorKind,
  message: Str,
  context: Option<Str>,
}

fn spawn(command: Command, options: SpawnOptions) -> Result<ProcessHandle, ProcessError> // effect {process}
fn wait(handle: ProcessHandle, timeout: Option<Duration>) -> Result<ExitStatus, ProcessError> // effect {process, io.blocking}
fn kill(handle: ProcessHandle, signal: Core.System.Signal.Signal) -> Result<(), ProcessError> // effect {process, signal}
fn create_thread(start: ThreadStart, options: ThreadOptions) -> Result<ThreadHandle, ThreadError> // effect {thread}
fn join_thread(handle: ThreadHandle, timeout: Option<Duration>) -> Result<(), ThreadError> // effect {thread, io.blocking}
```

- `spawn` は `core.process` Capability が存在しない場合、`ProcessErrorKind::Unsupported` を返す。
- `kill` は `Core.System.Signal` の `Signal` を使い、Capability が提供する低レベル `Signal` と一致することを要求する。
- 監査ログでは `process.spawn` / `process.wait` / `process.kill` を `AuditEnvelope.metadata` に記録し、`process.pid`, `process.command`, `process.exit_status` を必須メタデータとする。

## 4. Core.System.Signal

### 4.1 型と再エクスポート

```reml
module Core.System.Signal

pub type Signal = Core.Runtime.Signal
pub type SignalInfo = Core.Runtime.SignalInfo

pub enum SignalPayload =
  | UserData(Int)
  | RealTime(Int)
  | Custom(Map<Str, Str>)

pub type SignalDetail = {
  info: SignalInfo,
  timestamp: Option<Core.Numeric.Time.Timestamp>,
  payload: Option<SignalPayload>,
  source_pid: Option<Core.System.Process.ProcessId>,
  raw_code: Option<Int>,
}

pub enum SignalErrorKind = Unsupported | PermissionDenied | TimedOut | InvalidSignal | RuntimeFailure

pub type SignalError = {
  kind: SignalErrorKind,
  message: Str,
}

fn from_runtime_info(info: Core.Runtime.SignalInfo) -> SignalDetail
fn send(pid: Core.System.Process.ProcessId, signal: Signal) -> Result<(), SignalError> // effect {signal, process}
fn wait(signals: Set<Signal>, timeout: Option<Core.Numeric.Time.Duration>) -> Result<SignalDetail, SignalError> // effect {signal, io.blocking}
fn raise(signal: Signal) -> Result<(), SignalError> // effect {signal}
```

- `Signal` は `Core.Runtime.Signal`（`Int`）の型エイリアスとする。
- `SignalInfo` は `Core.Runtime.SignalInfo` を再エクスポートし、標準 API で一貫した型名を維持する。
- `SignalDetail.timestamp` は `Core.Numeric.Time.Timestamp` を参照し、`Core.System` では型の再エクスポートを行わない。

### 4.2 `from_runtime_info` の変換規約

- `from_runtime_info` は `SignalDetail` を生成し、追加情報が得られない場合は `None` を設定する。
- `raw_code` が取得不能・秘匿の場合は `None` を返し、失敗を返さない。
- `payload` は `Core.Runtime` 依存ではなく `Core.System` 側で定義し、ランタイムから該当情報が供給された場合のみ反映する。

### 4.3 `raw_code` 表記と監査マスク

- `raw_code` は OS 依存のシグナル番号を格納し、`Signal` の列挙値と一致しない場合がある。
- Windows では `CTRL_C_EVENT` 等の値を返す可能性があることを明記する。
- `raw_code = None` は「OS 未提供・取得不能・監査ポリシーで隠蔽」のいずれかを意味する。
- 監査ログでは `raw_code` を既定で `masked` とし、`Core.Diagnostics` の監査ポリシーで `signal.raw_code = "allow"` が明示された場合のみ数値を出力する。

## 5. Core.System.Env

`Core.System.Env` は [3-10 Core Env & Platform Bridge](3-10-core-env.md) を正準仕様とし、`Core.Env` は互換エイリアスとして維持する。Phase 4〜5 では `Core.System.Env` を標準 API とし、`Core.Env` は既存コード互換を目的とした再エクスポートに留める。

## 6. Core.System.Daemon（ドラフト）

```reml
module Core.System.Daemon

pub type DaemonConfig = {
  name: Str,
  pid_file: Option<Path>,
  user: Option<Str>,
  group: Option<Str>,
}

fn daemonize(config: DaemonConfig) -> Result<(), ProcessError> // effect {process, security}
fn write_pid_file(path: Path, pid: ProcessId) -> Result<(), ProcessError> // effect {io}
```

- `daemonize` は OS 依存の挙動を持つため、標準 API は最小限の契約のみ定義する。
- 詳細な実装は Phase 5 の `Core.System.Daemon` 拡張で規定し、`core.system` Capability との連携要件を追記する。

## 7. Capability ブリッジと監査

- `Core.System` の API は `CapabilityRegistry` を通じて低レベル実装に接続し、`ProcessCapability` / `SignalCapability` / `SyscallCapability` の Stage と監査ポリシーを継承する。
- Capability が存在しない場合は `Unsupported` 系の `ProcessError` / `SignalError` を返し、`Diagnostic.code = "system.capability.missing"` を付与する。
- `Core.Runtime` の `SignalInfo` とのブリッジは [3-8](3-8-core-runtime-capability.md) の SignalCapability 定義を正準とする。
