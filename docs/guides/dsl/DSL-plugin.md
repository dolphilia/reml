# DSL プラグイン & Capability ガイド

> 目的：`5-7-core-parse-plugin.md` で定義された `Core.Parse.Plugin` 契約を実務に適用する際の設計手順とベストプラクティスを整理する。仕様そのものは Chapter 5 を参照し、本ガイドは導入フロー・運用ポリシー・テンプレート構築のヒントを提供する。

## 0. 言語構文との関係

Reml コアはプラグイン用の `package` 宣言や `use plugin` 構文を持ちません。プラグインは CLI (`reml-plugin`) やビルドパイプラインで配布・有効化し、アプリケーションコードでは `Core.Parse.Plugin` 拡張が提供する `register_plugin` API か生成されたマニフェストを通じて読み込みます。API の契約・検証順序は [5-7 §2](../../spec/5-7-core-parse-plugin.md#2-登録-api-とランタイム契約) を参照してください。

プラグインのメタデータは `plugin.toml`（または同等のマニフェスト）に記述します。[5-7 §1.1](../../spec/5-7-core-parse-plugin.md#11-idバージョン互換性) の構造に準拠し、CI で差分検証できるよう Git 管理下に置いてください。例：

```toml
name = "Reml.Web.Templating"
version = "1.4.0"
checksum = "9b4d..."
capabilities = ["parser.atomic", "parser.syntax.highlight"]
[manifest.download]
url = "https://example.com/plugins/reml-web-1.4.0.tar.gz"
```

CLI はマニフェストを `register_bundle` に橋渡しし、コード側では従来通り `register_plugin` や `with_capabilities` を利用します。バンドル識別子や署名管理は [5-7 §1.3 / §3](../../spec/5-7-core-parse-plugin.md#13-依存とバンドル構造) を参照。

## 1. プラグインの構造


| 項目 | 説明 |
| --- | --- |
| `ParserPlugin.name` | プラグイン識別子。`SemVer` と組み合わせてバージョン管理を行う。 |
| `ParserPlugin.capabilities` | 提供する capability の一覧。`since` / `deprecated` を利用して互換性を管理。 |
| `PluginRegistrar` | `register_schema`, `register_parser`, `register_capability` を提供し、プラグインが DSL を公開する。 |

> 仕様の詳細は [5-7 §2.1](../../spec/5-7-core-parse-plugin.md#21-parserplugin-構造) を参照。ガイドでは設計レビュー時に確認すべきポイントと運用上の注意に焦点を当てる。

プラグインを Reml プロジェクトに導入する際は、`reml-plugin install` でマニフェストを検証→署名確認→`register_bundle` 呼び出し、という手順が基本となります。言語コード内での使用方法は従来と変わらず、`register_plugin` 経由で必要な capability を登録します。

```reml
let metadata = PluginMetadata {
  id = "Reml.Web.Templating",
  version = SemVer(1,4,0),
  checksum = None,
  description = Some("HTML テンプレート DSL"),
  homepage = Some(Url::parse("https://example.com")),
  license = Some("Apache-2.0"),
  required_core = SemVerRange::from("^1.4"),
  required_cli = Some(SemVerRange::from("^1.3")),
}

let cap_atomic = ParserPluginCapability {
  name = "parser.atomic",
  version = SemVer(1,4,0),
  stage = StageRequirement::AtLeast(Stable),
  effect_scope = Set::from(["parser", "audit"]),
  traits = Set::from(["cut"]),
  since = Some(SemVer(1,4,0)),
  deprecated = None,
}

let cap_trace = ParserPluginCapability {
  name = "parser.trace",
  version = SemVer(1,4,0),
  stage = StageRequirement::AtLeast(Beta),
  effect_scope = Set::from(["parser", "telemetry"]),
  traits = Set::from(["semantic-tokens"]),
  since = Some(SemVer(1,3,0)),
  deprecated = None,
}

let templating = ParserPlugin {
  metadata = metadata,
  capabilities = [cap_atomic.clone(), cap_trace.clone()],
  dependencies = [],
  register = |reg| {
    reg.register_capability(cap_atomic)?;
    reg.register_capability(cap_trace)?;
    reg.register_schema("TemplateConfig", templateSchema)?;
    reg.register_parser("render", || renderParser)?;
  }
}

register_plugin(templating, capability_registry)?
```

## 2. Capability の使い方

- 利用側は `with_capabilities({"parser.atomic", "parser.trace"}, parser)` のように要求 capability を指定する。要求集合は `StageRequirement` とともに `CapabilityRegistry::verify_conductor_contract` へ渡される（[5-7 §2.2](../../spec/5-7-core-parse-plugin.md#22-ランタイム-api)）。
- 依存するコンビネータは `2-2-core-combinator.md` の `Capability 要求パターン` に従い、必要 capability を定義。
- 不足している場合は `PluginError::MissingCapability` が返る。CI では `reml-plugin status --export status.json` を利用して不足 Capability を検出することを推奨。

## 3. バージョン互換性

| ケース | 対応 |
| --- | --- |
| 同名プラグインのバージョン差異 | 互換性があれば更新、非互換なら `PluginError::Conflict` |
| Deprecated capability | `PluginWarning::DeprecatedCapability` を発行し、ログや CLI に警告を出す |
| 破壊的変更 | `since` / `deprecated` を利用し、段階的に移行 |

> `PluginError` / `PluginWarning` の列挙体は [5-7 §4](../../spec/5-7-core-parse-plugin.md#4-エラーモデルと診断) を参照。ガイドでは移行計画とリリースノートの書き方に注力する。

## 4. 推奨運用フロー

1. プラグイン開発者は capability 一覧とバージョンポリシーを README に記載。
2. 利用者は `Scenario-requirements.md` を参照して必要 capability を特定。
3. CI/Pipeline で `register_plugin` → `with_capabilities` の成功可否を検証。
4. 週次で `reml-plugin status --refresh` を実行し、署名の有効期限と Stage 違反を点検（[5-7 §3](../../spec/5-7-core-parse-plugin.md#3-監査セキュリティ統合)）。

## 5. 依存解決と配布

- プラグインは `dependencies: List<PluginDependency>` を宣言し、`register_plugin` 時に依存が満たされているかチェック。
- 複数プラグインをまとめた `PluginBundle` を用意し、`register_bundle` で一括登録できる。
- CLI `reml-plugin install <bundle>` を利用して、リポジトリからバンドルを取得→検証→登録するワークフローを標準化する。
- 推奨ディレクトリ構成：`reml-plugins/<plugin-name>/<version>/plugin.ks` とメタデータ (`plugin.toml`) を配置。

> バンドルのメタデータ構造と署名検証手順は [5-7 §1.3 / §3.1](../../spec/5-7-core-parse-plugin.md#13-依存とバンドル構造) に準拠すること。

```bash
reml-plugin install reml-web-bundle --source https://example.com/plugins --policy strict
```

## 6. CLI プロトコルとフロー

1. `reml-plugin install <bundle>` を実行すると、CLI は以下の順序で処理する：
   1. バンドルメタデータ (`plugin.toml`) を取得し、`checksum` を検証。
   2. `PluginSignature` を `verify_plugin` API に渡し、公的鍵/証明書チェーンを検証。
   3. 依存プラグインを解決し、未解決の場合は `PluginError::MissingDependency` を表示。
   4. すべてのプラグインを `register_bundle` に渡し、成功時に `audit.log("plugin.install", {...})` を記録。
2. `reml-plugin status` はインストール済みバンドルの一覧と署名有効期限を表示。
3. `reml-plugin revoke <name>` は該当バンドルを無効化し、`PluginWarning::ExpiringSignature` が出た場合の自動更新フローを支援する。

> 公式仕様上の手順は [5-7 §5](../../spec/5-7-core-parse-plugin.md#5-cli-プロトコル付録) に準拠している。CLI 実装の派生や拡張を行う場合は同節の監査イベント命名規則を維持すること。

## 7. 署名と検証

- 署名ファイルは `bundle.sig`（Ed25519 または RSA-PSS）を想定し、`PluginSignature` にメタデータを格納する。
- `VerificationPolicy::Strict` は証明書の有効期限・失効リスト (CRL/OCSP) を必須チェックとし、`--policy permissive` では警告 (`PluginWarning::ExpiringSignature`) のみ発行。
- 署名付きバンドルの JSON 例：

```json
{
  "name": "reml-web-bundle",
  "version": "1.0.0",
  "checksum": "9b4d...",
  "signature": {
    "algorithm": "ed25519",
    "certificate": "BASE64...",
    "issued_to": "Reml Web Team",
    "valid_until": "2030-01-01T00:00:00Z"
  }
}
```

> 署名構造と `PluginWarning` の取り扱いは [5-7 §3.1 / §4](../../spec/5-7-core-parse-plugin.md#3-監査セキュリティ統合) を参照。運用面では証明書更新時のローテーション手順（CI 秘密情報の保管場所、失効リスト配信）をリリースノートに残すこと。

## 8. 既知の制限

- `PluginRegistrar` の診断フック・CodeAction 連携は `VerificationPolicy` に従い段階的に解禁される（今後の拡張候補）。
- 署名の検証結果は CLI にキャッシュされるが、再検証のトリガーは `reml-plugin status --refresh` で手動実行する。
