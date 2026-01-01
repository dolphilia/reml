# 2-4 診断・監査パイプライン 週次ログ（2025-10-24）

> 対象タスク: [2-4 診断・監査パイプライン強化計画](2-4-diagnostics-audit-pipeline.md)  
> 直近参照: [2-3 → 2-4 ハンドオーバー](2-3-to-2-4-handover.md)、[技術的負債リスト](../../../compiler/ocaml/docs/technical-debt.md)

## 1. 現状サマリー

### 1.1 完了済み・進行中の項目
- シリアライズ統合セクションの詳細化を文書側で完了（2025-10-24）。共通レイヤ導入、JSON/テキスト/LSP の三系統を同時に扱うタスク分解と完了条件を明文化。
- Phase 2-3 で引き継いだ `ffi_bridge.audit_pass_rate` / `iterator.stage.audit_pass_rate` のゲート条件は未導入。監査ログ運用は計画立案段階。
- LSP 互換テスト（`tooling/lsp/tests/client_compat`）は雛形のみ存在し、フィクスチャと JSON Schema 検証の手動実行パスが準備されている。
- FFI/効果/プラットフォーム差分フィクスチャ（`diagnostic-v2-ffi-*.json`, `diagnostic-v2-effects-sample.json`）を追加し、`npm run ci` でカバレッジを確認可能。
- `compiler/ocaml/src/diagnostic_serialization.ml(.mli)` を新設し、CLI/LSP/CI 共有を想定した中間表現と JSON 変換ユーティリティの骨格を実装。
- `scripts/validate-diagnostic-json.sh` と `tooling/lsp/tests/client_compat/validate-diagnostic-json.mjs` を追加し、AJV によるスキーマ検証フローを整備（`npm install` 前提）。Linux/Windows/macOS の CI ワークフローに `npm run ci` とスキーマ検証ステップを組み込んだ。
- `reports/diagnostic-format-regression.md` を作成し、JSON 出力変更時の差分レビュー手順を整理。
- `compiler/ocaml/src/cli/json_formatter.ml` と `tooling/lsp/diagnostic_transport.ml` を `diagnostic_serialization` ベースへリファクタリングし、`tooling/lsp/lsp_transport.ml`・`tooling/lsp/jsonrpc_server.ml` で V1/V2 変換パイプラインを整備。

### 1.2 リスク・ブロッカー
- 共通シリアライズ層は導入済みだが、テキスト出力（`--json-mode` など）とゴールデン差分の運用指針が未確立であり、互換性破壊リスクが残る。
- V1 互換レイヤは最小実装のため、既存クライアントで必要な補助データ（structured hints など）をどこまで保持するか精査が必要。
- JSON 検証ジョブは追加済みだが、エラー検出時のレビュー手順（失敗時のログリンク、差分比較ツール）を未整備のまま放置すると運用コストが高止まりする。

## 2. 週次ログ / スプリントトラッカー

| 週 | 期間 | 状態 | 主なアクション | 備考 |
| --- | --- | --- | --- | --- |
| 27 | 2025-10-24〜 | Kickoff | シリアライズ統合タスクの分解とドキュメント反映。LSP V2 現況レビューを実施し、欠落モジュールを洗い出し。 | CI 自動化・共通レイヤ実装は未着手。 |
| 28 | 2025-10-31〜 | Planned | 共通シリアライズレイヤ実装と JSON スキーマ検証スクリプト整備。 | `scripts/validate-diagnostic-json.sh` 追加予定。 |
| 29 | 2025-11-07〜 | Planned | LSP V2 互換レイヤ分割、`lsp-contract` CI ジョブ草案、クライアントフィクスチャ拡充。 | `tooling/lsp/lsp_transport.mli` / `tooling/lsp/jsonrpc_server.ml` 作成が前提。 |
| 30 | 2025-11-14〜 | Planned | CLI/LSP 統合テスト拡張と `ffi_bridge.audit_pass_rate` ゲート導入。 | Windows/macOS ワークフロー改修が必要。 |

## 3. LSP V2 連携 現況レビュー（2025-10-24）

### 3.1 確認済み資産
- `tooling/lsp/diagnostic_transport.ml` は V2 スキーマ（`schema_version = "2.0.0-draft"`）準拠で LSP PublishDiagnostics を生成。`extensions` / `audit_metadata` / `structured_hints` を JSON に反映する実装が存在。
- `tooling/lsp/lsp_transport.mli`・`tooling/lsp/compat/diagnostic_v1.ml` を追加し、V1/V2 分離前提の API 骨格を配置。
- `tooling/lsp/tests/client_compat/` には Vitest ベースの互換テスト雛形があり、`diagnostic-sample.json`（V1）、`diagnostic-v2-sample.json`（V2）に加えて FFI 監査ケース `diagnostic-v2-ffi-sample.json` が利用可能。
- `tooling/json-schema/diagnostic-v2.schema.json` がドラフト状態で配置済み。AJV を用いた検証は `client-v2.ts` 内で呼び出し可能な構造になっている。

### 3.2 計画との差異と着手条件
- `tooling/lsp/tests/client_compat` のフィクスチャは FFI ケースを追加済みだが、効果診断や Windows/macOS 固有サンプルが未登録。`npm run ci` を GitHub Actions に追加済みだが、カバレッジ拡張と差分レポート化が未整備。
- CLI 側の `--json-mode` フラグは未実装。共通シリアライズモジュール導入後のテキスト出力（スニペット無効化など）切替フローが残課題。
- `scripts/validate-diagnostic-json.sh` は Linux/Windows/macOS ワークフローに組み込んだが、生成物アーティファクトと連動した自動レビュー手順は未設定。

### 3.3 初動タスク（提案）
1. `tooling/lsp/tests/client_compat/fixtures/` に効果診断・プラットフォーム差分サンプル（`effects.contract.stage_mismatch` など）を追加し、AJV スキーマ検証の網羅性を高める。
2. `scripts/validate-diagnostic-json.sh` の結果を CI サマリ／アーティファクトへ連動させ、差分検出時のレビューフローを整備する。
3. CLI 側で `--json-mode` を実装し、テキスト出力（スニペット抑制等）を含めたフォーマッタ切替を `diagnostic_serialization` ベースで統合する。

## 4. TODO / フォローアップメモ
- [ ] 共通シリアライズ層導入前に CLI/LSP 双方の現在の JSON 出力を `reports/diagnostic-format-regression.md`（作成予定）へ記録する。
- [ ] `tooling/ci/sync-iterator-audit.sh` への LSP JSON スキーマ検証フック追加案を検討する。
- [ ] LSP V2 で追加される `structured_hints` の LSP `command` 変換仕様を `docs/guides/dsl/plugin-authoring.md` へ反映する。

## 5. 参考資料
- `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md`
- `docs/plans/bootstrap-roadmap/2-3-to-2-4-handover.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/guides/ecosystem/ai-integration.md`
- `tooling/json-schema/diagnostic-v2.schema.json`
- `tooling/lsp/tests/client_compat/README.md`
