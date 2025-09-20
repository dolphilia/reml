# 2.8 データモデリング API（Nest.Data） - Draft

> 目的：データパイプラインや分析 DSL で利用するスキーマ・列・リソース ID 型を提供し、検証とスキーマ進化を支援する。

## A. 型定義

```kestrel
type Column<T, Meta = ()>
type Schema<T>
type ResourceId<P, K>
```

- `Meta` には統計情報や制約を格納する。
- `ResourceId` はクラウド/ネットワークリソースを型安全に扱うタグ型。
- `Schema` は `Schema<Record>` としてフィールド名→`Column` のマップを保持。
- `SchemaDiff<T>` 型を提供し、旧/新スキーマの差分を表現。

## B. 検証ユーティリティ

```kestrel
fn validate<T>(schema: Schema<T>, value: T) -> Result<(), List<Diagnostic>>
```

- 失敗時は `Diagnostic`（2.5 節の拡張メタデータを利用）を返す。
- `Constraint` trait で `requires` 句を評価。
- `Profile` インターフェイスを導入し、`validate_with_profile(schema, value, profile)` でプロファイル別ルールを適用。

## C. スキーマ進化

```kestrel
fn diff(old: Schema<T>, new: Schema<T>) -> SchemaDiff<T>
fn apply_migration<T>(value: T, diff: SchemaDiff<T>) -> Result<T, MigrationError>
```

- `SchemaDiff` は `1-2` で導入した型と互換。
- マイグレーション DSL から呼び出すことを想定。
- `plan_migration(diff)` は `MigrationStep` のリストを生成し、手動レビューを支援。
- `rollback(step, value)` は失敗時に戻すためのヘルパ。

## D. 連携例

- データパイプライン DSL で `SchemaDiff` を利用し、ETL ジョブの安全性を検証。
- クラウド構成 DSL で `ResourceId` 型を用いてリソースの重複作成を防止。
- 機械学習 DSL で `Column<T>` の `Meta` に統計情報（平均・分散）を保持し、データドリフト検出を実装。

```kestrel
let schema = Schema.build(|s| {
  s.field("user_id", Column<Guid, Unique>)
   .field("score", Column<f64, { nullable = false }>)
})

match Data.validate(schema, incoming) with
| Ok(()) -> Ok(incoming)
| Err(diags) -> {
    audit.log("data.validation", diags)
    Err(diags)
  }
```

> 詳細 API はフェーズ2のドラフトで確定予定。
