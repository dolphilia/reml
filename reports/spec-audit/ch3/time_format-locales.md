# Core.Time ロケールテーブル更新ログ (2025-11-28)

- `tooling/scripts/update_time_locale_table.py` を新規作成し、`docs/plans/bootstrap-roadmap/assets/time-format-locale-map.csv` から `compiler/rust/runtime/src/time/locale_table_data.rs` を自動生成するフローを整備した。
- CSV には `und`/`ja-JP`/`tr-TR`/`az-Latn`/`zh-TW` を登録。`tr-TR` までを `LocaleStatus::Supported`、`az-Latn` と `zh-TW` を `Planned` とし、フォールバックやメモを表形式で維持する。
- 生成されたテーブルは `cargo test --manifest-path compiler/rust/runtime/Cargo.toml --features core_time time::tests::time_format_cases_from_dataset` で検証し、`tests/data/time/format/format_cases.json` に `tr-TR` カスタムケースを追加して挙動を固定。
- `LocaleStatus::Planned` の行は `format_with_locale` で拒否されるため、`time::tests::planned_locale_is_rejected` で `az-Latn` が `TimeErrorKind::InvalidFormat` を返すことを確認した。
- 監査観点: `TIME_LOCALE_TABLE` が CSV から再生成された際には本ログに追記し、`0-3-audit-and-metrics.md` の `numeric_time.effect_matrix_pass_rate` で `LocaleId` チェックがカバーされるよう CI を更新する。
