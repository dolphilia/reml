# CLAUDE.md

このファイルは、このリポジトリでコードを扱う際のClaude Code (claude.ai/code) への指針を提供します。

**重要**: このプロジェクトは日本語で記述されているため、Claude はユーザーとのすべてのやり取りで日本語で応答する必要があります。

## プロジェクト概要

これは **Reml言語仕様書** のリポジトリです。パーサーコンビネーターに最適化されたプログラミング言語であるReml (Readable & Expressive Meta Language) のドキュメントプロジェクトです。このリポジトリには、言語の構文、型システム、標準ライブラリ、パーサーコンビネーターAPIを定義する包括的な仕様書が日本語で書かれています。

## プロジェクト構造

これはドキュメント専用のリポジトリで、以下の主要コンポーネントで構成されています：

### 導入資料・ポリシー (0-*)

- `docs/spec/0-0-overview.md` - 高レベル言語概要と設計目標
- `docs/spec/0-1-project-purpose.md` - プロジェクトの目的と指針
- `docs/spec/0-2-glossary.md` - 用語集
- `docs/spec/0-3-code-style-guide.md` - コードスタイルガイド

### 言語コア仕様 (1-*)

- `docs/spec/1-0-language-core-overview.md` - 言語コア仕様の概要
- `docs/spec/1-1-syntax.md` - 字句解析、宣言、式を含む言語構文仕様
- `docs/spec/1-2-types-Inference.md` - 型システムとHindley-Milner推論
- `docs/spec/1-3-effects-safety.md` - 効果システムと安全性保証
- `docs/spec/1-4-test-unicode-model.md` - Unicode文字モデル（Byte/Char/Graphemeレイヤ）
- `docs/spec/1-5-formal-grammar-bnf.md` - BNF形式文法仕様

### 標準パーサーAPI仕様 (2-*)

- `docs/spec/2-0-parser-api-overview.md` - パーサーAPI概要
- `docs/spec/2-1-parser-type.md` - コアパーサー型と入力モデル
- `docs/spec/2-2-core-combinator.md` - 必須パーサーコンビネーター
- `docs/spec/2-3-lexer.md` - 字句解析ユーティリティ
- `docs/spec/2-4-op-builder.md` - 演算子優先度ビルダーシステム
- `docs/spec/2-5-error.md` - エラーハンドリングと報告設計
- `docs/spec/2-6-execution-strategy.md` - 実行時戦略（LL(∗)、Packrat、左再帰）

### 標準ライブラリ仕様 (3-*)

- `docs/spec/3-0-core-library-overview.md` - 標準ライブラリ概要
- `docs/spec/3-1-core-prelude-iteration.md` - プレリュードと反復処理
- `docs/spec/3-2-core-collections.md` - コレクション型とデータ構造
- `docs/spec/3-3-core-text-unicode.md` - テキスト処理とUnicode
- `docs/spec/3-4-core-numeric-time.md` - 数値計算と時間処理
- `docs/spec/3-5-core-io-path.md` - 入出力とパス操作
- `docs/spec/3-6-core-diagnostics-audit.md` - 診断とログ機能
- `docs/spec/3-7-core-config-data.md` - 設定データとシリアライゼーション
- `docs/spec/3-8-core-runtime-capability.md` - ランタイムCapabilityシステム
- `docs/spec/3-9-core-async-ffi-unsafe.md` - 非同期処理・FFI・Unsafeコード
- `docs/spec/3-10-core-env.md` - 環境変数とシステム情報

### 公式プラグイン仕様 (4-*)

- `docs/spec/4-0-official-plugins-overview.md` - 公式プラグイン概要
- `docs/spec/4-1-system-plugin.md` - システム連携プラグイン
- `docs/spec/4-2-process-plugin.md` - プロセス管理プラグイン
- `docs/spec/4-3-memory-plugin.md` - メモリ管理プラグイン
- `docs/spec/4-4-signal-plugin.md` - シグナル処理プラグイン
- `docs/spec/4-5-hardware-plugin.md` - ハードウェアアクセスプラグイン
- `docs/spec/4-6-realtime-plugin.md` - リアルタイム処理プラグイン

