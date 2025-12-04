# MigrationPlan パイロットログ（Run ID: 20251224-migration-plan-alpha）

Config/Data 計画 5.1 の成果として `reml_runtime::config::migration` を実装し、`experimental-migration` フィーチャ有効時のみ `MigrationPlan`/`MigrationStep` を公開した。`effect {migration}` を監査ログへ出力する際のリファレンスとして、最初のシリアライズ/デシリアライズ検証ログとサンプル JSON をまとめる。

## 1. 構成要素
- ソース: `compiler/rust/runtime/src/config/migration.rs`
  - `MigrationPlan` / `MigrationDuration` / `MigrationRiskLevel` / `MigrationStep`（`add_field`/`remove_field`/`rename_field`/`change_type`/`reorganize_data`）。
  - `ReorganizationStrategy` と `TypeConversionPlan` を `serde` 互換で実装し、`MIGRATION_EFFECT_TAG = "migration"` を公開。
- Cargo フィーチャ: `cargo test -p reml_runtime --features experimental-migration …` でのみコンパイル・公開。
- 監査連携: Config CLI が `effect {migration}` を発火した際、`config.migration.*` メタデータ付き `Diagnostic` と `AuditEnvelope` を生成することを前提に、Diagnostics 計画（3.6）へ Run ID を共有済み。

## 2. 実行手順
```
cargo test -p reml_runtime \
  --features experimental-migration \
  migration_plan::tests::plan_serialization_roundtrip \
  -- --nocapture
```
- 追加したユニットテストは `MigrationPlan` の JSON ラウンドトリップと `breaking` マーカーの伝搬を検証する。`serde_json::to_string_pretty` で得た JSON をパースし、`has_breaking_changes()` と推定所要時間 (`MigrationDuration::seconds`) を突き合わせる。
- 上記テストを `reports/spec-audit/ch3/migration_plan-pilot.md` に紐づけておくことで、Phase 4 で `reml config migrate` CLI を導入する際のゴールデン指標として扱える。

## 3. サンプル JSON
```json
{
  "steps": [
    {
      "kind": "add_field",
      "name": "new_column",
      "field": {
        "name": "new_column",
        "data_type": "string",
        "required": true,
        "description": "新しい文字列列",
        "default_value": null,
        "examples": [],
        "rules": [],
        "metadata": {}
      },
      "default_value": "fallback",
      "breaking": false
    },
    {
      "kind": "change_type",
      "name": "score",
      "old_type": "integer",
      "new_type": "number",
      "converter": {
        "converter_name": "score_to_float",
        "description": "整数→浮動小数へ安全に変換",
        "lossy": false
      },
      "breaking": true
    }
  ],
  "estimated_duration": {
    "seconds": 3600
  },
  "requires_downtime": true,
  "data_loss_risk": "medium"
}
```

## 4. KPI / TODO
- `collect-iterator-audit-metrics.py --section config` に `migration_effect_presence` を追加し、`MIGRATION_EFFECT_TAG` を含む診断/監査ログをカウントする。（次期スプリント）
- `reml config migrate` CLI（3-7 §5.3）のドラフト実装時に、本ログの Run ID を参照して `effect.stage.required = StageId::Beta` を必須チェックへ追加する。
- `docs/notes/dsl-plugin-roadmap.md` へ MigrationPlan のエスカレーション基準（`MigrationStep.breaking = true` のレビュー手順）を記す。
