# 5.7 Core.Parse.Plugin — DSL 拡張プラグイン契約

> 位置付け: 公式プラグイン（オプション）。`Core.Parse` の拡張点を opt-in で公開し、DSL をプロジェクト単位で拡張するための契約を定義する。標準ライブラリ（Chapter 3）と同等の互換性・監査要件を持ちつつ、外部提供物（プラグイン/バンドル）が Capability Registry（[3-8](3-8-core-runtime-capability.md)）へ安全に接続できることを目的とする。
>
> ドラフト再整理メモ: `Core.Dsl` 系モジュール（[3.16](3-16-core-dsl-paradigm-kits.md)）との接続方針を再整理中。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（再検討中／Core.Parse 拡張） |
| プラグインID | `core.parse.plugin` |
| 効果タグ | `effect {parser}`, `effect {runtime}`, `effect {audit}`, `effect {security}` |
| 依存モジュール | `Core.Parse`（[2-1](2-1-parser-type.md)）、`Core.Diagnostics`（[3-6](3-6-core-diagnostics-audit.md)）、`Core.Runtime.Capability`（[3-8](3-8-core-runtime-capability.md)） |
| 相互参照 | `../guides/dsl/DSL-plugin.md`（運用ガイド）、`../notes/dsl/dsl-plugin-roadmap.md`（提供計画）、`2-2-core-combinator.md`（Capability 要求パターン） |

## 0.5 改訂案（標準ライブラリとの接続）

- **DSL キットとの接続**: `Core.Dsl` 系（[3.16](3-16-core-dsl-paradigm-kits.md)）から利用する Capability を整理し、プラグイン導入時の互換性チェックを明文化する。
- **運用ガイドの更新**: `../guides/dsl/DSL-plugin.md` に標準ライブラリとの境界（内蔵 DSL と外部 DSL の線引き）を追記する。

## 1. プラグインメタデータとマニフェスト

### 1.1 ID・バージョン・互換性

```reml
pub type PluginId = Str       // 例: "Reml.Web.Templating"
pub type PluginSemVer = SemVer

pub struct PluginMetadata {
  pub id: PluginId,
  pub version: PluginSemVer,
  pub checksum: Option<Digest>,
  pub description: Option<Str>,
  pub homepage: Option<Url>,
  pub license: Option<Str>,
  pub required_core: SemVerRange,         // Reml コア互換範囲
  pub required_cli: Option<SemVerRange>,  // `reml-plugin` CLI 互換範囲
}
```

- `checksum` はマニフェスト内で提供される場合に限り、CLI/Runtime の検証に利用される。省略時は CLI が取得元のハッシュを計算し、`PluginSignature` に含める（§4）。
- `required_core` は Core.Parse/Plugin API の互換範囲を示す。`register_plugin` は呼び出し時に現在のコアバージョンと突き合わせ、範囲外であれば `PluginError::IncompatibleCore` を返す。
- `required_cli` は CLI からの操作（バンドル管理）に必要な最小バージョンであり、ランタイムのみの利用では必須ではない。

### 1.2 Capability 宣言

```reml
pub struct ParserPluginCapability {
  pub name: CapabilityId,              // 例: "parser.atomic"
  pub version: PluginSemVer,
  pub stage: StageRequirement,         // {Experimental | Beta | Stable}
  pub effect_scope: Set<EffectTag>,
  pub traits: Set<Str>,                // 追加メタデータ（"cut", "telemetry" 等）
  pub since: Option<PluginSemVer>,
  pub deprecated: Option<PluginSemVer>,
}
```

