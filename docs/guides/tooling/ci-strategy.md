# CI/テスト戦略ガイド

このガイドでは Reml プロジェクトを複数ターゲットで継続的に検証するためのベストプラクティスをまとめます。`../runtimeportability.md` と合わせて利用し、`RunConfig.extensions["target"]` と診断ポリシーの整合性を保ってください。

## 1. マルチターゲットマトリクス

| OS | アーキテクチャ | 推奨ジョブ | 備考 |
| --- | --- | --- | --- |
| Ubuntu 24.04 | x86_64 | `linux-x86_64` | 基準パフォーマンスベンチマーク、Packrat/左再帰 ON/OFF の両方を検証 |
| macOS 14 (arm64) | ARM64 | `mac-arm64` | Unicode/ファイルパス差異（`Core.Path`）と FFI 呼出規約（C ABI）を重点確認 |
| Windows Server 2022 | x86_64 | `windows-msvc` | `@cfg(target_os = "windows")` 分岐、Registry/Path 操作、MSVC 呼出規約を検証 |
| WASI Preview 2 | wasm32 | `wasi-sim` | `RunConfig.left_recursion` を `off` にした実行戦略、I/O 機能制限の確認 |

ジョブ開始時に以下のセットアップステップを追加すると、ターゲット差異の検出が容易になります。

```yaml
- name: Prepare Target Profile
  run: |
    reml target list
    reml target sync --write-cache
    reml toolchain install ${{ matrix.profile }} --auto-approve
    reml toolchain verify ${{ matrix.profile }} --output json > target-verify.json
  env:
    REML_TARGET_PROFILE: ${{ matrix.profile }}
    REML_TARGET_TRIPLE: ${{ matrix.triple }}
    REML_TARGET_CAPABILITIES: ${{ matrix.capabilities }}
```

Windows ジョブでは MSVC 用の環境変数を確実に設定し、診断ログをアーティファクト化するために以下のセットアップステップを追加します。

```yaml
- name: Setup Windows Toolchain
  shell: pwsh
  run: |
    pwsh -NoLogo -File tooling/toolchains/setup-windows-toolchain.ps1 -Quiet -NoCheck
    pwsh -NoLogo -File tooling/toolchains/check-windows-bootstrap-env.ps1 -OutputJson $env:CI_WINDOWS_ENV_JSON
  env:
    CI_WINDOWS_ENV_JSON: ${{ runner.temp }}\windows-env-check.json

- name: Upload Windows Toolchain Diagnostics
  uses: actions/upload-artifact@v4
  with:
    name: windows-env-check
    path: ${{ runner.temp }}\windows-env-check.json
```

`setup-windows-toolchain.ps1` は PowerShell プロファイル相当の PATH 初期化と `reml-msvc-env` 実行 (`vcvars64.bat` 呼び出し) を行い、続く診断スクリプトで `clang/llc/opt`・`cl/link/lib` のバージョンを安定して検出します。生成した JSON は `reports/windows-env-check.json` と同形式であり、CI 可観測性ダッシュボードや追跡ドキュメントに再利用できます。

各ジョブでは以下の共通ステップを推奨します。

1. `reml target validate` と `reml toolchain verify` を実行し、`target.profile.missing` / `target.abi.mismatch` が出ないことを確認。
2. `reml build --target $REML_TARGET_PROFILE --emit-metadata build/target.json` でビルドし、`RunArtifactMetadata` を保存。
3. `reml test --target $REML_TARGET_PROFILE --runtime smoke` などでテストを実行し、エミュレーション対象では `--runtime emulator=<name>` を指定する。
4. 失敗時の `Diagnostic.domain = Target` を `ci-artifacts/diagnostics.json` として収集し、ダッシュボードで可視化。

## 2. 環境変数と秘密情報

`Core.Env.infer_target_from_env()` は下表の環境変数を参照します。CI ではこれらを明示的に設定し、ログに残してください。

| 変数 | 意味 | 例 |
| --- | --- | --- |
| `REML_TARGET_PROFILE` | 使用する TargetProfile ID | `desktop-x86_64` |
| `REML_TARGET_TRIPLE` | 既定ターゲット Triple | `x86_64-unknown-linux-gnu` |
| `REML_TARGET_CAPABILITIES` | カンマ区切り Capability | `unicode.nfc,fs.case_sensitive` |
| `REML_TARGET_FEATURES` | カンマ区切りフィーチャ | `packrat_default,io.blocking.strict` |
| `REML_STD_VERSION` | 標準ライブラリの要求バージョン | `1.0.0` |
| `REML_RUNTIME_REVISION` | ランタイム互換リビジョン | `rc-2024-09` |

