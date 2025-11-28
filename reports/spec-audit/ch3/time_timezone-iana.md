# Core.Time タイムゾーン (IANA) 検証ログ — 2025-12-12

- 代表ケース: `Asia/Tokyo`, `Europe/London`, `America/New_York`
- 入力データ: `tests/data/time/timezone_iana.json`
- コマンド: `cargo test --manifest-path compiler/rust/runtime/Cargo.toml --features core_time time::tests::timezone_cases_from_dataset`
- 目的: `timezone()` が IANA 名を受理し、`offset.seconds()` が `tests/data/time/timezone_iana.json` の `expected_offset_seconds` と一致することを確認する。
- 備考: 現状は代表 3 件の静的オフセット（UTC±HH:MM）で運用。`collect-iterator-audit-metrics.py --section numeric_time --scenario timezone_lookup --tz-source tests/data/time/timezone_iana.json` を併用し、CI アーティファクトへケース数と offset 一致ステータスを記録する。
