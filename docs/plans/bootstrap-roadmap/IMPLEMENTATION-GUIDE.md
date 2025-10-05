# 実装ガイド - Remlブートストラップ計画の進め方

このドキュメントは、[README.md](../../spec/README.md) で定義された計画書を実際に実装する際の手引きです。

## 基本的な進め方

### 1. 事前準備

各Phaseを開始する前に：

1. **依存関係の確認**
   - 前Phase/前タスクの完了状況を確認
   - [README.md](../../spec/README.md) の依存関係グラフを参照
   - ブロッカーがあれば [0-4-risk-handling.md](0-4-risk-handling.md) に登録

2. **環境セットアップ**
   - 必要なツールチェーンのインストール
   - CIパイプラインの動作確認
   - 測定基盤の準備（[0-3-audit-and-metrics.md](0-3-audit-and-metrics.md) 参照）

3. **レビュア割当**
   - 各領域のレビュア確認
   - レビュー頻度の調整
   - 承認プロセスの確立

### 2. 実装フロー

各計画書の「作業ブレークダウン」に従って実装します：

```
週の開始
  ↓
担当ステップの確認（例: 1-1の「1. 文法資産の抽出と設計」）
  ↓
サブタスクの実行（1.1 → 1.2 → 1.3）
  ↓
成果物の生成（コード、テスト、ドキュメント）
  ↓
検証（ユニットテスト、統合テスト、CI）
  ↓
レビュー依頼
  ↓
承認 → 次ステップへ
  ↓
週の終了時：進捗報告（0-3に記録）
```

### 3. 品質保証

各ステップで以下を確認：

- **テスト**：ユニットテスト、統合テスト、ゴールデンテスト
- **性能**：測定指標が目標範囲内か（[0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)）
- **仕様整合**：関連仕様書との一致確認
- **ドキュメント**：技術文書の更新

## Phase別の重点事項

### Phase 1（1-16週）: Bootstrap Implementation

**目標**: OCaml実装でLLVM IR生成まで

**重点**:
- Parser（1-1）とTyper（1-2）は基盤となるため品質を最優先
- Core IR（1-3）の設計はPhase 2の拡張を見据える
- ランタイム（1-5）のRC実装は性能計測を徹底

**並行可能なタスク**:
- 9-12週: 1-3（Core IR）、1-6（DX）、1-7（Linux CI）は並行可
- 13-16週: 1-4（LLVM）、1-5（Runtime）は並行可

**チェックポイント**:
- M1（4週）: Parser MVP → AST出力確認
- M2（8週）: Typer MVP → 型推論結果確認
- M3（12週）: CodeGen MVP → LLVM IR検証
- M4（16週）: 診断フレーム → エラーメッセージ品質確認

### Phase 2（17-34週）: 仕様安定化

**目標**: 型クラス、効果、診断の本格実装

**重点**:
- 型クラス戦略（2-1）の評価は慎重に（辞書 vs モノモルフィゼーション）
- Windows対応（2-6）は早期着手で問題を早期発見
- 仕様差分（2-5）の解消はPhase 3の前提条件

**並行可能なタスク**:
- 17-24週: 2-1（型クラス）、2-6（Windows）は並行可
- 24-34週: 2-2（効果）、2-3（FFI）、2-4（診断）、2-5（仕様差分）は部分並行可

**重要な意思決定**:
- 24週: 型クラス実装方式の最終決定
- 34週: Windows対応完了判定 → Phase 3の前提

### Phase 3（35-68週）: Self-Host Transition

**目標**: Reml実装への段階的移行

**重点**:
- Parser移植（3-1）はCore.Parse APIの品質検証を兼ねる
- クロスコンパイル（3-3）は3ターゲット全てを並行開発
- メモリ管理評価（3-6）は定量的データに基づく決定
- セルフホストビルド（3-7）は3段階CIの安定性が鍵

