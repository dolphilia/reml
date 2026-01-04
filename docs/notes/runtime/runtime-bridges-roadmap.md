# Runtime Bridges ロードマップメモ

`docs/guides/runtime/runtime-bridges.md` と Phase3 Core.IO/Path の計画 (`docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md`) を接続する運用メモ。Bridge / Plugin / サンプル実行が参照すべき Runbook をここに集約し、CI 指標とリンクさせる。

## 1. Core.IO & Path サンプルの取り込み
- `examples/practical/core_io/file_copy/canonical.reml` と `examples/practical/core_path/security_check/relative_denied.reml`（旧 `examples/core_io` / `examples/core_path`）を `tooling/examples/run_examples.sh --suite core_io|core_path` から実行する。CI では `core_io.example_suite_pass_rate`（`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`）として 1.0 を維持する。
- 実行ログは `reports/spec-audit/ch3/core_io_examples-YYYYMMDD.md` と `reports/spec-audit/ch3/core_path_examples-YYYYMMDD.md` に保存し、Bridge/Plugin のレビュー時に引用する。
- サンプルが失敗した場合は `docs/notes/stdlib/core-io-path-gap-log.md` にギャップを登録し、`docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md#6` のステータスを `In Progress` へ戻す。

## 2. Runtime Bridge で確認すべきポイント
- `IoContext.helper` / `metadata.io.helper`: Bridge 名を一意に付与し、`examples.core_io.file_copy` などサンプルと同じ命名規約で揃える。
- `Capability` 検証: `io.fs.*`, `security.fs.policy`, `memory.buffered_io`, `watcher.*` を `FsAdapter::ensure_*` / `WatcherAdapter::ensure_*` / `SecurityPolicy::enforce` から呼ぶ。Stage mismatch は `effects.contract.stage_mismatch` を発火させる。
- `log_io` / 監査メタデータ: `docs/guides/runtime/runtime-bridges.md` §1.4 を更新済み。Bridge 実装前にサンプルを実行して `AuditEnvelope.metadata["io.*"]` のキーを確認する。

## 3. Runbook（簡易）
1. `tooling/examples/run_examples.sh --suite core_io`
2. `tooling/examples/run_examples.sh --suite core_path`
3. `python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario example_suite --output reports/spec-audit/ch3/core_io_examples.json --require-success`
4. `scripts/validate-diagnostic-json.sh --pattern core.io --pattern core.path.security reports/spec-audit/ch3/core_io_examples.json`

`reports/spec-audit/ch3/core_io_examples.json` に `run_id`, `helper`, `effect.stage.required/actual`, `metadata.security.reason` 等を保存し、Bridge/Plugin のレビューで引用する。

## 4. 参照ドキュメント
- `docs/spec/3-5-core-io-path.md`
- `docs/guides/runtime/runtime-bridges.md`
- `docs/guides/dsl/plugin-authoring.md`
- `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` §6
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`

## 5. WASM ブリッジ依存固定メモ（Rust 1.69 維持）
- `compiler/runtime` の WASM ブリッジ検証では、`rustc 1.69.0` を維持するため `wasmtime=6.0.2`（`default-features = false`, `features = ["cranelift"]`）へ固定。
- `wat=1.0.68` を採用して `wasm-encoder v0.31.1` に揃え、`url=2.3.1` / `bumpalo=3.12.0` を固定することで Rust 1.69 でのビルドを維持する。
