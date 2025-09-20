# 2.7 設定スキーマ API（Nest.Config） - Draft

> 目的：`schema` 構文で宣言された設定をプログラムから検証・差分適用するための標準 API を提供する。

## A. スキーマビルダ

```kestrel
let appSchema = Config.schema("AppConfig", |s| {
  s.field("env", Enum<Env>, default=Env::Dev)
   .field("database", Schema(DbConfig))
   .when(|cfg| cfg.env == Env::Prod, |p| p.set("logging.level", "info"))
})
```

- `schema(name, builder)` でスキーマを組み立てる。
- ビルダチェーンで以下の関数を提供：
  - `field(name, Type, default=?)`
  - `optional(name, Type)`
  - `compute(name, (Config) -> Value)`
  - `requires((Config) -> Bool, message)`
  - `when((Config) -> Bool, Patch)` – 条件付きパッチ。
- `extends(baseSchema)` を呼び出すことで親スキーマを継承し、フィールドの上書き・追加が行える。

## B. 差分検証

```kestrel
let diff = Config.compare(oldConfig, newConfig)
match diff with
| Ok(same)      -> ...
| Err(changes)  -> audit.log("config.diff", changes)
```

- `compare` は `Result<(), List<Change>>` を返す。
- `Change` 構造体は 2-5 節で定義した `change_set` と互換。
- `apply_diff(config, changes)` で差分を適用。`effect {config, audit}` を要求。
- `plan(old, new)` は `SchemaDiff` を生成し、マイグレーション DSL と連携。

## C. 条件付き設定・テンプレート

- `Config.apply_when(config, predicate, patch)` で条件付き適用。
- テンプレート関数 `Config.template("prod")` を定義し、複数プロファイル間で差分を共有。
- `merge(base, overrides, precedence = Precedence::LaterWins)` でテンプレート同士のマージ戦略を制御。
- `Config.render(template, env)` はランタイムでテンプレートを具現化し、`audit_id` を発番する。

## D. CLI 連携（Draft）

- `kestrel-config validate config.ks` で検証、`--format json` で構造化ログ出力。
- `kestrel-config diff old.ks new.ks` で `change_set` を表示。
- `kestrel-config render --template prod.ks --env staging` でテンプレートを適用。
- CLI は `audit_id` を生成し、JSON 出力の `audit_id` と一致させる。（2-5 の構造化ログ案と同期）

## E. サンプルワークフロー（Draft）

```kestrel
let base = Config.load("base.ks")?
let overrides = Config.load("env/prod.ks")?
let merged = Config.merge(base, overrides)

match Config.compare(base, merged) with
| Ok(()) -> Config.save("rendered/prod.ks", merged)
| Err(diff) -> {
    audit.log("config.diff", diff)
    Err(diff)
  }
```

> 詳細仕様はフェーズ2で確定予定。API 名称やシグネチャは変更の可能性があります。
> 運用手順については [設定 CLI ワークフロー](guides/config-cli.md) を参照してください。

## F. 型とエラー

```kestrel
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

```kestrel
fn render(template: Schema<T>, env: Map<String, Any>) -> Result<RenderedConfig, ConfigError>
```

- 成功時は `RenderedConfig` を返し、`audit_id` を自動発行。
- 失敗時は `ValidationError`（`Diagnostic` の一覧）などを返す。
- CLI `kestrel-config render` はエラー種別に応じて exit code を決定（下表参照）。

| ConfigError | exit code | 備考 |
| --- | --- | --- |
| `ValidationError` | 2 | スキーマ違反、`Diagnostic` を出力 |
| `RenderError` | 3 | テンプレート内の計算失敗 |
| `IoError` | 4 | ファイル読み書き・パーミッションエラー |

```

