# 0.4 docs-examples-audit への復帰手順

## 復帰条件
- `reml_frontend` を含む主要バイナリの再ビルドが成功している。
- `Cargo.lock` の差分と更新理由が記録されている。

## 復帰後に実施するタスク
1. `docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251223.md` のフェーズ 3 を再開する。
2. `--allow-top-level-expr` など再検証に必要な CLI オプションが最新バイナリで有効か確認する。
3. サンプル復元の再検証を実施し、`reports/spec-audit/summary.md` と `reports/spec-audit/ch1/docs-examples-fix-notes-YYYYMMDD.md` を更新する。

## 復帰時の参照ファイル
- `docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251223.md`
- `reports/spec-audit/summary.md`
- `reports/spec-audit/ch1/docs-examples-fix-notes-20251223.md`
