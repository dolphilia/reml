# 2.2 効果システム統合計画

## 目的
- `1-3-effects-safety.md` と `3-8-core-runtime-capability.md` に定義される効果タグと Stage 要件を Phase 2 で OCaml 実装へ統合する。
- Parser/Typer/Lint/Runtime が同一の Stage 判定ロジックを共有し、セルフホスト前の整合を確保する。

## スコープ
- **含む**: AST/TAST への `effect` 注釈保持、Stage 要件 (`Exact`, `AtLeast`) の検証、RuntimeCapability との照合、CI テスト。
- **含まない**: ランタイム Stage の動的変更、プラグインによる Stage 拡張。これらは Phase 3 以降。
- **前提**: Parser が効果構文を取り込み、Typer が型クラス拡張と競合しない設計であること。

## 作業ブレークダウン

### 1. 効果システム設計と仕様整理（24-25週目）
**担当領域**: 効果システム基盤設計

1.1. **効果タグとStage定義の抽出**
- `1-3-effects-safety.md` から効果タグの全種類をリスト化
- `3-8-core-runtime-capability.md` の Stage 定義をデータ構造化
- Stage 要件（`Exact`, `AtLeast`, `AtMost`）の形式化
- プラットフォーム別 Stage テーブルの整理

1.2. **データモデル設計**
- AST/TAST/IR への `EffectTag` フィールド追加
- `StageRequirement` 型の定義と検証ルール
- 効果注釈の構文表現（`@effect[Pure]` 等）
- 既存 API との後方互換性確保

1.3. **型システムとの統合方針**
- 型クラス制約と効果制約の分離設計
- 効果多相（effect polymorphism）の検討
- 関数シグネチャへの効果情報の埋め込み
- Phase 2 型クラスタスクとの調整

**成果物**: 効果データモデル、Stage 定義、統合設計書

### 2. Parser/AST 拡張（25週目）
**担当領域**: 構文解析

2.1. **効果構文の実装**
- 効果注釈の字句解析（`@effect`, `@stage` 等）
- 関数宣言・式への効果注釈の付与
- ネストした効果の構文解析
- エラーハンドリング（不正な効果指定）

2.2. **AST ノード拡張**
- `Decl::Fn` に `effects: EffectTag[]` を追加
- `Expr::*` に効果伝播用フィールド追加
- Span 情報の保持
- デバッグ用の AST pretty printer 更新

2.3. **パーサテスト整備**
- 効果注釈の正常系テスト
- 構文エラーのテスト
- ゴールデンテスト（AST 出力）
- Phase 1 パーサとの統合検証

**成果物**: 拡張 Parser、効果 AST、パーサテスト

### 3. Typer 統合と効果解析（25-26週目）
**担当領域**: 型推論と効果検証

3.1. **効果注釈の解析**
- AST から効果情報を抽出
- 関数シグネチャへの効果型の添付
- 効果の伝播ルール実装（呼び出し先→呼び出し元）
- 効果の合成（複数効果の統合）

3.2. **Stage 要件の検証**
- 関数の要求 Stage と実行環境の照合
- Stage 不一致のエラー検出
- Stage 推論（注釈がない場合のデフォルト）
- 効果型の単一化ルール

3.3. **型クラスとの整合**
- 型クラス制約と効果制約の同時解決
- 辞書引数と効果情報の独立性確保
- Typer パイプラインの責務分離
- Phase 2 型クラスタスクとの統合テスト

**成果物**: 効果解析ロジック、Stage 検証、統合 Typer

### 4. RuntimeCapability チェック実装（26-27週目）
**担当領域**: ランタイム検証

4.1. **Capability テーブル埋め込み**
- `3-8-core-runtime-capability.md` の Stage テーブルを OCaml に写像
- プラットフォーム別の Capability 定義
- Stage 判定の共通モジュール実装
- 動的 Stage 変更の検討（Phase 3 以降）

