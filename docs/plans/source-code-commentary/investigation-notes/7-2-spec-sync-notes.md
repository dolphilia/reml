# 調査メモ: 第23章 仕様との同期

## 対象モジュール/資料

- `docs/notes/process/spec-integrity-audit-checklist.md`
- `docs/guides/tooling/audit-metrics.md`
- `tooling/json-schema/validate-diagnostic-json.sh`
- `tooling/ci/ci-validate-audit.sh`
- `tooling/ci/create-audit-index.py`
- `tooling/ci/verify-audit-metadata.py`
- `tooling/review/audit-diff.py`
- `docs/plans/bootstrap-roadmap/checklists/doc-sync-text.md`
- `docs/notes/process/docs-update-log.md`
- `docs/notes/process/guides-to-spec-integration-plan.md`
- `docs/notes/process/examples-regression-log.md`
- `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md`
- `docs/guides/README.md`

## 入口と全体像

- 仕様と実装の同期は、`reports/spec-audit/` にエビデンスを蓄積し、`rust-gap` を追跡しつつ差分を解消するプロセスとして定義されている。監査ベースラインや差分メモの保存先まで明記され、Phase 2-8 の監査タスクと連動している。  
  - `docs/notes/process/spec-integrity-audit-checklist.md:5-49`
- 監査・診断の KPI は `reports/` と `docs/` のリンク維持を目的に定義され、スキーマ・KPI・Run ID を記録する運用ルールが書かれている。  
  - `docs/guides/tooling/audit-metrics.md:5-66`
- ガイド → 仕様の統合計画があり、API 契約がガイドに残っている場合は `docs/spec/` に移植する方針が明文化されている。  
  - `docs/notes/process/guides-to-spec-integration-plan.md:3-85`
- ドキュメント更新ログとチェックリストは、章単位の同期を継続運用するための履歴・検査表として活用されている。  
  - `docs/plans/bootstrap-roadmap/checklists/doc-sync-text.md:1-17`
  - `docs/notes/process/docs-update-log.md:1-21`

## 診断/監査の検証ツール

- 診断 JSON のスキーマ検証は `tooling/json-schema/validate-diagnostic-json.sh` が入口で、対象ディレクトリを `tests/`・`expected/`・`reports/` から収集する。  
  - `tooling/json-schema/validate-diagnostic-json.sh:1-108`
- 監査ログのスキーマ検証は `tooling/ci/ci-validate-audit.sh` が担当し、JSON/JSONL の各行を AJV で検証する。  
  - `tooling/ci/ci-validate-audit.sh:1-150`
- 監査インデックスは `tooling/ci/create-audit-index.py` で生成され、`build_id` と `path`、`pass_rate` を含むエントリを出力する。  
  - `tooling/ci/create-audit-index.py:1-173`
- 監査インデックスの整合性確認は `tooling/ci/verify-audit-metadata.py` が行い、`bridge.*` / `effect.*` / `typeclass.*` / `parse.*` の必須キーを検証する。  
  - `tooling/ci/verify-audit-metadata.py:1-117`

## レビュー/差分の入口

- 監査ログ差分は `tooling/review/audit-diff.py` が生成し、診断の増減・メタデータ変更・pass_rate 変化を出力する。  
  - `tooling/review/audit-diff.py:1-200`
- 仕様回帰の実行記録は `reports/spec-audit/` と `docs/notes/process/examples-regression-log.md` に残され、CLI 実行ログや期待診断を保存している。  
  - `docs/notes/process/examples-regression-log.md:1-101`
- Phase 4 の spec_core/practical 回帰は `phase4-scenario-matrix.csv` と `reports/spec-audit/ch5/*.md` を基準に追跡する計画になっている。  
  - `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md:1-160`

## ガイドと仕様の同期

- ガイドの更新時は `docs/spec/` と整合するよう維持し、`docs/README.md` とルート README も更新するルールが明記されている。  
  - `docs/guides/README.md:55-56`

## TODO / 不明点

- `docs/guides/tooling/audit-metrics.md` が参照する `scripts/validate-diagnostic-json.sh` はリポジトリ内に存在せず、実体は `tooling/json-schema/validate-diagnostic-json.sh`。同じく `tooling/ci/collect-iterator-audit-metrics.py` / `tooling/ci/sync-iterator-audit.sh` も現状見つからない。  
  - `docs/guides/tooling/audit-metrics.md:11-58`
- `reports/spec-audit/README.md` はチェックリストに登場するが、実ファイルは未確認（`reports/spec-audit/` 直下に README がない）。  
  - `docs/notes/process/spec-integrity-audit-checklist.md:5-12`
