# ポータビリティガイド

本ガイドは Reml プロジェクトで複数プラットフォームを対象とした開発を行う際の実務手順をまとめたものです。`0-2-project-purpose.md` が掲げる「実用性能」と「安全性」を満たしつつ、ターゲット差異を安全に扱うための仕様導線と実例を以下に整理します。

## 1. ビルドターゲットの宣言

### 1.1 `remlc` のターゲット指定

`remlc --target` で LLVM Triple を明示します（README.md 参照）。

```bash
# Windows 向けビルド
remlc --target x86_64-pc-windows-msvc src/main.reml

# Apple Silicon 向けビルド
remlc --target aarch64-apple-darwin src/main.reml
```

CI/CD では `REML_TARGET`・`REML_FEATURES` 等の環境変数を設定し、後述の `Core.Env.infer_target_from_env` で取得します。

### 1.2 `RunConfig.extensions["target"]`

`2-6-execution-strategy.md` に定義された構造体を利用し、パーサー実行時にターゲット情報を供給します。

```reml
let cfg = RunConfig {
  packrat = platform_info().capabilities.contains(SIMD),
  extensions = {
    "target": {
      os: platform_info().os.to_string(),
      family: platform_info().family_tag(),
      arch: platform_info().arch.to_string(),
      env: platform_info().variant,
      features: project_features(active_profile),
      extra: { "gpu": "cuda" },
      diagnostics: true,
    }
  }
}
```

`diagnostics=true` を設定すると `@cfg` 評価ログが `Diagnostic.extensions["cfg"]` に出力され、誤設定を検出できます（2-5-error.md）。

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
| `feature` | ビルド切替 | `"gpu_acceleration"`, `"use_packrat"` |
| `extra.*` | プロジェクト拡張 | 例: `extra.gpu = "cuda"` |

## 3. 環境・プラットフォーム API

### 3.1 `Core.Env`（3-10-core-env.md）

* `get_env` / `set_env` / `remove_env` で環境変数アクセスを統一。
* `get_temp_dir` や `cache_dir` を利用し、XDG/AppData などプラットフォーム標準のパスを取得。
* `infer_target_from_env` は `RunConfigTarget` を返し、`RunConfig.extensions["target"]` へマージできます。

```reml
match infer_target_from_env()? {
  Ok(target) => cfg.extensions["target"].merge(target),
  Err(err) => diagnostics.emit(env_to_diagnostic(err)),
}
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

* `target.config.*` / `effects.cfg.*` を用いた診断は 2-5-error.md の B-9 表を参照し、IDE/LSP へ `Diagnostic.extensions["cfg"] = { keys, evaluated, active }` を送信します。
* `Core.Diagnostics` と連携する場合、`set_env` や FFI 呼び出し時に `AuditEvent` を発行し、監査証跡を保持します。

## 7. 推奨ワークフロー

1. **ターゲット初期化**: `infer_target_from_env` → `RunConfig.extensions["target"]`。
2. **条件付き宣言**: `@cfg` 属性でモジュールや API を切り替える。
3. **ファイル・Env 操作**: `Core.Path` と `Core.Env` を経由して依存を統一。
4. **FFI/ABI 適応**: `resolve_calling_convention` + `with_abi_adaptation` でプラットフォーム差異を吸収。
5. **診断の可視化**: `diagnostics=true` と `Diagnostic.extensions["cfg"]` で IDE/CI にフィードバック。

## 8. チェックリスト

| 項目 | 内容 |
| --- | --- |
| CLI ターゲット | `remlc --target` と `RunConfigTarget` が一致しているか |
| @cfg 分岐 | すべての分岐で効果タグが整合し、到達不能診断が出ていないか |
| パス操作 | `PathStyle::Native` と `normalize_path` を利用しているか |
| FFI | `LibraryMetadata` に呼出規約と必要 Capability を記述したか |
| 診断 | CI で `target.config.*` が検出された際の運用手順を整備したか |

---

ポータビリティ対応は段階的な取り組みが推奨されます。Phase 1 の項目を満たした後は、`guides/ci-strategy.md`（未作成）でマルチターゲットテスト基盤を整備し、Phase 2 以降の TODO を進めてください。
