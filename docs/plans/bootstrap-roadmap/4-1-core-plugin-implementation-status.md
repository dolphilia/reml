# 4.1 Core.Plugin Rust 実装計画

## 目的
- Core.Plugin（プラグインシステム）を Rust 実装へ段階的に導入し、Capability Registry / Manifest / Diagnostics との整合を確立する。
- Phase 4 で必要な最小導線（登録・監査）を先行し、Bundle/実行基盤/WASM 連携へ繋げる。

## 計画の前提
- 仕様: `docs/spec/5-7-core-parse-plugin.md`, `docs/spec/3-8-core-runtime-capability.md`
- 既存基盤: `CapabilityRegistry` / `ManifestCapabilities` / `AuditEnvelope`
- 方針: 安全性と監査可能性を優先し、実装は段階的に公開する。

## 作業スコープ
- 対象: `compiler/runtime`, `compiler/frontend`, `compiler/runtime/ffi`
- 含む: Plugin Capability 登録・監査、Manifest 連携、最小ローダ
- 含まない: 実行経路の本格統合、WASM 実行基盤の本格実装（PoC は別ステップ）

## 実施済み（完了状況）

### A. 基盤定義
- [x] `PluginCapability` / `PluginCapabilityMetadata` の定義  
  - `compiler/runtime/src/capability/plugin.rs`
- [x] `CapabilityHandle::Plugin` / `CapabilityProvider::Plugin` の定義  
  - `compiler/runtime/src/capability/handle.rs`  
  - `compiler/runtime/src/capability/descriptor.rs`
- [x] FFI 側の Plugin 型定義  
  - `compiler/runtime/ffi/src/capability_handle.rs`  
  - `compiler/runtime/ffi/src/capability_metadata.rs`
- [x] Manifest の Plugin 種別定義  
  - `compiler/runtime/src/config/manifest.rs`（`ProjectKind::Plugin` / `DslCategory::Plugin`）
- [x] Diagnostics の Plugin ドメイン  
  - `compiler/frontend/src/diagnostic/model.rs`

### B. Capability Registry 連携（基盤）
- [x] `register_plugin_capability` の追加  
  - `compiler/runtime/src/capability/registry.rs`
- [x] Capability 監査メタデータへの provider 情報反映  
  - `capability.provider` / `capability.provider.kind` / `plugin.*` を追加
- [x] Plugin Capability の登録テスト  
  - `compiler/runtime/tests/verify_capability.rs`

### C. プラグインローダ導線（最小）
- [x] マニフェストから Capability を登録する `PluginLoader`  
  - `compiler/runtime/src/runtime/plugin.rs`
- [x] マニフェスト経由の登録テスト  
  - `compiler/runtime/tests/plugin_loader.rs`
- [x] ManifestCapabilities の列挙 API  
  - `compiler/runtime/src/config/manifest.rs`（`iter` / `ids`）

### D. Bundle/署名検証（最小導入）
- [x] Plugin Bundle の最小署名検証導線（Strict/Permissive 方針）
  - `compiler/runtime/src/runtime/plugin.rs`
- [x] `plugin.bundle_id` などの監査キーを監査ログへ転写
  - `compiler/runtime/src/runtime/plugin.rs`
- [x] Plugin Bundle の読込・検証（署名/ハッシュ）
  - `compiler/runtime/src/runtime/plugin.rs`

### E. ドキュメント整備（Bundle/CLI）
- [x] Bundle JSON 形式の仕様追記  
  - `docs/spec/5-7-core-parse-plugin.md`
- [x] `reml plugin install --bundle` のガイド追記  
  - `docs/guides/dsl/plugin-authoring.md`
  - `docs/guides/tooling/cli-workflow.md`
- [x] `--output json` の例とスキーマ参照を追加  
  - `docs/spec/5-7-core-parse-plugin.md`
  - `docs/guides/tooling/cli-workflow.md`
  - `docs/schemas/plugin-bundle-registration.schema.json`

### F. 実装安定化・テスト実行
- [x] プラグイン系テストの並列干渉を抑止するテストロックを追加  
  - `compiler/runtime/src/test_support.rs`  
  - `compiler/runtime/tests/plugin_loader.rs`  
  - `compiler/runtime/tests/plugin_manager.rs`
