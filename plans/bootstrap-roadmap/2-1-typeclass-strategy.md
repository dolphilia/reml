# 2.1 型クラス実装戦略評価計画

## 目的
- Phase 2 マイルストーン M1 に向け、辞書渡し方式を主実装としつつモノモルフィゼーションを PoC 規模で比較し、採用方針を決定する。
- `1-2-types-Inference.md` の型クラス仕様と `notes/llvm-spec-status-survey.md` に整理された懸案を検証し、Phase 3 以降のセルフホスト化に備える。

## スコープ
- **含む**: 辞書生成・渡しの実装、代表型クラス (`Eq`, `Ord`, `Iterable`) の性能測定、PoC モノモルフィゼーションの評価、メトリクス記録。
- **含まない**: 全型クラスのモノモルフィゼーション、特殊化の最適化、プラグイン型クラスの処理。必要に応じて Phase 3 で検討。
- **前提**: Phase 1 の Typer/Core IR/LLVM が安定稼働し、辞書引数を扱える拡張が可能であること。

## 作業ブレークダウン
1. **辞書渡し基盤の実装**: Core IR に辞書構造体を導入し、型クラスインスタンスを辞書として生成するパスを構築。
2. **Typer 拡張**: 型推論時に制約解決を行い、辞書構築/引数挿入/選択子展開を行う。
3. **PoC モノモルフィゼーション**: `Eq`, `Ord`, `Iterable` を限定対象にテンプレート展開し、単体テストを並行実装。
4. **性能・コードサイズ計測**: `0-3-audit-and-metrics.md` に測定スクリプトを登録し、辞書渡しと PoC モノモルフィゼーションを比較。
5. **診断更新**: 型クラス解決失敗時のエラーを `3-6-core-diagnostics-audit.md` に沿って強化し、辞書情報を含める。
6. **レビューと決定**: 評価結果をまとめ、`0-4-risk-handling.md` に採用方針と却下理由を記録。

## 成果物と検証
- 辞書渡し方式で `1-2-types-Inference.md` のサンプルが全て通過すること。
- PoC モノモルフィゼーションの出力を LLVM IR で比較し、差分とコストを `notes/llvm-spec-status-survey.md` に追記。
- メトリクスが `0-3-audit-and-metrics.md` に記録され、CI でレポート化される。

## リスクとフォローアップ
- PoC の工数が膨張する場合は対象型クラスを縮小し、Phase 3 で再評価する。
- 辞書構造の ABI が未確定だと FFI との互換性が崩れるため、Phase 2 FFI 拡張タスクと連携し、構造体定義を共通化する。
- 量産型クラスの可搬性を検証するため、セルフホスト時の影響を `3-2-reml-typechecker-port.md` に引き継ぐメモを残す。

## 参考資料
- [2-0-phase2-stabilization.md](2-0-phase2-stabilization.md)
- [1-2-types-Inference.md](../../1-2-types-Inference.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)
- [notes/llvm-spec-status-survey.md](../../notes/llvm-spec-status-survey.md)

