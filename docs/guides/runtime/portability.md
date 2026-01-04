# ポータビリティガイド

本ガイドは Reml プロジェクトで複数プラットフォームを対象とした開発を行う際の実務手順をまとめたものです。`0-1-project-purpose.md` が掲げる「実用性能」と「安全性」を満たしつつ、ターゲット差異を安全に扱うための仕様導線と実例を以下に整理します。

## 1. ターゲットプロファイルとツールチェーン整備

### 1.1 `reml target` によるプロファイル選択

1. `reml target list` で利用可能な `TargetProfile` を確認し、`profile_id` と `runtime_revision` を把握します。
2. 必要に応じて `reml target show <profile>` で `capabilities`・`stdlib_version` の詳細を閲覧し、`TargetCapability` が要求する機能を理解します。
3. プロジェクト固有のプロファイルを作成する場合は `reml target scaffold <id>` を実行し、生成された `profiles/<id>.toml` に `capabilities = ["unicode.nfc", ...]` のような宣言を記述します。
4. CI や新しいマシンでは `reml target validate <id>` を用いてプロファイル整合性を確認し、`target.profile.missing` や `target.capability.unknown` を早期に検知します。

### 1.2 `reml toolchain` と事前ビルド標準ライブラリ

`reml toolchain install <profile>` を実行して対応する標準ライブラリ（`artifact/std/<triple>/<hash>`）とランタイム（`runtime/<profile>`）を取得します。`toolchain-manifest.toml` に記録された `hash`/`runtime_revision` は `RunArtifactMetadata` と一致する必要があり、不一致が検出された場合は `target.abi.mismatch` が報告されます。

### 1.3 `remlc` のターゲット指定

`remlc --target` で LLVM Triple を明示します（README.md 参照）。Toolchain をインストール済みであれば、コンパイル時に `RunConfigTarget.triple` が `remlc --target` と一致しているかを自動検証できます。

```bash
# Windows 向けビルド
remlc --target x86_64-pc-windows-msvc src/main.reml

# Apple Silicon 向けビルド
remlc --target aarch64-apple-darwin src/main.reml
```

CI/CD では `REML_TARGET_PROFILE`・`REML_TARGET_TRIPLE`・`REML_TARGET_CAPABILITIES` 等の環境変数を設定し、後述の `Core.Env.infer_target_from_env` で取得します。

### 1.4 `RunConfig.extensions["target"]`

`2-6-execution-strategy.md` に定義された構造体を利用し、コンパイル時/実行時ターゲットを同期します。

```reml
let cfg = RunConfig {
  packrat = platform_info().runtime_capabilities.contains(RuntimeCapability::SIMD),
  extensions = {
    "target": {
      os: platform_info().os.to_string(),
      family: family_tag(platform_info()),
      arch: platform_info().arch.to_string(),
      abi: Some("gnu"),
      vendor: platform_info().variant,
      profile_id: Some("desktop-x86_64"),
      triple: Some("x86_64-unknown-linux-gnu"),
      features: project_features(active_profile),
      capabilities: target_capabilities(),
      stdlib_version: Some(semver!("1.0.0")),
      runtime_revision: Some("rc-2024-09"),
      diagnostics: true,
      extra: { "gpu": "cuda" }
    }
  }
}
```

`diagnostics=true` を設定すると `@cfg` 評価ログが `Diagnostic.extensions["cfg"]` に出力され、誤設定を検出できます（2-5-error.md）。`profile_id` や `runtime_revision` が欠落している場合は `target.profile.missing` が、値が不整合な場合は `target.abi.mismatch` / `target.config.mismatch` が発生します。

## 2. 条件付きコンパイルの設計

### 2.1 `@cfg` 属性の使用

`1-1-syntax.md` で規定された述語を用いて宣言や `use` 文を切り替えます。

```reml
@cfg(all(target_family = "unix", not(feature = "legacy_fs")))
use Core.Platform.Posix

@cfg(target_os = "windows")
fn open_registry() -> Result<(), PlatformError> { ... }
```

未定義キーや値は `target.config.unknown_key` / `target.config.unsupported_value` として停止します。複数定義の効果集合が矛盾する場合は `effects.cfg.contract_violation` が発生するため、効果タグの整合性を先に検証してください（1-3-effects-safety.md）。

### 2.2 代表的なキー

