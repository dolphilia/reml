# 調査メモ: 第14章 Capability と監査

## 対象モジュール

- `compiler/runtime/src/capability/mod.rs`
- `compiler/runtime/src/capability/descriptor.rs`
- `compiler/runtime/src/capability/registry.rs`
- `compiler/runtime/src/capability/handle.rs`
- `compiler/runtime/src/capability/contract.rs`
- `compiler/runtime/src/capability/audit.rs`
- `compiler/runtime/src/capability/security.rs`
- `compiler/runtime/src/audit/mod.rs`
- `compiler/runtime/src/stage.rs`

## 入口と全体像

- Capability は `CapabilityRegistry` のシングルトンで登録・検証される。内部では `CapabilityEntries` と `AuditEvent` の履歴を保持し、起動時に `builtin_capabilities()` で標準 Capability を登録する。
  - `compiler/runtime/src/capability/registry.rs:40-89`
- 監査ログは `AuditEventKind` と `AuditEnvelope` が中核で、イベントごとの必須キーを `AuditEnvelope::validate` で検証する。仕様は `docs/spec/3-6-core-diagnostics-audit.md` に対応。
  - `compiler/runtime/src/audit/mod.rs:1-253`

## データ構造

- **CapabilityDescriptor**: `id` / `stage` / `effect_scope` と、`CapabilityMetadata` を保持する公開メタデータ。
  - `compiler/runtime/src/capability/descriptor.rs:14-123`
- **CapabilityMetadata**: `provider` / `manifest_path` / `last_verified_at` / `security` を持つ。`security` に `audit_required` / `isolation_level` / `permissions` などを保持する。
  - `compiler/runtime/src/capability/descriptor.rs:105-182`
- **CapabilityProvider**: `Core` / `Plugin` / `ExternalBridge` / `RuntimeComponent` を区別。
  - `compiler/runtime/src/capability/descriptor.rs:67-83`
- **CapabilityHandle**: Capability ごとの型付きハンドルを列挙し、`descriptor()` で共通メタデータへアクセスできる。
  - `compiler/runtime/src/capability/handle.rs:15-190`
- **ConductorCapabilityContract**: DSL/Conductor が宣言する Capability 要件の集合と、マニフェストパスを保持する。
  - `compiler/runtime/src/capability/contract.rs:24-68`
- **StageId / StageRequirement**: Stage の比較とパースを行う簡易モデル。`StageId::Alpha` が実装に存在する点は仕様との整合確認が必要。
  - `compiler/runtime/src/stage.rs:7-105`
- **AuditEnvelope / AuditEvent**: 監査メタデータのコンテナとタイムスタンプ付きイベント。
  - `compiler/runtime/src/audit/mod.rs:167-301`
- **AuditCapabilityMetadata**: 監査ログの伝送方式やスキーマバージョンを保持。
  - `compiler/runtime/src/capability/audit.rs:30-55`
- **SecurityCapabilityMetadata**: セキュリティポリシーの概況とパスサンドボックスなどの制約を保持。
  - `compiler/runtime/src/capability/security.rs:30-55`

## コアロジック

- **登録/取得**: `register` と `unregister` で `CapabilityEntries` を更新し、重複は `CapabilityError::AlreadyRegistered` を返す。
  - `compiler/runtime/src/capability/registry.rs:91-115`
- **プラグイン Capability 登録**: `register_plugin_capability` が Provider 情報を付与して登録する。
  - `compiler/runtime/src/capability/registry.rs:117-132`
- **Stage/Effect 検証**: `verify_capability` が Stage requirement と effect scope を検査し、成功時に `last_verified_at` を更新する。
  - `compiler/runtime/src/capability/registry.rs:292-366`
- **マニフェスト整合**: `ensure_manifest_alignment` が manifest との stage/effects/source_span の一致を確認する。
  - `compiler/runtime/src/capability/registry.rs:134-247`
- **監査イベントの記録**: `record_capability_check` が `AuditEnvelope.metadata` に `effect.*` や `capability.*` を詰めて `AuditEvent` を生成し履歴へ追加する。
  - `compiler/runtime/src/capability/registry.rs:419-507`
- **監査メタデータ検証**: `AuditEnvelope::validate` が `event.kind` と付随する必須キーを検証し、不足時はエラーを返す。
  - `compiler/runtime/src/audit/mod.rs:215-252`

## エラー処理

- **CapabilityError**: Stage 不一致や effect scope 不一致、マニフェスト不一致を具体的なコードで表現する。
  - `compiler/runtime/src/capability/registry.rs:740-869`
- **AuditEvent/Envelope 検証**: `AuditEvent::validate` が timestamp 欠落や Envelope 検証失敗を `anyhow` で返す。
  - `compiler/runtime/src/audit/mod.rs:294-301`

## 仕様との対応メモ

- Capability Registry と Descriptor/Provider/Security は `docs/spec/3-8-core-runtime-capability.md` と対応。
- AuditEnvelope / AuditEvent の種別と必須キーは `docs/spec/3-6-core-diagnostics-audit.md` に対応。
- `StageId::Alpha` が実装に存在するため、仕様（Experimental/Beta/Stable）の差分整理が必要。
  - `compiler/runtime/src/stage.rs:12-17`

## TODO / 不明点

- Stage の `Alpha` 追加は仕様更新なのか暫定実装なのか確認する。
- `CapabilityError::code` が diagnostics 側のコード体系とどこまで一致しているか、章末で整理したい。
- `AuditEnvelope::validate` の必須キーが仕様の「必須メタデータ」一覧と完全一致しているか、spec 側を再確認する。
