# Phase 2 Outline (Standard API)

フェーズ2で拡張する予定の標準ライブラリ章について、骨子と必要項目を整理する。フェーズ1ドラフトの内容を前提に、API の形を明確化する。

---

## 1. `2-5-error.md` 拡張ポイント
- 監査メタデータ: `Diagnostic` に `audit_id`, `change_set`, `severity_hint` などを追加。
- エラーカテゴリ: `enum ErrorDomain { Config, Runtime, Network, ... }` を導入し、期待集合に紐づけ。
- FixIt テンプレート: 再利用可能な提案（例: "Add missing field", "Insert closing tag"）。
- IDE/LSP 連携: `to_lsp_diagnostics()` の仕様草案。
- ✅ システム別エラーコードの命名規則案を定義（2-5 節に表形式）

## 2. `2-6-execution-strategy.md` 拡張ポイント
- LSP/IDE メタデータ出力: `with_syntax_highlight`, `with_completion_items` のようなランナーオプション。
- 構造化ログ: `RunConfig.log_format = "json"` 等。
- ホットリロード API: `reload(parser, state, diff)` の仕様案。
- ✅ CLI 用の `reml-run` サブコマンド例を追加（2-6 節とガイド参照）

## 3. 新章 `2-7 Nest.Config` (仮)
- スキーマ宣言 API: `schema { ... }` を構築するビルダ関数。
- 差分検証: `compare(old, new) -> SchemaDiff`。
- 条件付き設定: `when` / `requires` / `compute` に対応する API。
- CLI 連携: `ConfigResult::audit()` など。
- ✅ 設定テンプレートのマージ戦略（優先順位）の仕様ドラフトを追加

## 4. 新章 `2-8 Nest.Data` (仮)
- データモデリング: `Schema`, `Column`, `ResourceId` の型定義。
- バリデータ: `validate(schema, value)`。
- スキーマ進化ユーティリティ: `diff`, `apply_migration`。
- ✅ 検証結果と `Diagnostic` の連携サンプルを追加（2-8 節）

## 5. `2-1` / `2-2` でのプラグイン登録 API
- `register_plugin(name, capabilities, parser_factory)`。
- Parser capability をチェックするための `CapabilitySet`。
- バージョン互換性: `PluginVersion` 構造体と解決ルール。

---

## レビュー観点
- フェーズ1ドラフトで定義した構文／型との整合性を確保する。
- 各 API にサンプルコードを添付し、`scenario-requirements.md` のシナリオ例へリンク。
- LSP・CLI 等エコシステムガイドとの役割分担を明確にする。