| キー | 用途 | 典型値 |
| --- | --- | --- |
| `target_os` | OS 判別 | `"windows"`, `"linux"`, `"macos"`, `"freebsd"`, `"wasm"` |
| `target_family` | 共通分岐 | `"unix"`, `"windows"`, `"wasm"` |
| `target_arch` | ABI/命令差異 | `"x86_64"`, `"aarch64"`, `"wasm32"` |
| `target_abi` | ツールチェーン/ABI 分岐 | `"gnu"`, `"msvc"`, `"musl"` |
| `target_profile` / `profile_id` | プロファイル固有切替 | `"desktop-x86_64"`, `"mobile-arm64"` |
| `runtime_revision` | ランタイム互換性 | `"rc-2024-09"` |
| `stdlib_version` | 標準ライブラリ互換性 | `"1.0.0"` |
| `capability` | ターゲット Capability | `"unicode.nfc"`, `"fs.case_insensitive"`, `"ffi.callconv.win64"` |
| `feature` | ビルド切替 | `"gpu_acceleration"`, `"use_packrat"` |
| `extra.*` | プロジェクト拡張 | 例: `extra.gpu = "cuda"` |

## 3. 環境・プラットフォーム API

### 3.1 `Core.Env`（3-10-core-env.md）

* `get_env` / `set_env` / `remove_env` で環境変数アクセスを統一。
* `get_temp_dir` や `cache_dir` を利用し、XDG/AppData などプラットフォーム標準のパスを取得。
* `infer_target_from_env` は `TargetProfile` を返し、`resolve_run_config_target` → `merge_runtime_target` を経て `RunConfig.extensions["target"]` へマージできます。エラー発生時は `Diagnostic.domain = Target` でレポートされるため、CI では `--fail-on-warning` を有効にしてポータビリティ回帰を即停止させます。

```reml
let profile = infer_target_from_env()?;
let target = resolve_run_config_target(profile);
let merged = merge_runtime_target(target, platform_info());
cfg.extensions.insert("target", merged);
```

### 3.2 `platform_info()`（3-8-core-runtime-capability.md）

`PlatformInfo` は実行中の OS / アーキテクチャ / 機能集合を提供し、`has_capability(RuntimeCapability::SIMD)` などで最適化可否を判定します。CI やクロスランタイムでは、コンパイル時ターゲットとの差異を `Diagnostic` に `data.cfg.mismatch = true` として記録します。

## 4. ファイルシステム抽象

`3-5-core-io-path.md` は `Path` API に加え、プラットフォーム固有の違いを吸収する文字列ユーティリティを提供します。

```reml
let unix_path = normalize_path("./data/../bin", PathStyle::Posix)?;
let is_abs = is_absolute_str("C:\\Windows" , PathStyle::Windows);
```

未サポートの操作は `IoErrorKind::UnsupportedPlatform` を返し、診断では `target.config.unsupported_value` と連携します。`PathStyle::Native` を使用すると `platform_info().os` に基づいて既定の区切り文字が選ばれます。

## 5. FFI と ABI 適応

`3-9-core-async-ffi-unsafe.md` では呼出規約を `resolve_calling_convention` が自動判定します。`LibraryMetadata` の `preferred_convention` が現環境でサポートされない場合、`FfiErrorKind::UnsupportedPlatform` と `target.config.unsupported_value` を発行して即時に失敗させます。

```reml
let foreign = link_foreign_library(lib_path, platform_info())?;
let conv = resolve_calling_convention(platform_info(), metadata)?;
let shim = with_abi_adaptation(symbol, conv)?;
```

この仕組みは Capability Registry の `platform` 登録と連動するため、`platform_info()` を差し替えることで FFI バックエンドも同時に切り替わります（3-8-core-runtime-capability.md）。

## 6. 診断とテレメトリ

* `target.profile.missing`, `target.abi.mismatch`, `target.config.mismatch`, `target.capability.unknown` など `DiagnosticDomain::Target` の診断を `reml build --emit-metadata target.json` で収集し、CI で `../toolingci-strategy.md` に従って集計します。
* `Diagnostic.extensions["target"]` に `requested` / `detected` / `capability` 等の情報が含まれるため、構造化ログとして保存し、再現手順を短縮します。
* `Core.Diagnostics` と連携する場合、`set_env` や FFI 呼び出し時に `AuditEvent` を発行し、監査証跡を保持します。ターゲット関連イベントは `AuditEnvelope.metadata["target"]` を添付することで `reml toolchain verify` と整合します。

## 7. ターゲット Capability リファレンス

