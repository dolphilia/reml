# Core.Path 文字列ユーティリティ検証 (2025-11-30)

- 変更内容
  - `compiler/rust/runtime/src/path/string_utils.rs` に `PathStyle` ベースの `normalize_path_str` / `join_paths_str` / `is_absolute_str` / `relative_to` を追加し、`PathErrorKind::UnsupportedPlatform` を導入。`Core.Text` の `Str` を返す純粋 API として効果記録 (`record_text_mem_copy`) を統一した。
  - `tests/path_string_utils.rs` と `tests/data/core_path/unicode_cases.json` を新設し、POSIX・Windows ドライブ・UNC の代表ケースを JSON ゴールデンで管理。プラン §4.3 の要件に沿って README・Text 計画書との相互参照を更新した。
- 検証
  - `cargo test --manifest-path compiler/rust/runtime/Cargo.toml path_string_utils`
- 期待されるゴールデン/メタデータ
  - `tests/data/core_path/unicode_cases.json` に含まれる `normalized` / `join.result` / `relative.result` が `tests/path_string_utils.rs` の比較対象となる。
  - `docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv`・`core-io-effects-matrix.md`・`docs/notes/core-io-path-gap-log.md` を更新し、Phase3 W49 `Core.Path.Strings` 行・エントリを `Implemented` / `Closed` 扱いに変更済み。
