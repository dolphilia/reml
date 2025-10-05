# 2.4 診断・監査パイプライン強化計画

## 目的
- Phase 2 マイルストーン M3 で必要となる `Diagnostic` + `AuditEnvelope` の完全実装を実現し、監査ログのフォーマットを仕様と同期させる。
- 効果システム・FFI 拡張など他タスクのメタデータを統合し、Phase 4 の移行期に備える。

## スコープ
- **含む**: 診断データ構造拡張、`extensions` フィールド設計、JSON/テキスト両方の出力整備、監査ログの永続化、レビューツール。
- **含まない**: 外部監査システム連携、GUI ビューワ。必要に応じて Phase 4 で検討。
- **前提**: Phase 1 の CLI 整備が完了し、診断結果を CLI から閲覧できる状態であること。

## 作業ブレークダウン
1. **データ構造再設計**: `Diagnostic` に `extensions`, `related`, `codes` を追加し、`AuditEnvelope` とフィールド整合。
2. **シリアライズ統合**: JSON/テキスト出力を共通レイヤにまとめ、フォーマットの変更を容易にする。
3. **監査ログ永続化**: CLI で `--emit-audit` フラグを実装し、ビルドごとのログを保存。
4. **メタデータ合流**: 効果システム・FFI・型クラスの情報を `extensions` に投入し、キー命名規約を `3-6-core-diagnostics-audit.md` に追加。
5. **レビュー支援ツール**: 監査ログの差分比較スクリプトを作成し、レビュー容易化。
6. **ドキュメント更新**: 仕様書 (`3-6-core-diagnostics-audit.md`) とガイド (`guides/ai-integration.md`) に反映し、監査ポリシーの更新を共有。

## 成果物と検証
- 診断/監査ログが全テストケースで期待フォーマットになることをスナップショットテストで確認。
- CLI で `--emit-audit` を指定した際に JSON が出力され、CI でスキーマ検証が行われる。
- 監査ログ差分ツールを docs に記載し、レビュー手順が共有される。

## リスクとフォローアップ
- フィールド追加によりテストが脆くなる恐れがあるため、スキーマ検証を導入しレグレッションを防止。
- 監査ログの出力量が多くなる場合、サマリ統計と詳細ログの二段構えに切り替える検討を行う。
- AI 支援関連の要件は `guides/ai-integration.md` と調整し、外部公開範囲を明示。

## 参考資料
- [2-0-phase2-stabilization.md](2-0-phase2-stabilization.md)
- [3-6-core-diagnostics-audit.md](../../3-6-core-diagnostics-audit.md)
- [guides/ai-integration.md](../../guides/ai-integration.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)

