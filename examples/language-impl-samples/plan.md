# Reml 言語実装サンプル：運用メモ

## 📊 現状サマリー

**実装済み**:
- Reml: 既存サンプル一式（`reml/`）

**対象外**:
- 他言語の比較実装は現在のリポジトリ構成では管理しない

---

## 🎯 整備タスクの方向性

1. **サンプルの仕様追従**
   仕様更新に合わせて Reml 実装の表記・API 呼び出しを同期し、`reml-improvement-matrix*.md` で根拠を追記する。

2. **テスト用シナリオの維持**
   `tooling/examples/run_examples.sh` の `language_impl_samples` スイートで参照されるサンプルが欠けないよう、追加・移動時は `phase4-scenario-matrix.csv` の経路を更新する。

3. **サンプルの拡充検討**
   仕様書の重点領域（診断、RunConfig、Async/Runtime）に合わせて新規サンプルを追加する際は、まず `reml-improvement-matrix-new-samples.md` に課題仮説を記録する。

---

## ✅ まとめ
本ディレクトリは Reml 実装サンプルのみに集中し、仕様検証と改善フィードバックの基盤として運用する。
