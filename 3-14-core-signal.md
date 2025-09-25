# 3.14 Core Signal — Inter-Process Signals & Handlers (Draft)

> 目的：OS シグナル機構を Reml から利用するための `Core.Signal` API を定義し、`ProcessCapability` / `SyscallCapability` と連携して安全にハンドラ登録・配信・マスク操作を行う。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト |
| 効果タグ | `effect {signal}`, `effect {process}`, `effect {unsafe}`, `effect {audit}`, `effect {security}`, `effect {io.blocking}` |
| 依存モジュール | `Core.Runtime`, `Core.System` (3-11), `Core.Process` (3-12), `Core.Diagnostics`, `Core.Unsafe.Ptr` |
| 相互参照 | [3-8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md), [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md) |

## 1. SignalCapability API

```reml
pub type SignalCapability = {
  register_handler: fn(Signal, SignalHandler) -> Result<PreviousHandler, SignalError>, // effect {signal, unsafe}
  mask: fn(Set<Signal>) -> Result<SignalMask, SignalError>,                             // effect {signal}
  unmask: fn(SignalMask) -> Result<(), SignalError>,                                   // effect {signal}
  send: fn(ProcessId, Signal) -> Result<(), SignalError>,                              // effect {signal, process}
  wait: fn(Set<Signal>, Option<Duration>) -> Result<SignalInfo, SignalError>,          // effect {signal, io.blocking}
  raise: fn(Signal) -> Result<(), SignalError>,                                        // effect {signal}
}
```

- `register_handler` は `unsafe`：シグナルハンドラ内で許可される操作が制限されるため、ユーザ側で安全なラッパを提供する必要がある。
- `mask` / `unmask` はスレッド別のシグナルマスク操作を行い、`SignalMask` を戻す。
- `wait` は `sigwait` / `signalfd` 相当の動作。タイムアウトへ対応。

## 2. 型定義

```reml
pub enum Signal = SIGTERM | SIGINT | SIGKILL | SIGUSR1 | SIGUSR2 | SIGCHLD | SIGPIPE | SIGALRM | SIGHUP | Custom(i32)

pub type SignalHandler = fn(SignalInfo) -> SignalAction

pub type SignalInfo = {
  signal: Signal,
  source_pid: Option<ProcessId>,
  timestamp: Timestamp,
  payload: Option<SignalPayload>,
}

pub enum SignalPayload = UserData(i32) | RealTime(u64) | None

pub enum SignalAction = Continue | Terminate | Ignore

pub type SignalMask = Set<Signal>
```

- `Custom(i32)` はプラットフォーム固有の数値シグナルを表現。Windows では擬似シグナル (`CTRL_C_EVENT` など) をマッピングする。
- `SignalPayload` は POSIX real-time signal の `siginfo_t` から抽出するデータ。

## 3. 監査とセキュリティ

```reml
fn log_signal(event: Str, info: SignalInfo, audit: AuditSink) -> Result<(), Diagnostic> // effect {audit}
```

- シグナル送信 (`send` / `raise`)・受信 (`register_handler`, `wait`) の双方で監査ログを推奨。
- `SecurityPolicy` で許可されたシグナルのみ送信可能とする `SignalPolicy` の導入を検討（`CapabilitySecurity.effect_scope` と連携）。

## 4. 使用例ドラフト

### 4.1 グレースフルシャットダウン

```reml
fn install_shutdown_handler(audit: AuditSink) -> Result<(), SignalError> = {
  let handler = |info: SignalInfo| {
    audit.log("signal.shutdown", json!({ "signal": info.signal }))?
    SignalAction::Terminate
  }
  SignalCapability::register_handler(SIGTERM, handler)?;
  Ok(())
}
```

- ハンドラ内では非同期安全な操作のみを行う。必要な処理はワーカースレッドへ通知。

### 4.2 シグナル待ち

```reml
fn wait_for_child() -> Result<SignalInfo, SignalError> =
  SignalCapability::wait(set![SIGCHLD], Some(Duration::from_secs(5)))
```

- タイムアウト経過時は `SignalErrorKind::TimedOut` を返す。

## 5. 今後の拡張

- `signalfd` / `kqueue` / `IOCP` などのイベント統合。
- Windows Job Object や macOS Dispatch Sources へのラッパ。
- リアルタイムシグナルの範囲（`SIGRTMIN`〜`SIGRTMAX`）とベクタ化ハンドラ。

---

*本章はドラフトです。最終仕様ではプラットフォームごとの制約と安全なハンドラ構造、再入性対策を詳細化します。*