### エコシステム仕様 (5-*)

- `docs/spec/5-0-ecosystem-overview.md` - エコシステム概要
- `docs/spec/5-1-package-manager-cli.md` - パッケージマネージャとCLI
- `docs/spec/5-2-registry-distribution.md` - レジストリと配布システム
- `docs/spec/5-3-developer-toolchain.md` - 開発者ツールチェーン
- `docs/spec/5-4-community-content.md` - コミュニティとコンテンツ
- `docs/spec/5-5-roadmap-metrics.md` - ロードマップとメトリクス
- `docs/spec/5-6-risk-governance.md` - リスク管理とガバナンス

### 実務ガイド (docs/guides/)

- ツールチェーン、DSL、プラグイン、AI連携など実装に関するガイド
- `docs/guides/ai-integration.md` - AI支援機能のポリシー
- `docs/guides/core-parse-streaming.md` - パーサーのストリーミング処理
- `docs/guides/plugin-authoring.md` - プラグイン開発ガイド
- `docs/guides/runtime-bridges.md` - ランタイムブリッジ

### 調査ノート (docs/notes/)

- 調査ノート、将来計画、設計検討資料
- `docs/notes/dsl-plugin-roadmap.md` - プラグイン提供ポリシーの基礎資料
- `docs/notes/core-library-outline.md` - 標準ライブラリ設計メモ
- `docs/notes/cross-compilation-spec-intro.md` - クロスコンパイル仕様

### サンプル (examples/)

- 代数的効果などの実装例とデモンストレーション

### 再編準備中の補足（2025-10）

- リポジトリ再編に備えて `docs/`, `compiler/`, `runtime/`, `tooling/`, `examples/` ディレクトリを新設しました（現在はプレースホルダと README のみ）。
- 仕様書やガイドを編集する際は、移設フェーズ（`docs/plans/repository-restructure-plan.md`）に従い、移動前後のパス整合を確認してください。
- 再編関連の変更を行った場合は `docs-migrations.log` へ記録し、相互参照切れがないか `grep` 等で検証してください。

## 開発ワークフロー

**注意: これはビルドシステムや実行可能コードのないドキュメントプロジェクトです。**

### 共通タスク

これは仕様書リポジトリのため、ビルド、テスト、リントコマンドはありません。開発は以下で構成されます：

1. **仕様書ドキュメントの編集** - 仕様変更に伴う `.md` ファイルの更新
2. **相互参照** - 関連する仕様セクション間の一貫性確保
3. **検証** - ドキュメント内の例が定義された構文と一致することの確認

### ドキュメント標準

- すべての仕様書は日本語で記述
- コード例は仕様で定義されたReml構文を使用
- セクション間の相互参照は相対リンクを使用（例：`[型システム](docs/spec/1-2-types-Inference.md)`）
- メインの目次は `docs/README.md` で管理

## 主要なアーキテクチャ概念

### 言語設計哲学

- **簡潔性**: 演算子優先度と空白処理を宣言で実現
- **可読性**: 左から右へのパイプライン、名前付き引数、強力な推論
- **高品質なエラー**: 位置追跡、期待集合、cut/commit、復旧、トレース
- **性能**: 末尾最適化、トランポリン、オプションのPackrat/左再帰
- **Unicode第一**: 3層モデル（byte/char/grapheme）

### パーサーコンビネーターアーキテクチャ

標準ライブラリ（`Core.Parse`）は階層設計に従います：