- [x] `PluginLoader` 系テストの構築ミス修正（`CapabilityId`/`Manifest` 初期化）
  - `compiler/runtime/tests/plugin_loader.rs`
- [x] `stage_requirement_label` 参照を `runtime/plugin.rs` 内に集約し、`metrics` 依存を回避  
  - `compiler/runtime/src/runtime/plugin.rs`
- [x] `BundleContext` を `plugin_manager` から参照可能に調整  
  - `compiler/runtime/src/runtime/plugin.rs`
- [x] テスト実行（対象: `plugin_`）  
  - `cargo test plugin_`  
  - `cargo test plugin_ -- --test-threads=1`

## 未実装（次フェーズ）

### F. 実行時ロード経路
- [ ] プラグインのロード/アンロード/実行ブリッジの統合
- [ ] 実行時の Capability 登録と Stage 検証の自動化
- [ ] 実行時ロード経路の責務分離（ロード管理・実行ブリッジ・監査/診断）

#### F.1 ライフサイクル設計（ロード/アンロード/再ロード）
1. [x] `PluginRuntimeManager`（仮）を追加し、ロード状態 (`Loaded`/`Failed`/`Unloaded`) を管理する。  
   - `compiler/runtime/src/runtime/plugin_manager.rs`
2. [x] `PluginLoader` と実行ブリッジを接続する `load_bundle_and_attach`（仮）を用意する。  
   - `compiler/runtime/src/runtime/plugin_manager.rs`
3. [x] `unload` 時に登録済み Capability を整理し、再ロード時に重複登録を防ぐ。  
   - `compiler/runtime/src/runtime/plugin_manager.rs`  
   - `compiler/runtime/src/capability/registry.rs`（`unregister` 追加）
4. [x] 監査ログは `plugin.install` / `plugin.revoke` / `plugin.verify_signature` / `plugin.signature.failure` を優先し、`plugin.register_capability` と相互参照できるキーを揃える。  
   - `compiler/runtime/src/runtime/plugin.rs`

**配置先と公開 API（確定）**
- 配置先: `compiler/runtime/src/runtime/plugin_manager.rs`
- 公開 API:
  - `pub struct PluginRuntimeManager`
  - `pub enum PluginRuntimeState { Loaded, Failed, Unloaded }`
  - `pub struct PluginRuntimeHandle { bundle_id: String, plugin_id: String }`
  - `pub fn new(loader: PluginLoader, bridge: Box<dyn PluginExecutionBridge>) -> Self`
  - `pub fn load_bundle_and_attach(&self, path: impl AsRef<Path>, policy: VerificationPolicy) -> Result<PluginBundleRegistration, PluginError>`
  - `pub fn unload(&self, plugin_id: &str) -> Result<(), PluginError>`
  - `pub fn state_of(&self, plugin_id: &str) -> Option<PluginRuntimeState>`

#### F.2 実行ブリッジ統合（ネイティブ/将来の WASM）
1. [x] `PluginExecutionBridge`（仮）トレイトを追加し、`load` / `invoke` / `unload` の責務を統一する。  
   - `compiler/runtime/src/runtime/plugin_bridge.rs`
2. [x] ネイティブ実装は最小のスタブで開始し、`RuntimeBridgeRegistry` に Stage 検証記録を残す。  
   - `compiler/runtime/src/runtime/plugin_bridge.rs`
3. [x] 失敗時は `PluginError::VerificationFailed` / `PluginError::IO` に寄せ、Diagnostics へ変換できるようにする。  
   - `compiler/runtime/src/runtime/plugin.rs`  
   - `compiler/runtime/src/runtime/plugin_bridge.rs`
4. [x] `PluginError` を `GuardDiagnostic` へ変換し、`bridge.*` を監査メタデータへ転写する。  
   - `compiler/runtime/src/runtime/plugin.rs`  
   - `compiler/runtime/src/runtime/bridge.rs`  
   - `compiler/runtime/src/io/bridge.rs`
5. [x] `plugin.invoke` の最小テストを追加する。  
   - `compiler/runtime/tests/plugin_bridge.rs`

