# データモデルリファレンス（Nest.Data）

> 目的：`Core.Data` / `Nest.Data` モジュールで利用するスキーマ・制約・品質指標を体系化し、CLI・ランタイム・監査ログと共通の言語で扱えるようにする。

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

- `Schema.build(name, builder)` は `2-8-data.md` の `Schema<T>` を組み立てる高水準ユーティリティ。
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
- `Diagnostic` には `domain = "schema"` とエラーコード（`E7001` など）を付与し、`guides/lsp-integration.md` の CodeAction と整合させる。

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

- `ValidationReport.stats` は列名→`ColumnStats` のマップ。`guides/runtime-bridges.md` の `RuntimeMetrics` と同じ JSON キー (`latency_ms` 等) を使用する。
- `audit_id` が付与されると `reml-data validate` の JSON でも同じ値が出力され、CI で突合できる。
- `severity_hint` はデータ品質に対する推奨アクション（`Retry`, `Rollback`, `Escalate` 等）を伝達し、運用ダッシュボードでの優先度付けに利用できる。

## 4. CLI サンプル

```bash
# スキーマ検証（prod プロファイル）
reml-data validate data/users.json --schema schemas/user.ks --profile prod --format json \
  | jq '.report | {audit_id, diagnostics, stats}'

# スキーマ差分（マイグレーション計画）
reml-data diff --schema-old schemas/user_v1.ks --schema-new schemas/user_v2.ks --format json \
  | jq '.changes[] | {path, kind, breaking}'

# マイグレーション適用（失敗時のロールバック情報を保存）
reml-data migrate --diff diff.json --input data/import.parquet --output data/output.parquet \
  || cat rollback.json
```

- CLI 出力の JSON は `2-8-data.md` の `SchemaDiff`/`MigrationStep`/`MigrationError` を直列化したもの。
- `--format json` の `report.metrics` セクションは `RuntimeMetrics` に準拠し、監視基盤へ直接送信できる。

## 5. 監査ログの統合

| イベント | 出所 | 主なフィールド |
| --- | --- | --- |
| `reml.data.validate` | `reml-data validate` | `audit_id`, `diagnostics`, `profile`, `stats` |
| `reml.data.migrate` | `reml-data migrate` | `audit_id`, `changes`, `duration_ms`, `status` |
| `reml.data.rollback` | `reml-data migrate --rollback` | `audit_id`, `actions`, `reason` |

- 監査ログは `guides/runtime-bridges.md` に記載の JSON 構造（`event`, `audit_id`, `change_set`）を踏襲。
- IDE との連携では `guides/lsp-integration.md` の `data` フィールドに `domain = "schema"` を埋め込み、差分レビュー画面へリンクする。

## 6. ベストプラクティス

1. **スキーマ・コード同居**: `Schema.build` で定義した DSL をリポジトリ内の `schemas/` ディレクトリに集約し、CI で常に `reml-data validate` を実行する。
2. **統計の自動更新**: バッチ処理後に `ColumnStats` を更新し、`stats.updated_at` をログへ出力する。
3. **Breaking 変更の承認**: `MigrationStep` に `breaking=true` が含まれる場合、`kestrel-plugin` の承認者ロールと同じレビュー手順を経る。
4. **可視化連携**: `RuntimeMetrics` を Prometheus や Grafana に輸出し、`audit_id` をキーにエラーとの関連を追跡。

## 7. 参考リンク

- [2.8 Core.Data](../2-8-data.md)
- [ランタイム連携ガイド](runtime-bridges.md)
- [LSP / IDE 連携ガイド](lsp-integration.md)
- [設定 CLI ワークフロー](config-cli.md)
