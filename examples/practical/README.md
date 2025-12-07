# practical スイート

Phase 4 のシナリオマトリクスで「実務投入」「監査ログ付き」のケースを共通化するためのディレクトリです。Chapter 3 の仕様（IO/Diagnostics/Runtime/Env）から引用した `.reml` を `examples/practical/<domain>/<scenario>/` 階層で整理し、`expected/practical/` 内のゴールデンと 1:1 対応させます。

- `core_io/file_copy/`: Core.IO + Core.Path の Reader/Writer、Sandbox、監査ログの組み合わせ
- `core_path/security_check/`: SecurityPolicy による拒否と診断 (`core.path.security.*`) の固定化
- `core_config/audit_bridge/`: `@dsl_export` で Runtime Bridge を登録し、Manifest ダンプと Stage 整合性を確認

> 補足: 旧 `examples/core_io/*.reml` 等は参照用として残しつつ、Phase 4 以降のテストは本ディレクトリを参照する運用へ切り替えます。
