# Core IO サンプル

`Core.IO` と `Core.Path` の組み合わせで Reader/Writer、`IoContext`、パスサンドボックスをどのように扱うかを確認するサンプル群です。Phase 3-5 実装計画 §6 で要求された「安全なファイル操作の実例」をこのディレクトリに集約します。

## `file_copy.reml`
- `with_reader` / `with_writer` / `copy` を組み合わせてファイルをコピーします。
- `sandbox_path` で書き込み先を固定ディレクトリに制限し、`log_io` で `metadata.io.*` と監査ログを記録します。
- `tooling/examples/run_examples.sh --suite core_io` で自動実行できます。CI からは `core_io.example_suite_pass_rate` 指標として監視されます。

関連ドキュメント:
- `docs/spec/3-5-core-io-path.md` §7「使用例」
- `docs/guides/runtime/runtime-bridges.md` §1.4「Core.IO コンテキストと監査」
- `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` §6「ドキュメント・サンプル更新」
