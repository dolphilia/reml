# 3.11 Core System — Syscall Interface & Platform Bindings

> 目的：Reml ランタイムが提供するシステムコールアクセス層 (`Core.System`) の責務を定義し、`CapabilityRegistry` から取得される `SyscallCapability` を通じた安全な OS 連携を仕様化する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト |
| 効果タグ | `effect {syscall}`, `effect {unsafe}`, `effect {audit}`, `effect {security}`, `effect {memory}` |
| 依存モジュール | `Core.Runtime`, `Core.IO`, `Core.Diagnostics`, `Core.Memory` (予定 3-13), Guides: `system-programming-primer` (予定) |
| 相互参照 | [3.8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md), [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [3-5 Core IO & Path](3-5-core-io-path.md) |

## 1. SyscallCapability API

```reml
pub type SyscallCapability = {
  raw_syscall: fn(SyscallNumber, [i64; 6]) -> Result<i64, SyscallError>,                // effect {syscall, unsafe}
  platform_syscalls: PlatformSyscalls,                                                  // effect {syscall}
  audited_syscall: fn(SyscallDescriptor, SyscallThunk) -> Result<SyscallRet, SyscallError>, // effect {syscall, audit}
  supports: fn(SyscallId) -> Bool,
}

pub type SyscallNumber = i64
pub type SyscallRet = i64
pub type SyscallThunk = fn() -> Result<SyscallRet, SyscallError>
```

- `raw_syscall` は OS 固有の番号と最大 6 引数を用いる最下層 API。呼び出しは `unsafe` かつ `effect {syscall}` を伴う。
- `platform_syscalls` は型安全なラッパ群を提供し、プラットフォームごとの命名・引数取り扱いを抽象化する。
- `audited_syscall` は [3.6](3-6-core-diagnostics-audit.md) で定義した `AuditContext` と統合し、必ず監査ログを発行する。
- `supports` は特定システムコールの有効性を判定し、`@cfg` 条件分岐の静的検査支援に利用する。

## 2. プラットフォーム別ラッパ構造

```reml
pub type PlatformSyscalls = {
  linux: Option<LinuxSyscalls>,
  windows: Option<WindowsSyscalls>,
  macos: Option<MacOSSyscalls>,
  wasi: Option<WasiSyscalls>,
}

pub struct LinuxSyscalls {
  pub sys_read: fn(fd: i32, buf: MutPtr<u8>, len: usize) -> Result<usize, SyscallError>,
  pub sys_write: fn(fd: i32, buf: Ptr<u8>, len: usize) -> Result<usize, SyscallError>,
  pub sys_openat: fn(dirfd: i32, path: Ptr<u8>, flags: i32, mode: i32) -> Result<i32, SyscallError>,
  pub sys_close: fn(fd: i32) -> Result<(), SyscallError>,
  // 追加候補: epoll, mmap, clone 等
}
```

- `PlatformSyscalls` は OS ごとに Option で提供し、Capability 登録時に該当プラットフォームのみ有効化する。
- ラッパ関数は `Ptr<T>`, `MutPtr<T>` を用い `Core.Unsafe.Ptr` と整合する。
- 将来的に `LinuxSyscalls::sys_mmap` は `Core.Memory` から再利用される。

## 3. SyscallDescriptor と監査連携

```reml
pub type SyscallDescriptor = {
  name: Str,
  number: SyscallNumber,
  effect_set: Set<EffectTag>,   // 例: {syscall, memory}
  audit_metadata: Map<Str, Json>,
}
```

- `effect_set` により `CapabilitySecurity.effect_scope` との整合性を検証する。
- `audit_metadata` には `fd`, `path`, `policy_digest` などを埋め込み、`AuditContext::log` のデフォルト引数として利用する。

### 3.1 `audited_syscall` の実装指針

```reml
fn audited_syscall<T>(desc: SyscallDescriptor, thunk: SyscallThunk, sink: AuditSink) -> Result<T, SyscallError> =
  let ctx = AuditContext::new("syscall", desc.name)?.with_metadata(desc.audit_metadata.clone());
  let start = now()?;
  let result = thunk()?;
  let duration = now()? - start;
  ctx.log("syscall.completed", json!({ "duration_ns": duration.as_nanos(), "result": result }))?;
  Ok(result);
```

- 監査ログは成功・失敗を問わず出力し、`SyscallError` 発生時もエラー情報を記録する。
- `sink` は Capability 登録時に構成された `AuditSink` を利用する。

## 4. システムエラーと変換

```reml
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

## 6. 既定実装の構成要素

- **Capability 登録**: ランタイム初期化時に `CapabilityRegistry::register("system", CapabilityHandle::System(...))` を行い、プラットフォーム別実装を注入する。
- **フォールバック**: 未サポート OS では `PlatformSyscalls` をすべて `None` にし、`raw_syscall` のみ提供。コンパイラは `supports` を通じて診断を提示する。
- **テスト戦略**: 擬似システムコールレイヤ (`MockSyscalls`) を提供し、`effect {audit}` を含むユニットテストで監査ログを検証する。

## 7. 今後の拡張

- `SyscallCapability` に `subscribe_signals` や `register_eventfd` 等を追加し、`Core.Signal` との連携を整理。
- `PlatformSyscalls` をテンプレート生成（`@cfg`）によりカスタマイズ可能にする。
- `SyscallPolicy` を `SecurityPolicy` の一部としてシリアライズし、CLI から構成可能にする。

---

*この章はドラフトであり、`Core.System` の詳細 API 定義（`LinuxSyscalls` のメソッド一覧など）は今後拡張する。最終仕様では各 OS ごとの最小サポートセットと効果タグが明記される予定。*
