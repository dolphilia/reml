# System Programming Primer for Reml (Draft)

> 目的：Reml でシステムプログラミング機能を活用する際の導入ガイドを提供し、Chapter 5 公式プラグイン（System〜RealTime）で定義した API を横断的に理解できるようにする。

## 1. 対象読者
- Reml で OS 連携・FFI を行う DSL 開発者
- Capability Registry をカスタマイズするプラットフォームエンジニア
- 監査・セキュリティポリシーの設計者

## 2. 概要と依存関係
- System Capability プラグイン (5-1): システムコールとプラットフォームラッパ
- Process Capability プラグイン (5-2): プロセス/スレッド制御
- Memory Capability プラグイン (5-3): メモリマップ・共有メモリ
- Signal Capability プラグイン (5-4): シグナル登録・待機
- Hardware Capability プラグイン (5-5): CPU/NUMA 情報
- RealTime Capability プラグイン (5-6): リアルタイムスケジューラ・精密タイマー
- その他: Core.Diagnostics (監査), Core.Unsafe.Ptr, guides/reml-ffi-handbook, guides/core-unsafe-ptr-api-draft

## 3. Capability Registry のセットアップ
1. `CapabilityRegistry::register("system", ...)` など各 Capability を登録。
2. `CapabilitySecurity.effect_scope` に `{syscall, process, memory, audit}` 等を追加。
3. `SecurityPolicy` とシンク (`AuditSink`) を設定し、監査ログの保管場所を定義。
4. デバッグ時にはモック Capability (`MockSyscalls`, `MockProcess`) を利用して CI でシナリオテストを実行する。

## 4. 典型的な操作フロー

### 4.1 システムコールを安全に呼び出す
1. `SyscallCapability` を取得し、`supports` で利用可否を確認。
2. `SyscallDescriptor` を構築し `audited_syscall` を通して呼び出し。
3. 監査ログと `SecurityPolicy` を併用し、許可リスト外のシステムコールを拒否する。

### 4.2 プロセス生成と監査
1. `Command`/`Environment` を構築し `ProcessCapability::spawn_process` を使用。
2. `AuditContext` で `process.spawned` を記録し、終了時に `ExitStatus` をログ化。
3. `wait_process` / `wait_with_options` でタイムアウトや出力収集を行う。

### 4.3 メモリマップド I/O
1. `Core.IO` でファイルを開き、`MmapRequest` を構築。
2. `MemoryCapability::mmap` に渡し、得られた `MappedMemory` から `Span<u8>` を生成。
3. 操作後は `munmap`、共有メモリの場合は `shared_close` を忘れずに。

### 4.4 シグナルとリアルタイム処理
1. `SignalCapability::register_handler` でコールバックを登録（`unsafe` ブロックに閉じ込める）。
2. `RealTimeCapability::set_scheduler` や `create_timer` を用いてリアルタイム要求を満たす。
3. 監査ログ (`log_signal`, `log_mmap`, `log_io`) を通じて可観測性を確保する。

## 5. ガイドラインとベストプラクティス
- `@no_blocking` / `@async_free` 等の属性を利用して非同期セクションを保護。
- `AuditContext` を共通テンプレートとして使用し、`syscall`, `process`, `ffi` 等のログ構造を統一。
- `CapabilitySecurity` の `policy` 参照を活用して `SecurityPolicy` と整合させる。
- CROSS: `../runtimeportability.md` のチェックリストを用いてターゲット差異を洗い出す（TODO: 次版で詳細を追加）。

## 6. 今後の拡張予定
- 各章の API サンプルを統合したクックブック（プロセス監視ツール、共有メモリ IPC など）を追加予定。
- `docs/guides/runtime-audit.md` を新規作成し、監査イベントの設計テンプレートを整理。
- README から本ガイドへのリンクとモジュール目次を追加する。

---

*ドラフト段階のため、今後 API 変更に合わせて内容を更新します。*
