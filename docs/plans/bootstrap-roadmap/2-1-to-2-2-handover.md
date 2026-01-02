# 2-1 → 2-2 ハンドオーバー

**作成日**: 2025-10-16  
**担当**: Phase 2 型クラス戦略チーム → 効果システム統合チーム

## 1. 概要
- Phase 2-1（型クラス戦略）は辞書渡し実装・モノモルフィゼーション PoC・統合テストを完了し、効果システム統合（2-2）に進むための下準備を整えた。
- `docs/plans/bootstrap-roadmap/2-2-effect-system-integration.md` の着手前チェックリストはすべて ✅ 済。静的ベンチ比較と iterator 監査パイプラインの試験運用を終え、記録を `0-3-audit-and-metrics.md` へ残している。

## 2. 主要成果物
- **辞書渡し実装**: `type_inference.ml`, `constraint_solver.ml`, `core_ir/desugar.ml`, `llvm_gen/codegen.ml` の更新で辞書生成・挿入・呼出まで完了。統合テスト 182 件 + 追加テスト成功。
- **モノモルフィゼーション PoC**: `core_ir/monomorphize_poc.ml` および CLI オプション `--typeclass-mode=both` を実装。辞書経路との IR 差分をゴールデン化済み。
- **静的比較フロー**: `compiler/ocaml/scripts/benchmark_typeclass.sh --static-only` により、辞書渡し／モノモルフィゼーションの IR 行数・ビットコード・バイナリサイズを JSON で出力。現在は while/for 未実装のため実測値は 0 だが、Phase 3 で即再計測可能。
- **監査メトリクス突合せ**: `tooling/ci/collect-iterator-audit-metrics.py` と新規 `tooling/ci/sync-iterator-audit.sh` を組み合わせ、`iterator.stage.audit_pass_rate` と `verify_llvm_ir` ログを Markdown サマリー化。
- **仕様更新**: `docs/spec/1-2-types-Inference.md`, `3-1-core-prelude-iteration.md`, `3-8-core-runtime-capability.md` に Stage 監査連携を追記。

## 3. 未完了タスク（フォローアップ）
| 項目 | 内容 | 推奨タイミング |
|------|------|----------------|
| 静的ベンチ拡充 | `benchmarks/micro_typeclass.reml` に静的比較専用ユーティリティを追加し、IR/BC が 0 にならないようにする | Phase 2-2 序盤 |
| 辞書診断強化 | `AmbiguousImpl` 発生時に `Diagnostic.extensions.typeclass.candidates` を埋め、UX を向上 | Phase 2-2 中盤 |
| while/for 実装 | Core IR ブロック生成リファクタリング案を確定し、Phase 3 でループベンチを復旧 | Phase 3 着手前レビュー |

詳細は `docs/notes/backend/loop-implementation-plan.md` と `docs/notes/types/typeclass-benchmark-status.md` の TODO セクションを参照。

## 4. 参照ログ・アーティファクト
- 静的比較: `compiler/ocaml/scripts/benchmark_typeclass.sh --static-only` 実行 → `compiler/ocaml/benchmark_results/static_comparison.json`（次回実行時は CI アーティファクトとして保存）。
- 監査サマリー: `tooling/ci/sync-iterator-audit.sh --metrics /tmp/iterator-audit.json --verify-log /tmp/verify.log --output /tmp/iterator-summary.md`
- 記録反映: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` 2025-10-16 項目を確認。

## 5. 2-2 着手時の確認事項
1. `docs/plans/bootstrap-roadmap/2-2-effect-system-integration.md` のチェックリストに従い、静的比較と監査サマリーを最新化する（Phase 2-2 作業前に再実行推奨）。
2. 効果タグ解析と Stage 判定を導入する際は、型クラス側で既に出力している `effect.stage.*` と整合が取れているか `collect-iterator-audit-metrics.py` を併用して検証する。
3. 新規仕様追記が生じた場合は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の記録フォーマットを更新し、Stage/Capability メトリクスを同じ表で扱えるようにする。

## 6. 連絡先・レビュア
- **型クラス実装担当**: compiler/ocaml チーム (typer, core_ir)
- **診断・監査担当**: tooling/ci & diagnostics チーム
- **仕様整合担当**: docs/spec Chapter 1 / 3 編集チーム

## 7. 付記
- `benchmark_typeclass.sh --static-only` で得られる値は while/for 実装完了まで 0 のままになる見込み。Phase 3 でループを有効化し次第、ベンチマークを通常モードへ戻す計画。ハンドラ導線が完成した段階で再計測をスケジューリングすること。
