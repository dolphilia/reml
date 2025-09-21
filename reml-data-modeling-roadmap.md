# Reml データモデリングAPI拡充ロードマップ

## 1. 現状の確認
- `Core.Data` は列メタデータ・制約トレイト・スキーマ差分を標準化し、バリデーション/マイグレーション/監査ログを統合している `2-8-data.md:1`
- `Core.Config` は設定スキーマと差分適用を提供し、`SchemaDiff` と `Change` 型を `Core.Data` と共有する `2-7-config.md:1`
- `scenario-requirements.md:100` ではデータパイプライン/分析DSLにおける品質検証、統計メタデータ、監査ログ統合が必須機能として挙げられている。

## 2. 拡張テーマ別の設計方針
### 2.1 データ品質DSL
- `quality { ... }` ブロックを追加し、列やリレーションに対するルールを宣言的に記述。
- `Constraint<T>` トレイトをラップする `QualityRule` を導入し、`severity`（`Warn`/`Error`）と `auto_fix` を保持。
- CLI 連携：`reml-data quality run dataset.json --profile prod` で `QualityReport` を JSON 出力し、`audit_id` を `Config` と共有。

### 2.2 統計型と集計メタデータ
- `ColumnMeta.stats` を拡張し、`percentiles`, `histogram`, `last_updated` を追加 `2-8-data.md:32`
- `StatType` 列（例: `MovingAverage`, `Counter`, `WindowedMetric`）を標準化し、時間窓指定をサポート。
- `StatsProvider` インターフェイスを導入してバックエンド（OLAP, 時系列DB）を切り替え可能に。

### 2.3 Core.Config との統合
- `Config.schema` に `data_source` 属性を追加し、`Core.Data.Schema` との双方向リンクを保持 `2-7-config.md:6`
- `Config.plan` で得られる `SchemaDiff` を `Data.plan_migration` に渡すワークフロー例を文書化。
- `RunConfig` に `data_profile` セクションを追加し、CLI/ランタイムの検証モードを切り替えられるようにする。

## 3. API 追加提案
### 3.1 DSL 構文スケッチ
```reml
schema UserAnalytics {
  field user_id: Guid
  field score: f64 { quality min = 0, warn_if > 100 }
  field region: Str { enum = Regions }

  quality dataset {
    require completeness >= 0.995
    alert if duplicates on user_id
  }
}
```
- `quality dataset` ブロックはデータセット全体のルールを宣言。
- 列レベルでは `warn_if` / `fail_if` / `auto_fix` を DSL 糖衣として提供。

### 3.2 ランタイム API 草案
```reml
type QualityRule = {
  id: Str,
  scope: QualityScope,
  severity: Severity,
  check: fn(&Value, &QualityContext) -> Result<(), Diagnostic>,
  auto_fix: Option<fn(Value) -> Value>
}

fn register_rule(schema: Schema<T>, rule: QualityRule) -> Schema<T>

fn run_quality<T>(schema: Schema<T>, data: Iterator<T>, profile: QualityProfile)
  -> Result<QualityReport, QualityReport>
```
- `QualityProfile` は環境別閾値を提供し、`Core.Config` 側からロード。
- `QualityReport` は `ValidationReport`（`2-8-data.md:52`）と同じ構造を継承し、`quality_domain: "dataset"|"column"` を追加。

### 3.3 統計型 API
```reml
type StatType =
  | MovingAverage { window: Duration }
  | Histogram { buckets: List<HistogramBucket> }
  | Counter { mode: CounterMode }

type HistogramBucket = { min: Numeric, max: Numeric, label: Str }
```
- `ColumnStats` に `histogram: Option<List<HistogramBucketState>>` を追加し、`run_quality` が収集した統計を保持。
- `StatsProvider` が存在する場合は `QualityReport` に `provider_id` を追記し、メトリクス連携を容易にする。

## 4. ガバナンスと監査
- `audit.log("data.quality", QualityReport)` を推奨し、`guides/runtime-bridges.md` の監査規約に従う。
- `QualityRule` には `rationale` と `owner` メタデータを含め、監査時に責任者と理由を追跡。
- `SchemaDiff` 適用時に `QualityRule` の互換性チェックを必須化し、破壊的変更には `MigrationError::DataLossRisk` を返す `2-8-data.md:69`。

## 5. 実装フェーズ計画
| フェーズ | 内容 | 成果物 |
| --- | --- | --- |
| P0 | DSL構文ドラフトと API 型 | `Core.Data`/`Core.Config` 更新案、サンプルDSL |
| P1 | バリデーション基盤の拡充 | `run_quality` 実装仕様、`QualityReport` JSON スキーマ |
| P2 | 統計型とプロファイル連携 | `StatsProvider` 契約、`RunConfig.data_profile` 仕様 |
| P3 | ガバナンス／監査統合 | CLI コマンド仕様、`audit.log` テンプレート |

## 6. 既存ドキュメント更新対象
- `2-8-data.md`：`QualityRule`、`QualityProfile`、統計型の詳細を追加。
- `2-7-config.md`：`data_source` 属性と `RunConfig.data_profile` の連携を追記。
- `guides/data-model-reference.md`：品質DSLの具体例と監査フローを増補。
- `guides/runtime-bridges.md`：`data.quality` ログと `StatsProvider` 連携例を追加。

## 7. 次アクション
1. DSL 糖衣のBNF案を `3-1-bnf.md` 用に作成。
2. `QualityReport` JSON スキーマ草案を `guides/data-model-reference.md` に挿入する準備。
3. `RunConfig` 拡張と CLI コマンド仕様案を `spec-update-plan.md` に追加。
4. 代表ユースケース（ETL, ML前処理, 監査）ごとのベンチマーク指標を洗い出し、P1 の計測目標を設定。
