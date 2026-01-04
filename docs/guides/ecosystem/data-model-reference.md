# データモデル運用ガイド（Nest.Data）

> 目的：`Core.Data` / `Nest.Data` モジュールで利用するスキーマ・制約・品質指標を運用面から整理し、CLI・ランタイム・監査ログと共通の手順で扱えるようにする。
> 仕様参照：公式 API・JSON スキーマは [3-7 Core Config & Data](../../spec/3-7-core-config-data.md#4-data-モデリング-api再整理) に統合済み。本ガイドは現場運用や CI 手順の補足に重点を置く。

## 1. 基本構造

```reml
use Nest.Data

let userSchema = Schema.build("User", |s| {
  s.field("id", Column<Guid, { nullable = false }>)
   .field("email", Column<Text, { nullable = false, description = "連絡先" }>)
   .field("signup_at", Column<DateTime, { nullable = false }>)
   .field("score", Column<f64, { nullable = true, stats = Some({ mean = 0.0, stddev = 1.0, ..ColumnStats::zero }) }))
   .index("pk_user", columns = ["id"], unique = true)
})
```

- `Schema.build(name, builder)` は `3-7-core-config-data.md` の `Schema<T>` を組み立てる高水準ユーティリティ。
- `ColumnMeta` の `stats` は計測値をキャッシュし、`reml-data validate` の出力と突合する。

## 2. 制約と品質チェック

```reml
struct EmailFormat;
impl Constraint<Text> for EmailFormat {
  fn id() -> Str = "constraint.email.format"
  fn check(value: &Text, ctx: ConstraintContext) -> Result<(), Diagnostic> {
    if Regex::is_match("^[^@]+@[^@]+$", value) {
      Ok(())
    } else {
      Err(Diagnostic::error(ctx.path, "メールアドレス形式が不正です"))
    }
  }
}

let hardenedSchema = userSchema.with(|s| {
  s.constraint("email", EmailFormat)
   .constraint("score", Range::new(-10.0, 10.0))
})
```

- `ConstraintContext.profile` は `prod` / `staging` 等の識別子。閾値を `Profile::overrides()` で差し替え可能。
- `Diagnostic` には `domain = "schema"` とエラーコード（`E7001` など）を付与し、`../lsp/lsp-integration.md` の CodeAction と整合させる。
- JSON 直列化時には `diagnostics[].locale` と `message_key` / `locale_args` を含め、CLI・LSP いずれからも同じ翻訳カタログを再
  利用できるようにする。

## 3. プロファイル別検証

```reml
struct ProdProfile;
impl Profile for ProdProfile {
  fn id(&self) -> ProfileId = ProfileId::new("prod")
  fn overrides(&self) -> Map<Str, Any> = map!{ "score.max" => 5.0 }
}

let report = validate_with_profile(hardenedSchema, incoming, &ProdProfile)?
if !report.diagnostics.is_empty() {
  emit_metrics("data.validation", {
    latency_ms = 12.4,
    throughput_per_min = 320.0,
    error_rate = report.diagnostics.len() as f64 / incoming.len() as f64,
    last_audit_id = report.audit_id,
    custom = map!{ "profile" => "prod" }
  })
}
```

- `ValidationReport.stats` は列名→`ColumnStats` のマップ。`../runtime/runtime-bridges.md` の `RuntimeMetrics` と同じ JSON キー (`latency_ms` 等) を使用する。
- `audit_id` が付与されると `reml-data validate` の JSON でも同じ値が出力され、CI で突合できる。
- `severity_hint` はデータ品質に対する推奨アクション（`Retry`, `Rollback`, `Escalate` 等）を伝達し、運用ダッシュボードでの優先度付けに利用できる。

## 4. CLI サンプル

```bash
# スキーマ検証（prod プロファイル）
reml-data validate data/users.json --schema schemas/user.ks --profile prod --format json --locale ja-JP \
  | jq '.report | {audit_id, locales: (.diagnostics | map(.locale) | unique), diagnostics, stats}'

# スキーマ差分（マイグレーション計画）
reml-data diff --schema-old schemas/user_v1.ks --schema-new schemas/user_v2.ks --format json --locale en-US \
  | jq '.changes[] | {path, kind, breaking}'

# マイグレーション適用（失敗時のロールバック情報を保存）
reml-data migrate --diff diff.json --input data/import.parquet --output data/output.parquet --locale en-US \
  || cat rollback.json

# データ品質評価（staging プロファイル）
reml-data quality run data/users.json --schema schemas/user.ks --profile staging --format json --locale ja-JP \
  | jq '.report | {audit_id, profile, severity_max, findings}'

# StatsProvider を使った統計更新
reml-data stats collect --schema schemas/user.ks --provider warehouse --format json \
  | jq '.stats | keys'
```

### 4.1 ロケール伝搬と警告ポリシー

1. `reml-data` 系 CLI は `--locale <lang-tag>` を `RunConfig.locale` に写し、解析・整形の両レイヤで同じロケールを使用する。
2. 指定が無い場合は `REML_LOCALE` → `LANG` を参照し、いずれも無ければ `Locale::EN_US` を採用して `PrettyOptions` にも同期する。
3. 既定ロケールへフォールバックしたときは **最初の実行でのみ警告**を表示する。`--format json` のときは警告を `report.diagnostics`
   に `severity = "Warning"`, `message_key = "cli.locale.default"`, `locale = "en-US"` として添付し、サイレントモードでは抑制する。
4. 解析結果の JSON は `diagnostics[].locale` を含むため、IDE や監査ダッシュボードが後段で別ロケールに再整形する際の基準として
   利用できる。

- CLI 出力の JSON は `3-7-core-config-data.md` の `SchemaDiff`/`MigrationStep`/`MigrationError` を直列化したもの。
- `--format json` の `report.metrics` セクションは `RuntimeMetrics` に準拠し、監視基盤へ直接送信できる。

## 5. 監査ログの統合

| イベント | 出所 | 主なフィールド |
| --- | --- | --- |
| `reml.data.validate` | `reml-data validate` | `audit_id`, `diagnostics`, `profile`, `stats` |
| `reml.data.migrate` | `reml-data migrate` | `audit_id`, `changes`, `duration_ms`, `status` |
| `reml.data.rollback` | `reml-data migrate --rollback` | `audit_id`, `actions`, `reason` |
| `reml.data.quality` | `reml-data quality run` | `audit_id`, `profile`, `findings`, `severity_max`, `stats` |
| `reml.data.quality.rule` | `register_quality_rule`, `reml-data quality rules list` | `rule_id`, `scope`, `severity`, `owner` |

- 監査ログは `../runtime/runtime-bridges.md` に記載の JSON 構造（`event`, `audit_id`, `change_set`）を踏襲。
- IDE との連携では `../lsp/lsp-integration.md` の `data` フィールドに `domain = "schema"` を埋め込み、差分レビュー画面へリンクする。

## 6. QualityReport スキーマ {#quality-report-schema}

```json
{
  "$id": "https://spec.reml.dev/schema/quality-report.json",
  "type": "object",
  "required": ["profile", "findings", "stats", "generated_at"],
  "properties": {
    "profile": {"type": "string"},
    "audit_id": {"type": ["string", "null"], "format": "uuid"},
    "generated_at": {"type": "string", "format": "date-time"},
    "severity_max": {"enum": ["None", "Warn", "Error"]},
    "stats": {
      "type": "object",
      "additionalProperties": {"$ref": "#/definitions/columnStats"}
    },
    "findings": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["rule", "scope", "severity", "diagnostic", "auto_fixed"],
        "properties": {
          "rule": {"type": "string"},
          "scope": {"$ref": "#/definitions/qualityScope"},
          "severity": {"enum": ["Warn", "Error"]},
          "diagnostic": {"$ref": "https://spec.reml.dev/schema/diagnostic.json"},
          "auto_fixed": {"type": "boolean"}
        }
      }
    }
  },
  "definitions": {
    "qualityScope": {
      "oneOf": [
        {"type": "object", "required": ["Column"], "properties": {"Column": {"type": "object", "required": ["name"], "properties": {"name": {"type": "string"}}}}},
        {"const": "Dataset"},
        {"type": "object", "required": ["Relation"], "properties": {"Relation": {"type": "object", "required": ["columns"], "properties": {"columns": {"type": "array", "items": {"type": "string"}}}}}}
      ]
    },
    "columnStats": {
      "type": "object",
      "required": ["count"],
      "properties": {
        "count": {"type": "integer", "minimum": 0},
        "distinct": {"type": ["integer", "null"], "minimum": 0},
        "min": {"type": ["number", "null"]},
        "max": {"type": ["number", "null"]},
        "mean": {"type": ["number", "null"]},
        "stddev": {"type": ["number", "null"]},
        "percentiles": {
          "type": ["object", "null"],
          "additionalProperties": {"type": "number"}
        },
        "histogram": {
          "type": ["array", "null"],
          "items": {
            "type": "object",
            "required": ["bucket", "count"],
            "properties": {
              "bucket": {
                "type": "object",
                "required": ["label", "min", "max"],
                "properties": {
                  "label": {"type": "string"},
                  "min": {"type": "number"},
                  "max": {"type": "number"}
                }
              },
              "count": {"type": "integer", "minimum": 0}
            }
          }
        },
        "last_updated": {"type": ["string", "null"], "format": "date-time"}
      }
    }
  }
}
```

### 6.1 監査ログ整合テストケース

1. **Severity Propagation**: `reml-data quality run` で `Error` finding を発生させ、CLI 出力と `audit.log("reml.data.quality")` の `severity_max` が一致するか確認。
2. **Stats Drift Guard**: `ColumnStats.last_updated` を未来日時に偽装した入力を流し、CLI が `Diagnostic` を返して JSON スキーマ検証が失敗することをチェック。
3. **Scope Serialization**: `Relation` スコープのルールで `findings[].scope.Relation.columns` が配列になるか `jq` テスト。
4. **Auto Fix Flag**: 自動修正が実行されたケースで `findings[].auto_fixed=true` と監査ログの `auto_fix=true` が両立するか検証。

### 6.2 StatsProvider との統合

- `reml-data stats collect` の JSON は `columnStats` 定義と互換であり、`run_quality` が返す `QualityReport.stats` へマージしてもスキーマ検証を通過すること。
- 異常系テスト: `StatsProvider` が重複したヒストグラム区間を返した場合、`run_quality` が `Diagnostic` を発生させ CLI exit code `8`（StatsInvalid）を返すことを確認。

## 7. ベストプラクティス

1. **スキーマ・コード同居**: `Schema.build` で定義した DSL をリポジトリ内の `schemas/` ディレクトリに集約し、CI で常に `reml-data validate` を実行する。
2. **統計の自動更新**: バッチ処理後に `ColumnStats` を更新し、`stats.updated_at` をログへ出力する。
3. **Breaking 変更の承認**: `MigrationStep` に `breaking=true` が含まれる場合、`reml-plugin` の承認者ロールと同じレビュー手順を経る。
4. **可視化連携**: `RuntimeMetrics` を Prometheus や Grafana に輸出し、`audit_id` をキーにエラーとの関連を追跡。

## 8. 参考リンク

- [3.7 Core Config & Data](../../spec/3-7-core-config-data.md)
- [ランタイム連携ガイド](../runtime/runtime-bridges.md)
- [LSP / IDE 連携ガイド](../lsp/lsp-integration.md)
- [設定 CLI ワークフロー](../tooling/config-cli.md)