1. **コア型**（`docs/spec/2-1-parser-type.md`）: `Parser<T>`、`Input`、`State`（consumed/committedセマンティクス）
2. **コアコンビネーター**（`docs/spec/2-2-core-combinator.md`）: 12-15の必須コンビネーター（map、then、or、many等）
3. **字句レイヤ**（`docs/spec/2-3-lexer.md`）: 空白処理、コメント、リテラル、識別子
4. **演算子ビルダー**（`docs/spec/2-4-op-builder.md`）: left/right/nonassocによる宣言的優先度
5. **エラーシステム**（`docs/spec/2-5-error.md`）: 期待集合、cut/label/recover/trace
6. **実行**（`docs/spec/2-6-execution-strategy.md`）: LL(∗)デフォルト、オプションPackratメモ化、左再帰サポート

### Unicodeモデル

3つの異なる文字レイヤをサポート：

- `Byte`: 生のUTF-8バイト
- `Char`: Unicodeスカラー値（コードポイント）
- `Grapheme`: 拡張書記素クラスター（ユーザーが認識する文字）

## このリポジトリでの作業

### 作業の基本原則

仕様に変更を加える際は以下の原則に従ってください：

1. **日本語での一貫性** - 本文・コメント・補足はすべて日本語で記述し、外部用語は必要に応じて原語を併記します
2. **相互参照の維持** - ファイル名・セクションタイトルを変更した場合は、関連するリンク（特に `docs/README.md`・章内の参照・ガイド）を更新します
3. **スタイル順守** - コード例は `docs/spec/0-3-code-style-guide.md` と `docs/spec/1-1-syntax.md` の規約に従います（インデント2スペース、`Result`/`Option`の利用など）
4. **仕様の整合性** - `1-*`と`3-*`の記述、`2-*`と`docs/guides/core-parse-streaming.md`などの関連資料をクロスチェックし、矛盾が出ないようにします
5. **調査の痕跡を残す** - 追加した判断やTODOはコメントや脚注に根拠ファイルをリンクし、読者が追跡できるようにします
6. **非破壊的編集** - 未確認の表や参考資料を削除せず、必要なら非推奨として明示します

### 推奨ワークフロー

1. **依頼内容の整理** - 変更範囲と関連チャプターを特定し、必要な背景資料（例：`docs/spec/3-6-core-diagnostics-audit.md`）を確認します
2. **調査** - 検索で関連セクション・用語を洗い出し、既存の用語統一や表記を把握します
3. **編集** - Markdownでは見出しレベルと用語表記を揃え、コードブロックは言語タグ`reml`を付与します。差分が複数ファイルに跨る場合は一貫した順序で更新します
4. **検証** - 新旧仕様の整合性、リンク切れ（相対パス）、表やリストの番号を確認し、影響範囲をコメントで説明します
5. **報告** - 変更概要・影響範囲・フォローアップが必要な項目を簡潔にまとめ、必要に応じて追試タスクを提案します

### タスク別チェックポイント

- **仕様章の追記** - 対応する概要ファイル（例：`docs/spec/1-0-language-core-overview.md`、`docs/spec/3-0-core-library-overview.md`）にも要約を追加します
- **新しいガイド追加** - `docs/guides/`内で関連ドキュメントからのリンクを張り、`docs/README.md`の該当リストを更新します
- **用語変更・リネーム** - 用語集 `docs/spec/0-2-glossary.md`を更新し、章内検索で古い表記が残っていないか確認します
- **プラグイン関連更新** - `docs/notes/dsl-plugin-roadmap.md`と`docs/spec/3-8-core-runtime-capability.md`を参照し、Capability整合性と監査ポリシー（`3-6`）を照合します

### 重要な注意事項

- メインの目次は`docs/README.md`で管理されているため、新しいセクション追加時は必ず更新してください
- セクション間の相互参照は相対リンクを使用してください（例：`[型システム](docs/spec/1-2-types-Inference.md)`）
- すべての仕様書は日本語で記述し、コード例は仕様で定義されたReml構文を使用してください

これらの仕様書は実装に依存しない設計でありながら、完全なReml言語実装を構築するのに十分な詳細を提供するよう設計されています。