**配置先と公開 API（確定）**
- 配置先: `compiler/runtime/src/runtime/plugin_bridge.rs`
- 公開 API:
  - `pub trait PluginExecutionBridge`
  - `pub struct PluginInstance { plugin_id: String }`
  - `pub struct PluginInvokeRequest { entrypoint: String, payload: Vec<u8> }`
  - `pub struct PluginInvokeResponse { payload: Vec<u8> }`
  - `fn load(&self, manifest: &Manifest) -> Result<PluginInstance, PluginError>`
  - `fn invoke(&self, instance: &PluginInstance, request: PluginInvokeRequest) -> Result<PluginInvokeResponse, PluginError>`
  - `fn unload(&self, instance: PluginInstance) -> Result<(), PluginError>`

#### F.3 Capability 登録と Stage 検証の自動化
1. [x] `PluginRuntimeManager` から `register_manifest` を呼び出し、ロードと同時に Capability を登録する。  
   - `compiler/runtime/src/runtime/plugin_manager.rs`
2. [x] `verify_capability_stage` と `StageRequirement` を橋渡しし、Stage mismatch を `effects.contract.stage_mismatch` へ転写する。  
   - `compiler/runtime/src/runtime/plugin_manager.rs`  
   - `compiler/runtime/src/runtime/plugin.rs`
3. [x] ロード失敗時は登録済み Capability をロールバックし、`PluginError::BundleInstallFailed` に寄せる。  
   - `compiler/runtime/src/runtime/plugin_manager.rs`

#### F.4 受け入れ条件
- [x] `bundle.json` を指定したロードで `plugin.verify_signature`/`plugin.install` が監査ログに揃って出力される。  
  - `compiler/runtime/tests/plugin_manager.rs`
- [x] `RuntimeBridgeRegistry` の Stage 記録と Capability Registry の Stage が一致する。  
  - `compiler/runtime/tests/plugin_manager.rs`
- [x] アンロード時に Capability の重複登録が起きず、再ロードが可能である。  
  - `compiler/runtime/tests/plugin_manager.rs`

**検証項目（F.4）**
- [x] `PluginRuntimeManager::load_bundle_and_attach` で bundle をロードし、`take_plugin_audit_events` に `plugin.verify_signature` / `plugin.install` が揃っていること。  
  - `compiler/runtime/src/runtime/plugin_manager.rs`  
  - `compiler/runtime/src/runtime/plugin.rs`  
  - `compiler/runtime/tests/plugin_manager.rs`
- [x] `RuntimeBridgeRegistry::stage_records` の `required/actual` と `CapabilityRegistry::describe` の `stage` が一致すること。  
  - `compiler/runtime/src/runtime/plugin_bridge.rs`  
  - `compiler/runtime/src/runtime/bridge.rs`  
  - `compiler/runtime/tests/plugin_manager.rs`
- [x] `unload` 後に `CapabilityRegistry::describe` が `NotRegistered` を返し、同一 bundle を再ロードしても `AlreadyRegistered` が発生しないこと。  
  - `compiler/runtime/src/runtime/plugin_manager.rs`  
  - `compiler/runtime/src/capability/registry.rs`  
  - `compiler/runtime/tests/plugin_manager.rs`

### G. CLI/運用導線

#### G.1 目的とスコープ
- [x] Phase 4 で必要な **最小導線**（`install`/`verify` と Capability の可視化）を固め、運用ログと監査キーを揃える
- [x] 仕様/ガイド（`docs/spec/5-7-core-parse-plugin.md`, `docs/guides/tooling/cli-workflow.md`）と整合する CLI 挙動を確定する

#### G.2 CLI 仕様（MVP）
- [x] `reml plugin install --bundle <path> --policy <strict|permissive> [--output human|json]`
  - `PluginLoader::register_bundle_path` までを通し、登録結果（`PluginBundleRegistration` 相当）を返す
  - `--output json` は `docs/schemas/plugin-bundle-registration.schema.json` に準拠
- [x] `reml plugin verify --bundle <path> --policy <strict|permissive> [--output human|json]`
  - 署名/ハッシュ検証まで実行し、**Capability 登録は行わない**
  - 出力は `bundle_id`/`bundle_version`/`signature_status`/`bundle_hash`/`manifest_paths` を最小セットとする
