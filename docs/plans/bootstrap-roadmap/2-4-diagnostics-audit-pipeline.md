# 2.4 診断・監査パイプライン強化計画

## 目的
- Phase 2 マイルストーン M3 で必要となる `Diagnostic` + `AuditEnvelope` の完全実装を実現し、監査ログのフォーマットを仕様と同期させる。
- 効果システム・FFI 拡張など他タスクのメタデータを統合し、Phase 4 の移行期に備える。

## スコープ
- **含む**: 診断データ構造拡張、`extensions` フィールド設計、JSON/テキスト両方の出力整備、監査ログの永続化、レビューツール。
- **含まない**: 外部監査システム連携、GUI ビューワ。必要に応じて Phase 4 で検討。
- **前提**:
  - Phase 1 の CLI 整備が完了し、診断結果を CLI から閲覧できる状態であること。
  - Phase 2-3 完了報告およびハンドオーバー（`docs/plans/bootstrap-roadmap/2-3-completion-report.md`, `docs/plans/bootstrap-roadmap/2-3-to-2-4-handover.md`）を確認し、`ffi_bridge.audit_pass_rate`・`bridge.*` フィールドが有効であること。
  - 技術的負債 ID 22（Windows Stage 自動検証不足）と ID 23（macOS FFI サンプル自動検証不足）を解消する計画を本フェーズのタスクに組み込むこと。
  - `tooling/runtime/audit-schema.json` v1.1 を基準スキーマとして採用し、差分変更が必要な場合は Phase 2-3 チームと調整する。

## 作業ディレクトリ
- `compiler/ocaml/src` : Diagnostic/AuditEnvelope 生成プログラム
- `tooling/cli` : CLI 出力、`--emit-diagnostic` などの整形
- `tooling/lsp` : 将来の LSP 連携に向けた仕様メモ
- `tooling/ci` : 診断 diff / JSON スキーマ検証ワークフロー
- `docs/spec/3-6-core-diagnostics-audit.md`, `docs/notes/guides-to-spec-integration-plan.md` : スキーマ更新と追跡

## 作業ブレークダウン

### 1. 診断データ構造の再設計（26-27週目）
**担当領域**: 診断基盤設計

1.1. **Diagnostic 構造の拡張**
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) の仕様を OCaml データ型に写像
- `extensions: (string * json) list` フィールドの追加
- `related: Diagnostic list` フィールドの追加（関連診断のリンク）
- `codes: string list` フィールドの追加（診断コード）

1.2. **AuditEnvelope との整合**
- `Diagnostic` と `AuditEnvelope` のフィールド共通化
- メタデータキーの命名規約策定
- Phase 2 他タスク（型クラス・効果・FFI）との調整
- バージョン管理（スキーマバージョン）の導入

1.3. **既存コードのマイグレーション**
- Phase 1 の診断生成箇所の洗い出し
- 新構造への段階的移行計画
- 後方互換性の確保（古い診断形式のサポート）
- テストコードの更新

**成果物**: 拡張 Diagnostic 型、AuditEnvelope 整合、マイグレーション計画

### 2. シリアライズ統合（27週目）
**担当領域**: 出力フォーマット

2.1. **共通シリアライズレイヤ設計**
- JSON/テキスト/構造化ログの共通抽象化
- フォーマット切替の設計（`--format` フラグ）
- カスタムフォーマッタの拡張ポイント
- エンコーディング処理（UTF-8 保証）

2.2. **JSON 出力の実装**
- `Diagnostic` → JSON のシリアライザ
- `AuditEnvelope` → JSON のシリアライザ
- JSON スキーマの定義（JSON Schema 形式）
- Pretty print/Compact のモード切替

2.3. **テキスト出力の実装**
- カラー出力対応（ANSI エスケープ）
- ソースコードスニペットの抽出
- Unicode 対応（Grapheme 単位の表示）
- Phase 1 の診断フォーマットとの統合

**成果物**: シリアライズレイヤ、JSON/テキスト出力、スキーマ

### 3. 監査ログ永続化（27-28週目）
**担当領域**: ログ管理

3.1. **CLI フラグの実装**
- `--emit-audit` フラグの追加
- `--audit-output=<path>` での出力先指定
- `--audit-level=<level>` での詳細度制御
- `--audit-format=<format>` でのフォーマット指定

3.2. **ログ永続化ロジック**
- ビルドごとの監査ログファイル生成
- ファイル名の命名規約（タイムスタンプ付き）
- ログローテーション機能
- ディスク容量管理（古いログの削除）

3.3. **ログ構造の設計**
- ビルドメタデータ（日時、バージョン、ターゲット）
- フェーズごとのログ分離（Parser/Typer/LLVM）
- 診断の重要度レベル（Error/Warning/Info）
- サマリ統計（エラー数、警告数、ビルド時間）

**成果物**: 監査ログ永続化、CLI フラグ、ログ管理

### 4. メタデータ統合（28-29週目）
**担当領域**: 拡張メタデータ

