# DSL プラグイン & Capability ガイド

> 目的：`ParserPlugin` / `CapabilitySet` を用いて DSL を拡張する際の設計手順とベストプラクティスを整理する。

## 1. プラグインの構造

| 項目 | 説明 |
| --- | --- |
| `ParserPlugin.name` | プラグイン識別子。`SemVer` と組み合わせてバージョン管理を行う。 |
| `ParserPlugin.capabilities` | 提供する capability の一覧。`since` / `deprecated` を利用して互換性を管理。 |
| `PluginRegistrar` | `register_schema`, `register_parser`, `register_capability` を提供し、プラグインが DSL を公開する。 |

```reml
let templating = ParserPlugin {
  name = "Reml.Web.Templating",
  version = SemVer(1, 4, 0),
  capabilities = [
    { name = "parser.atomic", version = SemVer(1,4,0), traits = {"cut"}, since = Some(SemVer(1,4,0)), deprecated = None },
    { name = "parser.trace", version = SemVer(1,4,0), traits = {"telemetry"}, since = Some(SemVer(1,4,0)), deprecated = None },
    { name = "parser.syntax.highlight", version = SemVer(1,4,0), traits = {"semantic-tokens"}, since = Some(SemVer(1,3,0)), deprecated = None }
  ],
  register = |reg| {
    reg.register_capability({"parser.atomic", "parser.trace", "parser.syntax.highlight"});
    reg.register_schema("TemplateConfig", templateSchema);
    reg.register_parser("render", || renderParser);
  }
}

register_plugin(templating)?
```

## 2. Capability の使い方

- 利用側は `with_capabilities({"parser.atomic", "parser.trace"}, parser)` のように要求 capability を指定。
- 依存するコンビネータは `2-2-core-combinator.md` の `Capability 要求パターン` に従い、必要 capability を定義。
- 不足している場合は `PluginError::MissingCapability` が返る。

## 3. バージョン互換性

| ケース | 対応 |
| --- | --- |
| 同名プラグインのバージョン差異 | 互換性があれば更新、非互換なら `PluginError::Conflict` |
| Deprecated capability | `PluginWarning::DeprecatedCapability` を発行し、ログや CLI に警告を出す |
| 破壊的変更 | `since` / `deprecated` を利用し、段階的に移行 |

## 4. 推奨運用フロー

1. プラグイン開発者は capability 一覧とバージョンポリシーを README に記載。
2. 利用者は `Scenario-requirements.md` を参照して必要 capability を特定。
3. CI/Pipeline で `register_plugin` → `with_capabilities` の成功可否を検証。

## 5. 依存解決と配布

- プラグインは `dependencies: List<PluginDependency>` を宣言し、`register_plugin` 時に依存が満たされているかチェック。
- 複数プラグインをまとめた `PluginBundle` を用意し、`register_bundle` で一括登録できる。
- CLI `kestrel-plugin install <bundle>` を利用して、リポジトリからバンドルを取得→検証→登録するワークフローを標準化する。
- 推奨ディレクトリ構成：`reml-plugins/<plugin-name>/<version>/plugin.ks` とメタデータ (`plugin.toml`) を配置。

```bash
kestrel-plugin install reml-web-bundle --source https://example.com/plugins --policy strict
```

## 6. CLI プロトコルとフロー

1. `kestrel-plugin install <bundle>` を実行すると、CLI は以下の順序で処理する：
   1. バンドルメタデータ (`plugin.toml`) を取得し、`checksum` を検証。
   2. `PluginSignature` を `verify_plugin` API に渡し、公的鍵/証明書チェーンを検証。
   3. 依存プラグインを解決し、未解決の場合は `PluginError::MissingDependency` を表示。
   4. すべてのプラグインを `register_bundle` に渡し、成功時に `audit.log("plugin.install", {...})` を記録。
2. `kestrel-plugin status` はインストール済みバンドルの一覧と署名有効期限を表示。
3. `kestrel-plugin revoke <name>` は該当バンドルを無効化し、`PluginWarning::ExpiringSignature` が出た場合の自動更新フローを支援する。

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

## 8. 既知の制限

- `PluginRegistrar` の診断フック・CodeAction 連携は `VerificationPolicy` に従い段階的に解禁される（今後の拡張候補）。
- 署名の検証結果は CLI にキャッシュされるが、再検証のトリガーは `kestrel-plugin status --refresh` で手動実行する。
