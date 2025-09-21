# Phase 0 Gap Analysis

横断テーマごとに、現行仕様で触れられている内容と未整備の領域を整理したメモ。該当箇所の確認には `scenario-priorities.md` と既存仕様章 (`1-*`, `2-*`, `README`, `scenario-requirements.md`) を参照している。

---

## 1. 型安全な設定／構成 DSL
- **既存の記述**
  - `1-1-syntax.md` の `B.6` に `schema` 構文・条件付き束縛・テンプレート展開が正式化。
  - `2-7-config.md` がスキーマ API・差分検証・テンプレート適用を網羅。
- **残課題**
  - `guides/config-cli.md` へさらなる事例（大規模差分、承認フローのベストプラクティス）を追加。
  - `SchemaDiff` の互換性マトリクス（破壊的変更と安全変更の分類）をフェーズ2で整理。

## 2. 高精度なエラー診断とリカバリ支援
- **既存の記述**
  - `2-5-error.md` が `ErrorDomain`・`SeverityHint`・`ChangeSetRef` を導入し、`toStructuredLog` を更新済み。
  - LSP/CLI/監査ガイド（`guides/lsp-integration.md`, `guides/config-cli.md`）が同じ JSON を使用。
- **残課題**
  - `reml-run` / `reml-data` CLI の出力例に `severity_hint` を反映したサンプルを追加。
  - `RuntimeMetrics` との関連（致命的エラー時の自動ロールバック指標）をフェーズ2で検討。

## 3. モジュール化された拡張ポイント（DSL プラグイン）
- **既存の記述**
  - `1-1-syntax.md` の `B.7` と `2-2-core-combinator.md` の Capability 表が整合。
  - `guides/DSL-plugin.md` が CLI (`reml-plugin`) と登録フローを記載。
- **残課題**
  - プラグイン署名の失効/更新手順を `guides/DSL-plugin.md` に図示。
  - Capability バンドル（複数 Capability をまとめて要求）の設計をフェーズ2で評価。

## 4. ツール／エコシステムとのシームレスな連携
- **既存の記述**
  - LSP ガイド・Runtime Bridges・Config CLI ガイドが `audit_id`／`change_set`／`severity_hint` を共有。
  - `2-6-execution-strategy.md` に `RunConfig` オプション・ログ出力の整理が進行中。
- **残課題**
  - `README.md` にガイド一覧と横断テーマをリンクする目次を追加。
  - `reml-data` CLI のワークフローガイドを新設し、`guides/data-model-reference.md` と接続。

## 5. 型システム拡張とデータモデリング
- **既存の記述**
  - `1-2-types-Inference.md` の `J` で Tensor/Column/Schema/Resource/EffectSet を定義。
  - `2-7-config.md`・`2-8`（計画中）と整合する差分制約が導入済み。
- **残課題**
  - `Core.Data` 章のドラフト（2-8）が未着手。
  - `ResourceOps` の Capability と Effect のマッピングを追加で文書化。

## 6. 実行基盤との橋渡し（FFI／ランタイム連携）
- **既存の記述**
  - `1-3-effects-safety.md` の `K` で効果タグ・監査義務・ホットリロード指針を定義。
  - `guides/runtime-bridges.md` が `audit_id` / `change_set` とホットリロード CLI の連携を説明。
- **残課題**
  - GPU/組み込み向けの詳細なフェイルセーフシーケンス例を追加。
  - `RunConfig` の JSON スキーマを付録化し、CLI 間で再利用できる形にする。

---

## 付記
- `scenario-requirements.md` に進捗トラッキング用の欄を追加するタスクは未実施 → フェーズ1の冒頭で対応予定（✅ 要追加）。
- 既存仕様の細部変更はこれ以降のフェーズで順番に反映する。