**並行可能なタスク**:
- 46-51週: 3-3のターゲット別実装は並行可
- 60-66週: 3-6（メモリ評価）、3-7（CI構築）は並行可

**クリティカルパス**:
- 3-1（Parser）→ 3-2（TypeChecker）→ 3-4（CodeGen）→ 3-7（セルフホスト）

**重要な意思決定**:
- 51週: クロスコンパイル機能確定
- 62週: RC vs GC方針決定

### Phase 4（69-86週）: 移行完了

**目標**: 正式リリースとエコシステム移行

**重点**:
- 互換性検証（4-1）は3ターゲット全てで徹底
- リリースパイプライン（4-2）は署名・notarization含む
- エコシステム移行（4-4）はコミュニティ支援を並行

**並行可能なタスク**:
- 73-79週: 4-2（リリース）、4-3（ドキュメント）は並行可
- 80-85週: 4-4（エコシステム）、4-5（互換性）は並行可

**ゴール条件**:
- OCaml実装との出力一致率 95%以上
- 3ターゲット全てでCI通過
- 後方互換チェックリスト完了

## トラブルシューティング

### 性能問題

- 測定値が目標から10%超過 → Phase進行停止、対策タスク作成
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md) に記録
- Phase 2以降で最適化検討

### 仕様不整合

- 実装と仕様のギャップ発見 → 仕様書先行修正
- 関連計画書に脚注追加
- [0-4-risk-handling.md](0-4-risk-handling.md) に登録

### スケジュール遅延

- 週次レビューで進捗確認
- クリティカルパスの遅延は優先対処
- 並行タスクへのリソース再配分

### Stage/Capabilityミスマッチ

- ミスマッチ検出 → 新機能凍結
- 原因調査と修正
- テスト強化

## 補助ツールとスクリプト

### 推奨する開発支援ツール

1. **AST可視化**: GraphvizでAST/IRを図示
2. **性能プロファイリング**: `perf`, `valgrind`, `flamegraph`
3. **メモリリーク検出**: AddressSanitizer, MemorySanitizer
4. **CI監視**: GitHub Actionsのステータスバッジ
5. **差分比較**: `llvm-diff`, `dwarfdump`

### 自動化スクリプト例

```bash
# ゴールデンテスト更新
./scripts/update-golden.sh

# IR差分レポート生成
./scripts/compare-ir.sh ocaml-output reml-output

# 性能測定とレポート
./scripts/benchmark.sh --output metrics.json

# 仕様書との整合性チェック
./scripts/spec-check.sh
```

## リソース

### 主要仕様書
- [1-1-syntax.md](../../spec/1-1-syntax.md) - 構文仕様
- [1-2-types-Inference.md](../../spec/1-2-types-Inference.md) - 型システム
- [2-0-parser-api-overview.md](../../spec/2-0-parser-api-overview.md) - Parser API
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) - 診断・監査

### 技術ガイド
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md) - LLVM連携
- [notes/cross-compilation-spec-update-plan.md](../../notes/cross-compilation-spec-update-plan.md) - クロスコンパイル

### 計画管理
- [README.md](../../spec/README.md) - 統合マップ
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md) - 測定指標
- [0-4-risk-handling.md](0-4-risk-handling.md) - リスク管理

## コミュニケーション

### 週次定例
- 進捗報告（各Phase担当）
- ブロッカー共有
- 次週計画確認

### レビュー体制
- Parser/Type/Runtime/Toolchainの各領域にレビュア割当
- 週次または隔週でのレビュー
- 承認記録は [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md) に保存

### エスカレーション
- 性能問題、仕様不整合、スケジュール遅延は即座にエスカレーション
- [0-4-risk-handling.md](0-4-risk-handling.md) に登録
- 24時間以内に担当者割当

---

このガイドは計画の実行を支援するための参考資料です。各Phase/タスクの詳細は個別の計画書を参照してください。
