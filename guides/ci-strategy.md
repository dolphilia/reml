# CI/テスト戦略ガイド

このガイドでは Reml プロジェクトを複数ターゲットで継続的に検証するためのベストプラクティスをまとめます。`guides/portability.md` と合わせて利用し、`RunConfig.extensions["target"]` と診断ポリシーの整合性を保ってください。

## 1. マルチターゲットマトリクス

| OS | アーキテクチャ | 推奨ジョブ | 備考 |
| --- | --- | --- | --- |
| Ubuntu 24.04 | x86_64 | `linux-x86_64` | 基準パフォーマンスベンチマーク、Packrat/左再帰 ON/OFF の両方を検証 |
| macOS 14 (arm64) | ARM64 | `mac-arm64` | Unicode/ファイルパス差異（`Core.Path`）と FFI 呼出規約（C ABI）を重点確認 |
| Windows Server 2022 | x86_64 | `windows-msvc` | `@cfg(target_os = "windows")` 分岐、Registry/Path 操作、MSVC 呼出規約を検証 |
| WASI Preview 2 | wasm32 | `wasi-sim` | `RunConfig.left_recursion` を `off` にした実行戦略、I/O 機能制限の確認 |

各ジョブでは以下の共通ステップを推奨します。

1. `remlc --target` で対象 Triple を指定しビルド。
2. `RunConfig.extensions["target"]` を JSON としてエクスポートし、アーティファクトに保存。
3. `reml-test`（将来の公式テストドライバ）または独自スクリプトで言語仕様テストを実行。
4. 失敗時の `Diagnostic.extensions["cfg"]` を収集し、ポータビリティ回帰を即座に可視化。

## 2. 環境変数と秘密情報

`Core.Env.infer_target_from_env()` は下表の環境変数を参照します。CI ではこれらを明示的に設定し、ログに残してください。

| 変数 | 意味 | 例 |
| --- | --- | --- |
| `REML_TARGET` | 既定ターゲット Triple | `x86_64-unknown-linux-gnu` |
| `REML_FEATURES` | カンマ区切りフィーチャ | `packrat_default,io.blocking.strict` |
| `REML_PROFILE` | ビルドプロファイル名 | `release`, `ci` |

秘密情報（API キー等）が必要なテストでは、`Core.Env` の `set_env`/`remove_env` を利用してスコープを限定し、`AuditEvent::EnvMutation` が監査ログに記録されるようにします。

## 3. FFI・ABI テスト

### 3.1 呼出規約の確認

* `resolve_calling_convention(platform_info(), metadata)` を用いた単体テストを各ターゲットで実行し、期待する `CallingConvention` が返ることを検証します。
* 失敗時の `FfiErrorKind::UnsupportedPlatform` が `target.config.unsupported_value` 診断を伴うかをアサートします。

### 3.2 ネイティブライブラリ取得

* Windows/MSVC 用バイナリは `REML_NATIVE_LIB_DIR` 等の環境変数でパスを渡し、CI 上での DLL 解決を追跡します。
* WASI テストでは FFI をスキップし、`@cfg` で無効化されたコードパスがコンパイルされないことを `cargo check` 相当のステップで確認します。

## 4. 診断メトリクスの収集

`guides/runtime-bridges.md` に記載された構造化ログを利用し、次の JSON フィールドを CI からメトリクスベースへ送信します。

```json
{
  "event": "reml.test",
  "target": "${REML_TARGET}",
  "features": ["packrat_default", "io.blocking.strict"],
  "diagnostics": {
    "target_config_errors": 0,
    "effects_cfg_contract": 0
  },
  "duration_ms": 1342
}
```

`target_config_errors` が 0 以外の場合は即座に失敗させ、レポートから問題の `@cfg` 分岐を特定します。

## 5. 推奨ワークフロー

1. **設定**: `setup-target` ステップで `REML_TARGET` と `RunConfig.extensions["target"]` を同期。
2. **ビルド**: `remlc --target` と `reml lint` を実行（syntax/spec 回帰を検出）。
3. **テスト**: プラットフォーム固有の統合テストを実行し、`Diagnostic` JSON を収集。
4. **レポート**: `guides/portability.md` のチェックリストに沿って結果を整理し、GitHub Projects などでトラッキング。
5. **自動化**: `platform_info()` から得た `capabilities` を使い、重いテスト（例: SIMD ベンチマーク）を必要ターゲットでのみ有効化。

---

今後、Phase 3 の新ターゲット（WASM/WASI・ARM64 組み込みなど）を組み込む際は、本ガイドをベースに追加チェックリストを拡張してください。

## 6. WASM 実機検証手順

### 6.1 実装タスク

- [ ] GitHub Actions に `wasmtime` をインストールするセットアップステップを追加（`actions/setup-python` + `wasmtime`）
- [ ] `ci/scripts/run-wasi.sh` を作成し、`remlc --target wasm32-wasi` と `wasmtime run` を連携
- [ ] `ci/scripts/collect-diagnostics.py` で `Diagnostic` JSON を集約し `ci-artifacts/wasi` に保存
- [ ] Nightly ワークフローでフルテスト、PR ワークフローではスモークテストのみを実行する YAML（`ci/wasi-nightly.yml` など）を追加
- [ ] `README.md` の CI 章に WASM ジョブのバッジとリンクを追記

1. **ツールチェーンの準備**: `wasmtime` または `wasmer` をインストールし、WASI Preview 2 対応ランタイムを CI ジョブに追加します。
2. **ターゲットビルド**: `remlc --target wasm32-wasi` でモジュールを生成し、`RunConfig.extensions["target"].extra.wasi = "preview2"` を書き出した JSON をアーティファクト化します。
3. **ランタイム試験**: `wasmtime run --dir=. build/main.wasm` のように実際の WASM ランタイムで DSL テストを実行し、`target_config_errors` が 0 であることを確認します。
4. **制約の検証**: Packrat/左再帰を無効化した構成と、WASI に適合しない機能（FFI、ネイティブ I/O）が `@cfg` により除去されているかをテストケースで明示します。
5. **レポート**: 実機テスト結果を `ci-artifacts/wasi/diagnostics.json` として保存し、`guides/portability.md` のチェックリストに沿って差分をレビューします。

> メモ: 実行時間の長いテストは nightly ジョブへ分離し、軽量スモークテストのみを PR 必須にすることで CI コストを抑えます。

