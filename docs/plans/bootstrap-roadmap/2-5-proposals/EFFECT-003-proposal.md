# EFFECT-003 複数 Capability 解析計画

## 1. 背景と症状
- 効果プロファイルは複数の Capability を要求できるが、実装では `effect_capabilities` の先頭 1 件しか解析せず、`Type_inference_effect.resolve_function_profile` が最初の項目のみ `resolved_capability` に反映している（compiler/ocaml/src/type_inference_effect.ml:50-107）。  
- `AuditEnvelope.metadata` や `Diagnostic.extensions["effects"]` でも Capability が単一値として扱われ、仕様で定義された Stage 検査（docs/spec/1-3-effects-safety.md:236-303, docs/spec/3-8-core-runtime-capability.md:120-180）と整合しない。  
- `with_capabilities` や `@requires_capability` を複数指定した場合、監査ログが欠落し Phase 3 の Stage 契約保証が破綻する。

## 2. Before / After
### Before
- Capability リストから最初の要素を取り出して Stage 照合し、残りは破棄。  
- `AuditEnvelope.metadata` では `effect.stage.required` 等のキーも単一値のまま記録されるため、複数 Capability を要求する効果で監査情報が欠落する。

### After
- `resolve_function_profile` を改修し、Capability リスト全体を `resolved_capabilities : string list` として保持。`StageRequirement` を複数値に対応させ、すべての Capability について Stage を検証。  
- `AuditEnvelope.metadata` と `Diagnostic.extensions["effects"]` に `required_capabilities` / `actual_capabilities` 配列を出力し、`collect-iterator-audit-metrics.py` が複数値を検証できるようにする。  
- 仕様には現状の制限を脚注で追加し、実装完了後に脚注を解除する。

## 3. 影響範囲と検証
- **メトリクス**: `0-3-audit-and-metrics.md` に `effect.capability_array_pass_rate`（仮）を追加し、複数 Capability が監査ログに記録されているか CI で確認。  
- **診断**: `reports/diagnostic-format-regression.md` に複数 Capability を要求するケースを追加し、`scripts/validate-diagnostic-json.sh` で配列出力を検証。  
- **効果解析**: EFFECT-001 / EFFECT-002 のタグ検出と連携し、Capability 情報が残余効果計算に反映されるかテスト。
- **OCaml テスト**: `compiler/ocaml/tests/capability_profile_tests.ml`（新設）で `resolve_function_profile` が `StageRequirement::{Exact, AtLeast}` の複数値判定を保持するか確認し、失敗時の診断内容をスナップショット化する。

## 4. 実施ステップ
1. **Step 0: 効果プロファイル資産の棚卸し（Week32 Day1 実施） — 完了（2025-11-21）**  
   - `compiler/ocaml/src/type_inference_effect.ml`・`compiler/ocaml/src/effect_profile.ml`・`compiler/ocaml/docs/effect-system-design-note.md` を精査し、Capability 配列が保持される経路と `resolved_stage` / `stage_trace` が先頭要素に固定される箇所を整理。仕様参照（`docs/spec/1-3-effects-safety.md` §I, `docs/spec/3-8-core-runtime-capability.md` §1.2）と突き合わせてギャップを明確化した。  
   - `compiler/ocaml/src/diagnostic.ml`・`compiler/ocaml/src/diagnostic_serialization.ml`・`tooling/ci/collect-iterator-audit-metrics.py` の単一値前提を列挙し、監査メタデータと CI 指標が配列化されていない現状を記録。  
   - 調査結果を `docs/plans/bootstrap-roadmap/2-5-review-log.md` の「EFFECT-003 Week32 Day1 効果プロファイル棚卸し（2025-11-21）」として追記し、後続ステップの TODO（`resolved_capability` 廃止、配列主体への移行、`stage_trace` 拡張）を共有した。

2. **Step 1: Typer／効果プロファイルを多重 Capability 対応へ移行（Week32 Day2-3 予定） — 完了（2025-11-23）**  
   - `Effect_profile.make_profile` が `resolved_capabilities` を一次情報として扱い、必要に応じて主 Capability を派生させるよう再設計。`profile_primary_capability_*` ヘルパを追加して後続モジュールから配列経由で参照できるようにした。  
   - `Type_inference_effect.resolve_function_profile` は Capability 配列をそのまま解決し、Stage 判定と `stage_trace` へ Capability ごとのステップを記録。ミスマッチ検出では違反した Capability 名と Stage を `capability_stage_pairs` に集約し、`Type_error.effect_stage_mismatch_error` へ伝搬するよう改修。  
   - `constraint_solver`／`type_inference`／`core_ir/desugar`／`main`／`type_error` の各モジュールで単一値の `resolved_capability` 依存を整理し、配列ベースの API に合わせてメタデータ生成処理を更新。監査メタデータの主 Capability も配列から導出するよう統一。  
   - **確認**: `compiler/ocaml/tests/test_type_inference.ml` と `compiler/ocaml/tests/test_cli_diagnostics.ml` の前提条件を再読し、型推論と診断経路に後方互換性があることを手動確認（自動テストは未実行、Step 4 で網羅予定）。残課題として診断／監査フォーマットの配列化は Step 2 へ委譲。

