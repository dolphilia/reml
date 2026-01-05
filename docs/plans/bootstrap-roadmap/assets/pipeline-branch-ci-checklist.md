# pipeline_branch Stage mismatch CI チェックリスト

## 目的と前提
- `examples/core_diagnostics/pipeline_branch.reml` が `effects.contract.stage_mismatch` を 1 件だけ返し、CLI/Audit の両経路で `capability.*`・`effect.stage.*`・`pipeline.*` が欠落しないことを継続的に保証する。
- `tooling/examples/run_examples.sh --suite core_diagnostics` をフェイルファストさせるのではなく、`pipeline_branch` を `allowed failure` として扱う運用を、再現手順と監査ログの両面で固定する。
- 本チェックリストで定義する成果物は `docs/plans/bootstrap-roadmap/pipeline_branch-stage-mismatch-plan.md` §7、および `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` §5.2/§7 の証跡として参照する。

## チェック項目
| # | 観点 | コマンド/手順 | 期待成果物 | ノート |
| --- | --- | --- | --- | --- |
| 1 | CLI/診断 | `cargo run --quiet --bin reml_frontend -- --output json --emit-audit-log examples/core_diagnostics/pipeline_branch.reml` | `examples/core_diagnostics/pipeline_branch.expected.diagnostic.json` / `.audit.jsonl` を再生成（`schema_version = "3.0.0-alpha"`、`capability.id=console`、`effect.stage.required=at_least:beta`）。 | 失敗時は `tmp/pipeline_branch.*` を保存し `reports/spec-audit/ch3/capability_stage-mismatch-YYYYMMDD.json` に Run ID を残す。 |
| 2 | 例題スイート | `tooling/examples/run_examples.sh --suite core_diagnostics --with-audit --update-golden` | `core_diagnostics` 全体のゴールデン更新と `pipeline_branch` を `allowed failure during --update-golden` として扱ったログ (`logs/core_diagnostics-pipeline_branch-*.log`) | `set -e` で止まらないことを `examples/core_diagnostics/README.md` へ記載し、`pipeline_success` との差を README で説明する。 |
| 3 | JSON バリデーション | `scripts/validate-diagnostic-json.sh reports/spec-audit/ch3/capability_stage-mismatch-YYYYMMDD.json --effect-tag runtime` | `reports/spec-audit/ch3/capability_stage-mismatch-YYYYMMDD.json`（CLI/Audit 抜粋）に `capability.*` / `effect.stage.*` / `bridge.stage.*` / `pipeline.*` が揃っている検証ログ | 失敗時のログは `reports/spec-audit/ch3/capability_stage-mismatch-YYYYMMDD.log` に保存して再解析する。 |
| 4 | 監査メトリクス | `python3 tooling/ci/collect-iterator-audit-metrics.py --section core_diagnostics --scenario pipeline_branch --diagnostic-source ...pipeline_branch.expected.diagnostic.json --audit-source ...pipeline_branch.expected.audit.jsonl --require-success` | `reports/spec-audit/ch3/pipeline_branch-metrics-YYYYMMDD.json` に `diagnostics=1` / `audit_events=2` / `capability.id=console` を保存し、`docs/guides/tooling/audit-metrics.md#core_diagnostics` の KPI に追記 | `--scenario pipeline_branch` は `core_diagnostics` セクションへ追加しておき、未登録の場合は `collect-iterator-audit-metrics` へ PR を出す。 |
| 5 | CI 統合 | `.github/workflows/core-diagnostics.yml`（Phase3で追加予定）で上記 1〜4 を nightly ジョブに束ね、失敗時には `docs/plans/bootstrap-roadmap/0-4-risk-handling.md#runtime-capability` に Run ID と Git SHA を `scripts/ci/post_failure_runtime_capability.sh` 経由で追記する | CI アーティファクト `core-diagnostics-stage-mismatch-*.tar.gz` に `reports/spec-audit/ch3/*.json` を含める | `3-0-phase3-self-host.md#core-diagnostics` の判定条件に本ジョブを足し、再現性を確保する。 |

## エスカレーションと記録
- 本チェックリストの実施結果は `docs/plans/bootstrap-roadmap/pipeline_branch-stage-mismatch-plan.md#7-テスト・ci-反映` に Run ID・担当者・残課題として追記する。
- 監査ログや CLI の再現ログは `reports/spec-audit/ch3/` 配下に日付フォルダを作成し、`docs/notes/runtime/runtime-capability-stage-log.md` からリンクする。
- `collect-iterator-audit-metrics` の `--scenario pipeline_branch` が失敗した場合、`docs/plans/bootstrap-roadmap/0-4-risk-handling.md` の `runtime-capability` 節へ即時記録し、`3-8-core-runtime-capability-plan.md` §7.3 の CI 指標で回 regression を追跡する。
