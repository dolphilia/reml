# 3.4 中間 IR と CodeGen 再実装計画

## 目的
- Phase 3 マイルストーン M4 を達成するため、Reml 実装で Core IR/MIR/LLVM IR 生成パイプラインを再構築し、OCaml 版との互換性を確保する。
- モノモルフィゼーションやターゲット別最適化を適用しやすい構造を整え、性能の回帰を防止する。

## スコープ
- **含む**: Core IR/MIR データ構造定義、変換パス、最小最適化の移植、LLVM IR 出力、ターゲット別 DataLayout 適用。
- **含まない**: 高度な最適化（レジスタ割り当て改善等）、JIT。必要に応じて Phase 4 以降。
- **前提**: Phase 1 の Core IR 設計と Phase 2 の型クラス・効果が Reml 実装へ引き継がれていること。

## 作業ブレークダウン
1. **IR データ構造移植**: Reml で Core IR/MIR のデータ型を定義し、OCaml 版と互換のシリアライズ形式を整備。
2. **変換パイプライン再構築**: TypedAST→Core IR→MIR→LLVM IR の各段を Reml で実装し、最小最適化を組み込む。
3. **モノモルフィゼーション統合**: Phase 2 の方針に従い、中間 IR 上で特殊化を実施し、キャッシュ戦略を設計。
4. **ターゲット適用**: クロスコンパイルタスクと連携し、ターゲットごとに DataLayout・TargetTriple を切り替える。
5. **出力検証**: LLVM IR の差分比較を自動化し、OCaml 版との差異を `notes/llvm-spec-status-survey.md` に記録。
6. **ローダブル成果物**: `--emit-ir` や `--emit-mir` オプションを Reml CLI に実装し、観測可能にする。

## 成果物と検証
- LLVM IR の差分が許容範囲内に収束し、差異理由が文書化される。
- MIR/LLVM のテストが CI で安定通過し、性能が Phase 2 ベースライン ±10% に収まる。
- 生成バイナリが主要ターゲットで実行できる。

## リスクとフォローアップ
- 差分が大きい場合は OCaml 版をフォールバックとして維持し、最適化戦略の再評価を行う。
- ターゲットごとの最適化が複雑になりすぎる場合、Phase 4 のリリースパイプラインでの整合チェックを強化する。
- モノモルフィゼーションのコード膨張を監視し、`0-3-audit-and-metrics.md` に統計を記録。

## 参考資料
- [3-0-phase3-self-host.md](3-0-phase3-self-host.md)
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)
- [notes/llvm-spec-status-survey.md](../../notes/llvm-spec-status-survey.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)

