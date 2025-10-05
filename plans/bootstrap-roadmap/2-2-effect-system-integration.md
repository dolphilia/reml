# 2.2 効果システム統合計画

## 目的
- `1-3-effects-safety.md` と `3-8-core-runtime-capability.md` に定義される効果タグと Stage 要件を Phase 2 で OCaml 実装へ統合する。
- Parser/Typer/Lint/Runtime が同一の Stage 判定ロジックを共有し、セルフホスト前の整合を確保する。

## スコープ
- **含む**: AST/TAST への `effect` 注釈保持、Stage 要件 (`Exact`, `AtLeast`) の検証、RuntimeCapability との照合、CI テスト。
- **含まない**: ランタイム Stage の動的変更、プラグインによる Stage 拡張。これらは Phase 3 以降。
- **前提**: Parser が効果構文を取り込み、Typer が型クラス拡張と競合しない設計であること。

## 作業ブレークダウン
1. **データモデルの更新**: AST/TAST/IR に `EffectTag` と `StageRequirement` を追加し、既存 API へ影響を与えないようにマイグレーション。
2. **解析ロジック追加**: Typer 内で効果注釈を解析し、関数シグネチャに Stage 情報を添付。
3. **Capability チェック**: `3-8-core-runtime-capability.md` の Stage テーブルを OCaml へ埋め込み、Stage 判定の共通モジュールを提供。
4. **診断強化**: 効果タグのミスマッチを `Diagnostic.extensions` へ `effect.stage.*` として追加し、CLI で表示。
5. **テスト整備**: 正常系/異常系の効果シナリオを `tests/effects/` に新設し、CI で実行。
6. **ドキュメント反映**: 実装差分を `1-3-effects-safety.md` にフィードバックし、必要な場合は脚注や TODO を追加。

## 成果物と検証
- Stage 判定の単体テストが全て通過し、Capability Stage のミスマッチ検査が CI で 0 件になる。
- CLI 診断で効果タグ・Stage 情報が表示され、`0-3-audit-and-metrics.md` にレポートされる。
- 仕様書の記述と実装が整合していることをレビューで確認し、差異があれば `0-4-risk-handling.md` に登録。

## リスクとフォローアップ
- Stage テーブルが増加した場合のメンテナンス負荷を軽減するため、外部定義ファイル（JSON 等）から読み込む設計を検討。
- 効果タグが増えると型クラス解析と競合する可能性があるため、Typer 内で責務を分離し、Phase 3 でセルフホスト型チェッカに渡す準備を整える。
- RuntimeCapability の定義がプラットフォーム依存となるため、Phase 2 の Windows 対応タスクと整合を取る。

## 参考資料
- [2-0-phase2-stabilization.md](2-0-phase2-stabilization.md)
- [1-3-effects-safety.md](../../1-3-effects-safety.md)
- [3-8-core-runtime-capability.md](../../3-8-core-runtime-capability.md)
- [3-6-core-diagnostics-audit.md](../../3-6-core-diagnostics-audit.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)

