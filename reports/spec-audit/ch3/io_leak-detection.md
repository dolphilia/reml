# Core.IO リーク検出スコープ検証 (2026-04-02)

- 変更内容
  - `compiler/rust/runtime/src/io/scope.rs` に `with_file` / `with_temp_dir` / `ScopeGuard` / リークトラッカーを実装し、`File` がスコープ外でも自動クリーンアップされるようにした。
  - `File` へ `FileHandleGuard` を組み込み、`leak_tracker_snapshot()` からオープンハンドル数を監視できるようにした。
  - `compiler/rust/runtime/tests/leak_detection.rs` と `tests/data/core_io/leak_detection/scoped_cleanup.json` を追加し、スコープ終了後に `open_files = 0` / `temp_dirs = 0` となることをゴールデン化。
- 検証
  - `cargo test --manifest-path compiler/rust/runtime/Cargo.toml leak_detection::scoped_resources_cleanup_matches_expected_snapshot`
- 期待される監査メタデータ
  - `leak_tracker_snapshot().open_files = 0`
  - `leak_tracker_snapshot().temp_dirs = 0`
  - `data/core_io/leak_detection/scoped_cleanup.json` の `case` 名と一致する `reports/spec-audit/ch3/io_leak-detection.md` エントリで参照する。