4.1. **型クラスメタデータ**
- `extensions.typeclass.*` キーの定義
- 辞書引数の型情報記録
- 制約解決の詳細ログ
- Phase 2 型クラスタスクとの連携

4.2. **効果システムメタデータ**
- `extensions.effect.*` キーの定義
- Stage 検証結果の記録
- 効果タグの伝播トレース
- Phase 2 効果タスクとの連携

4.3. **FFI メタデータ**
- `extensions.bridge.*` キーの定義
- ABI 種別・所有権注釈の記録
- FFI 呼び出しのトレース
- Phase 2 FFI タスクとの連携（技術的負債 ID 22/23 の解消を含む）

**成果物**: 統合メタデータ、キー命名規約、連携実装

### 5. レビュー支援ツール（29週目）
**担当領域**: ツール開発

5.1. **監査ログ差分ツール**
- 2つの監査ログの差分抽出
- 診断の追加/削除/変更の検出
- マークダウン/HTML レポート生成
- CI での自動実行

5.2. **統計ダッシュボード**
- 監査ログからの統計抽出
- エラー/警告の推移グラフ
- ビルド時間の推移
- 視覚化（グラフ生成）

5.3. **クエリツール**
- 監査ログの検索・フィルタリング
- 診断コードでの絞り込み
- メタデータでのクエリ
- jq 風の DSL 検討

**成果物**: 差分ツール、ダッシュボード、クエリツール

### 6. CI/CD 統合（29-30週目）
**担当領域**: 自動化

6.1. **CI での監査ログ生成**
- GitHub Actions での `--emit-audit` 実行
- 監査ログのアーティファクト保存
- PR ごとの監査ログ差分レポート
- コメント自動投稿（新規エラー/警告）
- Linux / Windows / macOS 各ワークフローで `iterator.stage.audit_pass_rate` と `ffi_bridge.audit_pass_rate` をゲート条件として導入（技術的負債 ID 22 の解消）

6.2. **スキーマ検証**
- JSON スキーマでの検証自動化
- スキーマ違反の検出とエラー報告
- スキーマバージョンの管理
- Phase 1/2 の CI との統合

6.3. **レグレッション検出**
- 診断の予期しない増加の検出
- ビルド時間の劣化検出
- 閾値設定と通知
- `0-3-audit-and-metrics.md` との連携

**成果物**: CI 統合、スキーマ検証、レグレッション検出

### 7. ドキュメント更新（30週目）
**担当領域**: 仕様整合

7.1. **仕様書フィードバック**
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) への実装差分の反映
- メタデータキー一覧の追加
- 診断フォーマットの例示
- 新規サンプルコードの追加

7.2. **ガイド更新**
- `docs/guides/ai-integration.md` の監査ログ連携を追記
- ツール使用例の追加
- トラブルシューティング情報
- ベストプラクティスの文書化

7.3. **メトリクス記録**
- `0-3-audit-and-metrics.md` に診断システムの性能記録
- スキーマバージョンの履歴
- CI レポートの自動生成設定
- 監査ポリシーの更新履歴

**成果物**: 更新仕様書、ガイド、メトリクス

### 8. 統合テストと安定化（30-31週目）
**担当領域**: 品質保証

8.1. **スナップショットテスト**
- 診断出力のゴールデンテスト
- 監査ログのゴールデンテスト
- スキーマ検証テスト
- Phase 1/2 の全テストでの監査ログ生成

8.2. **統合テスト**
- 型クラス + 効果 + FFI の診断統合テスト
- メタデータの一貫性検証
- 差分ツールの動作テスト
- ダッシュボードの生成テスト

8.3. **安定化とバグ修正**
- テスト失敗の原因調査と修正
- エッジケースの追加テスト
- 既知の制限事項の文書化
- Phase 3 への引き継ぎ準備（macOS FFI サンプル自動検証の進捗を技術的負債 ID 23 と照合）

**成果物**: スナップショットテスト、統合テスト、安定版

## 成果物と検証
- 診断/監査ログが全テストケースで期待フォーマットになることをスナップショットテストで確認。
- CLI で `--emit-audit` を指定した際に JSON が出力され、CI でスキーマ検証が行われる。
- 監査ログ差分ツールを docs に記載し、レビュー手順が共有される。

## リスクとフォローアップ
- フィールド追加によりテストが脆くなる恐れがあるため、スキーマ検証を導入しレグレッションを防止。
- 監査ログの出力量が多くなる場合、サマリ統計と詳細ログの二段構えに切り替える検討を行う。
- AI 支援関連の要件は `docs/guides/ai-integration.md` と調整し、外部公開範囲を明示。

## 参考資料
- [2-0-phase2-stabilization.md](2-0-phase2-stabilization.md)
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
- [guides/ai-integration.md](../../guides/ai-integration.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [2-3-to-2-4-handover.md](2-3-to-2-4-handover.md)
- [compiler/ocaml/docs/technical-debt.md](../../../compiler/ocaml/docs/technical-debt.md)
