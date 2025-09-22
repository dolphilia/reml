# 2.8 データモデリング API（Core.Data）

> **移行ガイド**: Chapter 3 にて標準ライブラリ観点の仕様と監査連携を再整理しています。最新版は [3.7 Core Config & Data](3-7-core-config-data.md) を参照してください。本章は旧仕様のリファレンスとして残されています。

> 目的：データパイプラインや分析 DSL で利用するスキーマ・列・リソース ID 型を提供し、検証とスキーマ進化を支援する。

## A. 型定義

```reml
type Column<T, Meta = ()>
type Schema<T>
type ResourceId<P, K>
```

- `Meta` には統計情報や制約を格納する。
- `ResourceId` はクラウド/ネットワークリソースを型安全に扱うタグ型。
- `Schema` は `Schema<Record>` としてフィールド名→`Column` のマップを保持。
- `SchemaDiff<T>` 型を提供し、旧/新スキーマの差分を表現。
- `stat_plan` は列ごとの統計収集方針 (`StatType`) を宣言し、`StatsProvider` もしくは `run_quality` が参照する。
```reml
type Column<T, Meta> = {
  dtype: TypeRef<T>,
  meta: Meta,
  constraints: List<Constraint<T>>
}

type ColumnMeta = {
  nullable: Bool,
  description: Option<Str>,
  stats: Option<ColumnStats>,
  stat_plan: Option<StatType>
}

type ColumnStats = {
  count: u64,
  distinct: Option<u64>,
  min: Option<Numeric>,
  max: Option<Numeric>,
  mean: Option<f64>,
  stddev: Option<f64>,
  percentiles: Option<Map<Percentile, f64>>,
  histogram: Option<List<HistogramBucketState>>,
  last_updated: Option<Timestamp>
}

type Percentile = f64   // 0.0〜1.0

type HistogramBucket = {
  label: Str,
  min: Numeric,
  max: Numeric
}

type HistogramBucketState = {
  bucket: HistogramBucket,
  count: u64
}

type StatType =
  | MovingAverage { window: Duration }
  | Histogram { buckets: List<HistogramBucket> }
  | Counter { mode: CounterMode }

type CounterMode = "Accumulate" | "ResetEachRun"

type ResourceId<P, K> = {
  provider: P,
  key: K,
  region: Option<Str>,
  version: Option<SemVer>
}

type Schema<T> = {
  name: Str,
  fields: Map<Str, Column<Any, ColumnMeta>>,
  indices: List<IndexSpec>,
  version: SemVer
}

type IndexSpec = {
  name: Str,
  columns: List<Str>,
  unique: Bool
}
```

## B. 検証ユーティリティ

```reml
fn validate<T>(schema: Schema<T>, value: T) -> Result<(), List<Diagnostic>>
```

- 失敗時は `Diagnostic`（2.5 節の拡張メタデータを利用）を返す。
- `Constraint` trait で `requires` 句を評価。
- `Profile` インターフェイスを導入し、`validate_with_profile(schema, value, profile)` でプロファイル別ルールを適用。
- `ValidationReport` は統計と FixIt を含み、CLI/IDE 双方で同一 JSON を共有する。
```reml
trait Constraint<T> {
  fn check(value: &T, ctx: ConstraintContext) -> Result<(), Diagnostic>;
  fn id() -> Str;               // 監査ログ用の安定ID
}

type ConstraintContext = {
  path: List<Str>,
  profile: ProfileId,
  audit_id: Option<Uuid>
}

trait Profile {
  fn id(&self) -> ProfileId;
  fn overrides(&self) -> Map<Str, Any>;  // 閾値等を上書き
}

type ValidationReport = {
  diagnostics: List<Diagnostic>,
  stats: Map<Str, ColumnStats>,
  audit_id: Option<Uuid>
}

fn validate_with_profile<T>(schema: Schema<T>, value: T, profile: &impl Profile)
  -> Result<ValidationReport, ValidationReport>

type StatsProviderId = Str

type StatsProvider = {
  id: StatsProviderId,
  collect: fn(Schema<any>, ProfileId) -> Result<Map<Str, ColumnStats>, Diagnostic>,
  capabilities: Set<StatType>
}
```

## C. スキーマ進化

```reml
fn diff(old: Schema<T>, new: Schema<T>) -> SchemaDiff<T>
fn apply_migration<T>(value: T, diff: SchemaDiff<T>) -> Result<T, MigrationError>
```

- `SchemaDiff` は `1-2` で導入した型と互換。
- マイグレーション DSL から呼び出すことを想定。
- `plan_migration(diff)` は `MigrationStep` のリストを生成し、手動レビューを支援。
- `rollback(step, value)` は失敗時に戻すためのヘルパ。
```reml
type MigrationStep =
  | AddField { name: Str, column: Column<Any, ColumnMeta> }
  | DropField { name: Str }
  | AlterField { name: Str, from: ColumnMeta, to: ColumnMeta }
  | Reindex { index: IndexSpec }

type MigrationError =
  | BreakingChange { path: List<Str>, reason: Str }
  | DataLossRisk { columns: List<Str>, detail: Str }
  | ApplyFailed { step: MigrationStep, reason: Str }
```

