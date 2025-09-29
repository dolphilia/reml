# 3.10 Core Env & Platform Bridge

> 目的：環境変数・一時ディレクトリ・実行中プラットフォーム情報へのアクセスを標準化し、`@cfg` と `RunConfig.extensions["target"]` の整合性を保つ。環境依存の差異を安全に露出し、ポータビリティ指針（[ポータビリティガイド](guides/portability.md)）を仕様へ落とし込む。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（Phase 1 優先タスク） |
| 効果タグ | `effect {io}`, `effect {runtime}`, `effect {security}` |
| 依存モジュール | `Core.Prelude`, `Core.IO`, `Core.Path`, `Core.Diagnostics`, `Core.Runtime` |
| 相互参照 | [3-5 Core IO & Path](3-5-core-io-path.md), [3-8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md), [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), Guides: [ポータビリティガイド](guides/portability.md) |

## 1. 環境変数アクセス

```reml
fn get_env(key: Str) -> Result<Option<Str>, EnvError>              // `effect {io}`
fn set_env(key: Str, value: Str) -> Result<(), EnvError>           // `effect {io, security}`
fn remove_env(key: Str) -> Result<(), EnvError>                    // `effect {io, security}`
```

* `get_env` は存在しないキーで `Ok(None)` を返し、値は UTF-8 を期待（無効バイト列は `EnvErrorKind::InvalidEncoding` で報告）。
* `set_env` / `remove_env` はプロセス環境の変更権限がない場合に `EnvErrorKind::PermissionDenied` を返す。CI や限定的環境ではノーオペレーションで成功させず、明示的にエラーとする。 
* 変更系 API は監査トレース向けに `Core.Diagnostics` の `AuditEvent::EnvMutation` を発行する。`RunConfig.extensions["audit"].capture_env=true`（任意）で詳細ログを有効化。

```reml
pub type EnvError = {
  kind: EnvErrorKind,
  message: Str,
  key: Option<Str>,
  context: Option<EnvContext>,
}

pub enum EnvErrorKind = NotFound | PermissionDenied | InvalidEncoding | UnsupportedPlatform | IoFailure

pub type EnvContext = {
  operation: Str,        // "get", "set", "remove"
  platform: PlatformInfo,
}
```

## 2. 一時ディレクトリとパス補助

```reml
fn get_temp_dir() -> Result<Path, EnvError>                        // `effect {io}`
fn cache_dir(app: Str) -> Result<Path, EnvError>                   // `effect {io}`
fn config_dir(app: Str) -> Result<Path, EnvError>                  // `effect {io}`
```

* `get_temp_dir` は O/S の既定一時パスを返し、存在しない場合は `EnvErrorKind::UnsupportedPlatform`。
* `cache_dir` / `config_dir` は XDG / AppData / Library 以下など**プラットフォームごとの標準位置**を解決し、`Core.Path.normalize_path`（[3-5](3-5-core-io-path.md)）で正規化した `Path` を返す。
* 生成されるディレクトリは呼び出し側が作成・権限調整を行う。自動作成は行わない（ライフサイクルはアプリケーション責務）。

## 3. プラットフォーム情報の取得

```reml
fn platform_info() -> PlatformInfo                               // `effect {runtime}`
fn has_capability(cap: Capability) -> Bool                       // `effect {runtime}`
```

