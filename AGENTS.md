# AGENTS.md

## 目的と前提
- この文書は Reml 言語仕様リポジトリで作業する AI コーディングエージェント向けの共通指針です。
- すべての対話とテキストは日本語で行い、仕様上のコード例も Reml 構文を前提とします。
- リポジトリはドキュメント専用です。ビルド・テストを想定したコマンドは存在せず、変更は Markdown ファイルの編集が中心になります。
- 既存仕様との整合を最優先し、差分が生じた場合は関連ファイルを横断的に更新します。

## リポジトリ概観
- `0-x` 系列: 導入資料と編集ポリシー。`docs/spec/0-0-overview.md`（概要）、`docs/spec/0-1-project-purpose.md`（目的と指針）、`docs/spec/0-3-code-style-guide.md`（コードスタイル）がオンボーディングの起点です。
- `1-x` 系列: 言語コア仕様。`docs/spec/1-1-syntax.md` から `docs/spec/1-5-formal-grammar-bnf.md` までが構文・型・効果・Unicode・形式文法を定義します。
- `2-x` 系列: 標準パーサー API。`Parser<T>` の基本設計からエラー戦略・実行戦略までを扱います。
- `3-x` 系列: 標準ライブラリ仕様。プレリュード、コレクション、テキスト、診断、ランタイム、環境など 3-0〜3-10 を参照してください。
- `4-x` 系列: 公式プラグイン仕様ドラフト。システム連携・ハードウェアアクセスなど Capability 拡張の設計案です。
- `5-x` 系列: エコシステム仕様ドラフト。パッケージマネージャ、レジストリ、コミュニティ運営の方針をまとめています。
- `docs/guides/`: ツールチェーン、DSL、プラグイン、AI 連携など実務ガイド。`docs/guides/ai-integration.md` は AI 支援機能のポリシーです。
- `docs/notes/`: 調査ノートと将来計画。`docs/notes/dsl-plugin-roadmap.md` はプラグイン提供ポリシーの基礎資料です。
- `README.md`: 章構成の索引。新セクションの追加や名前変更時は必ず更新します。

### 再編準備中の補足（2025-10）
- `docs/`・`compiler/`・`runtime/`・`tooling/`・`examples/` など再編用ディレクトリを整備しました。仕様・ガイド・ノート・計画書は `docs/` 配下へ移設済みで、サンプルは `examples/` に統合済みです。
- 仕様書・ガイド・ノートを編集する際は、移設前後でリンクパスが変わる可能性を考慮し、`docs/plans/repository-restructure-plan.md` のフェーズに従って作業してください。
- `docs-migrations.log` に再編作業の履歴を記録する方針です。大きな移動・rename を行う際はログ更新を忘れないでください。

## 作業の基本原則
- **日本語での一貫性**: 本文・コメント・補足はすべて日本語で記述し、外部用語は必要に応じて原語を併記します。
- **相互参照の維持**: ファイル名・セクションタイトルを変更した場合は、関連するリンク（とくに `README.md`・章内の参照・ガイド）を更新します。
- **スタイル順守**: コード例は `docs/spec/0-3-code-style-guide.md` と `docs/spec/1-1-syntax.md` の規約に従います（インデント 2 スペース、`Result`/`Option` の利用など）。
- **仕様の整合性**: `docs/spec/1-x` と `docs/spec/3-x` の記述、`docs/spec/2-x` と `docs/guides/core-parse-streaming.md` などの関連資料をクロスチェックし、矛盾が出ないようにします。
- **調査の痕跡を残す**: 追加した判断や TODO はコメントや脚注に根拠ファイルをリンクし、読者が追跡できるようにします。
- **非破壊的編集**: 未確認の表や参考資料を削除せず、必要なら非推奨として明示します。

## 推奨ワークフロー
1. **依頼内容の整理**: 変更範囲と関連チャプターを特定し、必要な背景資料（例: `3-6-core-diagnostics-audit.md`）を確認します。
2. **調査**: `rg` などの検索で関連セクション・用語を洗い出し、既存の用語統一や表記を把握します。
3. **編集**: Markdown では見出しレベルと用語表記を揃え、コードブロックは言語タグ `reml` を付与します。差分が複数ファイルに跨る場合は一貫した順序で更新します。
4. **検証**: 新旧仕様の整合性、リンク切れ（相対パス）、表やリストの番号を確認し、影響範囲をコメントで説明します。
5. **報告**: 変更概要・影響範囲・フォローアップが必要な項目を簡潔にまとめ、必要に応じて追試タスクを提案します。

## 代表的なタスク別チェックポイント
- **仕様章の追記**: 対応する概要ファイル（例: `docs/spec/1-0-language-core-overview.md`、`docs/spec/3-0-core-library-overview.md`）にも要約を追加します。
- **新しいガイド追加**: `docs/guides/` 内で関連ドキュメントからのリンクを張り、`README.md` の該当リストを更新します。
- **用語変更・リネーム**: 用語集 `docs/spec/0-2-glossary.md` を更新し、章内検索で古い表記が残っていないか確認します。
- **プラグイン関連更新**: `docs/notes/dsl-plugin-roadmap.md` と `docs/spec/3-8-core-runtime-capability.md` を参照し、Capability 整合性と監査ポリシー（`docs/spec/3-6-core-diagnostics-audit.md`）を照合します。

## 参考資料リンク
- 言語コア: `docs/spec/1-0-language-core-overview.md`, `docs/spec/1-1-syntax.md`, `docs/spec/1-2-types-Inference.md`, `docs/spec/1-3-effects-safety.md`, `docs/spec/1-5-formal-grammar-bnf.md`
- パーサー API: `docs/spec/2-0-parser-api-overview.md`, `docs/spec/2-2-core-combinator.md`, `docs/spec/2-5-error.md`, `docs/spec/2-6-execution-strategy.md`
- 標準ライブラリ: `docs/spec/3-0-core-library-overview.md`, `docs/spec/3-3-core-text-unicode.md`, `docs/spec/3-6-core-diagnostics-audit.md`, `docs/spec/3-8-core-runtime-capability.md`, `docs/spec/3-10-core-env.md`
- ガイド: `docs/guides/ai-integration.md`, `docs/guides/core-parse-streaming.md`, `docs/guides/plugin-authoring.md`, `docs/guides/runtime-bridges.md`
- 調査メモ: `docs/notes/core-library-outline.md`, `docs/notes/dsl-plugin-roadmap.md`, `docs/notes/cross-compilation-spec-intro.md`

## エージェント別メモ
- **Claude Code**: ユーザーへの返答を含むすべてのコミュニケーションを日本語で行い、本書の方針と `CLAUDE.md` に従ってください。
- **汎用 LLM (ChatGPT / Copilot 等)**: 上記原則に従い、変更理由と影響範囲を明文化します。英語での断片が生成された場合は日本語に置き換え、必要に応じて索引用語を括弧書きで補足します。
- **自動修正ツール**: 一括変換を行う場合は対象ファイルと理由を明記し、フォーマットや表の崩れがないか手動で確認します。

## フィードバック
- ドキュメント運用上のギャップを見つけた場合は `docs/notes/` に追記案を残し、メンテナが確認できるよう TODO ヘッダを付けてください。
