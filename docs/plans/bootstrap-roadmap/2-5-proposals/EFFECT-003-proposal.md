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

## 4. フォローアップ
- `docs/spec/3-8-core-runtime-capability.md` の Stage テーブルに複数 Capability の例を追加し、仕様変更時はここを基準に更新する。  
- Phase 2-7 の監査ダッシュボード更新タスクへ「複数 Capability に対応した可視化」を依頼。  
- Self-host 移行時に Reml 実装でも同様の配列出力が可能か確認する。
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に複数 Capability 表示の UI 要件を共有し、監査チームと共通チェックリストを作成する。
- **タイミング**: EFFECT-001 のタグ拡張完了直後（Phase 2-5 中盤）に着手し、Phase 2-5 の終了までに監査出力を複数値対応へ切り替える。

## 残課題
- Capability 名の正規化（小文字化・ハイフン/アンダースコア統一）をどのレイヤで行うか、Runtime チームと調整が必要。  
- Stage 照合の失敗時にどの Capability を優先的に報告するか（最初/すべて）について運用ポリシーを決めたい。
