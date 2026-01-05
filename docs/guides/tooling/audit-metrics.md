# 監査・メトリクス運用ガイド

本書は Reml リポジトリにおける監査ログと CI メトリクスの共通運用を定義する。旧 `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の後継として、現行の Rust 実装と CI パイプラインに合わせて更新する。

## 目的

- 診断 JSON / 監査ログ / KPI を同一のルールで収集し、仕様と実装の差分を定量化する。
- `reports/` 配下の成果物と、`docs/` 配下の計画・ガイド・ノートを同期させる。
- CI で検証される監査メタデータと診断キーの欠落を検知し、フェーズ判断に使う。

## 運用の中心となる入力

- 診断 JSON 検証: `scripts/validate-diagnostic-json.sh`
- 監査メトリクス集計: `tooling/ci/collect-iterator-audit-metrics.py`
- 監査ログ突合: `tooling/ci/sync-iterator-audit.sh`
- 監査インデックス生成: `tooling/ci/create-audit-index.py`
- 監査インデックス検証: `tooling/ci/verify-audit-metadata.py`

## 出力と保存場所

- 監査・診断の集計結果: `reports/audit/`, `reports/spec-audit/`
- 監査ゲートのサマリ: `reports/iterator-stage-summary.md`, `reports/iterator-collector-summary.md`
- 監査インデックス: `reports/audit/index.json`
- KPI 参照用の CSV/JSON: `docs/plans/bootstrap-roadmap/assets/metrics/`

## 記録ルール

- 監査ログのスキーマは `tooling/runtime/audit-schema.json` を正とし、更新時は `docs/spec/3-6-core-diagnostics-audit.md` と整合させる。
- 診断 JSON のスキーマ変更は `docs/schemas/` に反映し、`scripts/validate-diagnostic-json.sh` の検証観点に追記する。
- KPI の更新は `reports/` の成果物を根拠とし、更新理由と Run ID を `docs/notes/docs-migrations.log` に記録する。
- 仕様変更により KPI が変わる場合は、該当する計画・ガイド・ノートにリンクを追加し相互参照を維持する。

## KPI の所在

KPI の詳細値や閾値は、次の資料に分散して管理している。

- 計画上の KPI 参照: `docs/plans/bootstrap-roadmap/`
- 監査/診断の仕様: `docs/spec/3-6-core-diagnostics-audit.md`
- Capability/Stage の運用: `docs/spec/3-8-core-runtime-capability.md`
- 実測ログ: `reports/spec-audit/` と `reports/audit/`

## 主要 KPI（運用で必須）

| KPI | 目的 | 計測 | 出力/保存先 | 関連 |
| --- | --- | --- | --- | --- |
| `diagnostic.audit_presence_rate` | 診断 JSON と監査ログの必須キー欠落を検知する | `scripts/validate-diagnostic-json.sh` | CI ログ | `docs/spec/3-6-core-diagnostics-audit.md` |
| `iterator.stage.audit_pass_rate` | Iterator/Stage 監査メタデータの整合性を確認する | `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --require-success` | `tooling/ci/iterator-audit-metrics.json`, `reports/iterator-stage-summary.md` | `docs/spec/3-1-core-prelude-iteration.md` |
| `core_io.effect_matrix_pass_rate` | Core.IO/Path の効果・Capability 行列が揃うことを確認する | `python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario effects_matrix --matrix docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md --output reports/spec-audit/ch3/core_io_effects.json --require-success` | `reports/spec-audit/ch3/core_io_effects.json` | `docs/spec/3-5-core-io-path.md` |
| `numeric_time.effect_matrix_pass_rate` | Numeric/Time の効果・Capability 行列が揃うことを確認する | `python3 tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario effects_matrix --matrix docs/plans/bootstrap-roadmap/assets/core-numeric-time-effects-matrix.md --output reports/spec-audit/ch3/core_numeric_time_effects.json --require-success` | `reports/spec-audit/ch3/core_numeric_time_effects.json` | `docs/spec/3-4-core-numeric-time.md` |
| `runtime.capability_validation` | Runtime Capability の Stage 判定が揃うことを確認する | `python3 tooling/ci/collect-iterator-audit-metrics.py --section runtime --dry-run` | `docs/plans/bootstrap-roadmap/assets/metrics/runtime-capability-stage.csv` | `docs/spec/3-8-core-runtime-capability.md` |
| `text.mem.zero_copy_ratio` | Text/Bytes のゼロコピー比率を追跡する | `python3 tooling/ci/collect-iterator-audit-metrics.py --section text --scenario bytes_clone --text-mem-source reports/text-mem-metrics.json --output reports/text-mem-metrics.json --require-success` | `reports/text-mem-metrics.json` | `docs/spec/3-3-core-text-unicode.md` |

必要に応じて、計画書側の KPI 表（`docs/plans/bootstrap-roadmap/`）に詳細指標を追加する。

## 変更時のチェックリスト

1. 監査ログや診断 JSON を更新したら `reports/` の成果物を更新する。
2. `collect-iterator-audit-metrics.py` の出力を参照し、関連する計画文書の KPI 記述を更新する。
3. 参照元のドキュメントにリンク切れが無いか確認する。

## 関連ドキュメント

- `docs/guides/tooling/ci-strategy.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/spec/3-8-core-runtime-capability.md`
- `docs/notes/runtime/runtime-metrics-capability.md`
