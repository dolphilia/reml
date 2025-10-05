# 3.5 ランタイムと Capability 統合計画

## 目的
- Phase 3 マイルストーン M5 で、Reml 実装のランタイムを構築し、`3-8-core-runtime-capability.md` の Stage 契約を反映する。
- ターゲットごとの Capability 差異 (POSIX/Windows/macOS) を `TargetCapability` モデルで吸収し、セルフホスト後のランタイムが仕様通りに動作するよう検証する。

## スコープ
- **含む**: Reml ランタイムモジュール (`Core.Runtime` 系) の実装、Stage/Capability 検証 API、FFI ラッパ、環境検出、監査ログ連携。
- **含まない**: 高度なスケジューラ、マルチスレッド実行、JIT 対応。必要に応じて Phase 4 以降。
- **前提**: Phase 2 の効果システム・FFI 拡張が安定し、クロスコンパイルでターゲット別成果物が生成できる。

## 作業ブレークダウン
1. **ランタイム API 整理**: Phase 1 の C ランタイムをベースに Reml から制御できる API を設計し、Capability 判定フックを設置。
2. **Stage 検証 API**: `verify_capability_stage` を実装し、コンパイル時/実行時に Stage 違反を検出する仕組みを統合。
3. **ターゲット差異吸収**: POSIX/Windows/macOS 間の差異を `TargetCapability` と条件付きコンパイルで扱い、FFI ラッパを提供。
4. **監査ログ連携**: Stage 判定結果を `AuditEnvelope` に記録し、CLI で確認可能にする。
5. **テスト整備**: 各ターゲットで Stage/Capability の整合テストを実行し、CI マトリクスに組み込む。
6. **ドキュメント更新**: ランタイム API の更新を `guides/runtime-bridges.md`、`3-8-core-runtime-capability.md` に反映。

## 成果物と検証
- 各ターゲットで Stage/Capability テストが通過し、監査ログに差分が無いこと。
- ランタイム API が Reml で利用でき、セルフホストビルドが実行時に問題なく動作する。
- 仕様ドキュメントがアップデートされ、レビュー済みであること。

## リスクとフォローアップ
- ターゲットごとの条件分岐が増え複雑化する可能性があるため、モジュール分割と自動生成を検討。
- Stage 要件が未定義の Capability が出現した場合、`0-4-risk-handling.md` へ登録し追跡。
- 将来の拡張 (WASM/WASI 等) を見据え、Capability モデルの拡張余地を残す。

## 参考資料
- [3-0-phase3-self-host.md](3-0-phase3-self-host.md)
- [3-8-core-runtime-capability.md](../../3-8-core-runtime-capability.md)
- [guides/runtime-bridges.md](../../guides/runtime-bridges.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)

