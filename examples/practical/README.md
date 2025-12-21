# practical スイート

Phase 4 のシナリオマトリクスで「実務投入」「監査ログ付き」のケースを共通化するためのディレクトリです。Chapter 3 の仕様（IO/Diagnostics/Runtime/Env）から引用した `.reml` を `examples/practical/<domain>/<scenario>/` 階層で整理し、`expected/practical/` 内のゴールデンと 1:1 対応させます。

- `core_io/file_copy/`: Core.IO + Core.Path の Reader/Writer、Sandbox、監査ログの組み合わせ
- `core_path/security_check/`: SecurityPolicy による拒否と診断 (`core.path.security.*`) の固定化
- `core_config/audit_bridge/`: `@dsl_export` で Runtime Bridge を登録し、Manifest ダンプと Stage 整合性を確認
- `core_text/unicode/`: Chapter 3.3 の Grapheme/正規化 API を `.reml` から直接検証
- `core_text/pretty/`: Core.Text.Pretty のレイアウト最小例
- `core_test/snapshot/`: Core.Test のスナップショット最小例
- `core_cli/parse_flags/`: Core.Cli のフラグ/引数解析最小例
- `core_parse/cst_lossless/`: Core.Parse の CST/Pretty ロスレス経路の最小例
- `core_doc/`: Core.Doc のドキュメント生成最小例
- `core_lsp/`: Core.Lsp の診断送信最小例
- `embedded_dsl/`: Markdown + Reml の埋め込み DSL 合成例
- `core_diagnostics/audit_envelope/`: `AuditEnvelope.metadata` に `scenario.id` と Stage 情報を記録するログ例
- `core_runtime/capability/`: Runtime Bridge Stage の整合性チェック (`runtime.bridge.stage_mismatch`) を再現
- `core_env/envcfg/`: `core.env.merge_profiles` と `@cfg` プロファイル同期の成功例
- `core_async/`: Core.Async の `sleep_async`/`join`/`block_on` を使った最小例

> 補足: 旧 `examples/core_io/*.reml` 等は参照用として残しつつ、Phase 4 以降のテストは本ディレクトリを参照する運用へ切り替えます。
