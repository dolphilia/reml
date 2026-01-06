# 調査メモ: 第13章 ランタイムの全体像

## 対象モジュール

- `compiler/runtime/src/runtime/mod.rs`
- `compiler/runtime/src/runtime/api.rs`
- `compiler/runtime/src/runtime/bridge.rs`
- `compiler/runtime/src/runtime/async_bridge.rs`
- `compiler/runtime/src/runtime/plugin.rs`
- `compiler/runtime/src/runtime/plugin_manager.rs`
- `compiler/runtime/src/runtime/plugin_bridge.rs`
- `compiler/runtime/src/runtime/signal.rs`
- `compiler/runtime/src/embedding.rs`
- `compiler/runtime/src/run_config.rs`
- `compiler/runtime/src/stage.rs`
- `compiler/runtime/src/lib.rs`

## 入口と全体像

- ランタイムの中核は `runtime` モジュール群で、Capability 検証 (`api`)、Stage 監査の記録 (`bridge`)、プラグイン管理 (`plugin` / `plugin_manager` / `plugin_bridge`) を束ねる。
  - `compiler/runtime/src/runtime/mod.rs:1-9`
- Capability 検証のエントリポイントは `guard_capability` 系関数で、`CapabilityRegistry` に対して Stage/Effect を照会する。
  - `compiler/runtime/src/runtime/api.rs:36-98`
- Stage は `StageId` と `StageRequirement` によって管理され、`satisfies` 判定で許可判定を行う。
  - `compiler/runtime/src/stage.rs:7-105`
- プラグインは「バンドル + マニフェスト」で読み込まれ、署名検証 → Capability 登録 → 実行ブリッジ接続の順で扱う。
  - `compiler/runtime/src/runtime/plugin.rs:378-604`
  - `compiler/runtime/src/runtime/plugin_manager.rs:58-215`
  - `compiler/runtime/src/runtime/plugin_bridge.rs:34-256`
- 埋め込み API は `embedding.rs` の C ABI 関数群が入口で、最小フロー（create → load → run → dispose）を提供する。
  - `compiler/runtime/src/embedding.rs:53-231`

## データ構造

- **Stage/Requirement**: `StageId` と `StageRequirement` が Stage 文字列の解析と比較を提供する。
  - `compiler/runtime/src/stage.rs:7-127`
- **RunConfig**: パーサ実行に利用する構成。`extensions` に名前空間ごとの拡張設定を保持する。
  - `compiler/runtime/src/run_config.rs:23-132`
- **CapabilityGuard**: Capability 検証の結果（要求 Stage と実際の Stage）を保持するガード。
  - `compiler/runtime/src/runtime/api.rs:6-98`
- **BridgeStageRecord / RuntimeBridgeRegistry**: ブリッジが Stage 検証を実行した履歴を保持し、監査へ転写する。
  - `compiler/runtime/src/runtime/bridge.rs:9-164`
- **プラグイン関連**: `PluginBundleManifest` / `PluginRegistration` / `PluginBundleRegistration` がバンドル/登録結果を表現する。
  - `compiler/runtime/src/runtime/plugin.rs:73-109`
- **PluginRuntimeManager**: 実行時のロード状態 (`PluginRuntimeState`) とインスタンスを管理する。
  - `compiler/runtime/src/runtime/plugin_manager.rs:15-215`
- **PluginExecutionBridge**: ネイティブ実装と Wasm 実装の共通インターフェース。
  - `compiler/runtime/src/runtime/plugin_bridge.rs:34-256`
- **埋め込み API**: `RemlEmbedContext` と `RemlEmbedStatus` が C ABI の状態と戻り値を管理する。
  - `compiler/runtime/src/embedding.rs:20-231`
- **Signal**: `Signal` / `SignalInfo` / `SignalError` が `Core.Runtime` の信号モデルを表す。
  - `compiler/runtime/src/runtime/signal.rs:6-107`
- **ActorSystem**: `Core.Async` と `Core.Dsl.Actor` の橋渡しとして in-memory 実装を提供する。
  - `compiler/runtime/src/runtime/async_bridge.rs:1-97`

## コアロジック

- **Capability 検証**: `guard_capability` が `CapabilityRegistry::verify_capability_stage` を呼び出し、実際の Stage を `CapabilityGuard` として返す。
  - `compiler/runtime/src/runtime/api.rs:36-98`
- **Stage 監査記録**: `RuntimeBridgeRegistry` が Stage 検証の履歴を記録し、`attach_bridge_stage_metadata` が監査メタデータへ反映する。
  - `compiler/runtime/src/runtime/bridge.rs:42-164`
- **プラグイン登録/検証**:
  - 署名検証は `verify_plugin_signature` が行い、Strict モードでは署名未検出・不一致を失敗扱いにする。
    - `compiler/runtime/src/runtime/plugin.rs:547-604`
  - `PluginLoader::register_manifest_with_context` が Manifest 由来の Capability を Registry に登録する。
    - `compiler/runtime/src/runtime/plugin.rs:480-522`
  - `PluginRuntimeManager::load_bundle_and_attach` がバンドルを読み込み、署名検証 → Capability 登録 → ブリッジロードまでの一連のフローを実行する。
    - `compiler/runtime/src/runtime/plugin_manager.rs:58-215`
- **プラグインブリッジ**:
  - Native 実装はマニフェストの Stage をそのまま記録し、簡易な entrypoint でレスポンスを返す。
    - `compiler/runtime/src/runtime/plugin_bridge.rs:62-109`
  - Wasm 実装は Wasmtime でモジュールを読み込み、memory 書き込み/関数呼び出しで payload を往復する。
    - `compiler/runtime/src/runtime/plugin_bridge.rs:133-256`
- **埋め込み API**:
  - `reml_create_context` が ABI 互換性とターゲット対応を検証し、`RemlEmbedContext` を生成する。
    - `compiler/runtime/src/embedding.rs:53-95`
  - `reml_load_module` は UTF-8 文字列検証とロード状態更新を行う。
    - `compiler/runtime/src/embedding.rs:97-129`
  - `reml_run` はロード済みか確認し、監査イベントを記録する。
    - `compiler/runtime/src/embedding.rs:131-152`
- **RunConfig マニフェスト反映**: `apply_manifest_overrides` が `reml.toml` の情報を `RunConfig` 拡張へ転写する。
  - `compiler/runtime/src/run_config.rs:75-132`

## 仕様との対応メモ

- Capability / Stage / StageRequirement は `docs/spec/3-8-core-runtime-capability.md` と対応する。ただしコード側は `StageId::Alpha` を持つため、仕様側との差分がある。
- RunConfig のマニフェスト転写は `docs/spec/3-7-core-config-data.md` の `Manifest`/互換レイヤの説明と対応する。

## TODO / 不明点

- `StageId` に `Alpha` が存在するが、仕様側は `Experimental/Beta/Stable` のみのため同期方針を確認したい。
- `embedding.rs` の C ABI は `docs/spec` に明示された節が見当たらないため、仕様上の位置づけを整理する必要がある。
