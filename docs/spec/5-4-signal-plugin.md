# 5.4 Signal Capability プラグイン — Inter-Process Signals & Handlers

> 位置付け: 公式プラグイン（オプション）。OS シグナルは `unsafe` 操作とプロセス制御を伴うため、標準 API は [3-18 Core System](3-18-core-system.md) へ移行し、本章は低レベルのハンドラ登録と Capability 審査を扱う。
>
> ドラフト再整理メモ: `Core.System.Signal` への移行が確定したため、本章は低レベル Capability の残留範囲を整理する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（低レベル維持） |
| プラグインID | `core.signal` |
| 効果タグ | `effect {signal}`, `effect {process}`, `effect {unsafe}`, `effect {audit}`, `effect {security}`, `effect {io.blocking}` |
| 依存モジュール | `Core.Runtime`, `Core.System`, [5-1 System Capability プラグイン](5-1-system-plugin.md), [5-2 Process Capability プラグイン](5-2-process-plugin.md), `Core.Diagnostics`, `Core.Unsafe.Ptr` |
| 相互参照 | [3.18 Core System](3-18-core-system.md), [3.8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md), [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md) |

## 0.5 標準ライブラリ移行の確定

- **`Core.System.Signal` への移行**: シグナル型・安全な送受信 API は [3-18](3-18-core-system.md) に移行済み。
- **Capability の役割**: 低レベルのハンドラ登録や OS 依存機能を Capability 側に留め、標準 API は安全な操作のみを公開する。

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

## 2. 型定義（低レベル）

```reml
pub type Signal = Core.Runtime.Signal
pub type SignalInfo = Core.Runtime.SignalInfo
pub type SignalHandler = fn(Signal) -> ()

pub type SignalMask = Set<Signal>
```

- 高レベルの列挙型や `SignalDetail` は [3-18 Core System](3-18-core-system.md) で定義する。

## 3. 監査とセキュリティ

```reml
fn log_signal(event: Str, info: SignalInfo, audit: AuditSink) -> Result<(), Diagnostic> // effect {audit}
```

- シグナル送信 (`send` / `raise`)・受信 (`register_handler`, `wait`) の双方で監査ログを推奨。
- `SecurityPolicy` で許可されたシグナルのみ送信可能とする `SignalPolicy` の導入を検討（`CapabilitySecurity.effect_scope` と連携）。

## 4. 使用例ドラフト

### 4.1 グレースフルシャットダウン

```reml
fn install_shutdown_handler(audit: AuditSink) -> Result<(), SignalError> {
  let shutdown_handler = |signal: Signal| {
    let _ = audit.log(
      "signal.shutdown",
      Map::empty().insert("signal", signal),
    );
  };
  SignalCapability::register_handler(SIGTERM, shutdown_handler)?;
  Ok(())
}
```

- ハンドラ内では非同期安全な操作のみを行い、必要な処理はワーカースレッドへ通知する。

### 4.2 シグナル待ち

```reml
fn wait_for_child() -> Result<SignalInfo, SignalError> {
  SignalCapability::wait([SIGCHLD], Some(Duration::from_secs(5)))
}
```

- タイムアウト経過時は `SignalErrorKind::TimedOut` を返す。

## 5. 今後の拡張

- `signalfd` / `kqueue` / `IOCP` などのイベント統合。
- Windows Job Object や macOS Dispatch Sources へのラッパ。
- リアルタイムシグナルの範囲（`SIGRTMIN`〜`SIGRTMAX`）とベクタ化ハンドラ。

---

*本章はドラフトであり、公式プラグインとしての配布・審査プロセスは `Chapter 4` のエコシステム仕様と連携して更新される。*
