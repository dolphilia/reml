# 5.3 Memory Capability プラグイン — Virtual Memory & Shared Regions

> 位置付け: 公式プラグイン（オプション）。仮想メモリや共有領域の操作は `effect {memory}` `effect {unsafe}` を多用するため、標準APIから分離し、`SecurityCapability` による導入審査を前提とする。
>
> ドラフト再整理メモ: 標準ライブラリ移行が確定していないため、本章はプラグイン維持を前提に再評価中（`docs/notes/stdlib/stdlib-expansion-research.md` 参照）。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（再検討中） |
| プラグインID | `core.memory` |
| 効果タグ | `effect {memory}`, `effect {syscall}`, `effect {unsafe}`, `effect {process}`, `effect {security}` |
| 依存モジュール | `Core.Runtime`, [5-1 System Capability プラグイン](5-1-system-plugin.md), [3-18 Core System](3-18-core-system.md), `Core.Diagnostics`, `Core.Unsafe.Ptr`, `Core.IO` |
| 相互参照 | [3.18 Core System](3-18-core-system.md), [3.8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md), [3-5 Core IO & Path](3-5-core-io-path.md), [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md) |

## 0.5 改訂案（標準ライブラリとの境界）

- **プラグイン維持**: 低レベルな仮想メモリ操作は Capability 側に残す。
- **安全ラッパの検討**: 標準ライブラリに導入する場合は、`MappedMemory` の安全操作や共有メモリの限定 API に留める。

## 1. MemoryCapability API

```reml
pub type MemoryCapability = {
  mmap: fn(MmapRequest) -> Result<MappedMemory, MemoryError>,                // effect {memory, unsafe}
  munmap: fn(MappedMemory) -> Result<(), MemoryError>,                       // effect {memory}
  mprotect: fn(&mut MappedMemory, MemoryProtection) -> Result<(), MemoryError>, // effect {memory, security}
  msync: fn(&MappedMemory, SyncFlags) -> Result<(), MemoryError>,            // effect {memory, io}
  shared_open: fn(SharedMemoryRequest) -> Result<SharedMemory, MemoryError>, // effect {memory, process}
  shared_close: fn(SharedMemory) -> Result<(), MemoryError>,                 // effect {memory}
}
```

- `mmap` / `munmap` / `mprotect` / `msync` は POSIX の仮想メモリ API に準拠し、Windows 等では同等機能をラップする。
- `shared_open` は名前付き共有メモリを作成・接続し、プロセス間通信を可能にする。
- すべての API は `AuditContext` を通じて操作対象と許可を記録することが推奨される。

## 2. 型定義

```reml
pub type MappedMemory = {
  ptr: NonNullPtr<u8>,
  len: usize,
  protection: MemoryProtection,
  flags: MmapFlags,
}

pub enum MemoryProtection = ReadOnly | ReadWrite | ReadExecute | ReadWriteExecute

pub enum MmapFlags = Private | Shared | Anonymous | Fixed | HugePage | Custom(Str)
```

- `ptr` は `Core.Unsafe.Ptr` の `NonNullPtr` を利用。`len` が 0 の場合でも不正な非NULL値を持たないことが保証される。
- `Fixed` を指定する場合は `addr` を要求。安全性の観点から `Fixed` はデフォルトで無効とし、`security` 効果の許可が必要。

### 2.1 共有メモリ

```reml
pub type SharedMemory = {
  name: Option<Str>,
  region: MappedMemory,
  owner: SharedMemoryOwner,
}

pub enum SharedMemoryOwner = Local | Remote(Core.System.Process.ProcessId)
```

- `SharedMemory` は `MappedMemory` を内含し、所有者情報で参照元プロセスを追跡する。
- `Remote` の場合、プロセス終了時に自動解放されないため監査ログの追跡が必要。

## 3. リクエスト構造

```reml
pub type MmapRequest = {
  addr: Option<VoidPtr>,
  len: usize,
  protection: MemoryProtection,
  flags: Set<MmapFlags>,
  file: Option<Path>,
  offset: Option<i64>,
}

pub type SharedMemoryRequest = {
  name: Option<Str>,
  size: usize,
  create: Bool,
  protection: MemoryProtection,
  flags: Set<MmapFlags>,
}
```

- `file` を指定した場合はファイルバックドマッピング。`offset` はページサイズ単位で揃える必要がある。
- `SharedMemoryRequest.create = true` の場合、新規作成し既存存在時は `MemoryErrorKind::AlreadyExists`。
- `flags` に `Anonymous` と `Shared` を同時指定することはできない。

## 4. MemoryError と診断

```reml
pub type MemoryError = {
  kind: MemoryErrorKind,
  message: Str,
  errno: Option<i32>,
}

pub enum MemoryErrorKind = PermissionDenied | InvalidArgument | OutOfMemory | AlignmentError | AlreadyExists | NotFound | Unsupported | PolicyViolation
```

- `PolicyViolation` は `SecurityCapability` による拒否を表し、監査ログに詳細が記録される。
- `MemoryError` は `IntoDiagnostic` を実装し、`effect {memory}` のエラーを一貫した形で報告する。

## 5. 監査テンプレート

```reml
fn log_mmap(request: MmapRequest, result: Result<MappedMemory, MemoryError>, audit: AuditSink) -> Result<(), Diagnostic> // effect {audit}
```

- `request` の `protection`・`flags`・`file` を JSON で保存。
- `result` が成功した場合は `ptr` と `len` を記録。失敗した場合は `MemoryError` 詳細を記録。
- `AuditContext`（3.6）と組み合わせることで [5-1 System Capability プラグイン](5-1-system-plugin.md) の `SyscallCapability.audited_syscall` をラップできる。

## 6. 高レベルユーティリティ

```reml
fn map_file(path: Path, protection: MemoryProtection) -> Result<MappedMemory, MemoryError> // effect {memory, io, unsafe}
fn map_shared_buffer(name: Str, size: usize) -> Result<SharedMemory, MemoryError>          // effect {memory, process, unsafe}
fn unmap(memory: MappedMemory) -> Result<(), MemoryError>                                  // effect {memory}
fn remap(memory: &mut MappedMemory, new_len: usize) -> Result<(), MemoryError>             // effect {memory, unsafe}
```

- `map_file` は `Core.IO` を用いてファイルを開き、`MmapRequest` を構築する。`effect {io}` を追加。
- `remap` は `mremap` 互換。プラットフォームによって未サポートの場合 `MemoryErrorKind::Unsupported`。

## 7. セキュリティとポリシー

- `CapabilitySecurity.effect_scope` に `{memory, syscall, audit}` を含め、危険な `Fixed` マッピングや `ReadWriteExecute` を制限する。
- `SecurityPolicy` では `max_memory_usage`, `allowed_syscalls` を連携させ、メモリマッピングの許可を制御する。
- 共有メモリ名称は `SecurityPolicy` の `allowed_shared_memory`（将来追加予定）と照合する。

## 8. 将来の拡張

- ユーザ空間ページフォールトハンドラ（`userfaultfd`）や NUMA バインディングとの統合。
- `madvise` / `mlock` / `munlock` の追加。
- WebAssembly (`WASI`) でのメモリガードサポート。

---

*本章はドラフトであり、公式プラグインとしての配布・審査プロセスは `Chapter 4` のエコシステム仕様と連携して今後更新される。*
