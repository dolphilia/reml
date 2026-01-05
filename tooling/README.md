# tooling ディレクトリ構成

開発者体験、CI/検証、監査、リリース、LSP 連携など周辺ツール資産を集約する領域です。各サブディレクトリは必要に応じて `docs/plans` / `docs/spec` と連動します。

## サブディレクトリ
- `benchmarks/`: パフォーマンス計測用スクリプト（`tooling/benchmarks/benchmark-parse-throughput.sh` など）
- `ci/`: CI 補助スクリプトと監査メトリクス関連（`tooling/ci/README.md`, `tooling/ci/README-Windows.md`）
- `examples/`: 例題スイートや生成スクリプト（`run_examples.sh`, `run_phase4_suite.py` など）
- `json-schema/`: 監査・診断系の JSON Schema と配布用メタデータ
- `lsp/`: LSP/IDE 連携の設計メモと下書き（`tooling/lsp/README.md`）
- `release/`: 署名/配布パイプラインの下書き（`tooling/release/README.md`）
- `review/`: 監査ログの集計・差分・可視化ツール
- `runtime/`: ランタイム監査/Capability 関連の補助データと生成スクリプト
- `scripts/`: メンテナンス用の単発スクリプト
- `telemetry/`: テレメトリ収集の実験的ツール（Rust クレート）
- `templates/`: 生成テンプレートと回帰サンプル（`tooling/templates/lite/`）
- `toolchains/`: クロスツールチェーン管理資産（`tooling/toolchains/README.md`）

## 整備メモ
- 各サブディレクトリの README は「現状・用途・関連ドキュメント」を最小セットで記載する
- `.github/workflows/` は実行トリガーと環境定義に留め、実処理（CI 補助/署名/配布）は `tooling/ci/` と `tooling/release/` に集約する
