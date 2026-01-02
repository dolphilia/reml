# 5.1 System Capability プラグイン — Syscall Interface & Platform Bindings

> 位置付け: 公式プラグイン（オプション）。標準API（Chapter 3）には同梱せず、必要なプロジェクトが `CapabilityRegistry` へ明示的に登録することで利用する。`0-1-project-purpose.md` が定める安全性・段階的習得の原則を守るため、プラットフォーム依存かつ `unsafe` 効果を伴う API 群を別章に分離した。
>
> ドラフト再整理メモ: 標準ライブラリ拡張の方針に合わせて、本章は低レベル Capability と標準ライブラリの境界を再検討中（`docs/notes/stdlib/stdlib-expansion-research.md` / `docs/plans/bootstrap-roadmap/4-1-stdlib-improvement-implementation-plan.md` 参照）。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（再検討中） |
| プラグインID | `core.system` |
| 効果タグ | `effect {syscall}`, `effect {unsafe}`, `effect {audit}`, `effect {security}`, `effect {memory}` |
| 依存モジュール | `Core.Runtime`, `Core.IO`, `Core.Diagnostics`, `Core.Memory`（[5-3](5-3-memory-plugin.md)） |
| 相互参照 | [3.18 Core System](3-18-core-system.md), [3.8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md), [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [3-5 Core IO & Path](3-5-core-io-path.md), [5-4 Signal Capability プラグイン](5-4-signal-plugin.md) |

## 0.5 改訂案（標準ライブラリとの境界整理）

- **低レベル syscall の維持**: `raw_syscall` は Capability 側に残し、標準ライブラリは安全なラッパ API のみを公開する。
- **`Core.System` への橋渡し**: [3-18](3-18-core-system.md) の `Core.System.Process` / `Core.System.Env` / `Core.System.Signal` から本 Capability を間接利用する構成を想定する。
- **公開 API の制限**: 標準ライブラリから直接 `SyscallCapability` を露出しない方針を明文化する。

## 1. SyscallCapability API

```reml
pub type SyscallId = Str
pub type SyscallError
pub type SyscallDescriptor
pub type PlatformSyscalls

pub type i64
pub type SyscallNumber = i64
pub type SyscallRet = i64
pub type SyscallArgs = [i64; 6]
pub type SyscallThunk = fn() -> Result<SyscallRet, SyscallError>

pub type SyscallCapability = {
  raw_syscall: fn(SyscallNumber, SyscallArgs) -> Result<SyscallRet, SyscallError>,          // effect {syscall, unsafe}
  platform_syscalls: PlatformSyscalls,                                                     // effect {syscall}
  audited_syscall: fn(SyscallDescriptor, SyscallThunk) -> Result<SyscallRet, SyscallError>, // effect {syscall, audit}
  supports: fn(SyscallId) -> Bool,
}
```

- `raw_syscall` は OS 固有の番号と最大 6 引数を用いる最下層 API。呼び出しは `unsafe` かつ `effect {syscall}` を伴う。
- `platform_syscalls` は型安全なラッパ群を提供し、プラットフォームごとの命名・引数取り扱いを抽象化する。
- `audited_syscall` は [3.6](3-6-core-diagnostics-audit.md) の `AuditContext` と統合し、必ず監査ログを発行する。
- `supports` は特定システムコールの有効性を判定し、`@cfg` 条件分岐の静的検査支援に利用する。

## 2. プラットフォーム別ラッパ構造

```reml
pub type Ptr<T>
pub type MutPtr<T>
pub type SyscallError
pub type i32
pub type u8
pub type usize
pub type Option<T>
pub type WindowsSyscalls
pub type MacOSSyscalls
pub type WasiSyscalls

pub struct LinuxSyscalls {
  sys_read: fn(fd: i32, buf: MutPtr<u8>, len: usize) -> Result<usize, SyscallError>,
  sys_write: fn(fd: i32, buf: Ptr<u8>, len: usize) -> Result<usize, SyscallError>,
  sys_openat: fn(dirfd: i32, path: Ptr<u8>, flags: i32, mode: i32) -> Result<i32, SyscallError>,
  sys_close: fn(fd: i32) -> Result<(), SyscallError>,
  // 追加候補: epoll, mmap, clone 等
}

pub type PlatformSyscalls = {
  linux: Option<LinuxSyscalls>,
  windows: Option<WindowsSyscalls>,
  macos: Option<MacOSSyscalls>,
  wasi: Option<WasiSyscalls>,
}
```

- `PlatformSyscalls` は OS ごとに Option で提供し、Capability 登録時に該当プラットフォームのみ有効化する。
- ラッパ関数は `Ptr<T>`, `MutPtr<T>` を用い `Core.Unsafe.Ptr` と整合する。
- 将来的に `LinuxSyscalls::sys_mmap` は [5-3 Memory Capability プラグイン](5-3-memory-plugin.md) から再利用される。

## 3. SyscallDescriptor と監査連携

```reml
pub type EffectTag = Str
pub type Json
pub type Set<T> = List<T>
pub type Map<K, V> = List<(K, V)>
pub type i64
pub type SyscallNumber = i64

pub type SyscallDescriptor = {
  name: Str,
  number: SyscallNumber,
  effect_set: Set<EffectTag>,   // 例: {syscall, memory}
  audit_metadata: Map<Str, Json>,
}
```

- `effect_set` により `CapabilitySecurity.effect_scope` との整合性を検証する。
- `audit_metadata` には `fd`, `path`, `policy_digest` などを埋め込み、`AuditContext::log` の既定値として利用する。

### 3.1 `audited_syscall` の実装指針

```reml
pub type SyscallError
pub type i64
pub type SyscallRet = i64
pub type SyscallThunk = fn() -> Result<SyscallRet, SyscallError>
pub type AuditSink
pub type Json
pub type Map<K, V> = List<(K, V)>

pub type SyscallDescriptor = {
  name: Str,
  audit_metadata: Map<Str, Json>,
}

pub struct AuditContext {
  domain: Str,
  subject: Str,
  sink: AuditSink,
  metadata: Map<Str, Json>,
}

impl AuditContext {
  fn new(domain: Str, subject: Str, sink: AuditSink) -> Result<AuditContext, SyscallError> = todo
  fn with_metadata(self, metadata: Map<Str, Json>) -> Self = todo
  fn log(self, event: Str, payload: Json) -> Result<(), SyscallError> = todo
}

fn audited_syscall(
  desc: SyscallDescriptor,
  thunk: SyscallThunk,
  sink: AuditSink,
) -> Result<SyscallRet, SyscallError> = todo
```

- 監査ログは成功・失敗を問わず出力し、`SyscallError` 発生時もエラー情報を記録する。
- `sink` は Capability 登録時に構成された `AuditSink` を利用する。

## 4. システムエラーと変換

```reml
pub type i64
pub type i32
pub type SyscallNumber = i64

pub type SyscallError = {
  kind: SyscallErrorKind,
  message: Str,
  number: SyscallNumber,
  errno: Option<i32>,
}

pub enum SyscallErrorKind = AccessDenied | InvalidArgument | Interrupted | WouldBlock | NotSupported | Fault | Unknown
```

- `errno` は POSIX 系 OS での `errno` 値を格納し、Windows では `GetLastError()` の結果を `message` に含める。
- `SyscallError` は `IntoDiagnostic` を実装し、`effect {audit}` を伴う API でのエラー報告に利用する。

## 5. セキュリティポリシーとの統合

```reml
pub type SyscallId = Str
pub type SyscallRateLimit
pub type SecurityError
pub type SyscallDescriptor
pub type Set<T> = List<T>

pub type SyscallPolicy = {
  allowed: Set<SyscallId>,
  denied: Set<SyscallId>,
  rate_limit: Option<SyscallRateLimit>,
  require_audit: Bool,
}

fn enforce_syscall_policy(desc: SyscallDescriptor, policy: SyscallPolicy) -> Result<(), SecurityError> // `effect {security}`
```

- `SyscallPolicy` は `SecurityCapability.enforce_security_policy` から提供され、`audited_syscall` 呼び出し前に検証される。
- `rate_limit` は高頻度のシステムコールに対するスロットリングを表現する。

## 6. 公式プラグインとしての運用指針

- **登録前審査**: `CapabilityRegistry::register("system", CapabilityHandle::System(...))` を実行する前に、ターゲットOS・権限要件・監査設定を `SecurityCapability` と突き合わせる。
- **フォールバック**: 未サポート OS では `PlatformSyscalls` をすべて `None` にし、`raw_syscall` のみ提供する。コンパイラは `supports` を通じて診断を提示する。
- **テスト戦略**: 擬似システムコールレイヤ (`MockSyscalls`) を提供し、`effect {audit}` を含むユニットテストで監査ログを検証する。

## 7. 今後の拡張

- `SyscallCapability` に `subscribe_signals` や `register_eventfd` 等を追加し、[5-4 Signal Capability プラグイン](5-4-signal-plugin.md) との連携を整理。
- `PlatformSyscalls` をテンプレート生成（`@cfg`）によりカスタマイズ可能にする。
- `SyscallPolicy` を `SecurityPolicy` の一部としてシリアライズし、CLI から構成可能にする。

---

*この章はドラフトであり、公式プラグインとしての配布・審査プロセスは今後 `Chapter 4` のエコシステム仕様と連携して更新される。*