- `stage` は Capability Registry で検証される下限値を示す。`StageRequirement::AtLeast(Beta)` のように指定し、`verify_capability_stage`（[3-8 §1.2](3-8-core-runtime-capability.md#capability-stage-contract)）と連携する。
- `effect_scope` は Capability が追加する効果タグ集合であり、`with_capabilities` 契約と `effects.contract.stage_mismatch` 診断（[3-6 §2.4.1](3-6-core-diagnostics-audit.md#stage-diagnostics)）の根拠となる。
- `traits` は Capability 固有の性質（例: cut 処理、ハイライト用 semantic tokens）を列挙する。DSL ガイドや IDE 連携での機能選択に利用する。

### 1.3 依存とバンドル構造

```reml
pub struct PluginDependency {
  pub id: PluginId,
  pub version: SemVerRange,
  pub capabilities: Set<CapabilityId>,
}

pub struct PluginBundleManifest {
  pub bundle_id: Str,
  pub bundle_version: PluginSemVer,
  pub plugins: List<ParserPlugin>,
  pub signature: Option<PluginSignature>,
}
```

- `PluginDependency.capabilities` で依存先プラグインから最低限必要な Capability を指定する。`register_plugin` は依存する Capability が Stage 条件を満たすか検証する。
- `bundle_id` / `bundle_version` は CLI がキャッシュ・更新判定に利用する識別情報であり、`plugins` 内の各プラグインは `metadata.id` で一意に識別される。
- バンドル（`register_bundle`）は複数プラグインをまとめて配布する単位であり、マニフェスト内の `signature` を利用して署名検証を行う（§4）。

#### Bundle JSON 形式（CLI 連携用）

CLI が読み込むバンドルファイルは JSON とし、以下の形式を基準とする。

```json
{
  "bundle_id": "bundle.demo",
  "bundle_version": "0.1.0",
  "plugins": [
    { "manifest_path": "plugins/demo/reml.toml", "module_path": "plugins/demo/plugin.wasm" },
    { "manifest_path": "plugins/extra/reml.toml" }
  ],
  "signature": {
    "algorithm": "ed25519",
    "certificate": "base64-cert",
    "issued_to": "bundle.demo",
    "valid_until": "2027-01-01T00:00:00Z",
    "bundle_hash": "sha256:<hex>"
  }
}
```

- `plugins[*].manifest_path` はバンドル JSON からの相対パスとして解釈する。
- `plugins[*].module_path` は WASM プラグインの PoC 用に利用する任意項目で、バンドル JSON からの相対パスとして解釈する（Phase 5 以降の本格仕様で再整理）。
- `bundle_hash` は `bundle_id` / `bundle_version` と各 `manifest_path` の内容を連結した入力から算出する。
- `signature` が無い場合は `VerificationPolicy::Permissive` では警告のみ、`Strict` では失敗とする。

## 2. 登録 API とランタイム契約

### 2.1 `ParserPlugin` 構造

```reml
pub struct ParserPlugin {
  pub metadata: PluginMetadata,
  pub capabilities: List<ParserPluginCapability>,
  pub dependencies: List<PluginDependency>,
  pub register: fn(PluginRegistrar) -> Result<(), PluginError>, // effect {parser, runtime}
}

pub struct PluginRegistrar {
  pub register_capability: fn(ParserPluginCapability) -> Result<(), PluginError>,
  pub register_parser: fn(Str, Parser) -> Result<(), PluginError>,
  pub register_schema: fn(Str, Schema) -> Result<(), PluginError>,
  pub register_diagnostic: fn(DiagnosticDescriptor) -> Result<(), PluginError>,
  pub with_audit: fn(fn(AuditContext) -> Result<(), PluginError>) -> Result<(), PluginError>,
}
```

- `register` クロージャはプラグイン読み込み時に一度だけ呼び出され、提供するパーサ／スキーマ／診断を `PluginRegistrar` 経由で登録する。
- `with_audit` は監査コンテキストを注入し、`audit.log("plugin.install", …)` のような記録が保証される。監査イベント名は `plugin.install`, `plugin.register_capability`, `plugin.verify_signature` を推奨する。

### 2.2 ランタイム API

```reml
fn register_plugin(plugin: ParserPlugin, registry: CapabilityRegistry) -> Result<CapabilitySet, PluginError>
fn register_bundle(bundle: PluginBundleManifest, registry: CapabilityRegistry) -> Result<List<CapabilitySet>, PluginError>
fn with_capabilities(required: Set<CapabilityId>, parser: Parser) -> Result<Parser, PluginError>
```

- `register_plugin` は以下の順序で検証を行う：
  1. `required_core` / `required_cli` の互換性チェック。
  2. 依存 Capability の存在確認と Stage 条件検証（`verify_capability_stage`）。
  3. `register` クロージャ実行と `PluginRegistrar` 呼び出し。
  4. `CapabilityRegistry.plugins.register_plugin` で Capability を公開（登録インターフェースは [3-8 §6](3-8-core-runtime-capability.md#プラグイン-capability) を参照）。
  5. 監査ログ発行（`plugin.install`）。
- 戻り値の `CapabilitySet` は [3-8 §1.2](3-8-core-runtime-capability.md) で定義された Capability ID の集合であり、`with_capabilities` や CLI ステータス表示で再利用する。
- `register_bundle` は複数プラグインを順序依存で登録する。途中でエラーが発生した場合はロールバックし、成功分を取り消した上で `PluginError::BundleInstallFailed` を返す。
- `with_capabilities` は要求集合を `CapabilityRegistry::verify_conductor_contract`（[3-8 §2.3](3-8-core-runtime-capability.md#capability-contract)）へ委譲し、不足時は `PluginError::MissingCapability` を返す。

CLI 連携例:

```bash
reml plugin install --bundle plugins/bundle.json --policy strict
reml plugin install --bundle plugins/bundle.json --policy permissive
reml plugin install --bundle plugins/bundle.json --policy strict --output json
```

## 3. 監査・セキュリティ統合

### 3.1 署名・ステージ検証の連携

```reml
pub struct PluginSignature {
  pub algorithm: SignatureAlgorithm,    // "ed25519" など
  pub certificate: Option<Base64>,
  pub issued_to: Option<Str>,
  pub valid_until: Option<Timestamp>,
}

fn verify_plugin_signature(sig: PluginSignature, policy: VerificationPolicy) -> Result<(), PluginError>
```

- 署名検証は CLI/Runtime の双方で `VerificationPolicy`（`Strict` / `Permissive`）に従う。`Strict` は証明書の有効期限・失効情報を必須とし、`Permissive` は警告のみで利用を許可する。
- 署名検証結果は `AuditContext` に `plugin.signature` イベントとして記録する。`valid_until` が閾値未満の場合は `PluginWarning::ExpiringSignature` を生成し、`3-6` の診断ポリシーへ渡す。
- `CapabilityRegistry` は検証済み署名のハッシュ値をキャッシュし、再登録時に差分を比較する。ハッシュ不一致時は `PluginError::SignatureMismatch` を返し、再検証を強制する。

### 3.2 セキュリティポリシー

- プラグインは `SecurityCapability.apply_plugin_policy`（[3-8 §4](3-8-core-runtime-capability.md#security-capability)）を通じて追加の制約を適用できる。`policy_digest` を監査ログに埋め込み、ポリシー変更の追跡性を確保する。
- 依存関係に `effect {unsafe}` を含む Capability がある場合、`PluginRegistrar.register_capability` は `EffectScope::contains("unsafe")` を確認し、未定義の効果タグ登録を禁止する。
- CLI はバンドル配布時に `policy strict` を既定値とし、`--policy permissive` の場合は `PluginWarning::PolicyDowngrade` を発行する。

## 4. エラーモデルと診断

```reml
pub enum PluginError =
  | MissingCapability { id: CapabilityId, required_stage: StageRequirement }
  | StageViolation { id: CapabilityId, required_stage: StageRequirement, actual_stage: CapabilityStage }
  | IncompatibleCore { required: SemVerRange, actual: SemVer }
  | BundleInstallFailed { failed_at: PluginId, cause: Box<PluginError> }
  | SignatureMismatch { plugin: PluginId }
  | VerificationFailed { policy: VerificationPolicy, reason: Str }
  | RegistrarError { operation: Str, cause: Diagnostic }
  | IO { path: Path, cause: IOError }
```

- `StageViolation` は `Diag.EffectDiagnostic.stage_violation` を生成し、`../notes/dsl/dsl-plugin-roadmap.md` で定義された段階的採用シナリオのガードとして機能する。
- `RegistrarError` は `register_capability` などの呼び出しから得られた診断をカプセル化する。プラグイン開発者は `Diagnostic::with_plugin_context(plugin_id)` を用いて発生箇所を明示すること。
- 署名関連の失敗は `VerificationFailed` として統一し、CLI は `plugin.signature.failure` イベントをログに残す。

```reml
pub enum PluginWarning =
  | DeprecatedCapability { id: CapabilityId, deprecated_since: PluginSemVer }
  | ExpiringSignature { plugin: PluginId, valid_until: Timestamp }
  | PolicyDowngrade { plugin: PluginId, policy: VerificationPolicy }
```

- 警告は `PluginInstallReport`（CLI が表示する結果構造）に収集され、JSON 出力時は `warnings: [...]` に列挙される。ランタイム側では `Diagnostics::emit_warning` を通じて IDE へ提示する。

## 5. CLI プロトコル（付録）

1. `reml-plugin install <bundle>` を実行すると、CLI は以下を順に実行する：
   1. バンドルマニフェストを取得し、`checksum` を検証する。
   2. `PluginSignature` を `verify_plugin_signature` に渡し、政策に応じた検証を行う。
   3. 依存プラグインを解決し、未解決の場合は `PluginError::MissingCapability` を報告する。
   4. バンドル内の各プラグインを `register_bundle` に渡し、成功時に `audit.log("plugin.install", …)` を記録する。
2. `reml-plugin verify <bundle>` は署名/ハッシュ検証まで実行し、登録は行わない。出力は `bundle_id`/`bundle_version`/`signature_status`/`bundle_hash`/`manifest_paths` を最小セットとする。
3. `reml-plugin status` は登録済みプラグインを列挙し、Capability Stage・署名有効期限・最終検証時刻を表示する。結果は `status.json` としてエクスポートでき、CI で `StageRequirement` の逸脱を検出する。
4. `reml-plugin revoke <id>` は対象プラグインを無効化し、`CapabilityRegistry.plugins.revoke_plugin` を起動する。無効化イベントは `plugin.revoke` として監査ログに残る。

ガイドライン・ベストプラクティス（テンプレート作成、CI 連携手順、IDE 統合など）は引き続き [guides/DSL-plugin.md](../guides/dsl/DSL-plugin.md) に掲載し、本章では API 契約と互換条件のみを定義する。
