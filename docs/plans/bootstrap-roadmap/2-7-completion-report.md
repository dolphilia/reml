# Phase 2-7 診断パイプライン残課題・技術的負債整理 完了報告書

> 作成日: 2026-12-21  
> 担当: Phase 2-7 Diagnostics/Plugin/Effects 統合チーム

## 1. サマリー
- Windows/macOS CI に監査ゲート (`collect-iterator-audit-metrics.py --platform … --require-success`) を導入し、技術的負債 ID22/23 をクローズ。`ffi_bridge.audit_pass_rate` と `iterator.stage.audit_pass_rate` は全プラットフォームで 1.0 を維持している。
- CLI/LSP 両チャネルの `Diagnostic` フォーマッタを `SerializedDiagnostic` 経由に統合し、テキスト/JSON 出力差分を `reports/diagnostic-format-regression.md` で追跡できる状態を確立した。
- Streaming PoC と効果行統合タスクを進め、Packrat 共有や FlowController Auto の KPI を `reports/audit/dashboard/streaming.md` と `collect-iterator-audit-metrics.py --section effects` で監視可能にした。
- Unicode 識別子と効果行統合のローンチ条件を満たし、`lexer.identifier_profile_unicode = 1.0`、`type_row_mode = "ty-integrated"` を既定化した。
- Phase 2-8 に向けて診断ドメイン KPI（`diagnostics.domain_coverage`, `diagnostics.plugin_bundle_ratio`, `diagnostics.effect_stage_consistency`）のベースラインを整備し、`reports/audit/dashboard/diagnostics.md` と `0-3-audit-and-metrics.md` に記録した。

## 2. 達成事項
1. **監査ゲート整備と技術的負債クローズ**  
   - `collect-iterator-audit-metrics.py` に `--platform` オプションを追加し、Windows/MSVC と macOS/ARM64 向け監査ジョブを `bootstrap-windows.yml` / `bootstrap-macos.yml` に組み込んだ。  
   - `reports/iterator-stage-summary-*.md`、`reports/ffi-bridge-summary.md`、`reports/audit/index.json` を更新し、ID22/23 を `compiler/ocaml/docs/technical-debt.md` で完了扱いとした。
2. **CLI/LSP フォーマッタ統合**  
   - `Diagnostic_formatter`, `Json_formatter`, `diagnostic_serialization` を一本化し、空配列省略などの整形ルールを共有化。ゴールデン差分は `reports/diagnostic-format-regression.md` のチェックリストでレビューする運用に切り替えた。
3. **Streaming PoC 指標の可視化**  
   - Packrat 共有と FlowController Auto のテレメトリを `parser_driver.ml`, `parser_expectation.ml`, `streaming_runner_tests.ml` に導入し、`reports/audit/dashboard/streaming.md` を整備。`parser.stream.*` 系 KPI を Linux/macOS/Windows で 1.0 に維持。
4. **Unicode 識別子ローンチ**  
   - `REML_ENABLE_UNICODE_TESTS=1` を CI 既定とし、`lexer.identifier_profile_unicode = 1.0` を達成。関連脚注と差分ログを `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` に反映した。
5. **効果行統合のリリース判定**  
   - `type_row_mode` を `ty-integrated` 既定に切り替え、`diagnostics.effect_row_stage_consistency = 1.0`, `type_effect_row_equivalence = 1.0`, `effect_row_guard_regressions = 0` を確認。脚注撤去とリスククローズを実施した。
6. **診断ドメイン KPI のベースライン確立**  
   - `collect-iterator-audit-metrics.py --section diagnostics --require-success` を追加し、`reports/audit/phase2-7/diagnostics-domain-20261221.json` に集計値（domain_coverage 1.0 / plugin_bundle_ratio 0.98 / effect_stage_consistency 1.0）を保存。`reports/audit/dashboard/diagnostics.md` と `0-3-audit-and-metrics.md` に転記し、エスカレーション手順を定義した。

## 3. 未完了タスク（Phase 2-8 へ移管）
| ID/テーマ | 内容 | 追跡先 | 備考 |
|-----------|------|--------|------|
| SYNTAX-003 / EFFECT-002 H-O3〜H-O5 | 効果構文 Stage 監査の最終レビュー、互換モード終了可否の判断 | `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §8.2〜§8.3, `docs/notes/effects/effect-system-tracking.md` | Stage 監査ログと CLI/LSP の整合を Phase 2-8 で確認する。 |
| Plugin 互換モード廃止 | `diagnostics.plugin_bundle_ratio` を 1.0 に引き上げるため、互換モードテスト（`bundle.strict=false`）を更新 | `reports/audit/dashboard/diagnostics.md`, `docs/notes/dsl/dsl-plugin-roadmap.md` | Phase 2-8 で互換モード削除と署名再発行を実施。 |
| Spec 差分最終統合 | 差分リストと監査ログを Phase 2-8 仕様監査に統合 | `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md`, `docs/plans/bootstrap-roadmap/2-7-to-2-8-handover.md` | 監査用 TODO ノート（`docs/notes/process/spec-integrity-audit-checklist.md`）の開設が必要。 |

## 4. メトリクス
| 指標 | 値 | 備考 |
|------|----|------|
| `ffi_bridge.audit_pass_rate` | 1.0（Linux/macOS/Windows） | `collect-iterator-audit-metrics.py --platform … --require-success` を CI に統合。 |
| `iterator.stage.audit_pass_rate` | 1.0（Linux/macOS/Windows） | Stage メタデータ欠落時は CI で即時失敗。 |
| `diagnostics.domain_coverage` | 1.0 | `reports/audit/dashboard/diagnostics.md` 参照。 |
| `diagnostics.plugin_bundle_ratio` | 0.98 | 互換モードテストが残存。Phase 2-8 で 1.0 へ改善予定。 |
| `diagnostics.effect_stage_consistency` | 1.0 | Stage 整合性を CLI/LSP/監査ログで確認済み。 |
| `lexer.identifier_profile_unicode` | 1.0 | Unicode プロファイルを既定化。 |
| `diagnostics.effect_row_stage_consistency` | 1.0 | 効果行統合後の監査ゲートで確認。 |

## 5. フォローアップ
1. Phase 2-8 キックオフまでに `reports/audit/dashboard/diagnostics.md` を週次更新し、互換モード撤去時の閾値推移を記録する。
2. 効果構文 Stage 監査（H-O3/H-O5）を完了させ、`docs/spec/1-1-syntax.md` から残りの脚注を撤去する審査を実施する。
3. `collect-iterator-audit-metrics.py` の diagnostics セクションを Phase 3 CI にも組み込み、セルフホスト移行後も同指標を継続監視する。

## 6. 添付・参照
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`
- `docs/plans/bootstrap-roadmap/2-7-to-2-8-handover.md`
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`
- `docs/plans/bootstrap-roadmap/0-4-risk-handling.md`
- `reports/audit/dashboard/diagnostics.md`, `reports/audit/dashboard/streaming.md`
- `reports/audit/phase2-7/diagnostics-domain-20261221.json`
