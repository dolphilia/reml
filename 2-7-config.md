# 2.7 設定スキーマ API（Core.Config）

> 目的：設定 DSL を Reml コアの外で安全に構築・検証・差分適用できるようにする。Reml コアには `schema` キーワードが存在しないため、ここで定義する `Core.Config.schema` API が公式なエントリポイントになる。

## A. スキーマビルダ

```reml
let appSchema = Config.schema("AppConfig", |s| {
  s.field("env", Enum<Env>, default=Env::Dev)
   .field("database", Schema(DbConfig))
   .when(|cfg| cfg.env == Env::Prod, |p| p.set("logging.level", "info"))
})
```

Reml 本体に `schema` キーワードはありません。上記 API が言語レベルの糖衣に代わる標準的な記述方法です。必要であればマクロやプラグインがこの API を呼び出す形で従来風の構文を提供できます。

- `schema(name, builder)` でスキーマを組み立てる。
- ビルダチェーンで以下の関数を提供：
  - `field(name, Type, default=?)`
  - `optional(name, Type)`
  - `compute(name, (Config) -> Value)`
  - `requires((Config) -> Bool, message)`
  - `when((Config) -> Bool, Patch)` – 条件付きパッチ。
- 第3引数 `options` に `ConfigSchemaOptions` を渡すと、`data_source` 等のメタ情報を付与できる（§G）。
- `extends(baseSchema)` を呼び出すことで親スキーマを継承し、フィールドの上書き・追加が行える。

### A.1 マクロによる糖衣

`Core.Config.Dsl` モジュールは `schema` キーワード風のマクロを提供します。内部では本節のビルダ API を呼び出すため、生成される定義は全て `Core.Config.schema` と互換です。

```reml
use Core.Config.Dsl.schema;

schema AppConfig do
  field env: Enum<Env> := Env::Dev
  field database: Schema(DbConfig)
  when cfg.env == Env::Prod do
    set "logging.level" = "info"
  end
end
```

マクロの導入は任意であり、プロジェクトポリシーに応じて DSL を採用するかどうかを制御できます。

## B. 差分検証

```reml
let diff = Config.compare(oldConfig, newConfig)
match diff with
| Ok(same)      -> ...
| Err(changes)  -> audit.log("config.diff", changes)
```

- `compare` は `Result<(), List<Change>>` を返す。
- `Change` 構造体は 2-5 節で定義した `change_set` と互換。
- `apply_diff(config, changes)` で差分を適用。`effect {config, audit}` を要求。
- `plan(old, new)` は `SchemaDiff` を生成し、マイグレーション DSL と連携。

```reml
type Change = {
  path: List<Str>,
  kind: "Added" | "Removed" | "Modified",
  before: Option<Any>,
  after: Option<Any>,
  rationale: Option<Str>
}

type SchemaDiff<T> = {
  schema: Schema<T>,
  changes: List<Change>,
  breaking: Bool,
  version: SemVer
}
```

## C. 条件付き設定・テンプレート

- `Config.apply_when(config, predicate, patch)` で条件付き適用。
- テンプレート関数 `Config.template("prod")` を定義し、複数プロファイル間で差分を共有。
- `merge(base, overrides, precedence = Precedence::LaterWins)` でテンプレート同士のマージ戦略を制御。
- `Config.render(template, env)` はランタイムでテンプレートを具現化し、`audit_id` を発番する。

## D. CLI 連携

- `reml-config validate config.ks` で検証、`--format json` で構造化ログ出力。
- `reml-config diff old.ks new.ks` で `change_set` を表示。
- `reml-config render --template prod.ks --env staging` でテンプレートを適用。
- `reml-config approve <audit_id>` で差分承認フローを確定し、ホットリロード対象を決定する。
- CLI は `audit_id` を生成し、JSON 出力の `audit_id` と一致させる（2-5 の構造化ログと同期）。

## E. サンプルワークフロー

```reml
let base = Config.load("base.ks")?
let overrides = Config.load("env/prod.ks")?
let merged = Config.merge(base, overrides)

match Config.compare(base, merged) with
| Ok(()) -> Config.save("rendered/prod.ks", merged)
| Err(changes) -> {
    audit.log("config.diff", changes)
    Err(changes)
  }
```

## F. 型とエラー

```reml
type RenderedConfig = {
  source: String,
  values: Map<String, Any>,
  audit_id: Uuid
}

type ConfigError =
  | ValidationError(List<Diagnostic>)
  | RenderError(String)
  | IoError(String)
```

- `RenderedConfig` はテンプレート適用後の設定と `audit_id` を保持。
- `ConfigError` は検証失敗、レンダリング失敗、入出力失敗を表す。

### F-1. `Config.render`

```reml
fn render(template: Schema<T>, env: Map<String, Any>) -> Result<RenderedConfig, ConfigError>
```

- 成功時は `RenderedConfig` を返し、`audit_id` を自動発行。
- 失敗時は `ValidationError`（`Diagnostic` の一覧）などを返す。
- CLI `reml-config render` はエラー種別に応じて exit code を決定（下表参照）。

| ConfigError | exit code | 備考 |
| --- | --- | --- |
| `ValidationError` | 2 | スキーマ違反、`Diagnostic` を出力 |
| `RenderError` | 3 | テンプレート内の計算失敗 |
| `IoError` | 4 | ファイル読み書き・パーミッションエラー |

```

`Core.Config` の API と `reml-config` CLI は [設定 CLI ワークフロー](guides/config-cli.md) で定義したパイプラインに従い、`audit_id`・`change_set`・`exit code` を共有することで CI/CD・ホットリロード・監査報告における整合性を保証する。

---

## G. データ品質連携（ドラフト）

```reml
type ConfigSchemaOptions = {
  data_source: Option<DataSourceBinding>,
  default_profile: Option<ProfileId>
}

type DataSourceBinding = {
  schema: Schema<any>,
  profile: Option<ProfileId>,
  stats_provider: Option<StatsProviderId>
}

type DataProfileConfig = {
  profile: ProfileId,
  stats_provider: Option<StatsProviderId>,
  audit: Bool,
  emit_findings: Bool
}
```

* `Config.schema(name, builder, options)` で `data_source` を指定すると、該当スキーマを `Core.Data` の品質検証プロファイルへひも付けられる。
* `default_profile` は品質チェック拡張がプロファイル未指定で起動された際に利用するフォールバック値。
* `DataProfileConfig` は統計バックエンドや監査出力を宣言するための構造体であり、利用の有無は `reml-data` 系ツールのポリシーに依存する。
* CLI では `reml-config schema describe` で `data_source` を確認でき、`profile`/`stats_provider` の整合性を検証する。
