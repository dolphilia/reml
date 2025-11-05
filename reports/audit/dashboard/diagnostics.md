# 診断ドメイン指標ダッシュボード

Phase 2-7 で導入した診断ドメイン関連 KPI を集約する。`collect-iterator-audit-metrics.py --section diagnostics --require-success` の出力を基に、CLI/LSP/監査ログの三チャネルで語彙・メタデータ・Stage 整合性が維持されているかを確認する。集計ログは `reports/audit/phase2-7/diagnostics-domain-20261221.json` に保存し、Phase 2-8 以降は同形式で週次更新する。

## diagnostics.domain_coverage

| プラットフォーム | pass_rate | 集計コマンド | 補足 |
|------------------|-----------|--------------|------|
| linux-x86_64 | 1.0 | `collect-iterator-audit-metrics.py --section diagnostics --require-success --source reports/audit/phase2-7/diagnostics-domain-20261221.json` | CLI/LSP/監査ログで `Effect`／`Plugin`／`Lsp`／`Capability` を含む語彙が揃っていることを確認。 |
| macos-arm64 | 1.0 | `collect-iterator-audit-metrics.py --section diagnostics --require-success --platform macos-arm64 --source reports/audit/phase2-7/diagnostics-domain-20261221.json` | `diagnostic_transport` V2 フィクスチャと同期済み。 |
| windows-msvc | 1.0 | `collect-iterator-audit-metrics.py --section diagnostics --require-success --platform windows-msvc --source reports/audit/phase2-7/diagnostics-domain-20261221.json` | `bootstrap-windows` ジョブで LSP/CLI 出力を検証。 |

基準値を下回った場合は `docs/spec/3-6-core-diagnostics-audit.md` の語彙表と CLI/LSP ゴールデンを突き合わせ、欠落したドメインの発生箇所を特定する。

## diagnostics.plugin_bundle_ratio

| プラットフォーム | ratio | 集計コマンド | 補足 |
|------------------|-------|--------------|------|
| linux-x86_64 | 0.98 | `collect-iterator-audit-metrics.py --section diagnostics --require-success --metric diagnostics.plugin_bundle_ratio --source reports/audit/phase2-7/diagnostics-domain-20261221.json` | Bundle 署名が欠落した 1 件は互換モード（`bundle.strict=false`）で実行したテストに由来。Phase 2-8 で廃止予定。 |
| macos-arm64 | 0.98 | `collect-iterator-audit-metrics.py --section diagnostics --require-success --metric diagnostics.plugin_bundle_ratio --platform macos-arm64 --source reports/audit/phase2-7/diagnostics-domain-20261221.json` | macOS CI でも同じ互換テストが影響。 |
| windows-msvc | 0.98 | `collect-iterator-audit-metrics.py --section diagnostics --require-success --metric diagnostics.plugin_bundle_ratio --platform windows-msvc --source reports/audit/phase2-7/diagnostics-domain-20261221.json` | Windows では `registry.mirror` テストを互換モードで実行。 |

`ratio < 0.95` の場合は `docs/notes/dsl-plugin-roadmap.md` のバンドル署名再発行手順に従い、互換モードを廃止する。

## diagnostics.effect_stage_consistency

| プラットフォーム | pass_rate | 集計コマンド | 補足 |
|------------------|-----------|--------------|------|
| linux-x86_64 | 1.0 | `collect-iterator-audit-metrics.py --section diagnostics --require-success --metric diagnostics.effect_stage_consistency --source reports/audit/phase2-7/diagnostics-domain-20261221.json` | `effect.stage.required` と `effect.stage.actual` が CLI/LSP/監査で一致。 |
| macos-arm64 | 1.0 | `collect-iterator-audit-metrics.py --section diagnostics --require-success --metric diagnostics.effect_stage_consistency --platform macos-arm64 --source reports/audit/phase2-7/diagnostics-domain-20261221.json` | LSP V2 コントラクトテストで Stage 差分なし。 |
| windows-msvc | 1.0 | `collect-iterator-audit-metrics.py --section diagnostics --require-success --metric diagnostics.effect_stage_consistency --platform windows-msvc --source reports/audit/phase2-7/diagnostics-domain-20261221.json` | Stage ミスマッチ発生時は `bootstrap-windows` の `iterator-audit` ステップで即時失敗。 |

逸脱が発生した場合は `collect-iterator-audit-metrics.py --section effects --require-success` の結果と突き合わせ、該当サンプルの `AuditEnvelope.metadata` を `reports/audit/phase2-7/effects/` で確認する。

## 更新手順

1. `dune runtest compiler/ocaml/tests/test_cli_diagnostics.ml` と `tooling/lsp/tests/client_compat` のゴールデンを更新し、`collect-iterator-audit-metrics.py --section diagnostics --require-success --write-json reports/audit/phase2-7/diagnostics-domain-<date>.json` を実行する。
2. 本ダッシュボードを更新し、比率や pass_rate が 0.95 未満になった場合は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md#diagnostic-domain-metrics` を再オープンする。
3. `reports/audit/index.json` に新しい診断監査ログのパスを追加し、`tooling/ci/tests/test_create_audit_index.py` を更新する。

## 閾値

| 指標 | 閾値 | エスカレーション先 |
|------|------|--------------------|
| `diagnostics.domain_coverage` | 0.95 以上 | `docs/plans/bootstrap-roadmap/0-4-risk-handling.md#diagnostic-domain-metrics` |
| `diagnostics.plugin_bundle_ratio` | 0.95 以上 | 同上 |
| `diagnostics.effect_stage_consistency` | 1.0 | `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §5 / `docs/notes/effect-system-tracking.md` |

閾値を満たせない場合は Phase 2-8 仕様監査で追加検証を計画し、復旧まで CI の `--require-success` を一時ブロッキングに設定する。
