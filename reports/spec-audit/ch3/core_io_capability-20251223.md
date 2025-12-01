# Core.IO Capability 監査ログ (2025-12-23)

## 実行概要
- コマンド:
  - `cargo test --manifest-path compiler/rust/runtime/Cargo.toml --test core_io_capabilities`
- 参照行列: `tests/capabilities/core_io_registry.json`
  - `io.fs.read` / `io.fs.write` / `fs.permissions.*` / `fs.symlink.query` / `security.fs.policy` / `memory.buffered_io`
  - 期待ステージは `StageRequirement::{AtLeast(Beta), Exact(Stable)}` を組み合わせて定義
  - 故意に `io.fs.read` + `Exact(Alpha)` のフェイルケースを含め、`capability.stage.mismatch` の観測を確認

## 結果サマリ
- 期待値 `pass` のケースはすべて `StageId::Stable` を返し、`StageRequirement::matches` 判定を満たした
- 期待値 `fail` のケースは `capability.stage.mismatch` エラーを返し、`actual_stage = Stable` が JSON/テスト双方で観測された
- 新設 GitHub Actions ジョブ `rust-core-io-tests`（Linux）へ追加。`lint` 完了後に
  - `cargo test --manifest-path compiler/rust/runtime/Cargo.toml --features "core-io core-path" io::tests::`
  - `cargo test --manifest-path compiler/rust/runtime/Cargo.toml --features "core-io core-path" path::tests::`
  - `cargo test --manifest-path compiler/rust/runtime/Cargo.toml --test core_io_capabilities`
  - `bash scripts/validate-diagnostic-json.sh --suite core_io`
  を実行し、`core_io_capability-20251223` ログを自動更新できるようにした。

## フォローアップ
- Windows/macOS CI で同一ジョブを追加し、Capability 行列検証を多環境化する（Backlog: `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md#71`）
- `tests/capabilities/core_io_registry.json` に Watcher/Path Security 系 Capability を追加する際は `StageRequirement` の粒度（`AtLeast(StageId::Stable)` など）を仕様 `docs/spec/3-8-core-runtime-capability.md` と同期する