秘密情報（API キー等）が必要なテストでは、`Core.Env` の `set_env`/`remove_env` を利用してスコープを限定し、`AuditEvent::EnvMutation` が監査ログに記録されるようにします。

## 3. キャッシュとアーティファクト管理

### 3.1 Toolchain キャッシュ

- `REML_TOOLCHAIN_HOME` を CI キャッシュディレクトリ（例: `~/.cache/reml-toolchains`）に設定し、`actions/cache` で `toolchain-manifest.toml` と `std/`, `runtime/` ディレクトリを保存します。
- キャッシュヒット後は `reml toolchain verify --all --output json` を実行し、破損したアーティファクトがないかを検証します。失敗時はキャッシュを削除して再インストールしてください。

### 3.2 ビルドメタデータ

- `reml build --emit-metadata` が生成した `RunArtifactMetadata` をアーティファクトとしてアップロードし、後続ジョブやリリースパイプラインで再利用します。
- メタデータは `profile_id`・`hash`・`runtime_revision` を含むため、レジストリ公開 (`reml publish`) 前に差分比較が可能です。

## 4. エミュレーションとリモートテスト

| ターゲット | 推奨ランタイム | 設定 |
| --- | --- | --- |
| WASI Preview 2 | `wasmtime` | `reml test --target wasi-preview2 --runtime emulator=wasmtime --runtime-flags "--dir=."` |
| Linux/ARM64 on x86 CI | `qemu-aarch64` | `reml test --target mobile-arm64 --runtime emulator=qemu-aarch64 --runtime-flags "-L /usr/aarch64-linux-gnu"` |

- エミュレーションジョブでは `reml target sync --runtime emulator` を実行して実行時差異を明示し、`target.config.mismatch` を Warning としてレポートします。
- 実機検証が必要な場合は `../runtimecross-compilation.md` に記載のリモート実行テンプレートを利用してください。

## 5. FFI・ABI テスト

### 3.1 呼出規約の確認

* `resolve_calling_convention(platform_info(), metadata)` を用いた単体テストを各ターゲットで実行し、期待する `CallingConvention` が返ることを検証します。
* 失敗時の `FfiErrorKind::UnsupportedPlatform` が `target.config.unsupported_value` 診断を伴うかをアサートします。

### 3.2 ネイティブライブラリ取得

* Windows/MSVC 用バイナリは `REML_NATIVE_LIB_DIR` 等の環境変数でパスを渡し、CI 上での DLL 解決を追跡します。
* WASI テストでは FFI をスキップし、`@cfg` で無効化されたコードパスがコンパイルされないことを `cargo check` 相当のステップで確認します。

## 6. 診断メトリクスの収集

`../runtimeruntime-bridges.md` に記載された構造化ログを利用し、次の JSON フィールドを CI からメトリクスベースへ送信します。

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

## 7. 推奨ワークフロー

1. **設定**: `setup-target` ステップで `REML_TARGET` と `RunConfig.extensions["target"]` を同期。
2. **ビルド**: `reml build --target` と `reml fmt --check` を実行し、構文/仕様回帰を検出。
3. **テスト**: プラットフォーム固有の統合テストを実行し、`CliDiagnosticEnvelope` を収集。
4. **レポート**: `../runtimeportability.md` のチェックリストに沿って結果を整理し、GitHub Projects などでトラッキング。
5. **自動化**: `platform_info()` から得た `runtime_capabilities` を使い、重いテスト（例: SIMD ベンチマーク）を必要ターゲットでのみ有効化。

---

今後、Phase 3 の新ターゲット（WASM/WASI・ARM64 組み込みなど）を組み込む際は、本ガイドをベースに追加チェックリストを拡張してください。

## 8. WASM 実機検証手順

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
5. **レポート**: 実機テスト結果を `ci-artifacts/wasi/diagnostics.json` として保存し、`../runtimeportability.md` のチェックリストに沿って差分をレビューします。

> メモ: 実行時間の長いテストは nightly ジョブへ分離し、軽量スモークテストのみを PR 必須にすることで CI コストを抑えます。
