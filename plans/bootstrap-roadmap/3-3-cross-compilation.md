# 3.3 クロスコンパイル機能実装計画

## 目的
- Phase 3 マイルストーン M3 を達成するため、`notes/cross-compilation-spec-update-plan.md` の Phase A〜C を Reml セルフホスト実装へ組み込み、主要ターゲット (x86_64 Linux/Windows, ARM64 macOS) をサポートする。
- ターゲットプロファイル (`RunConfigTarget`) と `@cfg` キーを整備し、CLI で `reml build --target <profile>` を実行可能にする。

## スコープ
- **含む**: ターゲット構成管理、`TargetCapability` 定義、環境検出、ターゲット別標準ライブラリ配布、CI マトリクス化。
- **含まない**: 新規ターゲット (WASM 等)、高度な最適化。Phase 4 以降で検討。
- **前提**: Phase 2 で Windows x64 サポートが確立し、Phase 1 の x86_64 Linux フローが安定している。

## 作業ブレークダウン
1. **仕様実装 (Phase A)**: `RunConfigTarget` と `@cfg` キーを Reml 言語へ導入し、`1-1-syntax.md` と `2-6-execution-strategy.md` の更新内容を反映。
2. **環境推論 (Phase B)**: `TargetCapability` グループと `infer_target_from_env` を実装し、`3-10-core-env.md` に基づく環境変数検出を行う。
3. **ビルドコマンド (Phase C)**: `reml build --target` コマンドを CLI に追加し、ターゲット固有のランタイム・標準ライブラリを束ねて出力。
4. **ライブラリ配布**: ターゲットごとにビルド済みライブラリを生成し、アーティファクト管理を整備。
5. **CI マトリクス構築**: GitHub Actions で 3 ターゲット全てのビルド・スモークテストを実行するマトリクスジョブを設定。
6. **ドキュメント更新**: 仕様更新を `1-0-language-core-overview.md` と `3-0-core-library-overview.md` に反映し、README を更新。

## 成果物と検証
- `reml build --target` の各プロファイルが成功し、生成物が実機または VM で動作する。
- CI マトリクスが安定稼働し、失敗時はターゲットごとのログが参照可能。
- 仕様・ガイド類が最新状態であり、差分が `0-3-audit-and-metrics.md` に記録される。

## リスクとフォローアップ
- ターゲットごとの依存ライブラリが膨大になる可能性があるため、キャッシュ戦略を設計し CI 時間を抑制。
- macOS notarization 等の外部手続きは Phase 4 リリースパイプラインで本格対応するため、準備状況を `0-4-risk-handling.md` に記録。
- 環境検出ロジックが複雑になる際は、`guides/runtime-bridges.md` と連携してメンテを容易にする。

## 参考資料
- [3-0-phase3-self-host.md](3-0-phase3-self-host.md)
- [notes/cross-compilation-spec-update-plan.md](../../notes/cross-compilation-spec-update-plan.md)
- [notes/cross-compilation-spec-intro.md](../../notes/cross-compilation-spec-intro.md)
- [3-10-core-env.md](../../3-10-core-env.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)

