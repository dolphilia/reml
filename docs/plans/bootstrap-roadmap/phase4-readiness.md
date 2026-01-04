# Phase 4 Readiness Draft

> この文書は Phase 4 M4（ハンドオーバー判定）へ向けたドラフトです。`phase4-scenario-matrix.csv` の進捗と `.reml` 実行結果を集約し、Phase 5 Self-host チームが必要とする判断材料を記録します。

## 1. 実行スイート状況（速報値）

| Suite | 実行件数 | Pass | Fail | Coverage | 備考 |
|-------|---------|------|------|----------|------|
| spec-core | 24 | 18 | 6 | 75% | Chapter 1 のタプル束縛（CH1-LET-002）が落ちており `impl_fix` に分類。Unicode 境界（CH1-LET-003）は `spec_fix` 扱い。 |
| practical | 12 | 11 | 1 | 91% | `core_io` シナリオ（CH3-IO-101）が基準通り。残り 1 件は Capability Stage の要求が pending。 |
| integration | 6 | 6 | 0 | 100% | CLI + Plugin の smoke セット。 |
| selfhost-smoke | 3 | 2 | 1 | 67% | Self-host 対策の準備中、Phase 5 着手前に再評価。 |

## 2. Chapter 1 variant coverage

| 規則 (spec_anchor) | canonical | alternate | boundary | invalid | variant_coverage | 備考 |
|--------------------|-----------|-----------|----------|---------|------------------|------|
| `docs/spec/1-1-syntax.md§4.2` (`let` 束縛) | ✅ (CH1-LET-001) | ⚠️ (CH1-LET-002 Fail) | ⚠️ (CH1-LET-003 Spec Fix) | ✅ (CH1-EFFECT-004) | 0.75 | alternate/boundary が pending。 |
| `docs/spec/1-3-effects-safety.md§3.1` (`effect handler`) | ✅ | ✅ | ⛔（未準備） | ✅ | 0.75 | `boundary` バリエーション追加が必要。 |

> KPI 目標: `variant_coverage == 1.0`。現状不足している規則は 2 件で、追加 `.reml` を 1 週間以内に作成予定（CH1-EFFECT-005, CH1-EFFECT-006 仮）。

## 3. spec_vs_impl_decision の現状

| scenario_id | spec_vs_impl_decision | 判定理由 | 次アクション |
|-------------|----------------------|-----------|--------------|
| CH1-LET-002 | impl_fix | Rust 実装がタプル束縛の Span を誤る。 | `compiler/frontend/src/parser/let_binding.rs` 修正、E2E の snapshot 更新。 |
| CH1-LET-003 | spec_fix | Unicode 正規化の扱いが仕様に未掲載。 | `docs/spec/1-1-syntax.md` §4.2 に NFC/NFKC の脚注を追加。 |
| CH1-EFFECT-004 | ok | 診断 JSON が仕様どおり。 | 追加作業なし。 |
| CH3-IO-101 | ok | Capability Stage と性能値が基準内。 | `practical` suite を毎日実行し Golden を凍結。 |

## 4. KPI snapshot

- `spec.chapter1.pass_rate`: 0.75（目標 0.90）。`impl_fix` と `spec_fix` を解決すれば 0.92 まで上がる見込み。
- `spec.chapter1.variant_coverage`: 0.75（目標 1.0）。境界ケースの追加が必須。
- `spec.chapter1.boundary_pass_rate`: 0.50（目標 0.90）。Unicode 境界ケースが `spec_fix` 待ちのため。
- `practical.pass_rate`: 0.91（目標 0.90）— 目標クリア。
- `practical.stage_mismatch`: 0（目標 0）。
- `practical.impl_fix_ratio`: 0.0、`practical.spec_fix_ratio`: 0.09（1/11）。
- `core_prelude.guard.failures`: 2 件（`ensure_not_null` のテスト強化により暫定発生）。

## 5. 既知リスクとフォローアップ

1. **variant_coverage 未達**: Chapter 1 の複数規則で `variant=boundary` が不足。`phase4-scenario-matrix.csv` に `missing_variant` タグを付与済み。担当: Core Spec チーム、期限: 1 週間。
2. **Unicode 仕様の明文化遅延**: `spec_fix`（CH1-LET-003）が Phase 4 exit のブロッカー。`docs/spec/1-1-syntax.md` 改訂案を準備し、レビューへ回す。
3. **Self-host smoke の安定化**: `selfhost-smoke` suite に pending 1 件。Phase 5 の前提となるため、`4-2` のランナー改善と合わせて解決する。

## 6. Phase 5 への要求事項（ドラフト）

- Chapter 1 variant coverage が 1.0 になるまで Phase 5 Stage 0 着手を保留。完了後に `phase4-readiness.md` を更新し署名。
- `.reml` ログと `phase4-scenario-matrix.csv` の `spec_vs_impl_decision` 列を Phase 5 の self-host regression tracker へ取り込むため、CSV を週次で freeze する。

## 7. 参考リンク

- `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv`
- `reports/spec-audit/ch5/spec-core-dashboard.md`（variant coverage グラフ）
- `reports/spec-audit/ch5/practical-suite-20250115.md`
- `docs/plans/bootstrap-roadmap/4-0-phase4-migration.md#405b-reml-実行による仕様検証ガイドライン`
