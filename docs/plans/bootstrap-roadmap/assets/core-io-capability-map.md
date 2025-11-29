# Core IO Capability マップ

## 目的
- `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` §1.3 のタスクに従い、OS 依存機能（permissions/symlink/watcher 等）の Capability ID・Stage 要件・検証ポイントを整理する。
- 仕様 [3-5 Core IO & Path](../../spec/3-5-core-io-path.md) と [3-8 Core Runtime & Capability](../../spec/3-8-core-runtime-capability.md) §8-§10 を横断し、Runtime Capability Registry・Runtime Bridge・監査計画との整合を確認する。
- Phase3 `core-io-path` ジョブで `verify_capability_stage` の Runbook を共有し、`docs/notes/runtime-capability-stage-log.md`・`docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` へフィードバックできる状態を作る。

## Capability 対応表
| Capability ID | API/効果タグ | Stage 要件 | OS / Adapter 依存 | Rust 実装フック | Diagnostics / Audit 検証 | 状態 |
| --- | --- | --- | --- | --- | --- | --- |
| `io.fs.read` / `io.fs.write` | `Reader::read`, `Writer::write`, `File::open`, `File::create`, `effect {io}`, `{io.blocking}` | `StageRequirement::AtLeast(StageId::Beta)` | `runtime/native/src/os.c` (`reml_os_file_*`) を Rust `FsAdapter`（未作成）でラップ。Windows: UTF-16 → UTF-8 変換, Linux/macOS: POSIX FD | `compiler/rust/runtime/src/io/{reader.rs,writer.rs,adapters.rs}` が `FsAdapter::ensure_{read,write}` 経由で `CapabilityRegistry::verify_capability_stage("io.fs.*", ..)` を呼び出す | `core.io.*` 診断 (`docs/spec/3-6-core-diagnostics-audit.md`)、`collect-iterator-audit-metrics.py --section core_io --scenario effects_matrix` | In Progress: Reader/Writer が `FsAdapter` で Stage を検証済み。ファイル API/Adapter 実装は未 |
| `fs.permissions.read` | `File::metadata`, `Path::metadata`, `effect {security}`（policy 照合） | `StageRequirement::Exact(StageId::Stable)` | POSIX: `stat`, `lstat`, `access`; Windows: `GetFileInformationByHandleEx`, ACL 抽象。`runtime/native` に API が無いため追加要 | `compiler/rust/runtime/src/io/file.rs`（未作成）、`CapabilityRegistry` + `SecurityCapability` | `core.io.metadata.*` 診断, `metadata.security.policy`, `effect.stage.required = "stable"` | Missing |
| `fs.permissions.modify` | `FileOptions::permissions`, `chmod/chown`, `validate_path` | `StageRequirement::Exact(StageId::Stable)` + `SecurityCapability.policy` | POSIX: `chmod`, `fchmodat`; Windows: `SetFileSecurityW`. `IoContext` でモードを監査。 | `compiler/rust/runtime/src/io/options.rs`（未作成）、`runtime/native/src/os.c` に setter を追加予定 | `core.io.file.permission_denied`, `effects.contract.stage_mismatch`（policy 不足時） | Missing |
| `fs.symlink.query` | `is_safe_symlink`, `normalize`, `Path::read_link`, `effect {io.blocking, security}` | `StageRequirement::AtLeast(StageId::Beta)`、`SecurityCapability.audit_required = true` | POSIX: `lstat`, `readlink`; Windows: `GetFinalPathNameByHandleW`, 再解析ポイント検査 | `compiler/rust/runtime/src/path/security.rs`（未作成）、`runtime/native/include/reml_os.h` に API 追加要 | `core.path.security.symlink` 診断、`metadata.security.reason = "symlink_traversal"` | Missing |
| `fs.symlink.modify` | `create_symlink`, `sandbox_path`, `effect {security}` | `StageRequirement::Exact(StageId::Stable)` | POSIX: `symlink`, `linkat`; Windows: `CreateSymbolicLinkW`（Developer Mode 権限） | 同上 (`FsAdapter`)。`RuntimeBridge` から `SecurityCapability` を参照 | `core.path.security.create_symlink` 診断、`AuditEnvelope.metadata["fs.symlink.stage"]` | Missing |
| `fs.watcher.native` | `watch`, `watch_with_limits`, `effect {io.async}` | `StageRequirement::AtLeast(StageId::Beta)`、`AsyncCapability` 連携必須 | Linux: inotify、macOS: FSEvents、Windows: ReadDirectoryChangesW。`notify` crate 導入予定 | `compiler/rust/runtime/src/io/watcher.rs`（未作成）、`CapabilityRegistry::verify_capability_stage("io.fs.watch", ..)` | `core.io.watcher.*` 診断、`AuditEnvelope.metadata["io.watch.queue_size"]` | Missing |
| `fs.watcher.recursive` | `watch_with_limits` の `max_depth`, `exclude_patterns` | `StageRequirement::Exact(StageId::Stable)`（macOS のみ Beta） | 上記 API + OS ごとのリソース制限 (`inotify` watches, Windows バッファ) | `io/watcher.rs` 内で `WatcherAdapter` を抽象化し、`RuntimeBridge` で Stage 差異を記録 | `collect-iterator-audit-metrics.py --section core_io --scenario watcher_audit` | Missing |
| `security.fs.policy` | `validate_path`, `sandbox_path`, `IoErrorKind::SecurityViolation`, `effect {security}` | `StageRequirement::Exact(StageId::Stable)` | `SecurityCapability`（`compiler/rust/runtime/src/registry.rs`）＋ `runtime/native` のパス制約 | `Core.Diagnostics` ↔ `Core.IO`。`IoContext` に `security.policy_digest` を追加予定 | `core.path.security.*` 診断、`AuditEnvelope.metadata["security.policy.digest"]` | Missing |