* `PlatformInfo` と `Capability` は [3-8](3-8-core-runtime-capability.md#platform-info) にて定義される。`Core.Env` は単なるフェデレーションモジュールであり、`Core.Runtime` の Capability Registry からデータを引き出して公開する。
* `platform_info()` の結果は `RunConfig.extensions["target"]` と同期する責務がある。CLI はコンパイル時ターゲット、ランタイムは実行中ターゲットを提供するが、差異がある場合は `Diagnostic.domain = Target`（3-6 §7 新設）で `data.cfg.mismatch = true` を付けて警告を促す。

## 4. `@cfg` 連携ガイドライン

* `@cfg` の評価は言語仕様側（[1-1](1-1-syntax.md#条件付きコンパイル属性-cfg)）で行われるが、`Core.Env` は `RunConfigTarget` と `TargetProfile` を構築する補助を提供する。

```reml
pub type TargetProfile = {
  id: Str,
  triple: Str,
  os: Str,
  family: Str,
  arch: Str,
  abi: Option<Str>,
  vendor: Option<Str>,
  env: Option<Str>,
  stdlib_version: Option<SemVer>,
  runtime_revision: Option<Str>,
  features: Set<Str>,
  capabilities: Set<Str>,
  diagnostics: Bool,
  extra: Map<Str, Str>
}

pub type RunConfigTarget = {
  os: Str,
  family: Str,
  arch: Str,
  abi: Option<Str>,
  vendor: Option<Str>,
  env: Option<Str>,
  profile_id: Option<Str>,
  triple: Option<Str>,
  features: Set<Str>,
  capabilities: Set<Str>,
  stdlib_version: Option<SemVer>,
  runtime_revision: Option<Str>,
  diagnostics: Bool,
  extra: Map<Str, Str>
}

fn infer_target_from_env() -> Result<TargetProfile, EnvError>    // `effect {io}`
fn resolve_run_config_target(profile: TargetProfile) -> RunConfigTarget
fn merge_runtime_target(cfg: RunConfigTarget, runtime: PlatformInfo) -> RunConfigTarget
```

* `TargetProfile` は CLI やレジストリ（4-2）で配布されるターゲット宣言を表し、`infer_target_from_env` は環境変数・プロセス情報から `TargetProfile` を起こす。環境変数名は以下の通り（存在しない場合は `None`）。

  | 環境変数 | 例 | 対応フィールド |
  | --- | --- | --- |
  | `REML_TARGET_PROFILE` | `desktop-x86_64` | `id` |
  | `REML_TARGET_TRIPLE`  | `x86_64-unknown-linux-gnu` | `triple`（`os`/`arch`/`vendor`/`abi` を分解） |
  | `REML_TARGET_OS` / `REML_TARGET_FAMILY` / `REML_TARGET_ARCH` | `linux` / `unix` / `x86_64` | `os`/`family`/`arch`（`TRIPLE` が無い場合のフォールバック） |
  | `REML_TARGET_ENV` | `msvc` | `env` |
  | `REML_TARGET_VENDOR` | `apple` | `vendor` |
  | `REML_TARGET_ABI` | `gnu` | `abi` |
  | `REML_STD_VERSION` | `1.0.0` | `stdlib_version` |
  | `REML_RUNTIME_REVISION` | `rc-2024-09` | `runtime_revision` |
  | `REML_TARGET_FEATURES` | `simd,packrat` | `features`（カンマ区切り） |
  | `REML_TARGET_CAPABILITIES` | `unicode.nfc,fs.case_sensitive` | `capabilities` |
  | `REML_TARGET_DIAGNOSTICS` | `1` | `diagnostics`（`true`/`false` も可） |
  | `REML_TARGET_EXTRA_*` | `REML_TARGET_EXTRA_io.blocking=strict` | `extra["io.blocking"]` |

* すべての文字列は UTF-8 とし、無効なフォーマットや未知キーは `EnvErrorKind::InvalidEncoding` / `EnvErrorKind::UnsupportedPlatform` で報告する。`features` と `capabilities` は余白を除去した上で小文字に正規化する。
* `resolve_run_config_target` は `TargetProfile` を `RunConfigTarget` に昇格し、`profile_id = Some(profile.id)` と `triple = Some(profile.triple)` を設定する。`diagnostics` は `profile.diagnostics` を引き継ぐ。
* `merge_runtime_target` は `PlatformInfo` の情報で `RunConfigTarget` を補正し、`os`/`family`/`arch` が不一致の場合は `DiagnosticDomain::Target` で `target.config.mismatch` を生成する（3-6 §7）。
* CI やクロスコンパイル環境では `infer_target_from_env()?` の結果をマニフェスト/CLI 由来の `TargetProfile` と突き合わせ、差異がある場合は `target.profile.missing` または `target.abi.mismatch` を返す。既定値を補うだけの場合は `Ok(profile)` を返し、最終的な `RunConfigTarget` は `RunConfig.extensions["target"]` へ挿入される。

## 5. 診断と監査

* すべての `EnvError` は `Diagnostic.domain = Some(Config)`、`message_key = "env.access.failed"` を既定とし、`extensions["cfg"].evaluated` に `platform_info()` の抜粋を添付する。
* `TargetProfile` / `RunConfigTarget` の構築で発生したエラーは `DiagnosticDomain::Target`（3-6 §7）で報告し、`target.profile.missing` / `target.abi.mismatch` / `target.config.mismatch` / `target.capability.unknown` を使用する。追加情報として `extensions["target"] = { profile_id, triple, compared_with }` を添付し、CI でのトリアージを容易にする。
* `set_env`/`remove_env` は成功時にも `AuditEvent::EnvMutation` を記録し、`Core.Diagnostics` のポリシーによりマスク・匿名化を適用する。保持期間は `Core.Diagnostics` の監査ポリシー（3-6）に準拠。

## 6. 将来拡張メモ

* Phase 2 で `watch_env`（環境変数変更監視）と `ScopedEnv`（with-style スコープ設定）を検討。これらは `effect {io, io.async}` を要求するため、非同期実行基盤が整った段階でドラフトを追加する。
* プロセス以外の設定ソース（例：クラウドシークレット、HashiCorp Vault 等）との統合はプラグイン (`Core.Env.SecretProviders`) として別文書で扱う。
