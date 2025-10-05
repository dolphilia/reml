# 3.6 メモリ管理戦略評価計画

## 目的
- Phase 3 で予定されている RC 継続 vs GC 導入の評価を体系的に行い、Phase 3 終了時に採用方針を決定する。
- RC ツール整備・GC 候補調査・性能/メモリ比較を行い、`0-4-risk-handling.md` に意思決定の根拠を残す。

## スコープ
- **含む**: RC ツール（リーク検出・参照トレース）の整備、GC 候補（Boehm など）の PoC 統合、性能/メモリ測定、評価レポート。
- **含まない**: 本番 GC の完全実装、メモリモデルの仕様変更。必要に応じて Phase 4 以降。
- **前提**: Phase 1 で RC ベースのランタイムが完成しており、Phase 3 で Reml 実装に移植されている。

## 作業ブレークダウン
1. **RC ツール整備**: リーク・循環検出ツールを整備し、CLI 統合 (`--mem-trace`) を行う。
2. **計測ベンチマーク**: 代表ワークロードを選定し、メモリアロケーションと解放パターンを記録。
3. **GC PoC 統合**: Boehm GC など候補を取り込み、Reml コンパイラで動作する最小構成を実装。
4. **比較評価**: RC vs GC で性能/メモリ/実装複雑性を比較し、`0-3-audit-and-metrics.md` に結果を掲載。
5. **意思決定ミーティング**: M6 前に中間レビューを実施し、方針案をまとめる。
6. **ドキュメント化**: 結果を `notes/llvm-spec-status-survey.md` および新規メモ (必要に応じて `notes/memory-management-evaluation.md` 等) に残し、Phase 4 へ引き継ぐ。

## 成果物と検証
- RC ツールと GC PoC が CI で実行でき、測定データが収集される。
- 評価レポートが公開され、採用方針が決定。
- リスクとフォローアップが `0-4-risk-handling.md` に登録される。

## リスクとフォローアップ
- GC PoC の工数が大きい場合は対象範囲を最小限にし、評価軸を明確化。
- RC ツールが未成熟の場合、Phase 4 以降での継続開発を計画し、現段階では測定精度の限界を明示。
- 意思決定が遅れると Phase 4 の移行スケジュールに影響するため、レビューのタイムラインを早期に設定。

## 参考資料
- [3-0-phase3-self-host.md](3-0-phase3-self-host.md)
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)
- [notes/llvm-spec-status-survey.md](../../notes/llvm-spec-status-survey.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)