4.2. **Stage チェックロジック**
- コンパイル時の Stage 検証
- ランタイム Capability の照合（将来拡張用）
- Stage ミスマッチの詳細レポート
- テスト用の Capability モック機構

4.3. **プラットフォーム対応**
- Linux/Windows の Capability 差異の吸収
- Phase 2 Windows タスクとの連携
- Capability 定義の外部化検討（JSON 等）
- クロスコンパイル時の Stage 検証

**成果物**: Capability モジュール、Stage チェック、プラットフォーム対応

### 5. 診断システム強化（27週目）
**担当領域**: エラー報告

5.1. **効果診断の実装**
- `Diagnostic.extensions` に `effect.stage.*` を追加
- Stage ミスマッチの詳細メッセージ
- 効果タグの不一致エラー
- 候補 Stage の提示（"Available stages: ..."）

5.2. **CLI 出力統合**
- 効果情報の CLI 表示
- `--emit-effects` フラグの実装
- カラー出力対応（効果タグごとの色分け）
- `3-6-core-diagnostics-audit.md` との整合

5.3. **AuditEnvelope 統合**
- 効果メタデータの `AuditEnvelope` への記録
- Stage 検証結果の監査ログ出力
- Phase 2 診断タスクとの連携
- JSON 出力のスキーマ定義

**成果物**: 効果診断、CLI 統合、監査ログ

### 6. テスト整備（27-28週目）
**担当領域**: 品質保証

6.1. **効果シナリオテスト**
- 正常系: 各効果タグの基本動作テスト
- 異常系: Stage ミスマッチ、不正な効果指定
- 複合系: 型クラス + 効果の組み合わせ
- `tests/effects/` ディレクトリの新設

6.2. **Stage 検証テスト**
- `Exact`, `AtLeast`, `AtMost` の各要件テスト
- プラットフォーム別の Capability テスト
- ランタイム Stage の境界値テスト
- ゴールデンテスト（診断出力）

6.3. **CI/CD 統合**
- GitHub Actions に効果テストジョブ追加
- テストカバレッジの計測（>80%）
- Phase 1/2 他タスクとの統合テスト
- ビルド時間の監視

**成果物**: 効果テストスイート、CI 設定

### 7. ドキュメント更新と仕様同期（28週目）
**担当領域**: 仕様整合

7.1. **仕様書フィードバック**
- `1-3-effects-safety.md` への実装差分の反映
- 効果推論ルールの擬似コードを追加
- 新規サンプルコードの追加
- 実装上の制約・TODO の明示

7.2. **Capability 仕様の更新**
- `3-8-core-runtime-capability.md` の Stage テーブル更新
- プラットフォーム別の差異を文書化
- 将来拡張（プラグイン Stage）の検討メモ
- Phase 3 への引き継ぎ事項

7.3. **メトリクス記録**
- `0-3-audit-and-metrics.md` に効果検証のオーバーヘッド記録
- Stage チェックのコンパイル時間への影響測定
- CI レポートの自動生成設定

**成果物**: 更新仕様書、Capability 文書、メトリクス

### 8. 統合検証と Phase 3 準備（28-29週目）
**担当領域**: 統合と引き継ぎ

8.1. **Phase 2 タスク統合**
- 型クラス + 効果 + FFI の統合テスト
- 診断システムの一貫性検証
- Windows 対応との整合確認
- 仕様差分タスクとの調整

8.2. **セルフホスト準備**
- Phase 3 型チェッカへの効果システム移植計画
- OCaml 実装から Reml 実装への写像設計
- 責務分離の確認（Parser/Typer/Runtime）
- 残存課題の `notes/` への記録

8.3. **レビューと承認**
- M2/M3 マイルストーン達成報告
- 効果システムのデモンストレーション
- レビューフィードバックの反映
- Phase 3 への引き継ぎドキュメント作成

**成果物**: 統合検証レポート、セルフホスト設計、引き継ぎ文書

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