3. **Step 2: 診断／監査出力の多重化（Week32 Day3-4 実施） — 完了（2025-11-29）**  
   - `Diagnostic.extensions["effects"]` と `AuditEnvelope.metadata` に `required_capabilities`・`actual_capabilities` 配列を追加し、`effect.stage.required_capabilities` / `effect.stage.actual_capabilities` を含む共通キーを CLI/LSP/監査経路へ伝播。`compiler/ocaml/src/diagnostic.ml`・`compiler/ocaml/src/main.ml`・`compiler/ocaml/tests/test_effect_residual.ml` を更新して単一 Capability 互換を維持しつつ配列主体へ移行した。  
   - 監査集計を担う `tooling/ci/collect-iterator-audit-metrics.py` に新フィールド検証を追加し、`scripts/validate-diagnostic-json.sh` へ効果系配列の存在チェックを実装。`reports/diagnostic-format-regression.md` の手順に従いゴールデン（CLI/LSP/監査）の再生成と整合確認を実施。  
   - `docs/spec/3-6-core-diagnostics-audit.md` §3.2 / `docs/spec/3-8-core-runtime-capability.md` §8 の命名規則を確認し、`effect.stage.capabilities` は互換目的で保持したうえで配列キーを追加。更新内容は `docs/plans/bootstrap-roadmap/2-5-review-log.md` に記録し、Phase 2-7 で参照する TODO（Capability 名正規化ポリシー）を継続。

4. **Step 3: RunConfig／lex シムとの統合（Week32 Day4-5 予定） — 完了（2025-12-03）**
   - `compiler/ocaml/src/parser_run_config.{ml,mli}` に `Effects` サブモジュールを追加し、`stage`・`registry_path`・`required_capabilities` キーを設定／除去できるユーティリティを整備。`Cli.Options.to_run_config` で CLI オプションを同ネームスペースへ反映するよう調整した。  
   - `compiler/ocaml/src/runtime_capability_resolver.ml` を拡張し、RunConfig 由来の Stage override と Capability ヒントを `resolve` が取り込むよう変更。RunConfig で指定した Capability は default stage で補完され、`stage_trace` に `source="run_config"` のステップを追加する。  
   - `compiler/ocaml/src/main.ml` で RunConfig 構築を解析前に実施し、Runtime resolver の結果を `Effects.set_required_capabilities` で RunConfig へ書き戻す導線を追加。Lex シム (`Core_parse_lex.Bridge.derive`) と併用しても `extensions["effects"]` が維持されることを手動確認した。

5. **Step 4: テスト・メトリクス整備とドキュメント更新（Week32 Day5-Week33 Day1 予定） — 完了（2025-12-06）**  
   - `compiler/ocaml/tests/capability_profile_tests.ml` を追加し、`StageRequirement::{Exact, AtLeast}` の両ケースで複数 Capability の解析結果と Stage トレースが保持されることを検証。`compiler/ocaml/tests/test_cli_diagnostics.ml` では CLI/LSP/Audit 出力の `required_capabilities` / `actual_capabilities` 配列を比較し、ゴールデンを再生成して複数値サンプルを追加。  
   - `tooling/ci/collect-iterator-audit-metrics.py` に `effect.capability_array_pass_rate` を実装し、`--require-success` の強制判定へ組み込み。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ同指標を登録し、DIAG-003 の `diagnostics.effect_stage_consistency` と役割分担（Stage ミスマッチ検出 vs. 配列欠落検証）を明示。  
   - `docs/spec/1-3-effects-safety.md` / `docs/spec/3-8-core-runtime-capability.md` に脚注を追加し、Phase 2-5 時点で複数 Capability 配列が実装前提になったことを記録。`docs/plans/bootstrap-roadmap/2-5-review-log.md` および `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の差分一覧へ反映し、Phase 2-7 への引き継ぎ事項を更新。

## 5. フォローアップ
- `docs/spec/3-8-core-runtime-capability.md` の Stage テーブルに複数 Capability の例を追加し、仕様変更時はここを基準に更新する。  
- Phase 2-7 の監査ダッシュボード更新タスクへ「複数 Capability に対応した可視化」を依頼。  
- Self-host 移行時に Reml 実装でも同様の配列出力が可能か確認する。
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に複数 Capability 表示の UI 要件を共有し、監査チームと共通チェックリストを作成する。
- **タイミング**: EFFECT-001 のタグ拡張完了直後（Phase 2-5 中盤）に着手し、Phase 2-5 の終了までに監査出力を複数値対応へ切り替える。

## 6. 残課題
- Capability 名の正規化（小文字化・ハイフン/アンダースコア統一）をどのレイヤで行うか、Runtime チームと調整が必要。  
- Stage 照合の失敗時にどの Capability を優先的に報告するか（最初/すべて）について運用ポリシーを決めたい。
