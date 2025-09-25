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
* `platform_info()` の結果は `RunConfig.extensions["target"]` と同期する責務がある。CLI はコンパイル時ターゲット、ランタイムは実行中ターゲットを提供するが、差異がある場合は `Diagnostic` に `data.cfg.mismatch = true` を付けて警告を促すこと。

## 4. `@cfg` 連携ガイドライン

* `@cfg` の評価は言語仕様側（[1-1](1-1-syntax.md#条件付きコンパイル属性-cfg)）で行われるが、`Core.Env` は `RunConfig.extensions["target"]` の初期化補助を提供する。

```reml
fn infer_target_from_env() -> Result<RunConfigTarget, EnvError>  // `effect {io}`

struct RunConfigTarget = {
  os: Str,
  family: Str,
  arch: Str,
  env: Option<Str>,
  features: Set<Str>,
  extra: Map<Str, Str>,
}
```

* `RunConfigTarget` は `os`, `family`, `arch`, `features` を保持する構造体で、`RunConfig.extensions["target"]` へマージできる。
* CI やクロスコンパイル環境では `REML_TARGET`, `REML_FEATURES` 等の環境変数から値を取得する。未設定のキーは `Ok(default)` で返し、型安全な整形は CLI 側に任せる。

## 5. 診断と監査

* すべての `EnvError` は `Diagnostic.domain = Some(Config)`、`message_key = "env.access.failed"` を既定とし、`extensions["cfg"].evaluated` に `platform_info()` の抜粋を添付する。
* `set_env`/`remove_env` は成功時にも `AuditEvent::EnvMutation` を記録し、`Core.Diagnostics` のポリシーによりマスク・匿名化を適用する。保持期間は `Core.Diagnostics` の監査ポリシー（3-6）に準拠。

## 6. 将来拡張メモ

* Phase 2 で `watch_env`（環境変数変更監視）と `ScopedEnv`（with-style スコープ設定）を検討。これらは `effect {io, async}` を要求するため、非同期実行基盤が整った段階でドラフトを追加する。
* プロセス以外の設定ソース（例：クラウドシークレット、HashiCorp Vault 等）との統合はプラグイン (`Core.Env.SecretProviders`) として別文書で扱う。
