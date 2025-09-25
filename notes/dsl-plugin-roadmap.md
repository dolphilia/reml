# DSLプラグイン提供ロードマップ

Reml の DSL ファースト戦略に沿って、プラグイン領域で提供する拡張機能の設計・提供計画をまとめる。

## 1. DSLテンプレート／ジェネレーター

### 1.1 目的

- 小規模プロジェクトでも DSL ファーストアプローチを導入しやすくする。
- Core.Parse と Conductor 構文を最小セットで体験できるスケルトンを生成する。

### 1.2 提供物

- `reml-plugin-dsl-template`（CLI）：`dsl init --kind config` などのサブコマンドで各種テンプレートを生成。
- テンプレート内容：
  - Conductor ブロックを含むメインファイル
  - Core.Parse を利用した基本的な `rule` サンプル
  - Core.Diagnostics 連携済みの監視設定
  - プロジェクト構成案（tests/、guides/ への導線）

### 1.3 実装ステップ

1. **アルファ版 (0-1ヶ月)**: JSON/CSV 向けテンプレートと README を同梱。
2. **ベータ版 (1-2ヶ月)**: テンプレート選択肢の拡充（設定DSL、ETL DSLなど）と CI ワークフロー。
3. **安定版 (2-3ヶ月)**: モジュール式テンプレート（DSL断片の追加インストール）をサポート。

### 1.4 エコシステム連携

- guides/ に設置する各ガイドと相互リンク。
- Capability Registry のサンプル設定を `templates/runtime.toml` として配布。
- 公式プラグインカタログに登録し、`reml plugin install` から入手可能にする。

## 2. 運用監視／Circuit Breaker 拡張

### 2.1 目的

- Conductor 実行中の DSL に対する可観測性とフォールトトレランス機構を提供。
- Core.Async と Core.Diagnostics の橋渡し機能をパッケージ化する。

### 2.2 提供物

- `reml-plugin-dsl-observability`：
  - DSLメトリクス登録のヘルパー (`register_dsl_metrics` のデフォルト実装)
  - Grafana / OpenTelemetry へのエクスポータ設定
  - Circuit Breaker ポリシーを YAML/Reml から読み込むユーティリティ
- `reml-plugin-dsl-fallback`：
  - ExecutionPlan のエラーポリシーを動的に差し替えるランタイムフック
  - カウンタ付き Circuit Breaker の参照実装（半開制御、フェイルファスト）

### 2.3 実装ステップ

1. **PoC (0-1ヶ月)**: DSLメトリクスの自動登録とエラー率の閾値判定を実装。
2. **拡張 (1-3ヶ月)**: Circuit Breaker と Backpressure ポリシー可視化を統合。
3. **安定化 (3-4ヶ月)**: 複数環境（オンプレ／クラウド）向け設定プリセットとドキュメント整備。

### 2.4 依存と配布

- Core.Diagnostics 3.6 で定義した `DslMetricsHandle`, `start_dsl_span` を前提とする。
- Core.Async 3.9 の `ExecutionPlan` API と互換性を維持。
- プラグイン配布は公式レジストリ経由。インストール後に `plugins/dsl-observability/README.md` を展開し、環境変数設定例を案内。

## 3. 共通ガバナンス

- バージョン管理: Reml 本体とは独立した `reml-plugins` モノレポで管理、Semantic Versioning 適用。
- 互換性保証: Reml 本体のマイナーバージョンに同期して互換テストを実行。
- 監査対応: プラグイン経由で取得したテレメトリは Core.Diagnostics のポリシーに従い、匿名化と保持期間を設定。