| Capability 名 | キー | 主な効果 |
| --- | --- | --- |
| Unicode NFC 正規化 | `unicode.nfc` | `Core.Text.normalize` で NFC 処理が利用可能 |
| 拡張書記素クラスタ | `unicode.grapheme` | `Core.Text.grapheme_iter` が完全サポート |
| ファイルシステム（大文字小文字区別） | `fs.case_sensitive` | POSIX 互換の挙動。`Path` 比較時に小文字化不要 |
| ファイルシステム（大文字小文字無視） | `fs.case_insensitive` | Windows/一部 Mac の挙動。パス衝突回避ロジックが必要 |
| Path UTF-8 | `fs.path_utf8` | UTF-8 パスを想定。未対応環境ではバイト列 API を使用 |
| Thread Local Storage | `thread.local` | `Core.Runtime` の TLS API が有効 |
| Job Control | `process.job_control` | `Core.System.Process` でジョブ制御が利用可能 |
| 呼出規約 (C / SysV / Win64 / Wasm) | `ffi.callconv.*` | `Core.Async.Ffi` が指定の ABI を提供 |

Capability 名は `capability_name(TargetCapability::…)` の戻り値と一致させます。`@cfg(capability = "...")` の判定にも同じ文字列を使用してください。

## 8. 推奨ワークフロー

1. **ターゲット初期化**: `infer_target_from_env` → `RunConfig.extensions["target"]`。
2. **条件付き宣言**: `@cfg` 属性でモジュールや API を切り替える。
3. **ファイル・Env 操作**: `Core.Path` と `Core.Env` を経由して依存を統一。
4. **FFI/ABI 適応**: `resolve_calling_convention` + `with_abi_adaptation` でプラットフォーム差異を吸収。
5. **診断の可視化**: `diagnostics=true` と `Diagnostic.extensions["cfg"]` / `extensions["target"]` で IDE/CI にフィードバック。
6. **ツールチェーン検証**: `reml toolchain verify` を定期的に実行し、`target_failures` をダッシュボードで監視。

## 9. チェックリスト

| 項目 | 内容 |
| --- | --- |
| CLI ターゲット | `reml build --target` の出力メタデータ (`RunArtifactMetadata`) がプロファイルと一致しているか |
| @cfg 分岐 | すべての分岐で効果タグが整合し、到達不能診断が出ていないか |
| パス操作 | `PathStyle::Native` と `normalize_path` を利用し、Capability (`fs.case_*`) に応じて分岐しているか |
| FFI | `LibraryMetadata` に呼出規約と必要 Capability (`ffi.callconv.*`) を記述したか |
| 診断 | CI で `target.config.*` / `target.capability.*` が検出された際の運用手順を整備したか |
| ツールチェーン | `reml toolchain list` で `runtime_revision` が最新か、不要なハッシュが残っていないか |

## 10. システムプログラミングモジュールとターゲット差異

| モジュール | 主な `@cfg` キー | プラットフォーム注意点 |
| --- | --- | --- |
| System Capability プラグイン (5-1) | `target_os`, `target_arch` | `PlatformSyscalls` を `supports` で確認し、未実装 OS では `raw_syscall` へのフォールバックと監査ログを用意する |
| Process Capability プラグイン (5-2) | `target_family`, `feature = "job_control"` | Windows のハンドル vs POSIX PID の差異を抽象化し、終了コードの意味付けを明示する |
| Memory Capability プラグイン (5-3) | `feature = "shared_memory"`, `target_os` | `mmap`/`MapViewOfFile` のサポート状況、`Fixed` マッピングをポリシーで禁止する |
| Signal Capability プラグイン (5-4) | `target_family`, `feature = "sigqueue"` | Windows では擬似シグナル (`CTRL_C_EVENT` など) を `Custom(i32)` へマップする |
| Hardware Capability プラグイン (5-5) | `target_arch`, `feature = "rdtsc"` | 権限不足時は `HardwareErrorKind::PermissionDenied` を返し、監査で権限不足を通知 |
| RealTime Capability プラグイン (5-6) | `feature = "realtime"`, `target_os` | `SCHED_DEADLINE` や `mlock` が非対応の場合は `RealTimeErrorKind::Unsupported` を返却 |

これらモジュールを利用する際は `CapabilitySecurity.effect_scope` と `SecurityPolicy` をターゲットごとに調整し、`../runtimesystem-programming-primer.md` で紹介する監査テンプレートと併用することを推奨する。

---

ポータビリティ対応は段階的な取り組みが推奨されます。Phase 1 の項目を満たした後は、`../toolingci-strategy.md`（未作成）でマルチターゲットテスト基盤を整備し、Phase 2 以降の TODO を進めてください。