- [x] `reml_capability list` に plugin 由来の情報を統合
  - `provider=plugin` / `plugin_id` / `bundle_id` / `stage` / `registered_at` を表示
  - `--format json` では `CapabilityDescriptor` 相当を返す

#### G.3 実装タスク
- [x] CLI 側で `PluginLoader` / `PluginRuntimeManager` を呼び出す最小ラッパを追加し、`VerificationPolicy` を引き回す
- [x] エラーは `PluginError` を日本語メッセージへ変換し、終了コードを `0/1` に統一
- [x] 監査ログのキーを CLI の出力と一致させる  
  - `plugin.verify_signature` / `plugin.signature.failure` / `plugin.install` / `plugin.revoke`
- [x] 既存ガイドの記述に合わせ、`--output json` のサンプルを更新する（必要時）

#### G.4 受け入れ条件（最小）
- [x] `reml plugin install` 実行時に `plugin.verify_signature` と `plugin.install` が監査ログへ出力される
- [x] `reml plugin verify` 実行時に Capability が登録されず、`signature_status` が JSON 出力へ反映される
- [x] `reml_capability list` が plugin 由来 Capability の `provider=plugin` を表示できる
  - 検証メモ: `cargo test --manifest-path compiler/runtime/Cargo.toml plugin_` はパス済み

### H. WASM 実行基盤（PoC）

#### H.1 目的とスコープ
- [x] `PluginExecutionBridge` の WASM 実装を PoC で検証し、**ロード/呼び出し/アンロード** が動作することを確認する
- [x] 監査・Stage 検証の転写を確認し、`RuntimeBridgeRegistry` の記録が揃うことを確認する
- [x] 本格実装は Phase 5 以降とし、**WASI/ホスト I/O の解放は行わない**（PoC は最小権限）

#### H.2 実装タスク（PoC）
- [x] `PluginExecutionBridge` の WASM 版（例: `PluginWasmBridge`）を追加し、Wasmtime で `load`/`invoke`/`unload` を実装する
- [x] `PluginInvokeRequest.entrypoint` を WASM export 名へ対応づけ、`payload` はバイナリ引数として渡す
- [x] WASM モジュールのロード時に `bundle_hash` と `module_hash` を監査メタデータへ転写する
- [x] `bridge.kind=wasm` / `bridge.engine=wasmtime` を `RuntimeBridgeRegistry` に記録する

#### H.3 監査/Capability 連携
- [x] `plugin.verify_signature` / `plugin.install` の監査イベントに WASM モジュール情報を追加する
- [x] `verify_capability_stage` の結果と `RuntimeBridgeRegistry` の Stage 記録が一致することを確認する

#### H.4 受け入れ条件（PoC）
- [x] テスト用 bundle から WASM プラグインをロードし、1 回の `invoke` が成功する
- [x] `RuntimeBridgeRegistry` に `bridge.kind=wasm` が記録され、`CapabilityRegistry::describe` と Stage が一致する
- [x] 監査ログに `plugin.verify_signature` と `plugin.install` が揃い、`bundle_hash` が残る

#### 🧪 追試ログ（WASM プラグイン）
- `cargo test --manifest-path compiler/runtime/Cargo.toml plugin_ -- --test-threads=1` を実行したが、`wasm-encoder v0.243.0` が `rustc 1.76+` を要求するため失敗（現行 `rustc 1.69.0`）。
- 再実行には Rust toolchain の更新、または `wasm-encoder` の互換バージョン固定が必要。
- `wat=1.0.68`（`wasm-encoder v0.31.1`）と `url=2.3.1` / `bumpalo=3.12.0` を固定し、`wasmtime=6.0.2`（`default-features = false`, `features = ["cranelift"]`）に更新したうえで再実行。`plugin_` テストは成功し、WASM ブリッジ経路の回帰確認が完了。
- `cargo test --manifest-path compiler/runtime/Cargo.toml --test plugin_wasm_bridge -- --test-threads=1` を実行し、`wasm_bridge_loads_bundle_and_invokes` が成功することを確認。
- `cargo test --manifest-path compiler/runtime/Cargo.toml --test plugin_loader -- --test-threads=1` を実行し、`plugin_loader` の 3 テストが成功することを確認。
- `cargo test --manifest-path compiler/runtime/Cargo.toml --test plugin_manager -- --test-threads=1` を実行し、`plugin_manager` の 3 テストが成功することを確認。

