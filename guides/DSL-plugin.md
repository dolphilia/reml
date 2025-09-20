# DSL プラグイン & Capability ガイド（Draft）

> 目的：`ParserPlugin` / `CapabilitySet` を用いて DSL を拡張する際の設計手順とベストプラクティスを整理する。

## 1. プラグインの構造

| 項目 | 説明 |
| --- | --- |
| `ParserPlugin.name` | プラグイン識別子。`SemVer` と組み合わせてバージョン管理を行う。 |
| `ParserPlugin.capabilities` | 提供する capability の一覧。`since` / `deprecated` を利用して互換性を管理。 |
| `PluginRegistrar` | `register_schema`, `register_parser`, `register_capability` を提供し、プラグインが DSL を公開する。 |

```kestrel
let templating = ParserPlugin {
  name = "Kestrel.Web.Templating",
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

- `PluginRegistrar` が提供する API（schema、parser 以外の拡張ポイント）を追加整備予定。
- DSL パッケージの依存解決（プラグイン間依存関係）は未定義。今後整理。 
- プラグイン配布形式（バンドル、リポジトリ）に関する標準化は今後検討。

