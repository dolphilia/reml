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
- 含まない: 実行経路の本格統合、WASM 実行基盤の本格実装（PoC は別ステップ）

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

### D. Bundle/署名検証（最小導入）
- [x] Plugin Bundle の最小署名検証導線（Strict/Permissive 方針）
  - `compiler/rust/runtime/src/runtime/plugin.rs`
- [x] `plugin.bundle_id` などの監査キーを監査ログへ転写
  - `compiler/rust/runtime/src/runtime/plugin.rs`
- [x] Plugin Bundle の読込・検証（署名/ハッシュ）
  - `compiler/rust/runtime/src/runtime/plugin.rs`

### E. ドキュメント整備（Bundle/CLI）
- [x] Bundle JSON 形式の仕様追記  
  - `docs/spec/4-7-core-parse-plugin.md`
- [x] `reml plugin install --bundle` のガイド追記  
  - `docs/guides/plugin-authoring.md`
  - `docs/guides/cli-workflow.md`

## 未実装（次フェーズ）

### F. 実行時ロード経路
- [ ] プラグインのロード/アンロード/実行ブリッジの統合
- [ ] 実行時の Capability 登録と Stage 検証の自動化

### G. CLI/運用導線
- [ ] `reml plugin install/verify` の最小 CLI
- [ ] `reml_capability list` に plugin 由来の出力を統合

### H. WASM 実行基盤（PoC）
- [ ] Wasmtime による最小ロード/実行 PoC
- [ ] WASM 実行時の Capability/監査転写検証

## 実装計画（次のステップ）
1. **PluginLoader と実行経路の接続**  
   - Bundle から `PluginLoader` を呼び出す導線を構築する。
2. **CLI/運用導線の初期化**  
   - `reml plugin install/verify` の最小導線を追加する。
3. **WASM PoC の開始**  
   - Wasmtime でロードし、Capability 監査の整合性を確認する。

## 実行経路と PluginLoader の接続設計（案）

### 1. 入力と責務
- **入力**: `bundle.json`（`docs/spec/4-7-core-parse-plugin.md` の形式）
- **責務**: バンドル読み込み → 署名/ハッシュ検証 → Manifest 読み込み → Capability 登録 → 監査ログ出力

### 2. 主要コンポーネント
- **CLI**: `reml plugin install --bundle <path> --policy <strict|permissive>`
- **PluginLoader**: `compiler/rust/runtime/src/runtime/plugin.rs`
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
  - Bundle JSON のパス。`docs/spec/4-7-core-parse-plugin.md` の形式に従う。
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
- 失敗時は `PluginLoadError` を日本語メッセージで出力する。

### 4. 失敗時の挙動
- **署名/ハッシュ不一致**: `Strict` は失敗、`Permissive` は警告ログのみで続行。
- **Manifest 読込失敗**: `PluginLoadError::BundleLoad` を返し登録を中断。
- **Capability 登録失敗**: 失敗したプラグインを含む登録を中断し、再実行可能な状態を維持。

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
- `compiler/rust/runtime/tests/verify_capability.rs`
- `compiler/rust/runtime/tests/plugin_loader.rs`
- `docs/plans/bootstrap-roadmap/assets/capability-handle-inventory.csv`

## 参照先
- `docs/spec/4-7-core-parse-plugin.md`
- `docs/spec/3-8-core-runtime-capability.md`
- `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md`
- `docs/notes/performance-optimization-research-20251221.md`
