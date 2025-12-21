# 4.1 Core.Plugin Rust 実装計画

## 目的
- Core.Plugin（プラグインシステム）を Rust 実装へ段階的に導入し、Capability Registry / Manifest / Diagnostics との整合を確立する。
- Phase 4 で必要な最小導線（登録・監査）を先行し、Bundle/実行基盤/WASM 連携へ繋げる。

## 計画の前提
- 仕様: `docs/spec/4-7-core-parse-plugin.md`, `docs/spec/3-8-core-runtime-capability.md`
- 既存基盤: `CapabilityRegistry` / `ManifestCapabilities` / `AuditEnvelope`
- 方針: 安全性と監査可能性を優先し、実装は段階的に公開する。

## 作業スコープ
- 対象: `compiler/rust/runtime`, `compiler/rust/frontend`, `compiler/rust/runtime/ffi`
- 含む: Plugin Capability 登録・監査、Manifest 連携、最小ローダ
- 含まない: Bundle 署名検証、WASM 実行基盤の本格実装（PoC は別ステップ）

## 実施済み（完了状況）

### A. 基盤定義
- [x] `PluginCapability` / `PluginCapabilityMetadata` の定義  
  - `compiler/rust/runtime/src/capability/plugin.rs`
- [x] `CapabilityHandle::Plugin` / `CapabilityProvider::Plugin` の定義  
  - `compiler/rust/runtime/src/capability/handle.rs`  
  - `compiler/rust/runtime/src/capability/descriptor.rs`
- [x] FFI 側の Plugin 型定義  
  - `compiler/rust/runtime/ffi/src/capability_handle.rs`  
  - `compiler/rust/runtime/ffi/src/capability_metadata.rs`
- [x] Manifest の Plugin 種別定義  
  - `compiler/rust/runtime/src/config/manifest.rs`（`ProjectKind::Plugin` / `DslCategory::Plugin`）
- [x] Diagnostics の Plugin ドメイン  
  - `compiler/rust/frontend/src/diagnostic/model.rs`

### B. Capability Registry 連携（基盤）
- [x] `register_plugin_capability` の追加  
  - `compiler/rust/runtime/src/capability/registry.rs`
- [x] Capability 監査メタデータへの provider 情報反映  
  - `capability.provider` / `capability.provider.kind` / `plugin.*` を追加
- [x] Plugin Capability の登録テスト  
  - `compiler/rust/runtime/tests/verify_capability.rs`

### C. プラグインローダ導線（最小）
- [x] マニフェストから Capability を登録する `PluginLoader`  
  - `compiler/rust/runtime/src/runtime/plugin.rs`
- [x] マニフェスト経由の登録テスト  
  - `compiler/rust/runtime/tests/plugin_loader.rs`
- [x] ManifestCapabilities の列挙 API  
  - `compiler/rust/runtime/src/config/manifest.rs`（`iter` / `ids`）

## 未実装（次フェーズ）

### D. Bundle/署名検証
- [x] Plugin Bundle の最小署名検証導線（Strict/Permissive 方針）
  - `compiler/rust/runtime/src/runtime/plugin.rs`
- [x] `plugin.bundle_id` などの監査キーを監査ログへ転写
  - `compiler/rust/runtime/src/runtime/plugin.rs`
- [ ] Plugin Bundle の読込・検証（署名/ハッシュ）

### E. 実行時ロード経路
- [ ] プラグインのロード/アンロード/実行ブリッジの統合
- [ ] 実行時の Capability 登録と Stage 検証の自動化

### F. CLI/運用導線
- [ ] `reml plugin install/verify` の最小 CLI
- [ ] `reml_capability list` に plugin 由来の出力を統合

### G. WASM 実行基盤（PoC）
- [ ] Wasmtime による最小ロード/実行 PoC
- [ ] WASM 実行時の Capability/監査転写検証

## 実装計画（次のステップ）
1. **Bundle/署名検証の最小導入**  
   - Bundle 署名検証のスタブと監査キー定義を追加する。
2. **PluginLoader と実行経路の接続**  
   - Bundle から `PluginLoader` を呼び出す導線を構築する。
3. **CLI/運用導線の初期化**  
   - `reml plugin install/verify` の最小導線を追加する。
4. **WASM PoC の開始**  
   - Wasmtime でロードし、Capability 監査の整合性を確認する。

## 進捗の確認方法
- `compiler/rust/runtime/tests/verify_capability.rs`
- `compiler/rust/runtime/tests/plugin_loader.rs`
- `docs/plans/bootstrap-roadmap/assets/capability-handle-inventory.csv`

## 参照先
- `docs/spec/4-7-core-parse-plugin.md`
- `docs/spec/3-8-core-runtime-capability.md`
- `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md`
- `docs/notes/performance-optimization-research-20251221.md`