## D. 連携例

- データパイプライン DSL で `SchemaDiff` を利用し、ETL ジョブの安全性を検証。
- クラウド構成 DSL で `ResourceId` 型を用いてリソースの重複作成を防止。
- 機械学習 DSL で `Column<T>` の `Meta` に統計情報（平均・分散）を保持し、データドリフト検出を実装。

```reml
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

## E. CLI / 監査連携

- `reml-data validate <data.json> --schema schema.ks --profile prod` で `ValidationReport` を JSON 出力し、`audit_id` を `Core.Config` のログと共有する。
- `reml-data diff --schema-old old.ks --schema-new new.ks` は `SchemaDiff` を計算し、`MigrationStep` 単位のレビュー用レポートを生成する。
- `reml-data migrate --diff diff.json --input dataset.parquet --output migrated.parquet` は `apply_migration` を呼び出し、失敗時に `MigrationError` を exit code `5`（Breaking）、`6`（DataLossRisk）、`7`（ApplyFailed）で通知する。
- CLI の JSON は `domain = "schema"` を設定し、`guides/data-model-reference.md` の品質指標と統合する。

ランタイム統合やホットリロード時のデータ適用手順は [ランタイム連携ガイド](guides/runtime-bridges.md) を参照し、データ品質の詳細な指標とテンプレートは [データモデルリファレンス](guides/data-model-reference.md) に収録する。

---

## F. データ品質 DSL（ドラフト）

```reml
type QualityScope =
  | Column { name: Str }
  | Dataset
  | Relation { columns: List<Str> }

type QualitySeverity = "Warn" | "Error"

type QualityRule<T> = {
  id: Str,
  scope: QualityScope,
  severity: QualitySeverity,
  check: fn(&T, &QualityContext) -> Result<(), Diagnostic>,
  auto_fix: Option<fn(T) -> T>,
  rationale: Option<Str>,
  owner: Option<Str>
}

type QualityContext = {
  path: List<Str>,
  profile: ProfileId,
  stats: Option<ColumnStats>,
  audit_id: Option<Uuid>
}

type QualityProfile = {
  id: ProfileId,
  severity_overrides: Map<QualityRuleId, QualitySeverity>,
  thresholds: Map<Str, Numeric>,
  allow_auto_fix: Bool
}

type QualityRuleId = Str

type QualityFinding = {
  rule: QualityRuleId,
  scope: QualityScope,
  severity: QualitySeverity,
  diagnostic: Diagnostic,
  auto_fixed: Bool
}

type QualityReport = {
  findings: List<QualityFinding>,
  stats: Map<Str, ColumnStats>,
  profile: ProfileId,
  audit_id: Option<Uuid>,
  generated_at: Timestamp
}

fn register_quality_rule<T>(schema: Schema<T>, rule: QualityRule<T>) -> Schema<T>

fn run_quality<T, I>(schema: Schema<T>, data: I, profile: QualityProfile,
                     provider: Option<StatsProvider>)
  -> Result<QualityReport, QualityReport>
  where I: Iterator<Item = T>
```

* `QualityRule` は列・データセット・リレーションの各スコープに適用でき、`auto_fix` を備えたルールはプロファイルで許可されていれば自動修正を試みる。
* `QualityProfile` は環境別の閾値や自動修正可否を管理し、`severity_overrides` で特定ルールの重要度を調整する。
* `run_quality` は成功・失敗いずれのケースでも `QualityReport` を返し、CLI・IDE・監査ログに同一フォーマットを提供する。致命的エラー時は `Err(report)` として返却される。
* `StatsProvider` を指定すると統計収集を外部に委譲でき、指定がなければ `stat_plan` を参照して必要な統計を `run_quality` が推定する。

### F-1. CLI との結合

- `reml-data quality run <dataset>` は `QualityReport` を JSON で出力し、`audit_id`・`profile`・`findings` を構造化して監査ログへ送る。
- `reml-data quality rules list` は登録済みルール ID・スコープ・現在の重要度を表示し、変更があれば `audit.log("data.quality.rule", …)` を発生させる。
- `reml-data quality explain <rule_id>` は `rationale` と `owner` を提示し、責任者と根拠を素早く確認できる。

### F-2. ガバナンス連携

- `register_quality_rule` は登録/更新時に監査ログを出力し、承認フローを記録する。
- `QualityReport.findings[].diagnostic` は 2.5 節の構造化エラーに準拠し、JSON スキーマは [データモデルリファレンス](guides/data-model-reference.md#quality-report-schema) に定義する。
- `QualityReport.stats` は実行後の `ColumnStats` を更新し、`last_updated` の値を通じてデータドリフト監視に活用できる。