### 備考
- Capability ID は `docs/spec/3-5` の API と `docs/spec/3-8` Stage 契約をもとに命名し、`CapabilityRegistry::verify_capability_stage` に渡す正式 ID として使用する。`io.fs` 既存呼称（`core-io-effects-matrix.md`）と齟齬がない。
- Stage 要件: `fs.permissions.*` / `security.fs.policy` はセキュリティポリシーに直結するため `Exact(StageId::Stable)` を要求し、`fs.symlink.*` / `fs.watcher.*` は実装難度を考慮して `AtLeast(StageId::Beta)` を最低ラインとする。
- 実装状況: Reader/Writer は `FsAdapter` により `io.fs.*` Stage を検証済み。`File`/`Path`/`Watcher`/`Security` 連携は未作成のため、余剰 Capability（permissions/symlink/watch/security）と Diagnostics/Audit の結線は引き続き Backlog（`core-io-path-plan` Blocking 行）として残る。

## Adapter / Runtime Bridge 設計メモ
- **Rust Runtime (`compiler/rust/runtime/src/io`)**  
  - `adapters.rs` に `FsAdapter` / `WatcherAdapter` を追加し、`reader.rs` / `writer.rs` から `FsAdapter::ensure_{read,write}` を実行して `verify_capability_stage("io.fs.*")` を呼び出す。  
  - `effects.rs` には `mark_io_blocking`/`mark_io_async`/`mark_security` が実装済みで、`take_io_effects_snapshot()` を通じて `EffectLabels` を取得できる。Capability Registry と効果ラベルの結線はまだ部分的。  
  - `registry.rs` は `verify_capability_stage` を `StageId::Stable` 固定で返す仮実装のため、Capability ID を受け取っても OS 差異を反映できない。IO 実装では呼び出し箇所自体が存在しないため、`File::open` / `watch` 着手時に `io.fs.*` / `fs.watcher.*` を必須化する。

- **Runtime Native (`runtime/native/src/*.c`)**  
  - `os.c` は `reml_os_file_open_*` / `reml_os_file_read/write` 等の最小 API を提供するが、permissions・symlink・watcher・metadata の拡張が欠如している。  
  - Windows では UTF-8 → UTF-16 変換ヘルパを保持しており、`FsAdapter` で `PathBuf ↔ wchar_t` 変換を集約する。POSIX では `stat`/`lstat` の薄いラッパを追加して `fs.permissions.*` Capability と紐付ける。  
  - Watcher API（inotify/FSEvents/ReadDirectoryChangesW）は実装されておらず、`RuntimeBridge` 経由で外部監視デーモンを利用する案が `docs/guides/runtime-bridges.md` に記載されている。`core-io-capability-map` は Stage 判定で `RuntimeBridgeId = "native.fs.watch"` を参照する前提で記述している。

- **Runtime Bridge**  
  - Rust 実装側に `compiler/rust/runtime/src/runtime_bridge/` ディレクトリはまだ存在せず、`RuntimeBridgeRegistry` のロジックは OCaml 実装のみ（`compiler/ocaml/src/runtime_bridge_registry.ml`）にある。  
  - IO/Path Capability を Runtime Bridge 経由で提供する場合は、`bridge.fs.adapter`（`RuntimeBridgeDescriptor` の仮 ID）から `CapabilityRegistry.register("io.fs.watch", CapabilityHandle::Io(..))` へ再公開する。  
  - `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` で定義する P5.1 以降の統合ステップに従い、Bridge 説明文 (`describe_bridge`) へ Capability ID 列挙を追加する。

## CI / Runbook 提案
- `python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario capability_matrix --matrix docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md --output reports/spec-audit/ch3/core_io_capabilities.json --require-success` を Phase3 `core-io-path` ジョブへ追加し、Capability ID / Stage 要件 / effect ラベルを突合。
- `reml_frontend --runtime-capabilities io.fs.read,io.fs.write,fs.permissions.read` のように CLI 実行時の `--runtime-capabilities` フラグに本マップの ID を渡し、`verify_capability_stage` で Stage 診断（`effects.contract.stage_mismatch`）を即時検出する。
- Watcher 実装後は `RuntimeBridge` を介した Stage レポート (`RuntimeBridgeAuditSpec` §10) を `AuditEnvelope.metadata["io.watch.capability"]` に転写し、`docs/notes/runtime-capability-stage-log.md` に KPI を追記する。