## 実装計画（次のステップ）
1. **PluginLoader と実行経路の接続**  
   - Bundle から `PluginLoader` を呼び出す導線を構築する。
2. **CLI/運用導線の初期化**  
   - `reml plugin install/verify` の最小導線を追加する。
3. **WASM PoC の開始**  
   - Wasmtime でロードし、Capability 監査の整合性を確認する。

## 実行経路と PluginLoader の接続設計（案）

### 1. 入力と責務
- **入力**: `bundle.json`（`docs/spec/5-7-core-parse-plugin.md` の形式）
- **責務**: バンドル読み込み → 署名/ハッシュ検証 → Manifest 読み込み → Capability 登録 → 監査ログ出力

### 2. 主要コンポーネント
- **CLI**: `reml plugin install --bundle <path> --policy <strict|permissive>`
- **PluginLoader**: `compiler/runtime/src/runtime/plugin.rs`
  - `register_bundle_path` → `register_bundle`
- **CapabilityRegistry**: `register_plugin_capability` を通じて Capability 登録
- **Audit**: `plugin.verify_signature` / `plugin.install` を監査ログに記録

### 3. 呼び出しフロー（概要）
1. CLI が `bundle.json` を読み込み、`PluginLoader::register_bundle_path` を呼び出す。
2. `PluginLoader` が `bundle.json` を解析し、Manifest の内容から `bundle_hash` を算出。
3. `VerificationPolicy` に従い署名/ハッシュを検証し、`plugin.verify_signature` を監査ログへ出力。
4. バンドル内の各 `manifest_path` を読み込み、`register_manifest` を呼び出して Capability を登録。
5. 各プラグインの登録完了時に `plugin.install` を監査ログへ出力。

### 3.1 CLI 引数/戻り値（確定）

```bash
reml plugin install --bundle <path> --policy <strict|permissive> [--output human|json]
```

- **必須**: `--bundle`  
  - Bundle JSON のパス。`docs/spec/5-7-core-parse-plugin.md` の形式に従う。
- **任意**: `--policy`  
  - 既定値は `strict`。`permissive` は警告のみで続行。
- **任意**: `--output`  
  - 既定値は `human`。`json` の場合は `PluginBundleRegistration` 相当を出力する。

**戻り値（終了コード）**
- `0`: 登録成功（`PluginBundleRegistration` を出力）
- `1`: Bundle 読み込み/解析失敗、署名検証失敗、Manifest 読み込み失敗、Capability 登録失敗

**標準出力（`--output=json`）**
- `PluginBundleRegistration` 相当の JSON を返す:
  - `bundle_id`, `bundle_version`
  - `signature_status`
  - `plugins[{ plugin_id, capabilities[] }]`

**標準エラー**
- 失敗時は `PluginError` を日本語メッセージで出力する。

### 4. 失敗時の挙動
- **署名/ハッシュ不一致**: `Strict` は失敗、`Permissive` は警告ログのみで続行。
- **Manifest 読込失敗**: `PluginError::IO` を返し登録を中断。
- **Capability 登録失敗**: 失敗したプラグインを含む登録を中断し、`PluginError::BundleInstallFailed` を返す。

### 5. 監査キー（最小セット）
- `plugin.bundle_id`, `plugin.bundle_version`
- `plugin.bundle_hash`, `plugin.signature.bundle_hash`
- `plugin.signature.status`, `plugin.signature.algorithm`
- `plugin.id`, `plugin.capabilities`

### 6. 受け入れ条件
- CLI から `bundle.json` を指定して `PluginLoader` が呼ばれること。
- `plugin.verify_signature` / `plugin.install` が監査ログに残ること。
- Capability 登録結果が `reml_capability list --format json` に反映されること。

## 進捗の確認方法
- `compiler/runtime/tests/verify_capability.rs`
- `compiler/runtime/tests/plugin_loader.rs`
- `docs/plans/bootstrap-roadmap/assets/capability-handle-inventory.csv`

## 参照先
- `docs/spec/5-7-core-parse-plugin.md`
- `docs/spec/3-8-core-runtime-capability.md`
- `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md`
- `docs/notes/backend/performance-optimization-research-20251221.md`
