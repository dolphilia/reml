# 0.0 パターンマッチ強化計画 概要

## 背景
- `docs/notes/pattern-matching-improvement.md` で整理した課題（Active Patterns 不在、Or/Slice/Range パターン不足等）が Phase 4 の仕様・実装回収を阻害しつつある。
- `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` を進める際、パターンマッチ周辺の仕様不足がテスト整備や網羅性確認を遅延させている。
- Reml の価値観（`docs/spec/0-1-project-purpose.md`）に基づき、DSL ファーストで安全な記述を強化するために専用の計画群を分離する。

## 目的
1. Active Patterns を中心としたパターンマッチ拡張仕様を確定し、`docs/spec/1-1-syntax.md` / `1-5-formal-grammar-bnf.md` に落とし込む道筋を作る。
2. Or/Slice/Range/Binding/Regex など周辺機能を優先度付きで計画化し、実装・検証の順序を Phase 4 の進行と競合しない形で定義する。
3. 仕様更新に伴う実装・サンプル・診断資産（`examples/`、`reports/spec-audit/ch4/` 等）の更新フローを明示し、回帰計画と衝突しないようにする。

## スコープ
- **含む**: パターンマッチ構文・型/効果セマンティクスの拡張、網羅性・重複マッチ診断の設計、サンプル/テスト追加計画、リリース手順案。
- **含まない**: 実装作業自体（Rust/OCaml コード改修）、既存フェーズの KPI 変更、他機能（演算子、型推論以外）の仕様議論。

## 成果物
- Active Patterns 導入に関する詳細計画（`1-0-active-patterns-plan.md`）
- 周辺機能拡張計画（`1-1-pattern-surface-plan.md`）
- 仕様差分の反映先一覧と影響範囲メモ（各計画書内に記載）

## 進行フェーズ
- **Phase A: 仕様確定整備**  
  Active Patterns / Or / Slice / Range などの構文・BNF・診断ポリシーを整理し、既存仕様との整合チェックポイントを定義。
- **Phase B: アセット設計**  
  例題・スナップショット・テストケースの追加方針を決め、`examples/spec_core/chapter1/match_expr/` など既存資産との重複/欠落を整理。
- **Phase C: ロールアウトと回帰統合**  
  Phase 4 回帰計画との統合ポイント（`phase4-scenario-matrix.csv` 等）を決め、CI/レポート更新手順を準備。

## リスクと対応
- **仕様衝突**: 既存の `match` ガード/エイリアス構文と Active Patterns 記法の優先順位が衝突する可能性 → `docs/spec/1-5-formal-grammar-bnf.md` で優先順位表を更新し、パース規則の回帰チェックを必須化。
- **網羅性検査への影響**: Or/Slice/Range 追加で網羅性計算が複雑化 → 診断の最小実装（警告のみ）と完全実装（エラー化）の二段階を計画し、Phase C で段階的に有効化。
- **スケジュール干渉**: Phase 4 既存タスクとの競合 → 変更は `spec_fix` / `impl_fix` ラベルで明示し、`docs/plans/bootstrap-roadmap/` に事前通知する。また、新設される診断キーや構文ルールは必ず `phase4-scenario-matrix.csv` に新規シナリオ行として登録し、`4-1-spec-core-regression-plan.md` のトラッカーで監視する。

## 参照資料
- 主要メモ: `docs/notes/pattern-matching-improvement.md`
- 仕様: `docs/spec/1-1-syntax.md`, `docs/spec/1-5-formal-grammar-bnf.md`, `docs/spec/1-2-types-Inference.md`
- ガイド: `docs/guides/compiler/core-parse-streaming.md`（Parse 系との整合確認）
