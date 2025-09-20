# DSL プラグイン & Capability ガイド（Draft）

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
  version = SemVer(1, 2, 0),
  capabilities = [
    { name = "template", version = SemVer(1,0,0), traits = {"render"}, since = Some(SemVer(1,0,0)), deprecated = None }
  ],
  register = |reg| {
    reg.register_schema("TemplateConfig", templateSchema);
    reg.register_parser("render", || renderParser);
  }
}

register_plugin(templating)?
```

## 2. Capability の使い方

- 利用側は `with_capabilities({"template"}, parser)` のように要求 capability を指定。
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

## 5. TODO / 制限事項

- `PluginRegistrar` が提供するその他の拡張ポイント（診断フック等）は今後追加予定。
- プラグイン配布形式（バンドル、リポジトリ）の詳細仕様は標準化検討中。

## 6. 依存解決と配布

- プラグインは `dependencies: List<PluginDependency>` を宣言し、`register_plugin` 時に依存が満たされているかチェック。
- 複数プラグインをまとめた `PluginBundle` を用意し、`register_bundle` で一括登録できる。
- CLI `reml-plugin install <bundle>` を利用して、リポジトリからバンドルを取得→検証→登録するワークフローを想定。
- 推奨ディレクトリ構成：`reml-plugins/<plugin-name>/<version>/plugin.ks` とメタデータ (`plugin.toml`) を配置。

```bash
reml-plugin install reml-web-bundle --source https://example.com/plugins
```

> 依存解決や配布形式の詳細標準化は進行中（2-1 節の案を参照）。
